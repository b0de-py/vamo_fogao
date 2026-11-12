//! A agenda no binário: liga a config, o link ICS e a cópia local ao
//! leitor do `vamo-agenda`.

use color_eyre::eyre::{Result, WrapErr, eyre};
use jiff::ToSpan;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use vamo_agenda::{Calendar, Freshness, IcsLink};
use vamo_core::{Appointment, Projection, TimeRules, When};

use crate::config::AgendaConfig;
use crate::paths;

/// A agenda lida, e de onde ela veio.
pub(crate) struct Loaded {
    pub(crate) calendar: Calendar,
    pub(crate) freshness: Freshness,
}

/// Baixa a agenda da config (ou usa a cópia local, sem internet).
/// `Ok(None)` quando a config não tem `[agenda]`.
pub(crate) fn load(config: &AgendaConfig) -> Result<Option<Loaded>> {
    let Some(url) = &config.ics else {
        return Ok(None);
    };
    let link = IcsLink::new(url, &paths::agenda_cache()?);
    let (text, freshness) = link.fetch()?;
    let calendar = Calendar::parse(&text, TimeZone::system())
        .wrap_err("a agenda baixada não é um ICS que o vamo leia")?;
    Ok(Some(Loaded {
        calendar,
        freshness,
    }))
}

/// As regras do tempo que uma tarefa marcada na agenda fere, com o ID na
/// frente: `T-041: foco depois do teto da noite (22:00)`.
pub(crate) fn violations(
    appointment: &Appointment,
    projection: &Projection,
    rules: &TimeRules,
) -> Vec<String> {
    let (Some(id), Some((start, end))) = (appointment.task, appointment.span()) else {
        return Vec::new();
    };
    let Some(task) = projection.task(id) else {
        return Vec::new();
    };
    rules
        .check(&task, start, end)
        .iter()
        .map(|violation| format!("{id}: {violation}"))
        .collect()
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
