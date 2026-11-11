//! O erro do `vamo-core`.

use crate::{NotebookTitle, Status};

/// Tudo que pode dar errado ao criar ou mudar um valor do domínio.
///
/// As mensagens já saem prontas pra quem usa o app: dizem o que veio errado e
/// como seria o certo.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// O texto não é um ID de tarefa no formato `T-###`.
    #[error("`{input}` não é um ID de tarefa: o formato é T- seguido do número, como T-041")]
    InvalidTaskId {
        /// O texto recebido.
        input: String,
    },

    /// O texto não serve como nome de área.
    #[error(
        "`{input}` não serve como área: use letras minúsculas, números e hífen entre as \
         palavras, como projetos-paralelos"
    )]
    InvalidArea {
        /// O texto recebido.
        input: String,
    },

    /// O texto não é um encaixe conhecido.
    #[error("`{input}` não é um encaixe: os encaixes são foco, brecha e com-alguém")]
    UnknownFit {
        /// O texto recebido.
        input: String,
    },

    /// O texto não é um status conhecido.
    #[error(
        "`{input}` não é um status: os status são ideia, destrinchando, pronta, agendada, \
         recorrente, concluída e descartada"
    )]
    UnknownStatus {
        /// O texto recebido.
        input: String,
    },

    /// A tarefa não pode passar de um status pro outro.
    #[error("não dá pra passar de {from} pra {to}")]
    InvalidTransition {
        /// O status atual.
        from: Status,
        /// O status pedido.
        to: Status,
    },

    /// O título da tarefa está vazio.
    #[error("o título da tarefa está vazio")]
    EmptyTitle,

    /// O título não começa com um verbo no infinitivo.
    #[error(
        "o título começa com `{first_word}`, e no caderno a tarefa começa com verbo no \
         infinitivo, como Ligar, Comprar ou Pôr"
    )]
    TitleWithoutVerb {
        /// A primeira palavra do título.
        first_word: String,
    },

    /// O título passa do tamanho de uma linha do caderno.
    #[error(
        "o título tem {words} palavras e no caderno cabem até {max}: que tal \"{suggestion}\"? \
         O resto vai pras notas",
        max = NotebookTitle::MAX_WORDS
    )]
    TitleTooLong {
        /// Quantas palavras o título tem.
        words: usize,
        /// O título cortado no tamanho certo, sem terminar em artigo ou
        /// preposição.
        suggestion: NotebookTitle,
    },
}
