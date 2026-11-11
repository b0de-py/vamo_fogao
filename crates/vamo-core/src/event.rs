//! O log de eventos: cada mudança numa tarefa, com quando e onde aconteceu.

use std::fmt;
use std::str::FromStr;

use jiff::Timestamp;
use uuid::{Uuid, Variant, Version};

use crate::{Area, Error, Fit, Machine, NotebookTitle, Status, TaskId};

/// Uma mudança numa tarefa: o que mudou, quando e em que máquina.
///
/// O log de eventos é a fonte da verdade do app. O estado das tarefas não é
/// guardado, é calculado: a [`Projection`](crate::Projection) aplica os
/// eventos e chega nele. É isso que deixa várias máquinas em sincronia, porque
/// juntar dois logs é só juntar os eventos.
///
/// Os campos são públicos porque cada um já se valida no próprio tipo: não
/// tem como montar um evento inválido.
///
/// ```
/// use jiff::Timestamp;
/// use uuid::Uuid;
/// use vamo_core::{Change, Event, EventId, Status};
///
/// let event = Event {
///     id: EventId::new(Uuid::parse_str("0192f0c4-8a00-7000-8000-000000000001")?)?,
///     at: "2026-11-11T12:00:00Z".parse::<Timestamp>()?,
///     machine: "notebook".parse()?,
///     task: "T-041".parse()?,
///     change: Change::Status(Status::Done),
/// };
/// assert_eq!(event.task.to_string(), "T-041");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// O ID do evento, único entre todas as máquinas.
    pub id: EventId,
    /// Quando a mudança aconteceu, no relógio da máquina de origem.
    pub at: Timestamp,
    /// A máquina de origem.
    pub machine: Machine,
    /// A tarefa que mudou.
    pub task: TaskId,
    /// O que mudou.
    pub change: Change,
}

/// O que mudou numa tarefa.
///
/// A criação preenche todos os campos; as outras variantes mexem em um
/// campo só. É isso que deixa a projeção resolver conflito campo a campo:
/// se uma máquina troca o título e outra troca a área, as duas mudanças
/// ficam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A tarefa nasceu.
    Created {
        /// O título.
        title: NotebookTitle,
        /// A área.
        area: Area,
        /// O encaixe.
        fit: Fit,
        /// O status de partida.
        status: Status,
    },
    /// O título mudou.
    Title(NotebookTitle),
    /// A área mudou.
    Area(Area),
    /// O encaixe mudou.
    Fit(Fit),
    /// O status mudou. A regra de transição é conferida antes de o evento
    /// existir, com [`Status::transition_to`]; a projeção só aplica.
    Status(Status),
    /// As notas mudaram. Vazio apaga as notas.
    Notes(String),
}

/// O ID de um evento: um UUID v7.
///
/// O UUID v7 começa com o instante, em milissegundos, e termina com bits
/// sorteados. Por isso os IDs saem em ordem de criação e duas máquinas
/// criando eventos no mesmo instante não esbarram uma na outra.
///
/// Gerar um ID novo pede relógio e sorteio, e o core não tem nenhum dos dois.
/// Quem gera é o binário, com `Uuid::now_v7()`; aqui o ID só é conferido.
///
/// ```
/// use vamo_core::EventId;
///
/// let id: EventId = "0192f0c4-8a00-7000-8000-000000000001".parse()?;
/// assert_eq!(id.to_string(), "0192f0c4-8a00-7000-8000-000000000001");
///
/// // Um UUID v4, todo sorteado, não serve: não tem o instante na frente.
/// assert!("0192f0c4-8a00-4000-8000-000000000001".parse::<EventId>().is_err());
/// # Ok::<(), vamo_core::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(Uuid);

impl EventId {
    /// O ID a partir de um UUID.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidEventId`] se o UUID não for da versão 7 no padrão da
    /// RFC 9562.
    pub fn new(uuid: Uuid) -> Result<Self, Error> {
        if uuid.get_version() == Some(Version::SortRand) && uuid.get_variant() == Variant::RFC4122 {
            Ok(Self(uuid))
        } else {
            Err(Error::InvalidEventId {
                input: uuid.to_string(),
            })
        }
    }

    /// O UUID por trás do ID.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.hyphenated().fmt(f)
    }
}

impl FromStr for EventId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::try_parse(s).map_err(|_| Error::InvalidEventId {
            input: s.to_owned(),
        })?;
        Self::new(uuid)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use uuid::Builder;

    use super::*;

    #[test]
    fn only_version_7_is_an_event_id() {
        let v7 = Builder::from_unix_timestamp_millis(1, &[7; 10]).into_uuid();
        assert_eq!(EventId::new(v7).map(EventId::as_uuid), Ok(v7));

        for other in [
            Uuid::nil(),
            Uuid::max(),
            Builder::from_random_bytes([7; 16]).into_uuid(),
        ] {
            assert_eq!(
                EventId::new(other),
                Err(Error::InvalidEventId {
                    input: other.to_string()
                })
            );
        }
    }

    #[test]
    fn rejects_text_that_is_not_a_uuid() {
        assert_eq!(
            "T-041".parse::<EventId>(),
            Err(Error::InvalidEventId {
                input: "T-041".to_owned()
            })
        );
    }

    #[test]
    fn ids_follow_the_order_of_creation() {
        let earlier = EventId::new(Builder::from_unix_timestamp_millis(1, &[255; 10]).into_uuid());
        let later = EventId::new(Builder::from_unix_timestamp_millis(2, &[0; 10]).into_uuid());
        assert!(earlier.unwrap() < later.unwrap());
    }

    proptest! {
        #[test]
        fn display_then_parse_gives_the_same_id(millis in 0..1u64 << 48, random: [u8; 10]) {
            let id = EventId::new(Builder::from_unix_timestamp_millis(millis, &random).into_uuid())
                .unwrap();
            prop_assert_eq!(id.to_string().parse::<EventId>(), Ok(id));
        }

        #[test]
        fn parsing_any_text_never_panics(text in "\\PC*") {
            let _ = text.parse::<EventId>();
        }
    }
}
