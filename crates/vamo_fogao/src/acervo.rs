//! `vamo acervo validar`: passa cada arquivo do acervo pela régua do
//! `vamo-acervo`. É o mesmo comando que a CI roda em todo PR.
//!
//! A régua mora no `vamo-acervo`; aqui só se lê a pasta e se junta o
//! relatório.

use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, WrapErr, bail};
use vamo_acervo::{Banter, Error, Fact};

use crate::messages::Voice;

/// Confere o acervo da pasta e devolve o resumo. Se algum arquivo não passa,
/// o erro lista todos, cada um com o seu problema.
pub(crate) fn validate(folder: &Path, voice: Voice) -> Result<String> {
    let mut problems = Vec::new();

    let (mut verified, mut to_check) = (0, 0);
    for entry in read_folder(&folder.join("fatos"), &mut problems)? {
        match Fact::from_toml(&entry.id, &entry.toml) {
            Ok(fact) if fact.is_verified() => verified += 1,
            Ok(_) => to_check += 1,
            Err(error) => problems.push(Problem::from_error(entry.path, &error)),
        }
    }

    let mut banter = 0;
    for entry in read_folder(&folder.join("resenha"), &mut problems)? {
        match Banter::from_toml(&entry.id, &entry.toml) {
            Ok(_) => banter += 1,
            Err(error) => problems.push(Problem::from_error(entry.path, &error)),
        }
    }

    if !problems.is_empty() {
        problems.sort_by(|a, b| a.path.cmp(&b.path));
        let files = match problems.len() {
            1 => "1 arquivo".to_owned(),
            n => format!("{n} arquivos"),
        };
        let list: Vec<String> = problems
            .iter()
            .map(|problem| format!("  {}", problem.text))
            .collect();
        bail!("o acervo tem {files} com problema:\n{}", list.join("\n"));
    }
    Ok(voice.acervo_checked(verified, to_check, banter))
}

/// Um arquivo `.toml` de uma pasta do acervo.
struct Entry {
    path: PathBuf,
    /// O nome do arquivo sem o `.toml`, que é a id do verbete.
    id: String,
    toml: String,
}

/// Um arquivo que não passou, com a frase que diz por quê.
struct Problem {
    path: PathBuf,
    text: String,
}

impl Problem {
    fn new(path: PathBuf, message: &str) -> Self {
        let text = format!("{}: {message}", path.display());
        Self { path, text }
    }

    /// O problema que a régua achou. Com a linha, sai no formato
    /// `arquivo:linha`, que o editor e o terminal sabem abrir.
    fn from_error(path: PathBuf, error: &Error) -> Self {
        let text = match error {
            Error::At { line, error } => format!("{}:{line}: {error}", path.display()),
            error => format!("{}: {error}", path.display()),
        };
        Self { path, text }
    }
}

/// Os arquivos `.toml` de uma pasta do acervo, em ordem de nome. O que não é
/// verbete (outro tipo de arquivo, subpasta, nome que não é UTF-8) entra na
/// lista de problemas, pra aparecer junto com os outros. Arquivo escondido,
/// como o `.gitkeep`, passa batido.
fn read_folder(folder: &Path, problems: &mut Vec<Problem>) -> Result<Vec<Entry>> {
    let cannot_read = || format!("não deu pra ler a pasta {}", folder.display());
    let mut entries = Vec::new();
    for item in std::fs::read_dir(folder).wrap_err_with(cannot_read)? {
        let path = item.wrap_err_with(cannot_read)?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            problems.push(Problem::new(path, "o nome do arquivo não é UTF-8"));
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            problems.push(Problem::new(
                path,
                "subpasta não entra no acervo: cada verbete é um arquivo .toml",
            ));
            continue;
        }
        let Some(id) = name.strip_suffix(".toml").map(str::to_owned) else {
            problems.push(Problem::new(path, "só arquivo .toml entra no acervo"));
            continue;
        };
        match std::fs::read_to_string(&path) {
            Ok(toml) => entries.push(Entry { path, id, toml }),
            Err(error) => {
                let message = format!("não deu pra ler o arquivo ({error})");
                problems.push(Problem::new(path, &message));
            }
        }
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}
