//! O intervalo de uma tarefa que volta: de tantos em tantos meses ou semanas.

use std::fmt;
use std::str::FromStr;

use jiff::ToSpan;
use jiff::civil::Date;

use crate::Error;

/// Quanto tempo a tarefa espera, depois do gol, pra voltar pro banco.
///
/// A tarefa que volta não tem horário: leva gol normal, some da vista e
/// reaparece quando o intervalo passa, contado a partir do dia do gol. Nos
/// arquivos e na CLI o intervalo se escreve com um número e uma letra:
/// `1m` (todo mês), `2s` (a cada duas semanas). O número vai de 1 a 99.
///
/// ```
/// use jiff::civil::date;
/// use vamo_core::Interval;
///
/// let monthly: Interval = "1m".parse()?;
/// assert_eq!(monthly.after(date(2026, 11, 15)), Some(date(2026, 12, 15)));
///
/// // Mês curto não estoura: 31 de janeiro mais um mês é o fim de fevereiro.
/// assert_eq!(monthly.after(date(2027, 1, 31)), Some(date(2027, 2, 28)));
/// assert_eq!("2s".parse::<Interval>()?.to_string(), "2s");
/// # Ok::<(), vamo_core::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Interval {
    /// A cada tantas semanas.
    Weeks(u8),
    /// A cada tantos meses.
    Months(u8),
}

/// O maior número que um intervalo aceita.
const MAX: u8 = 99;

impl Interval {
    /// O dia em que a tarefa volta, contando a partir do dia do gol. `None`
    /// só se a conta passar do último dia que o calendário suporta.
    pub fn after(self, day: Date) -> Option<Date> {
        match self {
            Self::Weeks(n) => day.checked_add(i64::from(n).weeks()).ok(),
            Self::Months(n) => day.checked_add(i64::from(n).months()).ok(),
        }
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Weeks(n) => write!(f, "{n}s"),
            Self::Months(n) => write!(f, "{n}m"),
        }
    }
}

impl FromStr for Interval {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || Error::InvalidInterval {
            input: s.to_owned(),
        };
        // A unidade é a última letra; `strip_suffix` não pânica em letra
        // com acento, que `split_at` por byte faria.
        let (number, unit) = s
            .strip_suffix('s')
            .map(|number| (number, "s"))
            .or_else(|| s.strip_suffix('m').map(|number| (number, "m")))
            .ok_or_else(invalid)?;
        let n: u8 = number.parse().map_err(|_| invalid())?;
        if !(1..=MAX).contains(&n) {
            return Err(invalid());
        }
        match unit {
            "s" => Ok(Self::Weeks(n)),
            "m" => Ok(Self::Months(n)),
            _ => Err(invalid()),
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn reads_months_and_weeks() {
        assert_eq!("1m".parse(), Ok(Interval::Months(1)));
        assert_eq!("12m".parse(), Ok(Interval::Months(12)));
        assert_eq!("3s".parse(), Ok(Interval::Weeks(3)));
    }

    #[test]
    fn rejects_what_is_not_an_interval() {
        for input in [
            "", "m", "0m", "100s", "1", "1d", "-1m", "um mês", "1 m", "é", "1é",
        ] {
            assert_eq!(
                input.parse::<Interval>(),
                Err(Error::InvalidInterval {
                    input: input.to_owned()
                }),
                "{input:?}"
            );
        }
    }

    #[test]
    fn a_month_after_clamps_to_the_end_of_a_short_month() {
        let monthly = Interval::Months(1);
        assert_eq!(monthly.after(date(2026, 11, 15)), Some(date(2026, 12, 15)));
        assert_eq!(monthly.after(date(2026, 12, 31)), Some(date(2027, 1, 31)));
        assert_eq!(monthly.after(date(2027, 1, 31)), Some(date(2027, 2, 28)));
        assert_eq!(monthly.after(date(2028, 1, 31)), Some(date(2028, 2, 29)));
    }

    #[test]
    fn weeks_are_seven_days_each() {
        assert_eq!(
            Interval::Weeks(2).after(date(2026, 11, 15)),
            Some(date(2026, 11, 29))
        );
    }

    proptest! {
        #[test]
        fn display_then_parse_gives_the_same_interval(n in 1..=MAX, months: bool) {
            let interval = if months { Interval::Months(n) } else { Interval::Weeks(n) };
            prop_assert_eq!(interval.to_string().parse(), Ok(interval));
        }

        #[test]
        fn parsing_any_text_never_panics(text in "\\PC*") {
            let _ = text.parse::<Interval>();
        }
    }
}
