//! O leitor do formato iCalendar (ICS, RFC 5545), só no pedaço que a agenda
//! usa: os eventos (`VEVENT`) e as propriedades deles.
//!
//! O arquivo é uma lista de linhas `NOME;PARAM=valor:valor`. Linha comprida
//! continua na seguinte, que começa com espaço ou tab ("dobra"). Os
//! componentes abrem com `BEGIN:` e fecham com `END:`. Fuso (`VTIMEZONE`),
//! alarme (`VALARM`) e o resto ficam de fora: os horários usam o nome do
//! fuso da base IANA (`TZID=America/Sao_Paulo`), que é o que o Google manda.

use jiff::civil::{Date, DateTime};
use jiff::{SignedDuration, Timestamp};

use crate::Error;

/// Um instante como o ICS escreve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Moment {
    /// Só a data (`VALUE=DATE`): evento de dia inteiro.
    Date(Date),
    /// Data e hora sem fuso: vale o horário de quem lê.
    Floating(DateTime),
    /// Data e hora em UTC (termina em `Z`).
    Utc(Timestamp),
    /// Data e hora no fuso com esse nome.
    Zoned(DateTime, String),
}

/// Um `VEVENT`, com os campos que a agenda usa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawEvent {
    /// A linha do `BEGIN:VEVENT`, pras mensagens de erro.
    pub(crate) line: usize,
    pub(crate) uid: String,
    pub(crate) summary: String,
    pub(crate) description: String,
    pub(crate) start: Moment,
    pub(crate) end: Option<Moment>,
    pub(crate) duration: Option<SignedDuration>,
    /// A regra de recorrência, ainda em texto (`FREQ=WEEKLY;BYDAY=MO`).
    pub(crate) rrule: Option<String>,
    /// As ocorrências tiradas da recorrência.
    pub(crate) exdates: Vec<Moment>,
    /// Numa exceção, qual ocorrência ela substitui.
    pub(crate) recurrence_id: Option<Moment>,
    /// `STATUS:CANCELLED`.
    pub(crate) cancelled: bool,
}

/// Uma linha já desdobrada, com o número da linha onde começou.
struct Line {
    number: usize,
    text: String,
}

/// Uma propriedade: nome, parâmetros e valor.
struct Property {
    name: String,
    params: Vec<(String, String)>,
    value: String,
}

impl Property {
    fn param(&self, name: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// Lê todos os `VEVENT` do texto.
pub(crate) fn parse(text: &str) -> Result<Vec<RawEvent>, Error> {
    let mut events = Vec::new();
    let mut current: Option<Draft> = None;
    // Dentro de um componente que não é o evento (um `VALARM` dentro do
    // `VEVENT`, por exemplo), as propriedades são dele e ficam de fora.
    let mut nested = 0_usize;

    for line in unfold(text) {
        if line.text.trim().is_empty() {
            continue;
        }
        let property = split(&line)?;
        match (property.name.as_str(), &mut current) {
            ("BEGIN", None) if property.value.eq_ignore_ascii_case("VEVENT") => {
                current = Some(Draft::new(line.number));
            }
            ("BEGIN", Some(_)) => nested += 1,
            ("END", Some(_)) if nested > 0 => nested -= 1,
            ("END", Some(_)) if property.value.eq_ignore_ascii_case("VEVENT") => {
                if let Some(draft) = current.take() {
                    events.push(draft.finish()?);
                }
            }
            (_, Some(draft)) if nested == 0 => draft.read(&property, line.number)?,
            _ => {}
        }
    }
    Ok(events)
}

/// Junta as linhas dobradas: uma linha que começa com espaço ou tab continua
/// a anterior, sem esse primeiro caractere.
fn unfold(text: &str) -> Vec<Line> {
    let mut lines: Vec<Line> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        match (raw.strip_prefix([' ', '\t']), lines.last_mut()) {
            (Some(rest), Some(last)) => last.text.push_str(rest),
            _ => lines.push(Line {
                number: index + 1,
                text: raw.to_owned(),
            }),
        }
    }
    lines
}

/// Separa `NOME;PARAM=valor;OUTRO="com:dois pontos":valor`.
fn split(line: &Line) -> Result<Property, Error> {
    let broken = || Error::Ics {
        line: line.number,
        problem: "a linha não tem o formato NOME:valor".to_owned(),
    };
    // O valor começa no primeiro `:` fora de aspas.
    let mut quoted = false;
    let colon = line
        .text
        .char_indices()
        .find(|&(_, c)| {
            if c == '"' {
                quoted = !quoted;
            }
            c == ':' && !quoted
        })
        .map(|(index, _)| index)
        .ok_or_else(broken)?;
    let (head, value) = (&line.text[..colon], &line.text[colon + 1..]);

    let mut parts = head.split(';');
    let name = parts.next().unwrap_or_default().trim().to_ascii_uppercase();
    if name.is_empty() {
        return Err(broken());
    }
    let params = parts
        .map(|part| match part.split_once('=') {
            Some((key, value)) => (key.trim().to_owned(), value.trim_matches('"').to_owned()),
            None => (part.trim().to_owned(), String::new()),
        })
        .collect();
    Ok(Property {
        name,
        params,
        value: value.to_owned(),
    })
}

/// Um evento sendo lido, com os campos ainda opcionais.
struct Draft {
    line: usize,
    uid: Option<String>,
    summary: String,
    description: String,
    start: Option<Moment>,
    end: Option<Moment>,
    duration: Option<SignedDuration>,
    rrule: Option<String>,
    exdates: Vec<Moment>,
    recurrence_id: Option<Moment>,
    cancelled: bool,
}

impl Draft {
    fn new(line: usize) -> Self {
        Self {
            line,
            uid: None,
            summary: String::new(),
            description: String::new(),
            start: None,
            end: None,
            duration: None,
            rrule: None,
            exdates: Vec::new(),
            recurrence_id: None,
            cancelled: false,
        }
    }

    fn read(&mut self, property: &Property, line: usize) -> Result<(), Error> {
        match property.name.as_str() {
            "UID" => self.uid = Some(property.value.trim().to_owned()),
            "SUMMARY" => self.summary = unescape(&property.value),
            "DESCRIPTION" => self.description = unescape(&property.value),
            "DTSTART" => self.start = Some(moment(property, &property.value, line)?),
            "DTEND" => self.end = Some(moment(property, &property.value, line)?),
            "DURATION" => self.duration = Some(duration(&property.value, line)?),
            "RRULE" => self.rrule = Some(property.value.trim().to_owned()),
            "EXDATE" => {
                for value in property.value.split(',') {
                    self.exdates.push(moment(property, value, line)?);
                }
            }
            "RECURRENCE-ID" => {
                self.recurrence_id = Some(moment(property, &property.value, line)?);
            }
            "STATUS" => self.cancelled = property.value.trim().eq_ignore_ascii_case("CANCELLED"),
            _ => {}
        }
        Ok(())
    }

    fn finish(self) -> Result<RawEvent, Error> {
        let missing = |field: &str| Error::Ics {
            line: self.line,
            problem: format!("o evento não tem {field}"),
        };
        let start = self.start.clone().ok_or_else(|| missing("DTSTART"))?;
        Ok(RawEvent {
            line: self.line,
            uid: self.uid.clone().ok_or_else(|| missing("UID"))?,
            summary: self.summary,
            description: self.description,
            start,
            end: self.end,
            duration: self.duration,
            rrule: self.rrule,
            exdates: self.exdates,
            recurrence_id: self.recurrence_id,
            cancelled: self.cancelled,
        })
    }
}

/// Lê uma data ou data e hora, olhando o `VALUE=DATE` e o `TZID`.
fn moment(property: &Property, value: &str, line: usize) -> Result<Moment, Error> {
    let value = value.trim();
    let wrong = || Error::Ics {
        line,
        problem: format!(
            "`{value}` não é uma data do ICS: o formato é 20261112 ou 20261112T090000"
        ),
    };
    let is_date = property
        .param("VALUE")
        .is_some_and(|kind| kind.eq_ignore_ascii_case("DATE"))
        || value.len() == 8;
    if is_date {
        return parse_date(value).ok_or_else(wrong).map(Moment::Date);
    }
    if let Some(utc) = value.strip_suffix('Z') {
        let datetime = parse_datetime(utc).ok_or_else(wrong)?;
        let timestamp = datetime
            .to_zoned(jiff::tz::TimeZone::UTC)
            .map_err(|_| wrong())?
            .timestamp();
        return Ok(Moment::Utc(timestamp));
    }
    let datetime = parse_datetime(value).ok_or_else(wrong)?;
    Ok(match property.param("TZID") {
        Some(zone) => Moment::Zoned(datetime, zone.to_owned()),
        None => Moment::Floating(datetime),
    })
}

/// `20261112`.
pub(crate) fn parse_date(text: &str) -> Option<Date> {
    if text.len() != 8 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let year = text[0..4].parse().ok()?;
    let month = text[4..6].parse().ok()?;
    let day = text[6..8].parse().ok()?;
    Date::new(year, month, day).ok()
}

/// `20261112T090000`.
pub(crate) fn parse_datetime(text: &str) -> Option<DateTime> {
    let (date, time) = text.split_once('T')?;
    let date = parse_date(date)?;
    if time.len() != 6 || !time.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let hour = time[0..2].parse().ok()?;
    let minute = time[2..4].parse().ok()?;
    let second = time[4..6].parse().ok()?;
    let time = jiff::civil::Time::new(hour, minute, second, 0).ok()?;
    Some(date.to_datetime(time))
}

/// Uma duração do ICS: `PT1H30M`, `P1D`, `P1W`.
fn duration(text: &str, line: usize) -> Result<SignedDuration, Error> {
    let wrong = || Error::Ics {
        line,
        problem: format!("`{text}` não é uma duração do ICS: o formato é como PT1H30M ou P1D"),
    };
    let (negative, rest) = match text.trim().strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.trim().trim_start_matches('+')),
    };
    let rest = rest.strip_prefix('P').ok_or_else(wrong)?;
    let mut seconds: i64 = 0;
    let mut number = String::new();
    let mut in_time = false;
    for c in rest.chars() {
        match c {
            'T' => in_time = true,
            '0'..='9' => number.push(c),
            unit => {
                let n: i64 = number.parse().map_err(|_| wrong())?;
                number.clear();
                seconds += n * match (unit, in_time) {
                    ('W', false) => 7 * 86_400,
                    ('D', false) => 86_400,
                    ('H', true) => 3_600,
                    ('M', true) => 60,
                    ('S', true) => 1,
                    _ => return Err(wrong()),
                };
            }
        }
    }
    if !number.is_empty() {
        return Err(wrong());
    }
    let duration = SignedDuration::from_secs(seconds);
    Ok(if negative { -duration } else { duration })
}

/// Desfaz o escape do texto do ICS: `\n` vira quebra de linha, e `\,`,
/// `\;` e `\\` voltam a ser o caractere.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n' | 'N') => out.push('\n'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    const SAMPLE: &str = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
UID:um@exemplo\r\n\
DTSTART;TZID=America/Sao_Paulo:20261112T090000\r\n\
DTEND;TZID=America/Sao_Paulo:20261112T093000\r\n\
SUMMARY:Reunião\\, rápida\r\n\
DESCRIPTION:Pauta curta.\\nvamo: T-041 | Trocar o filtro da \r\n água | casa | brecha\r\n\
BEGIN:VALARM\r\n\
DESCRIPTION:Lembrete do alarme\r\n\
END:VALARM\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

    #[test]
    fn reads_an_event_with_folded_and_escaped_text() {
        let events = parse(SAMPLE).unwrap();
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.uid, "um@exemplo");
        assert_eq!(event.summary, "Reunião, rápida");
        assert_eq!(
            event.description,
            "Pauta curta.\nvamo: T-041 | Trocar o filtro da água | casa | brecha"
        );
        assert_eq!(
            event.start,
            Moment::Zoned(
                date(2026, 11, 12).at(9, 0, 0, 0),
                "America/Sao_Paulo".to_owned()
            )
        );
    }

    #[test]
    fn reads_dates_utc_and_floating_times() {
        let text = "BEGIN:VEVENT\nUID:a\nDTSTART;VALUE=DATE:20261120\nEND:VEVENT\n\
                    BEGIN:VEVENT\nUID:b\nDTSTART:20261120T120000Z\nEND:VEVENT\n\
                    BEGIN:VEVENT\nUID:c\nDTSTART:20261120T120000\nEND:VEVENT\n";
        let starts: Vec<Moment> = parse(text).unwrap().into_iter().map(|e| e.start).collect();
        assert_eq!(starts[0], Moment::Date(date(2026, 11, 20)));
        assert_eq!(
            starts[1],
            Moment::Utc("2026-11-20T12:00:00Z".parse().unwrap())
        );
        assert_eq!(
            starts[2],
            Moment::Floating(date(2026, 11, 20).at(12, 0, 0, 0))
        );
    }

    #[test]
    fn reads_durations() {
        assert_eq!(
            duration("PT1H30M", 1).unwrap(),
            SignedDuration::from_mins(90)
        );
        assert_eq!(duration("P1D", 1).unwrap(), SignedDuration::from_hours(24));
        assert_eq!(duration("P1W", 1).unwrap(), SignedDuration::from_hours(168));
        assert!(duration("1H", 1).is_err());
        assert!(duration("PT1X", 1).is_err());
    }

    #[test]
    fn an_event_without_start_says_the_line() {
        let error = parse("BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:x\nEND:VEVENT\n").unwrap_err();
        assert_eq!(
            error.to_string(),
            "o ICS tem erro na linha 2: o evento não tem DTSTART"
        );
    }

    #[test]
    fn a_broken_date_says_what_it_expected() {
        let error = parse("BEGIN:VEVENT\nUID:x\nDTSTART:amanhã\nEND:VEVENT\n").unwrap_err();
        assert!(error.to_string().contains("linha 3"), "{error}");
    }
}
