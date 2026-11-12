//! As atividades fixas: os compromissos recorrentes da agenda que aparecem
//! na tela Hoje como itens de marcar, sem virar tarefa no banco.

use std::collections::BTreeMap;

use crate::title::shorten;
use crate::{Appointment, Score, When};

/// Um compromisso recorrente do dia, com o título curto pra escalação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedActivity {
    /// O UID do evento na agenda. Com o dia, é a chave da marcação.
    pub uid: String,
    /// O título curto: o apelido da config ou o título do evento cortado
    /// no tamanho do caderno.
    pub title: String,
    /// Quando.
    pub when: When,
}

/// As atividades fixas do dia: os compromissos recorrentes de `agenda`, na
/// ordem em que vieram, cada UID uma vez só.
///
/// `aliases` liga o título do evento (sem ligar pra maiúscula e espaço em
/// volta) ao apelido.
///
/// ```
/// use std::collections::BTreeMap;
/// use jiff::civil::date;
/// use vamo_core::{Appointment, When, fixed_activities};
///
/// let gym = Appointment {
///     uid: "academia@exemplo".to_owned(),
///     title: "Treino de musculação na academia do bairro".to_owned(),
///     when: When::AllDay(date(2026, 11, 12)),
///     recurring: true,
///     task: None,
/// };
/// let aliases = BTreeMap::from([(
///     "treino de musculação na academia do bairro".to_owned(),
///     "Academia".to_owned(),
/// )]);
/// assert_eq!(fixed_activities(&[gym.clone()], &BTreeMap::new())[0].title, "Treino de musculação na academia");
/// assert_eq!(fixed_activities(&[gym], &aliases)[0].title, "Academia");
/// ```
pub fn fixed_activities(
    agenda: &[Appointment],
    aliases: &BTreeMap<String, String>,
) -> Vec<FixedActivity> {
    let mut found: Vec<FixedActivity> = Vec::new();
    for appointment in agenda.iter().filter(|appointment| appointment.recurring) {
        if found.iter().any(|activity| activity.uid == appointment.uid) {
            continue;
        }
        let key = appointment.title.trim().to_lowercase();
        let title = aliases
            .get(&key)
            .cloned()
            .unwrap_or_else(|| shorten(&appointment.title));
        found.push(FixedActivity {
            uid: appointment.uid.clone(),
            title,
            when: appointment.when,
        });
    }
    found
}

impl Score {
    /// O placar somando as atividades fixas da agenda: `done` marcadas de
    /// `planned`.
    #[must_use]
    pub fn plus(self, done: usize, planned: usize) -> Self {
        Self {
            done: self.done + done,
            planned: self.planned + planned,
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    fn event(uid: &str, title: &str, recurring: bool) -> Appointment {
        Appointment {
            uid: uid.to_owned(),
            title: title.to_owned(),
            when: When::AllDay(date(2026, 11, 12)),
            recurring,
            task: None,
        }
    }

    #[test]
    fn only_recurring_events_once_each() {
        let agenda = [
            event("a", "Caminhada", true),
            event("b", "Dentista", false),
            event("a", "Caminhada", true),
        ];
        let found = fixed_activities(&agenda, &BTreeMap::new());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Caminhada");
    }

    #[test]
    fn the_alias_ignores_case_and_spaces() {
        let aliases = BTreeMap::from([("reunião diária".to_owned(), "Daily".to_owned())]);
        let found = fixed_activities(&[event("a", "  Reunião Diária ", true)], &aliases);
        assert_eq!(found[0].title, "Daily");
    }

    #[test]
    fn the_score_adds_up() {
        let score = Score {
            done: 1,
            planned: 3,
        }
        .plus(1, 2);
        assert_eq!(
            score,
            Score {
                done: 2,
                planned: 5
            }
        );
    }
}
