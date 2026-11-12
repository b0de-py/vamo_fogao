//! A tarefa.

use std::collections::BTreeSet;

use jiff::civil::Date;

use crate::{Area, Error, Fit, NotebookTitle, Status, TaskId};

/// Uma tarefa: o que fazer, de que área, que espaço pede e em que pé está.
///
/// Os campos são privados de propósito. Uma `Task` só nasce por
/// [`Task::new`], com peças que já se validaram sozinhas (o título é um
/// [`NotebookTitle`], a área é uma [`Area`]), e só muda de status por
/// [`Task::set_status`], que respeita as transições de [`Status`]. Assim,
/// qualquer `Task` que exista no programa está num estado válido.
///
/// ```
/// use vamo_core::{Fit, Status, Task};
///
/// let mut task = Task::new(
///     "T-041".parse()?,
///     "Trocar o filtro da água".parse()?,
///     "casa".parse()?,
///     Fit::Gap,
///     Status::Ready,
/// );
///
/// task.set_status(Status::Done)?;
/// assert_eq!(task.status(), Status::Done);
///
/// // Concluída não volta pra agendada: só o VAR (concluída → pronta).
/// assert!(task.set_status(Status::Scheduled).is_err());
/// # Ok::<(), vamo_core::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    id: TaskId,
    title: NotebookTitle,
    area: Area,
    fit: Fit,
    status: Status,
    notes: String,
    day: Option<Date>,
    checks: BTreeSet<Date>,
}

impl Task {
    /// Cria uma tarefa, sem notas.
    ///
    /// Não tem o que dar errado aqui: cada peça já chega validada pelo
    /// próprio tipo.
    pub fn new(id: TaskId, title: NotebookTitle, area: Area, fit: Fit, status: Status) -> Self {
        Self {
            id,
            title,
            area,
            fit,
            status,
            notes: String::new(),
            day: None,
            checks: BTreeSet::new(),
        }
    }

    /// O ID da tarefa.
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// O título da tarefa.
    pub fn title(&self) -> &NotebookTitle {
        &self.title
    }

    /// A área da tarefa.
    pub fn area(&self) -> &Area {
        &self.area
    }

    /// O encaixe da tarefa.
    pub fn fit(&self) -> Fit {
        self.fit
    }

    /// O status da tarefa.
    pub fn status(&self) -> Status {
        self.status
    }

    /// As notas da tarefa: o detalhe que não cabe no título. Vazio quando
    /// não tem.
    pub fn notes(&self) -> &str {
        &self.notes
    }

    /// Quanto a tarefa deve levar, se as notas disserem: `~30 min`, `~1 h`,
    /// `~1h30`. É o que decide se ela cabe numa brecha.
    pub fn estimate(&self) -> Option<jiff::SignedDuration> {
        crate::gaps::estimate(&self.notes)
    }

    /// O dia em que a tarefa está escalada, se estiver.
    pub fn day(&self) -> Option<Date> {
        self.day
    }

    /// Se a tarefa (recorrente) foi marcada nesse dia.
    pub fn is_checked(&self, day: Date) -> bool {
        self.checks.contains(&day)
    }

    /// Escala a tarefa pra um dia, ou tira da escalação com `None`.
    pub fn set_day(&mut self, day: Option<Date>) {
        self.day = day;
    }

    /// Marca ou desmarca a tarefa num dia.
    pub fn set_checked(&mut self, day: Date, done: bool) {
        if done {
            self.checks.insert(day);
        } else {
            self.checks.remove(&day);
        }
    }

    /// Muda o status, se a regra de [`Status::can_become`] deixar.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidTransition`] quando a regra não deixa. Nesse caso a
    /// tarefa continua como estava.
    pub fn set_status(&mut self, next: Status) -> Result<(), Error> {
        self.status = self.status.transition_to(next)?;
        Ok(())
    }

    /// Troca as notas da tarefa.
    pub fn set_notes(&mut self, notes: &str) {
        notes.clone_into(&mut self.notes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(status: Status) -> Task {
        Task::new(
            TaskId::new(41).unwrap(),
            NotebookTitle::new("Trocar o filtro da água").unwrap(),
            Area::new("casa").unwrap(),
            Fit::Gap,
            status,
        )
    }

    #[test]
    fn a_new_task_keeps_what_it_was_given() {
        let task = task(Status::Ready);

        assert_eq!(task.id().to_string(), "T-041");
        assert_eq!(task.title().as_str(), "Trocar o filtro da água");
        assert_eq!(task.area().as_str(), "casa");
        assert_eq!(task.fit(), Fit::Gap);
        assert_eq!(task.status(), Status::Ready);
        assert_eq!(task.notes(), "");
    }

    #[test]
    fn a_forbidden_transition_leaves_the_task_as_it_was() {
        let mut task = task(Status::Idea);
        let before = task.clone();

        let result = task.set_status(Status::Done);

        assert_eq!(
            result,
            Err(Error::InvalidTransition {
                from: Status::Idea,
                to: Status::Done,
            })
        );
        assert_eq!(task, before);
    }

    #[test]
    fn notes_can_be_replaced() {
        let mut task = task(Status::Ready);

        task.set_notes("O modelo do refil está na caixa velha.");
        assert_eq!(task.notes(), "O modelo do refil está na caixa velha.");

        task.set_notes("");
        assert_eq!(task.notes(), "");
    }
}
