//! O estado da TUI, as teclas e o desenho de cada quadro.
//!
//! A TUI segue o modelo de "estado e desenho": o [`App`] guarda o estado, as
//! teclas mudam o estado, e cada quadro é desenhado do zero a partir dele.
//! Nada de mexer na tela aos pedaços. É o que deixa testar uma tela
//! desenhando o estado num terminal de mentira e comparando com o snapshot.

use std::cell::Cell;
use std::time::{Duration, Instant};

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use jiff::civil::Date;

use jiff::tz::TimeZone;
use ratatui::style::Style;
use vamo_core::{Change, Fit, Lineup, NotebookTitle, Projection, Status, TaskId};

use crate::bench::Bench;
use crate::curiosity::Acervo;
use crate::fire::{self, Fire};
use crate::museum::Museum;
use crate::prompt::{Prompt, Purpose, Step};
use crate::star::Star;
use crate::today::Today;
use crate::when::parse_day;
use crate::{Animation, Ledger, Setup, Voice, card, notebook, theme};

/// O que aconteceu depois de uma ação de quem usa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Feedback {
    /// A mudança foi gravada.
    Done,
    /// A mudança foi gravada e foi gol: a tarefa ficou concluída.
    Goal(TaskId),
    /// A mudança foi gravada, com um recado pro rodapé.
    Note(String),
    /// Não deu: a regra não deixou ou a gravação falhou. A mensagem vai pro
    /// rodapé, em amarelo plasma.
    Refused(String),
}

/// Grava as mudanças que o core devolveu, ou repassa a recusa dele.
pub(crate) fn record<L: Ledger>(
    ledger: &mut L,
    task: TaskId,
    changes: Result<Vec<Change>, vamo_core::Error>,
) -> Feedback {
    let changes = match changes {
        Ok(changes) if changes.is_empty() => return Feedback::Done,
        Ok(changes) => changes,
        Err(error) => return Feedback::Refused(error.to_string()),
    };
    match ledger.record(task, changes) {
        Ok(()) => Feedback::Done,
        Err(error) => Feedback::Refused(error.to_string()),
    }
}

/// Uma mensagem no rodapé, até a próxima tecla.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Message {
    text: String,
    style: Style,
}

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

/// O que uma tecla numa tela pede pro app fazer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    /// Mostrar o resultado de uma mudança.
    Feedback(Feedback),
    /// Abrir a Ficha da tarefa.
    Open(TaskId),
    /// Fechar a Ficha.
    Close,
    /// Abrir a linha de digitar.
    Ask(Prompt),
}

/// O estado da TUI.
#[derive(Debug)]
pub(crate) struct App {
    voice: Voice,
    acervo: Acervo,
    zone: TimeZone,
    /// O dia de hoje, na hora local. Quem roda a TUI atualiza a cada quadro.
    today: Date,
    tab: Tab,
    /// Se o quadro da curiosidade mostra a resenha no lugar do fato.
    banter: bool,
    today_screen: Today,
    bench: Bench,
    museum: Museum,
    /// A tarefa com a Ficha aberta, por cima do Banco.
    card: Option<TaskId>,
    prompt: Option<Prompt>,
    message: Option<Message>,
    help: bool,
    quit: bool,
    animation: Animation,
    /// Se a janela está à vista. Quando some (outro workspace, outra
    /// janela por cima), o fogo pausa.
    window: Window,
    /// Até quando o fogo queima sem ninguém mexer. Cada tecla e cada
    /// movimento do mouse empurram esse instante pra frente.
    awake_until: Option<Instant>,
    fire: Fire,
    /// O tamanho da tela no último quadro, ou zero linhas quando o fogo não
    /// cabe. O desenho não muda o estado, então guarda aqui e o próximo
    /// passo do fogo ajusta.
    fire_area: Cell<(u16, u16)>,
    /// Quantos passos do fogo ainda faltam no grito de gol.
    flash: u8,
}

/// Se a janela da TUI está à vista, pelo foco que o terminal avisa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Window {
    /// Com foco: o fogo queima.
    Visible,
    /// Sem foco: o fogo pausa.
    Hidden,
}

/// Quanto tempo o fogo queima depois da última tecla ou do último movimento
/// do mouse. São os 3 segundos do site: parado, o fogo congela e a TUI não
/// gasta CPU.
const IDLE: Duration = Duration::from_secs(3);

/// Quantos passos dura o grito de gol: com um passo a cada 40 ms, um
/// segundo e meio.
const FLASH_STEPS: u8 = 36;

/// De quantos em quantos passos do grito de gol estoura um foguete.
const FIREWORK_EVERY: u8 = 6;

/// De quantos em quantos passos o grito de gol acende e apaga.
const BLINK_EVERY: u8 = 4;

/// A partir de quantas linhas de terminal o fogo aparece. Num terminal
/// mais baixo, quase toda linha tem texto e não sobra onde ver a chama.
const FIRE_MIN_HEIGHT: u16 = 16;

impl App {
    pub(crate) fn new(setup: Setup, today: Date) -> Self {
        Self {
            voice: setup.voice,
            acervo: setup.acervo,
            zone: setup.time_zone,
            today,
            tab: Tab::Today,
            banter: false,
            today_screen: Today::default(),
            bench: Bench::default(),
            museum: Museum::default(),
            card: None,
            prompt: None,
            message: None,
            help: false,
            quit: false,
            animation: setup.animation,
            window: Window::Visible,
            awake_until: None,
            fire: Fire::new(fire::SEED),
            fire_area: Cell::new((0, 0)),
            flash: 0,
        }
    }

    /// Se a animação está ligada na config. Desligada, a TUI nem pede os
    /// eventos do mouse ao terminal.
    pub(crate) fn animated(&self) -> bool {
        self.animation == Animation::On
    }

    /// Se o fogo está queimando em `now`: com animação, a janela à vista e
    /// alguém mexendo há pouco (ou um gol sendo gritado). Quando não está,
    /// quem roda a TUI só espera um evento e não gasta CPU.
    pub(crate) fn animating(&self, now: Instant) -> bool {
        let awake = self.flash > 0 || self.awake_until.is_some_and(|until| now < until);
        self.animated() && self.window == Window::Visible && awake
    }

    /// Acorda o fogo: alguém mexeu em `now`. No site, é o que o movimento
    /// do mouse e as teclas fazem.
    pub(crate) fn wake(&mut self, now: Instant) {
        self.awake_until = Some(now + IDLE);
    }

    /// Avisa se a janela ganhou ou perdeu o foco.
    pub(crate) fn set_window(&mut self, window: Window) {
        self.window = window;
    }

    /// Um clique numa célula do terminal atiça o fogo ali, como o clique no
    /// fundo do site.
    pub(crate) fn on_click(&mut self, column: u16, row: u16) {
        if self.fire_area.get().1 > 0 {
            self.fire.explode(column, row);
        }
    }

    /// Um passo do fogo. A brasa segue o placar do dia; no grito de gol, ela
    /// vai ao máximo e os foguetes estouram.
    pub(crate) fn tick<L: Ledger>(&mut self, ledger: &L, now: Instant) {
        if !self.animating(now) {
            return;
        }
        let (columns, rows) = self.fire_area.get();
        self.fire.resize(columns, rows);
        let strength = if self.flash > 0 {
            if self.flash.is_multiple_of(FIREWORK_EVERY) {
                self.fire.burst();
            }
            self.flash -= 1;
            1.0
        } else {
            let score = Lineup::new(ledger.projection(), self.today).score();
            if score.planned == 0 {
                0.0
            } else {
                // Contagens de tarefa do dia cabem com folga num u32.
                f64::from(u32::try_from(score.done).unwrap_or(u32::MAX))
                    / f64::from(u32::try_from(score.planned).unwrap_or(u32::MAX))
            }
        };
        self.fire.step(strength);
    }

    /// Avisa que dia é hoje. A TUI pode ficar aberta de um dia pro outro.
    pub(crate) fn set_today(&mut self, today: Date) {
        self.today = today;
    }

    /// Se a TUI deve fechar depois deste quadro.
    pub(crate) fn should_quit(&self) -> bool {
        self.quit
    }

    /// Reage a uma tecla.
    pub(crate) fn on_key<L: Ledger>(&mut self, key: KeyEvent, ledger: &mut L) {
        // A mensagem do rodapé vale até a próxima tecla.
        self.message = None;
        // O ctrl+c fecha de qualquer lugar, como em todo programa de terminal.
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        // Com a linha de digitar aberta, as teclas são texto.
        if let Some(prompt) = &mut self.prompt {
            match prompt.on_key(key) {
                Step::Typing => {}
                Step::Cancel => self.prompt = None,
                Step::Submit => {
                    if let Some(prompt) = self.prompt.take() {
                        let feedback = self.submit(prompt, ledger);
                        self.show(Some(feedback), ledger.projection());
                    }
                }
            }
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
            KeyCode::Char('r') => self.banter = !self.banter,
            KeyCode::Tab => self.go_to(self.tab.next()),
            KeyCode::BackTab => self.go_to(self.tab.previous()),
            KeyCode::Char(digit @ '1'..='4') => {
                let index = usize::from(digit as u8 - b'1');
                self.go_to(Tab::ALL[index]);
            }
            _ => {
                let action = match (self.tab, self.card) {
                    (Tab::Bench, Some(id)) => match ledger.projection().task(id) {
                        Some(task) => card::on_key(key, &task, ledger, self.today),
                        None => Some(Action::Close),
                    },
                    (Tab::Bench, None) => self.bench.on_key(key, ledger, self.today),
                    (Tab::Today, _) => self.today_screen.on_key(key, ledger, self.today),
                    (Tab::Museum, _) => {
                        self.museum.on_key(key, &self.acervo, self.voice);
                        None
                    }
                    (Tab::Notebook, _) => None,
                };
                self.act(action, ledger.projection());
            }
        }
    }

    /// Troca de aba, fechando a Ficha.
    fn go_to(&mut self, tab: Tab) {
        self.tab = tab;
        self.card = None;
    }

    /// Faz o que a tela pediu.
    fn act(&mut self, action: Option<Action>, projection: &Projection) {
        match action {
            None => {}
            Some(Action::Feedback(feedback)) => self.show(Some(feedback), projection),
            Some(Action::Open(id)) => self.card = Some(id),
            Some(Action::Close) => self.card = None,
            Some(Action::Ask(prompt)) => self.prompt = Some(prompt),
        }
    }

    /// Confirma o que foi digitado. A validação é a dos tipos do domínio:
    /// um título fora do padrão do caderno nem vira mudança.
    fn submit<L: Ledger>(&mut self, prompt: Prompt, ledger: &mut L) -> Feedback {
        let text = prompt.text.trim();
        let projection = ledger.projection();
        match prompt.purpose {
            Purpose::Filter => {
                self.bench.set_filter(text);
                Feedback::Done
            }
            Purpose::NewTask => {
                let created = text.parse::<NotebookTitle>().and_then(|title| {
                    let id = projection
                        .next_id()
                        .ok_or(vamo_core::Error::UnknownTask { id: TaskId::FIRST })?;
                    Ok((id, title))
                });
                match created {
                    Ok((id, title)) => {
                        let change = Change::Created {
                            title: title.clone(),
                            area: "geral".parse().expect("`geral` é uma área válida"),
                            fit: Fit::Gap,
                            status: Status::Ready,
                        };
                        match record(ledger, id, Ok(vec![change])) {
                            Feedback::Done => Feedback::Note(self.voice.added(id, &title)),
                            other => other,
                        }
                    }
                    Err(error) => Feedback::Refused(error.to_string()),
                }
            }
            Purpose::Title(id) => {
                let change = text.parse().map(|title| vec![Change::Title(title)]);
                record(ledger, id, change)
            }
            Purpose::Area(id) => {
                let change = text.parse().map(|area| vec![Change::Area(area)]);
                record(ledger, id, change)
            }
            Purpose::Notes(id) => record(ledger, id, Ok(vec![Change::Notes(text.to_owned())])),
            Purpose::Day(id) => match parse_day(text, self.today, self.voice) {
                Ok(day) => {
                    let changes = projection.schedule(id, day);
                    record(ledger, id, changes)
                }
                Err(message) => Feedback::Refused(message),
            },
            Purpose::Status(id, options) => {
                let chosen = text
                    .parse::<usize>()
                    .ok()
                    .and_then(|n| n.checked_sub(1))
                    .and_then(|index| options.get(index).copied());
                match chosen {
                    Some(status) => {
                        let change = projection
                            .status_change(id, status)
                            .map(|change| vec![change]);
                        record(ledger, id, change)
                    }
                    None => Feedback::Refused(self.voice.bad_option(text)),
                }
            }
        }
    }

    /// Mostra no rodapé o que aconteceu.
    fn show(&mut self, feedback: Option<Feedback>, projection: &Projection) {
        self.message = match feedback {
            None | Some(Feedback::Done) => None,
            Some(Feedback::Goal(id)) => {
                self.flash = FLASH_STEPS;
                projection.task(id).map(|task| Message {
                    text: self.voice.goal(id, task.title()),
                    style: theme::success(),
                })
            }
            Some(Feedback::Note(text)) => Some(Message {
                text,
                style: theme::soft(),
            }),
            Some(Feedback::Refused(text)) => Some(Message {
                text,
                style: theme::warning(),
            }),
        };
    }

    /// Desenha um quadro inteiro.
    pub(crate) fn render<L: Ledger>(&self, frame: &mut Frame<'_>, ledger: &L) {
        let area = frame.area();
        frame.render_widget(Block::new().style(theme::base()), area);

        let [header, body, footer] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        self.render_header(frame, header);
        self.render_body(frame, body, ledger);
        let footer_line = match (&self.prompt, &self.message) {
            (Some(prompt), _) => Line::from(vec![
                Span::styled(prompt.label(self.voice), theme::selected()),
                Span::styled(prompt.text.clone(), theme::text()),
                Span::styled("▏", theme::selected()),
            ]),
            (None, Some(message)) => Line::styled(message.text.clone(), message.style),
            (None, None) => Line::styled(self.voice.hints(), theme::muted()),
        };
        // O rodapé é uma faixa de vidro de ponta a ponta, como o do site: é
        // onde a brasa é mais clara e as dicas são mais apagadas.
        frame.render_widget(Paragraph::new(footer_line).style(theme::surface()), footer);

        if self.help {
            self.render_help(frame, body);
        }

        // O fogo e a estrela entram por último, por baixo de tudo o que já
        // foi desenhado.
        let burns = self.animated() && area.height >= FIRE_MIN_HEIGHT;
        self.fire_area
            .set((area.width, if burns { area.height } else { 0 }));
        let shouting = burns && self.flash > 0 && !(self.flash / BLINK_EVERY).is_multiple_of(2);
        let star = self.star(area).map(|star| star.glowing(shouting));
        if burns || star.is_some() {
            self.fire.underlay(area, frame.buffer_mut(), star.as_ref());
        }
        if shouting {
            self.render_shout(frame, body);
        }
    }

    /// A Estrela Solitária do canto da tela. Só na linguagem de jogo: no
    /// modo neutro, a TUI não fala de futebol nem por desenho.
    fn star(&self, area: Rect) -> Option<Star> {
        match self.voice {
            Voice::Game => Star::in_corner(area.width, area.height),
            Voice::Neutral => None,
        }
    }

    /// O GOL! do grito de gol, por cima do fogo, perto do rodapé.
    fn render_shout(&self, frame: &mut Frame<'_>, area: Rect) {
        if area.height < 3 {
            return;
        }
        let line = Rect {
            y: area.bottom() - 2,
            height: 1,
            ..area
        };
        let shout = Line::styled(self.voice.goal_shout(), theme::shout());
        frame.render_widget(Paragraph::new(shout).centered(), line);
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
    fn render_body<L: Ledger>(&self, frame: &mut Frame<'_>, area: Rect, ledger: &L) {
        let area = area.inner(ratatui::layout::Margin::new(1, 1));
        let projection = ledger.projection();
        match self.tab {
            Tab::Today => self.today_screen.render(
                frame,
                area,
                projection,
                self.today,
                &self.acervo,
                self.banter,
                self.voice,
            ),
            Tab::Notebook => notebook::render(frame, area, projection, self.today, self.voice),
            Tab::Bench => match self.card.and_then(|id| projection.task(id)) {
                Some(task) => {
                    card::render(frame, area, &task, ledger.events(), &self.zone, self.voice);
                }
                None => self.bench.render(frame, area, projection, self.voice),
            },
            Tab::Museum => self
                .museum
                .render(frame, area, &self.acervo, self.today, self.voice),
        }
    }

    /// A ajuda, numa caixa por cima da tela.
    fn render_help(&self, frame: &mut Frame<'_>, area: Rect) {
        let rows = self
            .voice
            .help(self.tab, self.card.is_some(), self.animation);
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
            .style(theme::surface());
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

    /// O dia dos testes: 11 de novembro de 2026.
    pub(crate) fn today() -> Date {
        jiff::civil::date(2026, 11, 11)
    }

    /// Um acervo inventado pros testes, com um fato verificado, um a
    /// conferir e uma resenha.
    pub(crate) fn acervo() -> Acervo {
        let source =
            |name: &str, url: &str| format!("[[fontes]]\nnome = \"{name}\"\nurl = \"{url}\"\n");
        let both = format!(
            "{}{}",
            source("Jornal", "https://jornal.example.com.br/"),
            source("Revista", "https://revista.example.org/")
        );
        let facts = vec![
            vamo_acervo::Fact::from_toml(
                "goleada-de-teste",
                &format!("texto = \"Uma goleada inventada pros testes.\"\ndata = \"1950-11-11\"\nstatus = \"verificado\"\ntags = [\"adversario:time-de-teste\"]\n{both}"),
            )
            .unwrap(),
            vamo_acervo::Fact::from_toml(
                "fato-a-conferir",
                &format!("texto = \"Um fato que ainda não fechou.\"\ndata = \"1960\"\nstatus = \"a-conferir\"\n{both}"),
            )
            .unwrap(),
        ];
        let banter = vec![
            vamo_acervo::Banter::from_toml(
                "zoeira-de-teste",
                "texto = \"O time dos testes perdeu de novo.\"",
            )
            .unwrap(),
        ];
        Acervo { facts, banter }
    }

    pub(crate) fn new_app(voice: Voice) -> App {
        App::new(
            Setup {
                voice,
                acervo: acervo(),
                time_zone: TimeZone::UTC,
                // Os snapshots das telas não mostram o fogo; os testes do
                // fogo ligam a animação.
                animation: Animation::Off,
            },
            today(),
        )
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
        let app = new_app(Voice::Game);
        assert_eq!(app.tab, Tab::Today);
        assert_snapshot!(draw(&app, &ledger(), 70, 8));
    }

    #[test]
    fn tab_goes_around() {
        let mut app = new_app(Voice::Game);
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
        let mut app = new_app(Voice::Game);
        app.on_key(key(KeyCode::BackTab), &mut ledger);
        assert_eq!(app.tab, Tab::Museum);
    }

    /// O app com o fogo ligado e acordado, no dia de exemplo, e o instante
    /// em que ele acordou.
    fn burning() -> (App, MemoryLedger, Instant) {
        let mut app = new_app(Voice::Game);
        app.animation = Animation::On;
        let now = Instant::now();
        app.wake(now);
        (app, crate::today::tests::example_day(), now)
    }

    #[test]
    fn the_fire_burns_behind_the_screen() {
        let (mut app, ledger, now) = burning();
        // O primeiro quadro mede a tela; os passos acendem o fogo.
        draw(&app, &ledger, 60, 18);
        assert_eq!(app.fire_area.get(), (60, 18));
        for _ in 0..40 {
            app.tick(&ledger, now);
        }
        let screen = draw(&app, &ledger, 60, 18);
        assert!(screen.contains('▀'), "{screen}");
        // O fogo passa por trás: o que estava escrito continua escrito.
        assert!(screen.contains("vamo_fogao"), "{screen}");
        assert!(screen.contains(app.voice.hints()), "{screen}");
        assert_snapshot!(screen);
    }

    #[test]
    fn a_short_terminal_has_no_fire() {
        let (mut app, ledger, now) = burning();
        draw(&app, &ledger, 60, 12);
        assert_eq!(app.fire_area.get(), (60, 0));
        app.tick(&ledger, now);
        assert!(!draw(&app, &ledger, 60, 12).contains('▀'));
    }

    #[test]
    fn a_goal_shouts_over_the_fire() {
        let (mut app, mut ledger, now) = burning();
        draw(&app, &ledger, 60, 18);
        // O cursor começa na primeira tarefa do dia que ainda não foi feita?
        // Não importa: desce até a T-002, que está em aberto.
        app.on_key(key(KeyCode::Char('j')), &mut ledger);
        app.on_key(key(KeyCode::Char(' ')), &mut ledger);
        assert_eq!(app.flash, FLASH_STEPS);
        // O grito termina mesmo que ninguém mexa mais: o gol mantém o fogo
        // acordado depois dos 3 segundos.
        let later = now + IDLE * 2;
        assert!(app.animating(later));
        let mut shouted = false;
        for _ in 0..FLASH_STEPS {
            app.tick(&ledger, later);
            shouted |= draw(&app, &ledger, 60, 18).contains("G O O O L !");
        }
        assert!(shouted);
        assert_eq!(app.flash, 0);
        assert!(!app.animating(later));
        assert!(!draw(&app, &ledger, 60, 18).contains("G O O O L !"));
    }

    #[test]
    fn the_fire_sleeps_after_three_idle_seconds_and_any_move_wakes_it() {
        let (mut app, ledger, now) = burning();
        draw(&app, &ledger, 60, 18);
        assert!(app.animating(now + Duration::from_secs(2)));
        let later = now + Duration::from_secs(4);
        assert!(!app.animating(later));
        // Dormindo, o fogo fica como estava, e continua na tela.
        app.tick(&ledger, now);
        let before = app.fire.clone();
        app.tick(&ledger, later);
        assert_eq!(app.fire, before);
        assert!(draw(&app, &ledger, 60, 18).contains('▀'));

        app.wake(later);
        assert!(app.animating(later));
        app.tick(&ledger, later);
        assert_ne!(app.fire, before);
    }

    #[test]
    fn a_click_stokes_the_fire() {
        let (mut app, ledger, now) = burning();
        // Antes do primeiro quadro, o fogo não tem tamanho: o clique passa.
        app.on_click(30, 5);
        draw(&app, &ledger, 60, 18);
        app.tick(&ledger, now);
        let before = app.fire.clone();
        app.on_click(30, 5);
        assert_ne!(app.fire, before);
        // O clique acende um círculo lá no alto, longe da brasa.
        let row = draw(&app, &ledger, 60, 18)
            .lines()
            .nth(5)
            .unwrap()
            .to_owned();
        assert!(row.contains('▀'), "{row}");
    }

    /// Se a tela tem meio-bloco no canto de baixo à direita, onde mora a
    /// estrela, e em nenhum outro lugar.
    fn only_the_star(screen: &str) -> bool {
        let corner = screen
            .lines()
            .skip(12)
            .any(|line| line.chars().skip(50).any(|c| c == '▀'));
        let elsewhere = screen
            .lines()
            .any(|line| line.chars().take(50).any(|c| c == '▀'));
        corner && !elsewhere
    }

    #[test]
    fn the_lone_star_shines_in_the_corner() {
        // Sem animação não tem fogo, e a estrela fica lá do mesmo jeito.
        let app = new_app(Voice::Game);
        let ledger = ledger();
        let screen = draw(&app, &ledger, 80, 30);
        assert!(only_the_star(&screen), "{screen}");
        // Num terminal pequeno, ela não cabe.
        assert!(!draw(&app, &ledger, 80, 20).contains('▀'));
    }

    #[test]
    fn the_neutral_voice_has_no_star() {
        let app = new_app(Voice::Neutral);
        assert!(!draw(&app, &ledger(), 80, 30).contains('▀'));
    }

    #[test]
    fn the_help_teaches_the_click_only_with_the_fire_on() {
        let (mut app, mut ledger, _) = burning();
        app.on_key(key(KeyCode::Char('?')), &mut ledger);
        assert!(draw(&app, &ledger, 70, 18).contains("atiça o fogo"));
        app.animation = Animation::Off;
        assert!(!draw(&app, &ledger, 70, 18).contains("atiça o fogo"));
    }

    #[test]
    fn the_fire_stops_when_the_window_hides_or_the_animation_is_off() {
        let (mut app, ledger, now) = burning();
        draw(&app, &ledger, 60, 18);
        app.tick(&ledger, now);
        assert!(app.animating(now));

        app.set_window(Window::Hidden);
        assert!(!app.animating(now));
        let before = app.fire.clone();
        app.tick(&ledger, now);
        assert_eq!(app.fire, before);

        app.set_window(Window::Visible);
        app.animation = Animation::Off;
        assert!(!app.animating(now));
        app.tick(&ledger, now);
        assert_eq!(app.fire, before);
        assert!(!draw(&app, &ledger, 60, 18).contains('▀'));
        // Sem animação, o clique também não acende nada.
        app.on_click(30, 5);
        assert_eq!(app.fire, before);
    }

    #[test]
    fn numbers_jump_to_the_tab() {
        let mut app = new_app(Voice::Game);
        let mut ledger = ledger();
        app.on_key(key(KeyCode::Char('3')), &mut ledger);
        assert_eq!(app.tab, Tab::Bench);
        app.on_key(key(KeyCode::Char('9')), &mut ledger);
        assert_eq!(app.tab, Tab::Bench);
    }

    #[test]
    fn help_opens_and_any_key_closes_it() {
        let mut app = new_app(Voice::Game);
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
        let mut app = new_app(Voice::Game);
        app.on_key(key(KeyCode::Char('q')), &mut ledger);
        assert!(app.should_quit());

        let mut app = new_app(Voice::Game);
        app.on_key(
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            &mut ledger,
        );
        assert!(app.should_quit());
    }

    #[test]
    fn the_neutral_voice_renames_the_bench() {
        let app = new_app(Voice::Neutral);
        let screen = draw(&app, &ledger(), 70, 4);
        assert!(screen.contains("3 Tarefas"), "{screen}");
        assert!(!screen.contains("Banco"), "{screen}");
    }

    #[test]
    fn today_shows_the_curiosity_and_r_swaps_in_the_banter() {
        let mut app = new_app(Voice::Game);
        let mut ledger = ledger();
        let screen = draw(&app, &ledger, 60, 14);
        assert!(screen.contains("Neste dia, em 1950"), "{screen}");
        assert_snapshot!("curiosity", screen);

        app.on_key(key(KeyCode::Char('r')), &mut ledger);
        let screen = draw(&app, &ledger, 60, 14);
        assert!(screen.contains("é zoeira, não é fato"), "{screen}");
        assert_snapshot!("banter", screen);
    }

    #[test]
    fn a_tiny_terminal_does_not_panic() {
        let mut app = new_app(Voice::Game);
        app.help = true;
        for (width, height) in [(1, 1), (5, 2), (20, 3)] {
            draw(&app, &ledger(), width, height);
        }
    }
}
