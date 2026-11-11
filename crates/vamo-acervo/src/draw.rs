//! O sorteio da curiosidade do dia.
//!
//! "Sorteio" entre aspas: o mesmo dia dá o mesmo fato em qualquer PC, sem
//! guardar nada. O fato sai da data, por conta.

use crate::date::days_in_month;
use crate::{Banter, Fact};

/// Um dia do calendário, a data de hoje pra quem pede a curiosidade.
///
/// O acervo não lê o relógio: quem chama diz que dia é hoje.
///
/// ```
/// use vamo_acervo::Day;
///
/// assert!(Day::new(2026, 11, 11).is_some());
/// assert!(Day::new(2026, 2, 29).is_none());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Day {
    year: u16,
    month: u8,
    date: u8,
}

impl Day {
    /// O dia, se ele existir no calendário (a partir do ano 1000).
    pub fn new(year: u16, month: u8, day: u8) -> Option<Self> {
        let valid = year >= 1000
            && (1..=12).contains(&month)
            && (1..=days_in_month(year, month)).contains(&day);
        valid.then_some(Self {
            year,
            month,
            date: day,
        })
    }

    /// Quantos dias se passaram desde 1º de janeiro de 1970 (negativo
    /// antes disso). É o relógio do rodízio: um dia a mais, um fato adiante.
    fn number(self) -> i64 {
        // O algoritmo `days_from_civil`, de Howard Hinnant: conta os anos a
        // partir de março, pra que o dia extra do bissexto caia no fim do
        // ano contado.
        let month = i64::from(self.month);
        let year = i64::from(self.year) - i64::from(month <= 2);
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let day_of_year =
            (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(self.date) - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }
}

/// A curiosidade do dia e o motivo de ela ter saído.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Curiosity<'a> {
    /// Um fato que aconteceu neste mesmo dia e mês, em outro ano.
    OnThisDay(&'a Fact),
    /// O fato da vez no rodízio.
    Drawn(&'a Fact),
}

impl<'a> Curiosity<'a> {
    /// O fato, venha de onde vier.
    pub fn fact(self) -> &'a Fact {
        match self {
            Self::OnThisDay(fact) | Self::Drawn(fact) => fact,
        }
    }
}

/// A curiosidade do dia: só fato verificado entra.
///
/// Se algum fato aconteceu neste dia e mês, ele tem prioridade ("Neste
/// dia"). Com mais de um, cada ano mostra um. Sem nenhum, sai o fato da vez
/// num rodízio fixo, que passa por todos os fatos antes de repetir. `None`
/// só quando não tem fato verificado nenhum.
///
/// ```
/// use vamo_acervo::{Curiosity, Day, Fact, curiosity_of_the_day};
///
/// let fact = Fact::from_toml(
///     "campeao-da-libertadores-de-2024",
///     r#"
///     texto = "O Botafogo venceu a Libertadores de 2024."
///     data = "2024-11-30"
///     status = "verificado"
///
///     [[fontes]]
///     nome = "Trivela"
///     url = "https://trivela.com.br/"
///
///     [[fontes]]
///     nome = "ESPN"
///     url = "https://www.espn.com.br/"
///     "#,
/// )?;
/// let facts = [fact];
///
/// let anniversary = curiosity_of_the_day(&facts, Day::new(2026, 11, 30).unwrap());
/// assert!(matches!(anniversary, Some(Curiosity::OnThisDay(_))));
///
/// let other_day = curiosity_of_the_day(&facts, Day::new(2026, 11, 11).unwrap());
/// assert!(matches!(other_day, Some(Curiosity::Drawn(_))));
/// # Ok::<(), vamo_acervo::Error>(())
/// ```
pub fn curiosity_of_the_day(facts: &[Fact], today: Day) -> Option<Curiosity<'_>> {
    let mut verified: Vec<&Fact> = facts.iter().filter(|fact| fact.is_verified()).collect();
    // A ordem não pode depender da ordem em que os arquivos foram lidos.
    verified.sort_by(|a, b| a.id().cmp(b.id()));

    let on_this_day: Vec<&Fact> = verified
        .iter()
        .copied()
        .filter(|fact| {
            fact.date().is_some_and(|date| {
                date.month() == Some(today.month) && date.day() == Some(today.date)
            })
        })
        .collect();
    if !on_this_day.is_empty() {
        let index = usize::from(today.year) % on_this_day.len();
        return Some(Curiosity::OnThisDay(on_this_day[index]));
    }

    rotation(&verified, today).map(Curiosity::Drawn)
}

/// A zoeira do dia, no mesmo rodízio da curiosidade. `None` com a resenha
/// vazia.
pub fn banter_of_the_day(banter: &[Banter], today: Day) -> Option<&Banter> {
    let mut items: Vec<&Banter> = banter.iter().collect();
    items.sort_by(|a, b| a.id().cmp(b.id()));
    rotation(&items, today)
}

/// O item da vez: os itens são embaralhados uma vez, sempre do mesmo jeito,
/// e cada dia anda uma casa. Assim um item só repete depois de todos os
/// outros passarem.
fn rotation<'a, T>(items: &[&'a T], today: Day) -> Option<&'a T> {
    let len = i64::try_from(items.len()).ok().filter(|&len| len > 0)?;
    let order = shuffled(items.len());
    let turn = usize::try_from(today.number().rem_euclid(len)).ok()?;
    Some(items[order[turn]])
}

/// As posições de `0` a `len - 1` embaralhadas pelo Fisher-Yates, com uma
/// semente fixa: o resultado é o mesmo em qualquer máquina e em qualquer
/// versão do Rust.
fn shuffled(len: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..len).collect();
    let mut state: u64 = 0x5EED_1904_0812_0001;
    for last in (1..len).rev() {
        let random = splitmix64(&mut state);
        let bound = u64::try_from(last + 1).expect("um usize cabe num u64");
        let pick = usize::try_from(random % bound).expect("o resto é menor que um usize");
        order.swap(last, pick);
    }
    order
}

/// O `SplitMix64`: um gerador de números pseudoaleatórios pequeno e conhecido.
/// Não serve pra segredo, só pra embaralhar sempre igual.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use proptest::prelude::*;

    use super::*;

    fn fact(id: &str, date: &str, status: &str) -> Fact {
        let toml = format!(
            r#"
            texto = "Fato {id}."
            data = "{date}"
            status = "{status}"

            [[fontes]]
            nome = "Lance!"
            url = "https://www.lance.com.br/"

            [[fontes]]
            nome = "Wikipédia"
            url = "https://pt.wikipedia.org/"
            "#
        );
        Fact::from_toml(id, &toml).unwrap()
    }

    fn day(year: u16, month: u8, day: u8) -> Day {
        Day::new(year, month, day).unwrap()
    }

    fn facts(count: usize) -> Vec<Fact> {
        (0..count)
            .map(|n| fact(&format!("fato-{n:02}"), "1950", "verificado"))
            .collect()
    }

    #[test]
    fn day_numbers_match_the_calendar() {
        assert_eq!(day(1970, 1, 1).number(), 0);
        assert_eq!(day(1970, 1, 2).number(), 1);
        assert_eq!(day(1969, 12, 31).number(), -1);
        assert_eq!(day(2000, 3, 1).number(), 11_017);
        assert_eq!(day(2026, 11, 11).number(), 20_768);
        assert_eq!(day(1904, 8, 12).number(), -23_883);
    }

    #[test]
    fn days_that_do_not_exist_are_refused() {
        assert!(Day::new(2026, 2, 29).is_none());
        assert!(Day::new(2028, 2, 29).is_some());
        assert!(Day::new(2026, 13, 1).is_none());
        assert!(Day::new(2026, 4, 31).is_none());
        assert!(Day::new(999, 1, 1).is_none());
    }

    #[test]
    fn the_same_day_gives_the_same_fact() {
        let facts = facts(7);
        let first = curiosity_of_the_day(&facts, day(2026, 11, 11));
        let second = curiosity_of_the_day(&facts, day(2026, 11, 11));
        assert_eq!(first, second);
    }

    #[test]
    fn the_order_of_the_files_does_not_matter() {
        let facts = facts(7);
        let mut reversed = facts.clone();
        reversed.reverse();
        for n in 0..30 {
            let today = day(2026, 11, 1 + n % 28);
            assert_eq!(
                curiosity_of_the_day(&facts, today).map(|c| c.fact().id()),
                curiosity_of_the_day(&reversed, today).map(|c| c.fact().id()),
            );
        }
    }

    #[test]
    fn fixed_dates_give_fixed_facts() {
        // Se este teste quebrar, a curiosidade de cada dia mudou pra todo
        // mundo: só mude de propósito.
        let facts = facts(7);
        let picks: Vec<String> = [day(2026, 11, 11), day(2026, 11, 12), day(2027, 1, 1)]
            .into_iter()
            .map(|today| {
                curiosity_of_the_day(&facts, today)
                    .unwrap()
                    .fact()
                    .id()
                    .to_string()
            })
            .collect();
        insta::assert_debug_snapshot!(picks);
    }

    #[test]
    fn nothing_repeats_before_everyone_has_played() {
        let facts = facts(50);
        let seen: BTreeSet<&str> = (0..50)
            .map(|n| {
                let today = Day::new(2026, 1, 1).unwrap();
                let shifted = Day::from_number(today.number() + n);
                curiosity_of_the_day(&facts, shifted)
                    .unwrap()
                    .fact()
                    .id()
                    .as_str()
            })
            .collect();
        assert_eq!(seen.len(), 50);
    }

    #[test]
    fn a_fact_from_this_day_comes_first() {
        let mut facts = facts(7);
        facts.push(fact("goleada", "1909-05-30", "verificado"));
        let pick = curiosity_of_the_day(&facts, day(2026, 5, 30)).unwrap();
        assert!(matches!(pick, Curiosity::OnThisDay(_)));
        assert_eq!(pick.fact().id().as_str(), "goleada");
    }

    #[test]
    fn two_facts_on_the_same_day_take_turns_by_year() {
        let facts = [
            fact("primeiro", "1910-07-01", "verificado"),
            fact("segundo", "1960-07-01", "verificado"),
        ];
        let this_year = curiosity_of_the_day(&facts, day(2026, 7, 1)).unwrap();
        let next_year = curiosity_of_the_day(&facts, day(2027, 7, 1)).unwrap();
        assert_ne!(this_year.fact().id(), next_year.fact().id());
    }

    #[test]
    fn a_fact_to_check_never_shows_up() {
        let facts = [
            fact("conferir", "1904-08-12", "a-conferir"),
            fact("certo", "1950", "verificado"),
        ];
        for n in 1..=28 {
            let pick = curiosity_of_the_day(&facts, day(2026, 8, n)).unwrap();
            assert_eq!(pick.fact().id().as_str(), "certo");
        }
        assert_eq!(curiosity_of_the_day(&facts[..1], day(2026, 8, 12)), None);
    }

    #[test]
    fn an_empty_acervo_has_no_curiosity() {
        assert_eq!(curiosity_of_the_day(&[], day(2026, 11, 11)), None);
        assert_eq!(banter_of_the_day(&[], day(2026, 11, 11)), None);
    }

    proptest! {
        #[test]
        fn any_day_gives_a_fact(year in 1000_u16..3000, month in 1_u8..=12, d in 1_u8..=28) {
            let facts = facts(5);
            prop_assert!(curiosity_of_the_day(&facts, day(year, month, d)).is_some());
        }

        #[test]
        fn shuffling_keeps_every_position(len in 0_usize..200) {
            let mut order = shuffled(len);
            order.sort_unstable();
            prop_assert_eq!(order, (0..len).collect::<Vec<_>>());
        }
    }

    impl Day {
        /// O caminho de volta do `number`, só pros testes.
        fn from_number(number: i64) -> Self {
            let mut today = Day::new(1970, 1, 1).unwrap();
            let step = if number >= 0 { 1 } else { -1 };
            while today.number() != number {
                today = today.shifted(step);
            }
            today
        }

        fn shifted(self, step: i64) -> Self {
            let (mut year, mut month, mut d) = (self.year, self.month, i64::from(self.date) + step);
            if d < 1 {
                month = if month == 1 {
                    year -= 1;
                    12
                } else {
                    month - 1
                };
                d = i64::from(days_in_month(year, month));
            } else if d > i64::from(days_in_month(year, month)) {
                month = if month == 12 {
                    year += 1;
                    1
                } else {
                    month + 1
                };
                d = 1;
            }
            Day::new(year, month, u8::try_from(d).unwrap()).unwrap()
        }
    }
}
