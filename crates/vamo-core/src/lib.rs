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
//! # A agenda
//!
//! - [`Appointment`]: um compromisso da agenda, já no horário local.
//! - [`LinkPlan`]: o que o sync faz com as tarefas ligadas a eventos pelo
//!   rodapé `vamo:` (sai de [`Projection::link`]).
//! - [`gaps`]: as brechas do dia, e [`fitting`]: as tarefas que cabem
//!   em cada uma.
//! - [`fixed_activities`]: os compromissos recorrentes, que viram itens
//!   de marcar na tela Hoje.
//! - [`reminders`]: os lembretes que vencem numa [`ReminderWindow`], a
//!   janela de uma rodada do timer.
//! - [`TimeRules`]: as regras do tempo (expediente, teto da noite e blocos
//!   protegidos) e as [`Violation`]s de um horário marcado.
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
//! - [`Agenda`]: de onde vêm os compromissos da agenda, os
//!   [`Appointment`]s.
//! - [`Notifier`]: pra onde vão os avisos, as notificações do desktop.

mod appointment;
mod area;
mod error;
mod event;
mod fit;
mod fixed;
mod gaps;
mod id;
mod lineup;
mod link;
mod machine;
mod port;
mod projection;
mod reminder;
mod rules;
mod status;
mod task;
mod title;

pub use appointment::{Agenda, Appointment, When};
pub use area::Area;
pub use error::Error;
pub use event::{Change, Event, EventId};
pub use fit::Fit;
pub use fixed::{FixedActivity, fixed_activities};
pub use gaps::{DEFAULT_MIN_GAP, Gap, fitting, gaps};
pub use id::TaskId;
pub use lineup::{Lineup, Score};
pub use link::{LinkPlan, Skipped};
pub use machine::Machine;
pub use port::{EventStore, Notifier};
pub use projection::Projection;
pub use reminder::{DEFAULT_LEAD, MAX_CATCH_UP, Reminder, ReminderWindow, reminders};
pub use rules::{Block, TimeRange, TimeRules, Violation, time_of_day};
pub use status::Status;
pub use task::Task;
pub use title::NotebookTitle;
