//! A ligação entre a agenda e as tarefas: o que o sync faz com cada evento
//! que tem o rodapé `vamo:`.

use std::collections::BTreeMap;

use jiff::civil::Date;

use crate::{Appointment, Change, Projection, Status, Task, TaskId};

/// O resultado de cruzar a agenda com as tarefas: as mudanças pra gravar e
/// os eventos que ficaram de fora, com o motivo.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LinkPlan {
    /// As mudanças de cada tarefa, na ordem dos IDs. Tarefa que já está
    /// agendada no dia certo não aparece.
    pub changes: Vec<(TaskId, Vec<Change>)>,
    /// Os eventos que apontam pra uma tarefa que o sync não mexe.
    pub skipped: Vec<Skipped>,
}

/// Um evento com rodapé que o sync deixou de lado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skipped {
    /// O rodapé aponta pra uma tarefa que não existe no banco.
    Unknown(TaskId),
    /// A tarefa ainda não está pronta (ideia ou destrinchando), então não
    /// dá pra agendar.
    NotReady(TaskId, Status),
}

impl Projection {
    /// O plano do sync: cada tarefa ligada a um evento fica agendada no dia
    /// dele.
    ///
    /// Se a tarefa aparece em mais de um evento, vale o primeiro de hoje em
    /// diante; sem nenhum daqui pra frente, o último que já passou. Tarefa
    /// concluída, descartada ou recorrente não muda: o evento na agenda é só
    /// lembrança.
    ///
    /// ```
    /// use jiff::civil::date;
    /// use vamo_core::{Appointment, Projection, When};
    ///
    /// let agenda = [Appointment {
    ///     uid: "filtro@exemplo".to_owned(),
    ///     title: "Trocar o filtro".to_owned(),
    ///     when: When::AllDay(date(2026, 11, 14)),
    ///     recurring: false,
    ///     task: Some("T-041".parse()?),
    /// }];
    /// // Num banco vazio, o rodapé aponta pra uma tarefa que não existe.
    /// let plan = Projection::new().link(&agenda, date(2026, 11, 12));
    /// assert!(plan.changes.is_empty());
    /// assert_eq!(plan.skipped.len(), 1);
    /// # Ok::<(), vamo_core::Error>(())
    /// ```
    pub fn link(&self, agenda: &[Appointment], today: Date) -> LinkPlan {
        let mut days: BTreeMap<TaskId, Vec<Date>> = BTreeMap::new();
        for appointment in agenda {
            if let Some(task) = appointment.task {
                days.entry(task).or_default().push(appointment.day());
            }
        }

        let mut plan = LinkPlan::default();
        for (id, mut dates) in days {
            dates.sort_unstable();
            let day = dates
                .iter()
                .copied()
                .find(|&day| day >= today)
                .or_else(|| dates.last().copied());
            let (Some(task), Some(day)) = (self.task(id), day) else {
                plan.skipped.push(Skipped::Unknown(id));
                continue;
            };
            match task.status() {
                Status::Done | Status::Dropped | Status::Recurring => {}
                status @ (Status::Idea | Status::Refining) => {
                    plan.skipped.push(Skipped::NotReady(id, status));
                }
                Status::Ready | Status::Scheduled => {
                    // Prontas e agendadas sempre podem ser agendadas.
                    if let Ok(changes) = self.schedule(id, day)
                        && !changes.is_empty()
                    {
                        plan.changes.push((id, changes));
                    }
                }
            }
        }
        plan
    }

    /// A caixa "a marcar": as tarefas prontas que ainda não têm dia, na
    /// ordem dos IDs. É daqui que sai o que vai pra agenda; quando o evento
    /// com o rodapé volta no sync, a tarefa vira agendada e sai da caixa.
    ///
    /// ```
    /// use vamo_core::Projection;
    ///
    /// assert!(Projection::new().inbox().is_empty());
    /// ```
    pub fn inbox(&self) -> Vec<Task> {
        self.tasks()
            .filter(|task| task.status() == Status::Ready && task.day().is_none())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;
    use jiff::civil::date;
    use uuid::Builder;

    use super::*;
    use crate::{Event, EventId, Fit, Machine, When};

    const TODAY: Date = date(2026, 11, 12);

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
                self.projection.apply(&Event {
                    id: EventId::new(
                        Builder::from_unix_timestamp_millis(self.n, &[0; 10]).into_uuid(),
                    )
                    .unwrap(),
                    at: Timestamp::from_second(i64::try_from(self.n).unwrap()).unwrap(),
                    machine: Machine::new("notebook").unwrap(),
                    task: id,
                    change,
                });
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

    fn on(day: Date, task: TaskId) -> Appointment {
        Appointment {
            uid: format!("{task}@{day}"),
            title: "Evento".to_owned(),
            when: When::Timed {
                start: day.at(10, 0, 0, 0),
                end: day.at(10, 30, 0, 0),
            },
            recurring: false,
            task: Some(task),
        }
    }

    #[test]
    fn a_ready_task_becomes_scheduled_on_the_event_day() {
        let mut log = Log::new();
        let id = log.add("Trocar o filtro", Status::Ready);
        let plan = log.projection.link(&[on(date(2026, 11, 14), id)], TODAY);
        assert_eq!(
            plan.changes,
            [(
                id,
                vec![
                    Change::Status(Status::Scheduled),
                    Change::Day(Some(date(2026, 11, 14)))
                ]
            )]
        );
    }

    #[test]
    fn a_moved_event_moves_the_day_and_a_second_sync_changes_nothing() {
        let mut log = Log::new();
        let id = log.add("Trocar o filtro", Status::Ready);
        let plan = log.projection.link(&[on(date(2026, 11, 14), id)], TODAY);
        for (id, changes) in plan.changes {
            log.record(id, changes);
        }
        assert_eq!(
            log.projection.link(&[on(date(2026, 11, 14), id)], TODAY),
            LinkPlan::default()
        );
        let moved = log.projection.link(&[on(date(2026, 11, 16), id)], TODAY);
        assert_eq!(
            moved.changes,
            [(id, vec![Change::Day(Some(date(2026, 11, 16)))])]
        );
    }

    #[test]
    fn the_next_event_wins_over_past_ones() {
        let mut log = Log::new();
        let id = log.add("Trocar o filtro", Status::Ready);
        let agenda = [
            on(date(2026, 11, 10), id),
            on(date(2026, 11, 20), id),
            on(date(2026, 11, 13), id),
        ];
        let plan = log.projection.link(&agenda, TODAY);
        assert_eq!(plan.changes[0].1[1], Change::Day(Some(date(2026, 11, 13))));
    }

    #[test]
    fn a_task_leaves_the_inbox_once_the_agenda_schedules_it() {
        let mut log = Log::new();
        let id = log.add("Trocar o filtro", Status::Ready);
        log.add("Pensar na viagem", Status::Idea);
        let ids =
            |log: &Log| -> Vec<TaskId> { log.projection.inbox().iter().map(Task::id).collect() };
        assert_eq!(ids(&log), [id]);
        let plan = log.projection.link(&[on(date(2026, 11, 14), id)], TODAY);
        for (id, changes) in plan.changes {
            log.record(id, changes);
        }
        assert_eq!(ids(&log), []);
    }

    #[test]
    fn closed_recurring_and_unready_tasks_are_left_alone() {
        let mut log = Log::new();
        let done = log.add("Pagar a conta", Status::Ready);
        let finish = log.projection.finish(done, TODAY).unwrap();
        log.record(done, finish);
        let water = log.add("Beber água", Status::Recurring);
        let idea = log.add("Pensar na viagem", Status::Idea);
        let ghost = TaskId::new(99).unwrap();

        let agenda = [
            on(TODAY, done),
            on(TODAY, water),
            on(TODAY, idea),
            on(TODAY, ghost),
        ];
        let plan = log.projection.link(&agenda, TODAY);
        assert_eq!(plan.changes, []);
        assert_eq!(
            plan.skipped,
            [
                Skipped::NotReady(idea, Status::Idea),
                Skipped::Unknown(ghost)
            ]
        );
    }
}
