//! A agenda no binário: liga a config, o link ICS e a cópia local ao
//! leitor do `vamo-agenda`.

use color_eyre::eyre::{Result, WrapErr, eyre};
use jiff::ToSpan;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use vamo_agenda::{Calendar, Freshness, IcsLink};
use vamo_core::{Appointment, Projection, TimeRules, When};

use crate::config::{AgendaConfig, Config};
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

/// A agenda de hoje pra tela Hoje, lida só da cópia local: a TUI abre na
/// hora, sem esperar a internet. Quem atualiza a cópia é o `vamo sync` (e o
/// `vamo agenda`). Sem agenda na config, sem cópia ou com a cópia ilegível,
/// a tela mostra só as brechas do expediente, e o motivo vai pro log.
pub(crate) fn schedule(config: &Config, now: &jiff::Zoned) -> vamo_tui::Schedule {
    let today = now.date();
    let appointments = config
        .agenda
        .ics
        .as_deref()
        .and_then(|url| match paths::agenda_cache() {
            Ok(cache) => Some(IcsLink::new(url, &cache)),
            Err(error) => {
                tracing::warn!("sem pasta de cache pra agenda: {error}");
                None
            }
        })
        .and_then(|link| match link.cached() {
            Ok(text) => text,
            Err(error) => {
                tracing::warn!("a cópia local da agenda não abriu: {error}");
                None
            }
        })
        .and_then(
            |text| match Calendar::parse(&text, now.time_zone().clone()) {
                Ok(calendar) => Some(calendar.read(today, today).0),
                Err(error) => {
                    tracing::warn!("a cópia local da agenda não é um ICS legível: {error}");
                    None
                }
            },
        )
        .unwrap_or_default();
    vamo_tui::Schedule {
        fixed: vamo_core::fixed_activities(&appointments, &config.aliases),
        appointments,
        rules: config.rules.clone(),
        min_gap: config.min_gap.unwrap_or(vamo_core::DEFAULT_MIN_GAP),
        now: Some(now.time()),
    }
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
