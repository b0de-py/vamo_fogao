//! O quadro da curiosidade do dia, e da resenha no lugar dela.

use jiff::civil::Date;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};
use vamo_acervo::{Banter, Curiosity, Day, Fact, banter_of_the_day, curiosity_of_the_day};

use crate::{Voice, theme};

/// O acervo que a TUI mostra.
#[derive(Debug, Clone, Default)]
pub struct Acervo {
    /// Os fatos. Os que estão `a-conferir` ficam guardados, mas nunca
    /// aparecem.
    pub facts: Vec<Fact>,
    /// A resenha, a zoeira que aparece com o selo.
    pub banter: Vec<Banter>,
}

/// O dia do calendário no formato do acervo. `None` antes do ano 1000, o
/// que pra hoje não acontece.
pub(crate) fn day(date: Date) -> Option<Day> {
    let year = u16::try_from(date.year()).ok()?;
    Day::new(year, date.month().unsigned_abs(), date.day().unsigned_abs())
}

/// Desenha o quadro: a curiosidade do dia ou, com `banter`, a resenha.
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    acervo: &Acervo,
    today: Date,
    banter: bool,
    voice: Voice,
) {
    let (title, lines) = match day(today) {
        Some(today) if banter => banter_lines(acervo, today, voice),
        Some(today) => curiosity_lines(acervo, today, voice),
        None => (voice.curiosity_title().to_owned(), Vec::new()),
    };
    let block = Block::bordered()
        .border_style(theme::line())
        .title(Line::styled(format!(" {title} "), theme::museum()));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

fn curiosity_lines(acervo: &Acervo, today: Day, voice: Voice) -> (String, Vec<Line<'static>>) {
    let Some(curiosity) = curiosity_of_the_day(&acervo.facts, today) else {
        return (
            voice.curiosity_title().to_owned(),
            vec![Line::styled(voice.no_curiosity(), theme::muted())],
        );
    };
    let fact = curiosity.fact();
    let title = match (curiosity, fact.date()) {
        (Curiosity::OnThisDay(_), Some(date)) => voice.on_this_day(date.year()),
        _ => voice.curiosity_title().to_owned(),
    };
    let sources: Vec<&str> = fact
        .sources()
        .iter()
        .map(vamo_acervo::Source::name)
        .collect();
    let lines = vec![
        Line::styled(fact.text().to_string(), theme::base()),
        Line::default(),
        Line::from(vec![
            Span::styled(voice.sources(), theme::muted()),
            Span::styled(sources.join(" · "), theme::soft()),
        ]),
        Line::styled(voice.banter_hint(), theme::muted()),
    ];
    (title, lines)
}

fn banter_lines(acervo: &Acervo, today: Day, voice: Voice) -> (String, Vec<Line<'static>>) {
    let title = voice.banter_seal().to_owned();
    let lines = match banter_of_the_day(&acervo.banter, today) {
        Some(banter) => vec![
            Line::styled(banter.text().to_string(), theme::base()),
            Line::default(),
            Line::styled(voice.back_to_fact_hint(), theme::muted()),
        ],
        None => vec![Line::styled(voice.no_banter(), theme::muted())],
    };
    (title, lines)
}
