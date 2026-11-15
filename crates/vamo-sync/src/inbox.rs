//! A entrada de reforços: a branch `vamo-entrada` do backup.
//!
//! Quem está fora do app (o Claude, outro projeto) não mexe no banco: põe
//! uma linha no `reforcos.jsonl` da branch `vamo-entrada` e manda. O `vamo
//! sync` lê, cria as tarefas e tira da entrada as linhas que leu. A branch
//! é separada da do backup pra quem manda reforço não precisar da história
//! do log, nem esbarrar nos arquivos das máquinas.

use crate::backup::{json, to_line};
use crate::format::{ReinforcementLine, check_version};
use crate::{Backup, Error};

/// A branch da entrada, no remoto do backup.
pub const INBOX_BRANCH: &str = "vamo-entrada";
/// O arquivo dos reforços, na raiz da branch.
pub const INBOX_FILE: &str = "reforcos.jsonl";

/// Uma linha da entrada que não deu pra ler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected {
    /// O número da linha, contando de 1.
    pub line: usize,
    /// O que tem de errado.
    pub reason: String,
}

/// A entrada lida: os reforços e as linhas que não são reforço.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inbox {
    /// O commit da entrada que foi lido. A limpeza sai em cima dele, pra
    /// não apagar reforço que chegou depois da leitura.
    base: String,
    /// O texto das linhas que não deu pra ler. Elas ficam na entrada, pra
    /// quem mandou poder consertar.
    kept: Vec<String>,
    /// Os reforços, na ordem do arquivo.
    pub reinforcements: Vec<ReinforcementLine>,
    /// As linhas que não deu pra ler, com o motivo.
    pub rejected: Vec<Rejected>,
}

impl Inbox {
    /// Se a entrada não tem nada pra limpar.
    pub fn is_empty(&self) -> bool {
        self.reinforcements.is_empty()
    }

    /// Guarda a linha de um reforço que a regra do domínio recusou (título
    /// fora do padrão, por exemplo): ela fica na entrada na limpeza.
    pub fn keep(&mut self, reinforcement: &ReinforcementLine) {
        self.kept.push(to_line(reinforcement).trim_end().to_owned());
    }
}

/// Como terminou a limpeza da entrada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cleared {
    /// As linhas lidas saíram da entrada.
    Done,
    /// Alguém mandou reforço no meio: a limpeza fica pra próxima rodada, e
    /// o que já virou tarefa não vira de novo.
    Later,
}

impl Backup {
    /// A entrada de reforços, como ficou no último [`pull`](Backup::pull).
    /// Sem a branch no remoto, ou sem o arquivo nela, `None`.
    ///
    /// # Errors
    ///
    /// [`Error::Git`] se o git falhar ao ler a branch.
    pub fn inbox(&self) -> Result<Option<Inbox>, Error> {
        let remote = format!("origin/{INBOX_BRANCH}");
        let Ok(base) = self.git(&["rev-parse", "--verify", "--quiet", &remote]) else {
            return Ok(None);
        };
        let Ok(text) = self.git(&["show", &format!("{remote}:{INBOX_FILE}")]) else {
            return Ok(None);
        };
        let mut inbox = Inbox {
            base: base.trim().to_owned(),
            kept: Vec::new(),
            reinforcements: Vec::new(),
            rejected: Vec::new(),
        };
        let path = std::path::Path::new(INBOX_FILE);
        for (n, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let parsed = json::<ReinforcementLine>(path, n + 1, line)
                .and_then(|parsed| check_version(parsed.versao).map(|()| parsed));
            match parsed {
                Ok(reinforcement) => inbox.reinforcements.push(reinforcement),
                Err(error) => {
                    inbox.rejected.push(Rejected {
                        line: n + 1,
                        reason: error.to_string(),
                    });
                    inbox.kept.push(line.to_owned());
                }
            }
        }
        Ok(Some(inbox))
    }

    /// Tira da entrada as linhas lidas: o arquivo fica só com as que não
    /// deu pra ler ou que o domínio recusou. O commit sai em cima do que
    /// foi lido e o resto da branch fica como estava.
    ///
    /// # Errors
    ///
    /// [`Error::Git`] se o git falhar de um jeito que não seja a entrada ter
    /// mudado no meio.
    pub fn clear(&self, inbox: &Inbox) -> Result<Cleared, Error> {
        let mut text = inbox.kept.join("\n");
        if !text.is_empty() {
            text.push('\n');
        }
        let blob = self.git_with_input(&["hash-object", "-w", "--stdin"], Some(&text))?;
        let listing = self.git(&["ls-tree", &inbox.base])?;
        let suffix = format!("\t{INBOX_FILE}");
        let mut entries: Vec<String> = listing
            .lines()
            .filter(|entry| !entry.ends_with(&suffix))
            .map(str::to_owned)
            .collect();
        entries.push(format!("100644 blob {}{suffix}", blob.trim()));
        let mut tree = entries.join("\n");
        tree.push('\n');
        let tree = self.git_with_input(&["mktree"], Some(&tree))?;
        let commit = self.git(&[
            "commit-tree",
            tree.trim(),
            "-p",
            &inbox.base,
            "-m",
            "Tirar da entrada os reforços lidos",
        ])?;
        let target = format!("{}:refs/heads/{INBOX_BRANCH}", commit.trim());
        match self.git(&["push", "--quiet", "origin", &target]) {
            Ok(_) => Ok(Cleared::Done),
            Err(Error::Git { stderr, .. })
                if stderr.contains("rejected") || stderr.contains("fetch first") =>
            {
                Ok(Cleared::Later)
            }
            Err(error) => Err(error),
        }
    }
}
