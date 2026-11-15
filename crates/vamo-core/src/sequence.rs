//! A jogada em sequência: passos em ordem, e só o passo da vez aparece.

use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use crate::Error;

/// Quantos passos uma jogada aceita.
pub const MAX_STEPS: usize = 12;
/// Quantos itens um passo aceita.
pub const MAX_ITEMS: usize = 8;
/// O tamanho máximo do título de um passo ou de um item, em caracteres.
const MAX_TEXT: usize = 60;

/// Quando o passo vence, em relação ao que veio antes.
///
/// O texto é `48h` (horas depois de fechar o passo anterior) ou `pedir` (o
/// app pergunta a data na hora de fechar o passo anterior). Sem texto, o
/// passo não tem prazo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Due {
    /// Sem prazo: o passo fica no banco até alguém escalar.
    None,
    /// Tantas horas depois de fechar o passo anterior.
    After(u16),
    /// A data é informada ao fechar o passo anterior.
    Ask,
}

impl Due {
    /// Em quantos dias, a partir de `today`, o passo vence quando o prazo é
    /// relativo. Hora quebrada conta como dia cheio: 36h são dois dias.
    pub fn days(self) -> Option<i64> {
        match self {
            Self::After(hours) => Some((i64::from(hours) + 23) / 24),
            Self::None | Self::Ask => None,
        }
    }
}

/// Um passo da jogada: um título, o prazo e, se o passo depende de mais de
/// uma coisa, os itens que se marcam em qualquer ordem.
///
/// ```
/// use vamo_core::Step;
///
/// let step: Step = "Pedir as datas @pedir + Recebi as datas + Recebi o e-mail".parse()?;
/// assert_eq!(step.title(), "Pedir as datas");
/// assert_eq!(step.items().len(), 2);
/// assert_eq!(step.to_string().parse::<Step>()?, step);
/// # Ok::<(), vamo_core::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Step {
    title: String,
    due: Due,
    items: Vec<String>,
}

impl Step {
    /// O título do passo.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// O prazo do passo.
    pub fn due(&self) -> Due {
        self.due
    }

    /// Os itens que o passo pede, marcados em qualquer ordem.
    pub fn items(&self) -> &[String] {
        &self.items
    }
}

/// Texto de passo ou de item: não pode ser vazio, comprido nem ter os
/// caracteres que o formato usa como separador.
fn clean(text: &str, whole: &str) -> Result<String, Error> {
    let text = text.trim();
    let bad = text.is_empty()
        || text.chars().count() > MAX_TEXT
        || text.chars().any(|c| matches!(c, '@' | '+' | '\n' | '\r'));
    if bad {
        Err(Error::InvalidStep {
            input: whole.to_owned(),
        })
    } else {
        Ok(text.to_owned())
    }
}

impl FromStr for Step {
    type Err = Error;

    /// `Título [@prazo] [+ item]...`, com o prazo `48h` ou `pedir`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || Error::InvalidStep {
            input: s.to_owned(),
        };
        let mut parts = s.split('+');
        let head = parts.next().ok_or_else(invalid)?;
        let (title, due) = match head.split_once('@') {
            None => (head, Due::None),
            Some((title, due)) => {
                let due = due.trim();
                let due = if due == "pedir" {
                    Due::Ask
                } else {
                    let hours = due.strip_suffix('h').ok_or_else(invalid)?;
                    match hours.parse::<u16>() {
                        Ok(hours) if hours > 0 => Due::After(hours),
                        _ => return Err(invalid()),
                    }
                };
                (title, due)
            }
        };
        let items = parts
            .map(|item| clean(item, s))
            .collect::<Result<Vec<_>, _>>()?;
        if items.len() > MAX_ITEMS {
            return Err(invalid());
        }
        Ok(Self {
            title: clean(title, s)?,
            due,
            items,
        })
    }
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.title)?;
        match self.due {
            Due::None => {}
            Due::After(hours) => write!(f, " @{hours}h")?,
            Due::Ask => f.write_str(" @pedir")?,
        }
        for item in &self.items {
            write!(f, " + {item}")?;
        }
        Ok(())
    }
}

/// Os passos de uma jogada, em ordem. De 1 a 12 passos.
///
/// Escrito em texto, é um passo por linha, no formato de [`Step`].
///
/// ```
/// use vamo_core::Plan;
///
/// let plan: Plan = "Pedir as datas @pedir\nFazer a prévia @48h".parse()?;
/// assert_eq!(plan.steps().len(), 2);
/// # Ok::<(), vamo_core::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Plan(Vec<Step>);

impl Plan {
    /// Monta o plano a partir dos passos.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidPlan`] se não tiver nenhum passo ou tiver mais de 12.
    pub fn new(steps: Vec<Step>) -> Result<Self, Error> {
        if steps.is_empty() || steps.len() > MAX_STEPS {
            return Err(Error::InvalidPlan { count: steps.len() });
        }
        Ok(Self(steps))
    }

    /// Os passos, em ordem.
    pub fn steps(&self) -> &[Step] {
        &self.0
    }
}

impl FromStr for Plan {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let steps = s
            .lines()
            .map(str::parse)
            .collect::<Result<Vec<Step>, _>>()?;
        Self::new(steps)
    }
}

impl fmt::Display for Plan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, step) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str("\n")?;
            }
            step.fmt(f)?;
        }
        Ok(())
    }
}

/// Em que ponto da jogada a tarefa está: o passo da vez, os itens dele já
/// marcados e se ele está esperando alguma coisa de fora.
///
/// Escrito em texto como `passo,itens,espera`: `1,5,1` é o segundo passo
/// (conta a partir do zero), com os itens 0 e 2 marcados (os bits de 5) e
/// esperando.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Progress {
    /// O passo da vez, a partir do zero.
    pub step: u8,
    /// Os itens do passo marcados, um bit por item.
    pub items: u8,
    /// Se o passo está aguardando algo de fora (um e-mail, uma data).
    pub waiting: bool,
}

impl Progress {
    /// Se o item `index` do passo da vez está marcado.
    pub fn is_item_checked(self, index: usize) -> bool {
        index < MAX_ITEMS && self.items & (1 << index) != 0
    }

    /// Os itens marcados, pelo número.
    pub fn checked_items(self) -> BTreeSet<usize> {
        (0..MAX_ITEMS)
            .filter(|&index| self.is_item_checked(index))
            .collect()
    }
}

impl fmt::Display for Progress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},{}", self.step, self.items, u8::from(self.waiting))
    }
}

impl FromStr for Progress {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || Error::InvalidProgress {
            input: s.to_owned(),
        };
        let mut parts = s.split(',');
        let (Some(step), Some(items), Some(waiting), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(invalid());
        };
        Ok(Self {
            step: step.parse().map_err(|_| invalid())?,
            items: items.parse().map_err(|_| invalid())?,
            waiting: match waiting {
                "0" => false,
                "1" => true,
                _ => return Err(invalid()),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn a_step_reads_title_deadline_and_items() {
        let step: Step = "Pedir as datas @pedir + Recebi as datas + Recebi o e-mail"
            .parse()
            .unwrap();
        assert_eq!(step.title(), "Pedir as datas");
        assert_eq!(step.due(), Due::Ask);
        assert_eq!(step.items(), ["Recebi as datas", "Recebi o e-mail"]);

        let step: Step = "Fazer a primeira prévia @48h".parse().unwrap();
        assert_eq!(step.due(), Due::After(48));
        assert_eq!(Due::After(48).days(), Some(2));
        assert_eq!(Due::After(36).days(), Some(2));
        assert_eq!(Due::Ask.days(), None);

        let step: Step = "Revisar".parse().unwrap();
        assert_eq!(step.due(), Due::None);
    }

    #[test]
    fn a_bad_step_is_refused() {
        for input in [
            "",
            "   ",
            "@48h",
            "Fazer @0h",
            "Fazer @48",
            "Fazer @amanhã",
            "Fazer +",
            "Fazer + a + ",
            "Fazer\nisso",
        ] {
            assert!(input.parse::<Step>().is_err(), "{input:?}");
        }
        let nine = format!("Fazer{}", " + item".repeat(9));
        assert!(nine.parse::<Step>().is_err());
    }

    #[test]
    fn a_plan_needs_between_one_and_twelve_steps() {
        assert_eq!(Plan::new(Vec::new()), Err(Error::InvalidPlan { count: 0 }));
        let step: Step = "Fazer".parse().unwrap();
        assert!(Plan::new(vec![step.clone(); 12]).is_ok());
        assert_eq!(
            Plan::new(vec![step; 13]),
            Err(Error::InvalidPlan { count: 13 })
        );
        assert!("".parse::<Plan>().is_err());
    }

    #[test]
    fn progress_reads_back_and_refuses_garbage() {
        let progress = Progress {
            step: 1,
            items: 5,
            waiting: true,
        };
        assert_eq!(progress.to_string(), "1,5,1");
        assert_eq!("1,5,1".parse(), Ok(progress));
        assert_eq!(progress.checked_items(), BTreeSet::from([0, 2]));
        for input in ["", "1", "1,5", "1,5,2", "1,5,1,0", "a,b,c", "300,0,0"] {
            assert!(input.parse::<Progress>().is_err(), "{input:?}");
        }
    }

    proptest! {
        #[test]
        fn a_plan_survives_display_then_parse(
            titles in prop::collection::vec("[A-Za-zÀ-ú][A-Za-zÀ-ú ]{0,20}[A-Za-zÀ-ú]", 1..5),
            hours in prop::option::of(1..200u16),
            items in prop::collection::vec("[A-Za-z]{1,10}", 0..3),
        ) {
            let steps: Vec<Step> = titles
                .into_iter()
                .map(|title| {
                    let due = match hours { Some(hours) => format!(" @{hours}h"), None => String::new() };
                    let items = items.iter().fold(String::new(), |text, item| text + " + " + item);
                    format!("{title}{due}{items}").parse().unwrap()
                })
                .collect();
            let plan = Plan::new(steps).unwrap();
            prop_assert_eq!(plan.to_string().parse::<Plan>(), Ok(plan));
        }

        #[test]
        fn progress_survives_display_then_parse(step: u8, items: u8, waiting: bool) {
            let progress = Progress { step, items, waiting };
            prop_assert_eq!(progress.to_string().parse(), Ok(progress));
        }

        #[test]
        fn parsing_any_text_never_panics(text in "\\PC*") {
            let _ = text.parse::<Step>();
            let _ = text.parse::<Plan>();
            let _ = text.parse::<Progress>();
        }
    }
}
