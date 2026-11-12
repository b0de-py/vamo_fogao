//! O erro do `vamo-store`.

use std::path::PathBuf;

/// Tudo que pode dar errado com o banco.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Não deu pra criar a pasta onde o banco fica.
    #[error("não deu pra criar a pasta do banco em {}", .path.display())]
    CreateDir {
        /// A pasta.
        path: PathBuf,
        /// O erro do sistema.
        #[source]
        source: std::io::Error,
    },

    /// O SQLite deu erro.
    #[error("o SQLite deu erro")]
    Sqlite(#[from] rusqlite::Error),

    /// O banco foi criado ou migrado por uma versão mais nova do vamo.
    #[error(
        "o banco é de uma versão mais nova do vamo (esquema {found}, e esta versão conhece até \
         o {known}): atualize o vamo antes de abrir esse banco"
    )]
    NewerSchema {
        /// A versão do esquema no banco.
        found: u32,
        /// A versão mais nova que esta versão do vamo conhece.
        known: u32,
    },

    /// Uma linha do log não tem uma coluna que o tipo dela pede.
    #[error("a linha {seq} do log de eventos está sem a coluna `{column}`")]
    MissingColumn {
        /// A linha (`seq`) no log.
        seq: i64,
        /// A coluna.
        column: &'static str,
    },

    /// Uma coluna do log tem um valor que o domínio não aceita.
    #[error("a coluna `{column}` da linha {seq} do log de eventos tem um valor inválido")]
    InvalidValue {
        /// A linha (`seq`) no log.
        seq: i64,
        /// A coluna.
        column: &'static str,
        /// O que o domínio disse do valor.
        #[source]
        source: vamo_core::Error,
    },

    /// O instante de uma linha do log não é um instante válido.
    #[error("a coluna `at` da linha {seq} do log de eventos não é um instante válido")]
    InvalidInstant {
        /// A linha (`seq`) no log.
        seq: i64,
        /// O erro do jiff.
        #[source]
        source: jiff::Error,
    },

    /// O dia de uma linha do log não é um dia válido.
    #[error("a coluna `day` da linha {seq} do log de eventos não é um dia válido")]
    InvalidDay {
        /// A linha (`seq`) no log.
        seq: i64,
        /// O erro do jiff.
        #[source]
        source: jiff::Error,
    },

    /// O dia de uma marcação da agenda não é um dia válido.
    #[error("a marcação da agenda do dia `{day}` não tem um dia válido")]
    InvalidCheckDay {
        /// O texto guardado.
        day: String,
        /// O erro do jiff.
        #[source]
        source: jiff::Error,
    },

    /// Uma linha do log é de um tipo de mudança que esta versão não conhece.
    #[error(
        "a linha {seq} do log de eventos é do tipo `{kind}`, que esta versão do vamo não conhece"
    )]
    UnknownKind {
        /// A linha (`seq`) no log.
        seq: i64,
        /// O tipo que veio.
        kind: String,
    },
}
