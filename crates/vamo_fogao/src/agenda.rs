//! A agenda no binário: liga a config, o link ICS e a cópia local ao
//! leitor do `vamo-agenda`.

use color_eyre::eyre::{Result, WrapErr, eyre};
use jiff::civil::Date;
use jiff::tz::TimeZone;
use vamo_agenda::{Calendar, Freshness, IcsLink};
use vamo_core::{Appointment, When};

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

/// `vamo agenda`: os compromissos do dia, baixados agora.
pub(crate) fn show(config: &AgendaConfig, day: Date) -> Result<String> {
    let loaded = load(config)?.ok_or_else(|| {
        eyre!(
            "a agenda não está configurada: ponha o link ICS na config, numa seção \
             [agenda] com ics = \"https://...\""
        )
    })?;
    let (appointments, warnings) = loaded.calendar.read(day, day);
    let mut lines: Vec<String> = warnings.iter().map(|w| format!("aviso: {w}")).collect();
    if loaded.freshness == Freshness::Cached {
        lines.push("aviso: a agenda não baixou agora; esta é a cópia local".to_owned());
    }
    if appointments.is_empty() {
        lines.push(format!("Nada na agenda em {}.", day.strftime("%d/%m")));
    } else {
        lines.extend(appointments.iter().map(|a| line(a, day)));
    }
    Ok(lines.join("\n"))
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
