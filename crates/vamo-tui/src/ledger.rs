//! A porta da TUI pras tarefas.

use std::collections::BTreeSet;
use std::fmt::Display;

use jiff::civil::Date;

use vamo_core::{Change, Event, Projection, TaskId};

/// O que a TUI precisa das tarefas: ler o estado e gravar mudanças.
///
/// A TUI não sabe onde o log mora nem como um evento ganha ID, instante e
/// máquina. Quem sabe é o binário, que implementa esta trait em cima do
/// banco. Assim a TUI não depende do `vamo-store` (adaptador não chama
/// adaptador), e os testes usam um log em memória.
pub trait Ledger {
    /// O erro de gravação. A TUI só mostra a mensagem, em amarelo plasma.
    type Error: Display;

    /// O estado das tarefas, já com tudo que foi gravado.
    fn projection(&self) -> &Projection;

    /// O log inteiro, na ordem de gravação. É de onde sai a súmula da ficha.
    fn events(&self) -> &[Event];

    /// Grava as mudanças de uma tarefa como eventos e aplica na projeção.
    ///
    /// # Errors
    ///
    /// Quando a gravação falha. Nesse caso nada muda.
    fn record(&mut self, task: TaskId, changes: Vec<Change>) -> Result<(), Self::Error>;

    /// As atividades fixas da agenda marcadas, como `(dia, UID do evento)`.
    fn agenda_checks(&self) -> &BTreeSet<(Date, String)>;

    /// Marca (`done`) ou desmarca a atividade fixa `uid` em `day`.
    ///
    /// # Errors
    ///
    /// Quando a gravação falha. Nesse caso nada muda.
    fn check_agenda(&mut self, day: Date, uid: &str, done: bool) -> Result<(), Self::Error>;
}

#[cfg(test)]
pub(crate) mod memory {
    //! Um log em memória pros testes, com IDs e instantes previsíveis.

    use std::collections::BTreeSet;
    use std::convert::Infallible;

    use jiff::civil::Date;

    use jiff::Timestamp;
    use uuid::Builder;
    use vamo_core::{Change, Event, EventId, Fit, Machine, Projection, Status, TaskId};

    use super::Ledger;

    /// O log dos testes. Cada evento ganha um ID e um instante um segundo
    /// depois do anterior, a partir de `start`.
    #[derive(Debug)]
    pub(crate) struct MemoryLedger {
        events: Vec<Event>,
        projection: Projection,
        start: Timestamp,
        checks: BTreeSet<(Date, String)>,
    }

    impl MemoryLedger {
        pub(crate) fn new(start: Timestamp) -> Self {
            Self {
                events: Vec::new(),
                projection: Projection::new(),
                start,
                checks: BTreeSet::new(),
            }
        }

        /// Cria uma tarefa e devolve o ID dela.
        pub(crate) fn add(&mut self, title: &str, area: &str, fit: Fit, status: Status) -> TaskId {
            let id = self.projection.next_id().unwrap();
            let change = Change::Created {
                title: title.parse().unwrap(),
                area: area.parse().unwrap(),
                fit,
                status,
            };
            self.record(id, vec![change]).unwrap();
            id
        }
    }

    impl Ledger for MemoryLedger {
        type Error = Infallible;

        fn projection(&self) -> &Projection {
            &self.projection
        }

        fn events(&self) -> &[Event] {
            &self.events
        }

        fn record(&mut self, task: TaskId, changes: Vec<Change>) -> Result<(), Infallible> {
            for change in changes {
                let n = u64::try_from(self.events.len()).unwrap() + 1;
                let event = Event {
                    id: EventId::new(Builder::from_unix_timestamp_millis(n, &[0; 10]).into_uuid())
                        .unwrap(),
                    at: self.start + jiff::SignedDuration::from_secs(i64::try_from(n).unwrap()),
                    machine: Machine::new("notebook").unwrap(),
                    task,
                    change,
                };
                self.projection.apply(&event);
                self.events.push(event);
            }
            Ok(())
        }

        fn agenda_checks(&self) -> &BTreeSet<(Date, String)> {
            &self.checks
        }

        fn check_agenda(&mut self, day: Date, uid: &str, done: bool) -> Result<(), Infallible> {
            if done {
                self.checks.insert((day, uid.to_owned()));
            } else {
                self.checks.remove(&(day, uid.to_owned()));
            }
            Ok(())
        }
    }
}
