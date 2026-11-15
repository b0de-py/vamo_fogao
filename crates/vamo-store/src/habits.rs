//! Os hábitos do habit-tracker, lidos do banco SQLite dele.
//!
//! O habit-tracker é outro app: cada toque num hábito vira uma linha na
//! tabela `events`, com a chave do hábito (`kind`) e o horário local do
//! toque (`created_at`, como `2026-11-11T14:05:00`). O `vamo` só lê, e abre
//! o banco dele em modo somente leitura.

use std::path::{Path, PathBuf};

use jiff::civil::Date;
use rusqlite::{Connection, OpenFlags, params};
use vamo_core::{HabitCount, Habits};

use crate::Error;

/// Um hábito que o habit-tracker conhece.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Habit {
    /// A chave do hábito no banco, a coluna `kind`.
    pub key: String,
    /// O nome pra mostrar, como "Água".
    pub name: String,
    /// A unidade, no singular, como "copo".
    pub unit: String,
}

/// O banco do habit-tracker e os hábitos que valem a súmula.
#[derive(Debug, Clone)]
pub struct HabitTracker {
    path: PathBuf,
    habits: Vec<Habit>,
    turn: u8,
}

impl HabitTracker {
    /// O habit-tracker com o banco em `path`. `turn` é a hora em que o dia
    /// dele vira (a `hora_virada_dia` da config dele): com 0, o dia vai da
    /// meia-noite à meia-noite.
    ///
    /// Nada é aberto aqui: o banco só é lido na hora de contar.
    pub fn new(path: &Path, habits: Vec<Habit>, turn: u8) -> Self {
        Self {
            path: path.to_owned(),
            habits,
            turn,
        }
    }
}

impl Habits for HabitTracker {
    type Error = Error;

    fn count(&self, first: Date, last: Date) -> Result<Vec<HabitCount>, Error> {
        let conn = Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        // O horário do habit-tracker é texto ISO sem fuso, e texto nesse
        // formato compara na mesma ordem do tempo.
        let hour = i8::try_from(self.turn.min(23)).expect("uma hora até 23 cabe num i8");
        let from = first.at(hour, 0, 0, 0).strftime("%Y-%m-%dT%H:%M:%S");
        let next = last.tomorrow().map_err(Error::Habits)?;
        let until = next.at(hour, 0, 0, 0).strftime("%Y-%m-%dT%H:%M:%S");

        let mut select = conn.prepare(
            "SELECT COUNT(*) FROM events WHERE kind = ?1 AND created_at >= ?2 AND created_at < ?3",
        )?;
        self.habits
            .iter()
            .map(|habit| {
                let count: u32 = select.query_row(
                    params![habit.key, from.to_string(), until.to_string()],
                    |row| row.get(0),
                )?;
                Ok(HabitCount {
                    name: habit.name.clone(),
                    unit: habit.unit.clone(),
                    count,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    /// Um banco de habit-tracker de mentira, com os toques dados.
    fn tracker(touches: &[(&str, &str)], turn: u8) -> (tempfile::TempDir, HabitTracker) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("habitos.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE events (id TEXT PRIMARY KEY, kind TEXT NOT NULL, \
             created_at TEXT NOT NULL, host TEXT NOT NULL)",
        )
        .unwrap();
        for (n, (kind, at)) in touches.iter().enumerate() {
            conn.execute(
                "INSERT INTO events VALUES (?1, ?2, ?3, 'pc')",
                params![n.to_string(), kind, at],
            )
            .unwrap();
        }
        let habits = vec![
            Habit {
                key: "agua".to_owned(),
                name: "Água".to_owned(),
                unit: "copo".to_owned(),
            },
            Habit {
                key: "leitura".to_owned(),
                name: "Leitura".to_owned(),
                unit: "capítulo".to_owned(),
            },
        ];
        let tracker = HabitTracker::new(&path, habits, turn);
        (dir, tracker)
    }

    #[test]
    fn counts_each_habit_inside_the_days() {
        let (_dir, tracker) = tracker(
            &[
                ("agua", "2026-11-10T23:59:59"),
                ("agua", "2026-11-11T00:00:00"),
                ("agua", "2026-11-11T13:00:00"),
                ("agua", "2026-11-12T00:00:00"),
                ("cafe", "2026-11-11T08:00:00"),
            ],
            0,
        );
        let day = date(2026, 11, 11);
        let counts = tracker.count(day, day).unwrap();
        assert_eq!(counts[0].count, 2);
        // Hábito sem toque vem com zero, e hábito fora da lista não vem.
        assert_eq!(counts[1].count, 0);
        assert_eq!(counts.len(), 2);
    }

    #[test]
    fn the_turn_of_the_day_moves_the_edges() {
        let (_dir, tracker) = tracker(
            &[
                ("agua", "2026-11-11T02:00:00"),
                ("agua", "2026-11-12T02:00:00"),
            ],
            4,
        );
        let day = date(2026, 11, 11);
        // Com a virada às 4h, as 2h do dia 11 ainda são do dia 10, e as 2h
        // do dia 12 já são do 11.
        assert_eq!(tracker.count(day, day).unwrap()[0].count, 1);
    }

    #[test]
    fn a_whole_week_adds_up() {
        let (_dir, tracker) = tracker(
            &[
                ("agua", "2026-11-09T10:00:00"),
                ("agua", "2026-11-12T10:00:00"),
                ("agua", "2026-11-15T23:00:00"),
                ("agua", "2026-11-16T10:00:00"),
            ],
            0,
        );
        let counts = tracker
            .count(date(2026, 11, 9), date(2026, 11, 15))
            .unwrap();
        assert_eq!(counts[0].count, 3);
    }

    #[test]
    fn a_missing_database_is_an_error() {
        let tracker = HabitTracker::new(Path::new("/nao/existe/habitos.db"), Vec::new(), 0);
        let day = date(2026, 11, 11);
        assert!(tracker.count(day, day).is_err());
    }
}
