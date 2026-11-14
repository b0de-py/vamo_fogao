//! A sincronia do `vamo_fogao`.
//!
//! Exporta o log de eventos, importa os eventos das outras máquinas e junta
//! tudo pelo git, num repositório privado de backup. É também por aqui que
//! entram os reforços: tarefas que chegam de fora pela branch `vamo-entrada`.
//!
//! O que sai do app tem um formato só, versionado, no módulo [`format`](mod@format): o
//! log de eventos e as marcações em JSONL, e a súmula e a rodada em JSON.

mod error;
pub mod format;

pub use error::Error;
