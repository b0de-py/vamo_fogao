//! A tarefa.

use jiff::civil::Date;

use crate::{Area, Error, Fit, Interval, NotebookTitle, Plan, Progress, Status, Step, TaskId};

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
    repeat: Option<Interval>,
    plan: Option<Plan>,
    progress: Progress,
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
            repeat: None,
            plan: None,
            progress: Progress::default(),
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

    /// De quanto em quanto tempo a tarefa volta depois do gol, se volta.
    pub fn repeat(&self) -> Option<Interval> {
        self.repeat
    }

    /// O dia em que a tarefa concluída volta pro banco. `None` se ela não
    /// está concluída ou não volta.
    ///
    /// A que tem intervalo volta o intervalo depois do dia do gol. A jogada
    /// em sequência, que fechou o último passo, dorme até o primeiro dia do
    /// mês seguinte.
    pub fn returns_on(&self) -> Option<Date> {
        if self.status != Status::Done {
            return None;
        }
        let day = self.day?;
        if self.plan.is_some() {
            return day
                .first_of_month()
                .checked_add(jiff::ToSpan::months(1))
                .ok();
        }
        self.repeat?.after(day)
    }

    /// Os passos da jogada, se a tarefa for uma.
    pub fn plan(&self) -> Option<&Plan> {
        self.plan.as_ref()
    }

    /// O ponto da jogada: o passo da vez, os itens marcados e a espera.
    pub fn progress(&self) -> Progress {
        self.progress
    }

    /// O passo da vez, se a tarefa for uma jogada. Passo fora do plano (um
    /// log estranho) vale como o último.
    pub fn current_step(&self) -> Option<&Step> {
        let steps = self.plan.as_ref()?.steps();
        steps.get(usize::from(self.progress.step)).or(steps.last())
    }

    /// Se a jogada está esperando algo de fora no passo da vez.
    pub fn is_waiting(&self) -> bool {
        self.plan.is_some() && self.progress.waiting
    }

    /// Define os passos da jogada, ou tira com `None`.
    pub fn set_plan(&mut self, plan: Option<Plan>) {
        self.plan = plan;
    }

    /// Define o ponto da jogada.
    pub fn set_progress(&mut self, progress: Progress) {
        self.progress = progress;
    }

    /// Define o intervalo de volta, ou tira com `None`.
    pub fn set_repeat(&mut self, repeat: Option<Interval>) {
        self.repeat = repeat;
    }

    /// Escala a tarefa pra um dia, ou tira da escalação com `None`.
    pub fn set_day(&mut self, day: Option<Date>) {
        self.day = day;
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

        let result = task.set_status(Status::Scheduled);

        assert_eq!(
            result,
            Err(Error::InvalidTransition {
                from: Status::Idea,
                to: Status::Scheduled,
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
