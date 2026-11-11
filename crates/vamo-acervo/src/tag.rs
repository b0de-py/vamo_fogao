//! As tags de um fato.

use std::fmt;
use std::str::FromStr;

use crate::{Error, Slug};

/// Os tipos de tag, como aparecem no arquivo.
pub(crate) const PREFIXES: &[&str] = &["epoca", "idolo", "titulo", "adversario", "esporte"];

/// Uma tag de um fato: é por elas que a tela Museu agrupa o acervo e que o
/// dia de jogo acha o fato do confronto. No arquivo, a tag é `tipo:valor`,
/// como `adversario:flamengo`.
///
/// ```
/// use vamo_acervo::Tag;
///
/// let tag: Tag = "adversario:atletico-mg".parse()?;
/// assert!(matches!(tag, Tag::Opponent(_)));
/// assert_eq!(tag.to_string(), "adversario:atletico-mg");
/// # Ok::<(), vamo_acervo::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tag {
    /// Uma época do clube (`epoca:`).
    Era(Slug),
    /// Um ídolo que aparece no fato (`idolo:`).
    Idol(Slug),
    /// O título em jogo (`titulo:`).
    Title(Slug),
    /// O adversário (`adversario:`).
    Opponent(Slug),
    /// O esporte, quando não é o futebol (`esporte:`).
    Sport(Slug),
}

impl Tag {
    /// O valor da tag, a parte depois dos dois-pontos.
    pub fn value(&self) -> &Slug {
        match self {
            Self::Era(value)
            | Self::Idol(value)
            | Self::Title(value)
            | Self::Opponent(value)
            | Self::Sport(value) => value,
        }
    }

    /// O tipo da tag, como no arquivo.
    fn prefix(&self) -> &'static str {
        match self {
            Self::Era(_) => "epoca",
            Self::Idol(_) => "idolo",
            Self::Title(_) => "titulo",
            Self::Opponent(_) => "adversario",
            Self::Sport(_) => "esporte",
        }
    }
}

impl FromStr for Tag {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Error> {
        let invalid = || Error::InvalidTag {
            input: input.to_owned(),
        };
        let (prefix, value) = input.split_once(':').ok_or_else(invalid)?;
        let value: Slug = value.parse().map_err(|_| invalid())?;
        match prefix {
            "epoca" => Ok(Self::Era(value)),
            "idolo" => Ok(Self::Idol(value)),
            "titulo" => Ok(Self::Title(value)),
            "adversario" => Ok(Self::Opponent(value)),
            "esporte" => Ok(Self::Sport(value)),
            _ => Err(invalid()),
        }
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.prefix(), self.value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_prefix_reads_and_comes_back_the_same() {
        for prefix in PREFIXES {
            let text = format!("{prefix}:garrincha");
            let tag: Tag = text.parse().unwrap();
            assert_eq!(tag.to_string(), text);
            assert_eq!(tag.value().as_str(), "garrincha");
        }
    }

    #[test]
    fn refuses_unknown_prefixes_and_bad_values() {
        for input in [
            "garrincha",
            "jogador:garrincha",
            "idolo:",
            "idolo:Garrincha",
            "ídolo:garrincha",
            ":garrincha",
        ] {
            assert_eq!(
                input.parse::<Tag>(),
                Err(Error::InvalidTag {
                    input: input.to_owned()
                }),
                "{input:?}"
            );
        }
    }
}
