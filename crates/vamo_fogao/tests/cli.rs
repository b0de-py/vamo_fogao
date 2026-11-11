//! Testes de ponta a ponta do executável `vamo`: o Cargo compila o binário de
//! verdade e cada teste roda ele num processo separado, com um banco numa
//! pasta temporária.

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

/// Um `vamo` com a casa numa pasta temporária, que some no fim do teste.
struct Vamo {
    home: TempDir,
}

impl Vamo {
    fn new() -> Self {
        Self {
            home: tempfile::tempdir().expect("o sistema deveria criar uma pasta temporária"),
        }
    }

    /// Roda o `vamo` com os argumentos dados, num ambiente controlado: a
    /// pasta temporária é a casa e o diretório atual, o banco fica nela, a
    /// máquina se chama `notebook` e o fuso é o de Brasília.
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_vamo"))
            .args(args)
            .current_dir(self.home.path())
            .env("HOME", self.home.path())
            .env("XDG_DATA_HOME", self.home.path().join("dados"))
            .env("VAMO_MAQUINA", "notebook")
            .env("TZ", "America/Sao_Paulo")
            .env_remove("VAMO_LINGUAGEM")
            .env_remove("VAMO_LOG")
            .env_remove("RUST_BACKTRACE")
            .env_remove("RUST_LIB_BACKTRACE")
            .output()
            .expect("o executável `vamo` deveria rodar")
    }

    /// Roda e confere que deu certo, devolvendo a saída padrão.
    fn ok(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(
            out.status.success(),
            "`vamo {}` falhou: {}",
            args.join(" "),
            text(&out.stderr)
        );
        text(&out.stdout)
    }

    /// Roda e confere que falhou com o código dado, devolvendo a saída de
    /// erro.
    fn fails(&self, args: &[&str], code: i32) -> String {
        let out = self.run(args);
        assert_eq!(out.status.code(), Some(code), "`vamo {}`", args.join(" "));
        assert_eq!(text(&out.stdout), "");
        text(&out.stderr)
    }

    fn database(&self) -> std::path::PathBuf {
        self.home.path().join("dados/vamo_fogao/vamo.db")
    }
}

/// Converte a saída de um processo em texto.
fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("a saída do `vamo` deveria ser UTF-8")
}

/// Confere um snapshot trocando data e hora, que mudam a cada execução,
/// por um marcador.
macro_rules! assert_snapshot_without_time {
    ($value:expr) => {
        insta::with_settings!({
            filters => vec![(r"\d{2}/\d{2}/\d{4} \d{2}:\d{2}", "[data e hora]")],
        }, {
            insta::assert_snapshot!($value);
        });
    };
}

mod help_and_version {
    use super::*;

    #[test]
    fn help_is_shown_without_arguments() {
        insta::assert_snapshot!(Vamo::new().ok(&[]));
    }

    #[test]
    fn help_flags_print_the_same_text() {
        let vamo = Vamo::new();
        let without_args = vamo.ok(&[]);

        assert_eq!(vamo.ok(&["--help"]), without_args);
        assert_eq!(vamo.ok(&["-h"]), without_args);
    }

    #[test]
    fn each_command_has_its_own_help() {
        insta::assert_snapshot!(Vamo::new().ok(&["add", "--help"]));
    }

    #[test]
    fn version_prints_the_package_version() {
        assert_eq!(
            Vamo::new().ok(&["--version"]),
            format!("vamo {}\n", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn help_does_not_create_the_database() {
        let vamo = Vamo::new();
        vamo.ok(&["--help"]);
        assert!(!vamo.database().exists());
    }
}

mod usage_errors {
    use super::*;

    #[test]
    fn an_unknown_command_is_a_usage_error() {
        assert_eq!(
            Vamo::new().fails(&["chute"], 2),
            "vamo: o comando `chute` não existe\n\nPra ver os comandos: vamo --help\n"
        );
    }

    #[test]
    fn a_typo_gets_a_suggestion() {
        assert!(
            Vamo::new()
                .fails(&["lsit"], 2)
                .starts_with("vamo: o comando `lsit` não existe: não seria `list`?\n")
        );
    }

    #[test]
    fn an_unknown_option_is_a_usage_error() {
        assert!(
            Vamo::new()
                .fails(&["list", "--chute"], 2)
                .starts_with("vamo: não entendi o `--chute`\n")
        );
    }

    #[test]
    fn a_bad_value_brings_the_domain_explanation() {
        assert!(
            Vamo::new()
                .fails(&["add", "Ler", "--area", "Casa"], 2)
                .starts_with("vamo: `Casa` não serve como área: use letras minúsculas")
        );
    }

    #[test]
    fn a_bad_task_id_is_a_usage_error() {
        assert!(
            Vamo::new()
                .fails(&["done", "x"], 2)
                .starts_with("vamo: `x` não é um ID de tarefa")
        );
    }

    #[test]
    fn a_bad_voice_lists_the_good_ones() {
        assert!(
            Vamo::new()
                .fails(&["list", "--linguagem", "chute"], 2)
                .contains("os valores aceitos são jogo, neutra")
        );
    }
}

mod add {
    use super::*;

    #[test]
    fn a_new_task_gets_the_next_id() {
        let vamo = Vamo::new();

        assert_eq!(
            vamo.ok(&["add", "Trocar", "o", "filtro", "da", "água"]),
            "T-001 entrou no banco: Trocar o filtro da água\n"
        );
        assert_eq!(
            vamo.ok(&["add", "Ligar pro dentista"]),
            "T-002 entrou no banco: Ligar pro dentista\n"
        );
    }

    #[test]
    fn the_database_goes_to_the_xdg_data_folder() {
        let vamo = Vamo::new();
        vamo.ok(&["add", "Ler o livro"]);
        assert!(vamo.database().exists());
    }

    #[test]
    fn a_title_without_a_verb_is_refused() {
        let vamo = Vamo::new();

        let stderr = vamo.fails(&["add", "Reunião", "com", "o", "pessoal"], 1);

        assert_eq!(
            stderr,
            "Deu ruim: \n   0: o título começa com `Reunião`, e no caderno a tarefa começa com \
             verbo no infinitivo, como Ligar, Comprar ou Pôr\n"
        );
        assert_eq!(
            vamo.ok(&["list"]),
            "Ninguém no banco. Pra contratar: vamo add <título>\n"
        );
    }

    #[test]
    fn a_long_title_comes_back_with_a_suggestion() {
        let stderr = Vamo::new().fails(&["add", "Comprar ração e areia para o gato"], 1);
        assert!(stderr.contains("que tal \"Comprar ração e areia\"? O resto vai pras notas"));
    }
}

mod list {
    use super::*;

    fn with_three_tasks() -> Vamo {
        let vamo = Vamo::new();
        vamo.ok(&["add", "Trocar o filtro da água", "-a", "casa"]);
        vamo.ok(&["add", "Ligar pro dentista", "-a", "saúde", "-e", "foco"]);
        vamo.ok(&["add", "Ler o livro", "-a", "estudos", "-s", "ideia"]);
        vamo.ok(&["done", "1"]);
        vamo
    }

    #[test]
    fn an_empty_bench_says_how_to_start() {
        assert_eq!(
            Vamo::new().ok(&["list"]),
            "Ninguém no banco. Pra contratar: vamo add <título>\n"
        );
    }

    #[test]
    fn lists_the_tasks_in_play_in_columns() {
        insta::assert_snapshot!(with_three_tasks().ok(&["list"]));
    }

    #[test]
    fn todas_brings_the_finished_ones_too() {
        insta::assert_snapshot!(with_three_tasks().ok(&["list", "--todas"]));
    }

    #[test]
    fn filters_by_area_and_status() {
        let vamo = with_three_tasks();

        assert_eq!(
            vamo.ok(&["list", "--area", "saúde"]),
            "T-002  pronta  foco  saúde  Ligar pro dentista\n"
        );
        assert_eq!(
            vamo.ok(&["list", "--status", "concluída"]),
            "T-001  concluída  brecha  casa  Trocar o filtro da água\n"
        );
    }
}

mod done {
    use super::*;

    #[test]
    fn finishing_a_task_is_a_goal() {
        let vamo = Vamo::new();
        vamo.ok(&["add", "Trocar o filtro da água"]);

        assert_eq!(
            vamo.ok(&["done", "T-001"]),
            "GOOOL! T-001 Trocar o filtro da água\n"
        );
    }

    #[test]
    fn the_id_can_be_typed_short() {
        let vamo = Vamo::new();
        vamo.ok(&["add", "Ler o livro"]);
        assert!(vamo.ok(&["done", "t-1"]).starts_with("GOOOL! T-001"));
    }

    #[test]
    fn the_status_rule_still_holds() {
        let vamo = Vamo::new();
        vamo.ok(&["add", "Ler o livro", "--status", "ideia"]);

        assert_eq!(
            vamo.fails(&["done", "1"], 1),
            "Deu ruim: \n   0: não dá pra passar de ideia pra concluída\n"
        );
    }

    #[test]
    fn an_unknown_task_is_an_error() {
        assert_eq!(
            Vamo::new().fails(&["done", "99"], 1),
            "Deu ruim: \n   0: não tem nenhuma tarefa T-099\n"
        );
    }
}

mod show {
    use super::*;

    #[test]
    fn shows_the_task_and_its_history() {
        let vamo = Vamo::new();
        vamo.ok(&[
            "add",
            "Trocar o filtro da água",
            "-a",
            "casa",
            "-n",
            "O refil está na gaveta.",
        ]);
        vamo.ok(&["done", "1"]);

        assert_snapshot_without_time!(vamo.ok(&["show", "1"]));
    }

    #[test]
    fn an_unknown_task_is_an_error() {
        assert_eq!(
            Vamo::new().fails(&["show", "T-007"], 1),
            "Deu ruim: \n   0: não tem nenhuma tarefa T-007\n"
        );
    }
}

mod neutral_voice {
    use super::*;

    #[test]
    fn the_flag_switches_the_messages() {
        let vamo = Vamo::new();

        assert_eq!(
            vamo.ok(&["--linguagem", "neutra", "add", "Ler o livro"]),
            "Tarefa criada: T-001 Ler o livro\n"
        );
        assert_eq!(
            vamo.ok(&["done", "1", "--linguagem", "neutra"]),
            "Concluída: T-001 Ler o livro\n"
        );
    }

    #[test]
    fn the_environment_variable_does_the_same() {
        let vamo = Vamo::new();
        let out = Command::new(env!("CARGO_BIN_EXE_vamo"))
            .args(["list"])
            .env("HOME", vamo.home.path())
            .env("XDG_DATA_HOME", vamo.home.path().join("dados"))
            .env("VAMO_LINGUAGEM", "neutra")
            .output()
            .expect("o executável `vamo` deveria rodar");

        assert_eq!(
            text(&out.stdout),
            "Nenhuma tarefa. Pra criar uma: vamo add <título>\n"
        );
    }

    #[test]
    fn errors_change_too() {
        assert!(
            Vamo::new()
                .fails(&["--linguagem", "neutra", "show", "1"], 1)
                .starts_with("Erro: \n")
        );
    }
}

mod import {
    use super::*;

    const EMPTY: &str = "Ninguém no banco. Pra contratar: vamo add <título>\n";

    /// O banco antigo de exemplo, com dados inventados.
    fn fixture() -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/banco-antigo.md")
            .to_string_lossy()
            .into_owned()
    }

    /// Uma cópia do exemplo sem os itens pra decidir, como ficaria depois
    /// de a pessoa ajustar o arquivo.
    fn fixture_without_pending(vamo: &Vamo) -> String {
        let pending = ["| T-004 ", "| T-006 ", "| T-007 ", "| T-009 "];
        let text = std::fs::read_to_string(fixture()).expect("o exemplo existe");
        let ready = text
            .lines()
            .filter(|line| !pending.iter().any(|id| line.starts_with(id)))
            .collect::<Vec<_>>()
            .join("\n");
        let path = vamo.home.path().join("banco-ajustado.md");
        std::fs::write(&path, ready).expect("a pasta temporária aceita escrita");
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn the_dry_run_shows_the_triage_and_writes_nothing() {
        let vamo = Vamo::new();

        insta::assert_snapshot!(vamo.ok(&["import", "--dry-run", &fixture()]));
        assert_eq!(vamo.ok(&["list", "--todas"]), EMPTY);
    }

    #[test]
    fn items_to_decide_block_the_import() {
        let vamo = Vamo::new();

        let stderr = vamo.fails(&["import", &fixture()], 1);

        assert!(stderr.contains("ainda tem 4 itens pra decidir"), "{stderr}");
        assert_eq!(vamo.ok(&["list", "--todas"]), EMPTY);
    }

    #[test]
    fn the_import_keeps_the_old_ids() {
        let vamo = Vamo::new();
        let file = fixture_without_pending(&vamo);

        insta::assert_snapshot!("import_report", vamo.ok(&["import", &file]));
        insta::assert_snapshot!("list_after_import", vamo.ok(&["list", "--todas"]));
        // A próxima tarefa vem depois do maior ID importado.
        assert!(vamo.ok(&["add", "Ler o livro"]).starts_with("T-006 "));
    }

    #[test]
    fn what_did_not_fit_goes_to_the_notes() {
        let vamo = Vamo::new();
        vamo.ok(&["import", &fixture_without_pending(&vamo)]);

        assert_snapshot_without_time!(vamo.ok(&["show", "2"]));
    }

    #[test]
    fn importing_twice_changes_nothing() {
        let vamo = Vamo::new();
        let file = fixture_without_pending(&vamo);
        vamo.ok(&["import", &file]);

        let second = vamo.ok(&["import", &file]);

        assert!(second.contains("Já estavam no banco (4):"), "{second}");
        assert!(second.ends_with("Nenhum reforço entrou no banco.\n"));
    }

    #[test]
    fn a_file_that_does_not_exist_is_an_error() {
        let stderr = Vamo::new().fails(&["import", "nao-existe.md"], 1);
        assert!(stderr.contains("não deu pra ler o banco antigo em nao-existe.md"));
    }
}

mod acervo {
    use super::*;

    /// Um fato verificado, com duas fontes de domínios diferentes.
    const VERIFIED: &str = r#"texto = "Um fato verificado."
status = "verificado"

[[fontes]]
nome = "Lance!"
url = "https://www.lance.com.br/botafogo"

[[fontes]]
nome = "Trivela"
url = "https://trivela.com.br/botafogo"
"#;

    /// Um fato a conferir, com uma fonte só.
    const TO_CHECK: &str = r#"texto = "Um fato a conferir."
status = "a-conferir"

[[fontes]]
nome = "Wikipédia"
url = "https://pt.wikipedia.org/wiki/Botafogo_de_Futebol_e_Regatas"
"#;

    const BANTER: &str = "texto = \"Uma zoeira absurda de cara.\"\n";

    impl Vamo {
        /// Monta um acervo na pasta `acervo` do diretório atual do `vamo`,
        /// com os arquivos dados (o caminho de cada um conta a partir de
        /// `acervo`).
        fn acervo(&self, files: &[(&str, &str)]) {
            let root = self.home.path().join("acervo");
            for folder in ["fatos", "resenha"] {
                std::fs::create_dir_all(root.join(folder))
                    .expect("a pasta temporária aceita subpastas");
            }
            for (path, text) in files {
                std::fs::write(root.join(path), text).expect("a pasta temporária aceita escrita");
            }
        }
    }

    #[test]
    fn counts_what_passed() {
        let vamo = Vamo::new();
        vamo.acervo(&[
            ("fatos/um.toml", VERIFIED),
            ("fatos/dois.toml", VERIFIED),
            ("fatos/tres.toml", TO_CHECK),
            ("resenha/zoeira.toml", BANTER),
        ]);

        assert_eq!(
            vamo.ok(&["acervo", "validar"]),
            "O VAR conferiu o acervo: 2 fatos verificados, 1 a conferir e 1 da resenha.\n"
        );
        assert_eq!(
            vamo.ok(&["--linguagem", "neutra", "acervo", "validar"]),
            "Acervo validado: 2 fatos verificados, 1 a conferir e 1 da resenha.\n"
        );
    }

    #[test]
    fn a_verified_fact_with_a_single_source_fails() {
        let vamo = Vamo::new();
        let single = TO_CHECK.replace("a-conferir", "verificado");
        vamo.acervo(&[("fatos/uma-fonte.toml", &single)]);

        let stderr = vamo.fails(&["acervo", "validar"], 1);

        assert!(
            stderr.contains("acervo/fatos/uma-fonte.toml: o fato está verificado com uma fonte só"),
            "{stderr}"
        );
    }

    #[test]
    fn a_record_without_a_date_fails() {
        let vamo = Vamo::new();
        let record = VERIFIED.replace("status", "tipo = \"recorde\"\nstatus");
        vamo.acervo(&[("fatos/recorde.toml", &record)]);

        let stderr = vamo.fails(&["acervo", "validar"], 1);

        assert!(
            stderr
                .contains("acervo/fatos/recorde.toml: recorde precisa de data, porque recorde cai"),
            "{stderr}"
        );
    }

    #[test]
    fn every_problem_is_listed_with_its_file() {
        let vamo = Vamo::new();
        let bad_date = VERIFIED.replace("status", "data = \"30/05/1909\"\nstatus");
        vamo.acervo(&[
            ("fatos/bom.toml", VERIFIED),
            ("fatos/data-errada.toml", &bad_date),
            ("fatos/Maiuscula.toml", VERIFIED),
            ("fatos/LEIA.md", "nada"),
            ("fatos/.gitkeep", ""),
            (
                "resenha/com-fonte.toml",
                "texto = \"Zoeira.\"\nfontes = []\n",
            ),
        ]);

        insta::assert_snapshot!(vamo.fails(&["acervo", "validar"], 1));
    }

    #[test]
    fn a_missing_folder_is_an_error() {
        let stderr = Vamo::new().fails(&["acervo", "validar", "nao-existe"], 1);
        assert!(
            stderr.contains("não deu pra ler a pasta nao-existe/fatos"),
            "{stderr}"
        );
    }

    #[test]
    fn checking_the_acervo_does_not_touch_the_database() {
        let vamo = Vamo::new();
        vamo.acervo(&[("fatos/um.toml", VERIFIED)]);

        vamo.ok(&["acervo", "validar"]);

        assert!(!vamo.database().exists());
    }

    #[test]
    fn the_group_without_a_command_shows_its_help() {
        let out = Vamo::new().run(&["acervo"]);

        assert_eq!(out.status.code(), Some(2));
        assert_eq!(text(&out.stdout), "");
        insta::assert_snapshot!(text(&out.stderr));
    }
}

/// O banco fica onde o XDG manda, e não no diretório atual.
#[test]
fn nothing_is_written_outside_the_data_folder() {
    let vamo = Vamo::new();
    vamo.ok(&["add", "Ler o livro"]);

    let entries: Vec<String> = std::fs::read_dir(vamo.home.path())
        .expect("a pasta temporária existe")
        .map(|entry| {
            entry
                .expect("a entrada da pasta deveria ser lida")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(entries, ["dados"]);
    assert!(Path::new(&vamo.database()).exists());
}
