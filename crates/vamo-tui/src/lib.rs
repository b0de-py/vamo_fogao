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
mod curiosity;
mod ledger;
mod theme;
mod today;
mod voice;

use std::io;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

pub use curiosity::Acervo;
pub use ledger::Ledger;
pub use voice::Voice;

/// O que a TUI recebe pra começar.
#[derive(Debug, Clone, Default)]
pub struct Setup {
    /// A linguagem da interface.
    pub voice: Voice,
    /// O acervo, pra curiosidade do dia e o Museu.
    pub acervo: Acervo,
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

/// Desenha, espera uma tecla, reage. Até quem usa sair.
fn main_loop<L: Ledger>(
    terminal: &mut DefaultTerminal,
    ledger: &mut L,
    setup: Setup,
) -> io::Result<()> {
    let mut app = app::App::new(setup, today());
    while !app.should_quit() {
        app.set_today(today());
        terminal.draw(|frame| app.render(frame, ledger))?;
        // Sem animação, a TUI fica parada aqui até chegar um evento: não
        // gasta CPU enquanto ninguém mexe.
        if let Event::Key(key) = event::read()?
            // Alguns terminais mandam também o soltar da tecla.
            && key.kind == KeyEventKind::Press
        {
            app.on_key(key, ledger);
        }
    }
    Ok(())
}
