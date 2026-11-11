//! Os textos da interface que mudam com a linguagem: a de jogo e a neutra.
//!
//! Todo texto que muda de uma linguagem pra outra sai daqui, então trocar de
//! linguagem é trocar a [`Voice`], sem caçar string pelo código.

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
            // No histórico, cada evento ocupa uma linha só.
            Change::Notes(notes) => {
                format!("notas: {}", notes.lines().collect::<Vec<_>>().join(" · "))
            }
        }
    }

    /// Depois de `vamo import`, com quantas tarefas entraram.
    pub(crate) fn imported(self, count: usize) -> String {
        match (self, count) {
            (Self::Game, 0) => "Nenhum reforço entrou no banco.".to_owned(),
            (Self::Game, 1) => "1 reforço entrou no banco.".to_owned(),
            (Self::Game, n) => format!("{n} reforços entraram no banco."),
            (Self::Neutral, 0) => "Nenhuma tarefa importada.".to_owned(),
            (Self::Neutral, 1) => "1 tarefa importada.".to_owned(),
            (Self::Neutral, n) => format!("{n} tarefas importadas."),
        }
    }

    /// Depois de `vamo acervo validar`, quando o acervo inteiro passa.
    pub(crate) fn acervo_checked(self, verified: usize, to_check: usize, banter: usize) -> String {
        let facts = match verified {
            1 => "1 fato verificado".to_owned(),
            n => format!("{n} fatos verificados"),
        };
        let summary = format!("{facts}, {to_check} a conferir e {banter} da resenha");
        match self {
            Self::Game => format!("O VAR conferiu o acervo: {summary}."),
            Self::Neutral => format!("Acervo validado: {summary}."),
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
