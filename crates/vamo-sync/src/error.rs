//! O erro do `vamo-sync`.

/// Tudo que pode dar errado na exportação e na sincronia.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A linha é de uma versão do formato mais nova que esta.
    #[error(
        "o export é da versão {found} do formato, e esta versão do vamo lê até a {known}: \
         atualize o vamo antes de sincronizar"
    )]
    NewerFormat {
        /// A versão da linha.
        found: u32,
        /// A versão mais nova que esta versão do vamo lê.
        known: u32,
    },

    /// Um campo tem um valor que o domínio não aceita.
    #[error("um campo do export tem um valor inválido")]
    Domain(#[from] vamo_core::Error),

    /// Um instante ou dia do export não está no formato.
    #[error("um instante ou dia do export não está no formato")]
    Time(#[source] jiff::Error),

    /// Uma linha não é o JSON do formato.
    #[error("a linha {line} de {file} não é uma linha do formato")]
    Json {
        /// O arquivo.
        file: String,
        /// O número da linha, contando de 1.
        line: usize,
        /// O erro do leitor de JSON.
        #[source]
        source: serde_json::Error,
    },
}
