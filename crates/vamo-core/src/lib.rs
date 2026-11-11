//! O domínio do `vamo_fogao`: tarefas, status, encaixe, regras do tempo,
//! brechas e o título no padrão do caderno.
//!
//! É o centro da arquitetura hexagonal. Este crate não faz IO e não lê o
//! relógio: tudo que vem de fora (banco de dados, agenda, terminal, a hora
//! atual) entra por parâmetro ou por uma trait que um adaptador implementa.
//! Por isso ele é o crate mais testado do projeto.
//!
//! Regra de dependência: o `vamo-core` não depende de nenhum outro crate do
//! workspace. São os outros que dependem dele.
//!
//! # Os tipos
//!
//! - [`Task`]: a tarefa, que junta os tipos abaixo.
//! - [`TaskId`]: o número no formato `T-041`.
//! - [`NotebookTitle`]: o título no padrão do caderno, com verbo na frente e
//!   até cinco palavras.
//! - [`Area`]: o pedaço da vida a que a tarefa pertence.
//! - [`Fit`]: o encaixe, o tipo de espaço que a tarefa pede.
//! - [`Status`]: em que pé a tarefa está, com as transições permitidas.
//!
//! Todos validam na construção, então um valor que existe é um valor válido.
//! Quando algo dá errado, o erro é um [`Error`].

mod area;
mod error;
mod fit;
mod id;
mod status;
mod task;
mod title;

pub use area::Area;
pub use error::Error;
pub use fit::Fit;
pub use id::TaskId;
pub use status::Status;
pub use task::Task;
pub use title::NotebookTitle;
