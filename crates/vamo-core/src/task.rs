//! A tarefa.

use crate::{Area, Error, Fit, Status, TaskId};

/// Uma tarefa: o que fazer, de que área, que espaço pede e em que pé está.
///
/// Os campos são privados de propósito. Uma `Task` só nasce por
/// [`Task::new`], que valida o título, e só muda de status por
/// [`Task::set_status`], que respeita as transições de [`Status`]. Assim,
/// qualquer `Task` que exista no programa está num estado válido.
///
/// ```
/// use vamo_core::{Fit, Status, Task};
///
/// let mut task = Task::new(
///     "T-041".parse()?,
///     "Trocar o filtro da água",
///     "casa".parse()?,
///     Fit::Gap,
///     Status::Ready,
/// )?;
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
    title: String,
    area: Area,
    fit: Fit,
    status: Status,
    notes: String,
}

impl Task {
    /// Cria uma tarefa, sem notas.
    ///
    /// O título chega sem os espaços das pontas.
    ///
    /// # Errors
    ///
    /// [`Error::EmptyTitle`] se o título for vazio ou só tiver espaço.
    pub fn new(
        id: TaskId,
        title: &str,
        area: Area,
        fit: Fit,
        status: Status,
    ) -> Result<Self, Error> {
        let title = title.trim();
        if title.is_empty() {
            return Err(Error::EmptyTitle);
        }
        Ok(Self {
            id,
            title: title.to_owned(),
            area,
            fit,
            status,
            notes: String::new(),
        })
    }

    /// O ID da tarefa.
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// O título da tarefa.
    pub fn title(&self) -> &str {
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

    fn task(title: &str, status: Status) -> Result<Task, Error> {
        Task::new(
            TaskId::new(41).unwrap(),
            title,
            Area::new("casa").unwrap(),
            Fit::Gap,
            status,
        )
    }

    #[test]
    fn a_new_task_keeps_what_it_was_given() {
        let task = task("Trocar o filtro da água", Status::Ready).unwrap();

        assert_eq!(task.id().to_string(), "T-041");
        assert_eq!(task.title(), "Trocar o filtro da água");
        assert_eq!(task.area().as_str(), "casa");
        assert_eq!(task.fit(), Fit::Gap);
        assert_eq!(task.status(), Status::Ready);
        assert_eq!(task.notes(), "");
    }

    #[test]
    fn the_title_is_trimmed() {
        let task = task("  Trocar o filtro da água \n", Status::Ready).unwrap();
        assert_eq!(task.title(), "Trocar o filtro da água");
    }

    #[test]
    fn an_empty_title_is_rejected() {
        assert_eq!(task("", Status::Ready), Err(Error::EmptyTitle));
        assert_eq!(task(" \t\n", Status::Ready), Err(Error::EmptyTitle));
    }

    #[test]
    fn a_forbidden_transition_leaves_the_task_as_it_was() {
        let mut task = task("Trocar o filtro da água", Status::Idea).unwrap();
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
        let mut task = task("Trocar o filtro da água", Status::Ready).unwrap();

        task.set_notes("O modelo do refil está na caixa velha.");
        assert_eq!(task.notes(), "O modelo do refil está na caixa velha.");

        task.set_notes("");
        assert_eq!(task.notes(), "");
    }
}
