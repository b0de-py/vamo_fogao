//! A tela Museu: o acervo inteiro pra navegar.

use jiff::civil::Date;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use vamo_acervo::{Fact, Tag, happened_on};

use crate::curiosity::{Acervo, day};
use crate::{Voice, theme};

/// Como o Museu está arrumado.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Shelf {
    /// Os fatos em ordem de data.
    #[default]
    ByDate,
    /// Os fatos agrupados por tema (as tags).
    ByTheme,
    /// A resenha, a zoeira, separada dos fatos.
    Banter,
}

/// O estado do Museu.
#[derive(Debug, Default)]
pub(crate) struct Museum {
    shelf: Shelf,
    cursor: usize,
}

/// Uma linha da lista: um título de grupo ou um item.
enum Entry<'a> {
    Heading(String),
    Fact(&'a Fact),
    Banter(&'a vamo_acervo::Banter),
}

impl Museum {
    /// Reage a uma tecla da tela.
    pub(crate) fn on_key(&mut self, key: KeyEvent, acervo: &Acervo) {
        let items = self
            .entries(acervo)
            .iter()
            .filter(|e| !matches!(e, Entry::Heading(_)))
            .count();
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.cursor = (self.cursor + 1).min(items.saturating_sub(1));
            }
            KeyCode::Up | KeyCode::Char('k') => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Char('o') => {
                self.shelf = match self.shelf {
                    Shelf::ByDate => Shelf::ByTheme,
                    Shelf::ByTheme | Shelf::Banter => Shelf::ByDate,
                };
                self.cursor = 0;
            }
            KeyCode::Char('z') => {
                self.shelf = if self.shelf == Shelf::Banter {
                    Shelf::ByDate
                } else {
                    Shelf::Banter
                };
                self.cursor = 0;
            }
            _ => {}
        }
    }

    /// As linhas da lista, na ordem da prateleira. Fato a conferir nunca
    /// entra.
    fn entries<'a>(&self, acervo: &'a Acervo) -> Vec<Entry<'a>> {
        let mut facts: Vec<&Fact> = acervo
            .facts
            .iter()
            .filter(|fact| fact.is_verified())
            .collect();
        // Sem data vai pro fim; no empate, a ordem do id.
        facts.sort_by(|a, b| {
            (a.date().is_none(), a.date(), a.id()).cmp(&(b.date().is_none(), b.date(), b.id()))
        });
        match self.shelf {
            Shelf::ByDate => facts.into_iter().map(Entry::Fact).collect(),
            Shelf::ByTheme => {
                let mut tags: Vec<&Tag> = facts.iter().flat_map(|fact| fact.tags()).collect();
                tags.sort();
                tags.dedup();
                let mut entries = Vec::new();
                for tag in tags {
                    entries.push(Entry::Heading(tag_label(tag)));
                    entries.extend(
                        facts
                            .iter()
                            .filter(|fact| fact.tags().contains(tag))
                            .map(|fact| Entry::Fact(fact)),
                    );
                }
                let loose: Vec<&&Fact> =
                    facts.iter().filter(|fact| fact.tags().is_empty()).collect();
                if !loose.is_empty() {
                    entries.push(Entry::Heading("sem tema".to_owned()));
                    entries.extend(loose.into_iter().map(|fact| Entry::Fact(fact)));
                }
                entries
            }
            Shelf::Banter => {
                let mut banter: Vec<_> = acervo.banter.iter().collect();
                banter.sort_by(|a, b| a.id().cmp(b.id()));
                banter.into_iter().map(Entry::Banter).collect()
            }
        }
    }

    /// Desenha a tela.
    pub(crate) fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        acervo: &Acervo,
        today: Date,
        voice: Voice,
    ) {
        let [top, rest] =
            Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).areas(area);
        let arrangement = match self.shelf {
            Shelf::ByDate => "por data (o: por tema · z: resenha)",
            Shelf::ByTheme => "por tema (o: por data · z: resenha)",
            Shelf::Banter => "resenha (z: volta pros fatos)",
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(voice.museum_title(), theme::heading()),
                Span::styled(format!(" · {arrangement}"), theme::soft()),
            ])),
            top,
        );

        let entries = self.entries(acervo);
        let items = entries
            .iter()
            .filter(|e| !matches!(e, Entry::Heading(_)))
            .count();
        if items == 0 {
            let empty = match self.shelf {
                Shelf::Banter => voice.no_banter(),
                _ => voice.no_curiosity(),
            };
            frame.render_widget(Paragraph::new(Line::styled(empty, theme::muted())), rest);
            return;
        }
        let cursor = self.cursor.min(items - 1);
        let today = day(today);

        let (list_area, detail_area) = if rest.width >= 90 {
            let [list, detail] =
                Layout::horizontal([Constraint::Percentage(45), Constraint::Fill(1)])
                    .spacing(1)
                    .areas(rest);
            (list, detail)
        } else {
            let [list, detail] =
                Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)]).areas(rest);
            (list, detail)
        };

        // A lista, com o item do cursor destacado.
        let mut lines = Vec::new();
        let mut index = 0;
        let mut selected = None;
        let mut cursor_line = 0;
        for entry in &entries {
            match entry {
                Entry::Heading(text) => lines.push(Line::styled(text.clone(), theme::soft())),
                Entry::Fact(fact) => {
                    let this_day = today.is_some_and(|today| happened_on(fact, today));
                    let date = fact.date().map(|date| date.to_string()).unwrap_or_default();
                    let mut line = Line::from(vec![
                        Span::styled(if this_day { "★ " } else { "  " }, theme::museum()),
                        Span::styled(format!("{date:<10}  "), theme::muted()),
                        Span::styled(fact.text().to_string(), theme::base()),
                    ]);
                    if index == cursor {
                        line = line.style(theme::cursor());
                        selected = Some(entry);
                        cursor_line = lines.len();
                    }
                    lines.push(line);
                    index += 1;
                }
                Entry::Banter(banter) => {
                    let mut line = Line::styled(format!("  {}", banter.text()), theme::base());
                    if index == cursor {
                        line = line.style(theme::cursor());
                        selected = Some(entry);
                        cursor_line = lines.len();
                    }
                    lines.push(line);
                    index += 1;
                }
            }
        }
        // A lista rola pra manter o cursor à vista.
        let height = usize::from(list_area.height).max(1);
        let scroll = u16::try_from(cursor_line.saturating_sub(height - 1)).unwrap_or(u16::MAX);
        frame.render_widget(Paragraph::new(lines).scroll((scroll, 0)), list_area);

        let detail = match selected {
            Some(Entry::Fact(fact)) => fact_detail(
                fact,
                today.is_some_and(|today| happened_on(fact, today)),
                voice,
            ),
            Some(Entry::Banter(banter)) => vec![
                Line::styled(voice.banter_seal(), theme::museum()),
                Line::default(),
                Line::styled(banter.text().to_string(), theme::base()),
            ],
            _ => Vec::new(),
        };
        let block = Block::new()
            .borders(Borders::LEFT)
            .border_style(theme::line());
        frame.render_widget(
            Paragraph::new(detail)
                .block(block)
                .wrap(Wrap { trim: false }),
            detail_area,
        );
    }
}

/// O detalhe de um fato: o texto, a data, as fontes com o link e os temas.
fn fact_detail(fact: &Fact, this_day: bool, voice: Voice) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if this_day {
        lines.push(Line::styled(
            format!(" ★ {}", voice.on_this_day_badge()),
            theme::museum(),
        ));
    }
    let when = match (fact.date(), fact.until()) {
        (Some(date), Some(until)) => format!("{date} a {until}"),
        (Some(date), None) => date.to_string(),
        _ => String::new(),
    };
    lines.push(Line::styled(format!(" {when}"), theme::muted()));
    lines.push(Line::styled(format!(" {}", fact.text()), theme::base()));
    lines.push(Line::default());
    lines.push(Line::styled(
        format!(" {}", voice.sources().trim_end_matches(": ")),
        theme::soft(),
    ));
    for source in fact.sources() {
        let link = source
            .url()
            .map(|url| format!("  {url}"))
            .unwrap_or_default();
        lines.push(Line::from(vec![
            Span::styled(format!(" · {}", source.name()), theme::base()),
            Span::styled(link, theme::selected()),
        ]));
    }
    if !fact.tags().is_empty() {
        lines.push(Line::default());
        let tags: Vec<String> = fact.tags().iter().map(tag_label).collect();
        lines.push(Line::styled(
            format!(" {}", tags.join(" · ")),
            theme::muted(),
        ));
    }
    lines
}

/// O tema em português, como `ídolo: nilton santos`.
fn tag_label(tag: &Tag) -> String {
    let kind = match tag {
        Tag::Era(_) => "época",
        Tag::Idol(_) => "ídolo",
        Tag::Title(_) => "título",
        Tag::Opponent(_) => "adversário",
        Tag::Sport(_) => "esporte",
    };
    format!("{kind}: {}", tag.value().as_str().replace('-', " "))
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::*;
    use crate::app::tests::{draw, key, ledger, new_app};

    fn open_museum() -> (crate::app::App, crate::ledger::memory::MemoryLedger) {
        let mut ledger = ledger();
        let mut app = new_app(Voice::Game);
        app.on_key(key(KeyCode::Char('4')), &mut ledger);
        (app, ledger)
    }

    #[test]
    fn the_museum_by_date() {
        let (app, ledger) = open_museum();
        let screen = draw(&app, &ledger, 100, 16);
        assert!(
            !screen.contains("ainda não fechou"),
            "fato a conferir apareceu:\n{screen}"
        );
        assert_snapshot!(screen);
    }

    #[test]
    fn the_museum_by_theme_and_the_banter() {
        let (mut app, mut ledger) = open_museum();
        app.on_key(key(KeyCode::Char('o')), &mut ledger);
        let screen = draw(&app, &ledger, 100, 16);
        assert!(screen.contains("adversário: time de teste"), "{screen}");

        app.on_key(key(KeyCode::Char('z')), &mut ledger);
        let screen = draw(&app, &ledger, 100, 16);
        assert!(screen.contains("é zoeira, não é fato"), "{screen}");
        assert!(
            screen.contains("O time dos testes perdeu de novo."),
            "{screen}"
        );
    }
}
