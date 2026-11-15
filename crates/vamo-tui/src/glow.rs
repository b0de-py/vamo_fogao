//! O brilho da linha escolhida: acende quando o cursor chega e assenta.
//!
//! As telas marcam a linha do cursor com o [`theme::cursor`] e não sabem de
//! animação. Depois que a tela é desenhada, o [`settle`] passa pelo miolo,
//! acha essas células e dá a elas a cor do momento: o ciano cheio na
//! chegada, que em poucos passos escurece até o fundo de repouso. É o mesmo
//! jeito do fogo, que reconhece as caixas pela cor de fundo.
//!
//! O brilho tem começo e fim: no último passo a linha fica no repouso e a
//! TUI pode voltar a dormir.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::theme;

/// Quantos passos o brilho leva pra assentar: com um passo a cada 40 ms,
/// perto de um terço de segundo.
pub(crate) const STEPS: u8 = 8;

/// A barra que marca o começo da linha escolhida.
const BAR: &str = "▌";

/// Uma cor entre `from` e `to`: `amount` vai de 0 (só `from`) a `of` (só
/// `to`). Cor que não é RGB fica como `to`.
fn blend(from: Color, to: Color, amount: u8, of: u8) -> Color {
    let (Color::Rgb(r0, g0, b0), Color::Rgb(r1, g1, b1)) = (from, to) else {
        return to;
    };
    let mix = |a: u8, b: u8| {
        let (a, b) = (u16::from(a), u16::from(b));
        let (amount, of) = (u16::from(amount.min(of)), u16::from(of.max(1)));
        // A média pesada de dois u8 cabe num u8.
        u8::try_from((a * (of - amount) + b * amount) / of).unwrap_or(u8::MAX)
    };
    Color::Rgb(mix(r0, r1), mix(g0, g1), mix(b0, b1))
}

/// O estilo da linha escolhida com `glow` passos de brilho ainda por vir:
/// de [`STEPS`] (recém-chegada, ciano cheio) a 0 (assentada).
fn style(glow: u8) -> Style {
    let background = blend(theme::CURSOR_REST, theme::ACCENT, glow, STEPS);
    // Na metade clara do caminho a letra é escura, como no ciano cheio; na
    // escura, branca.
    let text = if glow > STEPS / 2 {
        theme::BACKGROUND
    } else {
        theme::WHITE
    };
    Style::new()
        .fg(text)
        .bg(background)
        .add_modifier(Modifier::BOLD)
}

/// Se a célula faz parte da linha escolhida, como as telas marcam.
fn is_cursor(buf: &Buffer, x: u16, y: u16) -> bool {
    // No miolo, só a linha escolhida tem o fundo de destaque. A cor da
    // letra não entra na conta: uma célula da linha pode ter a própria (o
    // ID apagado na tabela do Banco) e continua sendo da linha.
    buf[(x, y)].bg == theme::ACCENT
}

/// Dá à linha escolhida, dentro de `area`, a cor do momento e a barra do
/// começo. `glow` é quantos passos de brilho faltam.
pub(crate) fn settle(area: Rect, buf: &mut Buffer, glow: u8) {
    let area = area.intersection(buf.area);
    let style = style(glow);
    for y in area.top()..area.bottom() {
        let mut first = None;
        for x in area.left()..area.right() {
            if is_cursor(buf, x, y) {
                first.get_or_insert(x);
                buf[(x, y)].set_style(style);
            }
        }
        let Some(first) = first else { continue };
        // A barra entra na primeira célula da linha quando ela está vazia.
        // Quando a linha já começa com texto (a tabela do Banco), entra na
        // coluna de margem logo antes.
        let bar = if buf[(first, y)].symbol() == " " {
            Some(first)
        } else {
            first
                .checked_sub(1)
                .filter(|&x| x >= buf.area.left() && buf[(x, y)].symbol() == " ")
        };
        if let Some(x) = bar {
            buf[(x, y)].set_symbol(BAR).set_fg(theme::ACCENT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Um miolo de 10 por 3 com a linha do meio escolhida, das colunas 2 a 7.
    fn screen(text: &str) -> Buffer {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 3));
        buf.set_string(2, 1, text, theme::cursor());
        buf
    }

    #[test]
    fn the_glow_starts_full_and_settles_dark() {
        let mut arriving = screen(" linha");
        settle(arriving.area, &mut arriving, STEPS);
        assert_eq!(arriving[(4, 1)].bg, theme::ACCENT);
        assert_eq!(arriving[(4, 1)].fg, theme::BACKGROUND);

        let mut settled = screen(" linha");
        settle(settled.area, &mut settled, 0);
        assert_eq!(settled[(4, 1)].bg, theme::CURSOR_REST);
        assert_eq!(settled[(4, 1)].fg, theme::WHITE);
    }

    #[test]
    fn each_step_is_darker_than_the_one_before() {
        let green = |glow| match style(glow).bg {
            Some(Color::Rgb(_, g, _)) => g,
            other => panic!("o fundo do cursor é sempre RGB, veio {other:?}"),
        };
        for glow in 1..=STEPS {
            assert!(green(glow) > green(glow - 1), "passo {glow}");
        }
    }

    #[test]
    fn the_bar_takes_the_empty_start_of_the_line() {
        let mut buf = screen(" linha");
        settle(buf.area, &mut buf, 0);
        assert_eq!(buf[(2, 1)].symbol(), BAR);
        assert_eq!(buf[(2, 1)].fg, theme::ACCENT);
        assert_eq!(buf[(3, 1)].symbol(), "l");
    }

    #[test]
    fn a_line_that_starts_with_text_gets_the_bar_in_the_margin() {
        let mut buf = screen("T-001");
        settle(buf.area, &mut buf, 0);
        assert_eq!(buf[(1, 1)].symbol(), BAR);
        assert_eq!(buf[(2, 1)].symbol(), "T");
        // A margem não vira linha escolhida: só ganha a barra.
        assert_eq!(buf[(1, 1)].bg, Color::Reset);
    }

    #[test]
    fn what_is_not_chosen_is_left_alone() {
        let mut buf = screen(" linha");
        let before = buf.clone();
        // Fora da área pedida, nada muda.
        settle(Rect::new(0, 0, 10, 1), &mut buf, 0);
        assert_eq!(buf, before);
        // Dentro, só a linha escolhida muda.
        settle(buf.area, &mut buf, 0);
        assert_eq!(buf[(0, 0)], before[(0, 0)]);
        assert_eq!(buf[(9, 1)], before[(9, 1)]);
    }
}
