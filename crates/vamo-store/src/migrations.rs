//! As migrações do esquema do banco.
//!
//! Cada migração é um arquivo `.sql` em `migrations/`, embutido no binário
//! na compilação. O SQLite guarda no cabeçalho do arquivo um número livre,
//! o `user_version`, e ele vira a versão do esquema: o número de migrações
//! já aplicadas. Abrir o banco aplica as que faltam, em ordem.

use rusqlite::Connection;

use crate::Error;

/// As migrações, em ordem. A posição na lista é a versão: a primeira leva o
/// esquema pra versão 1, a segunda pra 2, e assim por diante.
///
/// Migração que já foi publicada não muda nunca: mudança no esquema é uma
/// migração nova no fim da lista.
const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001-log-de-eventos.sql"),
    include_str!("../migrations/0002-escalacao-e-marcacao.sql"),
    include_str!("../migrations/0003-marcacao-da-agenda.sql"),
];

/// A versão do esquema que esta versão do vamo conhece.
pub(crate) fn latest() -> u32 {
    u32::try_from(MIGRATIONS.len()).expect("a lista de migrações cabe num u32")
}

/// A versão do esquema do banco aberto.
pub(crate) fn version(conn: &Connection) -> Result<u32, Error> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

/// Aplica as migrações que faltam, cada uma na própria transação.
///
/// Se uma migração falha, a transação dela é desfeita e o banco fica na
/// versão anterior, inteiro.
pub(crate) fn run(conn: &mut Connection) -> Result<(), Error> {
    let current = version(conn)?;
    let known = latest();
    if current > known {
        return Err(Error::NewerSchema {
            found: current,
            known,
        });
    }

    for (version, sql) in (1..).zip(MIGRATIONS).skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        // O `user_version` é escrito dentro da transação, então ele só muda
        // se a migração inteira der certo.
        tx.pragma_update(None, "user_version", version)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_database_goes_to_the_latest_version() {
        let mut conn = Connection::open_in_memory().unwrap();
        assert_eq!(version(&conn).unwrap(), 0);

        run(&mut conn).unwrap();

        assert_eq!(version(&conn).unwrap(), latest());
    }

    #[test]
    fn running_twice_changes_nothing() {
        let mut conn = Connection::open_in_memory().unwrap();
        run(&mut conn).unwrap();
        run(&mut conn).unwrap();
        assert_eq!(version(&conn).unwrap(), latest());
    }

    #[test]
    fn a_database_from_a_newer_version_is_left_alone() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", latest() + 1)
            .unwrap();

        let result = run(&mut conn);

        assert!(matches!(
            result,
            Err(Error::NewerSchema { found, known }) if found == latest() + 1 && known == latest()
        ));
        assert_eq!(version(&conn).unwrap(), latest() + 1);
    }
}
