//! A súmula: o resumo de um dia que já fechou.
//!
//! A súmula só lê. Ela não conclui, não adia e não mexe em tarefa nenhuma:
//! olha o log de eventos do jeito que ele estava no começo e no fim do dia e
//! conta o que aconteceu no meio.

use std::collections::BTreeSet;

use jiff::Timestamp;
use jiff::civil::Date;

use crate::{Change, Event, Projection, Status, Task, TaskId};

/// O trecho do relógio que conta como um dia, do primeiro ao último instante.
///
/// O core não sabe de fuso, então quem chama diz onde o dia começa e onde
/// ele termina. No app, o dia começa à meia-noite e fecha às 4h do dia
/// seguinte: o que se faz de madrugada ainda é da noite anterior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayWindow {
    /// O primeiro instante do dia.
    pub start: Timestamp,
    /// O apito final: o primeiro instante que já não é do dia.
    pub end: Timestamp,
}

/// Quanto de um hábito foi registrado num trecho de dias.
///
/// Os hábitos vêm de fora (no app, do habit-tracker), pela porta
/// [`Habits`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HabitCount {
    /// O nome do hábito, como "Água".
    pub name: String,
    /// A unidade, no singular, como "copo".
    pub unit: String,
    /// Quantas vezes foi registrado.
    pub count: u32,
}

/// De onde vêm os números dos hábitos.
pub trait Habits {
    /// O erro de quem lê os hábitos.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Cada hábito conhecido e quanto foi registrado de `first` a `last`,
    /// os dois dias inclusos. Hábito sem registro vem com zero: o zero
    /// também é número de súmula.
    ///
    /// # Errors
    ///
    /// Quando o registro dos hábitos não pode ser lido.
    fn count(&self, first: Date, last: Date) -> Result<Vec<HabitCount>, Self::Error>;
}

/// O resumo de um dia: os gols, as fixas marcadas, o que voltou pro banco
/// e o que ficou pros acréscimos.
///
/// Uma tarefa conta como do dia quando estava escalada pra ele no começo do
/// dia ou foi escalada pra ele durante o dia. No apito final, cada uma cai
/// num lugar só:
///
/// - concluída no dia: gol;
/// - tirada do dia (sem dia ou mandada pra outro) sem ser concluída: voltou
///   pro banco;
/// - ainda no dia e sem ser concluída: acréscimos, a tarefa que venceu;
/// - descartada: não entra em lugar nenhum.
///
/// ```
/// use jiff::civil::date;
/// use vamo_core::{DaySummary, DayWindow};
///
/// let window = DayWindow {
///     start: "2026-11-11T03:00:00Z".parse()?,
///     end: "2026-11-12T07:00:00Z".parse()?,
/// };
/// let summary = DaySummary::close(&[], date(2026, 11, 11), window, Vec::new());
/// assert!(summary.goals.is_empty());
/// assert!(summary.is_empty());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaySummary {
    /// O dia.
    pub day: Date,
    /// Os gols: tarefas do dia concluídas.
    pub goals: Vec<Task>,
    /// As que estavam no dia e saíram dele sem gol: adiadas ou tiradas da
    /// escalação.
    pub benched: Vec<Task>,
    /// Os acréscimos: as que ficaram no dia sem gol e viraram atraso.
    pub stoppage: Vec<Task>,
    /// Os números dos hábitos do dia.
    pub habits: Vec<HabitCount>,
}

impl DaySummary {
    /// Fecha a súmula de `day` a partir do log inteiro. A ordem dos eventos
    /// não importa, e evento depois do apito final não conta: a súmula de
    /// um dia antigo sai igual hoje e daqui a um mês.
    pub fn close(events: &[Event], day: Date, window: DayWindow, habits: Vec<HabitCount>) -> Self {
        let kickoff = until(events, window.start);
        let final_whistle = until(events, window.end);

        // Quem estava escalado no começo do dia, mais quem foi escalado pro
        // dia durante ele.
        let mut planned: BTreeSet<TaskId> = kickoff
            .tasks()
            .filter(|task| task.day() == Some(day))
            .map(|task| task.id())
            .collect();
        planned.extend(
            events
                .iter()
                .filter(|event| window.start <= event.at && event.at < window.end)
                .filter(|event| event.change == Change::Day(Some(day)))
                .map(|event| event.task),
        );

        let mut summary = Self {
            day,
            goals: Vec::new(),
            benched: Vec::new(),
            stoppage: Vec::new(),
            habits,
        };
        for task in final_whistle.tasks() {
            match task.status() {
                Status::Dropped => {}
                // O `finish` passa a tarefa pro dia do gol, então gol com
                // esse dia é gol desse dia, escalada ou não.
                Status::Done => {
                    if task.day() == Some(day) {
                        summary.goals.push(task);
                    }
                }
                _ if !planned.contains(&task.id()) => {}
                _ if task.day() == Some(day) => summary.stoppage.push(task),
                _ => summary.benched.push(task),
            }
        }
        summary
    }

    /// Se o dia não teve nada: nem gol, nem fixa, nem tarefa adiada ou
    /// vencida. Os hábitos não contam.
    pub fn is_empty(&self) -> bool {
        self.goals.is_empty() && self.benched.is_empty() && self.stoppage.is_empty()
    }
}

/// O estado das tarefas no instante `end`, contando só os eventos de antes
/// dele.
fn until(events: &[Event], end: Timestamp) -> Projection {
    events.iter().filter(|event| event.at < end).collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use jiff::civil::date;
    use jiff::{SignedDuration, Timestamp};
    use uuid::Builder;

    use super::*;
    use crate::{EventId, Fit, Machine};

    /// Um log de exemplo montado aos poucos, com o relógio andando na mão.
    /// O dia de exemplo vai da meia-noite às 4h do dia seguinte, em UTC.
    pub(crate) struct Log {
        pub(crate) events: Vec<Event>,
        pub(crate) projection: Projection,
        pub(crate) now: Timestamp,
        n: u64,
    }

    impl Log {
        pub(crate) fn new() -> Self {
            Self {
                events: Vec::new(),
                projection: Projection::new(),
                now: midnight(date(2026, 11, 1)),
                n: 0,
            }
        }

        /// Leva o relógio pra hora dada do dia dado.
        pub(crate) fn at(&mut self, day: Date, hour: i8) {
            self.now = midnight(day) + SignedDuration::from_hours(i64::from(hour));
        }

        pub(crate) fn record(&mut self, id: TaskId, changes: Vec<Change>) {
            for change in changes {
                self.n += 1;
                let event = Event {
                    id: EventId::new(
                        Builder::from_unix_timestamp_millis(self.n, &[0; 10]).into_uuid(),
                    )
                    .unwrap(),
                    at: self.now,
                    machine: Machine::new("notebook").unwrap(),
                    task: id,
                    change,
                };
                self.projection.apply(&event);
                self.events.push(event);
            }
        }

        pub(crate) fn add(&mut self, title: &str, status: Status) -> TaskId {
            let id = self.projection.next_id().unwrap();
            self.record(
                id,
                vec![Change::Created {
                    title: title.parse().unwrap(),
                    area: "casa".parse().unwrap(),
                    fit: Fit::Gap,
                    status,
                }],
            );
            id
        }

        pub(crate) fn schedule(&mut self, id: TaskId, day: Date) {
            let changes = self.projection.schedule(id, day).unwrap();
            self.record(id, changes);
        }

        pub(crate) fn finish(&mut self, id: TaskId, day: Date) {
            let changes = self.projection.finish(id, day).unwrap();
            self.record(id, changes);
        }

        pub(crate) fn unschedule(&mut self, id: TaskId) {
            let changes = self.projection.unschedule(id).unwrap();
            self.record(id, changes);
        }
    }

    fn midnight(day: Date) -> Timestamp {
        format!("{day}T00:00:00Z").parse().unwrap()
    }

    /// O dia de exemplo: da meia-noite às 4h do dia seguinte, em UTC.
    pub(crate) fn window(day: Date) -> DayWindow {
        DayWindow {
            start: midnight(day),
            end: midnight(day.tomorrow().unwrap()) + SignedDuration::from_hours(4),
        }
    }

    pub(crate) fn titles(tasks: &[Task]) -> Vec<&str> {
        tasks.iter().map(|task| task.title().as_str()).collect()
    }

    const DAY: Date = date(2026, 11, 11);

    #[test]
    fn each_task_of_the_day_lands_in_one_place() {
        let mut log = Log::new();
        let goal = log.add("Trocar o filtro", Status::Ready);
        let moved = log.add("Pagar a conta", Status::Ready);
        let late = log.add("Lavar o carro", Status::Ready);
        let dropped = log.add("Pintar a parede", Status::Ready);
        log.add("Ler o livro", Status::Ready);
        for id in [goal, moved, late, dropped] {
            log.schedule(id, DAY);
        }

        log.at(DAY, 10);
        log.finish(goal, DAY);
        log.schedule(moved, date(2026, 11, 13));
        log.record(dropped, vec![Change::Status(Status::Dropped)]);

        let summary = DaySummary::close(&log.events, DAY, window(DAY), Vec::new());
        assert_eq!(titles(&summary.goals), ["Trocar o filtro"]);
        assert_eq!(titles(&summary.benched), ["Pagar a conta"]);
        assert_eq!(titles(&summary.stoppage), ["Lavar o carro"]);
        assert_eq!(summary.stoppage[0].id(), late);
    }

    #[test]
    fn a_task_scheduled_during_the_day_counts_too() {
        let mut log = Log::new();
        let first = log.add("Trocar o filtro", Status::Ready);
        let second = log.add("Pagar a conta", Status::Ready);

        log.at(DAY, 9);
        log.schedule(first, DAY);
        log.schedule(second, DAY);
        log.at(DAY, 20);
        log.unschedule(second);

        let summary = DaySummary::close(&log.events, DAY, window(DAY), Vec::new());
        assert_eq!(titles(&summary.stoppage), ["Trocar o filtro"]);
        assert_eq!(titles(&summary.benched), ["Pagar a conta"]);
    }

    #[test]
    fn the_small_hours_still_belong_to_the_day() {
        let mut log = Log::new();
        let late = log.add("Pagar a conta", Status::Ready);
        log.schedule(late, DAY);

        // Às 2h do dia seguinte ainda dá pra mandar a tarefa pra outro dia.
        log.at(DAY.tomorrow().unwrap(), 2);
        log.schedule(late, date(2026, 11, 14));

        let summary = DaySummary::close(&log.events, DAY, window(DAY), Vec::new());
        assert_eq!(summary.stoppage, []);
        assert_eq!(titles(&summary.benched), ["Pagar a conta"]);
    }

    #[test]
    fn what_happens_after_the_final_whistle_does_not_change_the_summary() {
        let mut log = Log::new();
        let late = log.add("Pagar a conta", Status::Ready);
        log.schedule(late, DAY);
        let before = DaySummary::close(&log.events, DAY, window(DAY), Vec::new());

        log.at(date(2026, 11, 20), 9);
        log.finish(late, date(2026, 11, 20));

        let after = DaySummary::close(&log.events, DAY, window(DAY), Vec::new());
        assert_eq!(before, after);
        assert_eq!(titles(&after.stoppage), ["Pagar a conta"]);
    }

    #[test]
    fn the_order_of_the_events_does_not_matter() {
        let mut log = Log::new();
        let goal = log.add("Trocar o filtro", Status::Ready);
        let late = log.add("Lavar o carro", Status::Ready);
        log.schedule(goal, DAY);
        log.schedule(late, DAY);
        log.at(DAY, 10);
        log.finish(goal, DAY);

        let mut reversed = log.events.clone();
        reversed.reverse();
        assert_eq!(
            DaySummary::close(&log.events, DAY, window(DAY), Vec::new()),
            DaySummary::close(&reversed, DAY, window(DAY), Vec::new())
        );
    }

    #[test]
    fn a_goal_from_another_day_is_not_a_goal_of_this_one() {
        let mut log = Log::new();
        let task = log.add("Trocar o filtro", Status::Ready);
        log.schedule(task, DAY);
        log.at(date(2026, 11, 10), 10);
        log.finish(task, date(2026, 11, 10));

        let summary = DaySummary::close(&log.events, DAY, window(DAY), Vec::new());
        assert!(summary.is_empty());
    }
}
