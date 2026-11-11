//! A área de uma tarefa: saúde, casa, estudos...

use std::fmt;
use std::str::FromStr;

use crate::Error;

/// O pedaço da vida a que uma tarefa pertence.
///
/// Quais áreas existem é escolha de quem usa e vem da config. Aqui só se
/// garante que o nome é bom pra ser digitado e guardado: letras minúsculas
/// (com acento, como em `saúde`), números e hífen entre as palavras, com até
/// [`Area::MAX_LEN`] caracteres.
///
/// ```
/// use vamo_core::Area;
///
/// let area: Area = "projetos-paralelos".parse()?;
/// assert_eq!(area.as_str(), "projetos-paralelos");
///
/// assert!("Casa".parse::<Area>().is_err());
/// assert!("casa-".parse::<Area>().is_err());
/// # Ok::<(), vamo_core::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Area(String);

impl Area {
    /// O tamanho máximo do nome, em caracteres.
    pub const MAX_LEN: usize = 32;

    /// A área com esse nome.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArea`] se o nome for vazio, longo demais, tiver
    /// maiúscula, espaço ou pontuação, ou hífen sobrando nas pontas ou
    /// dobrado.
    pub fn new(name: &str) -> Result<Self, Error> {
        if is_valid(name) {
            Ok(Self(name.to_owned()))
        } else {
            Err(Error::InvalidArea {
                input: name.to_owned(),
            })
        }
    }

    /// O nome da área.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A regra do nome: palavras de letras minúsculas e dígitos, separadas por
/// um hífen só. Como `split` devolve um pedaço vazio pra cada hífen nas
/// pontas ou repetido, exigir pedaços não vazios cobre esses casos.
fn is_valid(name: &str) -> bool {
    let len = name.chars().count();
    (1..=Area::MAX_LEN).contains(&len)
        && name.split('-').all(|word| {
            !word.is_empty() && word.chars().all(|c| c.is_lowercase() || c.is_ascii_digit())
        })
}

impl fmt::Display for Area {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Area {
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
    fn accepts_lowercase_words_with_accents_digits_and_hyphens() {
        for name in [
            "casa",
            "saúde",
            "família",
            "hábitos",
            "projetos-paralelos",
            "fase-2",
            "a",
        ] {
            assert_eq!(Area::new(name).map(|a| a.to_string()), Ok(name.to_owned()));
        }
    }

    #[test]
    fn rejects_bad_names() {
        for name in [
            "",
            "Casa",
            "SAÚDE",
            "projetos paralelos",
            "projetos_paralelos",
            "-casa",
            "casa-",
            "projetos--paralelos",
            "casa!",
            "estudos/inglês",
        ] {
            assert_eq!(
                Area::new(name),
                Err(Error::InvalidArea {
                    input: name.to_owned()
                }),
                "{name:?} não deveria ser área"
            );
        }
    }

    #[test]
    fn length_is_counted_in_characters_not_bytes() {
        // "ú" ocupa dois bytes em UTF-8, mas conta como um caractere.
        let longest = "ú".repeat(Area::MAX_LEN);
        assert!(Area::new(&longest).is_ok());
        assert!(Area::new(&format!("{longest}u")).is_err());
    }

    proptest! {
        // As letras com acento vão listadas uma a uma: um intervalo como
        // `à-ú` pegaria também o `÷`, que fica no meio deles no Unicode.
        #[test]
        fn display_then_parse_gives_the_same_area(
            name in "[a-záàâãéêíóôõúç0-9]{1,10}(-[a-záàâãéêíóôõúç0-9]{1,10}){0,1}"
        ) {
            let area = Area::new(&name).unwrap();
            prop_assert_eq!(area.to_string().parse::<Area>(), Ok(area));
        }

        #[test]
        fn parsing_any_text_never_panics(text in "\\PC*") {
            let _ = text.parse::<Area>();
        }
    }
}
