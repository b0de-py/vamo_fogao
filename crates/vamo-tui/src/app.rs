//! O estado da TUI, as teclas e o desenho de cada quadro.
//!
//! A TUI segue o modelo de "estado e desenho": o [`App`] guarda o estado, as
//! teclas mudam o estado, e cada quadro é desenhado do zero a partir dele.
//! Nada de mexer na tela aos pedaços. É o que deixa testar uma tela
//! desenhando o estado num terminal de mentira e comparando com o snapshot.

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::{Ledger, Voice, theme};

/// As abas, na ordem em que aparecem no topo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    /// A tela que abre primeiro: o placar e a escalação do dia.
    Today,
    /// A lista do dia no padrão do caderno de papel.
    Notebook,
    /// Todas as tarefas, com filtro.
    Bench,
    /// O acervo.
    Museum,
}

impl Tab {
    /// Todas as abas, na ordem do topo.
    const ALL: [Self; 4] = [Self::Today, Self::Notebook, Self::Bench, Self::Museum];

    /// A posição da aba no topo, a partir de zero.
    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|&tab| tab == self)
            .expect("toda aba está em Tab::ALL")
    }

    /// A aba seguinte, voltando pra primeira depois da última.
    fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    /// A aba anterior, indo pra última antes da primeira.
    fn previous(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// O estado da TUI.
#[derive(Debug)]
pub(crate) struct App {
    voice: Voice,
    tab: Tab,
    help: bool,
    quit: bool,
}

impl App {
    pub(crate) fn new(voice: Voice) -> Self {
        Self {
            voice,
            tab: Tab::Today,
            help: false,
            quit: false,
        }
    }

    /// Se a TUI deve fechar depois deste quadro.
    pub(crate) fn should_quit(&self) -> bool {
        self.quit
    }

    /// Reage a uma tecla.
    pub(crate) fn on_key<L: Ledger>(&mut self, key: KeyEvent, _ledger: &mut L) {
        // O ctrl+c fecha de qualquer lugar, como em todo programa de terminal.
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        // Com a ajuda aberta, qualquer tecla só fecha a ajuda.
        if self.help {
            self.help = false;
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('?') => self.help = true,
            KeyCode::Tab => self.tab = self.tab.next(),
            KeyCode::BackTab => self.tab = self.tab.previous(),
            KeyCode::Char(digit @ '1'..='4') => {
                let index = usize::from(digit as u8 - b'1');
                self.tab = Tab::ALL[index];
            }
            _ => {}
        }
    }

    /// Desenha um quadro inteiro.
    pub(crate) fn render<L: Ledger>(&self, frame: &mut Frame<'_>, _ledger: &L) {
        let area = frame.area();
        frame.render_widget(Block::new().style(theme::base()), area);

        let [header, body, footer] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        self.render_header(frame, header);
        self.render_body(frame, body);
        frame.render_widget(
            Paragraph::new(Line::styled(self.voice.hints(), theme::muted())),
            footer,
        );

        if self.help {
            self.render_help(frame, body);
        }
    }

    /// O nome do app e as abas, com uma linha embaixo.
    fn render_header(&self, frame: &mut Frame<'_>, area: Rect) {
        let mut spans = vec![
            Span::styled(" vamo_fogao ", theme::heading()),
            Span::raw(" "),
        ];
        for (number, tab) in (1..).zip(Tab::ALL) {
            let style = if tab == self.tab {
                theme::selected()
            } else {
                theme::muted()
            };
            spans.push(Span::styled(
                format!(" {number} {} ", self.voice.tab(tab)),
                style,
            ));
        }
        let block = Block::new()
            .borders(Borders::BOTTOM)
            .border_style(theme::line());
        frame.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
    }

    /// O miolo da tela atual.
    fn render_body(&self, frame: &mut Frame<'_>, area: Rect) {
        let title = Line::styled(self.voice.tab(self.tab), theme::heading());
        frame.render_widget(
            Paragraph::new(title),
            area.inner(ratatui::layout::Margin::new(1, 1)),
        );
    }

    /// A ajuda, numa caixa por cima da tela.
    fn render_help(&self, frame: &mut Frame<'_>, area: Rect) {
        let rows = self.voice.help();
        let key_width = rows
            .iter()
            .map(|(key, _)| key.chars().count())
            .max()
            .unwrap_or(0);
        let lines: Vec<Line<'_>> = rows
            .iter()
            .map(|(key, what)| {
                Line::from(vec![
                    Span::styled(format!("{key:<key_width$}  "), theme::selected()),
                    Span::styled(*what, theme::soft()),
                ])
            })
            .collect();
        let width = lines
            .iter()
            .map(Line::width)
            .max()
            .unwrap_or(0)
            .saturating_add(4);
        let height = lines.len().saturating_add(2);
        let popup = centered(area, width, height);
        let block = Block::bordered()
            .border_style(theme::line())
            .title(Line::styled(
                format!(" {} ", self.voice.help_title()),
                theme::heading(),
            ))
            .style(theme::base());
        frame.render_widget(Clear, popup);
        frame.render_widget(Paragraph::new(lines).block(block), popup);
    }
}

/// Um retângulo do tamanho pedido no meio de `area`, sem passar dela.
pub(crate) fn centered(area: Rect, width: usize, height: usize) -> Rect {
    let width = u16::try_from(width).unwrap_or(u16::MAX).min(area.width);
    let height = u16::try_from(height).unwrap_or(u16::MAX).min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use insta::assert_snapshot;
    use jiff::Timestamp;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{KeyEventKind, KeyEventState};

    use super::*;
    use crate::ledger::memory::MemoryLedger;

    pub(crate) fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    pub(crate) fn ledger() -> MemoryLedger {
        MemoryLedger::new("2026-11-11T10:00:00Z".parse::<Timestamp>().unwrap())
    }

    /// Desenha o app num terminal de mentira e devolve o texto da tela.
    pub(crate) fn draw(app: &App, ledger: &MemoryLedger, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| app.render(frame, ledger)).unwrap();
        terminal.backend().to_string()
    }

    #[test]
    fn opens_on_today() {
        let app = App::new(Voice::Game);
        assert_eq!(app.tab, Tab::Today);
        assert_snapshot!(draw(&app, &ledger(), 70, 8));
    }

    #[test]
    fn tab_goes_around() {
        let mut app = App::new(Voice::Game);
        let mut ledger = ledger();
        let mut seen = Vec::new();
        for _ in 0..5 {
            seen.push(app.tab);
            app.on_key(key(KeyCode::Tab), &mut ledger);
        }
        assert_eq!(
            seen,
            [
                Tab::Today,
                Tab::Notebook,
                Tab::Bench,
                Tab::Museum,
                Tab::Today
            ]
        );
        // Da primeira, o shift+tab vai pra última.
        let mut app = App::new(Voice::Game);
        app.on_key(key(KeyCode::BackTab), &mut ledger);
        assert_eq!(app.tab, Tab::Museum);
    }

    #[test]
    fn numbers_jump_to_the_tab() {
        let mut app = App::new(Voice::Game);
        let mut ledger = ledger();
        app.on_key(key(KeyCode::Char('3')), &mut ledger);
        assert_eq!(app.tab, Tab::Bench);
        app.on_key(key(KeyCode::Char('9')), &mut ledger);
        assert_eq!(app.tab, Tab::Bench);
    }

    #[test]
    fn help_opens_and_any_key_closes_it() {
        let mut app = App::new(Voice::Game);
        let mut ledger = ledger();
        app.on_key(key(KeyCode::Char('?')), &mut ledger);
        assert_snapshot!(draw(&app, &ledger, 70, 12));
        // Com a ajuda aberta, o q fecha a ajuda e não o app.
        app.on_key(key(KeyCode::Char('q')), &mut ledger);
        assert!(!app.help);
        assert!(!app.should_quit());
    }

    #[test]
    fn q_and_ctrl_c_quit() {
        let mut ledger = ledger();
        let mut app = App::new(Voice::Game);
        app.on_key(key(KeyCode::Char('q')), &mut ledger);
        assert!(app.should_quit());

        let mut app = App::new(Voice::Game);
        app.on_key(
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            &mut ledger,
        );
        assert!(app.should_quit());
    }

    #[test]
    fn the_neutral_voice_renames_the_bench() {
        let app = App::new(Voice::Neutral);
        let screen = draw(&app, &ledger(), 70, 4);
        assert!(screen.contains("3 Tarefas"), "{screen}");
        assert!(!screen.contains("Banco"), "{screen}");
    }

    #[test]
    fn a_tiny_terminal_does_not_panic() {
        let mut app = App::new(Voice::Game);
        app.help = true;
        for (width, height) in [(1, 1), (5, 2), (20, 3)] {
            draw(&app, &ledger(), width, height);
        }
    }
}
