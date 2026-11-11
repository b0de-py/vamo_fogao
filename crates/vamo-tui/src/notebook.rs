//! A tela Caderno: a lista do dia no padrão do caderno de papel.

use jiff::civil::Date;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use vamo_core::{Lineup, Projection, Status};

use crate::{Voice, theme};

/// Desenha a lista do dia: uma linha curta por tarefa, verbo na frente,
/// pronta pra passar pro papel. O que já foi feito leva o ✓ no fim.
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    projection: &Projection,
    today: Date,
    voice: Voice,
) {
    let lineup = Lineup::new(projection, today);
    let [top, rest] = Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).areas(area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(voice.notebook_title(), theme::heading()),
            Span::styled(format!(" · {}", today.strftime("%d/%m")), theme::soft()),
        ])),
        top,
    );

    let tasks = lineup.planned.iter().chain(&lineup.late);
    let mut lines: Vec<Line<'static>> = tasks
        .map(|task| {
            let done = task.status() == Status::Done;
            line(task.title().as_str(), done)
        })
        .collect();
    if !lineup.fixed.is_empty() {
        lines.push(Line::default());
        lines.extend(
            lineup
                .fixed
                .iter()
                .map(|task| line(task.title().as_str(), task.is_checked(today))),
        );
    }
    if lines.is_empty() {
        lines.push(Line::styled(voice.notebook_empty(), theme::muted()));
    }
    frame.render_widget(Paragraph::new(lines), rest);
}

/// Uma linha do caderno.
fn line(title: &str, done: bool) -> Line<'static> {
    if done {
        Line::from(vec![
            Span::styled(format!("  - {title} "), theme::base()),
            Span::styled("✓", theme::success()),
        ])
    } else {
        Line::styled(format!("  - {title}"), theme::base())
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
}
