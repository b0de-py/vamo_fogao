//! A invencibilidade: dias seguidos sem tarefa vencida, com o recorde.

use jiff::civil::Date;

use crate::{DaySummary, DayWindow, Event};

/// O recorde a bater: o Botafogo ficou 52 jogos invicto entre 1977 e 1978.
pub const CLUB_RECORD: u32 = 52;

/// Como um dia fechado conta pra invencibilidade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Teve gol e nenhuma tarefa venceu: a sequência anda.
    Win,
    /// Uma tarefa venceu, sem ser feita nem remarcada: a sequência quebra.
    Loss,
    /// Dia sem jogo: nada escalado (ou tudo remarcado) e nenhum gol. Não
    /// anda nem quebra a sequência.
    Idle,
}

impl DaySummary {
    /// Como o dia conta pra invencibilidade.
    ///
    /// Só as tarefas escaladas valem. As fixas ficam de fora, pra um dia
    /// sem marcar a água não quebrar a sequência, e a tarefa remarcada
    /// também, porque remarcar é decidir, não deixar vencer.
    pub fn outcome(&self) -> Outcome {
        if !self.stoppage.is_empty() {
            Outcome::Loss
        } else if self.goals.is_empty() {
            Outcome::Idle
        } else {
            Outcome::Win
        }
    }
}

/// A invencibilidade: a sequência atual de dias sem vencida e a maior que
/// já teve.
///
/// ```
/// use vamo_core::Unbeaten;
///
/// let unbeaten = Unbeaten::count(&[], []);
/// assert_eq!(unbeaten, Unbeaten { current: 0, record: 0 });
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Unbeaten {
    /// Os dias com gol seguidos até o último dia fechado, sem vencida no
    /// meio. Dia sem jogo fica de fora da conta, sem quebrar.
    pub current: u32,
    /// A maior sequência que já teve, a atual inclusa.
    pub record: u32,
}

impl Unbeaten {
    /// Conta a invencibilidade nos dias dados, que precisam vir em ordem e
    /// só com dias já fechados. Cada dia vem com o trecho do relógio que
    /// conta como ele, como na [`DaySummary::close`].
    ///
    /// A conta fecha a súmula de cada dia, então custa um passeio pelo log
    /// por dia. Com alguns anos de uso é coisa de milissegundos.
    pub fn count(events: &[Event], days: impl IntoIterator<Item = (Date, DayWindow)>) -> Self {
        let mut unbeaten = Self::default();
        for (day, window) in days {
            match DaySummary::close(events, day, window, Vec::new()).outcome() {
                Outcome::Win => {
                    unbeaten.current += 1;
                    unbeaten.record = unbeaten.record.max(unbeaten.current);
                }
                Outcome::Loss => unbeaten.current = 0,
                Outcome::Idle => {}
            }
        }
        unbeaten
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;
    use crate::Status;
    use crate::summary::tests::{Log, window};

    /// Os dias de `first` a `last`, cada um com o trecho dele.
    fn days(first: Date, last: Date) -> Vec<(Date, DayWindow)> {
        first
            .series(jiff::ToSpan::days(1))
            .take_while(|day| *day <= last)
            .map(|day| (day, window(day)))
            .collect()
    }

    /// Um dia com uma tarefa escalada que vira gol.
    fn win(log: &mut Log, day: Date) {
        let id = log.add("Trocar o filtro", Status::Ready);
        log.at(day, 8);
        log.schedule(id, day);
        log.at(day, 18);
        log.finish(id, day);
    }

    /// Um dia com uma tarefa escalada que vence.
    fn loss(log: &mut Log, day: Date) {
        let id = log.add("Pagar a conta", Status::Ready);
        log.at(day, 8);
        log.schedule(id, day);
    }

    #[test]
    fn wins_in_a_row_count_and_a_loss_breaks_them() {
        let mut log = Log::new();
        for day in [date(2026, 11, 2), date(2026, 11, 3), date(2026, 11, 4)] {
            win(&mut log, day);
        }
        loss(&mut log, date(2026, 11, 5));
        win(&mut log, date(2026, 11, 6));

        let unbeaten = Unbeaten::count(&log.events, days(date(2026, 11, 2), date(2026, 11, 6)));
        assert_eq!(
            unbeaten,
            Unbeaten {
                current: 1,
                record: 3
            }
        );
    }

    #[test]
    fn a_day_without_tasks_neither_counts_nor_breaks() {
        let mut log = Log::new();
        win(&mut log, date(2026, 11, 2));
        // O dia 3 passa em branco.
        win(&mut log, date(2026, 11, 4));

        let unbeaten = Unbeaten::count(&log.events, days(date(2026, 11, 2), date(2026, 11, 4)));
        assert_eq!(
            unbeaten,
            Unbeaten {
                current: 2,
                record: 2
            }
        );
    }

    #[test]
    fn the_streak_goes_through_the_turn_of_the_month() {
        let mut log = Log::new();
        for day in [
            date(2026, 11, 29),
            date(2026, 11, 30),
            date(2026, 12, 1),
            date(2026, 12, 2),
        ] {
            win(&mut log, day);
        }

        let unbeaten = Unbeaten::count(&log.events, days(date(2026, 11, 29), date(2026, 12, 2)));
        assert_eq!(unbeaten.current, 4);
    }

    #[test]
    fn a_rescheduled_task_does_not_break_the_streak() {
        let mut log = Log::new();
        win(&mut log, date(2026, 11, 2));
        let id = log.add("Pagar a conta", Status::Ready);
        log.at(date(2026, 11, 3), 8);
        log.schedule(id, date(2026, 11, 3));
        let other = log.add("Lavar o carro", Status::Ready);
        log.schedule(other, date(2026, 11, 3));
        log.at(date(2026, 11, 3), 21);
        log.schedule(id, date(2026, 11, 10));
        log.finish(other, date(2026, 11, 3));

        let unbeaten = Unbeaten::count(&log.events, days(date(2026, 11, 2), date(2026, 11, 3)));
        assert_eq!(unbeaten.current, 2);
    }

    #[test]
    fn a_late_task_breaks_only_its_own_day() {
        let mut log = Log::new();
        loss(&mut log, date(2026, 11, 2));
        // A tarefa vencida do dia 2 continua atrasada, mas o dia 3 tem gol e
        // nenhuma vencida dele: a sequência recomeça.
        win(&mut log, date(2026, 11, 3));

        let unbeaten = Unbeaten::count(&log.events, days(date(2026, 11, 2), date(2026, 11, 3)));
        assert_eq!(unbeaten.current, 1);
    }

    #[test]
    fn a_day_without_tasks_is_idle() {
        let log = Log::new();
        let day = date(2026, 11, 2);

        let summary = DaySummary::close(&log.events, day, window(day), Vec::new());
        assert_eq!(summary.outcome(), Outcome::Idle);
    }
}
