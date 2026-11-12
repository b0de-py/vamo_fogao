//! As regras do tempo: o impedimento. Horários que a config define e o app
//! respeita.
//!
//! - **Expediente:** a janela do dia em que as brechas são procuradas.
//! - **Teto da noite:** depois dele, nenhuma tarefa de foco. Pode ser à
//!   noite (`22:00`) ou já de madrugada (`01:30`).
//! - **Blocos protegidos:** horários que não são de tarefa (almoço, buscar
//!   na escola, academia).
//!
//! Tudo aqui é puro: as regras chegam por parâmetro, a hora também.

use std::fmt;
use std::str::FromStr;

use jiff::civil::{DateTime, Time, Weekday};

use crate::{Error, Fit, Task};

/// Um horário do dia, de um começo até um fim no mesmo dia.
///
/// ```
/// use vamo_core::TimeRange;
///
/// let lunch: TimeRange = "12:00-13:30".parse()?;
/// assert_eq!(lunch.to_string(), "12:00–13:30");
/// assert!("13:00-12:00".parse::<TimeRange>().is_err());
/// # Ok::<(), vamo_core::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    start: Time,
    end: Time,
}

impl TimeRange {
    /// O horário de `start` a `end`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidTimeRange`] se o fim não vier depois do começo.
    pub fn new(start: Time, end: Time) -> Result<Self, Error> {
        if end <= start {
            return Err(Error::InvalidTimeRange {
                input: format!("{}-{}", hm(start), hm(end)),
            });
        }
        Ok(Self { start, end })
    }

    /// O começo.
    pub fn start(self) -> Time {
        self.start
    }

    /// O fim.
    pub fn end(self) -> Time {
        self.end
    }
}

impl fmt::Display for TimeRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}–{}", hm(self.start), hm(self.end))
    }
}

impl FromStr for TimeRange {
    type Err = Error;

    /// `09:00-18:00` (o traço pode ser o comprido, `–`).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let wrong = || Error::InvalidTimeRange {
            input: s.to_owned(),
        };
        let (start, end) = s.split_once(['-', '–']).ok_or_else(wrong)?;
        let start = parse_time(start).ok_or_else(wrong)?;
        let end = parse_time(end).ok_or_else(wrong)?;
        Self::new(start, end).map_err(|_| wrong())
    }
}

/// Uma hora do dia como se escreve: `9:00`, `09:00`, `22h`, `22h30`.
///
/// ```
/// use jiff::civil::time;
///
/// assert_eq!(vamo_core::time_of_day("22h30")?, time(22, 30, 0, 0));
/// assert!(vamo_core::time_of_day("meia-noite").is_err());
/// # Ok::<(), vamo_core::Error>(())
/// ```
///
/// # Errors
///
/// [`Error::InvalidTime`] quando o texto não é uma hora do dia.
pub fn time_of_day(text: &str) -> Result<Time, Error> {
    parse_time(text).ok_or_else(|| Error::InvalidTime {
        input: text.to_owned(),
    })
}

fn parse_time(text: &str) -> Option<Time> {
    let text = text.trim();
    let (hour, minute) = text
        .split_once([':', 'h'])
        .map_or((text, ""), |(h, m)| (h, m));
    let hour: i8 = hour.parse().ok()?;
    let minute: i8 = if minute.is_empty() {
        0
    } else {
        minute.parse().ok()?
    };
    Time::new(hour, minute, 0, 0).ok()
}

/// `09:00`.
fn hm(time: Time) -> String {
    time.strftime("%H:%M").to_string()
}

/// Um horário que se repete em certos dias da semana.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// O nome, pra dizer o motivo ("Almoço").
    pub name: String,
    /// O horário.
    pub hours: TimeRange,
    /// Os dias em que vale. Vazio é todo dia.
    pub days: Vec<Weekday>,
}

impl Block {
    /// Se o bloco vale nesse dia da semana.
    pub fn applies_on(&self, weekday: Weekday) -> bool {
        self.days.is_empty() || self.days.contains(&weekday)
    }
}

/// As regras do tempo de quem usa. Sem config, não tem regra nenhuma.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimeRules {
    /// O expediente: onde as brechas são procuradas. O nome do bloco não é
    /// usado.
    pub workday: Option<Block>,
    /// O teto da noite: tarefa de foco não passa dele.
    pub night_cap: Option<Time>,
    /// Os blocos protegidos, que não são de tarefa.
    pub protected: Vec<Block>,
}

/// Uma regra que um horário marcado fere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// Tarefa de foco que passa do teto da noite.
    AfterNightCap {
        /// O teto.
        cap: Time,
    },
    /// Tarefa marcada em cima de um bloco protegido.
    Protected {
        /// O nome do bloco.
        name: String,
        /// O horário dele.
        hours: TimeRange,
    },
    /// Tarefa pesada (foco ou com alguém) perto do jogo, quando só cabe
    /// tarefa leve.
    NearGame {
        /// O começo do trecho perto do jogo.
        from: Time,
        /// O fim.
        to: Time,
    },
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AfterNightCap { cap } => {
                write!(f, "foco depois do teto da noite ({})", hm(*cap))
            }
            Self::Protected { name, hours } => {
                write!(f, "em cima do bloco protegido {name} ({hours})")
            }
            Self::NearGame { from, to } => {
                write!(f, "pesada perto do jogo ({}–{})", hm(*from), hm(*to))
            }
        }
    }
}

impl TimeRules {
    /// As regras que a tarefa fere se for feita de `start` a `end`.
    ///
    /// ```
    /// use jiff::civil::{date, time};
    /// use vamo_core::{Fit, Status, Task, TimeRules, Violation};
    ///
    /// let rules = TimeRules {
    ///     night_cap: Some(time(22, 0, 0, 0)),
    ///     ..TimeRules::default()
    /// };
    /// let task = Task::new(
    ///     "T-041".parse()?,
    ///     "Escrever o relatório".parse()?,
    ///     "trampo".parse()?,
    ///     Fit::Focus,
    ///     Status::Scheduled,
    /// );
    /// let night = date(2026, 11, 12);
    /// assert_eq!(
    ///     rules.check(&task, night.at(21, 30, 0, 0), night.at(23, 0, 0, 0)),
    ///     [Violation::AfterNightCap { cap: time(22, 0, 0, 0) }]
    /// );
    /// # Ok::<(), vamo_core::Error>(())
    /// ```
    pub fn check(&self, task: &Task, start: DateTime, end: DateTime) -> Vec<Violation> {
        let mut found = Vec::new();
        if let Some(cap) = self.night_cap
            && task.fit() == Fit::Focus
            && after_night_cap(cap, start, end)
        {
            found.push(Violation::AfterNightCap { cap });
        }
        for block in &self.protected {
            if !block.applies_on(start.weekday()) {
                continue;
            }
            let day = start.date();
            let block_start = day.to_datetime(block.hours.start());
            let block_end = day.to_datetime(block.hours.end());
            if start < block_end && block_start < end {
                found.push(Violation::Protected {
                    name: block.name.clone(),
                    hours: block.hours,
                });
            }
        }
        found
    }
}

/// O meio-dia, que separa o teto da noite (`22:00`, no mesmo dia) do teto
/// da madrugada (`01:30`, já no dia seguinte).
const NOON: Time = Time::constant(12, 0, 0, 0);

/// Até quando a madrugada ainda é a noite anterior, pro teto da madrugada.
const DAWN: Time = Time::constant(6, 0, 0, 0);

/// Se o horário de `start` a `end` passa do teto da noite.
///
/// O teto à noite (`22:00`) é do dia em que a tarefa começa, e passar da
/// meia-noite é passar dele. O teto antes do meio-dia (`01:30`) é da
/// madrugada que fecha a noite: a tarefa que começa de manhã ou à noite
/// tem até a 1h30 seguinte; a que começa de madrugada antes do teto tem
/// até ele; e a que começa entre o teto e as 6h já passou.
fn after_night_cap(cap: Time, start: DateTime, end: DateTime) -> bool {
    let day = start.date();
    if cap >= NOON {
        return end > day.to_datetime(cap);
    }
    let time = start.time();
    if time >= cap && time < DAWN {
        // Começou de madrugada, depois do teto: ainda é a noite anterior, e
        // já passou dela.
        return true;
    }
    let limit = if time < cap {
        day.to_datetime(cap)
    } else {
        day.tomorrow()
            .map_or(DateTime::MAX, |next| next.to_datetime(cap))
    };
    end > limit
}

#[cfg(test)]
mod tests {
    use jiff::civil::{date, time};

    use super::*;
    use crate::{Status, TaskId};

    fn task(fit: Fit) -> Task {
        Task::new(
            TaskId::new(1).unwrap(),
            "Escrever o relatório".parse().unwrap(),
            "trampo".parse().unwrap(),
            fit,
            Status::Scheduled,
        )
    }

    fn rules() -> TimeRules {
        TimeRules {
            workday: None,
            night_cap: Some(time(22, 0, 0, 0)),
            protected: vec![Block {
                name: "Almoço".to_owned(),
                hours: "12:00-13:00".parse().unwrap(),
                // 2026-11-12 é uma quinta.
                days: vec![Weekday::Monday, Weekday::Thursday],
            }],
        }
    }

    const DAY: jiff::civil::Date = date(2026, 11, 12);

    #[test]
    fn the_night_cap_is_only_for_focus() {
        let rules = rules();
        let (start, end) = (DAY.at(21, 0, 0, 0), DAY.at(22, 30, 0, 0));
        assert_eq!(rules.check(&task(Fit::Focus), start, end).len(), 1);
        assert_eq!(rules.check(&task(Fit::Gap), start, end), []);
        // Acabar em cima do teto vale.
        assert_eq!(
            rules.check(&task(Fit::Focus), start, DAY.at(22, 0, 0, 0)),
            []
        );
    }

    #[test]
    fn crossing_midnight_passes_the_cap() {
        let found = rules().check(
            &task(Fit::Focus),
            DAY.at(23, 30, 0, 0),
            date(2026, 11, 13).at(0, 30, 0, 0),
        );
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn a_cap_after_midnight_belongs_to_the_night_before() {
        let rules = TimeRules {
            night_cap: Some(time(1, 30, 0, 0)),
            ..TimeRules::default()
        };
        let next = date(2026, 11, 13);
        let focus = task(Fit::Focus);
        let hurts = |start, end| !rules.check(&focus, start, end).is_empty();
        assert!(!hurts(DAY.at(20, 0, 0, 0), DAY.at(23, 0, 0, 0)));
        assert!(!hurts(DAY.at(23, 0, 0, 0), next.at(1, 30, 0, 0)));
        assert!(hurts(DAY.at(23, 0, 0, 0), next.at(2, 0, 0, 0)));
        assert!(!hurts(DAY.at(0, 30, 0, 0), DAY.at(1, 0, 0, 0)));
        assert!(hurts(DAY.at(0, 30, 0, 0), DAY.at(2, 0, 0, 0)));
        assert!(hurts(DAY.at(2, 0, 0, 0), DAY.at(3, 0, 0, 0)));
        assert!(!hurts(DAY.at(9, 0, 0, 0), DAY.at(11, 0, 0, 0)));
        assert!(hurts(DAY.at(9, 0, 0, 0), next.at(2, 0, 0, 0)));
    }

    #[test]
    fn a_protected_block_counts_only_on_its_days() {
        let rules = rules();
        let found = rules.check(&task(Fit::Gap), DAY.at(12, 30, 0, 0), DAY.at(12, 45, 0, 0));
        assert_eq!(
            found.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["em cima do bloco protegido Almoço (12:00–13:00)"]
        );
        // Na sexta o almoço não é protegido.
        let friday = date(2026, 11, 13);
        assert_eq!(
            rules.check(
                &task(Fit::Gap),
                friday.at(12, 30, 0, 0),
                friday.at(12, 45, 0, 0)
            ),
            []
        );
        // Encostar no bloco não fere.
        assert_eq!(
            rules.check(&task(Fit::Gap), DAY.at(11, 0, 0, 0), DAY.at(12, 0, 0, 0)),
            []
        );
    }

    #[test]
    fn time_ranges_read_the_usual_ways() {
        for text in ["09:00-18:00", "9:00 - 18:00", "9h-18h", "09:00–18:00"] {
            let range: TimeRange = text.parse().unwrap();
            assert_eq!(range.to_string(), "09:00–18:00", "{text}");
        }
        for text in ["", "9", "18:00-09:00", "25:00-26:00", "09:00-09:00"] {
            assert!(text.parse::<TimeRange>().is_err(), "{text}");
        }
    }
}
