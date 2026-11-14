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

    /// Um arquivo do backup não pôde ser lido ou escrito.
    #[error("não deu pra ler ou escrever {}", .path.display())]
    Io {
        /// O arquivo ou a pasta.
        path: std::path::PathBuf,
        /// O erro do sistema.
        #[source]
        source: std::io::Error,
    },

    /// O programa `git` não rodou.
    #[error("o git não rodou: ele está instalado?")]
    GitMissing {
        /// O erro do sistema.
        #[source]
        source: std::io::Error,
    },

    /// Um comando do git falhou.
    #[error("o `git {command}` falhou: {stderr}")]
    Git {
        /// O comando, sem o `git`.
        command: String,
        /// O que o git disse.
        stderr: String,
    },

    /// O clone do backup não tem o remoto `origin`.
    #[error(
        "o repositório de backup em {} não tem o remoto origin: clone o repositório privado \
         de backup ali",
        .dir.display()
    )]
    NoRemote {
        /// A pasta do clone.
        dir: std::path::PathBuf,
    },

    /// O remoto do backup é o repositório público do vamo_fogao.
    #[error(
        "o remoto do backup é {url}, o repositório público do vamo_fogao: o backup tem os seus \
         dados e precisa de um repositório privado"
    )]
    PublicRepository {
        /// O endereço do remoto.
        url: String,
    },

    /// O remoto recusou o envio porque outra máquina mandou antes.
    #[error("o remoto recusou o envio: outra máquina mandou antes")]
    Rejected,
}
