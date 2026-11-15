//! A linha do dia: uma régua das horas com os compromissos, os blocos
//! protegidos, as brechas e a marca de agora.
//!
//! ```text
//! 08    10    12    14    16    18    20    22    00
//! ░░░░░████░░░░▓▓▓▓░░░░░░░██████░░░   ┃
//! ```
//!
//! A régua vai do começo do expediente até o teto da noite da config (sem
//! teto, até o fim do expediente). Quem acha as brechas é o core
//! ([`vamo_core::gaps`]); aqui cada célula só pergunta o que tem naquele
//! instante e desenha.

use jiff::civil::{Date, DateTime};
use jiff::{SignedDuration, ToSpan};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use vamo_core::{Appointment, gaps};

use crate::schedule::Schedule;
use crate::theme;

/// A menor régua que ainda diz alguma coisa, em colunas.
const MIN_WIDTH: usize = 24;

/// O que tem num instante do dia, da prioridade maior pra menor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    /// Um compromisso com horário.
    Busy,
    /// Um bloco protegido da config: não é hora de tarefa.
    Protected,
    /// Uma brecha do expediente.
    Gap,
    /// Tempo livre fora do expediente, ou brecha pequena demais.
    Idle,
}

impl Slot {
    /// O caractere e a cor da célula.
    fn cell(self) -> (&'static str, Style) {
        match self {
            Self::Busy => ("█", theme::soft()),
            Self::Protected => ("▓", theme::muted()),
            Self::Gap => ("░", theme::selected()),
            Self::Idle => ("·", theme::line()),
        }
    }
}

/// A legenda da régua, pra ir do lado do rótulo da seção.
pub(crate) fn legend() -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (slot, name) in [
        (Slot::Busy, "compromisso"),
        (Slot::Gap, "brecha"),
        (Slot::Protected, "protegido"),
    ] {
        let (symbol, style) = slot.cell();
        spans.push(Span::styled(format!("   {symbol} "), style));
        spans.push(Span::styled(name, theme::muted()));
    }
    spans.push(Span::styled("   ┃ ", theme::heading()));
    spans.push(Span::styled("agora", theme::muted()));
    spans
}

/// O começo e o fim da régua em `today`, em horas cheias. `None` sem
/// expediente na config ou num dia fora dele.
fn range(schedule: &Schedule, today: Date) -> Option<(DateTime, DateTime)> {
    let workday = schedule.rules.workday.as_ref()?;
    if !workday.applies_on(today.weekday()) {
        return None;
    }
    let start = today.at(workday.hours.start().hour(), 0, 0, 0);
    let end = match schedule.rules.night_cap {
        // Um teto que não vem depois do começo do expediente (01:30, por
        // exemplo) é da madrugada seguinte.
        Some(cap) if cap <= workday.hours.start() => today.tomorrow().ok()?.to_datetime(cap),
        Some(cap) => today.to_datetime(cap),
        None => today.to_datetime(workday.hours.end()),
    };
    // Arredonda o fim pra hora cheia de cima, pra régua fechar num número.
    let end = if end.minute() == 0 && end.second() == 0 {
        end
    } else {
        end.date()
            .at(end.hour(), 0, 0, 0)
            .checked_add(1.hour())
            .ok()?
    };
    (end > start).then_some((start, end))
}

/// O que tem em `at`.
fn slot(schedule: &Schedule, today: Date, found: &[vamo_core::Gap], at: DateTime) -> Slot {
    let inside = |(start, end): (DateTime, DateTime)| start <= at && at < end;
    if schedule
        .appointments
        .iter()
        .filter_map(Appointment::span)
        .any(inside)
    {
        return Slot::Busy;
    }
    let protected = schedule
        .rules
        .protected
        .iter()
        .filter(|block| block.applies_on(today.weekday()))
        .map(|block| {
            (
                today.to_datetime(block.hours.start()),
                today.to_datetime(block.hours.end()),
            )
        })
        .any(inside);
    if protected {
        return Slot::Protected;
    }
    if found.iter().any(|gap| inside((gap.start, gap.end))) {
        return Slot::Gap;
    }
    Slot::Idle
}

/// De quantas em quantas horas a régua escreve a hora, pra um número não
/// encostar no outro: cada rótulo precisa de pelo menos 5 colunas.
fn label_step(hours: usize, width: usize) -> usize {
    [1, 2, 3, 4, 6, 12]
        .into_iter()
        .find(|step| width * step >= hours * 5)
        .unwrap_or(24)
}

/// As duas linhas da régua, com `width` colunas: as horas em cima e a faixa
/// embaixo. Vazio quando não tem expediente hoje ou a tela é estreita
/// demais.
pub(crate) fn lines(schedule: &Schedule, today: Date, width: usize) -> Vec<Line<'static>> {
    let Some((start, end)) = range(schedule, today) else {
        return Vec::new();
    };
    if width < MIN_WIDTH {
        return Vec::new();
    }
    let total = start.duration_until(end).as_secs();
    // Larguras de tela e segundos de um dia cabem com folga num i64.
    let columns = i64::try_from(width).unwrap_or(i64::MAX);
    // O instante do meio da célula `column`.
    let middle = |column: i64| {
        let offset = SignedDuration::from_secs(total * (2 * column + 1) / (2 * columns));
        start.checked_add(offset).unwrap_or(end)
    };
    // As brechas do dia inteiro: a régua mostra também as que já passaram.
    let found = gaps(
        today,
        &schedule.rules,
        &schedule.appointments,
        schedule.min_gap,
        None,
    );
    // A célula de agora, quando agora cai dentro da régua.
    let now = schedule
        .now
        .map(|now| today.to_datetime(now))
        .filter(|&now| start <= now && now < end)
        .map(|now| start.duration_until(now).as_secs() * columns / total);

    let strip: Vec<Span<'static>> = (0..columns)
        .map(|column| {
            if now == Some(column) {
                return Span::styled("┃", theme::heading());
            }
            let (symbol, style) = slot(schedule, today, &found, middle(column)).cell();
            Span::styled(symbol, style)
        })
        .collect();

    // As horas, cada uma em cima da célula onde ela começa.
    let hours = usize::try_from(total / 3600).unwrap_or(0);
    let step = label_step(hours, width);
    let mut labels = vec![' '; width];
    for hour in (0..=hours).step_by(step) {
        let column = hour * width / hours.max(1);
        let at = start
            .checked_add(SignedDuration::from_hours(i64::try_from(hour).unwrap_or(0)))
            .unwrap_or(end);
        let text = format!("{:02}", at.hour());
        // A última hora, que cairia fora da tela, encosta na borda.
        let column = column.min(width.saturating_sub(text.len()));
        for (offset, digit) in text.chars().enumerate() {
            labels[column + offset] = digit;
        }
    }
    vec![
        Line::styled(labels.into_iter().collect::<String>(), theme::muted()),
        Line::from(strip),
    ]
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Time, date, time};
    use proptest::prelude::*;
    use vamo_core::{Block, TimeRules, When};

    use super::*;

    /// Um dia inventado: expediente das 8 às 18, almoço protegido ao
    /// meio-dia, uma reunião das 9 às 10 e teto da noite às 22.
    fn example(now: Option<Time>) -> (Schedule, Date) {
        let day = date(2026, 11, 11);
        let block = |name: &str, hours: &str| Block {
            name: name.to_owned(),
            hours: hours.parse().expect("o horário do exemplo é válido"),
            days: vec![],
        };
        let schedule = Schedule {
            appointments: vec![Appointment {
                uid: "reuniao@exemplo".to_owned(),
                title: "Reunião".to_owned(),
                when: When::Timed {
                    start: day.at(9, 0, 0, 0),
                    end: day.at(10, 0, 0, 0),
                },
                recurring: false,
                task: None,
            }],
            rules: TimeRules {
                workday: Some(block("expediente", "08:00-18:00")),
                night_cap: Some(time(22, 0, 0, 0)),
                protected: vec![block("Almoço", "12:00-13:00")],
            },
            min_gap: vamo_core::DEFAULT_MIN_GAP,
            now,
            ..Schedule::default()
        };
        (schedule, day)
    }

    fn text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn the_ruler_shows_the_day_two_cells_per_hour() {
        let (schedule, day) = example(None);
        // 14 horas (das 8 às 22) em 28 colunas: meia hora por célula.
        let lines = lines(&schedule, day, 28);
        assert_eq!(text(&lines[0]), "08    11    14    17    20  ");
        assert_eq!(text(&lines[1]), "░░██░░░░▓▓░░░░░░░░░░········");
    }

    #[test]
    fn now_takes_its_cell() {
        let (schedule, day) = example(Some(time(15, 10, 0, 0)));
        let lines = lines(&schedule, day, 28);
        assert_eq!(text(&lines[1]), "░░██░░░░▓▓░░░░┃░░░░░········");
        // Fora da régua, agora não aparece.
        let (schedule, day) = example(Some(time(23, 0, 0, 0)));
        assert!(!text(&lines_of(&schedule, day)).contains('┃'));
    }

    fn lines_of(schedule: &Schedule, day: Date) -> Line<'static> {
        lines(schedule, day, 28).swap_remove(1)
    }

    #[test]
    fn a_cap_after_midnight_belongs_to_the_next_dawn() {
        let (mut schedule, day) = example(None);
        schedule.rules.night_cap = Some(time(1, 30, 0, 0));
        // Das 8 às 2 da manhã (o teto arredondado pra cima): 18 horas.
        let lines = lines(&schedule, day, 36);
        assert!(text(&lines[0]).starts_with("08"));
        assert!(text(&lines[0]).ends_with("02"));
    }

    #[test]
    fn without_a_workday_there_is_no_ruler() {
        let (mut schedule, day) = example(None);
        assert_eq!(lines(&schedule, day, MIN_WIDTH - 1).len(), 0);
        schedule.rules.workday = None;
        assert_eq!(lines(&schedule, day, 80).len(), 0);
    }

    proptest! {
        /// As duas linhas têm sempre a largura pedida, em qualquer tela e
        /// com qualquer teto.
        #[test]
        fn both_lines_have_the_asked_width(
            width in MIN_WIDTH..240,
            cap_hour in 0i8..24,
            cap_minute in 0i8..60,
            now_hour in 0i8..24,
        ) {
            let (mut schedule, day) = example(Some(time(now_hour, 0, 0, 0)));
            schedule.rules.night_cap = Some(time(cap_hour, cap_minute, 0, 0));
            for line in lines(&schedule, day, width) {
                prop_assert_eq!(line.width(), width);
            }
        }
    }
}
