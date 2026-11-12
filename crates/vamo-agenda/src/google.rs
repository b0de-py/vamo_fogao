//! O Google Agenda pela API: a segunda porta da agenda, atrás da feature
//! `google` (desligada por padrão).
//!
//! O link ICS do Google às vezes demora horas pra mostrar um evento novo. A
//! API responde na hora, mas pede login: o OAuth de aplicativo de desktop,
//! com o navegador abrindo numa porta local (`127.0.0.1`). O token que
//! sobra do login (o `refresh_token`) fica num arquivo da pasta da config,
//! só com permissão pra quem usa, fora do repositório e fora do banco.
//!
//! A API já devolve a recorrência expandida (`singleEvents=true`), cada
//! ocorrência com o horário e o fuso dela. Daqui sai o mesmo
//! [`Appointment`] que o leitor do ICS entrega: o resto do app não sabe de
//! qual porta a agenda veio.

use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};
use serde_json::{Value, json};
use vamo_core::{Agenda, Appointment, When};

use crate::link::{Freshness, agent, read_cache, save_cache};
use crate::{Error, Footer};

const AUTHORIZE: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN: &str = "https://oauth2.googleapis.com/token";
const EVENTS: &str = "https://www.googleapis.com/calendar/v3/calendars";
/// Só leitura: o vamo nunca escreve na agenda.
const SCOPE: &str = "https://www.googleapis.com/auth/calendar.readonly";

/// O cliente OAuth que quem usa cria no Google Cloud (tipo "app de
/// desktop"). No app de desktop, o Google não trata o segredo como segredo
/// de verdade, mas ele fica só na config mesmo assim.
#[derive(Debug, Clone)]
pub struct OAuthClient {
    /// O ID do cliente (`...apps.googleusercontent.com`).
    pub id: String,
    /// O segredo do cliente.
    pub secret: String,
}

/// A conexão com o Google Agenda: o cliente, o arquivo do token, a agenda
/// lida e onde a cópia local fica.
#[derive(Debug, Clone)]
pub struct GoogleLink {
    client: OAuthClient,
    token: PathBuf,
    calendar: String,
    cache: PathBuf,
}

impl GoogleLink {
    /// A agenda `calendar` (`primary` é a principal), com o token em
    /// `token` e a cópia em `cache`.
    pub fn new(client: OAuthClient, token: &Path, calendar: &str, cache: &Path) -> Self {
        Self {
            client,
            token: token.to_owned(),
            calendar: calendar.to_owned(),
            cache: cache.to_owned(),
        }
    }

    /// O login: abre uma porta local, entrega o endereço de autorização pra
    /// `show` (o binário mostra e tenta abrir o navegador), espera o Google
    /// voltar com o código e troca o código pelo token, que fica gravado.
    ///
    /// # Errors
    ///
    /// - [`Error::Google`] quando quem usa recusa, o retorno não confere ou
    ///   o Google não devolve o token.
    /// - [`Error::GoogleToken`] quando não dá pra gravar o token.
    pub fn login(&self, show: impl FnOnce(&str)) -> Result<(), Error> {
        let failed = |problem: &str| Error::Google {
            problem: problem.to_owned(),
        };
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|_| failed("não deu pra abrir uma porta local pro retorno do login"))?;
        let port = listener
            .local_addr()
            .map_err(|_| failed("não deu pra abrir uma porta local pro retorno do login"))?
            .port();
        let redirect = format!("http://127.0.0.1:{port}");
        let state = random_state();
        show(&authorize_url(&self.client, &redirect, &state));

        let (mut stream, _) = listener
            .accept()
            .map_err(|_| failed("o retorno do login não chegou"))?;
        let mut request = String::new();
        BufReader::new(&stream)
            .read_line(&mut request)
            .map_err(|_| failed("o retorno do login veio quebrado"))?;
        let answer = Callback::parse(&request);
        let page = match &answer {
            Ok(_) => "Pronto: o vamo já pode ler a agenda. Pode fechar esta aba.",
            Err(_) => "O login não deu certo. Volte pro terminal pra ver o motivo.",
        };
        // Se a resposta pro navegador falhar, o login segue: ela é só
        // cortesia.
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
             <!doctype html><meta charset=\"utf-8\"><p>{page}</p>"
        );
        let callback = answer?;
        if callback.state != state {
            return Err(failed("o retorno do login não é deste pedido"));
        }

        let response = post_form(&[
            ("code", callback.code.as_str()),
            ("client_id", &self.client.id),
            ("client_secret", &self.client.secret),
            ("redirect_uri", &redirect),
            ("grant_type", "authorization_code"),
        ])?;
        let refresh = field(&response, "refresh_token")
            .ok_or_else(|| failed("o Google não devolveu o token de acesso contínuo"))?;
        save_token(&self.token, refresh)
    }

    /// Baixa os compromissos de `from` a `to` e atualiza a cópia. Se a
    /// internet ou o Google falharem e tiver cópia, devolve a cópia.
    ///
    /// # Errors
    ///
    /// - [`Error::NotLoggedIn`] sem o login feito.
    /// - [`Error::Google`] quando não deu pra baixar e não tem cópia.
    /// - [`Error::Cache`] quando não deu pra gravar a cópia.
    pub fn fetch(&self, from: Timestamp, to: Timestamp) -> Result<(String, Freshness), Error> {
        match self.download(from, to) {
            Ok(text) => {
                save_cache(&self.cache, &text)?;
                Ok((text, Freshness::Fresh))
            }
            Err(error @ (Error::NotLoggedIn | Error::GoogleToken { .. })) => Err(error),
            Err(error) => match read_cache(&self.cache)? {
                Some(text) => {
                    tracing::warn!(%error, "o Google Agenda não respondeu; usando a cópia local");
                    Ok((text, Freshness::Cached))
                }
                None => Err(error),
            },
        }
    }

    /// A cópia local, se já tiver uma.
    ///
    /// # Errors
    ///
    /// [`Error::Cache`] quando a cópia existe mas não dá pra ler.
    pub fn cached(&self) -> Result<Option<String>, Error> {
        read_cache(&self.cache)
    }

    fn download(&self, from: Timestamp, to: Timestamp) -> Result<String, Error> {
        let refresh = load_token(&self.token)?;
        let response = post_form(&[
            ("client_id", self.client.id.as_str()),
            ("client_secret", &self.client.secret),
            ("refresh_token", &refresh),
            ("grant_type", "refresh_token"),
        ])?;
        let access = field(&response, "access_token").ok_or_else(|| Error::Google {
            problem: "o Google não devolveu o token de acesso".to_owned(),
        })?;

        let mut items = Vec::new();
        let mut page: Option<String> = None;
        loop {
            let mut url = format!(
                "{EVENTS}/{}/events?singleEvents=true&orderBy=startTime&maxResults=2500\
                 &timeMin={}&timeMax={}",
                encode(&self.calendar),
                encode(&from.to_string()),
                encode(&to.to_string()),
            );
            if let Some(token) = &page {
                url = format!("{url}&pageToken={}", encode(token));
            }
            let body = get(&url, access)?;
            if let Some(Value::Array(found)) = body.get("items") {
                items.extend(found.iter().cloned());
            }
            page = field(&body, "nextPageToken").map(str::to_owned);
            if page.is_none() {
                break;
            }
        }
        Ok(json!({ "items": items }).to_string())
    }
}

/// O endereço de autorização que o navegador abre.
fn authorize_url(client: &OAuthClient, redirect: &str, state: &str) -> String {
    format!(
        "{AUTHORIZE}?client_id={}&redirect_uri={}&response_type=code&scope={}\
         &access_type=offline&prompt=consent&state={}",
        encode(&client.id),
        encode(redirect),
        encode(SCOPE),
        encode(state),
    )
}

/// O que o Google manda de volta pra porta local.
#[derive(Debug, PartialEq, Eq)]
struct Callback {
    code: String,
    state: String,
}

impl Callback {
    /// Lê a primeira linha do pedido: `GET /?code=...&state=... HTTP/1.1`.
    fn parse(request: &str) -> Result<Self, Error> {
        let failed = |problem: &str| Error::Google {
            problem: problem.to_owned(),
        };
        let target = request
            .split_whitespace()
            .nth(1)
            .ok_or_else(|| failed("o retorno do login veio quebrado"))?;
        let query = target.split_once('?').map_or("", |(_, query)| query);
        let mut code = None;
        let mut state = None;
        for pair in query.split('&') {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            let value = decode(value);
            match key {
                "code" => code = Some(value),
                "state" => state = Some(value),
                "error" => {
                    return Err(failed(&format!("o Google recusou o login ({value})")));
                }
                _ => {}
            }
        }
        Ok(Self {
            code: code.ok_or_else(|| failed("o retorno do login veio sem o código"))?,
            state: state.unwrap_or_default(),
        })
    }
}

/// Um valor difícil de adivinhar pro `state` do OAuth, que prova que o
/// retorno é deste pedido. A semente do `RandomState` vem do sistema; dois
/// hashes de duas sementes dão 128 bits, o que basta pra quem só precisa
/// não ser adivinhado durante um login.
fn random_state() -> String {
    let first = RandomState::new().hash_one(0_u8);
    let second = RandomState::new().hash_one(1_u8);
    format!("{first:016x}{second:016x}")
}

/// O texto codificado pra ir numa URL.
fn encode(text: &str) -> String {
    let mut encoded = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => {
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                encoded.push('%');
                encoded.push(char::from(HEX[usize::from(byte >> 4)]));
                encoded.push(char::from(HEX[usize::from(byte & 0x0F)]));
            }
        }
    }
    encoded
}

/// O contrário do [`encode`], com o `+` virando espaço.
fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => decoded.push(b' '),
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    decoded.push(byte);
                    index += 2;
                } else {
                    decoded.push(b'%');
                }
            }
            byte => decoded.push(byte),
        }
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// Um campo de texto do JSON.
fn field<'a>(value: &'a Value, name: &str) -> Option<&'a str> {
    value.get(name).and_then(Value::as_str)
}

/// O POST de formulário no endpoint do token.
fn post_form(form: &[(&str, &str)]) -> Result<Value, Error> {
    let mut response = agent()
        .post(TOKEN)
        .send_form(form.iter().copied())
        .map_err(|error| google(&error))?;
    read_json(response.body_mut().read_to_string())
}

/// O GET autenticado na API.
fn get(url: &str, access: &str) -> Result<Value, Error> {
    let mut response = agent()
        .get(url)
        .header("Authorization", &format!("Bearer {access}"))
        .call()
        .map_err(|error| google(&error))?;
    read_json(
        response
            .body_mut()
            .with_config()
            .limit(20 * 1024 * 1024)
            .read_to_string(),
    )
}

fn read_json(body: Result<String, ureq::Error>) -> Result<Value, Error> {
    let body = body.map_err(|error| google(&error))?;
    serde_json::from_str(&body).map_err(|_| Error::Google {
        problem: "a resposta não é um JSON".to_owned(),
    })
}

/// O motivo da falha, sem token nem URL.
fn google(error: &ureq::Error) -> Error {
    let problem = match error {
        ureq::Error::StatusCode(401 | 403) => {
            "o Google recusou o acesso (rode vamo conectar-google de novo)".to_owned()
        }
        ureq::Error::StatusCode(code) => format!("o Google respondeu com o código {code}"),
        ureq::Error::Timeout(_) => "o Google demorou demais pra responder".to_owned(),
        ureq::Error::HostNotFound | ureq::Error::Io(_) | ureq::Error::ConnectionFailed => {
            "não deu pra conectar no Google (sem internet?)".to_owned()
        }
        _ => "o Google Agenda não respondeu".to_owned(),
    };
    Error::Google { problem }
}

/// Grava o token só com permissão pra quem usa.
fn save_token(path: &Path, refresh: &str) -> Result<(), Error> {
    let failed = |source| Error::GoogleToken {
        path: path.to_owned(),
        source,
    };
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(failed)?;
    }
    let text = json!({ "refresh_token": refresh }).to_string();
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(path).map_err(failed)?;
    file.write_all(text.as_bytes()).map_err(failed)
}

fn load_token(path: &Path) -> Result<String, Error> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::NotLoggedIn);
        }
        Err(source) => {
            return Err(Error::GoogleToken {
                path: path.to_owned(),
                source,
            });
        }
    };
    serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|value| field(&value, "refresh_token").map(str::to_owned))
        .ok_or(Error::NotLoggedIn)
}

/// Um evento da resposta da API.
#[derive(Debug, Clone)]
struct GoogleEvent {
    id: String,
    /// A série, quando o evento é uma ocorrência de um recorrente.
    series: Option<String>,
    title: String,
    description: String,
    start: Edge,
    end: Edge,
}

/// O começo ou o fim: o dia (evento de dia inteiro) ou o instante.
#[derive(Debug, Clone, Copy)]
enum Edge {
    Day(Date),
    At(Timestamp),
}

impl Edge {
    fn parse(value: Option<&Value>) -> Option<Self> {
        let value = value?;
        if let Some(day) = field(value, "date") {
            return day.parse().ok().map(Self::Day);
        }
        field(value, "dateTime")?.parse().ok().map(Self::At)
    }
}

/// A agenda lida da resposta da API (ou da cópia dela).
#[derive(Debug, Clone)]
pub struct GoogleCalendar {
    events: Vec<GoogleEvent>,
    zone: TimeZone,
}

impl GoogleCalendar {
    /// Lê a resposta da API (`{"items": [...]}`), com os horários levados
    /// pro fuso `zone`. Evento cancelado fica de fora.
    ///
    /// # Errors
    ///
    /// [`Error::Google`] quando o texto não é um JSON com `items`.
    pub fn parse(text: &str, zone: TimeZone) -> Result<Self, Error> {
        let failed = || Error::Google {
            problem: "a cópia da agenda do Google não é um JSON com items".to_owned(),
        };
        let value: Value = serde_json::from_str(text).map_err(|_| failed())?;
        let Some(Value::Array(items)) = value.get("items") else {
            return Err(failed());
        };
        let events = items
            .iter()
            .filter(|item| field(item, "status") != Some("cancelled"))
            .filter_map(|item| {
                Some(GoogleEvent {
                    id: field(item, "id")?.to_owned(),
                    series: field(item, "recurringEventId").map(str::to_owned),
                    title: field(item, "summary").unwrap_or("(sem título)").to_owned(),
                    description: field(item, "description").unwrap_or("").to_owned(),
                    start: Edge::parse(item.get("start"))?,
                    end: Edge::parse(item.get("end"))?,
                })
            })
            .collect();
        Ok(Self { events, zone })
    }

    /// Os compromissos que tocam os dias de `from` a `to`, na ordem do dia,
    /// como o [`crate::Calendar::read`]. A API não deixa regra sem
    /// resolver, então a lista de avisos sai sempre vazia: ela existe pra
    /// as duas portas terem a mesma cara.
    pub fn read(&self, from: Date, to: Date) -> (Vec<Appointment>, Vec<String>) {
        let mut found = Vec::new();
        for event in &self.events {
            let whens: Vec<When> = match (event.start, event.end) {
                (Edge::Day(start), Edge::Day(end)) => {
                    // O fim do dia inteiro é exclusivo, como no ICS.
                    let mut days = Vec::new();
                    let mut day = start;
                    loop {
                        days.push(When::AllDay(day));
                        match day.checked_add(1.day()) {
                            Ok(next) if next < end => day = next,
                            _ => break,
                        }
                    }
                    days
                }
                (Edge::At(start), Edge::At(end)) => vec![When::Timed {
                    start: start.to_zoned(self.zone.clone()).datetime(),
                    end: end.to_zoned(self.zone.clone()).datetime(),
                }],
                _ => continue,
            };
            for when in whens {
                let touches = match when {
                    When::AllDay(day) => from <= day && day <= to,
                    When::Timed { start, end } => start.date() <= to && end.date() >= from,
                };
                if touches {
                    found.push(Appointment {
                        // A ocorrência de um recorrente tem ID próprio; a
                        // série é que é o mesmo compromisso todo dia.
                        uid: event.series.clone().unwrap_or_else(|| event.id.clone()),
                        title: event.title.trim().to_owned(),
                        when,
                        recurring: event.series.is_some(),
                        task: Footer::find(&event.description).map(|footer| footer.task),
                    });
                }
            }
        }
        found.sort_by(|a, b| {
            crate::calendar::sort_key(a)
                .cmp(&crate::calendar::sort_key(b))
                .then(a.title.cmp(&b.title))
        });
        (found, Vec::new())
    }
}

impl Agenda for GoogleCalendar {
    type Error = Error;

    fn appointments(&self, from: Date, to: Date) -> Result<Vec<Appointment>, Error> {
        Ok(self.read(from, to).0)
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::Calendar;

    fn zone() -> TimeZone {
        jiff::tz::db().get("America/Sao_Paulo").unwrap()
    }

    /// Uma resposta gravada da API, com nomes e horários inventados.
    fn recorded() -> GoogleCalendar {
        GoogleCalendar::parse(include_str!("../tests/fixtures/google.json"), zone()).unwrap()
    }

    #[test]
    fn the_recorded_response_becomes_appointments() {
        let (day, warnings) = recorded().read(date(2026, 11, 16), date(2026, 11, 16));
        assert_eq!(warnings, Vec::<String>::new());
        let lines: Vec<String> = day
            .iter()
            .map(|appointment| {
                format!(
                    "{:?} {} {} {:?}",
                    appointment.when, appointment.uid, appointment.title, appointment.task
                )
            })
            .collect();
        insta::assert_snapshot!(lines.join("\n"));
    }

    #[test]
    fn the_same_week_reads_the_same_from_either_port() {
        let ics = Calendar::parse(include_str!("../tests/fixtures/google.ics"), zone()).unwrap();
        let (from, to) = (date(2026, 11, 16), date(2026, 11, 22));
        let strip = |appointments: Vec<Appointment>| {
            appointments
                .into_iter()
                .map(|appointment| {
                    (
                        appointment.title,
                        appointment.when,
                        appointment.recurring,
                        appointment.task,
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            strip(recorded().read(from, to).0),
            strip(ics.read(from, to).0)
        );
    }

    #[test]
    fn urls_are_encoded_and_decoded() {
        assert_eq!(encode("a b/c:d+e"), "a%20b%2Fc%3Ad%2Be");
        assert_eq!(decode("4%2F0Ab+c%3D"), "4/0Ab c=");
        assert_eq!(decode("100%"), "100%");
        let url = authorize_url(
            &OAuthClient {
                id: "123.apps.googleusercontent.com".to_owned(),
                secret: "segredo".to_owned(),
            },
            "http://127.0.0.1:4321",
            "abc",
        );
        assert!(
            url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A4321"),
            "{url}"
        );
        assert!(url.contains("calendar.readonly"), "{url}");
        assert!(!url.contains("segredo"), "{url}");
    }

    #[test]
    fn the_callback_carries_the_code_or_the_refusal() {
        assert_eq!(
            Callback::parse("GET /?state=abc&code=4%2F0Ab&scope=x HTTP/1.1\r\n").unwrap(),
            Callback {
                code: "4/0Ab".to_owned(),
                state: "abc".to_owned()
            }
        );
        let refused = Callback::parse("GET /?error=access_denied&state=abc HTTP/1.1").unwrap_err();
        assert!(refused.to_string().contains("access_denied"), "{refused}");
        assert!(Callback::parse("GET /favicon.ico HTTP/1.1").is_err());
    }

    #[test]
    fn the_token_is_saved_for_the_user_only() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("vamo_fogao").join("google-token.json");
        assert!(matches!(load_token(&path), Err(Error::NotLoggedIn)));
        save_token(&path, "1//token-de-mentira").unwrap();
        assert_eq!(load_token(&path).unwrap(), "1//token-de-mentira");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn without_login_fetch_says_so_even_with_a_copy() {
        let folder = tempfile::tempdir().unwrap();
        let cache = folder.path().join("agenda-google.json");
        std::fs::write(&cache, "{\"items\": []}").unwrap();
        let link = GoogleLink::new(
            OAuthClient {
                id: "x".to_owned(),
                secret: "y".to_owned(),
            },
            &folder.path().join("sem-token.json"),
            "primary",
            &cache,
        );
        let now = Timestamp::from_second(1_800_000_000).unwrap();
        assert!(matches!(link.fetch(now, now), Err(Error::NotLoggedIn)));
        assert!(link.cached().unwrap().is_some());
    }
}
