//! O contrato do formato de exportação.
//!
//! O schema de cada tipo fica guardado em `schema/v{VERSAO}/`. Se um tipo
//! mudar, o schema gerado deixa de bater com o guardado e este teste quebra:
//! pra mudar o formato, suba a `VERSION` e gere a pasta da versão nova com
//!
//! ```sh
//! VAMO_GRAVAR_SCHEMA=1 cargo test -p vamo-sync --test schema
//! ```
//!
//! O gerador só escreve arquivo que ainda não existe, então a pasta de uma
//! versão publicada nunca muda sem alguém apagar um arquivo de propósito, e
//! isso aparece no PR.

use std::path::{Path, PathBuf};

use vamo_core::Event;
use vamo_sync::format::{self, EventLine, MarkLine, ReinforcementLine, VERSION};

fn schema_dir(version: u32) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("schema")
        .join(format!("v{version}"))
}

#[test]
fn the_schema_of_each_type_matches_the_saved_one() {
    let dir = schema_dir(VERSION);
    let write = std::env::var_os("VAMO_GRAVAR_SCHEMA").is_some();
    for (name, schema) in format::schemas() {
        let generated = serde_json::to_string_pretty(&schema).unwrap() + "\n";
        let path = dir.join(name);
        if write && !path.exists() {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(&path, &generated).unwrap();
        }
        let saved = std::fs::read_to_string(&path).unwrap_or_else(|_| {
            panic!(
                "falta o schema {}: gere com VAMO_GRAVAR_SCHEMA=1 cargo test -p vamo-sync --test \
                 schema",
                path.display()
            )
        });
        assert!(
            saved == generated,
            "o schema de {name} mudou sem a versão subir: o formato é um contrato. Suba a \
             VERSION em crates/vamo-sync/src/format.rs pra {} e gere schema/v{} com \
             VAMO_GRAVAR_SCHEMA=1 (a leitura da versão {VERSION} continua valendo)",
            VERSION + 1,
            VERSION + 1,
        );
    }
}

/// Versão nova lê export antigo: os exemplos de cada versão, de 1 até a
/// atual, continuam sendo lidos.
#[test]
fn every_old_export_is_still_read() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for version in 1..=VERSION {
        let dir = fixtures.join(format!("v{version}"));
        let events = std::fs::read_to_string(dir.join("eventos.jsonl")).unwrap();
        let mut count = 0;
        for line in events.lines() {
            let line: EventLine = serde_json::from_str(line).unwrap();
            Event::try_from(line).unwrap();
            count += 1;
        }
        assert!(count > 0, "o exemplo da versão {version} está vazio");

        let marks = std::fs::read_to_string(dir.join("marcacoes.jsonl")).unwrap();
        for line in marks.lines() {
            let line: MarkLine = serde_json::from_str(line).unwrap();
            format::check_version(line.versao).unwrap();
        }

        let reinforcements = std::fs::read_to_string(dir.join("reforcos.jsonl")).unwrap();
        for line in reinforcements.lines() {
            let line: ReinforcementLine = serde_json::from_str(line).unwrap();
            format::check_version(line.versao).unwrap();
        }
    }
}
