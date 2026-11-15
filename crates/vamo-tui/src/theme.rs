//! O tema: as cores do r4ul.com.br num lugar só.
//!
//! Toda cor da TUI sai daqui, então trocar o tema é mexer neste arquivo. O
//! fundo é quase preto e o texto quase branco, com o ciano dos links como
//! destaque. Vermelho não entra: atraso e regra ferida usam o amarelo
//! plasma.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;

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
/// O fundo da linha escolhida depois que o brilho assenta: o ciano do
/// destaque bem escuro, pra linha continuar marcada sem gritar.
pub(crate) const CURSOR_REST: Color = Color::Rgb(0x00, 0x38, 0x42);
/// O segundo destaque, pra o acervo e a resenha.
pub(crate) const MAGENTA: Color = Color::Rgb(0xc5, 0x86, 0xc0);
/// O que deu certo: tarefa concluída.
pub(crate) const GREEN: Color = Color::Rgb(0x00, 0xff, 0x00);
/// O amarelo plasma: atraso, regra ferida, erro. É o lugar do vermelho, que
/// aqui não entra.
pub(crate) const PLASMA: Color = Color::Rgb(0xff, 0xff, 0x00);

/// Todas as cores do tema, pro teste que garante que nenhuma é vermelha.
#[cfg(test)]
const ALL: [Color; 12] = [
    BACKGROUND,
    SURFACE,
    TEXT,
    MUTED,
    SOFT,
    LINE,
    WHITE,
    ACCENT,
    CURSOR_REST,
    MAGENTA,
    GREEN,
    PLASMA,
];

/// Uma cor entre `from` e `to`: `amount` vai de 0 (só `from`) a `of` (só
/// `to`). Cor que não é RGB fica como `to`.
pub(crate) fn blend(from: Color, to: Color, amount: u8, of: u8) -> Color {
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

/// A marca da linha com o cursor numa lista. As telas pintam a linha com
/// este estilo, e o `glow::settle` troca pela cor do momento do brilho.
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

/// Uma tecla no rodapé: branco em negrito num fundo um tom acima do vidro,
/// pra parecer a tampa de uma tecla.
pub(crate) fn keycap() -> Style {
    Style::new().fg(WHITE).bg(LINE).add_modifier(Modifier::BOLD)
}

/// O acervo e a resenha.
pub(crate) fn museum() -> Style {
    Style::new().fg(MAGENTA)
}

/// O rótulo de uma seção (Escalação, Atrasadas, Súmula): ciano e em negrito.
pub(crate) fn section() -> Style {
    Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// A linha de um rótulo de seção, já em caixa-alta espaçada e na cor de
/// rótulo.
pub(crate) fn section_line(text: &str) -> Line<'static> {
    Line::styled(spaced_caps(text), section())
}

/// Um rótulo em caixa-alta espaçada, como os labels do r4ul.com.br:
/// `Voltam em breve` vira `V O L T A M   E M   B R E V E`.
///
/// O terminal não tem espaçamento entre letras, então o espaço é uma célula
/// inteira entre cada letra, e três entre as palavras. Fica só nos rótulos
/// de seção, que são curtos: num título de tarefa, dobraria a largura.
pub(crate) fn spaced_caps(text: &str) -> String {
    let words: Vec<String> = text
        .split_whitespace()
        .map(|word| {
            let letters: Vec<String> = word.chars().map(|c| c.to_uppercase().collect()).collect();
            letters.join(" ")
        })
        .collect();
    words.join("   ")
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
    fn a_label_goes_to_spaced_caps() {
        assert_eq!(spaced_caps("Escalação"), "E S C A L A Ç Ã O");
        assert_eq!(
            spaced_caps("Voltam em  breve"),
            "V O L T A M   E M   B R E V E"
        );
        assert_eq!(spaced_caps(""), "");
    }

    #[test]
    fn a_blend_walks_from_one_color_to_the_other() {
        assert_eq!(blend(BACKGROUND, WHITE, 0, 4), BACKGROUND);
        assert_eq!(blend(BACKGROUND, WHITE, 4, 4), WHITE);
        assert_eq!(
            blend(Color::Rgb(0, 0, 0), Color::Rgb(100, 200, 40), 1, 4),
            Color::Rgb(25, 50, 10)
        );
        // Passar do fim não passa da cor de chegada.
        assert_eq!(blend(BACKGROUND, WHITE, 9, 4), WHITE);
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
