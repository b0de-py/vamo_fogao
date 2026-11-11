//! A máquina de onde veio um evento.

use std::fmt;
use std::str::FromStr;

use crate::Error;

/// O nome da máquina onde uma mudança aconteceu, como `notebook` ou
/// `pc-da-sala`.
///
/// Quando várias máquinas sincronizam o mesmo log, é por ele que se sabe de
/// onde veio cada evento. O nome vem da config ou do hostname. Aqui só se
/// garante que é curto e sem espaço: letras e dígitos ASCII, `-`, `_` e `.`,
/// com até [`Machine::MAX_LEN`] caracteres.
///
/// ```
/// use vamo_core::Machine;
///
/// let machine: Machine = "pc-da-sala".parse()?;
/// assert_eq!(machine.as_str(), "pc-da-sala");
///
/// assert!("pc da sala".parse::<Machine>().is_err());
/// # Ok::<(), vamo_core::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Machine(String);

impl Machine {
    /// O tamanho máximo do nome, o mesmo de um hostname no Linux.
    pub const MAX_LEN: usize = 64;

    /// A máquina com esse nome.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidMachine`] se o nome for vazio, passar de
    /// [`Machine::MAX_LEN`] caracteres ou tiver algo fora de letras e dígitos
    /// ASCII, `-`, `_` e `.`.
    pub fn new(name: &str) -> Result<Self, Error> {
        // Só ASCII, então contar bytes é o mesmo que contar caracteres.
        let valid = (1..=Self::MAX_LEN).contains(&name.len())
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
        if valid {
            Ok(Self(name.to_owned()))
        } else {
            Err(Error::InvalidMachine {
                input: name.to_owned(),
            })
        }
    }

    /// O nome da máquina.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Machine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Machine {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn accepts_hostname_like_names() {
        for name in ["notebook", "pc-da-sala", "Desktop_2", "casa.local", "a"] {
            assert_eq!(
                Machine::new(name).map(|m| m.to_string()),
                Ok(name.to_owned())
            );
        }
    }

    #[test]
    fn rejects_bad_names() {
        for name in [
            "",
            "pc da sala",
            "máquina",
            "pc/sala",
            "pc\n",
            &"a".repeat(65),
        ] {
            assert_eq!(
                Machine::new(name),
                Err(Error::InvalidMachine {
                    input: name.to_owned()
                }),
                "{name:?} não deveria ser máquina"
            );
        }
    }

    proptest! {
        #[test]
        fn display_then_parse_gives_the_same_machine(name in "[A-Za-z0-9._-]{1,64}") {
            let machine = Machine::new(&name).unwrap();
            prop_assert_eq!(machine.to_string().parse::<Machine>(), Ok(machine));
        }

        #[test]
        fn parsing_any_text_never_panics(text in "\\PC*") {
            let _ = text.parse::<Machine>();
        }
    }
}
