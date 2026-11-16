//! A tela Hoje: o placar, a escalação do dia e a curiosidade num canto.

use std::collections::BTreeSet;

use jiff::civil::Date;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use vamo_core::{FixedActivity, Lineup, Projection, Score, Status, Task, Unbeaten};

use crate::app::{Action, Feedback, record};
use crate::curiosity::{self, Acervo};
use crate::prompt::{Prompt, Purpose};
use crate::schedule::{self, Schedule};
use crate::visual::Look;
use crate::{Density, Ledger, Voice, bigtext, meter, theme};

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
    /// A fixa: o compromisso que se repete na agenda, de marcar sem virar
    /// tarefa. É o único tipo de fixa que a tela mostra.
    Fixed,
}

/// Uma linha da escalação: uma tarefa ou uma atividade fixa da agenda.
#[derive(Debug, Clone, Copy)]
enum Row<'a> {
    Task(Group, &'a Task),
    Activity(&'a FixedActivity),
}

impl Row<'_> {
    fn group(self) -> Group {
        match self {
            Self::Task(group, _) => group,
            Self::Activity(_) => Group::Fixed,
        }
    }
}

/// As linhas da escalação, na ordem da tela: o dia, o atraso e as fixas da
/// agenda.
fn rows<'a>(lineup: &'a Lineup, schedule: &'a Schedule) -> Vec<Row<'a>> {
    let planned = lineup
        .planned
        .iter()
        .map(|task| Row::Task(Group::Planned, task));
    let late = lineup.late.iter().map(|task| Row::Task(Group::Late, task));
    let agenda = schedule.fixed.iter().map(Row::Activity);
    planned.chain(late).chain(agenda).collect()
}

/// O placar do dia, com as atividades fixas da agenda marcadas.
pub(crate) fn score(
    lineup: &Lineup,
    schedule: &Schedule,
    checks: &BTreeSet<(Date, String)>,
) -> Score {
    lineup.score_with(&schedule.fixed, checks)
}

impl Today {
    /// Reage a uma tecla da tela.
    pub(crate) fn on_key<L: Ledger>(
        &mut self,
        key: KeyEvent,
        ledger: &mut L,
        today: Date,
        schedule: &Schedule,
    ) -> Option<Action> {
        // Escalar uma pronta do banco, sem sair do Hoje.
        if key.code == KeyCode::Char('e') {
            return Some(Action::Ask(Prompt::new(Purpose::Pull, "")));
        }
        // Fechar o passo de uma jogada que pede a data: pergunta antes.
        if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
            let lineup = Lineup::new(ledger.projection(), today);
            let rows = rows(&lineup, schedule);
            let cursor = self.cursor.min(rows.len().saturating_sub(1));
            if let Some(Row::Task(_, task)) = rows.get(cursor)
                && crate::bench::asks_date(ledger.projection(), task, today)
            {
                return Some(Action::Ask(Prompt::new(Purpose::StepDate(task.id()), "")));
            }
        }
        self.feedback(key, ledger, today, schedule)
            .map(Action::Feedback)
    }

    fn feedback<L: Ledger>(
        &mut self,
        key: KeyEvent,
        ledger: &mut L,
        today: Date,
        schedule: &Schedule,
    ) -> Option<Feedback> {
        let lineup = Lineup::new(ledger.projection(), today);
        let rows = rows(&lineup, schedule);
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
            KeyCode::Char(' ') | KeyCode::Enter => match *rows.get(self.cursor)? {
                Row::Task(_, task) => Some(toggle(ledger, task, today)),
                Row::Activity(activity) => Some(toggle_activity(ledger, activity, today)),
            },
            KeyCode::Char('a') => {
                let Row::Task(_, task) = *rows.get(self.cursor)? else {
                    return None;
                };
                let changes = ledger
                    .projection()
                    .schedule(task.id(), today.tomorrow().unwrap_or(today));
                Some(record(ledger, task.id(), changes))
            }
            KeyCode::Char('b') => {
                let Row::Task(_, task) = *rows.get(self.cursor)? else {
                    return None;
                };
                let changes = ledger.projection().unschedule(task.id());
                Some(record(ledger, task.id(), changes))
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
        schedule: &Schedule,
        checks: &BTreeSet<(Date, String)>,
        banter: bool,
        look: Look,
        voice: Voice,
    ) {
        let density = look.density;
        let lineup = Lineup::new(projection, today);
        let [top, rest] =
            Layout::vertical([Constraint::Length(top_height(area)), Constraint::Fill(1)])
                .areas(area);
        let score = score(&lineup, schedule, checks);
        render_scoreboard(frame, top, &lineup, score, schedule.unbeaten, voice);

        // Com espaço, a curiosidade vai do lado; sem, embaixo, no que sobrar
        // da escalação. Se sobrar pouco, a escalação fica com a tela toda:
        // ela é o que importa, e a curiosidade continua no Museu.
        // A largura da coluna da escalação, pra cortar só o que não cabe:
        // do lado, a curiosidade leva 42 colunas mais 2 de espaço.
        let width = usize::from(if rest.width >= 90 {
            rest.width.saturating_sub(44)
        } else {
            rest.width
        });
        let mut lines = match &schedule.game {
            Some(game_day) => {
                crate::matchday::lines(game_day, &schedule.team, projection, today, width, voice)
            }
            None => Vec::new(),
        };
        lines.extend(self.lines(&lineup, schedule, checks, width, density, voice));
        lines.extend(schedule.lines(projection, today, width, voice));
        let ready = projection.inbox().len();
        if ready > 0 {
            lines.push(Line::default());
            lines.push(Line::styled(voice.pull_hint(ready), theme::muted()));
        }
        let needed = u16::try_from(lines.len()).unwrap_or(u16::MAX);
        let spare = rest.height.saturating_sub(needed.saturating_add(1));
        let (list, corner) = if rest.width >= 90 {
            let [list, corner] = Layout::horizontal([Constraint::Fill(1), Constraint::Length(42)])
                .spacing(2)
                .areas(rest);
            // Na confortável, a caixa cresce o que a margem de dentro come.
            let height = match density {
                Density::Compact => 10,
                Density::Comfortable => 12,
            };
            let [corner] = Layout::vertical([Constraint::Max(height)]).areas(corner);
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
            let opponent = schedule.game.as_ref().map(|g| g.game.opponent.as_str());
            curiosity::render(frame, corner, acervo, today, opponent, banter, look, voice);
        }
    }

    /// As linhas da escalação, com o cursor marcado.
    fn lines(
        &self,
        lineup: &Lineup,
        schedule: &Schedule,
        checks: &BTreeSet<(Date, String)>,
        width: usize,
        density: Density,
        voice: Voice,
    ) -> Vec<Line<'static>> {
        let rows = rows(lineup, schedule);
        if rows.is_empty() {
            return vec![Line::styled(voice.empty_lineup(), theme::muted())];
        }
        let cursor = self.cursor.min(rows.len() - 1);
        let mut lines = Vec::new();
        let mut current = None;
        for (index, &row) in rows.iter().enumerate() {
            let group = row.group();
            if current != Some(group) {
                if current.is_some() {
                    lines.push(Line::default());
                }
                let heading = match group {
                    Group::Planned => theme::section(),
                    Group::Late => theme::warning(),
                    Group::Fixed => theme::soft(),
                };
                lines.push(Line::styled(
                    theme::spaced_caps(voice.group(group)),
                    heading,
                ));
                // Na confortável, o rótulo ganha uma linha de respiro.
                if density == Density::Comfortable {
                    lines.push(Line::default());
                }
                current = Some(group);
            }
            let cursor = index == cursor;
            lines.push(match row {
                Row::Task(group, task) => task_line(group, task, cursor, voice),
                Row::Activity(activity) => {
                    let done = checks.contains(&(lineup.today(), activity.uid.clone()));
                    activity_line(activity, lineup.today(), done, cursor, width)
                }
            });
        }
        lines
    }
}

/// Dá o gol na tarefa, ou anula o gol (VAR) se ela já estava concluída.
pub(crate) fn toggle<L: Ledger>(ledger: &mut L, task: &Task, today: Date) -> Feedback {
    let projection = ledger.projection();
    let id = task.id();
    match task.status() {
        Status::Done => {
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

/// Marca ou desmarca a atividade fixa da agenda no dia. Não é gol: a
/// atividade conta no placar, mas não vira tarefa.
pub(crate) fn toggle_activity<L: Ledger>(
    ledger: &mut L,
    activity: &FixedActivity,
    today: Date,
) -> Feedback {
    let done = !ledger
        .agenda_checks()
        .contains(&(today, activity.uid.clone()));
    match ledger.check_agenda(today, &activity.uid, done) {
        Ok(()) => Feedback::Done,
        Err(error) => Feedback::Refused(error.to_string()),
    }
}

/// Uma atividade fixa: a marca, o horário e o título, cortado só se não
/// couber na largura.
fn activity_line(
    activity: &FixedActivity,
    today: Date,
    done: bool,
    cursor: bool,
    width: usize,
) -> Line<'static> {
    let mark = if done { "[x]" } else { "[ ]" };
    let base = if done {
        theme::success()
    } else {
        theme::text()
    };
    let line = Line::from(vec![
        Span::styled(format!(" {mark} "), base),
        Span::styled(
            format!("{:<11}  ", schedule::when(activity.when, today)),
            theme::muted(),
        ),
        // A marca e o horário ocupam 18 colunas; o título fica com o resto.
        Span::styled(
            schedule::fit(&activity.title, width.saturating_sub(18)),
            base,
        ),
    ]);
    if cursor {
        line.style(theme::cursor())
    } else {
        line
    }
}

/// Uma linha: a marca, o ID, o título e o detalhe do lado.
fn task_line(group: Group, task: &Task, cursor: bool, voice: Voice) -> Line<'static> {
    let done = task.status() == Status::Done;
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
        Span::styled(crate::bench::label(task), base),
        Span::styled(format!("  {detail}"), theme::muted()),
    ]);
    if cursor {
        line.style(theme::cursor())
    } else {
        line
    }
}

/// A menor tela, em `(colunas, linhas)`, em que o placar eletrônico cabe
/// sem tirar o lugar da escalação. Abaixo disso, o placar é uma linha de
/// texto.
const BIG_SCORE_MIN: (u16, u16) = (70, 16);

/// A largura das barras de progresso do alto da tela, em blocos.
const METER_WIDTH: usize = 12;

/// Quantas linhas o alto da tela ocupa: o placar eletrônico e uma de
/// respiro, quando ele cabe; senão, a linha de texto e uma de respiro.
fn top_height(area: Rect) -> u16 {
    if area.width >= BIG_SCORE_MIN.0 && area.height >= BIG_SCORE_MIN.1 {
        bigtext::HEIGHT + 1
    } else {
        2
    }
}

/// O alto da tela: à esquerda, o dia da semana, a data e a invencibilidade;
/// à direita, o placar do dia.
///
/// Com espaço, o placar é o eletrônico: os números em letra de bloco, com a
/// Estrela Solitária do lado, igual ao da barra (`⭐ 3 x 11`: três feitas,
/// onze por fazer). Numa tela pequena, volta a ser uma linha de texto.
fn render_scoreboard(
    frame: &mut Frame<'_>,
    area: Rect,
    lineup: &Lineup,
    score: Score,
    unbeaten: Option<Unbeaten>,
    voice: Voice,
) {
    let today = lineup.today();
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
    let mut left = vec![Line::from(vec![
        Span::styled(voice.today_title(), theme::heading()),
        Span::styled(format!(" · {date}"), theme::soft()),
    ])];
    let unbeaten = unbeaten.filter(|u| u.record > 0);
    let unbeaten_text =
        unbeaten.map(|u| Line::styled(voice.unbeaten(u.current, u.record), theme::soft()));

    let big = voice.big_score(score.done, score.planned);
    if area.height > bigtext::HEIGHT && bigtext::can_write(&big) {
        // Embaixo da data, as duas barras: o placar do dia e a
        // invencibilidade contra o recorde.
        let meter = |done, total, style, text: String| {
            let [full, empty] = meter::spans(done, total, METER_WIDTH, style);
            Line::from(vec![
                full,
                empty,
                Span::styled(format!(" {text}"), theme::soft()),
            ])
        };
        left.push(meter(
            score.done,
            score.planned,
            score_style,
            voice.score(score.done, score.planned),
        ));
        if let Some(unbeaten) = unbeaten {
            // Dias de invencibilidade cabem com folga num usize.
            let days = |n: u32| usize::try_from(n).unwrap_or(usize::MAX);
            left.push(meter(
                days(unbeaten.current),
                days(unbeaten.record),
                theme::heading(),
                voice.unbeaten(unbeaten.current, unbeaten.record),
            ));
        }
        // A estrela e um espaço, antes dos números. Só na linguagem de jogo.
        let star = voice.score_star();
        let star_width = Span::raw(star).width();
        let width = u16::try_from(bigtext::width(&big) + star_width).unwrap_or(u16::MAX);
        let [left_area, right_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(width)])
                .spacing(2)
                .areas(area);
        frame.render_widget(Paragraph::new(left), left_area);
        let lines: Vec<Line<'_>> = (0..)
            .zip(bigtext::rows(&big))
            .map(|(row, digits)| {
                // A estrela fica na linha do meio, na altura do "x".
                let lead = if row == 1 {
                    Span::styled(star, theme::heading())
                } else {
                    Span::raw(" ".repeat(star_width))
                };
                Line::from(vec![lead, Span::styled(digits, score_style)])
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), right_area);
        return;
    }

    let [first, below] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(area);
    if let Some(unbeaten) = unbeaten_text {
        frame.render_widget(Paragraph::new(unbeaten).right_aligned(), below);
    }
    let [left_area, right_area] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(first);
    frame.render_widget(Paragraph::new(left), left_area);
    frame.render_widget(
        Paragraph::new(Line::styled(
            voice.score(score.done, score.planned),
            score_style,
        ))
        .right_aligned(),
        right_area,
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
        ledger.add("Ler o livro novo", "lazer", Fit::Focus, Status::Ready);

        let changes = ledger.projection().finish(filter, today).unwrap();
        ledger.record(filter, changes).unwrap();
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
        assert!(draw(&app, &ledger, 100, 18).contains("Placar: 1 de 2"));

        // O cursor desce pra segunda tarefa do dia e marca o gol.
        app.on_key(key(KeyCode::Char('j')), &mut ledger);
        app.on_key(key(KeyCode::Char(' ')), &mut ledger);
        assert!(draw(&app, &ledger, 100, 18).contains("Placar: 2 de 2"));

        // O VAR anula.
        app.on_key(key(KeyCode::Char(' ')), &mut ledger);
        assert!(draw(&app, &ledger, 100, 18).contains("Placar: 1 de 2"));
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
        assert_eq!(lineup.score().done, 2);
        assert_eq!(lineup.score().planned, 3);
    }

    fn pull(app: &mut crate::app::App, ledger: &mut MemoryLedger, text: &str) {
        app.on_key(key(KeyCode::Char('e')), ledger);
        for c in text.chars() {
            app.on_key(key(KeyCode::Char(c)), ledger);
        }
        app.on_key(key(KeyCode::Enter), ledger);
    }

    #[test]
    fn a_ready_task_is_pulled_into_today_by_a_word() {
        let mut app = new_app(Voice::Game);
        let mut ledger = example_day();
        // "Ler o livro novo" está pronta no banco, sem dia.
        assert!(draw(&app, &ledger, 100, 20).contains("No banco: 1 pronta"));

        pull(&mut app, &mut ledger, "livro");
        let lineup = Lineup::new(ledger.projection(), today());
        assert!(titles_of(&lineup.planned).contains(&"Ler o livro novo".to_owned()));
        assert!(!draw(&app, &ledger, 100, 20).contains("No banco:"));
    }

    #[test]
    fn a_ready_task_is_pulled_by_its_id_too() {
        let mut app = new_app(Voice::Game);
        let mut ledger = example_day();
        let id = ledger.projection().inbox()[0].id();
        pull(&mut app, &mut ledger, &id.to_string());
        assert_eq!(ledger.projection().task(id).unwrap().day(), Some(today()));
    }

    #[test]
    fn a_word_that_matches_nothing_is_refused() {
        let mut app = new_app(Voice::Game);
        let mut ledger = example_day();
        pull(&mut app, &mut ledger, "nada-disso");
        assert_eq!(Lineup::new(ledger.projection(), today()).planned.len(), 2);
    }

    fn titles_of(tasks: &[Task]) -> Vec<String> {
        tasks.iter().map(|task| task.title().to_string()).collect()
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
