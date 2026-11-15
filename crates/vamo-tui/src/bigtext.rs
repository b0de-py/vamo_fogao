//! Letra grande de bloco, pro placar e pros gritos da tela.
//!
//! O terminal só tem um tamanho de letra, então a letra grande é desenhada:
//! cada caractere é um mosaico de 3 por 5 pontos, e cada linha do terminal
//! mostra duas linhas de pontos com os meios-blocos `▀`, `▄` e `█` (o mesmo
//! truque do fogo). Um caractere fica com 3 colunas por 3 linhas de
//! terminal.

/// A altura de um texto grande, em linhas de terminal.
pub(crate) const HEIGHT: u16 = 3;

/// A largura de um caractere, em colunas.
const GLYPH_WIDTH: usize = 3;

/// As 5 linhas de pontos de um caractere, de cima pra baixo. Em cada linha,
/// os três bits de baixo são as três colunas: `0b101` é ponto, vazio, ponto.
type Glyph = [u8; 5];

/// O desenho de um caractere, ou `None` pro que a letra grande não sabe
/// escrever.
fn glyph(c: char) -> Option<Glyph> {
    Some(match c {
        // O zero e a letra O são o mesmo desenho.
        '0' | 'O' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b001, 0b001, 0b001],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        'G' => [0b111, 0b100, 0b101, 0b101, 0b111],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        '!' => [0b010, 0b010, 0b010, 0b000, 0b010],
        // O "x" do placar, menor que os números, no meio da altura.
        'x' => [0b000, 0b101, 0b010, 0b101, 0b000],
        '/' => [0b001, 0b001, 0b010, 0b100, 0b100],
        ' ' => [0b000; 5],
        _ => return None,
    })
}

/// Se o texto inteiro dá pra escrever em letra grande.
pub(crate) fn can_write(text: &str) -> bool {
    text.chars().all(|c| glyph(c).is_some())
}

/// Quantas colunas o texto ocupa em letra grande: 3 por caractere, com uma
/// de espaço entre eles.
pub(crate) fn width(text: &str) -> usize {
    let count = text.chars().count();
    count * GLYPH_WIDTH + count.saturating_sub(1)
}

/// O texto em letra grande: as três linhas de terminal, de cima pra baixo.
/// Caractere que a letra grande não sabe escrever vira um espaço.
pub(crate) fn rows(text: &str) -> [String; 3] {
    let glyphs: Vec<Glyph> = text.chars().map(|c| glyph(c).unwrap_or([0; 5])).collect();
    // A linha `row` do terminal mostra as linhas de pontos `2 * row` (em
    // cima) e `2 * row + 1` (embaixo). A sexta linha não existe: é vazia.
    [0, 1, 2].map(|row| {
        let mut line = String::new();
        for (index, glyph) in glyphs.iter().enumerate() {
            if index > 0 {
                line.push(' ');
            }
            let top = glyph[row * 2];
            let bottom = glyph.get(row * 2 + 1).copied().unwrap_or(0);
            for column in (0..GLYPH_WIDTH).rev() {
                let bit = 1 << column;
                line.push(match (top & bit != 0, bottom & bit != 0) {
                    (true, true) => '█',
                    (true, false) => '▀',
                    (false, true) => '▄',
                    (false, false) => ' ',
                });
            }
        }
        line
    })
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn a_score_in_big_letters() {
        assert_eq!(
            rows("3 x 11"),
            [
                "▀▀█     ▄ ▄     ▄█  ▄█ ",
                "▀▀█     ▄▀▄      █   █ ",
                "▀▀▀             ▀▀▀ ▀▀▀"
            ]
        );
    }

    #[test]
    fn every_digit_has_its_own_drawing() {
        let digits: Vec<[String; 3]> = "0123456789".chars().map(|c| rows(&c.to_string())).collect();
        for (index, digit) in digits.iter().enumerate() {
            assert!(!digits[..index].contains(digit), "o {index} repete outro");
        }
    }

    #[test]
    fn what_cannot_be_written_becomes_a_space() {
        assert!(can_write("12 x 0"));
        assert!(can_write("3/5"));
        assert!(can_write("GOOOL!"));
        assert!(can_write("FEITA!"));
        // Só as maiúsculas dos gritos: o resto do alfabeto não foi desenhado.
        assert!(!can_write("gol"));
        assert_eq!(rows("?"), rows(" "));
    }

    proptest! {
        /// Toda linha tem a largura prometida, pra quem alinha o placar.
        #[test]
        fn every_row_has_the_promised_width(text in "[0-9x/ ]{0,12}") {
            for row in rows(&text) {
                prop_assert_eq!(row.chars().count(), width(&text));
            }
        }
    }
}
