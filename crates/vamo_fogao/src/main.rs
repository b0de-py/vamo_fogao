//! O executável `vamo`, a porta de entrada do `vamo_fogao`.
//!
//! É a cola da arquitetura hexagonal: lê a linha de comando e o ambiente
//! (caminhos, relógio, nome da máquina), abre o banco e chama o core. Regra
//! de negócio não mora aqui.

mod cli;
mod commands;
mod messages;
mod paths;

use std::io::IsTerminal;
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use color_eyre::config::{HookBuilder, Theme};
use color_eyre::owo_colors::style;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

use crate::cli::Cli;

/// Código de saída pra uso errado da linha de comando, o mesmo do clap.
const EXIT_USAGE: u8 = 2;

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        // `--help` e `--version` chegam como "erro" do clap, mas não são:
        // ele mostra o texto e sai com sucesso.
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            error.exit()
        }
        Err(error) => {
            eprintln!(
                "vamo: {}\n\nPra ver os comandos: vamo --help",
                cli::usage_error(&error)
            );
            return ExitCode::from(EXIT_USAGE);
        }
    };

    init_logs();
    install_error_report();

    let Some(command) = cli.command else {
        print!("{}", Cli::command().render_help());
        return ExitCode::SUCCESS;
    };

    match commands::run(command, cli.voice) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(report) => {
            eprintln!("{} {report:?}", cli.voice.error());
            ExitCode::FAILURE
        }
    }
}

/// Liga os logs na saída de erro. O nível vem da variável `VAMO_LOG` (como
/// `debug`); sem ela, só aparecem os avisos.
fn init_logs() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::WARN.into())
        .with_env_var("VAMO_LOG")
        .from_env_lossy();
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .without_time()
        .init();
}

/// Liga o relatório de erro do color-eyre, sem as seções de localização no
/// código e de variáveis de ambiente, que são pra quem programa.
fn install_error_report() {
    let theme = if std::io::stderr().is_terminal() {
        // O tema padrão do color-eyre pinta erro de vermelho, e aqui vermelho
        // não entra: erro vai no amarelo plasma, como no resto do app.
        Theme::new()
            .error(style().bright_yellow())
            .help_info_error(style().bright_yellow())
            .help_info_warning(style().bright_yellow())
            .help_info_note(style().bright_cyan())
            .help_info_suggestion(style().bright_cyan())
            .panic_header(style().bright_yellow())
            .panic_message(style().bright_cyan())
    } else {
        // Sem terminal (num pipe ou num teste), sem cor nenhuma.
        Theme::new()
    };
    let installed = HookBuilder::blank()
        .display_location_section(false)
        .display_env_section(false)
        .theme(theme)
        .install();
    if let Err(error) = installed {
        tracing::warn!(%error, "o relatório de erro do color-eyre não ligou");
    }
}
