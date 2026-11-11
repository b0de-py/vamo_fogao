//! O título no padrão do caderno.

use std::fmt;
use std::str::FromStr;

use crate::Error;

/// O título de uma tarefa no padrão do caderno de papel: até cinco palavras,
/// começando com um verbo no infinitivo. É o que cabe numa linha e diz o que
/// fazer de cara: "Trocar o filtro da água", "Pôr o lixo pra fora".
///
/// Os espaços sobrando somem na criação: o título guarda as palavras
/// separadas por um espaço só. Artigos, preposições e números contam como
/// palavra.
///
/// ```
/// use vamo_core::{Error, NotebookTitle};
///
/// let title: NotebookTitle = "Pôr  o lixo pra fora".parse()?;
/// assert_eq!(title.as_str(), "Pôr o lixo pra fora");
///
/// // Título comprido não é só recusado: vem com a sugestão de corte.
/// let Err(Error::TitleTooLong { suggestion, .. }) =
///     "Comprar ração e areia para o gato".parse::<NotebookTitle>()
/// else {
///     panic!("deveria ser longo demais");
/// };
/// assert_eq!(suggestion.as_str(), "Comprar ração e areia");
/// # Ok::<(), vamo_core::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NotebookTitle(String);

impl NotebookTitle {
    /// Quantas palavras cabem numa linha do caderno.
    pub const MAX_WORDS: usize = 5;

    /// O título a partir do texto digitado.
    ///
    /// # Errors
    ///
    /// - [`Error::EmptyTitle`] se o texto for vazio ou só tiver espaço.
    /// - [`Error::TitleWithoutVerb`] se a primeira palavra não tiver cara de
    ///   verbo no infinitivo.
    /// - [`Error::TitleTooLong`] se passar de [`NotebookTitle::MAX_WORDS`]
    ///   palavras. O erro traz a sugestão de corte, que já é um título válido;
    ///   a frase inteira pode ir pras notas da tarefa.
    pub fn new(text: &str) -> Result<Self, Error> {
        let words: Vec<&str> = text.split_whitespace().collect();

        let Some(first_word) = words.first() else {
            return Err(Error::EmptyTitle);
        };
        if !looks_like_infinitive(first_word) {
            return Err(Error::TitleWithoutVerb {
                first_word: (*first_word).to_owned(),
            });
        }
        if words.len() > Self::MAX_WORDS {
            return Err(Error::TitleTooLong {
                words: words.len(),
                suggestion: Self(cut(&words)),
            });
        }
        Ok(Self(words.join(" ")))
    }

    /// O título.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// As palavras que não fecham bem um título cortado: artigos, preposições e
/// conjunções. "Comprar ração e areia para" vira "Comprar ração e areia".
const WEAK_ENDINGS: &[&str] = &[
    "a", "à", "ao", "aos", "as", "às", "com", "da", "das", "de", "do", "dos", "e", "em", "na",
    "nas", "no", "nos", "num", "numa", "o", "os", "ou", "para", "pela", "pelas", "pelo", "pelos",
    "por", "pra", "pras", "pro", "pros", "que", "um", "uma",
];

/// O corte sugerido: as primeiras palavras que cabem, sem terminar em
/// palavra fraca. Nunca fica vazio, porque a primeira palavra sempre fica.
fn cut(words: &[&str]) -> String {
    let mut kept = &words[..words.len().min(NotebookTitle::MAX_WORDS)];
    while let [rest @ .., last] = kept
        && !rest.is_empty()
        && WEAK_ENDINGS.contains(&last.to_lowercase().as_str())
    {
        kept = rest;
    }
    kept.join(" ")
}

/// Os verbos terminados em `-or`: o pôr e a família dele. Todo outro
/// infinitivo do português termina em `-ar`, `-er` ou `-ir`.
const POR_FAMILY: &[&str] = &[
    "antepor",
    "compor",
    "contrapor",
    "decompor",
    "depor",
    "dispor",
    "expor",
    "impor",
    "indispor",
    "interpor",
    "justapor",
    "opor",
    "pospor",
    "predispor",
    "propor",
    "pôr",
    "recompor",
    "repor",
    "sobrepor",
    "supor",
    "transpor",
];

/// Diz se a palavra tem cara de verbo no infinitivo.
///
/// É uma regra de bolso, não um dicionário: aceita palavra terminada em
/// `-ar`, `-er` ou `-ir` e a família do pôr. Com pronome colado por hífen
/// ("despedir-se"), vale a parte antes do hífen. Um substantivo como "lugar"
/// passa, mas título de tarefa raramente começa assim; o que a regra pega
/// de verdade é o título que começa com "Reunião", "O" ou "Dentista".
fn looks_like_infinitive(word: &str) -> bool {
    let word = word.to_lowercase();
    let stem = word.split_once('-').map_or(word.as_str(), |(stem, _)| stem);
    POR_FAMILY.contains(&stem)
        || (stem.chars().count() >= 2
            && (stem.ends_with("ar") || stem.ends_with("er") || stem.ends_with("ir")))
}

impl fmt::Display for NotebookTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for NotebookTitle {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn title(text: &str) -> Result<String, Error> {
        NotebookTitle::new(text).map(|title| title.to_string())
    }

    #[test]
    fn accepts_short_titles_that_start_with_a_verb() {
        for text in [
            "Trocar o filtro da água",
            "Ver o jogo",
            "Ir ao banco",
            "Ler",
            "Pagar 3 boletos",
            "Despedir-se da turma",
        ] {
            assert_eq!(title(text), Ok(text.to_owned()));
        }
    }

    #[test]
    fn accepts_the_irregular_por_family() {
        assert!(title("Pôr o lixo pra fora").is_ok());
        assert!(title("Repor o café").is_ok());
        assert!(title("Propor a troca de turno").is_ok());
    }

    #[test]
    fn the_preposition_por_is_not_the_verb_por() {
        assert_eq!(
            title("Por o lixo pra fora"),
            Err(Error::TitleWithoutVerb {
                first_word: "Por".to_owned()
            })
        );
    }

    #[test]
    fn rejects_titles_that_do_not_start_with_a_verb() {
        for (text, first_word) in [
            ("Reunião com o pessoal", "Reunião"),
            ("O filtro da água", "O"),
            ("Dentista", "Dentista"),
            ("3 boletos pra pagar", "3"),
        ] {
            assert_eq!(
                title(text),
                Err(Error::TitleWithoutVerb {
                    first_word: first_word.to_owned()
                })
            );
        }
    }

    #[test]
    fn collapses_extra_whitespace() {
        assert_eq!(
            title("  Trocar   o\tfiltro \n"),
            Ok("Trocar o filtro".to_owned())
        );
    }

    #[test]
    fn rejects_empty_titles() {
        assert_eq!(title(""), Err(Error::EmptyTitle));
        assert_eq!(title(" \t\n "), Err(Error::EmptyTitle));
    }

    #[test]
    fn long_titles_come_with_a_suggested_cut() {
        assert_eq!(
            NotebookTitle::new("Comprar ração e areia para o gato"),
            Err(Error::TitleTooLong {
                words: 7,
                suggestion: NotebookTitle("Comprar ração e areia".to_owned()),
            })
        );
    }

    #[test]
    fn the_cut_drops_every_weak_word_at_the_end() {
        // As cinco primeiras terminam em "de a", e as duas saem.
        assert_eq!(
            cut(&["Ligar", "pro", "banco", "de", "a", "tarde", "toda"]),
            "Ligar pro banco"
        );
    }

    #[test]
    fn the_cut_always_keeps_the_first_word() {
        assert_eq!(cut(&["Ir", "a", "o", "de", "da", "do", "e"]), "Ir");
    }

    #[test]
    fn the_error_message_shows_the_suggestion() {
        let error = NotebookTitle::new("Comprar ração e areia para o gato").unwrap_err();
        assert_eq!(
            error.to_string(),
            "o título tem 7 palavras e no caderno cabem até 5: que tal \
             \"Comprar ração e areia\"? O resto vai pras notas"
        );
    }

    proptest! {
        #[test]
        fn a_valid_title_survives_display_then_parse(
            verb in "[a-z]{1,8}(ar|er|ir)",
            rest in proptest::collection::vec("[a-zçãéó0-9]{1,8}", 0..=4),
        ) {
            let text = std::iter::once(verb).chain(rest).collect::<Vec<_>>().join(" ");
            let title = NotebookTitle::new(&text).unwrap();
            prop_assert_eq!(title.to_string().parse::<NotebookTitle>(), Ok(title));
        }

        #[test]
        fn a_suggested_cut_is_never_empty_and_always_fits(
            words in proptest::collection::vec("\\S{1,8}", 1..20),
        ) {
            let words: Vec<&str> = words.iter().map(String::as_str).collect();
            let suggestion = cut(&words);
            prop_assert!(!suggestion.is_empty());
            prop_assert!(suggestion.split(' ').count() <= NotebookTitle::MAX_WORDS);
        }

        #[test]
        fn the_suggestion_is_itself_a_valid_title(
            verb in "[a-z]{1,8}(ar|er|ir)",
            rest in proptest::collection::vec("\\S{1,8}", 5..15),
        ) {
            let text = std::iter::once(verb).chain(rest).collect::<Vec<_>>().join(" ");
            let Err(Error::TitleTooLong { suggestion, .. }) = NotebookTitle::new(&text) else {
                return Err(TestCaseError::fail("deveria ser longo demais"));
            };
            prop_assert_eq!(suggestion.as_str().parse::<NotebookTitle>(), Ok(suggestion));
        }

        #[test]
        fn parsing_any_text_never_panics(text in "\\PC*") {
            let _ = text.parse::<NotebookTitle>();
        }
    }
}
