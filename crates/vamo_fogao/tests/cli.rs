//! Testes de ponta a ponta do executável `vamo`: o Cargo compila o binário de
//! verdade e cada teste roda ele num processo separado.

use std::process::{Command, Output};

/// Roda o `vamo` compilado pelo Cargo com os argumentos dados.
fn vamo(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vamo"))
        .args(args)
        .output()
        .expect("o executável `vamo` deveria rodar")
}

/// Converte a saída de um processo em texto.
fn text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).expect("a saída do `vamo` deveria ser UTF-8")
}

#[test]
fn version_prints_the_package_version() {
    let out = vamo(&["--version"]);

    assert!(out.status.success());
    assert_eq!(
        text(out.stdout),
        format!("vamo {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn short_version_flag_prints_the_same() {
    assert_eq!(vamo(&["-V"]).stdout, vamo(&["--version"]).stdout);
}

#[test]
fn help_is_shown_without_arguments() {
    let out = vamo(&[]);

    assert!(out.status.success());
    assert!(text(out.stdout).starts_with("vamo, o app de tarefas"));
}

#[test]
fn help_flags_print_the_same_text() {
    let without_args = vamo(&[]).stdout;

    assert_eq!(vamo(&["--help"]).stdout, without_args);
    assert_eq!(vamo(&["-h"]).stdout, without_args);
}

#[test]
fn unknown_option_is_a_usage_error() {
    let out = vamo(&["--chute"]);

    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(text(out.stderr).starts_with("vamo: opção desconhecida: --chute\n"));
}

#[test]
fn extra_arguments_are_a_usage_error() {
    let out = vamo(&["--version", "--help"]);

    assert_eq!(out.status.code(), Some(2));
    assert!(text(out.stderr).starts_with("vamo: argumentos demais\n"));
}
