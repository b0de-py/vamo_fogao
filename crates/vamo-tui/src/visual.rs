//! As escolhas de visual: quanto a tela se mexe e quanto ela respira.
//!
//! São as duas chaves da seção `[visual]` da config. A TUI não lê a config:
//! o binário lê, e os valores chegam aqui dentro do [`Setup`](crate::Setup).

use std::fmt;
use std::str::FromStr;

/// Quanto a tela se mexe. Na config, `efeitos` na seção `[visual]`.
///
/// Todo efeito da TUI pergunta por aqui antes de animar, então quem não
/// gosta de movimento (ou usa o vamo por SSH) desliga tudo num lugar só.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Effects {
    /// Tudo ligado: o fogo, o GOL! que pisca e as cenas. É o padrão.
    #[default]
    Full,
    /// Só o fogo do fundo. As cenas e as transições ficam de fora.
    Light,
    /// Nada se mexe e o mouse fica solto: a TUI só acorda quando chega uma
    /// tecla.
    Off,
}

impl Effects {
    /// Os três níveis, do mais animado ao parado.
    pub const ALL: [Self; 3] = [Self::Full, Self::Light, Self::Off];

    /// O nome do nível na config.
    pub fn name(self) -> &'static str {
        match self {
            Self::Full => "cheio",
            Self::Light => "leve",
            Self::Off => "nenhum",
        }
    }

    /// Se alguma coisa anima. Com `false`, a TUI nem pede os eventos do
    /// mouse ao terminal.
    pub fn animated(self) -> bool {
        self != Self::Off
    }
}

impl fmt::Display for Effects {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// O nome que não é de nenhum nível de efeitos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownEffects(pub String);

impl fmt::Display for UnknownEffects {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` não é um nível de efeitos: os níveis são cheio, leve e nenhum",
            self.0
        )
    }
}

impl std::error::Error for UnknownEffects {}

impl FromStr for Effects {
    type Err = UnknownEffects;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|effects| effects.name() == s)
            .ok_or_else(|| UnknownEffects(s.to_owned()))
    }
}

/// Quanto a tela respira. Na config, `densidade` na seção `[visual]`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Density {
    /// Uma linha colada na outra, pra caber mais na tela. É o padrão.
    #[default]
    Compact,
    /// Margem dentro das caixas e respiro entre os blocos.
    Comfortable,
}

impl Density {
    /// As duas densidades.
    pub const ALL: [Self; 2] = [Self::Compact, Self::Comfortable];

    /// A menor tela, em `(colunas, linhas)`, em que a confortável cabe sem
    /// espremer o conteúdo.
    pub const ROOMY: (u16, u16) = (100, 30);

    /// A densidade que cabe numa tela de `columns` por `rows`: a escolhida
    /// quando tem espaço, e a compacta quando a tela é pequena demais pra
    /// respirar.
    #[must_use]
    pub fn fitting(self, columns: u16, rows: u16) -> Self {
        if columns >= Self::ROOMY.0 && rows >= Self::ROOMY.1 {
            self
        } else {
            Self::Compact
        }
    }

    /// O nome da densidade na config.
    pub fn name(self) -> &'static str {
        match self {
            Self::Compact => "compacta",
            Self::Comfortable => "confortavel",
        }
    }
}

impl fmt::Display for Density {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// O nome que não é de nenhuma densidade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownDensity(pub String);

impl fmt::Display for UnknownDensity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` não é uma densidade: as densidades são compacta e confortavel",
            self.0
        )
    }
}

impl std::error::Error for UnknownDensity {}

impl FromStr for Density {
    type Err = UnknownDensity;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|density| density.name() == s)
            .ok_or_else(|| UnknownDensity(s.to_owned()))
    }
}

/// Quanto de um texto que entra digitado já apareceu: o passo `step` de
/// `of`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Reveal {
    /// Quantos passos já foram.
    pub(crate) step: u8,
    /// Quantos passos a digitação leva inteira.
    pub(crate) of: u8,
}

impl Reveal {
    /// Quantas letras de um texto de `len` letras já apareceram. A conta é
    /// por fração, então um texto comprido e um curto terminam juntos.
    pub(crate) fn letters(self, len: usize) -> usize {
        let (step, of) = (usize::from(self.step), usize::from(self.of.max(1)));
        (len * step.min(of)).div_ceil(of)
    }
}

/// As escolhas de visual que valem num quadro, pras telas que desenham.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Look {
    /// A densidade que cabe na tela deste quadro.
    pub(crate) density: Density,
    /// Com a curiosidade ainda entrando digitada, o quanto dela já
    /// apareceu. `None` quando ela está inteira.
    pub(crate) reveal: Option<Reveal>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_level_of_effects_comes_back_from_its_name() {
        for effects in Effects::ALL {
            assert_eq!(effects.name().parse(), Ok(effects));
        }
        assert_eq!(
            "forte".parse::<Effects>(),
            Err(UnknownEffects("forte".to_owned()))
        );
    }

    #[test]
    fn only_the_off_level_stops_everything() {
        assert!(Effects::Full.animated());
        assert!(Effects::Light.animated());
        assert!(!Effects::Off.animated());
    }

    #[test]
    fn a_typed_text_grows_step_by_step_and_ends_whole() {
        let at = |step| Reveal { step, of: 10 }.letters(95);
        assert_eq!(at(0), 0);
        assert_eq!(at(5), 48);
        assert_eq!(at(10), 95);
        // Passar do fim não passa do texto.
        assert_eq!(at(200), 95);
        // Um texto curto aparece desde o primeiro passo.
        assert_eq!(Reveal { step: 1, of: 30 }.letters(3), 1);
    }

    #[test]
    fn a_small_screen_falls_back_to_compact() {
        let (columns, rows) = Density::ROOMY;
        assert_eq!(
            Density::Comfortable.fitting(columns, rows),
            Density::Comfortable
        );
        assert_eq!(
            Density::Comfortable.fitting(columns - 1, rows),
            Density::Compact
        );
        assert_eq!(
            Density::Comfortable.fitting(columns, rows - 1),
            Density::Compact
        );
        assert_eq!(Density::Compact.fitting(columns, rows), Density::Compact);
    }

    #[test]
    fn every_density_comes_back_from_its_name() {
        for density in Density::ALL {
            assert_eq!(density.name().parse(), Ok(density));
        }
        assert_eq!(
            "apertada".parse::<Density>(),
            Err(UnknownDensity("apertada".to_owned()))
        );
    }
}
