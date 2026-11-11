//! O executável `vamo`, a porta de entrada do `vamo_fogao`.
//!
//! Por enquanto ele só responde a `--help` e `--version`. Os comandos de
//! tarefa chegam na fase 1, com o clap cuidando dos argumentos; até lá, um
//! `match` simples dá conta e deixa a fundação sem nenhuma dependência.

use std::ffi::OsString;
use std::process::ExitCode;

/// O texto de `vamo --help`.
const HELP: &str = "\
vamo, o app de tarefas do vamo_fogao

Uso: vamo [OPÇÃO]

Opções:
  -h, --help     mostra esta ajuda
  -V, --version  mostra a versão

Os comandos de tarefa chegam na fase 1:
https://github.com/b0de-py/vamo_fogao/blob/main/ROADMAP.md
";

/// Código de saída pra uso errado da linha de comando. É o mesmo que o clap
/// usa, então nada muda pra quem chama o `vamo` quando ele chegar.
const EXIT_USAGE: u8 = 2;

fn main() -> ExitCode {
    // `args_os` em vez de `args`: um argumento que não é UTF-8 válido vira
    // erro de uso em vez de pânico.
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();

    match args.as_slice() {
        [] => print_help(),
        [arg] => match arg.to_str() {
            Some("-h" | "--help") => print_help(),
            Some("-V" | "--version") => {
                println!("vamo {}", env!("CARGO_PKG_VERSION"));
                ExitCode::SUCCESS
            }
            _ => usage_error(&format!("opção desconhecida: {}", arg.to_string_lossy())),
        },
        _ => usage_error("argumentos demais"),
    }
}

/// Mostra a ajuda na saída padrão e termina com sucesso.
fn print_help() -> ExitCode {
    print!("{HELP}");
    ExitCode::SUCCESS
}

/// Explica o erro de uso na saída de erro, mostra a ajuda e devolve o código
/// de saída de uso errado.
fn usage_error(message: &str) -> ExitCode {
    eprintln!("vamo: {message}\n");
    eprint!("{HELP}");
    ExitCode::from(EXIT_USAGE)
}
