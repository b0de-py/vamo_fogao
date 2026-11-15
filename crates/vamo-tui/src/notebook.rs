//! A tela Caderno: o dia no padrão do caderno de papel, com as tarefas e a
//! agenda juntas, em ordem de horário.

use std::collections::BTreeSet;

use jiff::civil::Date;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use vamo_core::{Appointment, FixedActivity, Lineup, Projection, Status, Task, When};

use crate::app::Action;
use crate::prompt::{Prompt, Purpose};
use crate::schedule::Schedule;
use crate::today::{toggle, toggle_activity};
use crate::{Ledger, Voice, theme};

/// O cursor do Caderno.
#[derive(Debug, Default)]
pub(crate) struct Notebook {
    cursor: usize,
}

/// Uma linha do caderno: uma tarefa (com o horário do evento que a marca, se
/// tiver), uma fixa da agenda ou um compromisso avulso.
#[derive(Debug, Clone, Copy)]
enum Entry<'a> {
    Task(&'a Task, Option<&'a When>),
    Fixed(&'a FixedActivity),
    Event(&'a Appointment),
}

impl Entry<'_> {
    /// A ordem no papel: primeiro as tarefas sem horário, depois o que é do
    /// dia inteiro e por fim o que tem hora, pelo começo.
    fn order(self) -> (u8, Option<jiff::civil::DateTime>) {
        let when = match self {
            Self::Task(_, when) => when,
            Self::Fixed(activity) => Some(&activity.when),
            Self::Event(appointment) => Some(&appointment.when),
        };
        match when {
            None => (0, None),
            Some(When::AllDay(_)) => (1, None),
            Some(When::Timed { start, .. }) => (2, Some(*start)),
        }
    }
}

/// As linhas do dia, na ordem do papel. O evento que já é uma tarefa
/// (ligado pelo rodapé) aparece uma vez só, como a tarefa, com o horário
/// dele.
fn entries<'a>(lineup: &'a Lineup, schedule: &'a Schedule) -> Vec<Entry<'a>> {
    let linked = |task: &Task| {
        schedule
            .appointments
            .iter()
            .find(|appointment| appointment.task == Some(task.id()))
            .map(|appointment| &appointment.when)
    };
    let mut entries: Vec<Entry<'a>> = lineup
        .planned
        .iter()
        .chain(&lineup.late)
        .map(|task| Entry::Task(task, linked(task)))
        .collect();
    entries.extend(schedule.fixed.iter().map(Entry::Fixed));
    entries.extend(
        schedule
            .appointments
            .iter()
            .filter(|appointment| !appointment.recurring && appointment.task.is_none())
            .map(Entry::Event),
    );
    // `sort_by_key` é estável: no empate, a ordem em que vieram.
    entries.sort_by_key(|entry| entry.order());
    entries
}

impl Notebook {
    /// Reage a uma tecla da tela: sobe e desce, e o espaço dá o gol na
    /// tarefa ou marca a fixa.
    pub(crate) fn on_key<L: Ledger>(
        &mut self,
        key: KeyEvent,
        ledger: &mut L,
        today: Date,
        schedule: &Schedule,
    ) -> Option<Action> {
        let lineup = Lineup::new(ledger.projection(), today);
        let entries = entries(&lineup, schedule);
        let last = entries.len().saturating_sub(1);
        self.cursor = self.cursor.min(last);
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.cursor = (self.cursor + 1).min(last);
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor = self.cursor.saturating_sub(1);
                None
            }
            KeyCode::Char(' ') | KeyCode::Enter => match *entries.get(self.cursor)? {
                Entry::Task(task, _) => {
                    if crate::bench::asks_date(ledger.projection(), task, today) {
                        return Some(Action::Ask(Prompt::new(Purpose::StepDate(task.id()), "")));
                    }
                    let task = task.clone();
                    Some(Action::Feedback(toggle(ledger, &task, today)))
                }
                Entry::Fixed(activity) => {
                    let activity = activity.clone();
                    Some(Action::Feedback(toggle_activity(ledger, &activity, today)))
                }
                Entry::Event(_) => None,
            },
            _ => None,
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "cada parâmetro é uma peça da tela, e juntar numa struct só esconderia isso"
    )]
    /// Desenha a lista do dia: uma linha curta por item, verbo na frente,
    /// pronta pra passar pro papel. O que já foi feito leva o ✓ no fim.
    pub(crate) fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        projection: &Projection,
        today: Date,
        schedule: &Schedule,
        checks: &BTreeSet<(Date, String)>,
        voice: Voice,
    ) {
        let lineup = Lineup::new(projection, today);
        let entries = entries(&lineup, schedule);
        let [top, rest] =
            Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).areas(area);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(voice.notebook_title(), theme::heading()),
                Span::styled(format!(" · {}", today.strftime("%d/%m")), theme::soft()),
            ])),
            top,
        );

        let cursor = self.cursor.min(entries.len().saturating_sub(1));
        let mut lines: Vec<Line<'static>> = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let (hour, title, done) = match *entry {
                    Entry::Task(task, when) => (
                        hour_of(when),
                        crate::bench::label(task),
                        task.status() == Status::Done,
                    ),
                    Entry::Fixed(activity) => (
                        hour_of(Some(&activity.when)),
                        activity.title.clone(),
                        checks.contains(&(today, activity.uid.clone())),
                    ),
                    Entry::Event(appointment) => (
                        hour_of(Some(&appointment.when)),
                        appointment.title.clone(),
                        false,
                    ),
                };
                let line = line(&hour, &title, done);
                if index == cursor {
                    line.style(theme::cursor())
                } else {
                    line
                }
            })
            .collect();
        if lines.is_empty() {
            lines.push(Line::styled(voice.notebook_empty(), theme::muted()));
        }
        frame.render_widget(Paragraph::new(lines), rest);
    }
}

/// O horário de começo, pra ir na frente da linha: vazio pra quem não tem.
fn hour_of(when: Option<&When>) -> String {
    match when {
        Some(When::Timed { start, .. }) => start.strftime("%H:%M").to_string(),
        Some(When::AllDay(_)) | None => String::new(),
    }
}

/// Uma linha do caderno.
fn line(hour: &str, title: &str, done: bool) -> Line<'static> {
    let hour = if hour.is_empty() {
        String::new()
    } else {
        format!("{hour}  ")
    };
    if done {
        Line::from(vec![
            Span::styled(format!("  - {hour}{title} "), theme::text()),
            Span::styled("✓", theme::success()),
        ])
    } else {
        Line::styled(format!("  - {hour}{title}"), theme::text())
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use ratatui::crossterm::event::KeyCode;

    use super::*;
    use crate::app::tests::{draw, key, ledger, new_app};
    use crate::today::tests::example_day;

    #[test]
    fn the_notebook_of_the_example_day() {
        let mut ledger = example_day();
        let mut app = new_app(Voice::Game);
        app.on_key(key(KeyCode::Char('2')), &mut ledger);
        assert_snapshot!(draw(&app, &ledger, 60, 12));
    }

    #[test]
    fn an_empty_notebook_says_so() {
        let mut ledger = ledger();
        let mut app = new_app(Voice::Game);
        app.on_key(key(KeyCode::Char('2')), &mut ledger);
        let screen = draw(&app, &ledger, 90, 8);
        assert!(screen.contains(Voice::Game.notebook_empty()), "{screen}");
    }

    /// O dia do exemplo com a agenda: uma reunião fixa às 10h, um evento
    /// avulso às 14h e o evento de uma tarefa às 12h30.
    fn with_agenda(app: &mut crate::app::App, ledger: &mut crate::ledger::memory::MemoryLedger) {
        use vamo_core::{Appointment, Fit};

        let day = crate::app::tests::today();
        let errand = ledger.add("Comprar o gesso", "casa", Fit::Gap, Status::Ready);
        let changes = ledger.projection().schedule(errand, day).unwrap();
        ledger.record(errand, changes).unwrap();
        let at = |hour, minute| day.at(hour, minute, 0, 0);
        let event = |uid: &str, title: &str, hour, minute, recurring, task| Appointment {
            uid: uid.to_owned(),
            title: title.to_owned(),
            when: When::Timed {
                start: at(hour, minute),
                end: at(hour + 1, minute),
            },
            recurring,
            task,
        };
        let appointments = vec![
            event("dentista@exemplo", "Ir ao dentista", 14, 0, false, None),
            event("reuniao@exemplo", "Reunião da equipe", 10, 0, true, None),
            event(
                "gesso@exemplo",
                "Comprar o gesso",
                12,
                30,
                false,
                Some(errand),
            ),
        ];
        app.set_schedule(Schedule {
            fixed: vamo_core::fixed_activities(&appointments, &std::collections::BTreeMap::new()),
            appointments,
            ..Schedule::default()
        });
    }

    #[test]
    fn the_notebook_mirrors_the_paper_with_the_agenda_in_time_order() {
        let mut ledger = example_day();
        let mut app = new_app(Voice::Game);
        with_agenda(&mut app, &mut ledger);
        app.on_key(key(KeyCode::Char('2')), &mut ledger);
        let screen = draw(&app, &ledger, 70, 14);

        // As tarefas sem horário vêm primeiro; depois o que tem hora, em
        // ordem, e o evento que é tarefa aparece uma vez, como a tarefa.
        let position = |text: &str| {
            screen
                .find(text)
                .unwrap_or_else(|| panic!("{text}\n{screen}"))
        };
        assert!(position("Trocar o filtro da água") < position("10:00  Reunião da equipe"));
        assert!(position("10:00  Reunião da equipe") < position("12:30  Comprar o gesso"));
        assert!(position("12:30  Comprar o gesso") < position("14:00  Ir ao dentista"));
        assert_eq!(screen.matches("Comprar o gesso").count(), 1, "{screen}");
        assert_snapshot!(screen);
    }

    #[test]
    fn the_goal_and_the_tick_work_from_the_notebook() {
        let mut ledger = example_day();
        let mut app = new_app(Voice::Game);
        with_agenda(&mut app, &mut ledger);
        app.on_key(key(KeyCode::Char('2')), &mut ledger);

        // A segunda linha é a tarefa escalada ainda sem gol.
        app.on_key(key(KeyCode::Char('j')), &mut ledger);
        app.on_key(key(KeyCode::Char(' ')), &mut ledger);
        let screen = draw(&app, &ledger, 70, 14);
        assert!(screen.contains("Revisar o capítulo dois ✓"), "{screen}");

        // A fixa das 10h é a linha 4: o espaço marca.
        for _ in 0..2 {
            app.on_key(key(KeyCode::Char('j')), &mut ledger);
        }
        app.on_key(key(KeyCode::Char(' ')), &mut ledger);
        let screen = draw(&app, &ledger, 70, 14);
        assert!(screen.contains("10:00  Reunião da equipe ✓"), "{screen}");
    }
}
