//! As brechas do dia: o contra-ataque. As janelas livres do expediente, e as
//! tarefas de brecha que cabem em cada uma.

use jiff::SignedDuration;
use jiff::civil::{Date, DateTime, Time};

use crate::{Appointment, Fit, Status, Task, TimeRules};

/// O tamanho mínimo padrão de uma brecha: 15 minutos.
pub const DEFAULT_MIN_GAP: SignedDuration = SignedDuration::from_mins(15);

/// Uma janela livre no expediente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gap {
    /// O começo.
    pub start: DateTime,
    /// O fim.
    pub end: DateTime,
}

impl Gap {
    /// O tamanho da brecha.
    pub fn length(self) -> SignedDuration {
        self.start.duration_until(self.end)
    }
}

/// As brechas de `day`: o expediente menos os compromissos com horário e os
/// blocos protegidos, só as janelas de pelo menos `min`. Com `not_before`,
/// nada começa antes dessa hora (hoje, a brecha que já passou não serve).
///
/// Sem expediente na config, ou num dia fora dele, não tem brecha: a
/// brecha é do expediente.
///
/// ```
/// use jiff::civil::{date, time};
/// use vamo_core::{Appointment, Block, TimeRules, When, gaps, DEFAULT_MIN_GAP};
///
/// let day = date(2026, 11, 12);
/// let rules = TimeRules {
///     workday: Some(Block {
///         name: "expediente".to_owned(),
///         hours: "09:00-12:00".parse()?,
///         days: vec![],
///     }),
///     ..TimeRules::default()
/// };
/// let meeting = Appointment {
///     uid: "reuniao@exemplo".to_owned(),
///     title: "Reunião".to_owned(),
///     when: When::Timed { start: day.at(10, 0, 0, 0), end: day.at(11, 0, 0, 0) },
///     recurring: false,
///     task: None,
/// };
/// let found = gaps(day, &rules, &[meeting], DEFAULT_MIN_GAP, None);
/// assert_eq!(found.len(), 2);
/// assert_eq!(found[0].start.time(), time(9, 0, 0, 0));
/// assert_eq!(found[1].end.time(), time(12, 0, 0, 0));
/// # Ok::<(), vamo_core::Error>(())
/// ```
pub fn gaps(
    day: Date,
    rules: &TimeRules,
    agenda: &[Appointment],
    min: SignedDuration,
    not_before: Option<Time>,
) -> Vec<Gap> {
    let Some(workday) = &rules.workday else {
        return Vec::new();
    };
    if !workday.applies_on(day.weekday()) {
        return Vec::new();
    }
    let mut open = day.to_datetime(workday.hours.start());
    let close = day.to_datetime(workday.hours.end());
    if let Some(now) = not_before {
        open = open.max(day.to_datetime(now));
    }

    // O que ocupa o dia: compromisso com horário e bloco protegido.
    let mut busy: Vec<(DateTime, DateTime)> = agenda.iter().filter_map(Appointment::span).collect();
    busy.extend(
        rules
            .protected
            .iter()
            .filter(|block| block.applies_on(day.weekday()))
            .map(|block| {
                (
                    day.to_datetime(block.hours.start()),
                    day.to_datetime(block.hours.end()),
                )
            }),
    );
    busy.sort_unstable();

    let mut found = Vec::new();
    let mut cursor = open;
    for (start, end) in busy {
        if end <= cursor {
            continue;
        }
        if start >= close {
            break;
        }
        push(&mut found, cursor, start.min(close), min);
        cursor = cursor.max(end);
    }
    push(&mut found, cursor, close, min);
    found
}

/// Guarda a janela se ela tiver pelo menos o tamanho mínimo.
fn push(found: &mut Vec<Gap>, start: DateTime, end: DateTime, min: SignedDuration) {
    if end > start && start.duration_until(end) >= min {
        found.push(Gap { start, end });
    }
}

/// As tarefas de brecha que cabem na janela: prontas, sem dia marcado (ou
/// marcadas pra `day`), fora da agenda e que não passam do tamanho dela.
/// Tarefa sem estimativa cabe em qualquer brecha: brecha é coisa rápida por
/// definição. A que já tem horário na `agenda` não é sugerida de novo.
pub fn fitting<'t>(
    gap: Gap,
    day: Date,
    tasks: &'t [Task],
    agenda: &[Appointment],
) -> Vec<&'t Task> {
    tasks
        .iter()
        .filter(|task| !agenda.iter().any(|event| event.task == Some(task.id())))
        .filter(|task| task.fit() == Fit::Gap)
        .filter(|task| matches!(task.status(), Status::Ready | Status::Scheduled))
        .filter(|task| task.day().is_none_or(|planned| planned == day))
        .filter(|task| {
            task.estimate()
                .is_none_or(|estimate| estimate <= gap.length())
        })
        .collect()
}

/// A estimativa de tempo escrita nas notas: `~30 min`, `~1 h`, `~1h30`,
/// `~2 h por pessoa`. Vale a primeira que aparecer.
pub(crate) fn estimate(notes: &str) -> Option<SignedDuration> {
    let (_, after) = notes.split_once('~')?;
    let after = after.trim_start();
    let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
    let amount: i64 = digits.parse().ok()?;
    let rest = after[digits.len()..].trim_start();
    if rest.starts_with("min") {
        return Some(SignedDuration::from_mins(amount));
    }
    let rest = rest.strip_prefix('h')?;
    let minutes: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let minutes: i64 = if minutes.is_empty() {
        0
    } else {
        minutes.parse().ok()?
    };
    Some(SignedDuration::from_mins(amount * 60 + minutes))
}

#[cfg(test)]
mod tests {
    use jiff::civil::{date, time};
    use proptest::prelude::*;

    use super::*;
    use crate::{Block, TaskId, When};

    const DAY: Date = date(2026, 11, 12);

    fn rules(protected: Vec<Block>) -> TimeRules {
        TimeRules {
            workday: Some(Block {
                name: "expediente".to_owned(),
                hours: "09:00-18:00".parse().unwrap(),
                days: vec![],
            }),
            night_cap: None,
            protected,
        }
    }

    fn event(start: (i8, i8), end: (i8, i8)) -> Appointment {
        Appointment {
            uid: format!("{start:?}"),
            title: "Evento".to_owned(),
            when: When::Timed {
                start: DAY.at(start.0, start.1, 0, 0),
                end: DAY.at(end.0, end.1, 0, 0),
            },
            recurring: false,
            task: None,
        }
    }

    fn spans(found: &[Gap]) -> Vec<String> {
        found
            .iter()
            .map(|gap| {
                format!(
                    "{}-{}",
                    gap.start.strftime("%H:%M"),
                    gap.end.strftime("%H:%M")
                )
            })
            .collect()
    }

    #[test]
    fn events_and_lunch_cut_the_workday() {
        let lunch = Block {
            name: "Almoço".to_owned(),
            hours: "12:00-13:00".parse().unwrap(),
            days: vec![],
        };
        let agenda = [
            event((10, 0), (11, 0)),
            event((10, 30), (11, 15)),
            event((17, 50), (19, 0)),
        ];
        assert_eq!(
            spans(&gaps(
                DAY,
                &rules(vec![lunch]),
                &agenda,
                DEFAULT_MIN_GAP,
                None
            )),
            ["09:00-10:00", "11:15-12:00", "13:00-17:50"]
        );
    }

    #[test]
    fn short_windows_and_the_past_are_left_out() {
        let agenda = [event((9, 10), (12, 0))];
        assert_eq!(
            spans(&gaps(
                DAY,
                &rules(vec![]),
                &agenda,
                DEFAULT_MIN_GAP,
                Some(time(14, 0, 0, 0))
            )),
            ["14:00-18:00"]
        );
        // As 9h às 9h10 têm 10 minutos, menos que o mínimo.
        assert_eq!(
            spans(&gaps(DAY, &rules(vec![]), &agenda, DEFAULT_MIN_GAP, None)),
            ["12:00-18:00"]
        );
    }

    #[test]
    fn no_workday_no_gaps() {
        assert_eq!(
            gaps(DAY, &TimeRules::default(), &[], DEFAULT_MIN_GAP, None),
            []
        );
        let weekend = Block {
            name: "expediente".to_owned(),
            hours: "09:00-18:00".parse().unwrap(),
            days: vec![jiff::civil::Weekday::Monday],
        };
        let rules = TimeRules {
            workday: Some(weekend),
            ..TimeRules::default()
        };
        assert_eq!(gaps(DAY, &rules, &[], DEFAULT_MIN_GAP, None), []);
    }

    #[test]
    fn estimates_are_read_from_the_notes() {
        assert_eq!(
            estimate("~30 min; compra pequena"),
            Some(SignedDuration::from_mins(30))
        );
        assert_eq!(
            estimate("~1 h; depois de triar"),
            Some(SignedDuration::from_mins(60))
        );
        assert_eq!(estimate("~1h30"), Some(SignedDuration::from_mins(90)));
        assert_eq!(
            estimate("~2 h por pessoa"),
            Some(SignedDuration::from_mins(120))
        );
        assert_eq!(estimate("sem pressa"), None);
        assert_eq!(estimate("~ algumas horas"), None);
    }

    #[test]
    fn only_quick_ready_tasks_that_fit_are_suggested() {
        let make = |n: u32, fit: Fit, notes: &str| {
            let mut task = Task::new(
                TaskId::new(n).unwrap(),
                "Comprar o gesso".parse().unwrap(),
                "casa".parse().unwrap(),
                fit,
                Status::Ready,
            );
            task.set_notes(notes);
            task
        };
        let tasks = [
            make(1, Fit::Gap, "~30 min"),
            make(2, Fit::Gap, "~2 h"),
            make(3, Fit::Focus, "~20 min"),
            make(4, Fit::Gap, ""),
        ];
        let gap = Gap {
            start: DAY.at(9, 0, 0, 0),
            end: DAY.at(10, 0, 0, 0),
        };
        let booked = Appointment {
            task: Some(TaskId::new(4).unwrap()),
            ..event((15, 0), (15, 30))
        };
        let ids: Vec<u32> = fitting(gap, DAY, &tasks, &[])
            .iter()
            .map(|task| task.id().number())
            .collect();
        assert_eq!(ids, [1, 4]);
        let ids: Vec<u32> = fitting(gap, DAY, &tasks, &[booked])
            .iter()
            .map(|task| task.id().number())
            .collect();
        assert_eq!(ids, [1]);
    }

    fn agenda() -> impl Strategy<Value = Vec<Appointment>> {
        proptest::collection::vec((0_i8..24, 0_i8..60, 1_i64..300), 0..8).prop_map(|events| {
            events
                .into_iter()
                .map(|(hour, minute, length)| {
                    let start = DAY.at(hour, minute, 0, 0);
                    let end = start.saturating_add(SignedDuration::from_mins(length));
                    Appointment {
                        uid: format!("{hour}:{minute}"),
                        title: "Evento".to_owned(),
                        when: When::Timed { start, end },
                        recurring: false,
                        task: None,
                    }
                })
                .collect()
        })
    }

    proptest! {
        #[test]
        fn a_gap_never_overlaps_an_event_nor_leaves_the_workday(
            agenda in agenda(),
            min in 1_i64..90,
        ) {
            let rules = rules(vec![]);
            let found = gaps(DAY, &rules, &agenda, SignedDuration::from_mins(min), None);
            let open = DAY.at(9, 0, 0, 0);
            let close = DAY.at(18, 0, 0, 0);
            for gap in &found {
                prop_assert!(gap.start >= open && gap.end <= close);
                prop_assert!(gap.length() >= SignedDuration::from_mins(min));
                for (start, end) in agenda.iter().filter_map(Appointment::span) {
                    prop_assert!(gap.end <= start || end <= gap.start);
                }
            }
            // E as brechas não se sobrepõem entre si.
            for pair in found.windows(2) {
                prop_assert!(pair[0].end <= pair[1].start);
            }
        }
    }
}
