//! A escalação do dia e o placar.

use jiff::civil::Date;

use crate::{Projection, Status, Task};

/// As tarefas de um dia, separadas do jeito que a tela Hoje mostra.
///
/// ```
/// use jiff::civil::date;
/// use vamo_core::{Lineup, Projection};
///
/// let lineup = Lineup::new(&Projection::new(), date(2026, 11, 11));
/// assert!(lineup.is_empty());
/// assert_eq!(lineup.score().planned, 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lineup {
    today: Date,
    /// As escaladas pro dia, concluídas ou não.
    pub planned: Vec<Task>,
    /// As escaladas pra um dia que já passou e que não foram concluídas: o
    /// atraso.
    pub late: Vec<Task>,
    /// As recorrentes, as atividades fixas que se marcam todo dia.
    pub fixed: Vec<Task>,
}

/// O placar do dia: quanto já foi feito do que estava previsto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Score {
    /// Os gols: tarefas do dia concluídas e fixas marcadas.
    pub done: usize,
    /// Tudo que estava previsto pro dia, fixas incluídas. O atraso fica de
    /// fora: é do dia em que foi escalado.
    pub planned: usize,
}

impl Lineup {
    /// A escalação de `today`. Tarefa descartada não entra em lugar nenhum.
    pub fn new(projection: &Projection, today: Date) -> Self {
        let mut lineup = Self {
            today,
            planned: Vec::new(),
            late: Vec::new(),
            fixed: Vec::new(),
        };
        for task in projection.tasks() {
            match (task.status(), task.day()) {
                (Status::Dropped, _) => {}
                (Status::Recurring, _) => lineup.fixed.push(task),
                (_, Some(day)) if day == today => lineup.planned.push(task),
                (status, Some(day)) if day < today && status != Status::Done => {
                    lineup.late.push(task);
                }
                _ => {}
            }
        }
        lineup
    }

    /// O dia da escalação.
    pub fn today(&self) -> Date {
        self.today
    }

    /// Se não tem nada no dia: nem escalada, nem atraso, nem fixa.
    pub fn is_empty(&self) -> bool {
        self.planned.is_empty() && self.late.is_empty() && self.fixed.is_empty()
    }

    /// O placar do dia.
    pub fn score(&self) -> Score {
        let goals = self
            .planned
            .iter()
            .filter(|task| task.status() == Status::Done)
            .count();
        let checks = self
            .fixed
            .iter()
            .filter(|task| task.is_checked(self.today))
            .count();
        Score {
            done: goals + checks,
            planned: self.planned.len() + self.fixed.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use uuid::Builder;

    use super::*;
    use crate::{Change, Event, EventId, Fit, Machine, TaskId};

    /// Um log montado aos poucos, com um evento por segundo.
    struct Log {
        projection: Projection,
        n: u64,
    }

    impl Log {
        fn new() -> Self {
            Self {
                projection: Projection::new(),
                n: 0,
            }
        }

        fn record(&mut self, id: TaskId, changes: Vec<Change>) {
            for change in changes {
                self.n += 1;
                let event = Event {
                    id: EventId::new(
                        Builder::from_unix_timestamp_millis(self.n, &[0; 10]).into_uuid(),
                    )
                    .unwrap(),
                    at: jiff::Timestamp::from_second(i64::try_from(self.n).unwrap()).unwrap(),
                    machine: Machine::new("notebook").unwrap(),
                    task: id,
                    change,
                };
                self.projection.apply(&event);
            }
        }

        fn add(&mut self, title: &str, status: Status) -> TaskId {
            let id = self.projection.next_id().unwrap();
            self.record(
                id,
                vec![Change::Created {
                    title: title.parse().unwrap(),
                    area: "casa".parse().unwrap(),
                    fit: Fit::Gap,
                    status,
                }],
            );
            id
        }
    }

    const TODAY: Date = date(2026, 11, 11);
    const YESTERDAY: Date = date(2026, 11, 10);

    fn titles(tasks: &[Task]) -> Vec<&str> {
        tasks.iter().map(|task| task.title().as_str()).collect()
    }

    #[test]
    fn each_task_goes_to_its_place() {
        let mut log = Log::new();
        let today = log.add("Trocar o filtro", Status::Ready);
        let late = log.add("Pagar a conta", Status::Ready);
        let tomorrow = log.add("Lavar o carro", Status::Ready);
        let water = log.add("Beber água", Status::Recurring);
        let dropped = log.add("Pintar a parede", Status::Ready);
        log.add("Ler o livro", Status::Ready);

        let schedule = |log: &mut Log, id, day| {
            let changes = log.projection.schedule(id, day).unwrap();
            log.record(id, changes);
        };
        schedule(&mut log, today, TODAY);
        schedule(&mut log, late, YESTERDAY);
        schedule(&mut log, tomorrow, date(2026, 11, 12));
        schedule(&mut log, dropped, TODAY);
        log.record(dropped, vec![Change::Status(Status::Dropped)]);

        let lineup = Lineup::new(&log.projection, TODAY);
        assert_eq!(titles(&lineup.planned), ["Trocar o filtro"]);
        assert_eq!(titles(&lineup.late), ["Pagar a conta"]);
        assert_eq!(titles(&lineup.fixed), ["Beber água"]);
        assert_eq!(lineup.fixed[0].id(), water);
    }

    #[test]
    fn the_score_counts_goals_and_checks() {
        let mut log = Log::new();
        let first = log.add("Trocar o filtro", Status::Ready);
        let second = log.add("Pagar a conta", Status::Ready);
        let water = log.add("Beber água", Status::Recurring);
        for id in [first, second] {
            let changes = log.projection.schedule(id, TODAY).unwrap();
            log.record(id, changes);
        }
        assert_eq!(
            Lineup::new(&log.projection, TODAY).score(),
            Score {
                done: 0,
                planned: 3
            }
        );

        let changes = log.projection.finish(first, TODAY).unwrap();
        log.record(first, changes);
        let change = log.projection.check(water, TODAY, true).unwrap();
        log.record(water, vec![change]);
        assert_eq!(
            Lineup::new(&log.projection, TODAY).score(),
            Score {
                done: 2,
                planned: 3
            }
        );

        // A marcação é do dia: amanhã a fixa começa desmarcada.
        let tomorrow = Lineup::new(&log.projection, date(2026, 11, 12));
        assert_eq!(
            tomorrow.score(),
            Score {
                done: 0,
                planned: 1
            }
        );
    }

    #[test]
    fn a_goal_counts_on_the_day_it_was_scored() {
        let mut log = Log::new();
        let late = log.add("Pagar a conta", Status::Ready);
        let changes = log.projection.schedule(late, YESTERDAY).unwrap();
        log.record(late, changes);
        let loose = log.add("Ler o livro", Status::Ready);

        for id in [late, loose] {
            let changes = log.projection.finish(id, TODAY).unwrap();
            log.record(id, changes);
        }
        let lineup = Lineup::new(&log.projection, TODAY);
        assert_eq!(lineup.late, []);
        assert_eq!(titles(&lineup.planned), ["Pagar a conta", "Ler o livro"]);
        assert_eq!(
            lineup.score(),
            Score {
                done: 2,
                planned: 2
            }
        );
    }

    #[test]
    fn unscheduling_sends_the_task_back_to_the_bench() {
        let mut log = Log::new();
        let id = log.add("Trocar o filtro", Status::Ready);
        let changes = log.projection.schedule(id, TODAY).unwrap();
        log.record(id, changes);
        let changes = log.projection.unschedule(id).unwrap();
        log.record(id, changes);

        let task = log.projection.task(id).unwrap();
        assert_eq!(task.status(), Status::Ready);
        assert_eq!(task.day(), None);
        assert!(Lineup::new(&log.projection, TODAY).is_empty());
    }

    #[test]
    fn rescheduling_only_moves_the_day() {
        let mut log = Log::new();
        let id = log.add("Trocar o filtro", Status::Ready);
        let changes = log.projection.schedule(id, TODAY).unwrap();
        assert_eq!(changes.len(), 2);
        log.record(id, changes);
        assert_eq!(
            log.projection.schedule(id, YESTERDAY).unwrap(),
            [Change::Day(Some(YESTERDAY))]
        );
        assert_eq!(log.projection.schedule(id, TODAY).unwrap(), []);
    }

    #[test]
    fn an_idea_cannot_be_scheduled() {
        let mut log = Log::new();
        let id = log.add("Pensar na viagem", Status::Idea);
        assert!(matches!(
            log.projection.schedule(id, TODAY),
            Err(crate::Error::InvalidTransition { .. })
        ));
    }

    #[test]
    fn only_recurring_tasks_are_checked() {
        let mut log = Log::new();
        let id = log.add("Trocar o filtro", Status::Ready);
        assert_eq!(
            log.projection.check(id, TODAY, true),
            Err(crate::Error::NotRecurring { id })
        );
    }
}
