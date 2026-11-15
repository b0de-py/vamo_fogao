//! As regras da sincronia entre máquinas: o número de tarefa repetido e a
//! marcação da agenda que chega de outra máquina.
//!
//! Juntar dois logs é só juntar os eventos (a [`Projection`] cuida do
//! resto), com uma exceção: o número da tarefa. Cada máquina dá o próximo
//! número livre que ela conhece, então duas máquinas sem sincronizar podem
//! criar duas tarefas diferentes com o mesmo `T-###`. A regra aqui decide
//! quem troca de número.

use std::collections::{BTreeMap, BTreeSet};

use jiff::Timestamp;
use jiff::civil::Date;

use crate::{Change, Event, EventId, Machine, Projection, TaskId};

/// Uma tarefa desta máquina que troca de número, com os eventos dela.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Renumber {
    /// O número que a tarefa tinha aqui.
    pub from: TaskId,
    /// O número novo, livre nas duas máquinas.
    pub to: TaskId,
    /// Os eventos desta máquina que passam pro número novo.
    pub events: Vec<EventId>,
}

/// As tarefas desta máquina que precisam trocar de número antes de juntar o
/// log com o das outras.
///
/// - `local`: o log desta máquina, com o que já veio das outras.
/// - `own`: o nome desta máquina.
/// - `exported`: os IDs dos eventos desta máquina que já estão no backup.
/// - `remote`: os eventos que chegaram das outras máquinas.
///
/// Há colisão quando uma tarefa que esta máquina criou e ainda não exportou
/// tem o mesmo número de uma tarefa que outra máquina criou. Quem troca é
/// sempre quem ainda não exportou: o que já está no backup outra máquina
/// pode ter visto, e mudar de número ali faria a tarefa sumir do outro
/// lado. Os eventos não exportados são só desta máquina, então trocar o
/// número deles não muda nada que alguém já leu.
///
/// O número novo é o próximo livre depois do maior que aparece nos dois
/// lados, e cada tarefa renumerada pega um diferente.
pub fn renumber_collisions(
    local: &[Event],
    own: &Machine,
    exported: &BTreeSet<EventId>,
    remote: &[Event],
) -> Vec<Renumber> {
    let unexported: Vec<&Event> = local
        .iter()
        .filter(|event| &event.machine == own && !exported.contains(&event.id))
        .collect();
    let created_locally: BTreeMap<TaskId, EventId> = unexported
        .iter()
        .filter(|event| matches!(event.change, Change::Created { .. }))
        .map(|event| (event.task, event.id))
        .collect();
    let created_remotely: BTreeMap<TaskId, EventId> = remote
        .iter()
        .filter(|event| matches!(event.change, Change::Created { .. }))
        .map(|event| (event.task, event.id))
        .collect();

    let mut next = local.iter().chain(remote).collect::<Projection>().next_id();
    let mut plan = Vec::new();
    for (&task, &here) in &created_locally {
        let collides = created_remotely
            .get(&task)
            .is_some_and(|&there| there != here);
        if !collides {
            continue;
        }
        let Some(to) = next else {
            // Os números acabaram: na prática não acontece, e a tarefa fica
            // com o número dela.
            break;
        };
        next = to.next();
        plan.push(Renumber {
            from: task,
            to,
            events: unexported
                .iter()
                .filter(|event| event.task == task)
                .map(|event| event.id)
                .collect(),
        });
    }
    plan
}

/// O começo da linha das notas que marca de qual reforço a tarefa veio:
/// `reforço: <chave>`.
pub const REINFORCEMENT_NOTE: &str = "reforço: ";

/// A linha das notas que marca a tarefa criada pelo reforço de `key`.
pub fn reinforcement_note(key: &str) -> String {
    format!("{REINFORCEMENT_NOTE}{key}")
}

impl Projection {
    /// Se o reforço de chave `key` já virou tarefa, aqui ou em outra
    /// máquina. A chave fica nas notas da tarefa, e as notas viajam com o
    /// log: é isso que impede o mesmo reforço de virar duas tarefas, mesmo
    /// lido por duas máquinas em dias diferentes.
    pub fn has_reinforcement(&self, key: &str) -> bool {
        let note = reinforcement_note(key);
        self.tasks()
            .any(|task| task.notes().lines().any(|line| line.trim() == note))
    }
}

/// Uma marcação de atividade fixa da agenda, com o instante em que foi
/// feita ou desfeita.
///
/// A marcação não entra no log de eventos (a ADR 0009 conta por quê), mas
/// viaja entre as máquinas com a mesma regra do log: a mais recente de cada
/// `(dia, uid)` ganha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgendaMark {
    /// O dia da marcação.
    pub day: Date,
    /// O UID do evento na agenda.
    pub uid: String,
    /// Se ficou marcada.
    pub done: bool,
    /// Quando foi marcada ou desmarcada.
    pub at: Timestamp,
}

impl AgendaMark {
    /// Se esta marcação ganha da `other` do mesmo `(dia, uid)`: só se for
    /// mais recente. No empate, fica a que já estava.
    pub fn wins_over(&self, other: &Self) -> bool {
        self.at > other.at
    }
}

#[cfg(test)]
mod tests {
    use uuid::Builder;

    use super::*;
    use crate::{Fit, Status};

    struct Machines {
        n: u64,
    }

    impl Machines {
        fn event(&mut self, machine: &str, task: u32, change: Change) -> Event {
            self.n += 1;
            Event {
                id: EventId::new(
                    Builder::from_unix_timestamp_millis(self.n, &[u8::try_from(task).unwrap(); 10])
                        .into_uuid(),
                )
                .unwrap(),
                at: Timestamp::from_second(i64::try_from(self.n).unwrap()).unwrap(),
                machine: machine.parse().unwrap(),
                task: TaskId::new(task).unwrap(),
                change,
            }
        }

        fn created(&mut self, machine: &str, task: u32, title: &str) -> Event {
            self.event(
                machine,
                task,
                Change::Created {
                    title: title.parse().unwrap(),
                    area: "casa".parse().unwrap(),
                    fit: Fit::Gap,
                    status: Status::Ready,
                },
            )
        }
    }

    fn ids(events: &[&Event]) -> BTreeSet<EventId> {
        events.iter().map(|event| event.id).collect()
    }

    #[test]
    fn the_same_number_created_on_both_sides_moves_the_unexported_one() {
        let mut m = Machines { n: 0 };
        let shared = m.created("notebook", 1, "Trocar o filtro");
        let mine = m.created("notebook", 2, "Lavar o carro");
        let note = m.event("notebook", 2, Change::Notes("Com sabão".to_owned()));
        let theirs = m.created("pc", 2, "Pagar a conta");
        let local = vec![shared.clone(), mine.clone(), note.clone()];

        let plan = renumber_collisions(
            &local,
            &"notebook".parse().unwrap(),
            &ids(&[&shared]),
            &[theirs],
        );
        assert_eq!(
            plan,
            [Renumber {
                from: TaskId::new(2).unwrap(),
                to: TaskId::new(3).unwrap(),
                events: vec![mine.id, note.id],
            }]
        );
    }

    #[test]
    fn an_exported_task_keeps_its_number() {
        let mut m = Machines { n: 0 };
        let mine = m.created("notebook", 1, "Lavar o carro");
        let theirs = m.created("pc", 1, "Pagar a conta");

        // O notebook já exportou: quem troca é o PC, do lado de lá.
        let plan = renumber_collisions(
            std::slice::from_ref(&mine),
            &"notebook".parse().unwrap(),
            &ids(&[&mine]),
            &[theirs],
        );
        assert_eq!(plan, []);
    }

    #[test]
    fn changes_to_a_task_from_the_other_side_are_not_a_collision() {
        let mut m = Machines { n: 0 };
        let theirs = m.created("pc", 1, "Pagar a conta");
        let mine = m.event("notebook", 1, Change::Notes("Até sexta".to_owned()));

        let plan = renumber_collisions(
            &[theirs.clone(), mine],
            &"notebook".parse().unwrap(),
            &BTreeSet::new(),
            &[theirs],
        );
        assert_eq!(plan, []);
    }

    #[test]
    fn each_renumbered_task_gets_its_own_number() {
        let mut m = Machines { n: 0 };
        let local = vec![
            m.created("notebook", 1, "Lavar o carro"),
            m.created("notebook", 2, "Regar as plantas"),
        ];
        let remote = vec![
            m.created("pc", 1, "Pagar a conta"),
            m.created("pc", 2, "Trocar o filtro"),
            m.created("pc", 3, "Ler o livro"),
        ];

        let plan = renumber_collisions(
            &local,
            &"notebook".parse().unwrap(),
            &BTreeSet::new(),
            &remote,
        );
        let moves: Vec<(u32, u32)> = plan
            .iter()
            .map(|r| (r.from.number(), r.to.number()))
            .collect();
        assert_eq!(moves, [(1, 4), (2, 5)]);
    }

    proptest::proptest! {
        /// Duas máquinas criam tarefas sem sincronizar, com os números se
        /// cruzando: depois de renumerar e juntar, nenhuma tarefa some.
        #[test]
        fn no_task_is_lost_after_the_merge(here in 0u32..6, there in 0u32..6) {
            let mut m = Machines { n: 0 };
            let local: Vec<Event> =
                (1..=here).map(|n| m.created("notebook", n, "Lavar o carro")).collect();
            let remote: Vec<Event> =
                (1..=there).map(|n| m.created("pc", n, "Pagar a conta")).collect();

            let plan = renumber_collisions(
                &local,
                &"notebook".parse().unwrap(),
                &BTreeSet::new(),
                &remote,
            );
            let mut moved = local.clone();
            for renumber in &plan {
                for event in &mut moved {
                    if renumber.events.contains(&event.id) {
                        event.task = renumber.to;
                    }
                }
            }
            let merged: Projection = moved.iter().chain(&remote).collect();
            proptest::prop_assert_eq!(merged.tasks().count(), (here + there) as usize);
        }
    }

    #[test]
    fn a_reinforcement_is_found_by_the_note() {
        let mut m = Machines { n: 0 };
        let created = m.created("pc", 1, "Comprar o filtro");
        let note = m.event(
            "pc",
            1,
            Change::Notes(format!(
                "Na loja da esquina\n\n{}",
                reinforcement_note("claude-1")
            )),
        );
        let projection: Projection = [&created, &note].into_iter().collect();
        assert!(projection.has_reinforcement("claude-1"));
        assert!(!projection.has_reinforcement("claude-2"));
        assert!(!projection.has_reinforcement("claude"));
    }

    #[test]
    fn the_newest_mark_wins() {
        let day = jiff::civil::date(2026, 11, 11);
        let older = AgendaMark {
            day,
            uid: "evento@agenda".to_owned(),
            done: true,
            at: Timestamp::from_second(10).unwrap(),
        };
        let newer = AgendaMark {
            done: false,
            at: Timestamp::from_second(20).unwrap(),
            ..older.clone()
        };
        assert!(newer.wins_over(&older));
        assert!(!older.wins_over(&newer));
        assert!(!older.wins_over(&older));
    }
}
