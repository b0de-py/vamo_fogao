//! A rodada: o resumo de uma semana, pra coletiva de domingo.

use jiff::civil::{Date, Weekday};

use crate::{DaySummary, DayWindow, Event, HabitCount, TaskId, Unbeaten, Violation};

/// Um impedimento: uma tarefa marcada na agenda num horário que fere uma
/// regra do tempo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offside {
    /// O dia do compromisso.
    pub day: Date,
    /// A tarefa ligada ao compromisso pelo rodapé `vamo:`.
    pub task: TaskId,
    /// O título do compromisso na agenda.
    pub appointment: String,
    /// A regra ferida.
    pub violation: Violation,
}

/// O resumo de uma semana, de segunda a domingo: a súmula de cada dia, os
/// impedimentos, os hábitos somados e a invencibilidade no fim dela.
///
/// ```
/// use jiff::civil::date;
/// use vamo_core::{Unbeaten, WeekSummary};
///
/// let week = WeekSummary::close(&[], Vec::new(), Vec::new(), Vec::new(), Unbeaten::default());
/// assert_eq!(week.goals(), 0);
/// assert_eq!(WeekSummary::monday_of(date(2026, 11, 15)), date(2026, 11, 9));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeekSummary {
    /// A súmula de cada dia, em ordem. Os hábitos de cada dia ficam vazios:
    /// os da semana vêm somados em [`habits`](Self::habits).
    pub days: Vec<DaySummary>,
    /// Os impedimentos da semana, em ordem de dia.
    pub offside: Vec<Offside>,
    /// Os hábitos somados na semana.
    pub habits: Vec<HabitCount>,
    /// A invencibilidade no fim da semana.
    pub unbeaten: Unbeaten,
}

impl WeekSummary {
    /// Fecha a rodada: a súmula de cada um dos `days` (em ordem, cada um
    /// com o trecho do relógio dele) e o que vem de fora do log.
    pub fn close(
        events: &[Event],
        days: Vec<(Date, DayWindow)>,
        mut offside: Vec<Offside>,
        habits: Vec<HabitCount>,
        unbeaten: Unbeaten,
    ) -> Self {
        offside.sort_by_key(|offside| offside.day);
        Self {
            days: days
                .into_iter()
                .map(|(day, window)| DaySummary::close(events, day, window, Vec::new()))
                .collect(),
            offside,
            habits,
            unbeaten,
        }
    }

    /// A segunda-feira da semana de `day`. A semana vai de segunda a
    /// domingo, como na ISO 8601.
    ///
    /// # Panics
    ///
    /// Nunca na prática: só se `day` for um dos seis primeiros dias do
    /// calendário do jiff, no ano -9999.
    pub fn monday_of(day: Date) -> Date {
        let back = day.weekday().since(Weekday::Monday);
        day.checked_sub(jiff::ToSpan::days(back))
            .expect("voltar até seis dias não sai do calendário do jiff")
    }

    /// Os gols da semana.
    pub fn goals(&self) -> usize {
        self.days.iter().map(|day| day.goals.len()).sum()
    }

    /// As tarefas que voltaram pro banco na semana. A mesma tarefa adiada
    /// duas vezes conta duas.
    pub fn benched(&self) -> usize {
        self.days.iter().map(|day| day.benched.len()).sum()
    }

    /// As tarefas que venceram na semana.
    pub fn stoppage(&self) -> usize {
        self.days.iter().map(|day| day.stoppage.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::{date, time};

    use super::*;
    use crate::Status;
    use crate::summary::tests::{Log, window};

    #[test]
    fn the_week_starts_on_monday() {
        assert_eq!(WeekSummary::monday_of(date(2026, 11, 9)), date(2026, 11, 9));
        assert_eq!(
            WeekSummary::monday_of(date(2026, 11, 12)),
            date(2026, 11, 9)
        );
        assert_eq!(
            WeekSummary::monday_of(date(2026, 11, 15)),
            date(2026, 11, 9)
        );
        // Semana que atravessa o mês.
        assert_eq!(
            WeekSummary::monday_of(date(2026, 12, 3)),
            date(2026, 11, 30)
        );
    }

    #[test]
    fn the_week_adds_up_its_days() {
        let mut log = Log::new();
        let monday = date(2026, 11, 9);
        let tuesday = date(2026, 11, 10);
        let first = log.add("Trocar o filtro", Status::Ready);
        let second = log.add("Pagar a conta", Status::Ready);
        let third = log.add("Lavar o carro", Status::Ready);
        log.schedule(first, monday);
        log.schedule(second, monday);
        log.schedule(third, tuesday);
        log.at(monday, 10);
        log.finish(first, monday);
        log.at(monday, 20);
        log.schedule(second, tuesday);
        log.at(tuesday, 10);
        log.finish(second, tuesday);

        let days = monday
            .series(jiff::ToSpan::days(1))
            .take(7)
            .map(|day| (day, window(day)))
            .collect();
        let offside = vec![
            Offside {
                day: tuesday,
                task: third,
                appointment: "Lavar o carro".to_owned(),
                violation: Violation::AfterNightCap {
                    cap: time(22, 0, 0, 0),
                },
            },
            Offside {
                day: monday,
                task: first,
                appointment: "Trocar o filtro".to_owned(),
                violation: Violation::AfterNightCap {
                    cap: time(22, 0, 0, 0),
                },
            },
        ];
        let week = WeekSummary::close(&log.events, days, offside, Vec::new(), Unbeaten::default());

        assert_eq!(week.days.len(), 7);
        assert_eq!(week.goals(), 2);
        assert_eq!(week.benched(), 1);
        assert_eq!(week.stoppage(), 1);
        assert_eq!(week.offside[0].day, monday);
    }
}
