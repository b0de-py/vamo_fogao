//! Embute o acervo no binário.
//!
//! Os fatos e a resenha moram em `acervo/`, na raiz do repositório. Quem
//! instala o `vamo` não tem essa pasta, então na compilação cada arquivo
//! `.toml` vira um texto dentro do executável (com `include_str!`), e a
//! curiosidade do dia funciona em qualquer lugar.

use std::fmt::Write as _;
use std::path::Path;
use std::{env, fs};

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../acervo");
    println!("cargo::rerun-if-changed={}", root.display());

    let mut code = String::from("// Gerado pelo build.rs: o acervo embutido.\n\n");
    for (constant, folder) in [("FACTS", "fatos"), ("BANTER", "resenha")] {
        let dir = root.join(folder);
        println!("cargo::rerun-if-changed={}", dir.display());
        let mut entries: Vec<(String, String)> = fs::read_dir(&dir)
            .unwrap_or_else(|error| panic!("não deu pra ler {}: {error}", dir.display()))
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
            .filter_map(|path| {
                let id = path.file_stem()?.to_str()?.to_owned();
                let full = path.canonicalize().ok()?.to_str()?.to_owned();
                Some((id, full))
            })
            .collect();
        // A mesma ordem em qualquer máquina.
        entries.sort();
        writeln!(code, "pub(crate) const {constant}: &[(&str, &str)] = &[")
            .expect("escrever numa String não falha");
        for (id, path) in entries {
            writeln!(code, "    ({id:?}, include_str!({path:?})),")
                .expect("escrever numa String não falha");
        }
        code.push_str("];\n\n");
    }

    let out =
        Path::new(&env::var("OUT_DIR").expect("o Cargo sempre define o OUT_DIR")).join("acervo.rs");
    fs::write(&out, code)
        .unwrap_or_else(|error| panic!("não deu pra escrever {}: {error}", out.display()));
}
