//! A agenda lida de um ICS: os eventos, com a recorrência e as exceções
//! resolvidas, convertidos pro horário de quem usa.

use std::collections::{BTreeMap, HashSet};

use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp, ToSpan};
use vamo_core::{Agenda, Appointment, When};

use crate::ics::{self, Moment, RawEvent};
use crate::rrule::Rule;
use crate::{Error, Footer};

/// A agenda de um arquivo ICS, pronta pra responder o que tem em cada dia.
///
/// ```
/// use jiff::civil::date;
/// use jiff::tz::TimeZone;
/// use vamo_agenda::Calendar;
/// use vamo_core::Agenda;
///
/// let ics = "BEGIN:VCALENDAR\n\
///            BEGIN:VEVENT\n\
///            UID:caminhada@exemplo\n\
///            DTSTART:20261112T070000\n\
///            DTEND:20261112T074500\n\
///            SUMMARY:Caminhada\n\
///            RRULE:FREQ=DAILY\n\
///            END:VEVENT\n\
///            END:VCALENDAR\n";
/// let calendar = Calendar::parse(ics, TimeZone::UTC)?;
/// let week = calendar.appointments(date(2026, 11, 12), date(2026, 11, 18))?;
/// assert_eq!(week.len(), 7);
/// assert!(week.iter().all(|a| a.recurring));
/// # Ok::<(), vamo_agenda::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct Calendar {
    events: Vec<RawEvent>,
    zone: TimeZone,
}

impl Calendar {
    /// Lê o texto de um ICS. Os horários saem no fuso `zone`, o de quem usa.
    ///
    /// # Errors
    ///
    /// [`Error::Ics`] quando o texto não é um ICS que dê pra ler, com a
    /// linha do problema.
    pub fn parse(text: &str, zone: TimeZone) -> Result<Self, Error> {
        Ok(Self {
            events: ics::parse(text)?,
            zone,
        })
    }

    /// Os compromissos que tocam os dias de `from` a `to` (incluídos), em
    /// ordem, e os avisos: um evento com regra de recorrência que o leitor
    /// não conhece fica de fora, e o aviso diz qual foi.
    pub fn read(&self, from: Date, to: Date) -> (Vec<Appointment>, Vec<String>) {
        // As exceções de um evento recorrente vêm como eventos à parte, com
        // o mesmo UID e o RECURRENCE-ID da ocorrência que trocam (ou que
        // cancelam).
        let mut replaced: BTreeMap<&str, HashSet<Timestamp>> = BTreeMap::new();
        for event in &self.events {
            if let Some(moment) = &event.recurrence_id {
                let frame = self.frame(moment);
                if let Ok(instant) = instant(civil(moment, &frame), &frame) {
                    replaced.entry(&event.uid).or_default().insert(instant);
                }
            }
        }

        let mut found = Vec::new();
        let mut warnings = Vec::new();
        for event in self.events.iter().filter(|event| !event.cancelled) {
            match self.expand(event, from, to, replaced.get(event.uid.as_str())) {
                Ok(appointments) => found.extend(appointments),
                Err(warning) => warnings.push(warning),
            }
        }
        found.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)).then(a.title.cmp(&b.title)));
        (found, warnings)
    }

    /// As ocorrências de um evento que tocam a janela.
    fn expand(
        &self,
        event: &RawEvent,
        from: Date,
        to: Date,
        replaced: Option<&HashSet<Timestamp>>,
    ) -> Result<Vec<Appointment>, String> {
        let frame = self.frame(&event.start);
        let start = civil(&event.start, &frame);
        let length = length(event, start, &frame, &self.zone);
        let is_exception = event.recurrence_id.is_some();

        let starts = match (&event.rrule, is_exception) {
            (Some(rule), false) => {
                let rule = Rule::parse(rule).map_err(|error| {
                    format!(
                        "{error} (evento \"{}\", linha {})",
                        event.summary, event.line
                    )
                })?;
                let until = rule
                    .until
                    .as_deref()
                    .map(|text| until(text, &frame, event.line))
                    .transpose()?;
                rule.occurrences(start, to, until)
            }
            _ => vec![start],
        };

        let exdates: HashSet<Timestamp> = event
            .exdates
            .iter()
            .filter_map(|moment| {
                // EXDATE sem fuso próprio está no fuso do evento.
                let frame = match moment {
                    Moment::Zoned(..) => self.frame(moment),
                    _ => frame.clone(),
                };
                instant(civil(moment, &frame), &frame).ok()
            })
            .collect();

        let mut appointments = Vec::new();
        for occurrence in starts {
            let at = instant(occurrence, &frame)?;
            let removed =
                exdates.contains(&at) || replaced.is_some_and(|replaced| replaced.contains(&at));
            if !is_exception && removed {
                continue;
            }
            for when in self.when(&event.start, occurrence, length, &frame)? {
                let touches = match when {
                    When::AllDay(day) => from <= day && day <= to,
                    When::Timed { start, end } => start.date() <= to && end.date() >= from,
                };
                if touches {
                    appointments.push(Appointment {
                        uid: event.uid.clone(),
                        title: event.summary.trim().to_owned(),
                        when,
                        recurring: event.rrule.is_some() || is_exception,
                        task: Footer::find(&event.description).map(|footer| footer.task),
                    });
                }
            }
        }
        Ok(appointments)
    }

    /// O fuso em que um horário foi escrito: o do `TZID`, UTC, ou o de quem
    /// usa (pra data sem hora e hora sem fuso). Fuso com nome que não existe
    /// na base vira o de quem usa, com um aviso no log.
    fn frame(&self, moment: &Moment) -> TimeZone {
        match moment {
            Moment::Zoned(_, name) => jiff::tz::db().get(name).unwrap_or_else(|_| {
                tracing::warn!(zone = %name, "fuso desconhecido no ICS; usando o local");
                self.zone.clone()
            }),
            Moment::Utc(_) => TimeZone::UTC,
            Moment::Date(_) | Moment::Floating(_) => self.zone.clone(),
        }
    }

    /// O `when` de uma ocorrência, já no horário de quem usa. Evento de dia
    /// inteiro com vários dias vira um por dia.
    fn when(
        &self,
        start: &Moment,
        occurrence: DateTime,
        length: SignedDuration,
        frame: &TimeZone,
    ) -> Result<Vec<When>, String> {
        if let Moment::Date(_) = start {
            let days = (length.as_hours() / 24).max(1);
            return Ok((0..days)
                .filter_map(|n| occurrence.date().checked_add(n.days()).ok())
                .map(When::AllDay)
                .collect());
        }
        let begin = occurrence
            .to_zoned(frame.clone())
            .map_err(|error| error.to_string())?
            .with_time_zone(self.zone.clone());
        let end = begin
            .checked_add(length.max(SignedDuration::ZERO))
            .map_err(|error| error.to_string())?;
        Ok(vec![When::Timed {
            start: begin.datetime(),
            end: end.datetime(),
        }])
    }
}

impl Agenda for Calendar {
    type Error = Error;

    fn appointments(&self, from: Date, to: Date) -> Result<Vec<Appointment>, Error> {
        let (appointments, warnings) = self.read(from, to);
        for warning in warnings {
            tracing::warn!("{warning}");
        }
        Ok(appointments)
    }
}

/// Quanto o evento dura: o `DURATION`, a distância até o `DTEND` ou, sem
/// nenhum dos dois, um dia (evento de dia inteiro) ou nada (um lembrete).
fn length(event: &RawEvent, start: DateTime, frame: &TimeZone, local: &TimeZone) -> SignedDuration {
    if let Some(duration) = event.duration {
        return duration;
    }
    match (&event.start, &event.end) {
        (Moment::Date(first), Some(Moment::Date(last))) => {
            let days = i64::from((*last - *first).get_days()).max(1);
            SignedDuration::from_hours(24 * days)
        }
        (Moment::Date(_), _) => SignedDuration::from_hours(24),
        (_, Some(end)) => {
            let end_frame = match end {
                Moment::Zoned(_, name) => {
                    jiff::tz::db().get(name).unwrap_or_else(|_| local.clone())
                }
                Moment::Utc(_) => TimeZone::UTC,
                _ => frame.clone(),
            };
            match (
                instant(start, frame),
                instant(civil(end, &end_frame), &end_frame),
            ) {
                (Ok(a), Ok(b)) => b.duration_since(a),
                _ => SignedDuration::ZERO,
            }
        }
        (_, None) => SignedDuration::ZERO,
    }
}

/// A data e hora de um horário do ICS, no fuso dele.
fn civil(moment: &Moment, frame: &TimeZone) -> DateTime {
    match moment {
        Moment::Date(date) => date.to_datetime(jiff::civil::Time::midnight()),
        Moment::Floating(at) | Moment::Zoned(at, _) => *at,
        Moment::Utc(instant) => instant.to_zoned(frame.clone()).datetime(),
    }
}

/// O instante de uma data e hora num fuso.
fn instant(at: DateTime, frame: &TimeZone) -> Result<Timestamp, String> {
    at.to_zoned(frame.clone())
        .map(|zoned| zoned.timestamp())
        .map_err(|error| error.to_string())
}

/// O `UNTIL` da regra no horário do evento.
fn until(text: &str, frame: &TimeZone, line: usize) -> Result<DateTime, String> {
    let wrong = || format!("o UNTIL `{text}` da linha {line} não é uma data do ICS");
    if let Some(date) = ics::parse_date(text) {
        // Até aquele dia, inteiro.
        return Ok(date.at(23, 59, 59, 0));
    }
    match text.strip_suffix('Z') {
        Some(utc) => {
            let at = ics::parse_datetime(utc).ok_or_else(wrong)?;
            let zoned = at.to_zoned(TimeZone::UTC).map_err(|_| wrong())?;
            Ok(zoned.with_time_zone(frame.clone()).datetime())
        }
        None => ics::parse_datetime(text).ok_or_else(wrong),
    }
}

/// A ordem dos compromissos: no dia, os de dia inteiro primeiro, depois
/// pelo começo.
pub(crate) fn sort_key(appointment: &Appointment) -> (Date, Option<DateTime>) {
    match appointment.when {
        When::AllDay(day) => (day, None),
        When::Timed { start, .. } => (start.date(), Some(start)),
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    fn read(name: &str, zone: &str, from: Date, to: Date) -> Vec<Appointment> {
        let text = std::fs::read_to_string(format!(
            "{}/tests/fixtures/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let calendar = Calendar::parse(&text, jiff::tz::db().get(zone).unwrap()).unwrap();
        let (appointments, warnings) = calendar.read(from, to);
        assert_eq!(warnings, Vec::<String>::new());
        appointments
    }

    fn timed(appointment: &Appointment) -> String {
        match appointment.when {
            When::Timed { start, end } => format!(
                "{} {}-{} {}",
                start.date(),
                start.time().strftime("%H:%M"),
                end.time().strftime("%H:%M"),
                appointment.title
            ),
            When::AllDay(day) => format!("{day} dia inteiro {}", appointment.title),
        }
    }

    #[test]
    fn a_weekly_meeting_with_an_exception_and_a_skipped_week() {
        let week = read(
            "semana.ics",
            "America/Sao_Paulo",
            date(2026, 11, 16),
            date(2026, 11, 29),
        );
        let lines: Vec<String> = week.iter().map(timed).collect();
        assert_eq!(
            lines,
            [
                "2026-11-16 09:00-09:30 Reunião da equipe",
                "2026-11-18 09:00-09:30 Reunião da equipe",
                // A de sexta foi pra tarde, numa exceção.
                "2026-11-20 dia inteiro Feriado",
                "2026-11-20 14:00-14:30 Reunião da equipe (remarcada)",
                // A de segunda, 23, foi cancelada com EXDATE.
                "2026-11-25 09:00-09:30 Reunião da equipe",
                "2026-11-27 09:00-09:30 Reunião da equipe",
            ]
        );
        assert!(
            week.iter()
                .filter(|a| a.title.starts_with("Reunião"))
                .all(|a| a.recurring)
        );
        assert!(!week.iter().any(|a| a.title == "Evento cancelado"));
    }

    #[test]
    fn the_meeting_stays_at_nine_across_daylight_saving() {
        // Em Nova York, o horário de verão acaba em 1º de novembro de 2026.
        let days = read(
            "verao.ics",
            "America/New_York",
            date(2026, 10, 29),
            date(2026, 11, 3),
        );
        let lines: Vec<String> = days.iter().map(timed).collect();
        assert_eq!(
            lines,
            [
                "2026-10-29 09:00-10:00 Daily",
                "2026-10-30 09:00-10:00 Daily",
                "2026-11-02 09:00-10:00 Daily",
                "2026-11-03 09:00-10:00 Daily",
            ]
        );
    }

    #[test]
    fn the_time_moves_to_the_reader_zone() {
        // O mesmo evento das 9h de Nova York, lido em São Paulo: antes da
        // virada são 10h, depois 11h.
        let days = read(
            "verao.ics",
            "America/Sao_Paulo",
            date(2026, 10, 30),
            date(2026, 11, 2),
        );
        let lines: Vec<String> = days.iter().map(timed).collect();
        assert_eq!(
            lines,
            [
                "2026-10-30 10:00-11:00 Daily",
                "2026-11-02 11:00-12:00 Daily",
            ]
        );
    }

    #[test]
    fn a_trip_of_three_days_shows_on_each_day() {
        let days = read(
            "semana.ics",
            "America/Sao_Paulo",
            date(2026, 12, 1),
            date(2026, 12, 31),
        );
        let trip: Vec<Date> = days
            .iter()
            .filter(|a| a.title == "Viagem")
            .map(Appointment::day)
            .collect();
        assert_eq!(
            trip,
            [date(2026, 12, 4), date(2026, 12, 5), date(2026, 12, 6)]
        );
    }

    #[test]
    fn an_unknown_rule_is_a_warning_not_an_error() {
        let text = "BEGIN:VEVENT\nUID:x\nDTSTART:20261112T090000\nSUMMARY:Estranho\n\
                    RRULE:FREQ=MONTHLY;BYSETPOS=-1;BYDAY=MO,TU\nEND:VEVENT\n\
                    BEGIN:VEVENT\nUID:y\nDTSTART:20261112T100000\nSUMMARY:Normal\nEND:VEVENT\n";
        let calendar = Calendar::parse(text, TimeZone::UTC).unwrap();
        let (appointments, warnings) = calendar.read(date(2026, 11, 12), date(2026, 11, 12));
        assert_eq!(appointments.len(), 1);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("BYSETPOS"), "{}", warnings[0]);
    }
}
