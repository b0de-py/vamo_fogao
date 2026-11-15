//! O rodapé: as teclas que valem na tela aberta, desenhadas como tecla, e o
//! placar do dia no canto.
//!
//! As teclas chegam na ordem de importância. Quando a tela é estreita, as
//! menos importantes somem primeiro, e as de sempre (a ajuda, a troca de
//! tela, a saída) ficam até o fim: sem elas, quem usa fica sem saída.

use ratatui::text::{Line, Span};

use crate::theme;

/// Uma tecla e o que ela faz, em poucas letras: `("e", "escala")`.
pub(crate) type Key = (&'static str, &'static str);

/// O espaço entre uma tecla e a seguinte.
const GAP: usize = 2;

/// A largura de uma tecla no rodapé: ` e ` mais o espaço e o nome.
fn key_width((key, what): Key) -> usize {
    Span::raw(key).width() + 2 + 1 + Span::raw(what).width()
}

/// A largura de uma fila de teclas, com os espaços entre elas.
fn row_width(keys: &[Key]) -> usize {
    let gaps = keys.len().saturating_sub(1) * GAP;
    keys.iter().copied().map(key_width).sum::<usize>() + gaps
}

/// A linha do rodapé pra uma tela de `width` colunas.
///
/// `screen` são as teclas da tela, da mais importante pra menos; `always`,
/// as de sempre. `right` é o texto do canto direito (o placar), que some
/// antes de qualquer tecla quando não cabe.
pub(crate) fn line(
    screen: &[Key],
    always: &[Key],
    right: Option<&str>,
    width: usize,
) -> Line<'static> {
    // Uma coluna de margem de cada lado.
    let room = width.saturating_sub(2);
    let right = right.filter(|text| {
        let all: Vec<Key> = screen.iter().chain(always).copied().collect();
        row_width(&all) + GAP + Span::raw(*text).width() <= room
    });

    let mut screen = screen.to_vec();
    let mut always = always.to_vec();
    let fits = |screen: &[Key], always: &[Key]| {
        let all: Vec<Key> = screen.iter().chain(always).copied().collect();
        row_width(&all) <= room
    };
    // Primeiro saem as da tela, da menos importante pra mais; só depois, as
    // de sempre.
    while !fits(&screen, &always) {
        if screen.pop().is_none() && always.pop().is_none() {
            break;
        }
    }

    let mut spans = vec![Span::raw(" ")];
    for (index, (key, what)) in screen.iter().chain(&always).enumerate() {
        if index > 0 {
            spans.push(Span::raw(" ".repeat(GAP)));
        }
        spans.push(Span::styled(format!(" {key} "), theme::keycap()));
        spans.push(Span::styled(format!(" {what}"), theme::soft()));
    }
    if let Some(text) = right {
        let used: usize = spans.iter().map(Span::width).sum();
        let pad = width.saturating_sub(used + Span::raw(text).width() + 1);
        spans.push(Span::raw(" ".repeat(pad)));
        spans.push(Span::styled(text.to_owned(), theme::soft()));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    const SCREEN: [Key; 3] = [("␣", "gol"), ("e", "escala"), ("a", "adia")];
    const ALWAYS: [Key; 2] = [("?", "ajuda"), ("q", "sai")];

    fn text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn with_room_everything_shows_and_the_score_goes_to_the_corner() {
        let line = line(&SCREEN, &ALWAYS, Some("Placar: 1 de 2"), 70);
        assert_eq!(
            text(&line),
            "  ␣  gol   e  escala   a  adia   ?  ajuda   q  sai     Placar: 1 de 2"
        );
        assert_eq!(line.width(), 69);
    }

    #[test]
    fn the_score_leaves_before_any_key() {
        let line = line(&SCREEN, &ALWAYS, Some("Placar: 1 de 2"), 55);
        assert_eq!(
            text(&line),
            "  ␣  gol   e  escala   a  adia   ?  ajuda   q  sai"
        );
    }

    #[test]
    fn the_least_important_keys_of_the_screen_leave_first() {
        let line = line(&SCREEN, &ALWAYS, None, 41);
        assert_eq!(text(&line), "  ␣  gol   e  escala   ?  ajuda   q  sai");
    }

    #[test]
    fn the_keys_of_always_stay_until_the_end() {
        let narrow = line(&SCREEN, &ALWAYS, None, 22);
        assert_eq!(text(&narrow), "  ?  ajuda   q  sai");
        let tiny = line(&SCREEN, &ALWAYS, None, 12);
        assert_eq!(text(&tiny), "  ?  ajuda");
        assert_eq!(text(&line(&SCREEN, &ALWAYS, None, 3)), " ");
    }

    proptest! {
        /// O rodapé nunca passa da largura da tela, em nenhuma largura.
        #[test]
        fn the_footer_never_overflows(width in 0usize..200, with_score: bool) {
            let score = with_score.then_some("Placar: 10 de 12");
            let line = line(&SCREEN, &ALWAYS, score, width);
            prop_assert!(line.width() <= width.max(1));
        }
    }
}
