//! O tema: as cores do r4ul.com.br num lugar só.
//!
//! Toda cor da TUI sai daqui, então trocar o tema é mexer neste arquivo. O
//! fundo é quase preto e o texto quase branco, com o ciano dos links como
//! destaque. Vermelho não entra: atraso e regra ferida usam o amarelo
//! plasma.

use ratatui::style::{Color, Modifier, Style};

/// O cinza do fundo, de 0 a 255. O fogo mistura as chamas com ele.
pub(crate) const BACKGROUND_GRAY: u8 = 0x0a;
/// O fundo de tudo.
pub(crate) const BACKGROUND: Color = Color::Rgb(BACKGROUND_GRAY, BACKGROUND_GRAY, BACKGROUND_GRAY);
/// O fundo das caixas (a ajuda, o quadro da curiosidade): o vidro fumê das
/// caixas de terminal do site. O fogo passa por trás dele bem mais escuro.
pub(crate) const SURFACE: Color = Color::Rgb(0x0f, 0x0f, 0x0f);
/// O texto comum.
pub(crate) const TEXT: Color = Color::Rgb(0xe0, 0xe0, 0xe0);
/// O texto que fica em segundo plano: atalhos, legendas, fontes.
pub(crate) const MUTED: Color = Color::Rgb(0x66, 0x66, 0x66);
/// Texto de apoio, entre o comum e o apagado.
pub(crate) const SOFT: Color = Color::Rgb(0xb0, 0xb0, 0xb0);
/// As linhas das bordas e dos separadores.
pub(crate) const LINE: Color = Color::Rgb(0x2a, 0x2a, 0x2a);
/// O branco puro, pra o que precisa saltar: o título da tela e o GOL.
pub(crate) const WHITE: Color = Color::Rgb(0xff, 0xff, 0xff);
/// O destaque: a tela atual, a linha escolhida, os links.
pub(crate) const ACCENT: Color = Color::Rgb(0x00, 0xd9, 0xff);
/// O segundo destaque, pra o acervo e a resenha.
pub(crate) const MAGENTA: Color = Color::Rgb(0xc5, 0x86, 0xc0);
/// O que deu certo: tarefa concluída.
pub(crate) const GREEN: Color = Color::Rgb(0x00, 0xff, 0x00);
/// O amarelo plasma: atraso, regra ferida, erro. É o lugar do vermelho, que
/// aqui não entra.
pub(crate) const PLASMA: Color = Color::Rgb(0xff, 0xff, 0x00);

/// Todas as cores do tema, pro teste que garante que nenhuma é vermelha.
#[cfg(test)]
const ALL: [Color; 11] = [
    BACKGROUND, SURFACE, TEXT, MUTED, SOFT, LINE, WHITE, ACCENT, MAGENTA, GREEN, PLASMA,
];

/// O estilo de base de toda a tela.
pub(crate) fn base() -> Style {
    Style::new().fg(TEXT).bg(BACKGROUND)
}

/// O texto comum, sem mexer no fundo de onde ele cai: dentro de uma caixa,
/// o fundo continua o da caixa.
pub(crate) fn text() -> Style {
    Style::new().fg(TEXT)
}

/// O fundo de uma caixa.
pub(crate) fn surface() -> Style {
    Style::new().fg(TEXT).bg(SURFACE)
}

/// Texto em segundo plano.
pub(crate) fn muted() -> Style {
    Style::new().fg(MUTED)
}

/// Texto de apoio.
pub(crate) fn soft() -> Style {
    Style::new().fg(SOFT)
}

/// Bordas e separadores.
pub(crate) fn line() -> Style {
    Style::new().fg(LINE)
}

/// O título de uma tela ou de um bloco.
pub(crate) fn heading() -> Style {
    Style::new().fg(WHITE).add_modifier(Modifier::BOLD)
}

/// O que está escolhido: a aba atual, a linha com o cursor.
pub(crate) fn selected() -> Style {
    Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// O fundo da linha com o cursor numa lista.
pub(crate) fn cursor() -> Style {
    Style::new()
        .fg(BACKGROUND)
        .bg(ACCENT)
        .add_modifier(Modifier::BOLD)
}

/// Atraso, regra ferida e erro: amarelo plasma.
pub(crate) fn warning() -> Style {
    Style::new().fg(PLASMA).add_modifier(Modifier::BOLD)
}

/// O que deu certo.
pub(crate) fn success() -> Style {
    Style::new().fg(GREEN)
}

/// O grito de gol por cima do fogo: preto no branco, alvinegro.
pub(crate) fn shout() -> Style {
    Style::new()
        .fg(BACKGROUND)
        .bg(WHITE)
        .add_modifier(Modifier::BOLD)
}

/// O acervo e a resenha.
pub(crate) fn museum() -> Style {
    Style::new().fg(MAGENTA)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vermelho é vermelho mesmo quando vem disfarçado: muito R, pouco G e
    /// pouco B. As cores com nome (`Color::Red`) também não passam.
    fn is_red(color: Color) -> bool {
        match color {
            Color::Red | Color::LightRed => true,
            Color::Rgb(r, g, b) => r > 0x99 && g < 0x80 && b < 0x80,
            _ => false,
        }
    }

    #[test]
    fn red_never_comes_in() {
        for color in ALL {
            assert!(!is_red(color), "{color:?} é vermelho");
        }
    }

    #[test]
    fn the_red_detector_works() {
        assert!(is_red(Color::Rgb(0xff, 0x00, 0x00)));
        assert!(is_red(Color::Rgb(0xd0, 0x30, 0x30)));
        assert!(is_red(Color::Red));
        assert!(!is_red(PLASMA));
        assert!(!is_red(MAGENTA));
    }
}
