//! A caixa da TUI: um vidro fumê com colchetes nos cantos, como os cards do
//! r4ul.com.br.
//!
//! Toda caixa (a curiosidade, a ajuda, a Ficha, o detalhe do Museu) é um
//! [`Panel`], então mudar a cara das caixas é mexer só aqui. Em vez da
//! moldura fechada, só os quatro cantos são desenhados, cada um com um braço
//! curto pra cada lado:
//!
//! ```text
//! ┏━ Título                ━┓
//! ┃                         ┃
//!
//! ┃                         ┃
//! ┗━                       ━┛
//! ```
//!
//! A cor dos cantos conta o estado da caixa (o [`Tone`]): apagada no
//! repouso, ciano no foco e amarelo plasma no alerta.

use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Widget;

use crate::{Density, theme};

/// O estado que a cor dos cantos conta.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Tone {
    /// A caixa está só ali, sem pedir atenção.
    #[default]
    Idle,
    /// É a caixa em uso: a que abriu por cima, a Ficha aberta.
    Focus,
    /// Tem atraso ou regra ferida lá dentro. Amarelo plasma, nunca vermelho.
    Alert,
}

impl Tone {
    /// A cor dos cantos neste estado.
    fn style(self) -> Style {
        match self {
            Self::Idle => theme::muted(),
            Self::Focus => theme::selected(),
            Self::Alert => theme::warning(),
        }
    }
}

/// Uma caixa com colchetes nos cantos.
///
/// O `Panel` desenha só o vidro, os cantos e o título. O conteúdo é de quem
/// chama, que desenha dentro do retângulo que o [`Panel::inner`] devolve.
#[derive(Debug, Clone, Default)]
pub(crate) struct Panel<'a> {
    title: Option<Line<'a>>,
    tone: Tone,
    density: Density,
}

impl<'a> Panel<'a> {
    /// Uma caixa sem título, no repouso.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// O título, na linha de cima, logo depois do canto.
    pub(crate) fn title(mut self, title: impl Into<Line<'a>>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// O estado que os cantos mostram.
    pub(crate) fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Quanto o conteúdo se afasta dos cantos.
    pub(crate) fn density(mut self, density: Density) -> Self {
        self.density = density;
        self
    }

    /// A margem entre a borda da caixa e o conteúdo. Na compacta, uma linha
    /// em cima e embaixo e duas colunas de cada lado (a do canto e uma de
    /// respiro); na confortável, o dobro.
    fn margin(&self) -> Margin {
        match self.density {
            Density::Compact => Margin::new(2, 1),
            Density::Comfortable => Margin::new(4, 2),
        }
    }

    /// Onde o conteúdo cabe.
    pub(crate) fn inner(&self, area: Rect) -> Rect {
        area.inner(self.margin())
    }

    /// Quanto a caixa ocupa além do conteúdo: `(colunas, linhas)`. Serve pra
    /// quem calcula o tamanho da caixa a partir do texto.
    pub(crate) fn chrome(&self) -> (usize, usize) {
        let margin = self.margin();
        (
            usize::from(margin.horizontal) * 2,
            usize::from(margin.vertical) * 2,
        )
    }
}

impl Widget for Panel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, theme::surface());
        // Pequena demais pra ter cantos: fica só o vidro.
        if area.width < 4 || area.height < 2 {
            return;
        }
        let style = self.tone.style();
        let (left, right) = (area.left(), area.right() - 1);
        let (top, bottom) = (area.top(), area.bottom() - 1);
        buf.set_string(left, top, "┏━", style);
        buf.set_string(right - 1, top, "━┓", style);
        buf.set_string(left, bottom, "┗━", style);
        buf.set_string(right - 1, bottom, "━┛", style);
        // Os braços de pé só entram quando sobra uma linha livre entre os de
        // cima e os de baixo, pra o canto não virar uma barra inteira.
        if area.height >= 5 {
            for x in [left, right] {
                buf.set_string(x, top + 1, "┃", style);
                buf.set_string(x, bottom - 1, "┃", style);
            }
        }
        if let Some(title) = self.title {
            // O título começa depois do `┏━ ` e para antes do ` ━┓`.
            let room = area.width.saturating_sub(6);
            buf.set_line(left + 3, top, &title, room);
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    /// Desenha a caixa num terminal de mentira e devolve o texto e o buffer.
    fn draw(panel: Panel<'_>, width: u16, height: u16) -> (Vec<String>, Buffer) {
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("o terminal de teste abre");
        terminal
            .draw(|frame| frame.render_widget(panel, frame.area()))
            .expect("o terminal de teste desenha");
        let buffer = terminal.backend().buffer().clone();
        let rows = (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol().to_owned())
                    .collect()
            })
            .collect();
        (rows, buffer)
    }

    #[test]
    fn the_box_is_only_its_four_corners() {
        let (rows, _) = draw(Panel::new().title("Título"), 16, 6);
        assert_eq!(
            rows,
            [
                "┏━ Título     ━┓",
                "┃              ┃",
                "                ",
                "                ",
                "┃              ┃",
                "┗━            ━┛",
            ]
        );
    }

    #[test]
    fn a_short_box_drops_the_standing_arms() {
        let (rows, _) = draw(Panel::new(), 8, 3);
        assert_eq!(rows, ["┏━    ━┓", "        ", "┗━    ━┛"]);
    }

    #[test]
    fn a_long_title_stops_before_the_corner() {
        let (rows, _) = draw(Panel::new().title("Um título comprido demais"), 14, 3);
        assert_eq!(rows[0], "┏━ Um títul ━┓");
    }

    #[test]
    fn a_tiny_area_gets_only_the_glass() {
        let (rows, buffer) = draw(Panel::new().title("x"), 3, 1);
        assert_eq!(rows, ["   "]);
        assert_eq!(buffer[(0, 0)].bg, theme::SURFACE);
    }

    #[test]
    fn the_corners_tell_the_state() {
        let corner = |tone| draw(Panel::new().tone(tone), 8, 3).1[(0, 0)].fg;
        assert_eq!(corner(Tone::Idle), theme::MUTED);
        assert_eq!(corner(Tone::Focus), theme::ACCENT);
        assert_eq!(corner(Tone::Alert), theme::PLASMA);
    }

    #[test]
    fn the_content_stays_clear_of_the_corners() {
        let area = Rect::new(0, 0, 20, 8);
        let compact = Panel::new();
        assert_eq!(compact.inner(area), Rect::new(2, 1, 16, 6));
        assert_eq!(compact.chrome(), (4, 2));
        // Na confortável, o conteúdo se afasta o dobro.
        let comfortable = Panel::new().density(Density::Comfortable);
        assert_eq!(comfortable.inner(area), Rect::new(4, 2, 12, 4));
        assert_eq!(comfortable.chrome(), (8, 4));
    }
}
