//! O ID de uma tarefa: `T-` seguido do número, como `T-041`.

use std::fmt;
use std::num::NonZeroU32;
use std::str::FromStr;

use crate::Error;

/// O número de uma tarefa, mostrado como `T-041`.
///
/// O número começa em 1 e aparece com pelo menos três dígitos. Na leitura, o
/// zero à esquerda é opcional: `T-41` e `T-041` são a mesma tarefa.
///
/// Por dentro é um [`NonZeroU32`]. Além de deixar o zero de fora, isso faz um
/// `Option<TaskId>` ocupar o mesmo espaço que um `TaskId`: o Rust usa o zero
/// proibido pra representar o `None`.
///
/// ```
/// use vamo_core::TaskId;
///
/// let id: TaskId = "T-41".parse()?;
/// assert_eq!(id.to_string(), "T-041");
/// assert_eq!(id.number(), 41);
/// # Ok::<(), vamo_core::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(NonZeroU32);

impl TaskId {
    /// O primeiro ID, `T-001`.
    pub const FIRST: Self = Self(NonZeroU32::MIN);

    /// O ID com esse número, ou `None` pro zero, que não é número de tarefa.
    pub fn new(number: u32) -> Option<Self> {
        NonZeroU32::new(number).map(Self)
    }

    /// O número da tarefa, sem o `T-`.
    pub fn number(self) -> u32 {
        self.0.get()
    }

    /// O ID seguinte, pra numerar a próxima tarefa. `None` se o número
    /// estourar o limite do `u32`, o que na prática não acontece.
    pub fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "T-{:03}", self.0)
    }
}

impl FromStr for TaskId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || Error::InvalidTaskId {
            input: s.to_owned(),
        };

        let digits = s.strip_prefix("T-").ok_or_else(invalid)?;
        // O `parse` do `u32` aceitaria um `+` na frente; aqui só vale dígito.
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(invalid());
        }
        let number: u32 = digits.parse().map_err(|_| invalid())?;
        Self::new(number).ok_or_else(invalid)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn id(number: u32) -> TaskId {
        TaskId::new(number).unwrap()
    }

    #[test]
    fn displays_with_at_least_three_digits() {
        assert_eq!(id(7).to_string(), "T-007");
        assert_eq!(id(41).to_string(), "T-041");
        assert_eq!(id(999).to_string(), "T-999");
        assert_eq!(id(1000).to_string(), "T-1000");
    }

    #[test]
    fn parses_with_or_without_leading_zeros() {
        assert_eq!("T-041".parse(), Ok(id(41)));
        assert_eq!("T-41".parse(), Ok(id(41)));
        assert_eq!("T-0041".parse(), Ok(id(41)));
    }

    #[test]
    fn rejects_anything_else() {
        for input in [
            "",
            "T-",
            "T-0",
            "T-000",
            "41",
            "t-041",
            "T041",
            "T-+41",
            "T- 41",
            "T-41 ",
            " T-41",
            "T-4a",
            "T-٤١",
            "T-99999999999",
        ] {
            assert_eq!(
                input.parse::<TaskId>(),
                Err(Error::InvalidTaskId {
                    input: input.to_owned()
                }),
                "{input:?} não deveria ser um ID"
            );
        }
    }

    #[test]
    fn zero_is_not_an_id() {
        assert_eq!(TaskId::new(0), None);
    }

    #[test]
    fn next_counts_up_and_stops_at_the_limit() {
        assert_eq!(id(41).next(), Some(id(42)));
        assert_eq!(id(u32::MAX).next(), None);
    }

    #[test]
    fn an_optional_id_costs_nothing_extra() {
        assert_eq!(size_of::<Option<TaskId>>(), size_of::<TaskId>());
    }

    proptest! {
        #[test]
        fn display_then_parse_gives_the_same_id(number in 1..=u32::MAX) {
            let original = id(number);
            prop_assert_eq!(original.to_string().parse::<TaskId>(), Ok(original));
        }

        #[test]
        fn parsing_any_text_never_panics(text in "\\PC*") {
            let _ = text.parse::<TaskId>();
        }
    }
}
