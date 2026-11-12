//! A regra de recorrência do ICS (`RRULE`), no pedaço que as agendas usam
//! no dia a dia: todo dia, toda semana (em certos dias), todo mês (num dia
//! do mês ou no "segundo sábado") e todo ano, com intervalo, `COUNT` e
//! `UNTIL`.
//!
//! A expansão acontece no horário do próprio evento (a "parede" do fuso
//! dele): uma reunião às 9h continua às 9h depois da virada do horário de
//! verão, que é o que a RFC 5545 pede e o que as agendas fazem.
//!
//! Regra com parte que este leitor não conhece (`BYSETPOS`, `BYHOUR`,
//! frequência de hora em hora...) é erro: melhor avisar do que inventar
//! ocorrência.

use jiff::civil::{Date, DateTime, Weekday};
use jiff::{Span, ToSpan};

use crate::Error;

/// O teto de períodos olhados numa expansão. Uma regra diária de 10 anos
/// tem uns 3.650; o teto só existe pra regra quebrada não travar o app.
const MAX_PERIODS: u32 = 50_000;

/// De quanto em quanto tempo a regra repete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// Uma regra de recorrência já lida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rule {
    frequency: Frequency,
    interval: i32,
    count: Option<u32>,
    /// O `UNTIL` ainda em texto: quem converte pro horário do evento é quem
    /// expande, porque depende do fuso dele.
    pub(crate) until: Option<String>,
    /// `BYDAY`: o dia da semana, com a posição no mês (`2SA`, `-1FR`) quando
    /// tem.
    by_day: Vec<(Option<i8>, Weekday)>,
    by_month_day: Vec<i8>,
    by_month: Vec<i8>,
}

impl Rule {
    /// Lê `FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;UNTIL=20261231T000000Z`.
    pub(crate) fn parse(text: &str) -> Result<Self, Error> {
        let unsupported = |part: &str| Error::UnsupportedRule {
            rule: text.to_owned(),
            part: part.to_owned(),
        };
        let mut frequency = None;
        let mut rule = Self {
            frequency: Frequency::Daily,
            interval: 1,
            count: None,
            until: None,
            by_day: Vec::new(),
            by_month_day: Vec::new(),
            by_month: Vec::new(),
        };
        for part in text.split(';').filter(|part| !part.is_empty()) {
            let (key, value) = part.split_once('=').ok_or_else(|| unsupported(part))?;
            match key.to_ascii_uppercase().as_str() {
                "FREQ" => {
                    frequency = Some(match value.to_ascii_uppercase().as_str() {
                        "DAILY" => Frequency::Daily,
                        "WEEKLY" => Frequency::Weekly,
                        "MONTHLY" => Frequency::Monthly,
                        "YEARLY" => Frequency::Yearly,
                        _ => return Err(unsupported(part)),
                    });
                }
                "INTERVAL" => {
                    rule.interval = value
                        .parse()
                        .ok()
                        .filter(|&n| n > 0)
                        .ok_or_else(|| unsupported(part))?;
                }
                "COUNT" => rule.count = Some(value.parse().map_err(|_| unsupported(part))?),
                "UNTIL" => rule.until = Some(value.to_owned()),
                "BYDAY" => {
                    for day in value.split(',') {
                        rule.by_day
                            .push(by_day(day).ok_or_else(|| unsupported(part))?);
                    }
                }
                "BYMONTHDAY" => {
                    for day in value.split(',') {
                        let day: i8 = day.parse().map_err(|_| unsupported(part))?;
                        if day == 0 || !(-31..=31).contains(&day) {
                            return Err(unsupported(part));
                        }
                        rule.by_month_day.push(day);
                    }
                }
                "BYMONTH" => {
                    for month in value.split(',') {
                        let month: i8 = month.parse().map_err(|_| unsupported(part))?;
                        if !(1..=12).contains(&month) {
                            return Err(unsupported(part));
                        }
                        rule.by_month.push(month);
                    }
                }
                // O começo da semana só muda o resultado em regra semanal
                // com intervalo e dias antes do começo da semana; as agendas
                // usam segunda ou domingo, e a semana aqui começa na segunda.
                "WKST" => {}
                _ => return Err(unsupported(part)),
            }
        }
        rule.frequency = frequency.ok_or_else(|| unsupported("FREQ"))?;
        Ok(rule)
    }

    /// As ocorrências a partir de `start` (que é a primeira), até o dia
    /// `last` ou até o `until`, o que vier antes, respeitando o `COUNT`.
    /// Todas no horário de `start`.
    pub(crate) fn occurrences(
        &self,
        start: DateTime,
        last: Date,
        until: Option<DateTime>,
    ) -> Vec<DateTime> {
        let mut found = Vec::new();
        let mut counted: u32 = 0;
        for period in 0..MAX_PERIODS {
            let Some(anchor) = self.period_start(start.date(), period) else {
                break;
            };
            if anchor > last {
                break;
            }
            for day in self.days_in_period(anchor, start.date()) {
                if day < start.date() {
                    continue;
                }
                let at = day.to_datetime(start.time());
                if day > last || until.is_some_and(|until| at > until) {
                    return found;
                }
                if self.count.is_some_and(|count| counted >= count) {
                    return found;
                }
                counted += 1;
                found.push(at);
            }
        }
        found
    }

    /// O primeiro dia do período `n`, contando do período do começo.
    fn period_start(&self, start: Date, n: u32) -> Option<Date> {
        let steps = i64::from(n) * i64::from(self.interval);
        let span: Span = match self.frequency {
            Frequency::Daily => steps.days(),
            Frequency::Weekly => (steps * 7).days(),
            Frequency::Monthly => steps.months(),
            Frequency::Yearly => steps.years(),
        };
        let base = match self.frequency {
            Frequency::Daily => start,
            // A semana começa na segunda.
            Frequency::Weekly => start
                .checked_sub(i64::from(start.weekday().to_monday_zero_offset()).days())
                .ok()?,
            Frequency::Monthly => start.first_of_month(),
            Frequency::Yearly => start.first_of_year(),
        };
        base.checked_add(span).ok()
    }

    /// Os dias de um período, em ordem.
    fn days_in_period(&self, anchor: Date, start: Date) -> Vec<Date> {
        let mut days: Vec<Date> = match self.frequency {
            Frequency::Daily => vec![anchor],
            Frequency::Weekly => {
                let weekdays: Vec<Weekday> = if self.by_day.is_empty() {
                    vec![start.weekday()]
                } else {
                    self.by_day.iter().map(|&(_, weekday)| weekday).collect()
                };
                weekdays
                    .into_iter()
                    .filter_map(|weekday| {
                        anchor
                            .checked_add(i64::from(weekday.to_monday_zero_offset()).days())
                            .ok()
                    })
                    .collect()
            }
            Frequency::Monthly => self.days_in_month(anchor, start),
            Frequency::Yearly => {
                let months: Vec<i8> = if self.by_month.is_empty() {
                    vec![start.month()]
                } else {
                    self.by_month.clone()
                };
                months
                    .into_iter()
                    .filter_map(|month| Date::new(anchor.year(), month, 1).ok())
                    .flat_map(|first| self.days_in_month(first, start))
                    .collect()
            }
        };
        days.sort_unstable();
        days.dedup();
        days
    }

    /// Os dias de um mês: pelo `BYMONTHDAY`, pelo `BYDAY` (com ou sem
    /// posição) ou, sem nenhum dos dois, o mesmo dia do começo. Mês que não
    /// tem o dia (31 em abril) fica sem ocorrência, como manda a RFC.
    fn days_in_month(&self, first: Date, start: Date) -> Vec<Date> {
        let length = first.days_in_month();
        if !self.by_month_day.is_empty() {
            return self
                .by_month_day
                .iter()
                .filter_map(|&day| {
                    let day = if day > 0 { day } else { length + 1 + day };
                    first.with().day(day).build().ok()
                })
                .collect();
        }
        if !self.by_day.is_empty() {
            let mut days = Vec::new();
            for &(position, weekday) in &self.by_day {
                match position {
                    Some(nth) => days.extend(first.nth_weekday_of_month(nth, weekday).ok()),
                    None => days.extend(
                        (1..=length)
                            .filter_map(|day| first.with().day(day).build().ok())
                            .filter(|day| day.weekday() == weekday),
                    ),
                }
            }
            return days;
        }
        first.with().day(start.day()).build().into_iter().collect()
    }
}

/// `MO`, `2SA`, `-1FR`.
fn by_day(text: &str) -> Option<(Option<i8>, Weekday)> {
    let text = text.trim();
    let split = text.len().checked_sub(2)?;
    let (position, name) = text.split_at(split);
    let weekday = match name.to_ascii_uppercase().as_str() {
        "MO" => Weekday::Monday,
        "TU" => Weekday::Tuesday,
        "WE" => Weekday::Wednesday,
        "TH" => Weekday::Thursday,
        "FR" => Weekday::Friday,
        "SA" => Weekday::Saturday,
        "SU" => Weekday::Sunday,
        _ => return None,
    };
    let position = match position {
        "" => None,
        number => Some(number.trim_start_matches('+').parse().ok()?),
    };
    Some((position, weekday))
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    fn days(rule: &str, start: DateTime, last: Date) -> Vec<String> {
        Rule::parse(rule)
            .unwrap()
            .occurrences(start, last, None)
            .into_iter()
            .map(|at| at.date().to_string())
            .collect()
    }

    #[test]
    fn weekly_on_chosen_days() {
        // 2026-11-09 é uma segunda.
        let start = date(2026, 11, 9).at(9, 0, 0, 0);
        assert_eq!(
            days("FREQ=WEEKLY;BYDAY=MO,WE", start, date(2026, 11, 18)),
            ["2026-11-09", "2026-11-11", "2026-11-16", "2026-11-18"]
        );
    }

    #[test]
    fn every_other_week_with_count() {
        let start = date(2026, 11, 10).at(9, 0, 0, 0);
        assert_eq!(
            days("FREQ=WEEKLY;INTERVAL=2;COUNT=3", start, date(2027, 12, 31)),
            ["2026-11-10", "2026-11-24", "2026-12-08"]
        );
    }

    #[test]
    fn daily_stops_at_until() {
        let rule = Rule::parse("FREQ=DAILY;UNTIL=20261113T090000").unwrap();
        let start = date(2026, 11, 11).at(9, 0, 0, 0);
        let found = rule.occurrences(
            start,
            date(2026, 12, 31),
            Some(date(2026, 11, 13).at(9, 0, 0, 0)),
        );
        assert_eq!(found.len(), 3);
    }

    #[test]
    fn monthly_on_the_second_saturday_and_on_the_last_day() {
        let start = date(2026, 11, 14).at(10, 0, 0, 0);
        assert_eq!(
            days("FREQ=MONTHLY;BYDAY=2SA", start, date(2027, 1, 31)),
            ["2026-11-14", "2026-12-12", "2027-01-09"]
        );
        let start = date(2026, 11, 30).at(10, 0, 0, 0);
        assert_eq!(
            days("FREQ=MONTHLY;BYMONTHDAY=-1", start, date(2027, 2, 28)),
            ["2026-11-30", "2026-12-31", "2027-01-31", "2027-02-28"]
        );
    }

    #[test]
    fn monthly_skips_months_without_the_day() {
        let start = date(2027, 1, 31).at(8, 0, 0, 0);
        assert_eq!(
            days("FREQ=MONTHLY", start, date(2027, 4, 30)),
            ["2027-01-31", "2027-03-31"]
        );
    }

    #[test]
    fn yearly_birthday() {
        let start = date(2026, 3, 5).at(0, 0, 0, 0);
        assert_eq!(
            days("FREQ=YEARLY", start, date(2028, 12, 31)),
            ["2026-03-05", "2027-03-05", "2028-03-05"]
        );
    }

    #[test]
    fn unknown_parts_are_an_error() {
        for rule in [
            "FREQ=HOURLY",
            "FREQ=MONTHLY;BYSETPOS=-1",
            "BYDAY=MO",
            "FREQ=DAILY;INTERVAL=0",
        ] {
            assert!(Rule::parse(rule).is_err(), "{rule}");
        }
    }
}
