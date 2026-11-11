//! Os textos da TUI, na linguagem de jogo ou na neutra.

use crate::app::Tab;

/// A linguagem da interface.
///
/// A de jogo é a padrão: banco, escalação, gol, súmula. A neutra troca o
/// vocabulário do futebol por palavras comuns, pra quem não quer o tema.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Voice {
    /// A linguagem de jogo.
    #[default]
    Game,
    /// O modo neutro.
    Neutral,
}

impl Voice {
    /// O nome de uma aba.
    pub(crate) fn tab(self, tab: Tab) -> &'static str {
        match (self, tab) {
            (_, Tab::Today) => "Hoje",
            (_, Tab::Notebook) => "Caderno",
            (Self::Game, Tab::Bench) => "Banco",
            (Self::Neutral, Tab::Bench) => "Tarefas",
            (_, Tab::Museum) => "Museu",
        }
    }

    /// Os atalhos de sempre, no rodapé.
    pub(crate) fn hints(self) -> &'static str {
        match self {
            Self::Game => "tab troca de tela · ? ajuda · q encerra a partida",
            Self::Neutral => "tab troca de tela · ? ajuda · q sai",
        }
    }

    /// O título da ajuda.
    pub(crate) fn help_title(self) -> &'static str {
        match self {
            Self::Game => "Prancheta",
            Self::Neutral => "Ajuda",
        }
    }

    /// As linhas da ajuda: a tecla e o que ela faz.
    pub(crate) fn help(self) -> Vec<(&'static str, &'static str)> {
        let quit = match self {
            Self::Game => "encerra a partida",
            Self::Neutral => "sai",
        };
        vec![
            ("1 a 4", "vai direto pra tela"),
            ("tab, shift+tab", "próxima tela, tela anterior"),
            ("?", "abre e fecha esta ajuda"),
            ("q, ctrl+c", quit),
        ]
    }
}
