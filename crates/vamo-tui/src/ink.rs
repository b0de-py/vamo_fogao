//! A cara da TUI fora dela: as mesmas cores e rótulos, em texto.
//!
//! A CLI, a dica da barra e a notificação não desenham com o ratatui, mas
//! falam com a mesma pessoa. Este módulo dá a elas a paleta do [`theme`](crate)
//! em códigos ANSI e os enfeites que viram texto puro (o rótulo em caixa-alta
//! espaçada e a barra `▰▱`), pra a cara nova não ficar presa na TUI.
//!
//! Quem decide se tem cor é quem chama: o binário olha se a saída é um
//! terminal e se o `NO_COLOR` está ligado. Sem cor, o [`Ink`] devolve o
//! texto como veio, então a saída num pipe ou num script não muda.

use ratatui::style::Color;

use crate::{meter, theme};

/// O pincel da saída de texto: pinta com as cores do tema, ou não pinta.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Ink {
    colored: bool,
}

/// O jeito da letra, além da cor.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Weight {
    Regular,
    Bold,
}

impl Ink {
    /// O pincel sem tinta: tudo sai como veio. É o padrão.
    pub const PLAIN: Self = Self { colored: false };

    /// O pincel com as cores do tema.
    pub const COLORED: Self = Self { colored: true };

    /// Se este pincel pinta.
    pub fn is_colored(self) -> bool {
        self.colored
    }

    /// O destaque: o ID de uma tarefa, o passo da vez. Ciano em negrito.
    pub fn accent(self, text: &str) -> String {
        self.paint(text, theme::ACCENT, Weight::Bold)
    }

    /// O que fica no fundo: rótulos, datas, a máquina do evento.
    pub fn muted(self, text: &str) -> String {
        self.paint(text, theme::MUTED, Weight::Regular)
    }

    /// O texto de apoio, um tom abaixo do principal.
    pub fn soft(self, text: &str) -> String {
        self.paint(text, theme::SOFT, Weight::Regular)
    }

    /// O título de uma ficha: branco em negrito.
    pub fn heading(self, text: &str) -> String {
        self.paint(text, theme::WHITE, Weight::Bold)
    }

    /// Atraso e regra ferida: amarelo plasma, nunca vermelho.
    pub fn warning(self, text: &str) -> String {
        self.paint(text, theme::PLASMA, Weight::Bold)
    }

    /// O rótulo de uma seção. Com cor, vai em caixa-alta espaçada e ciano,
    /// como na TUI (`S Ú M U L A`); sem cor, fica como veio, pra a saída de
    /// um script não mudar.
    ///
    /// ```
    /// use vamo_tui::Ink;
    ///
    /// assert_eq!(Ink::PLAIN.section("Súmula"), "Súmula");
    /// assert!(Ink::COLORED.section("Súmula").contains("S Ú M U L A"));
    /// ```
    pub fn section(self, text: &str) -> String {
        if self.colored {
            self.paint(&theme::spaced_caps(text), theme::ACCENT, Weight::Bold)
        } else {
            text.to_owned()
        }
    }

    /// O grito de gol: preto no branco, alvinegro, com uma folga de cada
    /// lado. Sem cor, só o texto.
    pub fn shout(self, text: &str) -> String {
        if !self.colored {
            return text.to_owned();
        }
        let (bg, fg) = (rgb(theme::WHITE), rgb(theme::BACKGROUND));
        format!("\x1b[1;38;2;{fg};48;2;{bg}m {text} {RESET}")
    }

    fn paint(self, text: &str, color: Color, weight: Weight) -> String {
        if !self.colored || text.is_empty() {
            return text.to_owned();
        }
        let bold = if weight == Weight::Bold { "1;" } else { "" };
        format!("\x1b[{bold}38;2;{}m{text}{RESET}", rgb(color))
    }
}

/// O código que volta a letra ao normal.
const RESET: &str = "\x1b[0m";

/// Os três números de uma cor do tema, no formato do ANSI (`r;g;b`). As
/// cores do tema são todas RGB; uma que não fosse sairia branca.
fn rgb(color: Color) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("{r};{g};{b}"),
        _ => "255;255;255".to_owned(),
    }
}

/// Um rótulo em caixa-alta espaçada, como na TUI: `Em campo` vira
/// `E M   C A M P O`.
///
/// ```
/// assert_eq!(vamo_tui::spaced_caps("Em campo"), "E M   C A M P O");
/// ```
pub fn spaced_caps(text: &str) -> String {
    theme::spaced_caps(text)
}

/// A barra de progresso em texto, com `width` blocos: `▰▰▱▱`. Enche
/// inteira só quando `done` chega em `total`, como a da TUI.
///
/// ```
/// assert_eq!(vamo_tui::meter_text(1, 2, 4), "▰▰▱▱");
/// assert_eq!(vamo_tui::meter_text(0, 0, 3), "▱▱▱");
/// ```
pub fn meter_text(done: usize, total: usize, width: usize) -> String {
    let filled = meter::filled(done, total, width);
    format!("{}{}", "▰".repeat(filled), "▱".repeat(width - filled))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plain_ink_leaves_the_text_alone() {
        let ink = Ink::PLAIN;
        for text in [
            ink.accent("T-001"),
            ink.muted("T-001"),
            ink.soft("T-001"),
            ink.heading("T-001"),
            ink.warning("T-001"),
            ink.section("T-001"),
            ink.shout("T-001"),
        ] {
            assert_eq!(text, "T-001");
        }
    }

    #[test]
    fn the_colored_ink_uses_the_theme_and_resets() {
        let ink = Ink::COLORED;
        assert_eq!(ink.accent("T-001"), "\x1b[1;38;2;0;217;255mT-001\x1b[0m");
        assert_eq!(ink.muted("x"), "\x1b[38;2;102;102;102mx\x1b[0m");
        assert_eq!(ink.warning("x"), "\x1b[1;38;2;255;255;0mx\x1b[0m");
        assert_eq!(
            ink.shout("GOL"),
            "\x1b[1;38;2;10;10;10;48;2;255;255;255m GOL \x1b[0m"
        );
    }

    #[test]
    fn an_empty_text_gets_no_paint() {
        assert_eq!(Ink::COLORED.accent(""), "");
    }

    #[test]
    fn no_red_reaches_the_terminal() {
        // O vermelho do ANSI (31, 91) e o fundo vermelho (41, 101) não
        // aparecem em nenhum pincel.
        let ink = Ink::COLORED;
        let all = [
            ink.accent("x"),
            ink.muted("x"),
            ink.soft("x"),
            ink.heading("x"),
            ink.warning("x"),
            ink.section("x"),
            ink.shout("x"),
        ]
        .concat();
        for code in ["[31m", ";31m", "[91m", "[41m", "[101m"] {
            assert!(!all.contains(code), "{code} em {all:?}");
        }
    }
}
