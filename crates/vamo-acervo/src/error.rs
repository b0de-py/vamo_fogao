//! O erro do acervo, com a mensagem que o validador mostra.

use crate::{FactDate, Origin, ShortText};

/// O que pode dar errado ao ler um verbete do acervo.
///
/// Quem lê a mensagem é quem escreveu o arquivo, então cada uma diz o que
/// está errado e como fica certo.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// O problema está numa linha conhecida do arquivo.
    #[error("linha {line}: {error}")]
    At {
        /// A linha, contando do 1.
        line: usize,
        /// O problema em si.
        error: Box<Error>,
    },

    /// O arquivo não é um TOML válido.
    #[error("o TOML não abre ({message})")]
    Syntax {
        /// A explicação do leitor de TOML, que vem em inglês.
        message: String,
    },

    /// Falta um campo obrigatório.
    #[error("falta o campo `{field}`")]
    MissingField {
        /// O campo que falta.
        field: &'static str,
    },

    /// Um campo que o formato não conhece, quase sempre erro de digitação.
    #[error("o campo `{field}` não existe aqui: os campos são {}", list(known))]
    UnknownField {
        /// O campo que veio no arquivo.
        field: String,
        /// Os campos que existem nesse lugar.
        known: &'static [&'static str],
    },

    /// Um campo com o tipo de valor errado, como um número onde vai texto.
    #[error("o campo `{field}` precisa ser {expected}")]
    WrongType {
        /// O campo.
        field: String,
        /// O que ele precisa ser, já em forma de frase.
        expected: &'static str,
    },

    /// Um slug fora do formato, seja a id do verbete (o nome do arquivo) ou
    /// o valor de uma tag.
    #[error(
        "`{input}` não serve: use só letra minúscula sem acento, número e hífen, \
         como `24-a-0-no-mangueira`"
    )]
    InvalidSlug {
        /// O que veio.
        input: String,
    },

    /// O texto do verbete está vazio.
    #[error("o texto está vazio")]
    EmptyText,

    /// O texto tem quebra de linha.
    #[error("o texto tem quebra de linha, e a curiosidade ocupa uma linha só")]
    MultilineText,

    /// O texto passa do tamanho.
    #[error("o texto tem {chars} caracteres, e cabem até {max}", max = ShortText::MAX_CHARS)]
    TextTooLong {
        /// Quantos caracteres o texto tem.
        chars: usize,
    },

    /// Uma data fora do formato ou que não existe no calendário.
    #[error("`{input}` não é uma data: o formato é AAAA, AAAA-MM ou AAAA-MM-DD")]
    InvalidDate {
        /// O que veio.
        input: String,
    },

    /// Um `ate` sem a `data` de começo.
    #[error("o `ate` precisa de uma `data` pra começar")]
    UntilWithoutDate,

    /// Um `ate` que não vem depois da `data`.
    #[error("o `ate` ({until}) precisa vir depois da `data` ({date})")]
    UntilNotAfterDate {
        /// O começo.
        date: FactDate,
        /// O fim.
        until: FactDate,
    },

    /// Um tipo de fato que não existe.
    #[error("`{input}` não é um tipo: os tipos são fato e recorde")]
    UnknownKind {
        /// O que veio.
        input: String,
    },

    /// Um status que não existe.
    #[error("`{input}` não é um status: os status são verificado e a-conferir")]
    UnknownVerification {
        /// O que veio.
        input: String,
    },

    /// Um recorde sem data.
    #[error("recorde precisa de data, porque recorde cai")]
    RecordWithoutDate,

    /// Uma tag fora do formato.
    #[error(
        "`{input}` não é uma tag: o formato é tipo:valor, com o tipo entre {}, \
         como `idolo:garrincha`",
        list(crate::tag::PREFIXES)
    )]
    InvalidTag {
        /// O que veio.
        input: String,
    },

    /// Um link que não serve.
    #[error("`{input}` não é um link: o formato é https://site.com.br/caminho")]
    InvalidUrl {
        /// O que veio.
        input: String,
    },

    /// Uma fonte com o nome vazio.
    #[error("a fonte está sem nome")]
    EmptySourceName,

    /// Um fato sem fonte nenhuma.
    #[error(
        "fato sem fonte: todo fato diz de onde veio, nem que seja uma fonte só enquanto está a conferir"
    )]
    NoSource,

    /// Um fato verificado com uma fonte só.
    #[error("o fato está verificado com uma fonte só: precisa de duas, de domínios diferentes")]
    SingleSource,

    /// Um fato verificado com todas as fontes no mesmo lugar.
    #[error(
        "o fato está verificado, mas todas as fontes são de {origin}: precisa de duas, \
         de domínios diferentes (a Wikipédia em qualquer idioma conta como uma só)"
    )]
    SameOrigin {
        /// O lugar de onde vêm todas as fontes.
        origin: Origin,
    },
}

impl Error {
    /// Prende o problema numa linha do arquivo.
    pub(crate) fn at(line: usize, error: Self) -> Self {
        Self::At {
            line,
            error: Box::new(error),
        }
    }

    /// A linha do arquivo onde está o problema, quando dá pra saber.
    ///
    /// ```
    /// use vamo_acervo::Fact;
    ///
    /// let error = Fact::from_toml("x", "texto = 3").unwrap_err();
    /// assert_eq!(error.line(), Some(1));
    /// ```
    pub fn line(&self) -> Option<usize> {
        match self {
            Self::At { line, .. } => Some(*line),
            _ => None,
        }
    }
}

/// Junta os itens numa lista em português: `a, b e c`.
fn list(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [only] => (*only).to_owned(),
        [rest @ .., last] => format!("{} e {last}", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_read_like_portuguese() {
        assert_eq!(list(&[]), "");
        assert_eq!(list(&["texto"]), "texto");
        assert_eq!(list(&["nome", "url"]), "nome e url");
        assert_eq!(list(&["a", "b", "c"]), "a, b e c");
    }

    #[test]
    fn the_line_comes_first_in_the_message() {
        let error = Error::at(3, Error::EmptyText);
        assert_eq!(error.to_string(), "linha 3: o texto está vazio");
        assert_eq!(error.line(), Some(3));
        assert_eq!(Error::EmptyText.line(), None);
    }
}
