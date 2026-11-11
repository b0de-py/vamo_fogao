//! Os textos da interface, nas duas linguagens: a de jogo e a neutra.
//!
//! Todo texto que o `vamo` mostra sai daqui, então trocar de linguagem é
//! trocar a [`Voice`], sem caçar string pelo código.

use clap::ValueEnum;
use vamo_core::{Change, Task};

/// A linguagem das mensagens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Voice {
    /// A linguagem de jogo, que é a padrão: banco, gol, súmula.
    #[value(name = "jogo")]
    Game,
    /// O modo neutro: tarefa, concluída, histórico.
    #[value(name = "neutra")]
    Neutral,
}

impl Voice {
    /// Depois de `vamo add`.
    pub(crate) fn added(self, task: &Task) -> String {
        match self {
            Self::Game => format!("{} entrou no banco: {}", task.id(), task.title()),
            Self::Neutral => format!("Tarefa criada: {} {}", task.id(), task.title()),
        }
    }

    /// Depois de `vamo done`.
    pub(crate) fn done(self, task: &Task) -> String {
        match self {
            Self::Game => format!("GOOOL! {} {}", task.id(), task.title()),
            Self::Neutral => format!("Concluída: {} {}", task.id(), task.title()),
        }
    }

    /// Quando a lista não tem nenhuma tarefa.
    pub(crate) fn nothing_to_list(self) -> &'static str {
        match self {
            Self::Game => "Ninguém no banco. Pra contratar: vamo add <título>",
            Self::Neutral => "Nenhuma tarefa. Pra criar uma: vamo add <título>",
        }
    }

    /// O título do histórico de uma tarefa, no `vamo show`.
    pub(crate) fn history(self) -> &'static str {
        match self {
            Self::Game => "Súmula",
            Self::Neutral => "Histórico",
        }
    }

    /// O que aconteceu num evento, numa linha do histórico.
    pub(crate) fn change(self, change: &Change) -> String {
        match change {
            Change::Created {
                title,
                area,
                fit,
                status,
            } => {
                let what = match self {
                    Self::Game => "entrou no banco",
                    Self::Neutral => "criada",
                };
                format!("{what}: {title} ({area}, {fit}, {status})")
            }
            Change::Title(title) => format!("título: {title}"),
            Change::Area(area) => format!("área: {area}"),
            Change::Fit(fit) => format!("encaixe: {fit}"),
            Change::Status(status) => format!("status: {status}"),
            Change::Notes(notes) if notes.is_empty() => "notas apagadas".to_owned(),
            Change::Notes(notes) => format!("notas: {notes}"),
        }
    }

    /// O que vem antes da mensagem de um erro.
    pub(crate) fn error(self) -> &'static str {
        match self {
            Self::Game => "Deu ruim:",
            Self::Neutral => "Erro:",
        }
    }
}
