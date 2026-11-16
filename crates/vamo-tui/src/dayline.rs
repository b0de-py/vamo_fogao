//! A linha do dia: uma linha fina, logo abaixo das abas, com as horas, os
//! compromissos, os blocos protegidos, as brechas e a marca de agora.
//!
//! ```text
//! 08───────09━━━━━━━10───────11───────12╌╌╌╌╌╌╌13───┃───14···
//! ```
//!
//! O traço fino é brecha, o grosso é compromisso, o tracejado é bloco
//! protegido e os pontos são tempo fora do expediente. As horas vão
//! escritas na própria linha, e a barra de pé é agora.
//!
//! A linha vai do começo do expediente até o teto da noite da config (sem
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
            Self::Busy => ("━", theme::heading()),
            Self::Protected => ("╌", theme::muted()),
            Self::Gap => ("─", Style::new().fg(theme::ACCENT)),
            Self::Idle => ("·", theme::line()),
        }
    }
}

/// A legenda da linha do dia, pra ajuda: cada traço e o que ele quer
/// dizer.
pub(crate) const LEGEND: &str = "━ compromisso · ─ brecha · ╌ protegido · ┃ agora";

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
/// encostar no outro: cada rótulo precisa de pelo menos 6 colunas, as duas
/// do número e quatro de linha.
fn label_step(hours: usize, width: usize) -> usize {
    [1, 2, 3, 4, 6, 12]
        .into_iter()
        .find(|step| width * step >= hours * 6)
        .unwrap_or(24)
}

/// A linha do dia, com `width` colunas. `None` quando não tem expediente
/// hoje ou a tela é estreita demais.
pub(crate) fn line(schedule: &Schedule, today: Date, width: usize) -> Option<Line<'static>> {
    let (start, end) = range(schedule, today)?;
    if width < MIN_WIDTH {
        return None;
    }
    let total = start.duration_until(end).as_secs();
    // Larguras de tela e segundos de um dia cabem com folga num i64.
    let columns = i64::try_from(width).unwrap_or(i64::MAX);
    // O instante do meio da célula `column`.
    let middle = |column: i64| {
        let offset = SignedDuration::from_secs(total * (2 * column + 1) / (2 * columns));
        start.checked_add(offset).unwrap_or(end)
    };
    // As brechas do dia inteiro: a linha mostra também as que já passaram.
    let found = gaps(
        today,
        &schedule.rules,
        &schedule.appointments,
        schedule.min_gap,
        None,
    );
    let mut cells: Vec<Span<'static>> = (0..columns)
        .map(|column| {
            let (symbol, style) = slot(schedule, today, &found, middle(column)).cell();
            Span::styled(symbol, style)
        })
        .collect();

    // As horas, escritas na própria linha, cada uma na célula onde começa.
    let hours = usize::try_from(total / 3600).unwrap_or(0);
    let step = label_step(hours, width);
    for hour in (0..=hours).step_by(step) {
        let at = start
            .checked_add(SignedDuration::from_hours(i64::try_from(hour).unwrap_or(0)))
            .unwrap_or(end);
        let text = format!("{:02}", at.hour());
        // A última hora, que cairia fora da tela, encosta na borda.
        let column = (hour * width / hours.max(1)).min(width.saturating_sub(text.len()));
        for (offset, digit) in text.chars().enumerate() {
            cells[column + offset] = Span::styled(digit.to_string(), theme::soft());
        }
    }

    // A marca de agora entra por último, por cima do que estiver na célula.
    let now = schedule
        .now
        .map(|now| today.to_datetime(now))
        .filter(|&now| start <= now && now < end)
        .and_then(|now| {
            usize::try_from(start.duration_until(now).as_secs() * columns / total).ok()
        });
    if let Some(cell) = now.and_then(|column| cells.get_mut(column)) {
        *cell = Span::styled("┃", theme::selected());
    }
    Some(Line::from(cells))
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

    fn drawn(schedule: &Schedule, day: Date, width: usize) -> String {
        line(schedule, day, width)
            .map(|line| text(&line))
            .unwrap_or_default()
    }

    #[test]
    fn the_line_shows_the_day_two_cells_per_hour() {
        let (schedule, day) = example(None);
        // 14 horas (das 8 às 22) em 28 colunas: meia hora por célula. A
        // reunião das 9 às 10, o almoço protegido e o resto do expediente
        // em brecha; depois das 18, fora do expediente.
        assert_eq!(drawn(&schedule, day, 28), "08━━──11╌╌──14────17····20··");
    }

    #[test]
    fn now_takes_its_cell() {
        let (schedule, day) = example(Some(time(15, 10, 0, 0)));
        assert_eq!(drawn(&schedule, day, 28), "08━━──11╌╌──14┃───17····20··");
        // Fora da linha, agora não aparece.
        let (schedule, day) = example(Some(time(23, 0, 0, 0)));
        assert!(!drawn(&schedule, day, 28).contains('┃'));
    }

    #[test]
    fn a_cap_after_midnight_belongs_to_the_next_dawn() {
        let (mut schedule, day) = example(None);
        schedule.rules.night_cap = Some(time(1, 30, 0, 0));
        // Das 8 às 2 da manhã (o teto arredondado pra cima): 18 horas.
        let line = drawn(&schedule, day, 36);
        assert!(line.starts_with("08") && line.ends_with("02"), "{line}");
    }

    #[test]
    fn without_a_workday_there_is_no_line() {
        let (mut schedule, day) = example(None);
        assert_eq!(line(&schedule, day, MIN_WIDTH - 1), None);
        schedule.rules.workday = None;
        assert_eq!(line(&schedule, day, 80), None);
    }

    proptest! {
        /// A linha tem sempre a largura pedida, em qualquer tela e com
        /// qualquer teto.
        #[test]
        fn the_line_has_the_asked_width(
            width in MIN_WIDTH..240,
            cap_hour in 0i8..24,
            cap_minute in 0i8..60,
            now_hour in 0i8..24,
        ) {
            let (mut schedule, day) = example(Some(time(now_hour, 0, 0, 0)));
            schedule.rules.night_cap = Some(time(cap_hour, cap_minute, 0, 0));
            let line = line(&schedule, day, width).expect("o exemplo tem expediente");
            prop_assert_eq!(line.width(), width);
        }
    }
}
