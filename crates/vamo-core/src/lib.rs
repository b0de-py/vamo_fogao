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
//! # As tarefas
//!
//! - [`Task`]: a tarefa, que junta os tipos abaixo.
//! - [`TaskId`]: o número no formato `T-041`.
//! - [`NotebookTitle`]: o título no padrão do caderno, com verbo na frente e
//!   até cinco palavras.
//! - [`Area`]: o pedaço da vida a que a tarefa pertence.
//! - [`Fit`]: o encaixe, o tipo de espaço que a tarefa pede.
//! - [`Status`]: em que pé a tarefa está, com as transições permitidas.
//!
//! # O log de eventos
//!
//! O estado das tarefas não é guardado direto: toda mudança vira um
//! [`Event`], e a [`Projection`] calcula o estado a partir deles.
//!
//! - [`Event`]: uma mudança, com quando e onde aconteceu.
//! - [`Change`]: o que mudou.
//! - [`EventId`]: o ID do evento, um UUID v7.
//! - [`Machine`]: a máquina de onde veio o evento.
//! - [`Projection`]: o estado das tarefas, com conflito resolvido campo a
//!   campo.
//!
//! # O dia
//!
//! - [`Lineup`]: a escalação de um dia, com o atraso e as fixas.
//! - [`Score`]: o placar do dia.
//!
//! Todos esses tipos validam na construção, então um valor que existe é um
//! valor válido. Quando algo dá errado, o erro é um [`Error`].
//!
//! # As portas
//!
//! O que o core precisa do mundo de fora vira trait, e um adaptador
//! implementa.
//!
//! - [`EventStore`]: onde o log de eventos fica guardado.

mod area;
mod error;
mod event;
mod fit;
mod id;
mod lineup;
mod machine;
mod port;
mod projection;
mod status;
mod task;
mod title;

pub use area::Area;
pub use error::Error;
pub use event::{Change, Event, EventId};
pub use fit::Fit;
pub use id::TaskId;
pub use lineup::{Lineup, Score};
pub use machine::Machine;
pub use port::EventStore;
pub use projection::Projection;
pub use status::Status;
pub use task::Task;
pub use title::NotebookTitle;
