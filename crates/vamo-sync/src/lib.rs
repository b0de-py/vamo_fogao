//! A sincronia do `vamo_fogao`.
//!
//! Exporta o log de eventos, importa os eventos das outras máquinas e junta
//! tudo pelo git, num repositório privado de backup. É também por aqui que
//! entram os reforços: tarefas que chegam de fora pela branch `vamo-entrada`.
//!
//! O que sai do app tem um formato só, versionado, no módulo [`format`](mod@format): o
//! log de eventos e as marcações em JSONL, e a súmula e a rodada em JSON.
//!
//! O [`Backup`] é o clone do repositório privado de backup. Cada máquina
//! escreve só os arquivos dela, `eventos/<máquina>.jsonl` e
//! `marcacoes/<máquina>.jsonl`, e lê os das outras. A regra de quem troca de
//! número quando duas máquinas criam o mesmo `T-###` é do core
//! ([`vamo_core::renumber_collisions`]); aqui só se lê e escreve.

mod backup;
mod error;
pub mod format;
mod inbox;

pub use backup::{Backup, Pushed, Snapshot};
pub use error::Error;
pub use inbox::{Cleared, INBOX_BRANCH, INBOX_FILE, Inbox, Rejected};
