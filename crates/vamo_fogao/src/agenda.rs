//! A agenda no binário: liga a config, a porta escolhida (o link ICS ou,
//! com a feature `google`, o Google Agenda) e a cópia local ao leitor do
//! `vamo-agenda`.

use color_eyre::eyre::{Result, WrapErr, eyre};
use jiff::ToSpan;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use vamo_agenda::{Calendar, Freshness, IcsLink};
use vamo_core::{Appointment, GameDay, Projection, TimeRules, When};

use crate::config::{AgendaConfig, AgendaSource, Config};
use crate::paths;

/// A agenda lida, de uma porta ou da outra. Daqui pra frente, o app não
/// sabe de qual veio.
pub(crate) enum Source {
    /// Do link ICS.
    Ics(Calendar),
    /// Do Google Agenda.
    #[cfg(feature = "google")]
    Google(vamo_agenda::GoogleCalendar),
}

impl Source {
    /// Os compromissos de `from` a `to` e os avisos da leitura.
    pub(crate) fn read(&self, from: Date, to: Date) -> (Vec<Appointment>, Vec<String>) {
        match self {
            Self::Ics(calendar) => calendar.read(from, to),
            #[cfg(feature = "google")]
            Self::Google(calendar) => calendar.read(from, to),
        }
    }
}

/// A agenda lida, e de onde ela veio.
pub(crate) struct Loaded {
    pub(crate) calendar: Source,
    pub(crate) freshness: Freshness,
}

/// Baixa a agenda da config (ou usa a cópia local, sem internet). O Google
/// só devolve os dias pedidos, de `from` a `to`; o ICS vem inteiro.
/// `Ok(None)` quando a config não tem agenda.
pub(crate) fn load(config: &AgendaConfig, from: Date, to: Date) -> Result<Option<Loaded>> {
    match config.source {
        AgendaSource::Ics => {
            let Some(url) = &config.ics else {
                return Ok(None);
            };
            let link = IcsLink::new(url, &paths::agenda_cache()?);
            let (text, freshness) = link.fetch()?;
            Ok(Some(Loaded {
                calendar: Source::Ics(parse_ics(&text, TimeZone::system())?),
                freshness,
            }))
        }
        AgendaSource::Google => google::load(config, from, to).map(Some),
    }
}

fn parse_ics(text: &str, zone: TimeZone) -> Result<Calendar> {
    Calendar::parse(text, zone).wrap_err("a agenda não é um ICS que o vamo leia")
}

/// A agenda da cópia local, sem tocar na internet.
fn cached(config: &AgendaConfig, zone: TimeZone) -> Result<Option<Source>> {
    match config.source {
        AgendaSource::Ics => {
            let Some(url) = &config.ics else {
                return Ok(None);
            };
            let Some(text) = IcsLink::new(url, &paths::agenda_cache()?).cached()? else {
                return Ok(None);
            };
            Ok(Some(Source::Ics(parse_ics(&text, zone)?)))
        }
        AgendaSource::Google => google::cached(config, zone),
    }
}

/// Os compromissos de `from` a `to`, lidos só da cópia local, como na
/// [`schedule`]. Sem agenda, sem cópia ou com a cópia ilegível, nenhum, e o
/// motivo vai pro log.
pub(crate) fn cached_appointments(
    config: &Config,
    zone: TimeZone,
    from: Date,
    to: Date,
) -> Vec<Appointment> {
    match cached(&config.agenda, zone) {
        Ok(Some(source)) => source.read(from, to).0,
        Ok(None) => Vec::new(),
        Err(error) => {
            tracing::warn!("a cópia local da agenda não deu pra ler: {error:#}");
            Vec::new()
        }
    }
}

/// A agenda de hoje pra tela Hoje, lida só da cópia local: a TUI abre na
/// hora, sem esperar a internet. Quem atualiza a cópia é o `vamo sync` (e o
/// `vamo agenda`). Sem agenda na config, sem cópia ou com a cópia ilegível,
/// a tela mostra só as brechas do expediente, e o motivo vai pro log.
pub(crate) fn schedule(config: &Config, now: &jiff::Zoned) -> vamo_tui::Schedule {
    let today = now.date();
    let appointments = cached_appointments(config, now.time_zone().clone(), today, today);
    vamo_tui::Schedule {
        fixed: vamo_core::fixed_activities(&appointments, &config.aliases),
        appointments,
        rules: config.rules.clone(),
        min_gap: config.min_gap.unwrap_or(vamo_core::DEFAULT_MIN_GAP),
        now: Some(now.time()),
        // O dia de jogo depende da linguagem: quem chama põe.
        game: None,
        team: config.games.team().to_owned(),
    }
}

/// O Google Agenda, com a feature `google`.
#[cfg(feature = "google")]
pub(crate) mod google {
    use color_eyre::eyre::{Result, WrapErr, eyre};
    use jiff::ToSpan;
    use jiff::civil::Date;
    use jiff::tz::TimeZone;
    use vamo_agenda::{GoogleCalendar, GoogleLink, OAuthClient};

    use super::{Loaded, Source};
    use crate::config::AgendaConfig;
    use crate::paths;

    fn link(config: &AgendaConfig) -> Result<GoogleLink> {
        let google = &config.google;
        let (Some(id), Some(secret)) = (&google.client_id, &google.client_secret) else {
            return Err(eyre!(
                "a fonte google precisa do cliente OAuth: ponha google_cliente e google_segredo \
                 na seção [agenda] da config"
            ));
        };
        let client = OAuthClient {
            id: id.clone(),
            secret: secret.clone(),
        };
        let calendar = google.calendar.as_deref().unwrap_or("primary");
        Ok(GoogleLink::new(
            client,
            &paths::google_token()?,
            calendar,
            &paths::google_cache()?,
        ))
    }

    fn parse(text: &str, zone: TimeZone) -> Result<GoogleCalendar> {
        GoogleCalendar::parse(text, zone).wrap_err("a agenda do Google veio num formato estranho")
    }

    pub(super) fn load(config: &AgendaConfig, from: Date, to: Date) -> Result<Loaded> {
        let zone = TimeZone::system();
        let start = from.to_zoned(zone.clone())?.timestamp();
        let end = to.checked_add(1.day())?.to_zoned(zone.clone())?.timestamp();
        let (text, freshness) = link(config)?.fetch(start, end)?;
        Ok(Loaded {
            calendar: Source::Google(parse(&text, zone)?),
            freshness,
        })
    }

    pub(super) fn cached(config: &AgendaConfig, zone: TimeZone) -> Result<Option<Source>> {
        let Some(text) = link(config)?.cached()? else {
            return Ok(None);
        };
        Ok(Some(Source::Google(parse(&text, zone)?)))
    }

    /// `vamo conectar-google`: o login, com o endereço mostrado no
    /// terminal e aberto no navegador, se der.
    pub(crate) fn connect(config: &AgendaConfig) -> Result<String> {
        link(config)?.login(|url| {
            eprintln!(
                "Abra este endereço no navegador pra autorizar o vamo a ler a agenda:\n\n{url}\n"
            );
            // Se o navegador não abrir sozinho, o endereço já está na tela.
            let _ = std::process::Command::new("xdg-open")
                .arg(url)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        })?;
        Ok(format!(
            "Conectado ao Google Agenda. O token ficou em {}; apagar o arquivo desconecta.",
            paths::google_token()?.display()
        ))
    }
}

/// Sem a feature `google`, a fonte `google` só explica como ligar.
#[cfg(not(feature = "google"))]
pub(crate) mod google {
    use color_eyre::eyre::{Result, eyre};
    use jiff::civil::Date;
    use jiff::tz::TimeZone;

    use super::{Loaded, Source};
    use crate::config::AgendaConfig;

    fn not_compiled() -> color_eyre::Report {
        eyre!(
            "esta versão do vamo foi compilada sem o Google Agenda: instale com \
             cargo install --path crates/vamo_fogao --features google, ou use fonte = \"ics\""
        )
    }

    pub(super) fn load(_: &AgendaConfig, _: Date, _: Date) -> Result<Loaded> {
        Err(not_compiled())
    }

    pub(super) fn cached(_: &AgendaConfig, _: TimeZone) -> Result<Option<Source>> {
        Err(not_compiled())
    }

    pub(crate) fn connect(_: &AgendaConfig) -> Result<String> {
        Err(not_compiled())
    }
}

/// As regras do tempo que uma tarefa marcada na agenda fere, com o ID na
/// frente: `T-041: foco depois do teto da noite (22:00)`. Com `game`, a
/// regra do dia de jogo também vale.
pub(crate) fn violations(
    appointment: &Appointment,
    projection: &Projection,
    rules: &TimeRules,
    game: Option<&GameDay>,
) -> Vec<String> {
    let (Some(id), Some((start, end))) = (appointment.task, appointment.span()) else {
        return Vec::new();
    };
    let Some(task) = projection.task(id) else {
        return Vec::new();
    };
    let near_game = game.and_then(|game_day| game_day.check(&task, start, end));
    rules
        .check(&task, start, end)
        .iter()
        .chain(near_game.iter())
        .map(|violation| format!("{id}: {violation}"))
        .collect()
}

/// As brechas do dia no `vamo agenda`, com as tarefas que cabem em cada
/// uma. Hoje, só as que ainda não passaram. Sem expediente na config, nada.
pub(crate) fn gap_lines(
    config: &Config,
    appointments: &[Appointment],
    projection: &Projection,
    day: Date,
) -> Vec<String> {
    if config.rules.workday.is_none() {
        return Vec::new();
    }
    let now = jiff::Zoned::now();
    let not_before = (now.date() == day).then(|| now.time());
    let min = config.min_gap.unwrap_or(vamo_core::DEFAULT_MIN_GAP);
    let found = vamo_core::gaps(day, &config.rules, appointments, min, not_before);
    let tasks: Vec<_> = projection.tasks().collect();
    let mut lines = vec![String::new(), "Brechas:".to_owned()];
    if found.is_empty() {
        lines.push("nenhuma".to_owned());
    }
    for gap in found {
        let fits: Vec<String> = vamo_core::fitting(gap, day, &tasks, appointments)
            .iter()
            .map(|task| format!("{} {}", task.id(), task.title()))
            .collect();
        let when = format!(
            "{}–{}",
            gap.start.strftime("%H:%M"),
            gap.end.strftime("%H:%M")
        );
        if fits.is_empty() {
            lines.push(when);
        } else {
            lines.push(format!("{when:<11}  {}", fits.join(" · ")));
        }
    }
    lines
}

/// Uma duração como se escreve: `45 min`, `2h`, `1h30`.
pub(crate) fn length(span: jiff::SignedDuration) -> String {
    let minutes = span.as_mins();
    match (minutes / 60, minutes % 60) {
        (0, minutes) => format!("{minutes} min"),
        (hours, 0) => format!("{hours}h"),
        (hours, minutes) => format!("{hours}h{minutes:02}"),
    }
}

/// O erro de quando a config não tem o link da agenda.
pub(crate) fn not_configured() -> color_eyre::Report {
    eyre!(
        "a agenda não está configurada: ponha o link ICS na config, numa seção [agenda] com \
         ics = \"https://...\""
    )
}

/// Os dias que o `vamo sync` olha: duas semanas pra trás (pra pegar evento
/// que mudou de dia) e três meses pra frente.
pub(crate) fn sync_window(today: Date) -> Result<(Date, Date)> {
    let from = today.checked_sub(14.days())?;
    let to = today.checked_add(3.months())?;
    Ok((from, to))
}

/// Uma linha da agenda: `09:00–09:30  Reunião`, com `↻` nos recorrentes.
pub(crate) fn line(appointment: &Appointment, day: Date) -> String {
    let when = match appointment.when {
        When::AllDay(_) => "dia inteiro".to_owned(),
        When::Timed { start, end } => {
            // Um evento que começou num dia anterior mostra 00:00 no começo.
            let begin = if start.date() < day {
                "00:00".to_owned()
            } else {
                start.strftime("%H:%M").to_string()
            };
            let finish = if end.date() > day {
                "24:00".to_owned()
            } else {
                end.strftime("%H:%M").to_string()
            };
            format!("{begin}–{finish}")
        }
    };
    let repeat = if appointment.recurring { " ↻" } else { "" };
    format!("{when:<11}  {}{repeat}", appointment.title)
}
