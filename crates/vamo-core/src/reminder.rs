//! Os lembretes: quando avisar de cada compromisso.
//!
//! A conta é pura. Quem roda é um timer, de minuto em minuto, que pergunta
//! "o que vence entre a última vez que eu olhei e agora?". O core responde
//! com a janela ([`ReminderWindow`]) e com os lembretes dela
//! ([`reminders`]); quem lê o relógio, guarda a última vez e manda a
//! notificação é o binário.

use jiff::SignedDuration;
use jiff::civil::DateTime;

use crate::{Appointment, Projection, Status, TaskId};

/// A antecedência padrão do aviso: dez minutos antes de começar.
pub const DEFAULT_LEAD: SignedDuration = SignedDuration::from_mins(10);

/// O máximo que a janela olha pra trás. Com a máquina dormindo por horas,
/// acordar não despeja todos os avisos da tarde de uma vez: só o que venceu
/// nos últimos minutos ainda vale.
pub const MAX_CATCH_UP: SignedDuration = SignedDuration::from_mins(15);

/// O trecho de tempo que uma rodada do timer olha: de `after` (sem incluir)
/// até `until` (incluindo).
///
/// Sem incluir o começo, o aviso que caiu bem no fim da rodada anterior não
/// sai de novo nesta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReminderWindow {
    /// Até onde a rodada anterior já olhou.
    pub after: DateTime,
    /// Agora.
    pub until: DateTime,
}

impl ReminderWindow {
    /// A janela desta rodada, a partir da última vez que o timer olhou.
    ///
    /// Sem última vez (a primeira rodada), ou com a última vez muito longe
    /// (a máquina dormiu), a janela começa [`MAX_CATCH_UP`] antes de agora.
    /// Uma última vez no futuro (o relógio voltou) também cai nesse caso.
    ///
    /// ```
    /// use jiff::civil::date;
    /// use vamo_core::ReminderWindow;
    ///
    /// let now = date(2026, 11, 12).at(14, 0, 0, 0);
    /// let window = ReminderWindow::since(Some(date(2026, 11, 12).at(13, 59, 0, 0)), now);
    /// assert_eq!(window.after, date(2026, 11, 12).at(13, 59, 0, 0));
    ///
    /// // A máquina dormiu desde a manhã: só os últimos 15 minutos contam.
    /// let window = ReminderWindow::since(Some(date(2026, 11, 12).at(8, 0, 0, 0)), now);
    /// assert_eq!(window.after, date(2026, 11, 12).at(13, 45, 0, 0));
    /// ```
    pub fn since(last: Option<DateTime>, now: DateTime) -> Self {
        let earliest = now.saturating_sub(MAX_CATCH_UP);
        let after = match last {
            Some(last) if last >= earliest && last <= now => last,
            _ => earliest,
        };
        Self { after, until: now }
    }

    /// Se o instante cai na janela.
    pub fn contains(self, at: DateTime) -> bool {
        self.after < at && at <= self.until
    }
}

/// Um aviso que venceu: o que é e quando começa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reminder {
    /// Quando o compromisso começa.
    pub start: DateTime,
    /// O que avisar: o título da tarefa ligada ao evento ou, sem tarefa, o
    /// título do evento.
    pub title: String,
    /// A tarefa ligada pelo rodapé `vamo:`, se tiver.
    pub task: Option<TaskId>,
}

/// Os lembretes que vencem na janela: um por compromisso com horário cujo
/// aviso (o começo menos a antecedência `lead`) cai nela, em ordem de
/// começo.
///
/// Compromisso de dia inteiro não tem hora pra avisar e fica de fora. Um
/// evento ligado a uma tarefa já concluída ou descartada também: o gol já
/// saiu, não tem do que lembrar.
///
/// ```
/// use jiff::civil::date;
/// use vamo_core::{Appointment, DEFAULT_LEAD, Projection, ReminderWindow, When, reminders};
///
/// let day = date(2026, 11, 12);
/// let dentist = Appointment {
///     uid: "dentista@exemplo".to_owned(),
///     title: "Dentista".to_owned(),
///     when: When::Timed { start: day.at(15, 0, 0, 0), end: day.at(16, 0, 0, 0) },
///     recurring: false,
///     task: None,
/// };
/// // O aviso é às 14:50; a rodada das 14:50 avisa, a das 14:51 não.
/// let window = ReminderWindow::since(Some(day.at(14, 49, 0, 0)), day.at(14, 50, 0, 0));
/// let due = reminders(&[dentist.clone()], &Projection::new(), DEFAULT_LEAD, window);
/// assert_eq!(due[0].title, "Dentista");
///
/// let window = ReminderWindow::since(Some(day.at(14, 50, 0, 0)), day.at(14, 51, 0, 0));
/// assert!(reminders(&[dentist], &Projection::new(), DEFAULT_LEAD, window).is_empty());
/// ```
pub fn reminders(
    appointments: &[Appointment],
    projection: &Projection,
    lead: SignedDuration,
    window: ReminderWindow,
) -> Vec<Reminder> {
    let mut due: Vec<Reminder> = appointments
        .iter()
        .filter_map(|appointment| {
            let (start, _) = appointment.span()?;
            if !window.contains(start.saturating_sub(lead)) {
                return None;
            }
            let title = match appointment.task.and_then(|id| projection.task(id)) {
                Some(task) if matches!(task.status(), Status::Done | Status::Dropped) => {
                    return None;
                }
                Some(task) => format!("{} {}", task.id(), task.title()),
                None => appointment.title.clone(),
            };
            Some(Reminder {
                start,
                title,
                task: appointment.task,
            })
        })
        .collect();
    due.sort_by_key(|reminder| reminder.start);
    due
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Date, date};
    use proptest::prelude::*;
    use uuid::Builder;

    use super::*;
    use crate::{Change, Event, EventId, Fit, Machine, When};

    fn day() -> Date {
        date(2026, 11, 12)
    }

    fn at(hour: i8, minute: i8) -> DateTime {
        day().at(hour, minute, 0, 0)
    }

    fn timed(title: &str, hour: i8, minute: i8, task: Option<TaskId>) -> Appointment {
        Appointment {
            uid: format!("{title}@exemplo"),
            title: title.to_owned(),
            when: When::Timed {
                start: at(hour, minute),
                end: at(hour + 1, minute),
            },
            recurring: false,
            task,
        }
    }

    fn window(from: DateTime, to: DateTime) -> ReminderWindow {
        ReminderWindow::since(Some(from), to)
    }

    /// Uma projeção com uma tarefa no status pedido.
    fn with_task(status: Status) -> (Projection, TaskId) {
        let id = TaskId::new(41).unwrap();
        let mut projection = Projection::new();
        for (n, change) in [
            Change::Created {
                title: "Revisar o orçamento".parse().unwrap(),
                area: "casa".parse().unwrap(),
                fit: Fit::Focus,
                status: Status::Ready,
            },
            Change::Status(status),
        ]
        .into_iter()
        .enumerate()
        {
            let n = u64::try_from(n).unwrap() + 1;
            projection.apply(&Event {
                id: EventId::new(Builder::from_unix_timestamp_millis(n, &[0; 10]).into_uuid())
                    .unwrap(),
                at: jiff::Timestamp::from_second(i64::try_from(n).unwrap()).unwrap(),
                machine: Machine::new("notebook").unwrap(),
                task: id,
                change,
            });
        }
        (projection, id)
    }

    #[test]
    fn the_first_round_looks_back_a_little() {
        let window = ReminderWindow::since(None, at(14, 0));
        assert_eq!(window.after, at(13, 45));
    }

    #[test]
    fn a_last_time_in_the_future_is_ignored() {
        let window = ReminderWindow::since(Some(at(15, 0)), at(14, 0));
        assert_eq!(window.after, at(13, 45));
    }

    #[test]
    fn all_day_appointments_have_no_reminder() {
        let birthday = Appointment {
            uid: "aniversario@exemplo".to_owned(),
            title: "Aniversário da prima".to_owned(),
            when: When::AllDay(day()),
            recurring: false,
            task: None,
        };
        let due = reminders(&[birthday], &Projection::new(), DEFAULT_LEAD, {
            ReminderWindow {
                after: at(0, 0).saturating_sub(SignedDuration::from_hours(1)),
                until: at(23, 59),
            }
        });
        assert_eq!(due, []);
    }

    #[test]
    fn a_linked_task_names_the_reminder() {
        let (projection, id) = with_task(Status::Ready);
        let due = reminders(
            &[timed("Bloco de foco", 9, 0, Some(id))],
            &projection,
            DEFAULT_LEAD,
            window(at(8, 49), at(8, 50)),
        );
        assert_eq!(due[0].title, "T-041 Revisar o orçamento");
        assert_eq!(due[0].task, Some(id));
    }

    #[test]
    fn a_finished_task_needs_no_reminder() {
        for status in [Status::Done, Status::Dropped] {
            let (projection, id) = with_task(status);
            let due = reminders(
                &[timed("Bloco de foco", 9, 0, Some(id))],
                &projection,
                DEFAULT_LEAD,
                window(at(8, 49), at(8, 50)),
            );
            assert!(due.is_empty(), "{status:?}");
        }
    }

    #[test]
    fn a_task_that_is_not_in_the_bank_keeps_the_event_title() {
        let due = reminders(
            &[timed("Bloco de foco", 9, 0, TaskId::new(7))],
            &Projection::new(),
            DEFAULT_LEAD,
            window(at(8, 49), at(8, 50)),
        );
        assert_eq!(due[0].title, "Bloco de foco");
    }

    #[test]
    fn reminders_come_in_order_of_start() {
        let due = reminders(
            &[timed("Depois", 9, 5, None), timed("Antes", 9, 0, None)],
            &Projection::new(),
            DEFAULT_LEAD,
            window(at(8, 45), at(9, 0)),
        );
        let titles: Vec<&str> = due.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(titles, ["Antes", "Depois"]);
    }

    proptest! {
        /// Rodadas seguidas, de minuto em minuto, avisam cada compromisso
        /// exatamente uma vez, não importa a hora nem a antecedência.
        #[test]
        fn back_to_back_rounds_remind_exactly_once(
            hour in 2i8..22,
            minute in 0i8..60,
            lead in 0i64..120,
        ) {
            let appointment = timed("Consulta", hour, minute, None);
            let lead = SignedDuration::from_mins(lead);
            let mut last = Some(at(0, 0));
            let mut count = 0;
            let mut now = at(0, 1);
            while now <= at(23, 59) {
                let window = ReminderWindow::since(last, now);
                count += reminders(std::slice::from_ref(&appointment), &Projection::new(), lead, window).len();
                last = Some(now);
                now = now.saturating_add(SignedDuration::from_mins(1));
            }
            prop_assert_eq!(count, 1);
        }
    }
}
