//! A barra de progresso em blocos: `▰▰▰▱▱▱▱▱`.
//!
//! Serve pra ver de relance o quanto falta: o placar do dia e a
//! invencibilidade contra o recorde.

use ratatui::style::Style;
use ratatui::text::Span;

use crate::theme;

/// O bloco cheio e o vazio.
const FULL: &str = "▰";
const EMPTY: &str = "▱";

/// Quantos blocos de uma barra de `width` ficam cheios com `done` de
/// `total`.
///
/// A conta arredonda pro mais perto, com duas exceções que valem mais que a
/// proporção exata: a barra só enche inteira quando `done` chega em `total`,
/// e só fica vazia quando `done` é zero. Assim ninguém vê "tudo feito"
/// faltando uma, nem "nada feito" já tendo começado. Com `total` zero, a
/// barra fica vazia.
pub(crate) fn filled(done: usize, total: usize, width: usize) -> usize {
    if total == 0 || done == 0 || width == 0 {
        return 0;
    }
    if done >= total {
        return width;
    }
    let rounded = (done * width + total / 2) / total;
    rounded.clamp(1, width.saturating_sub(1).max(1))
}

/// A barra, em dois pedaços: os blocos cheios no estilo dado e os vazios
/// apagados. Sempre com `width` colunas.
pub(crate) fn spans(done: usize, total: usize, width: usize, style: Style) -> [Span<'static>; 2] {
    let filled = filled(done, total, width);
    [
        Span::styled(FULL.repeat(filled), style),
        Span::styled(EMPTY.repeat(width - filled), theme::muted()),
    ]
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn half_done_is_half_full() {
        assert_eq!(filled(1, 2, 10), 5);
        assert_eq!(filled(3, 12, 12), 3);
        assert_eq!(filled(0, 0, 10), 0);
    }

    #[test]
    fn almost_done_is_not_full_and_just_started_is_not_empty() {
        assert_eq!(filled(99, 100, 10), 9);
        assert_eq!(filled(1, 100, 10), 1);
        assert_eq!(filled(100, 100, 10), 10);
        // Passar do total (o recorde quebrado) enche e para.
        assert_eq!(filled(13, 12, 10), 10);
    }

    #[test]
    fn the_bar_is_drawn_in_blocks() {
        let [full, empty] = spans(1, 4, 8, theme::selected());
        assert_eq!(full.content, "▰▰");
        assert_eq!(empty.content, "▱▱▱▱▱▱");
    }

    proptest! {
        /// A barra nunca passa da largura, e a proporção nunca anda pra trás.
        #[test]
        fn the_bar_fits_and_never_goes_backwards(
            done in 0usize..500,
            total in 0usize..500,
            width in 0usize..80,
        ) {
            let now = filled(done, total, width);
            prop_assert!(now <= width);
            prop_assert!(filled(done + 1, total, width) >= now);
            let [full, empty] = spans(done, total, width, theme::selected());
            prop_assert_eq!(full.width() + empty.width(), width);
        }

        /// Cheia só com tudo feito; vazia só com nada feito.
        #[test]
        fn full_means_done_and_empty_means_nothing(
            done in 0usize..500,
            total in 1usize..500,
            width in 2usize..80,
        ) {
            prop_assert_eq!(filled(done, total, width) == width, done >= total);
            prop_assert_eq!(filled(done, total, width) == 0, done == 0);
        }
    }
}
