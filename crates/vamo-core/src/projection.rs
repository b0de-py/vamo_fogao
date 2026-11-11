//! A projeção: o estado das tarefas calculado a partir do log de eventos.

use std::collections::BTreeMap;

use jiff::Timestamp;
use jiff::civil::Date;

use crate::{Area, Change, Error, Event, EventId, Fit, NotebookTitle, Status, Task, TaskId};

/// O estado de todas as tarefas, calculado a partir dos eventos.
///
/// Cada campo de cada tarefa fica com o valor do evento mais recente que
/// mexeu nele: é o "último que escreveu ganha", campo a campo. Mais recente
/// quer dizer o maior instante e, no empate exato, o maior ID de evento.
///
/// Essa regra tem duas consequências que a sincronia entre máquinas usa:
///
/// - A ordem de chegada não importa. Aplicar os mesmos eventos em qualquer
///   ordem dá o mesmo estado, então cada máquina pode receber os eventos das
///   outras quando der.
/// - Receber o mesmo evento duas vezes não muda nada.
///
/// Quem decide o mais recente é o relógio de cada máquina. Uma máquina com o
/// relógio muito atrasado pode ver uma mudança sua perder pra uma mais
/// antiga, então vale deixar o relógio certo (o NTP do sistema cuida disso).
///
/// O [`apply`](Projection::apply) não confere regra nenhuma: ele aplica
/// fatos. A regra (título do caderno, transição de status) é conferida
/// antes, quando o evento é criado, por métodos como
/// [`status_change`](Projection::status_change).
///
/// ```
/// use jiff::Timestamp;
/// use uuid::Uuid;
/// use vamo_core::{Change, Event, EventId, Fit, Projection, Status, TaskId};
///
/// let created = Event {
///     id: EventId::new(Uuid::parse_str("0192f0c4-8a00-7000-8000-000000000001")?)?,
///     at: "2026-11-11T12:00:00Z".parse::<Timestamp>()?,
///     machine: "notebook".parse()?,
///     task: "T-041".parse()?,
///     change: Change::Created {
///         title: "Trocar o filtro da água".parse()?,
///         area: "casa".parse()?,
///         fit: Fit::Gap,
///         status: Status::Ready,
///     },
/// };
/// let done = Event {
///     id: EventId::new(Uuid::parse_str("0192f0c4-8a00-7000-8000-000000000002")?)?,
///     at: "2026-11-11T18:30:00Z".parse::<Timestamp>()?,
///     change: Change::Status(Status::Done),
///     ..created.clone()
/// };
///
/// // A ordem de chegada não importa.
/// let projection: Projection = [&done, &created].into_iter().collect();
///
/// let task = projection.task("T-041".parse()?).expect("a tarefa foi criada");
/// assert_eq!(task.status(), Status::Done);
/// assert_eq!(projection.next_id(), TaskId::new(42));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct Projection {
    tasks: BTreeMap<TaskId, Draft>,
}

impl Projection {
    /// Uma projeção vazia, sem nenhuma tarefa.
    pub fn new() -> Self {
        Self::default()
    }

    /// Aplica um evento.
    pub fn apply(&mut self, event: &Event) {
        let stamp = Stamp {
            at: event.at,
            id: event.id,
        };
        let draft = self.tasks.entry(event.task).or_default();
        match &event.change {
            Change::Created {
                title,
                area,
                fit,
                status,
            } => {
                draft.created = true;
                write(&mut draft.title, title, stamp);
                write(&mut draft.area, area, stamp);
                write(&mut draft.fit, fit, stamp);
                write(&mut draft.status, status, stamp);
            }
            Change::Title(title) => write(&mut draft.title, title, stamp),
            Change::Area(area) => write(&mut draft.area, area, stamp),
            Change::Fit(fit) => write(&mut draft.fit, fit, stamp),
            Change::Status(status) => write(&mut draft.status, status, stamp),
            Change::Notes(notes) => write(&mut draft.notes, notes, stamp),
            Change::Day(day) => write(&mut draft.day, day, stamp),
            // Cada dia é um campo à parte: marcar hoje não mexe em ontem.
            Change::Check { day, done } => {
                write(draft.checks.entry(*day).or_default(), done, stamp);
            }
        }
    }

    /// A tarefa com esse ID, se ela já foi criada.
    pub fn task(&self, id: TaskId) -> Option<Task> {
        self.tasks.get(&id).and_then(|draft| draft.to_task(id))
    }

    /// Todas as tarefas criadas, em ordem de ID.
    pub fn tasks(&self) -> impl Iterator<Item = Task> {
        self.tasks
            .iter()
            .filter_map(|(&id, draft)| draft.to_task(id))
    }

    /// A mudança que passa a tarefa pro status pedido, se a regra deixar.
    ///
    /// É assim que um comando como "concluir" vira evento: o estado atual
    /// confere a regra, e o que volta é a [`Change`] pra registrar no log.
    ///
    /// # Errors
    ///
    /// - [`Error::UnknownTask`] se a tarefa não existir.
    /// - [`Error::InvalidTransition`] se a regra de [`Status`] não deixar.
    pub fn status_change(&self, id: TaskId, next: Status) -> Result<Change, Error> {
        let task = self.task(id).ok_or(Error::UnknownTask { id })?;
        task.status().transition_to(next)?;
        Ok(Change::Status(next))
    }

    /// As mudanças que concluem a tarefa: o gol, e o gol conta no dia em
    /// que saiu. Se a tarefa estava escalada pra outro dia (ou pra nenhum),
    /// ela passa pra `today`.
    ///
    /// # Errors
    ///
    /// - [`Error::UnknownTask`] se a tarefa não existir.
    /// - [`Error::InvalidTransition`] se a regra de [`Status`] não deixar.
    pub fn finish(&self, id: TaskId, today: Date) -> Result<Vec<Change>, Error> {
        let task = self.task(id).ok_or(Error::UnknownTask { id })?;
        let mut changes = vec![self.status_change(id, Status::Done)?];
        if task.day() != Some(today) {
            changes.push(Change::Day(Some(today)));
        }
        Ok(changes)
    }

    /// As mudanças que escalam a tarefa pra um dia: ela fica agendada. Se
    /// já estava agendada, só muda o dia.
    ///
    /// # Errors
    ///
    /// - [`Error::UnknownTask`] se a tarefa não existir.
    /// - [`Error::InvalidTransition`] se a tarefa não puder ser agendada
    ///   (uma ideia, por exemplo, precisa ficar pronta antes).
    pub fn schedule(&self, id: TaskId, day: Date) -> Result<Vec<Change>, Error> {
        let task = self.task(id).ok_or(Error::UnknownTask { id })?;
        let mut changes = Vec::new();
        if task.status() != Status::Scheduled {
            changes.push(self.status_change(id, Status::Scheduled)?);
        }
        if task.day() != Some(day) {
            changes.push(Change::Day(Some(day)));
        }
        Ok(changes)
    }

    /// As mudanças que tiram a tarefa da escalação. Se ela estava agendada,
    /// volta pro banco, pronta; com outro status, só perde o dia.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownTask`] se a tarefa não existir.
    pub fn unschedule(&self, id: TaskId) -> Result<Vec<Change>, Error> {
        let task = self.task(id).ok_or(Error::UnknownTask { id })?;
        let mut changes = Vec::new();
        if task.status() == Status::Scheduled {
            changes.push(Change::Status(Status::Ready));
        }
        if task.day().is_some() {
            changes.push(Change::Day(None));
        }
        Ok(changes)
    }

    /// A mudança que marca (ou desmarca) uma tarefa recorrente num dia.
    ///
    /// # Errors
    ///
    /// - [`Error::UnknownTask`] se a tarefa não existir.
    /// - [`Error::NotRecurring`] se ela não for recorrente.
    pub fn check(&self, id: TaskId, day: Date, done: bool) -> Result<Change, Error> {
        let task = self.task(id).ok_or(Error::UnknownTask { id })?;
        if task.status() != Status::Recurring {
            return Err(Error::NotRecurring { id });
        }
        Ok(Change::Check { day, done })
    }

    /// O ID pra próxima tarefa: um a mais que o maior ID que já apareceu em
    /// algum evento, ou `T-001` se ainda não tem nenhum. `None` só se os
    /// números acabarem, o que na prática não acontece.
    pub fn next_id(&self) -> Option<TaskId> {
        match self.tasks.last_key_value() {
            Some((last, _)) => last.next(),
            None => Some(TaskId::FIRST),
        }
    }
}

impl<'a> FromIterator<&'a Event> for Projection {
    fn from_iter<I: IntoIterator<Item = &'a Event>>(events: I) -> Self {
        let mut projection = Self::new();
        for event in events {
            projection.apply(event);
        }
        projection
    }
}

/// O que se sabe de uma tarefa até aqui. Cada campo guarda o valor e o
/// carimbo do evento que escreveu por último; `None` é campo que nenhum
/// evento preencheu ainda.
#[derive(Debug, Clone, Default)]
struct Draft {
    /// Se o evento de criação já chegou. Sem ele a tarefa não aparece, mesmo
    /// que mudanças posteriores já tenham chegado de outra máquina: elas
    /// ficam guardadas esperando.
    created: bool,
    title: Option<Latest<NotebookTitle>>,
    area: Option<Latest<Area>>,
    fit: Option<Latest<Fit>>,
    status: Option<Latest<Status>>,
    notes: Option<Latest<String>>,
    day: Option<Latest<Option<Date>>>,
    checks: BTreeMap<Date, Option<Latest<bool>>>,
}

impl Draft {
    fn to_task(&self, id: TaskId) -> Option<Task> {
        if !self.created {
            return None;
        }
        // A criação preenche esses quatro, então com `created` eles existem.
        let mut task = Task::new(
            id,
            self.title.as_ref()?.value.clone(),
            self.area.as_ref()?.value.clone(),
            self.fit.as_ref()?.value,
            self.status.as_ref()?.value,
        );
        if let Some(notes) = &self.notes {
            task.set_notes(&notes.value);
        }
        if let Some(day) = &self.day {
            task.set_day(day.value);
        }
        for (&day, check) in &self.checks {
            if let Some(check) = check {
                task.set_checked(day, check.value);
            }
        }
        Some(task)
    }
}

/// Um valor e o carimbo do evento que o escreveu.
#[derive(Debug, Clone)]
struct Latest<T> {
    value: T,
    stamp: Stamp,
}

/// A ordem entre eventos: primeiro o instante, e no empate o ID. O `derive`
/// de `Ord` compara os campos na ordem em que estão declarados.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Stamp {
    at: Timestamp,
    id: EventId,
}

/// Escreve o valor no campo se o evento for mais recente que o último que
/// escreveu ali. Com o mesmo carimbo não escreve: é o mesmo evento chegando
/// de novo.
fn write<T: Clone>(slot: &mut Option<Latest<T>>, value: &T, stamp: Stamp) {
    if slot.as_ref().is_none_or(|latest| stamp > latest.stamp) {
        *slot = Some(Latest {
            value: value.clone(),
            stamp,
        });
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use uuid::Builder;

    use super::*;
    use crate::Machine;

    /// Um evento de teste. O `n` vira o ID do evento, então IDs maiores
    /// ganham no empate; o `second` vira o instante.
    fn event(n: u64, second: i64, task: u32, change: Change) -> Event {
        let uuid = Builder::from_unix_timestamp_millis(n, &[0; 10]).into_uuid();
        Event {
            id: EventId::new(uuid).unwrap(),
            at: Timestamp::from_second(second).unwrap(),
            machine: Machine::new(if n.is_multiple_of(2) {
                "notebook"
            } else {
                "pc-da-sala"
            })
            .unwrap(),
            task: TaskId::new(task).unwrap(),
            change,
        }
    }

    fn created(title: &str) -> Change {
        Change::Created {
            title: title.parse().unwrap(),
            area: "casa".parse().unwrap(),
            fit: Fit::Gap,
            status: Status::Ready,
        }
    }

    fn title(text: &str) -> Change {
        Change::Title(text.parse().unwrap())
    }

    fn projection(events: &[Event]) -> Projection {
        events.iter().collect()
    }

    fn title_of(projection: &Projection, task: u32) -> String {
        projection
            .task(TaskId::new(task).unwrap())
            .unwrap()
            .title()
            .to_string()
    }

    #[test]
    fn a_created_task_appears_with_what_it_was_created_with() {
        let projection = projection(&[event(1, 0, 41, created("Trocar o filtro da água"))]);

        let task = projection.task(TaskId::new(41).unwrap()).unwrap();
        assert_eq!(task.title().as_str(), "Trocar o filtro da água");
        assert_eq!(task.area().as_str(), "casa");
        assert_eq!(task.fit(), Fit::Gap);
        assert_eq!(task.status(), Status::Ready);
        assert_eq!(task.notes(), "");
    }

    #[test]
    fn each_change_touches_only_its_field() {
        let projection = projection(&[
            event(1, 0, 41, created("Trocar o filtro da água")),
            event(2, 1, 41, Change::Area("cozinha".parse().unwrap())),
            event(3, 2, 41, Change::Fit(Fit::Focus)),
            event(4, 3, 41, Change::Status(Status::Done)),
            event(
                5,
                4,
                41,
                Change::Notes("O refil está na gaveta.".to_owned()),
            ),
        ]);

        let task = projection.task(TaskId::new(41).unwrap()).unwrap();
        assert_eq!(task.title().as_str(), "Trocar o filtro da água");
        assert_eq!(task.area().as_str(), "cozinha");
        assert_eq!(task.fit(), Fit::Focus);
        assert_eq!(task.status(), Status::Done);
        assert_eq!(task.notes(), "O refil está na gaveta.");
    }

    #[test]
    fn the_latest_write_wins_even_when_it_arrives_first() {
        let newer = event(2, 20, 41, title("Trocar o filtro"));
        let older = event(3, 10, 41, title("Lavar o filtro"));

        let projection = projection(&[event(1, 0, 41, created("Ver o filtro")), newer, older]);

        assert_eq!(title_of(&projection, 41), "Trocar o filtro");
    }

    #[test]
    fn two_machines_editing_different_fields_keep_both_changes() {
        let projection = projection(&[
            event(1, 0, 41, created("Trocar o filtro")),
            event(2, 10, 41, title("Trocar o filtro da água")),
            event(3, 10, 41, Change::Fit(Fit::Focus)),
        ]);

        let task = projection.task(TaskId::new(41).unwrap()).unwrap();
        assert_eq!(task.title().as_str(), "Trocar o filtro da água");
        assert_eq!(task.fit(), Fit::Focus);
    }

    #[test]
    fn a_tie_in_time_goes_to_the_larger_event_id() {
        let created = event(1, 0, 41, created("Ver o filtro"));
        let smaller_id = event(2, 10, 41, title("Lavar o filtro"));
        let larger_id = event(3, 10, 41, title("Trocar o filtro"));

        for events in [
            [created.clone(), smaller_id.clone(), larger_id.clone()],
            [created.clone(), larger_id.clone(), smaller_id.clone()],
        ] {
            assert_eq!(title_of(&projection(&events), 41), "Trocar o filtro");
        }
    }

    #[test]
    fn changes_wait_for_the_creation_to_arrive() {
        let change = event(2, 10, 41, title("Trocar o filtro"));
        let mut projection = projection(std::slice::from_ref(&change));
        assert_eq!(projection.task(TaskId::new(41).unwrap()), None);
        assert_eq!(projection.tasks().count(), 0);

        projection.apply(&event(1, 0, 41, created("Ver o filtro")));

        assert_eq!(title_of(&projection, 41), "Trocar o filtro");
    }

    #[test]
    fn a_status_change_follows_the_rule() {
        let projection = projection(&[event(1, 0, 41, created("Trocar o filtro"))]);
        let id = TaskId::new(41).unwrap();

        assert_eq!(
            projection.status_change(id, Status::Done),
            Ok(Change::Status(Status::Done))
        );
        assert_eq!(
            projection.status_change(id, Status::Idea),
            Err(Error::InvalidTransition {
                from: Status::Ready,
                to: Status::Idea,
            })
        );
    }

    #[test]
    fn a_status_change_needs_a_task_that_exists() {
        let id = TaskId::new(7).unwrap();
        assert_eq!(
            Projection::new().status_change(id, Status::Done),
            Err(Error::UnknownTask { id })
        );
    }

    #[test]
    fn tasks_come_in_id_order() {
        let projection = projection(&[
            event(1, 0, 7, created("Ler o livro")),
            event(2, 0, 3, created("Pagar a luz")),
            event(3, 0, 41, created("Trocar o filtro")),
        ]);

        let ids: Vec<String> = projection.tasks().map(|t| t.id().to_string()).collect();
        assert_eq!(ids, ["T-003", "T-007", "T-041"]);
    }

    #[test]
    fn the_next_id_comes_after_the_largest_one_seen() {
        assert_eq!(Projection::new().next_id(), TaskId::new(1));

        let projection = projection(&[
            event(1, 0, 3, created("Pagar a luz")),
            event(2, 0, 41, created("Trocar o filtro")),
            // Uma mudança de uma tarefa cuja criação ainda não chegou
            // também reserva o número.
            event(3, 0, 50, title("Ler o livro")),
        ]);
        assert_eq!(projection.next_id(), TaskId::new(51));
    }

    #[test]
    fn the_next_id_runs_out_at_the_limit() {
        let projection = projection(&[event(1, 0, u32::MAX, created("Ler o livro"))]);
        assert_eq!(projection.next_id(), None);
    }

    /// Eventos sorteados em poucas tarefas e poucos instantes, pra ter
    /// bastante conflito e bastante empate. Os IDs saem do índice, então não
    /// se repetem.
    fn events() -> impl Strategy<Value = Vec<Event>> {
        let titles = prop::sample::select(vec!["Ler o livro", "Trocar o filtro", "Pôr a mesa"])
            .prop_map(|text| text.parse::<NotebookTitle>().unwrap());
        let areas = prop::sample::select(vec!["casa", "saúde", "estudos"])
            .prop_map(|name| name.parse::<Area>().unwrap());
        let fits = prop::sample::select(Fit::ALL.to_vec());
        let statuses = prop::sample::select(Status::ALL.to_vec());
        let notes =
            prop::sample::select(vec!["", "Na gaveta.", "Ligar antes."]).prop_map(str::to_owned);

        let change = prop_oneof![
            (
                titles.clone(),
                areas.clone(),
                fits.clone(),
                statuses.clone()
            )
                .prop_map(|(title, area, fit, status)| Change::Created {
                    title,
                    area,
                    fit,
                    status
                }),
            titles.prop_map(Change::Title),
            areas.prop_map(Change::Area),
            fits.prop_map(Change::Fit),
            statuses.prop_map(Change::Status),
            notes.prop_map(Change::Notes),
            prop::option::of(10..=12_i8)
                .prop_map(|day| Change::Day(day.map(|day| jiff::civil::date(2026, 11, day)))),
            (10..=12_i8, any::<bool>()).prop_map(|(day, done)| Change::Check {
                day: jiff::civil::date(2026, 11, day),
                done
            }),
        ];

        prop::collection::vec((1..=3u32, 0..4i64, change), 0..40).prop_map(|items| {
            items
                .into_iter()
                .zip(1..)
                .map(|((task, second, change), n)| event(n, second, task, change))
                .collect()
        })
    }

    fn tasks(projection: &Projection) -> Vec<Task> {
        projection.tasks().collect()
    }

    proptest! {
        #[test]
        fn the_order_of_arrival_does_not_matter(
            (events, shuffled) in events().prop_flat_map(|events| {
                let shuffled = Just(events.clone()).prop_shuffle();
                (Just(events), shuffled)
            })
        ) {
            prop_assert_eq!(tasks(&projection(&events)), tasks(&projection(&shuffled)));
        }

        #[test]
        fn receiving_an_event_twice_changes_nothing(events in events()) {
            let once = projection(&events);
            let twice: Projection = events.iter().chain(&events).collect();
            prop_assert_eq!(tasks(&once), tasks(&twice));
        }

        #[test]
        fn merging_two_machines_gives_the_same_state_on_both(
            (events, split) in events().prop_flat_map(|events| {
                let len = events.len();
                (Just(events), 0..=len)
            })
        ) {
            // Cada máquina começa com uma parte do log e depois recebe a
            // parte da outra.
            let (first, second) = events.split_at(split);
            let mut here = projection(first);
            let mut there = projection(second);
            for event in second {
                here.apply(event);
            }
            for event in first {
                there.apply(event);
            }
            prop_assert_eq!(tasks(&here), tasks(&there));
        }
    }
}
