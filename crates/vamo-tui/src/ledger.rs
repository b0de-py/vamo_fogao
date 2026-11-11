//! A porta da TUI pras tarefas.

use std::fmt::Display;

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
}

#[cfg(test)]
pub(crate) mod memory {
    //! Um log em memória pros testes, com IDs e instantes previsíveis.

    use std::convert::Infallible;

    use jiff::Timestamp;
    use uuid::Builder;
    use vamo_core::{Change, Event, EventId, Machine, Projection, TaskId};

    use super::Ledger;

    /// O log dos testes. Cada evento ganha um ID e um instante um segundo
    /// depois do anterior, a partir de `start`.
    #[derive(Debug)]
    pub(crate) struct MemoryLedger {
        events: Vec<Event>,
        projection: Projection,
        start: Timestamp,
    }

    impl MemoryLedger {
        pub(crate) fn new(start: Timestamp) -> Self {
            Self {
                events: Vec::new(),
                projection: Projection::new(),
                start,
            }
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
    }
}
