//! O card do dia de jogo, no alto da tela Hoje: o jogo, as tarefas dele e a
//! regra de só tarefa leve perto do horário.

use jiff::civil::Date;
use ratatui::text::{Line, Span};
use vamo_core::{GameDay, Projection, Side, Status};

use crate::{Voice, theme};

/// O jogo como se escreve, com o mandante na frente: `Botafogo x Bangu`.
pub(crate) fn matchup(game_day: &GameDay, team: &str) -> String {
    let opponent = &game_day.game.opponent;
    match game_day.game.side {
        Side::Home => format!("{team} x {opponent}"),
        Side::Away => format!("{opponent} x {team}"),
    }
}

/// As linhas do card, com uma linha em branco no fim pra separar da
/// escalação.
pub(crate) fn lines(
    game_day: &GameDay,
    team: &str,
    projection: &Projection,
    today: Date,
    width: usize,
    voice: Voice,
) -> Vec<Line<'static>> {
    let game = &game_day.game;
    let mut details = vec![game.kickoff.strftime("%H:%M").to_string()];
    if let Some(competition) = &game.competition {
        details.push(competition.clone());
    }
    let mut lines = vec![
        Line::styled(voice.game_day(), theme::selected()),
        Line::from(vec![
            Span::styled(matchup(game_day, team), theme::heading()),
            Span::styled(format!(" · {}", details.join(" · ")), theme::soft()),
        ]),
    ];
    for task in game_day.tasks(projection, today) {
        let done = task.status() == Status::Done;
        let base = if done {
            theme::success()
        } else {
            theme::text()
        };
        // A marca e o ID ocupam 13 colunas; o título fica com o resto.
        let title = crate::schedule::fit(&task.title().to_string(), width.saturating_sub(13));
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", if done { "[x]" } else { "[ ]" }), base),
            Span::styled(format!("{}  ", task.id()), theme::muted()),
            Span::styled(title, base),
        ]));
    }
    let (from, to) = game_day.light_only();
    lines.push(Line::styled(
        voice.light_only(
            &from.strftime("%H:%M").to_string(),
            &to.strftime("%H:%M").to_string(),
        ),
        theme::muted(),
    ));
    lines.push(Line::default());
    lines
}
