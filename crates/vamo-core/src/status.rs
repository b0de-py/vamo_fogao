//! O status de uma tarefa e as transições permitidas.

use std::fmt;
use std::str::FromStr;

use crate::Error;

/// Em que pé uma tarefa está.
///
/// O caminho comum é ideia → destrinchando → pronta → agendada → concluída,
/// mas dá pra pular etapas e voltar algumas. As regras completas estão em
/// [`Status::can_become`].
///
/// Nos arquivos e na CLI, os nomes são os de [`Status::as_str`]. A leitura
/// também aceita `concluida`, sem acento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    /// Só a ideia, ainda sem forma (ideia).
    Idea,
    /// Sendo quebrada em passos menores (destrinchando).
    Refining,
    /// Bem definida, esperando um horário (pronta).
    Ready,
    /// Com horário marcado na agenda (agendada).
    Scheduled,
    /// Volta de tempos em tempos e não termina num gol só (recorrente).
    Recurring,
    /// Feita: gol (concluída).
    Done,
    /// Não vai mais ser feita (descartada).
    Dropped,
}

impl Status {
    /// Todos os status, na ordem do caminho comum.
    pub const ALL: [Self; 7] = [
        Self::Idea,
        Self::Refining,
        Self::Ready,
        Self::Scheduled,
        Self::Recurring,
        Self::Done,
        Self::Dropped,
    ];

    /// O nome do status nos arquivos e na CLI.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idea => "ideia",
            Self::Refining => "destrinchando",
            Self::Ready => "pronta",
            Self::Scheduled => "agendada",
            Self::Recurring => "recorrente",
            Self::Done => "concluída",
            Self::Dropped => "descartada",
        }
    }

    /// Diz se uma tarefa neste status pode passar pra `next`.
    ///
    /// | De | Pode virar |
    /// |----|------------|
    /// | ideia | destrinchando, pronta, recorrente, descartada |
    /// | destrinchando | ideia, pronta, recorrente, descartada |
    /// | pronta | destrinchando, agendada, recorrente, concluída, descartada |
    /// | agendada | pronta (volta pro banco), concluída, descartada |
    /// | recorrente | pronta, descartada |
    /// | concluída | pronta (o VAR anulou o gol) |
    /// | descartada | ideia (volta do arquivo) |
    ///
    /// Ficar no mesmo status não é transição, então nunca é permitido.
    ///
    /// ```
    /// use vamo_core::Status;
    ///
    /// assert!(Status::Ready.can_become(Status::Scheduled));
    /// assert!(!Status::Idea.can_become(Status::Done));
    /// ```
    pub fn can_become(self, next: Self) -> bool {
        use Status::{Done, Dropped, Idea, Ready, Recurring, Refining, Scheduled};

        matches!(
            (self, next),
            (Idea, Refining | Ready | Recurring | Dropped)
                | (Refining, Idea | Ready | Recurring | Dropped)
                | (Ready, Refining | Scheduled | Recurring | Done | Dropped)
                | (Scheduled, Ready | Done | Dropped)
                | (Recurring, Ready | Dropped)
                | (Done, Ready)
                | (Dropped, Idea)
        )
    }

    /// O status `next`, se a regra de [`Status::can_become`] deixar.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidTransition`] quando a regra não deixa, inclusive pra
    /// continuar no mesmo status.
    pub fn transition_to(self, next: Self) -> Result<Self, Error> {
        if self.can_become(next) {
            Ok(next)
        } else {
            Err(Error::InvalidTransition {
                from: self,
                to: next,
            })
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Status {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "concluida" {
            return Ok(Self::Done);
        }
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == s)
            .ok_or_else(|| Error::UnknownStatus {
                input: s.to_owned(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Status::{Done, Dropped, Idea, Ready, Recurring, Refining, Scheduled};

    #[test]
    fn every_status_survives_display_then_parse() {
        for status in Status::ALL {
            assert_eq!(status.to_string().parse(), Ok(status));
        }
    }

    #[test]
    fn done_reads_without_the_accent_too() {
        assert_eq!("concluida".parse(), Ok(Done));
    }

    #[test]
    fn unknown_names_are_rejected() {
        for input in ["", "Pronta", "feita", "done"] {
            assert_eq!(
                input.parse::<Status>(),
                Err(Error::UnknownStatus {
                    input: input.to_owned()
                })
            );
        }
    }

    #[test]
    fn the_common_path_is_allowed() {
        let path = [Idea, Refining, Ready, Scheduled, Done];
        for pair in path.windows(2) {
            assert!(pair[0].can_become(pair[1]), "{} → {}", pair[0], pair[1]);
        }
    }

    #[test]
    fn going_back_is_allowed_where_the_game_allows() {
        assert!(Scheduled.can_become(Ready), "volta pro banco");
        assert!(Done.can_become(Ready), "o VAR anulou o gol");
        assert!(Dropped.can_become(Idea), "volta do arquivo");
    }

    #[test]
    fn shortcuts_that_skip_the_definition_are_not_allowed() {
        assert!(!Idea.can_become(Scheduled));
        assert!(!Idea.can_become(Done));
        assert!(!Refining.can_become(Done));
        assert!(!Recurring.can_become(Done));
        assert!(!Done.can_become(Scheduled));
        assert!(!Dropped.can_become(Ready));
    }

    #[test]
    fn staying_in_the_same_status_is_never_a_transition() {
        for status in Status::ALL {
            assert!(!status.can_become(status), "{status} → {status}");
        }
    }

    #[test]
    fn transition_to_follows_can_become() {
        for from in Status::ALL {
            for to in Status::ALL {
                let expected = if from.can_become(to) {
                    Ok(to)
                } else {
                    Err(Error::InvalidTransition { from, to })
                };
                assert_eq!(from.transition_to(to), expected);
            }
        }
    }

    #[test]
    fn every_status_has_a_way_out() {
        for from in Status::ALL {
            assert!(
                Status::ALL.into_iter().any(|to| from.can_become(to)),
                "{from} não tem saída"
            );
        }
    }

    #[test]
    fn the_error_message_names_both_statuses() {
        let error = Idea.transition_to(Done).unwrap_err();
        assert_eq!(
            error.to_string(),
            "não dá pra passar de ideia pra concluída"
        );
    }
}
