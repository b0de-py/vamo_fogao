//! O encaixe de uma tarefa: foco, brecha ou com alguém.

use std::fmt;
use std::str::FromStr;

use crate::Error;

/// O tipo de espaço na agenda que uma tarefa pede.
///
/// É o que decide onde a tarefa cabe: tarefa de foco precisa de um bloco
/// longo, tarefa de brecha cabe numa janela curta e tarefa com alguém
/// depende de outra pessoa junto.
///
/// Nos arquivos e na CLI, os nomes são `foco`, `brecha` e `com-alguém` (a
/// leitura também aceita `com-alguem`, sem acento).
///
/// ```
/// use vamo_core::Fit;
///
/// assert_eq!("brecha".parse(), Ok(Fit::Gap));
/// assert_eq!(Fit::WithSomeone.to_string(), "com-alguém");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fit {
    /// Precisa de um bloco longo e sem interrupção (foco).
    Focus,
    /// Cabe numa janela curta entre outras coisas (brecha).
    Gap,
    /// Depende de outra pessoa junto (com alguém).
    WithSomeone,
}

impl Fit {
    /// Todos os encaixes, na ordem em que aparecem pra quem usa.
    pub const ALL: [Self; 3] = [Self::Focus, Self::Gap, Self::WithSomeone];

    /// O nome do encaixe nos arquivos e na CLI.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Focus => "foco",
            Self::Gap => "brecha",
            Self::WithSomeone => "com-alguém",
        }
    }
}

impl fmt::Display for Fit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Fit {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "foco" => Ok(Self::Focus),
            "brecha" => Ok(Self::Gap),
            "com-alguém" | "com-alguem" => Ok(Self::WithSomeone),
            _ => Err(Error::UnknownFit {
                input: s.to_owned(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_fit_survives_display_then_parse() {
        for fit in Fit::ALL {
            assert_eq!(fit.to_string().parse(), Ok(fit));
        }
    }

    #[test]
    fn with_someone_reads_without_the_accent_too() {
        assert_eq!("com-alguem".parse(), Ok(Fit::WithSomeone));
    }

    #[test]
    fn unknown_names_are_rejected() {
        for input in ["", "Foco", "gap", "com alguém", "brechas"] {
            assert_eq!(
                input.parse::<Fit>(),
                Err(Error::UnknownFit {
                    input: input.to_owned()
                })
            );
        }
    }
}
