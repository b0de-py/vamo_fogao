//! A Ficha: os detalhes de uma tarefa, com edição e a súmula.

use jiff::civil::Date;
use jiff::tz::TimeZone;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use vamo_core::{Change, Event, Fit, Status, Task};

use crate::app::{Action, record};
use crate::bench::task_key;
use crate::prompt::{Prompt, Purpose};
use crate::{Ledger, Voice, theme};

/// Reage a uma tecla na Ficha da tarefa.
pub(crate) fn on_key<L: Ledger>(
    key: KeyEvent,
    task: &Task,
    ledger: &mut L,
    today: Date,
) -> Option<Action> {
    let id = task.id();
    let ask = |purpose, text: &str| Some(Action::Ask(Prompt::new(purpose, text)));
    match key.code {
        KeyCode::Esc => Some(Action::Close),
        KeyCode::Char('t') => ask(Purpose::Title(id), task.title().as_str()),
        KeyCode::Char('a') => ask(Purpose::Area(id), task.area().as_str()),
        KeyCode::Char('n') => ask(Purpose::Notes(id), task.notes()),
        KeyCode::Char('d') => ask(Purpose::Day(id), ""),
        KeyCode::Char('s') => {
            let options: Vec<Status> = Status::ALL
                .into_iter()
                .filter(|&next| task.status().can_become(next))
                .collect();
            ask(Purpose::Status(id, options), "")
        }
        KeyCode::Char('f') => {
            // O encaixe gira entre os três.
            let position = Fit::ALL
                .iter()
                .position(|&fit| fit == task.fit())
                .unwrap_or(0);
            let next = Fit::ALL[(position + 1) % Fit::ALL.len()];
            Some(Action::Feedback(record(
                ledger,
                id,
                Ok(vec![Change::Fit(next)]),
            )))
        }
        _ => task_key(key, task, ledger, today),
    }
}

/// Desenha a Ficha.
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    task: &Task,
    events: &[Event],
    zone: &TimeZone,
    voice: Voice,
) {
    let field = |name: &str, value: String| {
        Line::from(vec![
            Span::styled(format!("{name:<9}"), theme::muted()),
            Span::styled(value, theme::base()),
        ])
    };
    let mut lines = vec![
        Line::from(vec![
            Span::styled(format!("{} ", task.id()), theme::muted()),
            Span::styled(task.title().to_string(), theme::heading()),
        ]),
        Line::default(),
        field("área", task.area().to_string()),
        field("encaixe", task.fit().to_string()),
        field("status", task.status().to_string()),
        field(
            "dia",
            task.day().map_or_else(
                || "-".to_owned(),
                |day| day.strftime("%d/%m/%Y").to_string(),
            ),
        ),
    ];
    let mut notes = task.notes().lines();
    if let Some(first) = notes.next() {
        lines.push(field("notas", first.to_owned()));
        lines.extend(notes.map(|line| field("", line.to_owned())));
    }

    lines.push(Line::default());
    lines.push(Line::styled(voice.history(), theme::heading()));
    let mut history: Vec<&Event> = events
        .iter()
        .filter(|event| event.task == task.id())
        .collect();
    history.sort_by_key(|event| (event.at, event.id));
    for event in history {
        let when = event
            .at
            .to_zoned(zone.clone())
            .strftime("%d/%m %H:%M")
            .to_string();
        lines.push(Line::from(vec![
            Span::styled(format!("{when}  "), theme::muted()),
            Span::styled(format!("{}  ", event.machine), theme::muted()),
            Span::styled(voice.change(&event.change), theme::soft()),
        ]));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::*;
    use crate::app::tests::{draw, key, new_app, today};
    use crate::bench::tests::bench;
    use crate::ledger::memory::MemoryLedger;

    fn open_first_card(ledger: &mut MemoryLedger) -> crate::app::App {
        let mut app = new_app(Voice::Game);
        app.on_key(key(KeyCode::Char('3')), ledger);
        app.on_key(key(KeyCode::Enter), ledger);
        app
    }

    fn type_text(app: &mut crate::app::App, ledger: &mut MemoryLedger, text: &str) {
        for c in text.chars() {
            app.on_key(key(KeyCode::Char(c)), ledger);
        }
        app.on_key(key(KeyCode::Enter), ledger);
    }

    fn first(ledger: &MemoryLedger) -> Task {
        ledger.projection().tasks().next().unwrap()
    }

    #[test]
    fn the_card_shows_the_task_and_its_history() {
        let mut ledger = bench();
        let mut app = open_first_card(&mut ledger);
        app.on_key(key(KeyCode::Char('n')), &mut ledger);
        type_text(&mut app, &mut ledger, "O refil está na gaveta.");
        assert_snapshot!(draw(&app, &ledger, 80, 18));
    }

    #[test]
    fn every_field_can_be_edited() {
        let mut ledger = bench();
        let mut app = open_first_card(&mut ledger);

        app.on_key(key(KeyCode::Char('t')), &mut ledger);
        // A linha abre com o título atual: apaga tudo e escreve outro.
        for _ in 0.."Trocar o filtro da água".chars().count() {
            app.on_key(key(KeyCode::Backspace), &mut ledger);
        }
        type_text(&mut app, &mut ledger, "Trocar o refil");
        assert_eq!(first(&ledger).title().as_str(), "Trocar o refil");

        app.on_key(key(KeyCode::Char('a')), &mut ledger);
        for _ in 0..4 {
            app.on_key(key(KeyCode::Backspace), &mut ledger);
        }
        type_text(&mut app, &mut ledger, "cozinha");
        assert_eq!(first(&ledger).area().as_str(), "cozinha");

        app.on_key(key(KeyCode::Char('f')), &mut ledger);
        assert_eq!(first(&ledger).fit(), Fit::WithSomeone);

        app.on_key(key(KeyCode::Char('d')), &mut ledger);
        type_text(&mut app, &mut ledger, "amanhã");
        assert_eq!(first(&ledger).day(), today().tomorrow().ok());
        assert_eq!(first(&ledger).status(), Status::Scheduled);

        // De agendada, a regra deixa: pronta, concluída, descartada.
        app.on_key(key(KeyCode::Char('s')), &mut ledger);
        type_text(&mut app, &mut ledger, "3");
        assert_eq!(first(&ledger).status(), Status::Dropped);
    }

    #[test]
    fn a_bad_area_is_refused_and_nothing_changes() {
        let mut ledger = bench();
        let mut app = open_first_card(&mut ledger);
        app.on_key(key(KeyCode::Char('a')), &mut ledger);
        type_text(&mut app, &mut ledger, " Com Espaço");
        assert_eq!(first(&ledger).area().as_str(), "casa");
        let screen = draw(&app, &ledger, 120, 18);
        assert!(screen.contains("não serve como área"), "{screen}");
    }

    #[test]
    fn esc_goes_back_to_the_bench() {
        let mut ledger = bench();
        let mut app = open_first_card(&mut ledger);
        app.on_key(key(KeyCode::Esc), &mut ledger);
        let screen = draw(&app, &ledger, 90, 12);
        assert!(screen.contains("Revisar o capítulo dois"), "{screen}");
    }
}
