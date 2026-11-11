//! A tela Banco: todas as tarefas, com filtro.

use jiff::civil::Date;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table};
use vamo_core::{Fit, Projection, Status, Task};

use crate::app::{Action, Feedback, record};
use crate::prompt::{Prompt, Purpose};
use crate::{Ledger, Voice, theme};

/// O cursor e o filtro do Banco.
#[derive(Debug, Default)]
pub(crate) struct Bench {
    cursor: usize,
    filter: String,
}

impl Bench {
    /// Troca o filtro e volta o cursor pro topo.
    pub(crate) fn set_filter(&mut self, filter: &str) {
        filter.trim().clone_into(&mut self.filter);
        self.cursor = 0;
    }

    /// As tarefas que o filtro deixa ver, em ordem de ID.
    fn visible(&self, projection: &Projection) -> Vec<Task> {
        projection
            .tasks()
            .filter(|task| matches(task, &self.filter))
            .collect()
    }

    /// Reage a uma tecla da tela.
    pub(crate) fn on_key<L: Ledger>(
        &mut self,
        key: KeyEvent,
        ledger: &mut L,
        today: Date,
    ) -> Option<Action> {
        let tasks = self.visible(ledger.projection());
        let last = tasks.len().saturating_sub(1);
        self.cursor = self.cursor.min(last);
        let selected = tasks.get(self.cursor);
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.cursor = (self.cursor + 1).min(last);
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor = self.cursor.saturating_sub(1);
                None
            }
            KeyCode::Char('n') => Some(Action::Ask(Prompt::new(Purpose::NewTask, ""))),
            KeyCode::Char('/') => Some(Action::Ask(Prompt::new(Purpose::Filter, &self.filter))),
            KeyCode::Esc if !self.filter.is_empty() => {
                self.set_filter("");
                None
            }
            KeyCode::Enter => selected.map(|task| Action::Open(task.id())),
            _ => selected.and_then(|task| task_key(key, task, ledger, today)),
        }
    }

    /// Desenha a tela.
    pub(crate) fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        projection: &Projection,
        voice: Voice,
    ) {
        let tasks = self.visible(projection);
        let [top, rest] =
            Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).areas(area);
        let filter = if self.filter.is_empty() {
            Span::styled(voice.bench_in_play(), theme::muted())
        } else {
            Span::styled(format!("filtro: {}", self.filter), theme::selected())
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(voice.bench_title(), theme::heading()),
                Span::styled(format!(" · {} · ", tasks.len()), theme::soft()),
                filter,
            ])),
            top,
        );
        if tasks.is_empty() {
            frame.render_widget(
                Paragraph::new(Line::styled(voice.bench_empty(), theme::muted())),
                rest,
            );
            return;
        }

        // A janela de linhas acompanha o cursor quando a lista não cabe.
        let height = usize::from(rest.height.saturating_sub(1)).max(1);
        let cursor = self.cursor.min(tasks.len() - 1);
        let first = cursor.saturating_sub(height - 1);
        let rows = tasks
            .iter()
            .enumerate()
            .skip(first)
            .take(height)
            .map(|(index, task)| row(task, index == cursor));
        let header =
            Row::new(["ID", "status", "encaixe", "área", "dia", "título"]).style(theme::muted());
        let widths = [
            Constraint::Length(5),
            Constraint::Length(13),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(5),
            Constraint::Fill(1),
        ];
        frame.render_widget(
            Table::new(rows, widths).header(header).column_spacing(2),
            rest,
        );
    }
}

/// As teclas que agem sobre uma tarefa. Valem no Banco e na Ficha.
pub(crate) fn task_key<L: Ledger>(
    key: KeyEvent,
    task: &Task,
    ledger: &mut L,
    today: Date,
) -> Option<Action> {
    let id = task.id();
    let projection = ledger.projection();
    let changes = match key.code {
        KeyCode::Char('e') => projection.schedule(id, today),
        KeyCode::Char('b') => projection.unschedule(id),
        KeyCode::Char('x') => projection
            .status_change(id, Status::Dropped)
            .map(|change| vec![change]),
        KeyCode::Char(' ') if task.status() == Status::Done => projection
            .status_change(id, Status::Ready)
            .map(|change| vec![change]),
        KeyCode::Char(' ') => {
            let feedback = match record(ledger, id, projection.finish(id, today)) {
                Feedback::Done => Feedback::Goal,
                other => other,
            };
            return Some(Action::Feedback(feedback));
        }
        _ => return None,
    };
    Some(Action::Feedback(record(ledger, id, changes)))
}

/// Uma linha da tabela.
fn row(task: &Task, cursor: bool) -> Row<'static> {
    let style = match task.status() {
        Status::Done => theme::success(),
        Status::Dropped => theme::muted(),
        _ => theme::base(),
    };
    let day = task
        .day()
        .map(|day| day.strftime("%d/%m").to_string())
        .unwrap_or_default();
    let row = Row::new([
        Cell::from(Span::styled(task.id().to_string(), theme::muted())),
        Cell::from(task.status().as_str()),
        Cell::from(task.fit().as_str()),
        Cell::from(task.area().to_string()),
        Cell::from(Span::styled(day, theme::soft())),
        Cell::from(task.title().to_string()),
    ])
    .style(style);
    if cursor {
        row.style(theme::cursor())
    } else {
        row
    }
}

/// Se a tarefa passa no filtro.
///
/// Cada palavra do filtro precisa bater com alguma coisa da tarefa: a área,
/// o status, o encaixe ou um pedaço do título. Sem filtro de status, só
/// aparecem as tarefas em jogo (nem concluídas, nem descartadas); `todas`
/// traz tudo.
fn matches(task: &Task, filter: &str) -> bool {
    let words: Vec<String> = filter.split_whitespace().map(str::to_lowercase).collect();
    let names_a_status = words
        .iter()
        .any(|word| word == "todas" || word.parse::<Status>().is_ok());
    if !names_a_status && matches!(task.status(), Status::Done | Status::Dropped) {
        return false;
    }
    let title = task.title().as_str().to_lowercase();
    words.iter().all(|word| {
        word == "todas"
            || task.area().as_str() == word
            || word.parse::<Status>() == Ok(task.status())
            || word.parse::<Fit>() == Ok(task.fit())
            || title.contains(word.as_str())
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use insta::assert_snapshot;
    use vamo_core::Fit;

    use super::*;
    use crate::app::tests::{draw, key, ledger, new_app, today};
    use crate::ledger::memory::MemoryLedger;

    /// Um banco inventado, com tarefas de várias áreas e status.
    pub(crate) fn bench() -> MemoryLedger {
        let mut ledger = ledger();
        ledger.add("Trocar o filtro da água", "casa", Fit::Gap, Status::Ready);
        ledger.add(
            "Revisar o capítulo dois",
            "estudos",
            Fit::Focus,
            Status::Ready,
        );
        ledger.add("Pensar na viagem", "lazer", Fit::WithSomeone, Status::Idea);
        let done = ledger.add("Consertar a torneira", "casa", Fit::Focus, Status::Ready);
        let changes = ledger.projection().finish(done, today()).unwrap();
        ledger.record(done, changes).unwrap();
        ledger.add("Lavar o tapete", "casa", Fit::Focus, Status::Refining);
        ledger
    }

    fn open_bench(ledger: &mut MemoryLedger) -> crate::app::App {
        let mut app = new_app(Voice::Game);
        app.on_key(key(KeyCode::Char('3')), ledger);
        app
    }

    fn type_text(app: &mut crate::app::App, ledger: &mut MemoryLedger, text: &str) {
        for c in text.chars() {
            app.on_key(key(KeyCode::Char(c)), ledger);
        }
        app.on_key(key(KeyCode::Enter), ledger);
    }

    #[test]
    fn the_bench_shows_the_tasks_in_play() {
        let mut ledger = bench();
        let app = open_bench(&mut ledger);
        assert_snapshot!(draw(&app, &ledger, 90, 12));
    }

    #[test]
    fn what_is_there_from_home() {
        let mut ledger = bench();
        let mut app = open_bench(&mut ledger);
        app.on_key(key(KeyCode::Char('/')), &mut ledger);
        type_text(&mut app, &mut ledger, "casa");
        let screen = draw(&app, &ledger, 90, 12);
        assert!(screen.contains("Trocar o filtro"), "{screen}");
        assert!(screen.contains("Lavar o tapete"), "{screen}");
        assert!(!screen.contains("Revisar"), "{screen}");
        assert!(!screen.contains("Consertar"), "{screen}");
    }

    #[test]
    fn filters_combine_and_todas_brings_the_finished() {
        let tasks: Vec<Task> = bench().projection().tasks().collect();
        let count = |filter| tasks.iter().filter(|task| matches(task, filter)).count();
        assert_eq!(count(""), 4);
        assert_eq!(count("casa foco"), 1);
        assert_eq!(count("casa todas"), 3);
        assert_eq!(count("concluida"), 1);
        assert_eq!(count("ideia"), 1);
        assert_eq!(count("FILTRO"), 1);
        assert_eq!(count("nada-disso"), 0);
    }

    #[test]
    fn a_new_task_is_created_from_the_bench() {
        let mut ledger = bench();
        let mut app = open_bench(&mut ledger);
        app.on_key(key(KeyCode::Char('n')), &mut ledger);
        type_text(&mut app, &mut ledger, "Comprar pilha pro controle");
        let task = ledger.projection().tasks().last().unwrap();
        assert_eq!(task.title().as_str(), "Comprar pilha pro controle");
        assert_eq!(task.area().as_str(), "geral");
        assert_eq!(task.status(), Status::Ready);
    }

    #[test]
    fn a_bad_title_is_refused_with_the_reason() {
        let mut ledger = bench();
        let mut app = open_bench(&mut ledger);
        app.on_key(key(KeyCode::Char('n')), &mut ledger);
        type_text(&mut app, &mut ledger, "filtro da água");
        let screen = draw(&app, &ledger, 120, 12);
        assert!(screen.contains("verbo"), "{screen}");
        assert_eq!(ledger.projection().tasks().count(), 5);
    }

    #[test]
    fn scheduling_and_dropping_from_the_bench() {
        let mut ledger = bench();
        let mut app = open_bench(&mut ledger);
        app.on_key(key(KeyCode::Char('e')), &mut ledger);
        let first = ledger.projection().tasks().next().unwrap();
        assert_eq!(first.status(), Status::Scheduled);
        assert_eq!(first.day(), Some(today()));

        app.on_key(key(KeyCode::Char('j')), &mut ledger);
        app.on_key(key(KeyCode::Char('x')), &mut ledger);
        let second = ledger.projection().tasks().nth(1).unwrap();
        assert_eq!(second.status(), Status::Dropped);
    }
}
