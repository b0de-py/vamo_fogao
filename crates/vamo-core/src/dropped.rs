//! As descartadas há pouco, e o caminho de volta delas pro banco.
//!
//! Descartar é só mais um evento no log: a tarefa continua lá, com o
//! histórico inteiro. O que some é ela da vista. Por isso dá pra achar o
//! descarte e o status de antes lendo o log, e desfazer sem perder nada.

use jiff::Timestamp;

use crate::{Change, Error, Event, Projection, Status, Task, TaskId};

/// Uma tarefa descartada: quando foi e o status que ela tinha antes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discarded {
    /// A tarefa, como está agora (descartada).
    pub task: Task,
    /// Quando ela foi descartada.
    pub at: Timestamp,
    /// O status logo antes do descarte.
    pub before: Status,
}

impl Projection {
    /// As tarefas descartadas a partir de `since` que continuam
    /// descartadas, da mais recente pra mais antiga.
    ///
    /// Os eventos de cada tarefa são lidos na ordem da projeção (instante,
    /// depois ID do evento), pra o "status de antes" ser o mesmo que a
    /// projeção via naquela hora. Uma tarefa descartada mais de uma vez
    /// conta pelo último descarte.
    pub fn dropped_since(&self, events: &[Event], since: Timestamp) -> Vec<Discarded> {
        let mut found: Vec<Discarded> = self
            .tasks()
            .filter(|task| task.status() == Status::Dropped)
            .filter_map(|task| {
                let (at, before) = last_drop(events, task.id())?;
                (at >= since).then_some(Discarded { task, at, before })
            })
            .collect();
        found.sort_by_key(|discarded| std::cmp::Reverse((discarded.at, discarded.task.id())));
        found
    }

    /// As mudanças que trazem uma descartada de volta pro banco, com o
    /// status de `before`.
    ///
    /// A regra de [`Status`] só deixa a descartada virar ideia, então a
    /// volta passa por ela: descartada, ideia e, se for o caso, o status de
    /// antes. Agendada e concluída voltam como pronta: o dia da escalação
    /// provavelmente já passou, e um gol não se devolve no lugar de outro.
    ///
    /// # Errors
    ///
    /// - [`Error::UnknownTask`] se a tarefa não existir.
    /// - [`Error::InvalidTransition`] se ela não estiver descartada.
    pub fn restore(&self, id: TaskId, before: Status) -> Result<Vec<Change>, Error> {
        let mut changes = vec![self.status_change(id, Status::Idea)?];
        let target = match before {
            Status::Scheduled | Status::Done => Status::Ready,
            other => other,
        };
        if target != Status::Idea && target != Status::Dropped {
            changes.push(Change::Status(Status::Idea.transition_to(target)?));
        }
        Ok(changes)
    }
}

/// O último descarte da tarefa no log: quando foi e o status de antes.
fn last_drop(events: &[Event], id: TaskId) -> Option<(Timestamp, Status)> {
    let mut mine: Vec<&Event> = events.iter().filter(|event| event.task == id).collect();
    mine.sort_by_key(|event| (event.at, event.id));
    let mut status = None;
    let mut drop = None;
    for event in mine {
        let next = match &event.change {
            Change::Created { status, .. } | Change::Status(status) => *status,
            _ => continue,
        };
        if next == Status::Dropped
            && let Some(before) = status
            && before != Status::Dropped
        {
            drop = Some((event.at, before));
        }
        status = Some(next);
    }
    drop
}

#[cfg(test)]
mod tests {
    use uuid::Builder;

    use super::*;
    use crate::{EventId, Fit, Machine};

    /// Um evento da tarefa `task`, `minute` minutos depois do meio-dia.
    fn event(task: u32, minute: i64, change: Change) -> Event {
        let at: Timestamp = "2026-11-17T12:00:00Z".parse().unwrap();
        let at = at + jiff::SignedDuration::from_mins(minute);
        // O minuto vira o ID do evento: na mesma tarefa, nenhum se repete.
        let millis = u64::try_from(minute).unwrap() + u64::from(task) * 1000;
        let uuid = Builder::from_unix_timestamp_millis(millis, &[0; 10]).into_uuid();
        Event {
            id: EventId::new(uuid).unwrap(),
            at,
            machine: "notebook".parse::<Machine>().unwrap(),
            task: TaskId::new(task).unwrap(),
            change,
        }
    }

    fn created(task: u32, status: Status) -> Event {
        event(
            task,
            0,
            Change::Created {
                title: "Lavar o carro".parse().unwrap(),
                area: "casa".parse().unwrap(),
                fit: Fit::Gap,
                status,
            },
        )
    }

    fn at(minute: i64) -> Timestamp {
        "2026-11-17T12:00:00Z".parse::<Timestamp>().unwrap()
            + jiff::SignedDuration::from_mins(minute)
    }

    #[test]
    fn the_recent_drops_come_newest_first_with_the_status_before() {
        let events = [
            created(1, Status::Ready),
            event(1, 10, Change::Status(Status::Dropped)),
            created(2, Status::Idea),
            event(2, 5, Change::Status(Status::Refining)),
            event(2, 20, Change::Status(Status::Dropped)),
            // A 3 foi descartada cedo demais pra contar.
            created(3, Status::Ready),
            event(3, 1, Change::Status(Status::Dropped)),
            // A 4 nem está descartada.
            created(4, Status::Ready),
        ];
        let projection: Projection = events.iter().collect();
        let found = projection.dropped_since(&events, at(2));
        let summary: Vec<(u32, Status)> = found
            .iter()
            .map(|discarded| (discarded.task.id().number(), discarded.before))
            .collect();
        assert_eq!(summary, [(2, Status::Refining), (1, Status::Ready)]);
        assert_eq!(found[0].at, at(20));
    }

    #[test]
    fn a_task_brought_back_is_not_listed() {
        let events = [
            created(1, Status::Ready),
            event(1, 10, Change::Status(Status::Dropped)),
            event(1, 11, Change::Status(Status::Idea)),
        ];
        let projection: Projection = events.iter().collect();
        assert_eq!(projection.dropped_since(&events, at(0)), []);
    }

    #[test]
    fn the_way_back_goes_through_idea() {
        let events = [
            created(1, Status::Ready),
            event(1, 10, Change::Status(Status::Dropped)),
        ];
        let projection: Projection = events.iter().collect();
        let id = TaskId::new(1).unwrap();
        assert_eq!(
            projection.restore(id, Status::Ready),
            Ok(vec![
                Change::Status(Status::Idea),
                Change::Status(Status::Ready)
            ])
        );
        assert_eq!(
            projection.restore(id, Status::Idea),
            Ok(vec![Change::Status(Status::Idea)])
        );
        // Agendada volta pro banco como pronta.
        assert_eq!(
            projection.restore(id, Status::Scheduled),
            Ok(vec![
                Change::Status(Status::Idea),
                Change::Status(Status::Ready)
            ])
        );
    }

    #[test]
    fn only_a_dropped_task_comes_back() {
        let events = [created(1, Status::Ready)];
        let projection: Projection = events.iter().collect();
        assert!(matches!(
            projection.restore(TaskId::new(1).unwrap(), Status::Ready),
            Err(Error::InvalidTransition { .. })
        ));
    }
}
