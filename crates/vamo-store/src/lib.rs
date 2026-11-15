//! O armazenamento do `vamo_fogao` em SQLite.
//!
//! Adaptador da porta [`EventStore`] que o `vamo-core` define: guarda o log
//! de eventos num arquivo SQLite local. As migrações são arquivos `.sql`
//! versionados, aplicados em ordem com `PRAGMA user_version` (veja o módulo
//! de migrações).
//!
//! Também lê, só lendo, o banco do habit-tracker: é de lá que vêm os
//! números dos hábitos da súmula ([`HabitTracker`], adaptador da porta
//! [`Habits`](vamo_core::Habits)).
//!
//! O adaptador só traduz. Cada valor vai pro banco como o texto que o tipo do
//! domínio escreve (`T-041`, `brecha`) e volta pelo `FromStr` do mesmo tipo,
//! que confere de novo. Um banco mexido por fora vira erro na leitura, nunca
//! um valor inválido solto no programa.
//!
//! ```
//! use vamo_core::EventStore;
//! use vamo_store::Store;
//!
//! let store = Store::in_memory()?;
//! assert!(store.events()?.is_empty());
//! # Ok::<(), vamo_store::Error>(())
//! ```

mod error;
mod habits;
mod migrations;

use std::collections::BTreeSet;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use jiff::civil::Date;
use rusqlite::{Connection, Row, params};
use vamo_core::{AgendaMark, Change, Event, EventStore, TaskId};

pub use error::Error;
pub use habits::{Habit, HabitTracker};

/// O banco do `vamo_fogao`.
#[derive(Debug)]
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Abre o banco nesse caminho e aplica as migrações que faltam. A pasta e
    /// o arquivo são criados se ainda não existirem.
    ///
    /// # Errors
    ///
    /// - [`Error::CreateDir`] se a pasta não puder ser criada.
    /// - [`Error::NewerSchema`] se o banco for de uma versão mais nova do vamo.
    ///   Nesse caso o banco não é tocado.
    /// - [`Error::Sqlite`] se o SQLite falhar ao abrir ou migrar.
    pub fn open(path: &Path) -> Result<Self, Error> {
        if let Some(dir) = path.parent()
            && !dir.as_os_str().is_empty()
        {
            std::fs::create_dir_all(dir).map_err(|source| Error::CreateDir {
                path: dir.to_owned(),
                source,
            })?;
        }
        let conn = Connection::open(path)?;
        // Com o WAL, quem lê (a barra, a TUI) não fica esperando quem escreve
        // (a CLI), e vice-versa.
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get::<_, String>(0))?;
        Self::start(conn)
    }

    /// Um banco em memória, que some quando o `Store` sai de cena. Serve pra
    /// teste e pra simular uma operação sem gravar nada.
    ///
    /// # Errors
    ///
    /// [`Error::Sqlite`] se o SQLite falhar.
    pub fn in_memory() -> Result<Self, Error> {
        Self::start(Connection::open_in_memory()?)
    }

    /// A versão do esquema do banco, que é o número de migrações aplicadas.
    ///
    /// # Errors
    ///
    /// [`Error::Sqlite`] se o SQLite falhar.
    pub fn schema_version(&self) -> Result<u32, Error> {
        migrations::version(&self.conn)
    }

    fn start(mut conn: Connection) -> Result<Self, Error> {
        // Se outro processo estiver escrevendo, espera um pouco antes de
        // desistir.
        conn.busy_timeout(Duration::from_secs(5))?;
        migrations::run(&mut conn)?;
        Ok(Self { conn })
    }
}

impl Store {
    /// As atividades fixas da agenda marcadas, como `(dia, UID do evento)`.
    ///
    /// # Errors
    ///
    /// Quando o SQLite falha, ou um dia guardado não é um dia válido.
    pub fn agenda_checks(&self) -> Result<BTreeSet<(Date, String)>, Error> {
        Ok(self
            .agenda_marks()?
            .into_iter()
            .filter(|mark| mark.done)
            .map(|mark| (mark.day, mark.uid))
            .collect())
    }

    /// Todas as marcações da agenda, as desfeitas inclusas, com o instante
    /// de cada uma: é o que vai pra sincronia.
    ///
    /// # Errors
    ///
    /// Quando o SQLite falha, ou um dia ou instante guardado não é válido.
    pub fn agenda_marks(&self) -> Result<Vec<AgendaMark>, Error> {
        let mut select = self
            .conn
            .prepare("SELECT day, uid, done, at FROM agenda_checks ORDER BY day, uid")?;
        let mut rows = select.query([])?;
        let mut marks = Vec::new();
        while let Some(row) = rows.next()? {
            let day: String = row.get(0)?;
            let at: String = row.get(3)?;
            marks.push(AgendaMark {
                day: day
                    .parse()
                    .map_err(|source| Error::InvalidCheckDay { day, source })?,
                uid: row.get(1)?,
                done: row.get(2)?,
                at: at
                    .parse()
                    .map_err(|source| Error::InvalidCheckInstant { at, source })?,
            });
        }
        Ok(marks)
    }

    /// Marca (`done`) ou desmarca a atividade fixa `uid` em `day`, no
    /// instante `at`.
    ///
    /// # Errors
    ///
    /// Quando o SQLite falha.
    pub fn set_agenda_check(
        &mut self,
        day: Date,
        uid: &str,
        done: bool,
        at: jiff::Timestamp,
    ) -> Result<(), Error> {
        self.conn.execute(
            "INSERT INTO agenda_checks (day, uid, done, at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (day, uid) DO UPDATE SET done = ?3, at = ?4",
            params![day.to_string(), uid, done, at.to_string()],
        )?;
        Ok(())
    }

    /// Junta uma marcação que veio de outra máquina: ela só entra se for
    /// mais recente que a daqui (ou se aqui não tiver nenhuma). Devolve se
    /// entrou.
    ///
    /// # Errors
    ///
    /// Quando o SQLite falha, ou a marcação guardada não é válida.
    pub fn merge_agenda_mark(&mut self, mark: &AgendaMark) -> Result<bool, Error> {
        let current = self
            .agenda_marks()?
            .into_iter()
            .find(|known| known.day == mark.day && known.uid == mark.uid);
        if current.is_some_and(|current| !mark.wins_over(&current)) {
            return Ok(false);
        }
        self.set_agenda_check(mark.day, &mark.uid, mark.done, mark.at)?;
        Ok(true)
    }

    /// Passa os eventos dados pra outra tarefa: é a troca de número da
    /// sincronia, quando outra máquina criou uma tarefa com o mesmo número
    /// (veja [`vamo_core::renumber_collisions`]). Só serve pra evento que
    /// ainda não saiu desta máquina. Tudo numa transação: ou passam todos,
    /// ou nenhum.
    ///
    /// # Errors
    ///
    /// Quando o SQLite falha.
    pub fn renumber(&mut self, events: &[vamo_core::EventId], to: TaskId) -> Result<(), Error> {
        let tx = self.conn.transaction()?;
        {
            let mut update = tx.prepare("UPDATE events SET task = ?1 WHERE id = ?2")?;
            for id in events {
                update.execute(params![to.to_string(), id.to_string()])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}

impl Store {
    /// Guarda a súmula fechada de `day`, já no formato de exportação. Fechar
    /// o mesmo dia de novo troca a súmula guardada: nunca fica mais de uma
    /// por dia.
    ///
    /// # Errors
    ///
    /// Quando o SQLite falha.
    pub fn save_summary(
        &mut self,
        day: Date,
        closed_at: jiff::Timestamp,
        content: &str,
    ) -> Result<(), Error> {
        self.conn.execute(
            "INSERT INTO summaries (day, closed_at, content) VALUES (?1, ?2, ?3)
             ON CONFLICT (day) DO UPDATE SET closed_at = ?2, content = ?3",
            params![day.to_string(), closed_at.to_string(), content],
        )?;
        Ok(())
    }

    /// As súmulas guardadas, em ordem de dia, como `(dia, conteúdo)`.
    ///
    /// # Errors
    ///
    /// Quando o SQLite falha, ou um dia guardado não é um dia válido.
    pub fn summaries(&self) -> Result<Vec<(Date, String)>, Error> {
        let mut select = self
            .conn
            .prepare("SELECT day, content FROM summaries ORDER BY day")?;
        let mut rows = select.query([])?;
        let mut summaries = Vec::new();
        while let Some(row) = rows.next()? {
            let day: String = row.get(0)?;
            let day = day
                .parse()
                .map_err(|source| Error::InvalidSummaryDay { day, source })?;
            summaries.push((day, row.get(1)?));
        }
        Ok(summaries)
    }
}

impl EventStore for Store {
    type Error = Error;

    fn append(&mut self, events: &[Event]) -> Result<usize, Error> {
        let tx = self.conn.transaction()?;
        let mut added = 0;
        {
            // O `OR IGNORE` deixa de fora o evento cujo ID já está no banco.
            let mut insert = tx.prepare(
                "INSERT OR IGNORE INTO events
                     (id, at, machine, task, kind, title, area, fit, status, notes, day, checked)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            )?;
            for event in events {
                let columns = Columns::from(&event.change);
                added += insert.execute(params![
                    event.id.to_string(),
                    event.at.to_string(),
                    event.machine.as_str(),
                    event.task.to_string(),
                    columns.kind,
                    columns.title,
                    columns.area,
                    columns.fit,
                    columns.status,
                    columns.notes,
                    columns.day,
                    columns.checked,
                ])?;
            }
        }
        tx.commit()?;
        Ok(added)
    }

    fn events(&self) -> Result<Vec<Event>, Error> {
        let mut select = self.conn.prepare(
            "SELECT seq, id, at, machine, task, kind, title, area, fit, status, notes, day, checked
             FROM events
             ORDER BY seq",
        )?;
        let mut rows = select.query([])?;
        let mut events = Vec::new();
        while let Some(row) = rows.next()? {
            events.push(read_event(row)?);
        }
        Ok(events)
    }
}

// Os valores da coluna `kind`, um pra cada variante de `Change`.
const CREATED: &str = "created";
const TITLE: &str = "title";
const AREA: &str = "area";
const FIT: &str = "fit";
const STATUS: &str = "status";
const NOTES: &str = "notes";
const DAY: &str = "day";
const CHECK: &str = "check";

/// As colunas de uma mudança. A criação preenche quatro; as outras mudanças
/// preenchem só a delas.
struct Columns {
    kind: &'static str,
    title: Option<String>,
    area: Option<String>,
    fit: Option<&'static str>,
    status: Option<&'static str>,
    notes: Option<String>,
    day: Option<String>,
    checked: Option<bool>,
}

impl From<&Change> for Columns {
    fn from(change: &Change) -> Self {
        let empty = |kind| Columns {
            kind,
            title: None,
            area: None,
            fit: None,
            status: None,
            notes: None,
            day: None,
            checked: None,
        };
        match change {
            Change::Created {
                title,
                area,
                fit,
                status,
            } => Columns {
                title: Some(title.to_string()),
                area: Some(area.to_string()),
                fit: Some(fit.as_str()),
                status: Some(status.as_str()),
                ..empty(CREATED)
            },
            Change::Title(title) => Columns {
                title: Some(title.to_string()),
                ..empty(TITLE)
            },
            Change::Area(area) => Columns {
                area: Some(area.to_string()),
                ..empty(AREA)
            },
            Change::Fit(fit) => Columns {
                fit: Some(fit.as_str()),
                ..empty(FIT)
            },
            Change::Status(status) => Columns {
                status: Some(status.as_str()),
                ..empty(STATUS)
            },
            Change::Notes(notes) => Columns {
                notes: Some(notes.clone()),
                ..empty(NOTES)
            },
            Change::Day(day) => Columns {
                day: day.map(|day| day.to_string()),
                ..empty(DAY)
            },
            Change::Check { day, done } => Columns {
                day: Some(day.to_string()),
                checked: Some(*done),
                ..empty(CHECK)
            },
        }
    }
}

/// Monta o evento de uma linha do log, conferindo cada valor pelo tipo do
/// domínio.
fn read_event(row: &Row<'_>) -> Result<Event, Error> {
    let seq: i64 = row.get("seq")?;
    let kind: String = row.get("kind")?;
    let change = match kind.as_str() {
        CREATED => Change::Created {
            title: value(row, seq, "title")?,
            area: value(row, seq, "area")?,
            fit: value(row, seq, "fit")?,
            status: value(row, seq, "status")?,
        },
        TITLE => Change::Title(value(row, seq, "title")?),
        AREA => Change::Area(value(row, seq, "area")?),
        FIT => Change::Fit(value(row, seq, "fit")?),
        STATUS => Change::Status(value(row, seq, "status")?),
        // Notas são texto livre: não tem o que conferir além de existir.
        NOTES => Change::Notes(text(row, seq, "notes")?),
        // Sem dia, a tarefa saiu da escalação.
        DAY => match row.get::<_, Option<String>>("day")? {
            Some(day) => Change::Day(Some(date(&day, seq)?)),
            None => Change::Day(None),
        },
        CHECK => Change::Check {
            day: date(&text(row, seq, "day")?, seq)?,
            done: row
                .get::<_, Option<bool>>("checked")?
                .ok_or(Error::MissingColumn {
                    seq,
                    column: "checked",
                })?,
        },
        _ => return Err(Error::UnknownKind { seq, kind }),
    };
    Ok(Event {
        id: value(row, seq, "id")?,
        at: text(row, seq, "at")?
            .parse()
            .map_err(|source| Error::InvalidInstant { seq, source })?,
        machine: value(row, seq, "machine")?,
        task: value(row, seq, "task")?,
        change,
    })
}

/// O texto de uma coluna que não pode estar vazia (`NULL`).
/// Lê um dia no formato `2026-11-11`.
fn date(text: &str, seq: i64) -> Result<jiff::civil::Date, Error> {
    text.parse()
        .map_err(|source| Error::InvalidDay { seq, source })
}

fn text(row: &Row<'_>, seq: i64, column: &'static str) -> Result<String, Error> {
    let text: Option<String> = row.get(column)?;
    text.ok_or(Error::MissingColumn { seq, column })
}

/// O valor de uma coluna, convertido no tipo do domínio pelo `FromStr` dele.
fn value<T>(row: &Row<'_>, seq: i64, column: &'static str) -> Result<T, Error>
where
    T: FromStr<Err = vamo_core::Error>,
{
    text(row, seq, column)?
        .parse()
        .map_err(|source| Error::InvalidValue {
            seq,
            column,
            source,
        })
}
