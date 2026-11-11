//! A tela Hoje: o placar, a escalação do dia e a curiosidade num canto.

use jiff::civil::Date;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use vamo_core::{Lineup, Projection, Status, Task};

use crate::app::{Action, Feedback, record};
use crate::curiosity::{self, Acervo};
use crate::{Ledger, Voice, theme};

/// Onde está o cursor na escalação.
#[derive(Debug, Default)]
pub(crate) struct Today {
    cursor: usize,
}

/// O grupo de uma linha da escalação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Group {
    /// Escalada pro dia.
    Planned,
    /// Escalada pra um dia que passou, e não feita.
    Late,
    /// Recorrente, de marcar todo dia.
    Fixed,
}

/// As linhas da escalação, na ordem da tela: o dia, o atraso e as fixas.
fn rows(lineup: &Lineup) -> Vec<(Group, &Task)> {
    let planned = lineup.planned.iter().map(|task| (Group::Planned, task));
    let late = lineup.late.iter().map(|task| (Group::Late, task));
    let fixed = lineup.fixed.iter().map(|task| (Group::Fixed, task));
    planned.chain(late).chain(fixed).collect()
}

impl Today {
    /// Reage a uma tecla da tela.
    pub(crate) fn on_key<L: Ledger>(
        &mut self,
        key: KeyEvent,
        ledger: &mut L,
        today: Date,
    ) -> Option<Action> {
        self.feedback(key, ledger, today).map(Action::Feedback)
    }

    fn feedback<L: Ledger>(
        &mut self,
        key: KeyEvent,
        ledger: &mut L,
        today: Date,
    ) -> Option<Feedback> {
        let lineup = Lineup::new(ledger.projection(), today);
        let rows = rows(&lineup);
        self.cursor = self.cursor.min(rows.len().saturating_sub(1));
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.cursor = (self.cursor + 1).min(rows.len().saturating_sub(1));
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor = self.cursor.saturating_sub(1);
                None
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                let &(group, task) = rows.get(self.cursor)?;
                Some(toggle(ledger, group, task, today))
            }
            KeyCode::Char('a') => {
                let &(group, task) = rows.get(self.cursor)?;
                (group != Group::Fixed).then(|| {
                    let changes = ledger
                        .projection()
                        .schedule(task.id(), today.tomorrow().unwrap_or(today));
                    record(ledger, task.id(), changes)
                })
            }
            KeyCode::Char('b') => {
                let &(group, task) = rows.get(self.cursor)?;
                (group != Group::Fixed).then(|| {
                    let changes = ledger.projection().unschedule(task.id());
                    record(ledger, task.id(), changes)
                })
            }
            _ => None,
        }
    }

    /// Desenha a tela.
    #[allow(
        clippy::too_many_arguments,
        reason = "cada parâmetro é uma peça da tela, e juntar numa struct só esconderia isso"
    )]
    pub(crate) fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        projection: &Projection,
        today: Date,
        acervo: &Acervo,
        banter: bool,
        voice: Voice,
    ) {
        let lineup = Lineup::new(projection, today);
        let [top, rest] =
            Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).areas(area);
        render_scoreboard(frame, top, &lineup, voice);

        // Com espaço, a curiosidade vai do lado; sem, embaixo, no que sobrar
        // da escalação. Se sobrar pouco, a escalação fica com a tela toda:
        // ela é o que importa, e a curiosidade continua no Museu.
        let lines = self.lines(&lineup, voice);
        let needed = u16::try_from(lines.len()).unwrap_or(u16::MAX);
        let spare = rest.height.saturating_sub(needed.saturating_add(1));
        let (list, corner) = if rest.width >= 90 {
            let [list, corner] = Layout::horizontal([Constraint::Fill(1), Constraint::Length(42)])
                .spacing(2)
                .areas(rest);
            let [corner] = Layout::vertical([Constraint::Max(10)]).areas(corner);
            (list, corner)
        } else if spare >= 5 {
            let [list, corner] =
                Layout::vertical([Constraint::Fill(1), Constraint::Length(spare.min(8))])
                    .areas(rest);
            (list, corner)
        } else {
            (rest, Rect::default())
        };
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), list);
        if !corner.is_empty() {
            curiosity::render(frame, corner, acervo, today, banter, voice);
        }
    }

    /// As linhas da escalação, com o cursor marcado.
    fn lines(&self, lineup: &Lineup, voice: Voice) -> Vec<Line<'static>> {
        let rows = rows(lineup);
        if rows.is_empty() {
            return vec![Line::styled(voice.empty_lineup(), theme::muted())];
        }
        let cursor = self.cursor.min(rows.len() - 1);
        let mut lines = Vec::new();
        let mut current = None;
        for (index, &(group, task)) in rows.iter().enumerate() {
            if current != Some(group) {
                if current.is_some() {
                    lines.push(Line::default());
                }
                let heading = match group {
                    Group::Planned => theme::heading(),
                    Group::Late => theme::warning(),
                    Group::Fixed => theme::soft(),
                };
                lines.push(Line::styled(voice.group(group), heading));
                current = Some(group);
            }
            lines.push(row(group, task, lineup.today(), index == cursor, voice));
        }
        lines
    }
}

/// Marca ou desmarca a linha: gol (ou VAR) na tarefa, marcação na fixa.
fn toggle<L: Ledger>(ledger: &mut L, group: Group, task: &Task, today: Date) -> Feedback {
    let projection = ledger.projection();
    let id = task.id();
    match group {
        Group::Fixed => {
            let done = !task.is_checked(today);
            let change = projection.check(id, today, done).map(|change| vec![change]);
            record(ledger, id, change)
        }
        _ if task.status() == Status::Done => {
            // O VAR anulou: a tarefa volta a ficar pronta, no mesmo dia.
            let change = projection
                .status_change(id, Status::Ready)
                .map(|change| vec![change]);
            record(ledger, id, change)
        }
        _ => match record(ledger, id, projection.finish(id, today)) {
            Feedback::Done => Feedback::Goal(id),
            other => other,
        },
    }
}

/// Uma linha: a marca, o ID, o título e o detalhe do lado.
fn row(group: Group, task: &Task, today: Date, cursor: bool, voice: Voice) -> Line<'static> {
    let done = match group {
        Group::Fixed => task.is_checked(today),
        _ => task.status() == Status::Done,
    };
    let mark = if done { "[x]" } else { "[ ]" };
    let base = match (group, done) {
        (_, true) => theme::success(),
        (Group::Late, false) => theme::warning(),
        _ => theme::text(),
    };
    let detail = match (group, task.day()) {
        (Group::Late, Some(day)) => voice.was_planned_for(&day.strftime("%d/%m").to_string()),
        _ => task.area().to_string(),
    };
    let line = Line::from(vec![
        Span::styled(format!(" {mark} "), base),
        Span::styled(format!("{}  ", task.id()), theme::muted()),
        Span::styled(task.title().to_string(), base),
        Span::styled(format!("  {detail}"), theme::muted()),
    ]);
    if cursor {
        line.style(theme::cursor())
    } else {
        line
    }
}

/// A linha de cima: o dia da semana, a data e o placar.
fn render_scoreboard(frame: &mut Frame<'_>, area: Rect, lineup: &Lineup, voice: Voice) {
    let today = lineup.today();
    let score = lineup.score();
    let date = format!(
        "{}, {}",
        voice.weekday(today.weekday()),
        today.strftime("%d/%m")
    );
    let score_style: Style = if score.planned > 0 && score.done == score.planned {
        theme::success()
    } else {
        theme::selected()
    };
    let [left, right] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(voice.today_title(), theme::heading()),
            Span::styled(format!(" · {date}"), theme::soft()),
        ])),
        left,
    );
    frame.render_widget(
        Paragraph::new(Line::styled(
            voice.score(score.done, score.planned),
            score_style,
        ))
        .right_aligned(),
        right,
    );
}

#[cfg(test)]
pub(crate) mod tests {
    use insta::assert_snapshot;
    use vamo_core::Fit;

    use super::*;
    use crate::app::tests::{draw, key, ledger, new_app, today};
    use crate::ledger::memory::MemoryLedger;

    /// Um dia de exemplo, todo inventado: duas tarefas escaladas (uma já
    /// feita), uma atrasada, uma fixa marcada e uma no banco.
    pub(crate) fn example_day() -> MemoryLedger {
        let mut ledger = ledger();
        let today = today();
        let yesterday = today.yesterday().unwrap();
        let schedule = |ledger: &mut MemoryLedger, title, area, day| {
            let id = ledger.add(title, area, Fit::Gap, Status::Ready);
            let changes = ledger.projection().schedule(id, day).unwrap();
            ledger.record(id, changes).unwrap();
            id
        };
        let filter = schedule(&mut ledger, "Trocar o filtro da água", "casa", today);
        schedule(&mut ledger, "Revisar o capítulo dois", "estudos", today);
        schedule(&mut ledger, "Pagar a conta de luz", "casa", yesterday);
        let water = ledger.add(
            "Beber dois litros d'água",
            "saúde",
            Fit::Gap,
            Status::Recurring,
        );
        ledger.add("Ler o livro novo", "lazer", Fit::Focus, Status::Ready);

        let changes = ledger.projection().finish(filter, today).unwrap();
        ledger.record(filter, changes).unwrap();
        let change = ledger.projection().check(water, today, true).unwrap();
        ledger.record(water, vec![change]).unwrap();
        ledger
    }

    #[test]
    fn the_example_day() {
        let app = new_app(Voice::Game);
        assert_snapshot!(draw(&app, &example_day(), 100, 18));
    }

    #[test]
    fn a_narrow_terminal_puts_the_curiosity_below() {
        let app = new_app(Voice::Game);
        assert_snapshot!(draw(&app, &example_day(), 60, 24));
    }

    #[test]
    fn an_empty_day_says_so() {
        let app = new_app(Voice::Game);
        let screen = draw(&app, &ledger(), 100, 12);
        assert!(screen.contains("Ninguém escalado hoje."), "{screen}");
        assert!(screen.contains("0 de 0"), "{screen}");
    }

    #[test]
    fn finishing_a_task_updates_the_score_right_away() {
        let mut app = new_app(Voice::Game);
        let mut ledger = example_day();
        assert!(draw(&app, &ledger, 100, 18).contains("Placar: 2 de 3"));

        // O cursor desce pra segunda tarefa do dia e marca o gol.
        app.on_key(key(KeyCode::Char('j')), &mut ledger);
        app.on_key(key(KeyCode::Char(' ')), &mut ledger);
        assert!(draw(&app, &ledger, 100, 18).contains("Placar: 3 de 3"));

        // O VAR anula.
        app.on_key(key(KeyCode::Char(' ')), &mut ledger);
        assert!(draw(&app, &ledger, 100, 18).contains("Placar: 2 de 3"));
    }

    #[test]
    fn the_late_task_can_be_scored_today() {
        let mut app = new_app(Voice::Game);
        let mut ledger = example_day();
        for _ in 0..2 {
            app.on_key(key(KeyCode::Down), &mut ledger);
        }
        app.on_key(key(KeyCode::Enter), &mut ledger);
        let lineup = Lineup::new(ledger.projection(), today());
        assert_eq!(lineup.late, []);
        assert_eq!(lineup.score().done, 3);
        assert_eq!(lineup.score().planned, 4);
    }

    #[test]
    fn the_fixed_task_is_unchecked_and_checked_again() {
        let mut app = new_app(Voice::Game);
        let mut ledger = example_day();
        for _ in 0..10 {
            app.on_key(key(KeyCode::Down), &mut ledger);
        }
        app.on_key(key(KeyCode::Char(' ')), &mut ledger);
        assert_eq!(Lineup::new(ledger.projection(), today()).score().done, 1);
        app.on_key(key(KeyCode::Char(' ')), &mut ledger);
        assert_eq!(Lineup::new(ledger.projection(), today()).score().done, 2);
    }

    #[test]
    fn a_task_can_be_postponed_or_sent_back_to_the_bench() {
        let mut app = new_app(Voice::Game);
        let mut ledger = example_day();
        app.on_key(key(KeyCode::Down), &mut ledger);
        app.on_key(key(KeyCode::Char('a')), &mut ledger);
        let lineup = Lineup::new(ledger.projection(), today());
        assert_eq!(lineup.planned.len(), 1);

        // O cursor fica no lugar: agora é a atrasada, que volta pro banco.
        app.on_key(key(KeyCode::Char('b')), &mut ledger);
        let lineup = Lineup::new(ledger.projection(), today());
        assert_eq!(lineup.late, []);
    }
}
