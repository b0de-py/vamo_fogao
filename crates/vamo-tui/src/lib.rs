//! A interface de terminal do `vamo_fogao`.
//!
//! As telas Hoje, Caderno, Banco, Ficha e Museu, o tema com as cores do
//! r4ul.com.br (sem vermelho) e o fogo alvinegro queimando atrás de tudo.
//!
//! A TUI é um adaptador: mostra o estado que o core calcula e transforma
//! tecla em mudança. Ela não abre o banco. Quem entrega as tarefas é quem
//! chama [`run`], por meio da porta [`Ledger`].
//!
//! # Como uma tela é desenhada
//!
//! O estado mora numa struct só (o `App`). A cada tecla, o estado muda; a
//! cada quadro, a tela inteira é desenhada de novo a partir dele, com o
//! [ratatui](https://ratatui.rs). Os testes desenham o mesmo estado num
//! terminal de mentira e comparam com um snapshot.

mod app;
mod bench;
mod card;
mod curiosity;
mod fire;
mod ledger;
mod museum;
mod notebook;
mod prompt;
mod theme;
mod today;
mod voice;
mod when;

use std::io;
use std::time::{Duration, Instant};

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{
    self, DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture, Event,
    KeyEventKind, MouseButton, MouseEventKind,
};
use ratatui::crossterm::execute;

pub use curiosity::Acervo;
pub use ledger::Ledger;
pub use voice::Voice;

/// Se o fogo do fundo anima.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Animation {
    /// O fogo queima, o GOL! pisca e o clique atiça o fogo. É o padrão.
    #[default]
    On,
    /// Sem fogo e sem mouse: a TUI só acorda quando chega uma tecla.
    Off,
}

/// O que a TUI recebe pra começar.
#[derive(Debug, Clone)]
pub struct Setup {
    /// A linguagem da interface.
    pub voice: Voice,
    /// O acervo, pra curiosidade do dia e o Museu.
    pub acervo: Acervo,
    /// O fuso dos horários da súmula. No app, o do sistema.
    pub time_zone: jiff::tz::TimeZone,
    /// Se o fogo anima. Na config, `animacao = false` desliga.
    pub animation: Animation,
}

/// Abre a TUI no terminal e só volta quando quem usa sai.
///
/// Entra na tela alternativa e no modo cru do terminal, e devolve o terminal
/// como estava na saída, inclusive se o programa entrar em pânico: o ratatui
/// troca o gancho de pânico por um que arruma o terminal antes de mostrar a
/// mensagem.
///
/// # Errors
///
/// Quando o terminal não aceita os comandos de entrada e saída (por exemplo,
/// sem um terminal de verdade na saída padrão).
pub fn run<L: Ledger>(ledger: &mut L, setup: Setup) -> io::Result<()> {
    let result = ratatui::try_init().and_then(|mut terminal| {
        let mut app = app::App::new(setup, today());
        listen(&app)?;
        let result = main_loop(&mut terminal, &mut app, ledger);
        // Na saída, desfaz os pedidos mesmo se o laço terminou com erro. Se
        // o terminal não aceitar, o erro que importa é o do laço.
        let _ = execute!(io::stdout(), DisableMouseCapture, DisableFocusChange);
        result
    });
    ratatui::restore();
    result
}

/// Pede ao terminal os avisos que o fogo usa.
///
/// O de foco diz quando a janela some (no Hyprland, quando o workspace
/// especial se esconde), e aí o fogo pausa. Os do mouse fazem o papel que
/// têm no site: mexer o mouse acorda o fogo e clicar atiça. Com o mouse
/// capturado, selecionar texto no terminal pede o shift apertado, então sem
/// animação a TUI não captura nada.
fn listen(app: &app::App) -> io::Result<()> {
    execute!(io::stdout(), EnableFocusChange)?;
    if app.animated() {
        execute!(io::stdout(), EnableMouseCapture)?;
    }
    Ok(())
}

/// O dia de hoje na hora local.
fn today() -> jiff::civil::Date {
    jiff::Zoned::now().date()
}

/// O intervalo entre dois passos do fogo: 25 quadros por segundo. No site
/// são 60, com pixels de 4 px; aqui cada célula do fogo é meia linha de
/// terminal, bem maior, e a chama sobe na tela mais ou menos na mesma
/// velocidade.
const TICK: Duration = Duration::from_millis(40);

/// Desenha, espera um evento, reage. Até quem usa sair.
fn main_loop<L: Ledger>(
    terminal: &mut DefaultTerminal,
    app: &mut app::App,
    ledger: &mut L,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    app.wake(last_tick);
    while !app.should_quit() {
        app.set_today(today());
        terminal.draw(|frame| app.render(frame, ledger))?;
        // Com o fogo parado (sem animação, janela escondida ou ninguém
        // mexendo há 3 segundos), a TUI fica no `read` até chegar um evento:
        // não gasta CPU. Com o fogo queimando, espera no máximo até o
        // próximo passo.
        let ready = if app.animating(Instant::now()) {
            event::poll(TICK.saturating_sub(last_tick.elapsed()))?
        } else {
            true
        };
        if ready {
            let event = event::read()?;
            let now = Instant::now();
            match event {
                // Alguns terminais mandam também o soltar da tecla.
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    app.wake(now);
                    app.on_key(key, ledger);
                }
                Event::Mouse(mouse) => {
                    app.wake(now);
                    if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                        app.on_click(mouse.column, mouse.row);
                    }
                }
                Event::FocusLost => app.set_window(app::Window::Hidden),
                Event::FocusGained => {
                    app.wake(now);
                    app.set_window(app::Window::Visible);
                }
                // No site, mudar o tamanho da janela também reacende o fogo.
                Event::Resize(..) => app.wake(now),
                _ => {}
            }
        }
        let now = Instant::now();
        if app.animating(now) && now.duration_since(last_tick) >= TICK {
            app.tick(ledger, now);
            last_tick = now;
        }
    }
    Ok(())
}
