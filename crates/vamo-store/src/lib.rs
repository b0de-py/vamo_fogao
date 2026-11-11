//! O armazenamento do `vamo_fogao` em SQLite.
//!
//! Adaptador da porta [`EventStore`] que o `vamo-core` define: guarda o log
//! de eventos num arquivo SQLite local. As migrações são arquivos `.sql`
//! versionados, aplicados em ordem com `PRAGMA user_version` (veja o módulo
//! de migrações).
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
mod migrations;

use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use rusqlite::{Connection, Row, params};
use vamo_core::{Change, Event, EventStore};

pub use error::Error;

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

impl EventStore for Store {
    type Error = Error;

    fn append(&mut self, events: &[Event]) -> Result<usize, Error> {
        let tx = self.conn.transaction()?;
        let mut added = 0;
        {
            // O `OR IGNORE` deixa de fora o evento cujo ID já está no banco.
            let mut insert = tx.prepare(
                "INSERT OR IGNORE INTO events
                     (id, at, machine, task, kind, title, area, fit, status, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
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
                ])?;
            }
        }
        tx.commit()?;
        Ok(added)
    }

    fn events(&self) -> Result<Vec<Event>, Error> {
        let mut select = self.conn.prepare(
            "SELECT seq, id, at, machine, task, kind, title, area, fit, status, notes
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

/// As colunas de uma mudança. A criação preenche quatro; as outras mudanças
/// preenchem só a delas.
struct Columns {
    kind: &'static str,
    title: Option<String>,
    area: Option<String>,
    fit: Option<&'static str>,
    status: Option<&'static str>,
    notes: Option<String>,
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
