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

#[allow(
    clippy::unused_self,
    reason = "todo texto passa pela Voice, até o que ainda é igual nas duas linguagens"
)]
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

    /// O título do quadro da curiosidade quando o fato é o da vez.
    pub(crate) fn curiosity_title(self) -> &'static str {
        match self {
            Self::Game => "Do baú do Glorioso",
            Self::Neutral => "Curiosidade do dia",
        }
    }

    /// O título do quadro quando o fato aconteceu neste dia.
    pub(crate) fn on_this_day(self, year: u16) -> String {
        format!("Neste dia, em {year}")
    }

    /// O começo da linha das fontes.
    pub(crate) fn sources(self) -> &'static str {
        "Fontes: "
    }

    /// O selo da resenha, que avisa que aquilo não é fato.
    pub(crate) fn banter_seal(self) -> &'static str {
        match self {
            Self::Game => "Resenha (é zoeira, não é fato)",
            Self::Neutral => "Piada (não é fato)",
        }
    }

    /// A dica da tecla que mostra a resenha.
    pub(crate) fn banter_hint(self) -> &'static str {
        match self {
            Self::Game => "r: a resenha do dia",
            Self::Neutral => "r: a piada do dia",
        }
    }

    /// A dica da tecla que volta pro fato.
    pub(crate) fn back_to_fact_hint(self) -> &'static str {
        "r: volta pro fato"
    }

    /// Quando o acervo não tem fato verificado.
    pub(crate) fn no_curiosity(self) -> &'static str {
        match self {
            Self::Game => "O baú está vazio: nenhum fato verificado ainda.",
            Self::Neutral => "Nenhum fato verificado ainda.",
        }
    }

    /// Quando a resenha está vazia.
    pub(crate) fn no_banter(self) -> &'static str {
        match self {
            Self::Game => "A resenha está vazia.",
            Self::Neutral => "Nenhuma piada no acervo.",
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
            ("r", "troca a curiosidade pela resenha, e volta"),
            ("?", "abre e fecha esta ajuda"),
            ("q, ctrl+c", quit),
        ]
    }
}
