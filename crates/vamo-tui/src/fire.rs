//! O fogo alvinegro do rodapé: uma simulação de fogo em preto e branco cuja
//! altura acompanha o placar do dia.
//!
//! É o fogo clássico das demos (o do Doom no PlayStation): a linha de baixo é
//! a brasa, e a cada passo cada célula puxa o calor da de baixo, perdendo um
//! pouco e escorregando pro lado ao acaso. Quanto mais quente a brasa, mais
//! alto o fogo sobe. O acaso vem de um gerador com semente fixa, então a
//! mesma semente e os mesmos passos dão sempre o mesmo fogo, e dá pra testar.
//!
//! Cada linha do terminal mostra duas linhas do fogo com o meio-bloco `▀`: a
//! cor da letra é a célula de cima e a do fundo, a de baixo.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

/// A semente do fogo do app. Qualquer número serve; fixo, o fogo começa
/// igual toda vez.
pub(crate) const SEED: u64 = 0xF060_1904_0812_0002;

/// A brasa mínima, a do fogo piloto: com o placar zerado, ainda tem uma
/// chama baixinha.
const PILOT: f64 = 0.15;

/// O fogo: uma grade de calor, de 0 (apagado) a [`Fire::hottest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Fire {
    width: usize,
    /// Linhas do fogo, o dobro das linhas do terminal.
    height: usize,
    /// O calor de cada célula, linha por linha, de cima pra baixo.
    heat: Vec<u8>,
    /// O estado do gerador de acaso.
    state: u64,
}

impl Fire {
    /// Um fogo apagado, com a semente dada.
    pub(crate) fn new(seed: u64) -> Self {
        Self {
            width: 0,
            height: 0,
            heat: Vec::new(),
            state: seed,
        }
    }

    /// O calor máximo. A perda média é um ponto por linha, então com a
    /// brasa no máximo o fogo chega perto do topo, e com metade, à metade.
    fn hottest(&self) -> u8 {
        u8::try_from(self.height).unwrap_or(u8::MAX)
    }

    /// Ajusta o tamanho do fogo ao da área, em células do terminal. Com o
    /// tamanho mudado, o fogo recomeça apagado.
    pub(crate) fn resize(&mut self, columns: u16, rows: u16) {
        let (width, height) = (usize::from(columns), usize::from(rows) * 2);
        if (width, height) != (self.width, self.height) {
            self.width = width;
            self.height = height;
            self.heat = vec![0; width * height];
        }
    }

    /// Avança um passo. A `strength`, de 0 a 1, é o quanto a brasa está
    /// acesa: o placar do dia, ou 1 no grito de gol.
    pub(crate) fn step(&mut self, strength: f64) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        let hottest = f64::from(self.hottest());
        let strength = PILOT + (1.0 - PILOT) * strength.clamp(0.0, 1.0);
        // A brasa: a linha de baixo, com um tremor de até um ponto.
        let bottom = (self.height - 1) * self.width;
        for x in 0..self.width {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "o valor fica entre 0 e o calor máximo, que cabe num u8"
            )]
            let base = (hottest * strength).round() as u8;
            let flicker = u8::from(self.next().is_multiple_of(4));
            self.heat[bottom + x] = base.saturating_sub(flicker);
        }
        // Cada célula puxa o calor da de baixo, de cima pra baixo pra não
        // usar o valor que acabou de mudar.
        for y in 0..self.height - 1 {
            for x in 0..self.width {
                let random = self.next();
                let below = self.heat[(y + 1) * self.width + x];
                // A perda: 0, 1 ou 2 pontos, um em média.
                let loss = u8::try_from(random % 3).unwrap_or(1);
                // O vento: a chama vai pra esquerda, pro lugar ou pra direita.
                let drift = usize::try_from((random >> 8) % 3).unwrap_or(1);
                let target = (x + self.width + drift - 1) % self.width;
                self.heat[y * self.width + target] = below.saturating_sub(loss);
            }
        }
    }

    /// O calor de uma célula, de 0 a 1.
    fn level(&self, x: usize, y: usize) -> f64 {
        f64::from(self.heat[y * self.width + x]) / f64::from(self.hottest().max(1))
    }

    /// Quantas linhas do fogo têm alguma chama.
    #[cfg(test)]
    fn flame_height(&self) -> usize {
        (0..self.height)
            .find(|&y| (0..self.width).any(|x| self.heat[y * self.width + x] > 0))
            .map_or(0, |top| self.height - top)
    }

    /// O próximo número do gerador (xorshift64*, curto e bom o bastante pra
    /// enfeite).
    fn next(&mut self) -> u64 {
        // O xorshift não sai do zero; a semente zero vira outra.
        if self.state == 0 {
            self.state = SEED;
        }
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        self.state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

/// A cor de um calor: do fundo quase preto ao branco, passando pelos cinzas.
/// Vermelho não entra, nem no fogo.
fn shade(level: f64) -> Color {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "o nível fica entre 0 e 1, então o cinza fica entre 10 e 255"
    )]
    let gray = (10.0 + level.clamp(0.0, 1.0) * 245.0).round() as u8;
    Color::Rgb(gray, gray, gray)
}

impl Widget for &Fire {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rows = (self.height / 2).min(usize::from(area.height));
        let columns = self.width.min(usize::from(area.width));
        for row in 0..rows {
            for x in 0..columns {
                let (Ok(dx), Ok(dy)) = (u16::try_from(x), u16::try_from(row)) else {
                    continue;
                };
                let top = self.level(x, row * 2);
                let bottom = self.level(x, row * 2 + 1);
                if top == 0.0 && bottom == 0.0 {
                    continue;
                }
                if let Some(cell) = buf.cell_mut((area.x + dx, area.y + dy)) {
                    cell.set_char('▀').set_fg(shade(top)).set_bg(shade(bottom));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::*;

    /// Um fogo de 24 por 3 linhas do terminal, depois de `steps` passos.
    fn burnt(strength: f64, steps: usize) -> Fire {
        let mut fire = Fire::new(SEED);
        fire.resize(24, 3);
        for _ in 0..steps {
            fire.step(strength);
        }
        fire
    }

    /// O fogo em texto, um dígito de calor por célula (0 a 9).
    fn picture(fire: &Fire) -> String {
        (0..fire.height)
            .map(|y| {
                (0..fire.width)
                    .map(|x| {
                        #[expect(
                            clippy::cast_possible_truncation,
                            clippy::cast_sign_loss,
                            reason = "o nível fica entre 0 e 1"
                        )]
                        let digit = (fire.level(x, y) * 9.0).round() as u32;
                        char::from_digit(digit, 10).unwrap_or('?')
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_same_seed_burns_the_same() {
        assert_eq!(burnt(0.5, 40), burnt(0.5, 40));
        let mut other = Fire::new(SEED + 1);
        other.resize(24, 3);
        for _ in 0..40 {
            other.step(0.5);
        }
        assert_ne!(burnt(0.5, 40), other);
    }

    #[test]
    fn a_fixed_seed_gives_a_fixed_fire() {
        assert_snapshot!(picture(&burnt(0.6, 30)));
    }

    #[test]
    fn the_flame_grows_with_the_score() {
        // A média de muitos passos, pra o acaso não decidir sozinho.
        let average = |strength| {
            let mut fire = burnt(strength, 20);
            let mut total = 0;
            for _ in 0..200 {
                fire.step(strength);
                total += fire.flame_height();
            }
            total / 200
        };
        let (empty, half, full) = (average(0.0), average(0.5), average(1.0));
        assert!(empty < half && half < full, "{empty} {half} {full}");
        assert!(empty > 0, "o fogo piloto apagou");
        assert_eq!(full, 6, "com o placar fechado, o fogo chega ao topo");
    }

    #[test]
    fn a_new_size_starts_cold() {
        let mut fire = burnt(1.0, 10);
        fire.resize(30, 2);
        assert_eq!(fire.flame_height(), 0);
        // O mesmo tamanho não apaga nada.
        fire.step(1.0);
        fire.resize(30, 2);
        assert!(fire.flame_height() > 0);
    }

    #[test]
    fn no_red_in_the_fire() {
        for step in 0..=10 {
            let Color::Rgb(r, g, b) = shade(f64::from(step) / 10.0) else {
                panic!("o fogo só usa cinza");
            };
            assert!(r == g && g == b);
        }
    }
}
