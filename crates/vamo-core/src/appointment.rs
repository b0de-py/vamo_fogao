//! O compromisso: um evento da agenda, já no horário local.

use jiff::civil::{Date, DateTime};

use crate::TaskId;

/// Um evento da agenda num dia, já com a recorrência resolvida e no horário
/// local de quem usa.
///
/// Quem monta é o adaptador da agenda: ele lê o ICS (ou a API do Google),
/// expande a recorrência, converte o fuso e lê o rodapé `vamo:` da
/// descrição. O core só recebe o resultado e não sabe de onde veio.
///
/// ```
/// use jiff::civil::date;
/// use vamo_core::{Appointment, When};
///
/// let lunch = Appointment {
///     uid: "almoco@exemplo".to_owned(),
///     title: "Almoço com a equipe".to_owned(),
///     when: When::Timed {
///         start: date(2026, 11, 12).at(12, 0, 0, 0),
///         end: date(2026, 11, 12).at(13, 0, 0, 0),
///     },
///     recurring: false,
///     task: None,
/// };
/// assert_eq!(lunch.day(), date(2026, 11, 12));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Appointment {
    /// O identificador do evento na agenda. As ocorrências de um evento
    /// recorrente têm o mesmo.
    pub uid: String,
    /// O título do evento, como está na agenda.
    pub title: String,
    /// Quando acontece.
    pub when: When,
    /// Se o evento se repete (veio de uma regra de recorrência).
    pub recurring: bool,
    /// A tarefa ligada pelo rodapé `vamo:` da descrição, se tiver.
    pub task: Option<TaskId>,
}

/// Quando um compromisso acontece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum When {
    /// O dia inteiro, sem horário (aniversário, feriado, viagem).
    AllDay(Date),
    /// Com hora de começar e de acabar, no horário local.
    Timed {
        /// O começo.
        start: DateTime,
        /// O fim. Nunca antes do começo.
        end: DateTime,
    },
}

impl Appointment {
    /// O dia em que o compromisso começa.
    pub fn day(&self) -> Date {
        match self.when {
            When::AllDay(day) => day,
            When::Timed { start, .. } => start.date(),
        }
    }

    /// O começo e o fim, se o compromisso tiver horário.
    pub fn span(&self) -> Option<(DateTime, DateTime)> {
        match self.when {
            When::AllDay(_) => None,
            When::Timed { start, end } => Some((start, end)),
        }
    }
}

/// A porta da agenda: de onde vêm os compromissos.
///
/// O `vamo-agenda` implementa com o link ICS e, como opção, com a API do
/// Google Agenda. As duas devolvem a mesma coisa, então o resto do app não
/// sabe qual está ligada.
pub trait Agenda {
    /// O erro da leitura.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Os compromissos que começam entre `from` e `to`, os dois dias
    /// incluídos, em ordem de começo.
    ///
    /// # Errors
    ///
    /// Quando a agenda não pode ser lida.
    fn appointments(&self, from: Date, to: Date) -> Result<Vec<Appointment>, Self::Error>;
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    #[test]
    fn an_all_day_appointment_has_no_span() {
        let holiday = Appointment {
            uid: "feriado".to_owned(),
            title: "Feriado".to_owned(),
            when: When::AllDay(date(2026, 11, 20)),
            recurring: false,
            task: None,
        };
        assert_eq!(holiday.day(), date(2026, 11, 20));
        assert_eq!(holiday.span(), None);
    }
}
