//! O erro do `vamo-agenda`.

use std::path::PathBuf;

/// Tudo que pode dar errado ao ler a agenda.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// O texto não é um ICS que dê pra ler.
    #[error("o ICS tem erro na linha {line}: {problem}")]
    Ics {
        /// A linha do arquivo.
        line: usize,
        /// O que está errado.
        problem: String,
    },

    /// A regra de recorrência tem uma parte que o leitor não conhece.
    #[error("a regra de recorrência `{rule}` tem `{part}`, que o vamo ainda não entende")]
    UnsupportedRule {
        /// A regra inteira.
        rule: String,
        /// A parte que não deu.
        part: String,
    },

    /// A agenda não baixou e não tem cópia local.
    #[error("{problem}, e ainda não tem cópia local da agenda")]
    Download {
        /// O motivo, sem a URL (que é secreta).
        problem: String,
    },

    /// Não deu pra ler ou gravar a cópia local.
    #[error("não deu pra usar a cópia da agenda em {}", .path.display())]
    Cache {
        /// O arquivo da cópia.
        path: PathBuf,
        /// O erro do sistema.
        #[source]
        source: std::io::Error,
    },

    /// O Google Agenda recusou, falhou ou respondeu o que não se esperava.
    #[error("Google Agenda: {problem}")]
    Google {
        /// O motivo, sem token nem URL.
        problem: String,
    },

    /// O login no Google Agenda ainda não foi feito.
    #[error("o vamo ainda não está conectado ao Google Agenda: rode vamo conectar-google")]
    NotLoggedIn,

    /// Não deu pra ler ou gravar o token do Google.
    #[error("não deu pra usar o token do Google em {}", .path.display())]
    GoogleToken {
        /// O arquivo do token.
        path: PathBuf,
        /// O erro do sistema.
        #[source]
        source: std::io::Error,
    },
}
