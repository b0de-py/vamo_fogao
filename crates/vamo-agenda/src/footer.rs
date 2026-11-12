//! O rodapé `vamo:`, o contrato entre a agenda e o `vamo_fogao`: uma linha
//! na descrição do evento que diz qual tarefa ele é.
//!
//! ```text
//! vamo: T-041 | Trocar o filtro da água | casa | brecha
//! ```
//!
//! Quem marca o horário na agenda (você, ou o Claude pela caixa "a marcar")
//! cola essa linha na descrição. No `vamo sync`, o evento liga à tarefa, e
//! ela passa pra agendada no dia do evento.

use std::fmt;

use vamo_core::{Area, Fit, NotebookTitle, Task, TaskId};

/// O rodapé lido de uma descrição. Só o ID é obrigatório: o título, a área
/// e o encaixe estão lá pra quem lê a agenda, e o `vamo` confia no banco.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Footer {
    /// A tarefa.
    pub task: TaskId,
    /// O título, se veio.
    pub title: Option<NotebookTitle>,
    /// A área, se veio.
    pub area: Option<Area>,
    /// O encaixe, se veio.
    pub fit: Option<Fit>,
}

impl Footer {
    /// O rodapé completo de uma tarefa, pra colar na descrição do evento.
    ///
    /// ```
    /// use vamo_agenda::Footer;
    /// use vamo_core::{Fit, Status, Task};
    ///
    /// let task = Task::new(
    ///     "T-041".parse()?,
    ///     "Trocar o filtro da água".parse()?,
    ///     "casa".parse()?,
    ///     Fit::Gap,
    ///     Status::Ready,
    /// );
    /// assert_eq!(
    ///     Footer::of(&task).to_string(),
    ///     "vamo: T-041 | Trocar o filtro da água | casa | brecha"
    /// );
    /// # Ok::<(), vamo_core::Error>(())
    /// ```
    pub fn of(task: &Task) -> Self {
        Self {
            task: task.id(),
            title: Some(task.title().clone()),
            area: Some(task.area().clone()),
            fit: Some(task.fit()),
        }
    }

    /// Acha o rodapé no meio de uma descrição. Vale a última linha que
    /// começa com `vamo:` (sem ligar pra maiúscula e espaço em volta) e tem
    /// um ID de tarefa. Sem uma linha assim, `None`.
    ///
    /// O leitor é tolerante: o título pode ter `|` no meio, e área ou
    /// encaixe que não batem com os do banco ficam de fora em vez de
    /// derrubar a linha.
    ///
    /// ```
    /// use vamo_agenda::Footer;
    ///
    /// let description = "Levar a nota fiscal.\n  VAMO:t-41|Trocar o filtro|casa|brecha \n";
    /// let footer = Footer::find(description).unwrap();
    /// assert_eq!(footer.task.to_string(), "T-041");
    /// ```
    pub fn find(description: &str) -> Option<Self> {
        description.lines().rev().find_map(Self::parse_line)
    }

    fn parse_line(line: &str) -> Option<Self> {
        let line = line.trim();
        let prefix = line.get(..5)?;
        if !prefix.eq_ignore_ascii_case("vamo:") {
            return None;
        }
        let mut fields: Vec<&str> = line[5..].split('|').map(str::trim).collect();
        let task = parse_id(fields.first()?)?;
        fields.remove(0);

        // A área e o encaixe ficam no fim; o que sobra no meio é o título,
        // que pode ter `|`.
        let fit = fields.last().and_then(|field| field.parse::<Fit>().ok());
        if fit.is_some() {
            fields.pop();
        }
        // Com um campo só sobrando, ele pode ser o título ou a área: se tem
        // cara de título ("ler"), fica sendo o título.
        let area = fields
            .last()
            .filter(|field| fields.len() > 1 || field.parse::<NotebookTitle>().is_err())
            .and_then(|field| field.parse::<Area>().ok());
        if area.is_some() {
            fields.pop();
        }
        let title = Some(fields.join(" | "))
            .filter(|title| !title.is_empty())
            .and_then(|title| title.parse().ok());
        Some(Self {
            task,
            title,
            area,
            fit,
        })
    }
}

/// O ID com tolerância: `T-041`, `t-41`, `T041`.
fn parse_id(text: &str) -> Option<TaskId> {
    let upper = text.trim().to_uppercase();
    let digits = upper
        .strip_prefix("T-")
        .or_else(|| upper.strip_prefix('T'))?;
    let number: u32 = digits.parse().ok()?;
    TaskId::new(number)
}

impl fmt::Display for Footer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "vamo: {}", self.task)?;
        if let Some(title) = &self.title {
            write!(f, " | {title}")?;
        }
        if let Some(area) = &self.area {
            write!(f, " | {area}")?;
        }
        if let Some(fit) = self.fit {
            write!(f, " | {fit}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn the_footer_is_found_among_other_lines() {
        let description = "Pauta:\n- ver o orçamento\n\nvamo: T-041 | Trocar o filtro da água | casa | brecha\nLink da sala: exemplo";
        let footer = Footer::find(description).unwrap();
        assert_eq!(footer.task, TaskId::new(41).unwrap());
        assert_eq!(footer.title.unwrap().as_str(), "Trocar o filtro da água");
        assert_eq!(footer.area.unwrap().as_str(), "casa");
        assert_eq!(footer.fit, Some(Fit::Gap));
    }

    #[test]
    fn only_the_id_is_needed() {
        let footer = Footer::find("vamo: T-7").unwrap();
        assert_eq!(footer.task, TaskId::new(7).unwrap());
        assert_eq!((footer.title, footer.area, footer.fit), (None, None, None));
    }

    #[test]
    fn spaces_case_and_accents_are_tolerated() {
        let footer =
            Footer::find("   Vamo:   t-12 |  Pôr o lixo   pra fora|casa|com-alguem  ").unwrap();
        assert_eq!(footer.task, TaskId::new(12).unwrap());
        assert_eq!(footer.title.unwrap().as_str(), "Pôr o lixo pra fora");
        assert_eq!(footer.fit, Some(Fit::WithSomeone));
    }

    #[test]
    fn a_title_with_a_bar_survives() {
        let footer = Footer::find("vamo: T-3 | Revisar A|B | trampo | foco").unwrap();
        assert_eq!(footer.title.unwrap().as_str(), "Revisar A | B");
        assert_eq!(footer.area.unwrap().as_str(), "trampo");
    }

    #[test]
    fn lines_without_an_id_are_not_footers() {
        for text in [
            "vamo: amanhã",
            "vamos: T-041",
            "o vamo: T-041 é isso",
            "",
            "vamo:",
        ] {
            assert_eq!(Footer::find(text), None, "{text}");
        }
    }

    fn title() -> impl Strategy<Value = NotebookTitle> {
        (
            prop::sample::select(vec!["Trocar", "Pôr", "Ler", "Ligar", "Pagar", "Revisar"]),
            proptest::collection::vec("[a-zçãéóA-Z0-9]{1,8}", 0..=4),
        )
            .prop_map(|(verb, rest)| {
                let mut words = vec![verb.to_owned()];
                words.extend(rest);
                words.join(" ").parse().unwrap()
            })
    }

    /// Uma área que não se confunde com encaixe nem com título: no rodapé
    /// sem os outros campos, "foco" seria o encaixe e "ler", o título.
    fn area() -> impl Strategy<Value = Area> {
        "[a-z0-9]{1,8}(-[a-z0-9]{1,8}){0,2}"
            .prop_filter("área com cara de encaixe ou de título", |name| {
                name.parse::<Fit>().is_err() && name.parse::<NotebookTitle>().is_err()
            })
            .prop_map(|name| name.parse().unwrap())
    }

    proptest! {
        #[test]
        fn writing_then_reading_gives_the_same_footer(
            number in 1_u32..1000,
            title in proptest::option::of(title()),
            area in proptest::option::of(area()),
            fit in proptest::option::of(prop::sample::select(Fit::ALL.to_vec())),
        ) {
            let footer = Footer {
                task: TaskId::new(number).unwrap(),
                title,
                area,
                fit,
            };
            let description = format!("Outra linha\n{footer}\n");
            prop_assert_eq!(Footer::find(&description), Some(footer));
        }
    }
}
