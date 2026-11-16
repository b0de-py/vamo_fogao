//! A tela Museu: o acervo inteiro pra navegar.

use jiff::civil::Date;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use vamo_acervo::{Fact, Tag, happened_on};

use crate::curiosity::{Acervo, day};
use crate::panel::Panel;
use crate::{Density, Voice, theme};

/// Como o Museu está arrumado.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Shelf {
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
    /// A década escolhida na linha do tempo (1950 é a de 1950 a 1959).
    /// `None` mostra todas.
    decade: Option<u16>,
}

/// A década de um ano: 1957 é da década de 1950.
fn decade_of(year: u16) -> u16 {
    year - year % 10
}

/// Uma linha da lista: um título de grupo ou um item.
enum Entry<'a> {
    Heading(String),
    Fact(&'a Fact),
    Banter(&'a vamo_acervo::Banter),
}

impl Museum {
    /// Reage a uma tecla da tela.
    pub(crate) fn on_key(&mut self, key: KeyEvent, acervo: &Acervo, voice: Voice) {
        let items = self
            .entries(acervo, voice)
            .iter()
            .filter(|e| !matches!(e, Entry::Heading(_)))
            .count();
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.cursor = (self.cursor + 1).min(items.saturating_sub(1));
            }
            KeyCode::Up | KeyCode::Char('k') => self.cursor = self.cursor.saturating_sub(1),
            // A linha do tempo: as setas dos lados andam pelas décadas que
            // têm fato, pulando as vazias. Antes da primeira fica "todas".
            KeyCode::Left | KeyCode::Char('h') if self.shelf == Shelf::ByDate => {
                let decades = decades(acervo);
                self.decade = match self.decade {
                    None => None,
                    Some(current) => decades.into_iter().rev().find(|&d| d < current),
                };
                self.cursor = 0;
            }
            KeyCode::Right | KeyCode::Char('l') if self.shelf == Shelf::ByDate => {
                let decades = decades(acervo);
                let next = match self.decade {
                    None => decades.first().copied(),
                    Some(current) => decades.into_iter().find(|&d| d > current),
                };
                // Na última década, a seta não sai do lugar.
                if next.is_some() {
                    self.decade = next;
                    self.cursor = 0;
                }
            }
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
    fn entries<'a>(&self, acervo: &'a Acervo, voice: Voice) -> Vec<Entry<'a>> {
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
            // Com uma década escolhida na linha do tempo, só os fatos dela.
            // Fato sem data só aparece em "todas".
            Shelf::ByDate => facts
                .into_iter()
                .filter(|fact| match self.decade {
                    None => true,
                    Some(decade) => fact
                        .date()
                        .is_some_and(|date| decade_of(date.year()) == decade),
                })
                .map(Entry::Fact)
                .collect(),
            Shelf::ByTheme => {
                let mut tags: Vec<&Tag> = facts.iter().flat_map(|fact| fact.tags()).collect();
                tags.sort();
                tags.dedup();
                let mut entries = Vec::new();
                for tag in tags {
                    entries.push(Entry::Heading(tag_label(tag, voice)));
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
                    entries.push(Entry::Heading(voice.no_theme().to_owned()));
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

    /// O alto da tela: o título com a prateleira e, na prateleira por data,
    /// a linha do tempo embaixo.
    fn header(&self, acervo: &Acervo, width: usize, voice: Voice) -> Vec<Line<'static>> {
        let arrangement = voice.shelf(self.shelf);
        let mut header = vec![Line::from(vec![
            Span::styled(voice.museum_title(), theme::heading()),
            Span::styled(format!(" · {arrangement}"), theme::soft()),
        ])];
        if self.shelf == Shelf::ByDate {
            let decades = decades(acervo);
            if !decades.is_empty() {
                header.push(timeline(&decades, self.decade, width, voice));
            }
        }
        header
    }

    /// Desenha a tela.
    pub(crate) fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        acervo: &Acervo,
        today: Date,
        density: Density,
        voice: Voice,
    ) {
        let header = self.header(acervo, usize::from(area.width), voice);
        // O título, a linha do tempo quando tem, e uma linha de respiro.
        let height = u16::try_from(header.len() + 1).unwrap_or(u16::MAX);
        let [top, rest] =
            Layout::vertical([Constraint::Length(height), Constraint::Fill(1)]).areas(area);
        frame.render_widget(Paragraph::new(header), top);

        let entries = self.entries(acervo, voice);
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
                // O tema e o adversário vêm do acervo e podem ser compridos,
                // então ganham a cor de rótulo, mas não a caixa-alta espaçada.
                Entry::Heading(text) => {
                    lines.push(Line::styled(text.clone(), theme::section()));
                }
                Entry::Fact(fact) => {
                    let this_day = today.is_some_and(|today| happened_on(fact, today));
                    let date = fact.date().map(|date| date.to_string()).unwrap_or_default();
                    // A linha começa na margem, como a tabela do Banco: os
                    // fatos são textos longos e cada coluna conta. A marca
                    // do "Neste dia" fica entre a data e o texto.
                    let mut line = Line::from(vec![
                        Span::styled(format!("{date:<10}"), theme::muted()),
                        Span::styled(if this_day { " ★ " } else { "   " }, theme::museum()),
                        Span::styled(fact.text().to_string(), theme::text()),
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
                    let mut line = Line::styled(banter.text().to_string(), theme::text());
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
                Line::styled(banter.text().to_string(), theme::text()),
            ],
            _ => Vec::new(),
        };
        let panel = Panel::new().density(density);
        let inner = panel.inner(detail_area);
        frame.render_widget(panel, detail_area);
        frame.render_widget(Paragraph::new(detail).wrap(Wrap { trim: false }), inner);
    }
}

/// As décadas que têm pelo menos um fato verificado com data, em ordem.
fn decades(acervo: &Acervo) -> Vec<u16> {
    let mut decades: Vec<u16> = acervo
        .facts
        .iter()
        .filter(|fact| fact.is_verified())
        .filter_map(Fact::date)
        .map(|date| decade_of(date.year()))
        .collect();
    decades.sort_unstable();
    decades.dedup();
    decades
}

/// A linha do tempo: "todas" e uma década atrás da outra, da primeira à
/// última que têm fato. A escolhida fica acesa, as que têm fato ficam à
/// vista, e as vazias do meio aparecem apagadas (as setas pulam por cima).
///
/// Quando a régua inteira não cabe em `width`, vira a forma curta: só a
/// década escolhida, entre setas.
fn timeline(decades: &[u16], chosen: Option<u16>, width: usize, voice: Voice) -> Line<'static> {
    let (Some(&first), Some(&last)) = (decades.first(), decades.last()) else {
        return Line::default();
    };
    let label = |text: String, lit: bool, on: bool| {
        let style = match (on, lit) {
            (true, _) => theme::chip(),
            (false, true) => theme::soft(),
            (false, false) => theme::line(),
        };
        Span::styled(format!(" {text} "), style)
    };
    let mut spans = vec![label(
        voice.all_decades().to_owned(),
        true,
        chosen.is_none(),
    )];
    for decade in (first..=last).step_by(10) {
        spans.push(label(
            decade.to_string(),
            decades.contains(&decade),
            chosen == Some(decade),
        ));
    }
    if spans.iter().map(Span::width).sum::<usize>() <= width {
        return Line::from(spans);
    }
    let current = chosen.map_or_else(|| voice.all_decades().to_owned(), |d| d.to_string());
    Line::from(vec![
        Span::styled("‹", theme::muted()),
        label(current, true, true),
        Span::styled("›", theme::muted()),
    ])
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
    lines.push(Line::styled(format!(" {}", fact.text()), theme::text()));
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
            Span::styled(format!(" · {}", source.name()), theme::text()),
            Span::styled(link, theme::selected()),
        ]));
    }
    if !fact.tags().is_empty() {
        lines.push(Line::default());
        let tags: Vec<String> = fact
            .tags()
            .iter()
            .map(|tag| tag_label(tag, voice))
            .collect();
        lines.push(Line::styled(
            format!(" {}", tags.join(" · ")),
            theme::muted(),
        ));
    }
    lines
}

/// O tema em português, como `ídolo: nilton santos`.
fn tag_label(tag: &Tag, voice: Voice) -> String {
    format!(
        "{}: {}",
        voice.tag_kind(tag),
        tag.value().as_str().replace('-', " ")
    )
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

    /// Um acervo inventado com fatos em três décadas, uma vazia no meio
    /// (a de 1960 só tem fato a conferir) e um fato sem data.
    fn three_decades() -> Acervo {
        let fact = |slug: &str, date: Option<&str>, status: &str| {
            let date = date
                .map(|d| format!("data = \"{d}\"\n"))
                .unwrap_or_default();
            let sources = "[[fontes]]\nnome = \"Jornal\"\nurl = \"https://jornal.example.com.br/\"\n\
                           [[fontes]]\nnome = \"Revista\"\nurl = \"https://revista.example.org/\"\n";
            vamo_acervo::Fact::from_toml(
                slug,
                &format!(
                    "texto = \"Fato de teste {slug}.\"\n{date}status = \"{status}\"\n{sources}"
                ),
            )
            .expect("o fato de teste é válido")
        };
        Acervo {
            facts: vec![
                fact("um-de-cinquenta", Some("1950-11-11"), "verificado"),
                fact("outro-de-cinquenta", Some("1957"), "verificado"),
                fact("a-conferir-de-sessenta", Some("1960"), "a-conferir"),
                fact("um-de-setenta", Some("1972-05-01"), "verificado"),
                fact("um-de-noventa", Some("1995"), "verificado"),
                fact("sem-data", None, "verificado"),
            ],
            banter: Vec::new(),
        }
    }

    fn shown(museum: &Museum, acervo: &Acervo) -> usize {
        museum.entries(acervo, Voice::Game).len()
    }

    fn text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn only_decades_with_a_verified_dated_fact_count() {
        assert_eq!(decades(&three_decades()), [1950, 1970, 1990]);
    }

    #[test]
    fn the_arrows_walk_the_decades_and_skip_the_empty_ones() {
        let acervo = three_decades();
        let mut museum = Museum::default();
        let press = |museum: &mut Museum, code| museum.on_key(key(code), &acervo, Voice::Game);
        // Começa em "todas": os cinco verificados, o sem data inclusive.
        assert_eq!((museum.decade, shown(&museum, &acervo)), (None, 5));
        press(&mut museum, KeyCode::Right);
        assert_eq!((museum.decade, shown(&museum, &acervo)), (Some(1950), 2));
        // A de 1960 não tem fato verificado: a seta pula pra de 1970.
        press(&mut museum, KeyCode::Right);
        assert_eq!((museum.decade, shown(&museum, &acervo)), (Some(1970), 1));
        press(&mut museum, KeyCode::Char('l'));
        assert_eq!(museum.decade, Some(1990));
        // Na última, a seta não sai do lugar.
        press(&mut museum, KeyCode::Right);
        assert_eq!(museum.decade, Some(1990));
        // De volta até "todas", e de lá não passa.
        for _ in 0..4 {
            press(&mut museum, KeyCode::Left);
        }
        assert_eq!(museum.decade, None);
    }

    #[test]
    fn the_timeline_only_works_on_the_shelf_by_date() {
        let acervo = three_decades();
        let mut museum = Museum::default();
        museum.on_key(key(KeyCode::Char('o')), &acervo, Voice::Game);
        museum.on_key(key(KeyCode::Right), &acervo, Voice::Game);
        assert_eq!(museum.decade, None);
    }

    #[test]
    fn the_timeline_shows_every_decade_and_dims_the_empty_ones() {
        let line = timeline(&[1950, 1970, 1990], Some(1970), 80, Voice::Game);
        assert_eq!(text(&line), " todas  1950  1960  1970  1980  1990 ");
        let style_of = |label: &str| {
            line.spans
                .iter()
                .find(|span| span.content.trim() == label)
                .map(|span| span.style)
        };
        assert_eq!(style_of("1970"), Some(theme::chip()));
        assert_eq!(style_of("1950"), Some(theme::soft()));
        assert_eq!(style_of("1960"), Some(theme::line()));
        // Sem lugar pra régua inteira, fica só a escolhida entre setas.
        assert_eq!(
            text(&timeline(&[1950, 1970, 1990], Some(1970), 20, Voice::Game)),
            "‹ 1970 ›"
        );
        assert_eq!(
            text(&timeline(&[1950, 1970, 1990], None, 20, Voice::Game)),
            "‹ todas ›"
        );
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
