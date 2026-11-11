//! O slug: a id de cada verbete e o valor de cada tag.

use std::fmt;
use std::str::FromStr;

use crate::Error;

/// Um nome curto pra máquina: letra minúscula sem acento, número e hífen,
/// como `24-a-0-no-mangueira`. É a id de cada verbete (o nome do arquivo, sem
/// o `.toml`) e o valor de cada tag.
///
/// Sem acento e sem maiúscula, o nome do arquivo é o mesmo em qualquer
/// sistema, e dois arquivos nunca ficam com a mesma id só porque um sistema
/// não diferencia `A` de `a`.
///
/// ```
/// use vamo_acervo::Slug;
///
/// let slug: Slug = "atletico-mg".parse()?;
/// assert_eq!(slug.as_str(), "atletico-mg");
///
/// assert!("Atlético-MG".parse::<Slug>().is_err());
/// assert!("-mg".parse::<Slug>().is_err());
/// # Ok::<(), vamo_acervo::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Slug(String);

impl Slug {
    /// O tamanho máximo, em caracteres.
    pub const MAX_LEN: usize = 80;

    /// O slug como texto.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Slug {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Error> {
        // Hífen só entre dois pedaços: nada de `-x`, `x-` ou `x--y`.
        let valid = (1..=Self::MAX_LEN).contains(&input.len())
            && input
                .split('-')
                .all(|part| !part.is_empty() && part.bytes().all(is_slug_byte));
        if valid {
            Ok(Self(input.to_owned()))
        } else {
            Err(Error::InvalidSlug {
                input: input.to_owned(),
            })
        }
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn is_slug_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn accepts_lowercase_digits_and_inner_hyphens() {
        for input in ["x", "24-a-0-no-mangueira", "1989", "sao-paulo"] {
            assert_eq!(input.parse::<Slug>().unwrap().as_str(), input);
        }
    }

    #[test]
    fn refuses_anything_else() {
        let too_long = "a".repeat(Slug::MAX_LEN + 1);
        for input in [
            "",
            "-x",
            "x-",
            "x--y",
            "São-Paulo",
            "sao paulo",
            "sao_paulo",
            "são",
            too_long.as_str(),
        ] {
            assert_eq!(
                input.parse::<Slug>(),
                Err(Error::InvalidSlug {
                    input: input.to_owned()
                }),
                "{input:?}"
            );
        }
    }

    proptest! {
        #[test]
        fn a_well_formed_slug_comes_back_the_same(
            input in "[a-z0-9]{1,10}(-[a-z0-9]{1,10}){0,4}",
        ) {
            let slug: Slug = input.parse().unwrap();
            prop_assert_eq!(slug.to_string(), input);
        }
    }
}
