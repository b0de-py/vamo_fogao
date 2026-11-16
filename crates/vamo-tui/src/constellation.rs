//! A constelação da invencibilidade: uma estrelinha por dia invicto, em
//! volta da Estrela Solitária do canto.
//!
//! As estrelinhas são caracteres comuns, desenhados antes do fogo. Elas só
//! entram em célula vazia: nunca cobrem texto nem caixa. O lugar de cada
//! uma é fixo (a primeira é sempre a mesma), pra o céu não mudar de um
//! quadro pro outro e cada dia invicto acender uma estrela nova no mesmo
//! canto.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::star::Star;
use crate::theme;

/// Onde fica cada estrelinha, em raios da estrela grande a partir do centro
/// dela: `(pra direita, pra baixo)`. As primeiras ficam mais perto. O
/// tamanho da lista é o limite da constelação.
const PLACES: [(f64, f64); 12] = [
    (-1.5, -0.9),
    (-0.6, -1.5),
    (0.7, -1.4),
    (-2.2, -0.2),
    (-1.9, -1.6),
    (0.1, -2.0),
    (-2.9, -1.0),
    (-1.2, -2.3),
    (-2.6, 0.6),
    (0.9, -2.3),
    (-3.4, -0.1),
    (-2.3, -2.4),
];

/// Quantas estrelinhas a constelação tem no máximo.
pub(crate) const LIMIT: usize = PLACES.len();

/// Os desenhos das estrelinhas, revezando pra o céu não ficar em fila.
const SHAPES: [&str; 3] = ["✦", "·", "✧"];

/// A célula do terminal onde cai a estrelinha de número `index`, se cair
/// dentro de `area`.
fn place(star: &Star, index: usize, area: Rect) -> Option<(u16, u16)> {
    let (right, down) = PLACES.get(index)?;
    let (center_x, center_y) = star.center();
    let x = center_x + right * star.radius();
    // A estrela é medida em células do fogo, duas por linha do terminal.
    let y = f64::midpoint(center_y, down * star.radius());
    if x < 0.0 || y < 0.0 {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "posição de tela, já conferida como não negativa e limitada à área logo abaixo"
    )]
    let (x, y) = (
        x.min(f64::from(u16::MAX)) as u16,
        y.min(f64::from(u16::MAX)) as u16,
    );
    let (x, y) = (area.x.saturating_add(x), area.y.saturating_add(y));
    (x < area.right() && y < area.bottom()).then_some((x, y))
}

/// Acende uma estrelinha por dia invicto, até o [`LIMIT`], nas células
/// vazias em volta da estrela.
pub(crate) fn render(buf: &mut Buffer, area: Rect, star: &Star, days: u32) {
    let area = area.intersection(buf.area);
    let days = usize::try_from(days).unwrap_or(usize::MAX).min(LIMIT);
    for index in 0..days {
        let Some(position) = place(star, index, area) else {
            continue;
        };
        let cell = &mut buf[position];
        // Só em célula vazia do fundo: texto e caixa ficam como estão.
        if cell.symbol() != " "
            || (cell.bg != theme::BACKGROUND && cell.bg != ratatui::style::Color::Reset)
        {
            continue;
        }
        cell.set_symbol(SHAPES[index % SHAPES.len()])
            .set_style(theme::soft());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sky(days: u32) -> (Buffer, usize) {
        let area = Rect::new(0, 0, 120, 40);
        let mut buf = Buffer::empty(area);
        let star = Star::in_corner(area.width, area.height).expect("a estrela cabe em 120 por 40");
        render(&mut buf, area, &star, days);
        let lit = buf
            .content()
            .iter()
            .filter(|cell| SHAPES.contains(&cell.symbol()))
            .count();
        (buf, lit)
    }

    #[test]
    fn one_small_star_per_unbeaten_day() {
        assert_eq!(sky(0).1, 0);
        assert_eq!(sky(1).1, 1);
        assert_eq!(sky(5).1, 5);
    }

    #[test]
    fn the_constellation_has_a_limit() {
        assert_eq!(sky(u32::MAX).1, LIMIT);
    }

    #[test]
    fn a_new_day_keeps_the_stars_that_were_there() {
        let (four, _) = sky(4);
        let (five, _) = sky(5);
        for (before, after) in four.content().iter().zip(five.content()) {
            if SHAPES.contains(&before.symbol()) {
                assert_eq!(before.symbol(), after.symbol());
            }
        }
    }

    #[test]
    fn the_stars_never_cover_what_is_written() {
        let area = Rect::new(0, 0, 120, 40);
        let mut buf = Buffer::empty(area);
        // Uma tela cheia de texto: não sobra célula vazia.
        for y in 0..area.height {
            buf.set_string(0, y, "x".repeat(120), theme::text());
        }
        let before = buf.clone();
        let star = Star::in_corner(area.width, area.height).expect("a estrela cabe em 120 por 40");
        render(&mut buf, area, &star, 12);
        assert_eq!(buf, before);
    }
}
