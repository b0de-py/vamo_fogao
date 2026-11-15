//! O repositório de backup: um clone git privado onde cada máquina escreve
//! os próprios eventos e lê os das outras.
//!
//! Cada máquina escreve só os arquivos dela (`eventos/<máquina>.jsonl` e
//! `marcacoes/<máquina>.jsonl`), então duas máquinas nunca mexem no mesmo
//! arquivo e o git junta tudo sem conflito. Quem fala com o git é o
//! programa `git` do sistema, com as credenciais que ele já tem.

use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use vamo_core::{AgendaMark, Event, EventId, Machine};

use crate::Error;
use crate::format::{EventLine, MarkLine, VERSION, check_version};

/// A pasta dos eventos, dentro do repositório.
const EVENTS_DIR: &str = "eventos";
/// A pasta das marcações da agenda.
const MARKS_DIR: &str = "marcacoes";

/// O que veio do backup na leitura.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// Os IDs dos eventos desta máquina que já estão no backup.
    pub exported: BTreeSet<EventId>,
    /// Os eventos das outras máquinas.
    pub remote: Vec<Event>,
    /// As marcações das outras máquinas.
    pub marks: Vec<AgendaMark>,
}

/// Como terminou o envio pro backup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pushed {
    /// Nada mudou desde o último envio: nem commit, nem push.
    Unchanged,
    /// Um commit novo subiu.
    Sent,
}

/// Um clone do repositório de backup, visto por uma máquina.
#[derive(Debug, Clone)]
pub struct Backup {
    dir: PathBuf,
    machine: Machine,
    branch: String,
}

impl Backup {
    /// O clone em `dir`, pra máquina `machine`.
    ///
    /// # Errors
    ///
    /// - [`Error::Git`] se `dir` não for um repositório git, ou o git falhar.
    /// - [`Error::NoRemote`] se o clone não tiver o remoto `origin`.
    /// - [`Error::PublicRepository`] se o `origin` for o repositório público
    ///   do próprio vamo_fogao: o backup tem dado pessoal e não pode ir pra
    ///   lá.
    pub fn open(dir: &Path, machine: Machine) -> Result<Self, Error> {
        let backup = Self {
            dir: dir.to_owned(),
            machine,
            branch: String::new(),
        };
        backup.git(&["rev-parse", "--git-dir"])?;
        let url = backup
            .git(&["remote", "get-url", "origin"])
            .map_err(|_| Error::NoRemote {
                dir: dir.to_owned(),
            })?;
        let name = url.trim().trim_end_matches('/').trim_end_matches(".git");
        if name.ends_with("vamo_fogao") {
            return Err(Error::PublicRepository {
                url: url.trim().to_owned(),
            });
        }
        let branch = backup.git(&["symbolic-ref", "--short", "HEAD"])?;
        Ok(Self {
            branch: branch.trim().to_owned(),
            ..backup
        })
    }

    /// Traz o que as outras máquinas mandaram. Se o remoto ainda não tem a
    /// branch, não traz nada. Como cada máquina só mexe nos arquivos dela,
    /// o rebase nunca esbarra em conflito.
    ///
    /// # Errors
    ///
    /// [`Error::Git`] se o git falhar (sem rede, por exemplo).
    pub fn pull(&self) -> Result<(), Error> {
        self.git(&["fetch", "--quiet", "origin"])?;
        let remote = format!("origin/{}", self.branch);
        if self
            .git(&["rev-parse", "--verify", "--quiet", &remote])
            .is_err()
        {
            return Ok(());
        }
        if self
            .git(&["rev-parse", "--verify", "--quiet", "HEAD"])
            .is_err()
        {
            // Clone de um repositório que estava vazio: ainda sem commit
            // aqui, a branch começa da do remoto.
            self.git(&["reset", "--quiet", "--hard", &remote])?;
        } else {
            self.git(&["rebase", "--quiet", "--autostash", &remote])?;
        }
        Ok(())
    }

    /// Lê o backup: quais eventos desta máquina já estão lá, e os eventos e
    /// marcações das outras.
    ///
    /// # Errors
    ///
    /// - [`Error::Io`] se um arquivo não puder ser lido.
    /// - [`Error::Json`] se uma linha não for do formato.
    /// - [`Error::NewerFormat`] se uma linha for de um vamo mais novo.
    /// - Os erros do domínio, se um campo tiver valor inválido.
    pub fn read(&self) -> Result<Snapshot, Error> {
        let mut snapshot = Snapshot::default();
        for (machine, path) in self.files(EVENTS_DIR)? {
            let events: Vec<Event> = lines(&path)?
                .into_iter()
                .map(|(line, text)| {
                    let parsed: EventLine = json(&path, line, &text)?;
                    Event::try_from(parsed)
                })
                .collect::<Result<_, Error>>()?;
            if machine == self.machine.as_str() {
                snapshot
                    .exported
                    .extend(events.iter().map(|event| event.id));
            } else {
                snapshot.remote.extend(events);
            }
        }
        for (machine, path) in self.files(MARKS_DIR)? {
            if machine == self.machine.as_str() {
                continue;
            }
            for (line, text) in lines(&path)? {
                let parsed: MarkLine = json(&path, line, &text)?;
                snapshot.marks.push(mark(parsed)?);
            }
        }
        Ok(snapshot)
    }

    /// Escreve os arquivos desta máquina: os eventos dela, em ordem de
    /// instante, e as marcações que ela conhece, em ordem de dia. A ordem é
    /// fixa pro arquivo só mudar quando o conteúdo muda.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] se a pasta ou o arquivo não puderem ser escritos.
    pub fn write(&self, events: &[Event], marks: &[AgendaMark]) -> Result<(), Error> {
        let mut own: Vec<&Event> = events
            .iter()
            .filter(|event| event.machine == self.machine)
            .collect();
        own.sort_by_key(|event| (event.at, event.id));
        let mut text = String::new();
        for event in own {
            text.push_str(&to_line(&EventLine::from(event)));
        }
        self.save(EVENTS_DIR, &text)?;

        let mut marks: Vec<&AgendaMark> = marks.iter().collect();
        marks.sort_by(|a, b| (a.day, &a.uid).cmp(&(b.day, &b.uid)));
        let mut text = String::new();
        for mark in marks {
            text.push_str(&to_line(&MarkLine {
                versao: VERSION,
                dia: mark.day.to_string(),
                uid: mark.uid.clone(),
                feita: mark.done,
                em: mark.at.to_string(),
            }));
        }
        self.save(MARKS_DIR, &text)
    }

    /// Faz o commit dos arquivos desta máquina e manda pro remoto. Sem
    /// mudança, não faz nada.
    ///
    /// # Errors
    ///
    /// - [`Error::Rejected`] se o remoto recusar porque outra máquina mandou
    ///   antes: é só trazer de novo e repetir.
    /// - [`Error::Git`] se o git falhar de outro jeito.
    pub fn send(&self) -> Result<Pushed, Error> {
        self.git(&["add", "--all", "--", EVENTS_DIR, MARKS_DIR])?;
        if self
            .git(&["diff", "--cached", "--quiet", "--", EVENTS_DIR, MARKS_DIR])
            .is_ok()
        {
            return Ok(Pushed::Unchanged);
        }
        let message = format!("Sincronizar o {}", self.machine);
        self.git(&["commit", "--quiet", "--no-verify", "-m", &message])?;
        let target = format!("HEAD:refs/heads/{}", self.branch);
        match self.git(&["push", "--quiet", "origin", &target]) {
            Ok(_) => Ok(Pushed::Sent),
            Err(Error::Git { stderr, .. })
                if stderr.contains("rejected") || stderr.contains("fetch first") =>
            {
                Err(Error::Rejected)
            }
            Err(error) => Err(error),
        }
    }

    /// A pasta do clone.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Roda um comando do git no clone e devolve a saída padrão.
    pub(crate) fn git(&self, args: &[&str]) -> Result<String, Error> {
        self.git_with_input(args, None)
    }

    /// Roda um comando do git com `input` na entrada padrão.
    pub(crate) fn git_with_input(
        &self,
        args: &[&str],
        input: Option<&str>,
    ) -> Result<String, Error> {
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(&self.dir)
            // O commit do backup é do vamo, com o nome da máquina: não pede
            // a identidade de quem usa nem assinatura, que travaria o timer
            // esperando senha.
            .args(["-c", &format!("user.name=vamo ({})", self.machine)])
            .args(["-c", &format!("user.email=vamo@{}.local", self.machine)])
            .args(["-c", "commit.gpgsign=false"])
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let failed = |source| Error::GitMissing { source };
        let mut child = command.spawn().map_err(failed)?;
        if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
            stdin.write_all(input.as_bytes()).map_err(failed)?;
        }
        let Output {
            status,
            stdout,
            stderr,
        } = child.wait_with_output().map_err(failed)?;
        if status.success() {
            Ok(String::from_utf8_lossy(&stdout).into_owned())
        } else {
            Err(Error::Git {
                command: args.join(" "),
                stderr: String::from_utf8_lossy(&stderr).trim().to_owned(),
            })
        }
    }

    /// Os arquivos `.jsonl` de uma pasta do clone, com o nome da máquina de
    /// cada um. Pasta que ainda não existe é pasta vazia.
    fn files(&self, folder: &str) -> Result<Vec<(String, PathBuf)>, Error> {
        let dir = self.dir.join(folder);
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => return Err(Error::Io { path: dir, source }),
        };
        let mut files = Vec::new();
        for entry in entries {
            let path = entry
                .map_err(|source| Error::Io {
                    path: dir.clone(),
                    source,
                })?
                .path();
            if path.extension().is_some_and(|ext| ext == "jsonl")
                && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
            {
                files.push((stem.to_owned(), path));
            }
        }
        files.sort();
        Ok(files)
    }

    /// Grava o arquivo desta máquina numa pasta do clone.
    fn save(&self, folder: &str, text: &str) -> Result<(), Error> {
        let dir = self.dir.join(folder);
        std::fs::create_dir_all(&dir).map_err(|source| Error::Io {
            path: dir.clone(),
            source,
        })?;
        let path = dir.join(format!("{}.jsonl", self.machine));
        std::fs::write(&path, text).map_err(|source| Error::Io { path, source })
    }
}

/// As linhas com conteúdo de um arquivo, com o número de cada uma.
fn lines(path: &Path) -> Result<Vec<(usize, String)>, Error> {
    let text = std::fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.to_owned(),
        source,
    })?;
    Ok(text
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(n, line)| (n + 1, line.to_owned()))
        .collect())
}

/// Lê uma linha de JSON, apontando o arquivo e a linha no erro.
pub(crate) fn json<T: serde::de::DeserializeOwned>(
    path: &Path,
    line: usize,
    text: &str,
) -> Result<T, Error> {
    serde_json::from_str(text).map_err(|source| Error::Json {
        file: path.display().to_string(),
        line,
        source,
    })
}

/// Uma linha de JSONL, com a quebra no fim.
pub(crate) fn to_line<T: serde::Serialize>(value: &T) -> String {
    let mut line =
        serde_json::to_string(value).expect("os tipos do formato só têm texto, número e lista");
    line.push('\n');
    line
}

/// A marcação do domínio a partir da linha.
fn mark(line: MarkLine) -> Result<AgendaMark, Error> {
    check_version(line.versao)?;
    Ok(AgendaMark {
        day: line.dia.parse().map_err(Error::Time)?,
        uid: line.uid,
        done: line.feita,
        at: line.em.parse().map_err(Error::Time)?,
    })
}
