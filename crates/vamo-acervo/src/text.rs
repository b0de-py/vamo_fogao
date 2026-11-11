//! O texto de um verbete.

use std::fmt;
use std::str::FromStr;

use crate::Error;

/// O texto de um verbete: uma linha só, com até [`ShortText::MAX_CHARS`]
/// caracteres. O espaço nas pontas sai na leitura.
///
/// ```
/// use vamo_acervo::{Error, ShortText};
///
/// let text: ShortText = " 24 x 0 no Mangueira. ".parse()?;
/// assert_eq!(text.as_str(), "24 x 0 no Mangueira.");
///
/// assert_eq!("um\ndois".parse::<ShortText>(), Err(Error::MultilineText));
/// # Ok::<(), Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortText(String);

impl ShortText {
    /// Quantos caracteres cabem: o bastante pra contar um fato, e pouco o
    /// bastante pra caber numa linha da tela Hoje e no tooltip da barra.
    pub const MAX_CHARS: usize = 200;

    /// O texto.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ShortText {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Error> {
        let text = input.trim();
        if text.is_empty() {
            return Err(Error::EmptyText);
        }
        if text.contains(['\n', '\r']) {
            return Err(Error::MultilineText);
        }
        // Conta caractere, não byte: o "ã" de "São" ocupa dois bytes.
        let chars = text.chars().count();
        if chars > Self::MAX_CHARS {
            return Err(Error::TextTooLong { chars });
        }
        Ok(Self(text.to_owned()))
    }
}

impl fmt::Display for ShortText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_characters_not_bytes() {
        let at_the_limit = "ã".repeat(ShortText::MAX_CHARS);
        assert!(at_the_limit.parse::<ShortText>().is_ok());

        let over = "ã".repeat(ShortText::MAX_CHARS + 1);
        assert_eq!(
            over.parse::<ShortText>(),
            Err(Error::TextTooLong {
                chars: ShortText::MAX_CHARS + 1
            })
        );
    }

    #[test]
    fn refuses_empty_and_multiline_text() {
        assert_eq!("".parse::<ShortText>(), Err(Error::EmptyText));
        assert_eq!("   ".parse::<ShortText>(), Err(Error::EmptyText));
        assert_eq!("a\r\nb".parse::<ShortText>(), Err(Error::MultilineText));
    }
}
