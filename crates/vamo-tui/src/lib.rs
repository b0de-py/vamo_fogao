//! A interface de terminal do `vamo_fogao`.
//!
//! As telas Hoje, Caderno, Banco, Ficha e Museu, o tema com as cores do
//! r4ul.com.br (sem vermelho) e o fogo alvinegro no rodapé.
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
use ratatui::crossterm::event::{self, DisableFocusChange, EnableFocusChange, Event, KeyEventKind};
use ratatui::crossterm::execute;

pub use curiosity::Acervo;
pub use ledger::Ledger;
pub use voice::Voice;

/// Se o fogo do rodapé anima.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Animation {
    /// O fogo queima e o GOL! pisca. É o padrão.
    #[default]
    On,
    /// Sem fogo: a TUI só acorda quando chega uma tecla.
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
    let result =
        ratatui::try_init().and_then(|mut terminal| main_loop(&mut terminal, ledger, setup));
    ratatui::restore();
    result
}

/// O dia de hoje na hora local.
fn today() -> jiff::civil::Date {
    jiff::Zoned::now().date()
}

/// O intervalo entre dois passos do fogo: uns 12 quadros por segundo, o
/// bastante pra parecer fogo sem pesar.
const TICK: Duration = Duration::from_millis(80);

/// Desenha, espera um evento, reage. Até quem usa sair.
fn main_loop<L: Ledger>(
    terminal: &mut DefaultTerminal,
    ledger: &mut L,
    setup: Setup,
) -> io::Result<()> {
    // Pede ao terminal pra avisar quando a janela ganha e perde o foco: é
    // assim que o fogo sabe que a janela sumiu (no Hyprland, quando o
    // workspace especial se esconde).
    execute!(io::stdout(), EnableFocusChange)?;
    let mut app = app::App::new(setup, today());
    let mut last_tick = Instant::now();
    while !app.should_quit() {
        app.set_today(today());
        terminal.draw(|frame| app.render(frame, ledger))?;
        // Sem animação, ou com a janela escondida, a TUI fica parada no
        // `read` até chegar um evento: não gasta CPU enquanto ninguém mexe.
        // Com o fogo queimando, espera no máximo até o próximo passo.
        let ready = if app.animating() {
            event::poll(TICK.saturating_sub(last_tick.elapsed()))?
        } else {
            true
        };
        if ready {
            match event::read()? {
                // Alguns terminais mandam também o soltar da tecla.
                Event::Key(key) if key.kind == KeyEventKind::Press => app.on_key(key, ledger),
                Event::FocusLost => app.set_window(app::Window::Hidden),
                Event::FocusGained => app.set_window(app::Window::Visible),
                _ => {}
            }
        }
        if app.animating() && last_tick.elapsed() >= TICK {
            app.tick(ledger);
            last_tick = Instant::now();
        }
    }
    execute!(io::stdout(), DisableFocusChange)?;
    Ok(())
}
