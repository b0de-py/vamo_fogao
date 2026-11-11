//! A Estrela Solitária: uma estrela branca de cinco pontas, grande, no canto
//! de baixo da tela.
//!
//! Ela não é uma imagem guardada: é uma conta. Pra cada célula do fogo, a
//! [`Star::coverage`] responde quanto daquela célula cai dentro da estrela, e
//! o fogo ([`crate::fire::Fire::underlay`]) pinta a célula de acordo. Assim a
//! estrela muda de tamanho com o terminal sem perder a forma.
//!
//! É só a estrela, que é símbolo de torcedor. O escudo é marca do clube e
//! não entra no projeto.

/// O ângulo entre uma ponta da estrela e a reentrância vizinha: 36 graus.
const HALF_STEP: f64 = std::f64::consts::PI / 5.0;

/// O raio das reentrâncias de uma estrela de cinco pontas regular, como
/// fração do raio das pontas. É o que deixa as bordas de pontas opostas
/// alinhadas, como no pentagrama.
const INNER: f64 = 0.381_966;

/// A partir de que tamanho de terminal a estrela aparece. Menor que isso,
/// não sobra canto vazio pra ela.
const MIN_COLUMNS: u16 = 60;
const MIN_ROWS: u16 = 24;

/// Quantas amostras por lado a [`Star::coverage`] tira de cada célula. Com
/// 3, são 9 pontos por célula, e a borda da estrela ganha meios-tons no
/// lugar de degraus.
const SAMPLES: u8 = 3;

/// Uma estrela de cinco pontas, com uma ponta pra cima, medida em células do
/// fogo (cada linha do terminal tem duas).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Star {
    center_x: f64,
    center_y: f64,
    /// O raio das pontas.
    radius: f64,
    /// Se a estrela está acesa no máximo: no grito de gol.
    glowing: bool,
}

impl Star {
    /// A estrela do canto de baixo à direita de um terminal de `columns`
    /// por `rows`. Num terminal pequeno, não tem estrela.
    pub(crate) fn in_corner(columns: u16, rows: u16) -> Option<Self> {
        if columns < MIN_COLUMNS || rows < MIN_ROWS {
            return None;
        }
        let (width, height) = (f64::from(columns), f64::from(rows) * 2.0);
        // Um sexto da altura de raio dá uma estrela de um terço da tela.
        let radius = (height / 6.0).clamp(8.0, 16.0);
        Some(Self {
            center_x: width - radius - 3.0,
            // As duas pontas de baixo ficam a 80% da altura: logo acima da
            // brasa do placar fechado, no meio das chamas.
            center_y: height * 0.8 - radius * HALF_STEP.cos(),
            radius,
            glowing: false,
        })
    }

    /// A mesma estrela, acesa no máximo ou não.
    pub(crate) fn glowing(self, glowing: bool) -> Self {
        Self { glowing, ..self }
    }

    /// Se a estrela está acesa no máximo.
    pub(crate) fn is_glowing(&self) -> bool {
        self.glowing
    }

    /// Quanto da célula do fogo `(x, y)` cai dentro da estrela, de 0 (nada)
    /// a 1 (a célula inteira).
    pub(crate) fn coverage(&self, x: usize, y: usize) -> f64 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "posições de tela cabem com folga num f64"
        )]
        let (left, top) = (x as f64, y as f64);
        // Longe da estrela, nem precisa amostrar.
        let reach = self.radius + 1.0;
        if (left - self.center_x).abs() > reach || (top - self.center_y).abs() > reach {
            return 0.0;
        }
        let step = 1.0 / f64::from(SAMPLES);
        let mut inside = 0_u8;
        for row in 0..SAMPLES {
            for column in 0..SAMPLES {
                // O meio de cada pedacinho da célula.
                let sample_x = left + (f64::from(column) + 0.5) * step;
                let sample_y = top + (f64::from(row) + 0.5) * step;
                if self.contains(sample_x - self.center_x, sample_y - self.center_y) {
                    inside += 1;
                }
            }
        }
        f64::from(inside) / f64::from(SAMPLES * SAMPLES)
    }

    /// Se o ponto a `(dx, dy)` do centro está dentro da estrela.
    ///
    /// A estrela tem dez fatias iguais de 36 graus, cada uma espelho da
    /// vizinha. Então a conta dobra o ponto pra dentro de uma fatia só, a
    /// que vai de uma ponta (ângulo 0) até uma reentrância (36 graus), e
    /// confere de que lado da borda dessa fatia o ponto caiu.
    fn contains(&self, dx: f64, dy: f64) -> bool {
        // O ângulo a partir da ponta de cima. Na tela, o y cresce pra baixo.
        let angle = dx.atan2(-dy).rem_euclid(2.0 * HALF_STEP);
        let angle = if angle > HALF_STEP {
            2.0 * HALF_STEP - angle
        } else {
            angle
        };
        let distance = dx.hypot(dy);
        let (x, y) = (distance * angle.cos(), distance * angle.sin());
        // A borda da fatia vai da ponta até a reentrância.
        let (tip_x, tip_y) = (self.radius, 0.0);
        let (dent_x, dent_y) = (
            self.radius * INNER * HALF_STEP.cos(),
            self.radius * INNER * HALF_STEP.sin(),
        );
        // O produto vetorial diz de que lado da borda o ponto está; o centro
        // da estrela fica do lado positivo.
        (dent_x - tip_x) * (y - tip_y) - (dent_y - tip_y) * (x - tip_x) >= 0.0
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::*;

    /// Uma estrela de raio 10 com o centro na célula (12, 12).
    fn star() -> Star {
        Star {
            center_x: 12.0,
            center_y: 12.0,
            radius: 10.0,
            glowing: false,
        }
    }

    #[test]
    fn the_star_has_five_points_and_one_points_up() {
        let picture = (0..24)
            .map(|y| {
                (0..24)
                    .map(|x| match star().coverage(x, y) {
                        covered if covered >= 0.75 => '#',
                        covered if covered >= 0.25 => '+',
                        _ => '.',
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert_snapshot!(picture);
    }

    #[test]
    fn inside_and_outside() {
        let star = star();
        assert!(star.contains(0.0, 0.0));
        // A ponta de cima, e logo depois dela.
        assert!(star.contains(0.0, -9.5));
        assert!(!star.contains(0.0, -10.5));
        // Entre duas pontas, a estrela só vai até a reentrância.
        let (sin, cos) = HALF_STEP.sin_cos();
        assert!(star.contains(3.0 * sin, -3.0 * cos));
        assert!(!star.contains(5.0 * sin, -5.0 * cos));
        // Pra baixo não tem ponta: tem a reentrância entre as duas de baixo.
        assert!(!star.contains(0.0, 5.0));
    }

    #[test]
    fn the_star_is_the_same_on_both_sides() {
        let star = star();
        for y in 0..24 {
            for x in 0..12 {
                let (left, right) = (star.coverage(x, y), star.coverage(23 - x, y));
                assert!((left - right).abs() < 1e-9, "({x}, {y})");
            }
        }
    }

    #[test]
    fn the_star_sits_in_the_bottom_right_corner() {
        let star = Star::in_corner(172, 45).unwrap();
        // Dentro da tela, com folga à direita e acima do rodapé.
        assert!(star.center_x + star.radius < 172.0);
        assert!(star.center_y + star.radius < 90.0);
        assert!(star.center_x > 86.0 && star.center_y > 45.0);
        assert!(!star.is_glowing());
        assert!(star.glowing(true).is_glowing());
    }

    #[test]
    fn a_small_terminal_has_no_star() {
        assert!(Star::in_corner(59, 45).is_none());
        assert!(Star::in_corner(172, 23).is_none());
        assert!(Star::in_corner(60, 24).is_some());
    }
}
