//! O glitch da troca de tela: por alguns quadros, linhas soltas da tela
//! escorregam pro lado, em ciano e magenta, como no r4ul.com.br.
//!
//! Funciona como o brilho da linha escolhida: a tela nova é desenhada
//! inteira, e o [`apply`] bagunça o resultado por cima. A bagunça sai de
//! uma conta com o número do passo, sem acaso de verdade, então o mesmo
//! passo dá sempre o mesmo quadro e dá pra testar. Dura [`STEPS`] passos e
//! acaba: no último, a tela fica limpa e a TUI pode voltar a dormir.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Span;

use crate::theme;

/// Quantos passos o glitch dura: com um passo a cada 40 ms, 160 ms.
pub(crate) const STEPS: u8 = 4;

/// O quanto uma linha escorrega, no máximo, em colunas.
const MAX_SHIFT: u16 = 3;

/// Um número embaralhado a partir da linha e do passo. É o "acaso" do
/// glitch: parece sorteio, mas a mesma entrada dá sempre a mesma saída.
fn scramble(row: u16, step: u8) -> u32 {
    let mut value = (u32::from(row) << 8 | u32::from(step)).wrapping_mul(0x9E37_79B1);
    value ^= value >> 15;
    value = value.wrapping_mul(0x85EB_CA6B);
    value ^ (value >> 13)
}

/// Bagunça `area` no passo `step` do glitch (de [`STEPS`] até 1). No passo
/// 0, não faz nada.
///
/// Mais ou menos uma linha em cada quatro escorrega até [`MAX_SHIFT`]
/// colunas: pra esquerda em ciano, pra direita em magenta. Linha com
/// caractere de duas colunas só muda de cor, pra o caractere não ser
/// cortado no meio.
pub(crate) fn apply(area: Rect, buf: &mut Buffer, step: u8) {
    if step == 0 {
        return;
    }
    let area = area.intersection(buf.area);
    for y in area.top()..area.bottom() {
        let roll = scramble(y, step);
        if !roll.is_multiple_of(4) {
            continue;
        }
        // O `% MAX_SHIFT` cabe num u16 com folga.
        let shift = 1 + u16::try_from((roll >> 8) % u32::from(MAX_SHIFT)).unwrap_or(0);
        let leftward = (roll >> 4).is_multiple_of(2);
        let color = if leftward {
            theme::ACCENT
        } else {
            theme::MAGENTA
        };
        let symbols: Vec<String> = (area.left()..area.right())
            .map(|x| buf[(x, y)].symbol().to_owned())
            .collect();
        let narrow = symbols.iter().all(|s| Span::raw(s.as_str()).width() <= 1);
        let width = symbols.len();
        for (index, x) in (area.left()..area.right()).enumerate() {
            let cell = &mut buf[(x, y)];
            if narrow {
                // A célula mostra o caractere de `shift` colunas adiante (ou
                // atrás); o que vem de fora da área é espaço.
                let shift = usize::from(shift);
                let from = if leftward {
                    index.checked_add(shift).filter(|&from| from < width)
                } else {
                    index.checked_sub(shift)
                };
                cell.set_symbol(from.map_or(" ", |from| symbols[from].as_str()));
            }
            if cell.symbol() != " " {
                cell.set_fg(color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Style;

    use super::*;

    /// Uma tela de 20 por 40, cada linha com um texto diferente.
    fn screen() -> Buffer {
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 40));
        for y in 0..40 {
            buf.set_string(
                2,
                y,
                format!("linha {y:02} do teste"),
                Style::new().fg(theme::TEXT),
            );
        }
        buf
    }

    fn row(buf: &Buffer, y: u16) -> String {
        (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect()
    }

    #[test]
    fn the_last_step_leaves_the_screen_clean() {
        let mut buf = screen();
        apply(buf.area, &mut buf, 0);
        assert_eq!(buf, screen());
    }

    #[test]
    fn some_rows_slide_and_the_others_stay() {
        let clean = screen();
        let mut buf = screen();
        apply(buf.area, &mut buf, STEPS);
        let moved = (0..40).filter(|&y| row(&buf, y) != row(&clean, y)).count();
        // Perto de uma em cada quatro: nem a tela toda, nem nenhuma.
        assert!((4..=20).contains(&moved), "{moved} linhas escorregaram");
    }

    #[test]
    fn a_slid_row_keeps_its_letters_and_takes_the_glitch_colors() {
        let clean = screen();
        let mut buf = screen();
        apply(buf.area, &mut buf, STEPS);
        for y in 0..40 {
            if row(&buf, y) == row(&clean, y) {
                continue;
            }
            // O miolo do texto continua lá, só mudou de lugar.
            assert!(row(&buf, y).contains("nha"), "{}", row(&buf, y));
            let colors: Vec<_> = (0..20)
                .filter(|&x| buf[(x, y)].symbol() != " ")
                .map(|x| buf[(x, y)].fg)
                .collect();
            assert!(
                colors.iter().all(|&c| c == theme::ACCENT)
                    || colors.iter().all(|&c| c == theme::MAGENTA),
                "{colors:?}"
            );
        }
    }

    #[test]
    fn the_same_step_gives_the_same_frame() {
        let (mut first, mut second) = (screen(), screen());
        apply(first.area, &mut first, 3);
        apply(second.area, &mut second, 3);
        assert_eq!(first, second);
        // E um passo diferente bagunça outras linhas.
        let mut other = screen();
        apply(other.area, &mut other, 2);
        assert_ne!(first, other);
    }

    #[test]
    fn a_row_with_a_wide_character_only_changes_color() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 40));
        for y in 0..40 {
            buf.set_string(2, y, "jogo ⚽ hoje", Style::new().fg(theme::TEXT));
        }
        let clean = buf.clone();
        apply(buf.area, &mut buf, STEPS);
        for y in 0..40 {
            assert_eq!(row(&buf, y), row(&clean, y));
        }
        assert_ne!(buf, clean);
    }
}
