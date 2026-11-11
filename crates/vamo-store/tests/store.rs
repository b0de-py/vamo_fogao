//! Testes do banco num arquivo de verdade, numa pasta temporária.

use std::path::{Path, PathBuf};

use jiff::Timestamp;
use rusqlite::Connection;
use tempfile::TempDir;
use uuid::Builder;
use vamo_core::{Change, Event, EventId, EventStore, Fit, Status, TaskId};
use vamo_store::{Error, Store};

/// Um banco numa pasta que ainda não existe, dentro de uma pasta temporária
/// que some no fim do teste.
fn temp_db() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("o sistema deveria criar uma pasta temporária");
    let path = dir.path().join("dados").join("vamo.db");
    (dir, path)
}

/// Um evento de teste. O `n` vira o ID e o instante, com fração de segundo
/// pra conferir que nada se perde no caminho.
fn event(n: u32, task: u32, change: Change) -> Event {
    let uuid = Builder::from_unix_timestamp_millis(n.into(), &[0; 10]).into_uuid();
    Event {
        id: EventId::new(uuid).expect("o Builder monta um UUID v7"),
        at: Timestamp::new(1_800_000_000 + i64::from(n), 123_456_789)
            .expect("o instante está dentro do que o jiff aceita"),
        machine: "notebook".parse().expect("o nome da máquina é válido"),
        task: TaskId::new(task).expect("os testes não usam a tarefa zero"),
        change,
    }
}

fn created(title: &str) -> Change {
    Change::Created {
        title: title.parse().expect("os testes usam títulos válidos"),
        area: "casa".parse().expect("a área é válida"),
        fit: Fit::Gap,
        status: Status::Ready,
    }
}

/// Grava uma linha direto no SQLite, por fora do `Store`, como alguém
/// mexendo no banco na mão.
fn insert_raw(path: &Path, kind: &str, column: &str, value: &str) {
    let conn = Connection::open(path).expect("o banco do teste deveria abrir");
    conn.execute(
        &format!(
            "INSERT INTO events (id, at, machine, task, kind, {column})
             VALUES ('0192f0c4-8a00-7000-8000-000000000099', '2026-11-11T12:00:00Z',
                     'notebook', 'T-041', ?1, ?2)"
        ),
        [kind, value],
    )
    .expect("a linha cabe no esquema");
}

#[test]
fn a_new_database_is_created_and_migrated_from_zero() {
    let (_dir, path) = temp_db();

    let store = Store::open(&path).unwrap();

    assert!(path.exists());
    assert_eq!(store.schema_version().unwrap(), 2);
    assert_eq!(store.events().unwrap(), []);
}

#[test]
fn events_survive_closing_and_opening_again() {
    let (_dir, path) = temp_db();
    let events = [
        event(1, 41, created("Trocar o filtro da água")),
        event(2, 41, Change::Status(Status::Done)),
    ];

    let mut store = Store::open(&path).unwrap();
    store.append(&events).unwrap();
    drop(store);

    let store = Store::open(&path).unwrap();
    assert_eq!(store.events().unwrap(), events);
}

#[test]
fn every_kind_of_change_comes_back_the_same() {
    let mut store = Store::in_memory().unwrap();
    let events = [
        event(1, 41, created("Pôr o lixo pra fora")),
        event(2, 41, Change::Title("Levar o lixo".parse().unwrap())),
        event(3, 41, Change::Area("saúde".parse().unwrap())),
        event(4, 41, Change::Fit(Fit::WithSomeone)),
        event(5, 41, Change::Status(Status::Done)),
        event(6, 41, Change::Notes("Saco azul é reciclável. ♻".to_owned())),
        event(7, 41, Change::Notes(String::new())),
        event(8, 41, Change::Day(Some(jiff::civil::date(2026, 11, 11)))),
        event(9, 41, Change::Day(None)),
        event(
            10,
            41,
            Change::Check {
                day: jiff::civil::date(2026, 11, 11),
                done: true,
            },
        ),
        event(
            11,
            41,
            Change::Check {
                day: jiff::civil::date(2026, 11, 12),
                done: false,
            },
        ),
    ];

    store.append(&events).unwrap();

    assert_eq!(store.events().unwrap(), events);
}

#[test]
fn the_log_comes_back_in_the_order_it_was_saved() {
    let mut store = Store::in_memory().unwrap();
    // O evento 2 é gravado antes do 1. A ordem de gravação é a que vale.
    let later = event(2, 41, Change::Status(Status::Done));
    let earlier = event(1, 41, created("Trocar o filtro"));

    store.append(std::slice::from_ref(&later)).unwrap();
    store.append(std::slice::from_ref(&earlier)).unwrap();

    assert_eq!(store.events().unwrap(), [later, earlier]);
}

#[test]
fn the_same_event_is_saved_only_once() {
    let mut store = Store::in_memory().unwrap();
    let first = event(1, 41, created("Trocar o filtro"));
    let second = event(2, 41, Change::Status(Status::Done));

    assert_eq!(store.append(std::slice::from_ref(&first)).unwrap(), 1);
    assert_eq!(store.append(&[first.clone(), second.clone()]).unwrap(), 1);

    assert_eq!(store.events().unwrap(), [first, second]);
}

#[test]
fn a_database_from_a_newer_version_is_refused_and_left_alone() {
    let (_dir, path) = temp_db();
    drop(Store::open(&path).unwrap());
    Connection::open(&path)
        .unwrap()
        .pragma_update(None, "user_version", 99)
        .unwrap();

    let result = Store::open(&path);

    assert!(matches!(
        result,
        Err(Error::NewerSchema {
            found: 99,
            known: 2
        })
    ));
    let version: u32 = Connection::open(&path)
        .unwrap()
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 99);
}

#[test]
fn a_value_the_domain_rejects_is_an_error_not_a_task() {
    let (_dir, path) = temp_db();
    drop(Store::open(&path).unwrap());
    insert_raw(&path, "title", "title", "Reunião com o pessoal");

    let result = Store::open(&path).unwrap().events();

    assert!(matches!(
        result,
        Err(Error::InvalidValue {
            column: "title",
            source: vamo_core::Error::TitleWithoutVerb { .. },
            ..
        })
    ));
}

#[test]
fn a_missing_column_is_an_error() {
    let (_dir, path) = temp_db();
    drop(Store::open(&path).unwrap());
    // Uma criação só com o título, sem área, encaixe e status.
    insert_raw(&path, "created", "title", "Trocar o filtro");

    let result = Store::open(&path).unwrap().events();

    assert!(matches!(
        result,
        Err(Error::MissingColumn { column: "area", .. })
    ));
}

#[test]
fn the_schema_refuses_an_unknown_kind_of_change() {
    let (_dir, path) = temp_db();
    drop(Store::open(&path).unwrap());
    let conn = Connection::open(&path).unwrap();

    let result = conn.execute(
        "INSERT INTO events (id, at, machine, task, kind)
         VALUES ('0192f0c4-8a00-7000-8000-000000000099', '2026-11-11T12:00:00Z',
                 'notebook', 'T-041', 'deleted')",
        [],
    );

    assert!(result.is_err());
}

#[test]
fn a_database_from_the_first_version_is_migrated_with_its_events() {
    let (_dir, path) = temp_db();
    // Um banco como o da versão 1: só a primeira migração.
    {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(include_str!("../migrations/0001-log-de-eventos.sql"))
            .unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        conn.execute(
            "INSERT INTO events (id, at, machine, task, kind, status)
             VALUES ('0192f0c4-8a00-7000-8000-000000000001', '2026-11-11T12:00:00Z',
                     'notebook', 'T-041', 'status', 'pronta')",
            [],
        )
        .unwrap();
    }
    let mut store = Store::open(&path).unwrap();
    assert_eq!(store.schema_version().unwrap(), 2);
    let old = store.events().unwrap();
    assert_eq!(old.len(), 1);
    assert_eq!(old[0].change, Change::Status(Status::Ready));

    // E a tabela nova aceita as mudanças novas.
    store
        .append(&[event(
            2,
            41,
            Change::Day(Some(jiff::civil::date(2026, 11, 11))),
        )])
        .unwrap();
    assert_eq!(store.events().unwrap().len(), 2);
}
