//! A data de um fato, com a precisão que as fontes dão.

use std::fmt;
use std::str::FromStr;

use crate::Error;

/// Quando o fato aconteceu, com a precisão que as fontes dão: só o ano
/// (`1910`), o ano e o mês (`1942-12`) ou o dia (`1909-05-30`).
///
/// A ordem é a do tempo, e no mesmo ano a data menos precisa vem antes:
/// `1909` < `1909-05` < `1909-05-30`.
///
/// ```
/// use vamo_acervo::FactDate;
///
/// let date: FactDate = "1909-05-30".parse()?;
/// assert_eq!(date.year(), 1909);
/// assert_eq!(date.month(), Some(5));
/// assert_eq!(date.day(), Some(30));
///
/// // Fevereiro de 1909 não teve dia 30.
/// assert!("1909-02-30".parse::<FactDate>().is_err());
/// # Ok::<(), vamo_acervo::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FactDate {
    // A ordem dos campos é a ordem da comparação: o `derive(Ord)` compara o
    // ano, depois o mês, depois o dia, e `None` vem antes de qualquer `Some`.
    year: u16,
    month: Option<u8>,
    // Só tem dia quando tem mês.
    day: Option<u8>,
}

impl FactDate {
    /// O ano.
    pub fn year(self) -> u16 {
        self.year
    }

    /// O mês (de 1 a 12), se as fontes dão.
    pub fn month(self) -> Option<u8> {
        self.month
    }

    /// O dia do mês, se as fontes dão.
    pub fn day(self) -> Option<u8> {
        self.day
    }
}

impl FromStr for FactDate {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Error> {
        parse(input).ok_or_else(|| Error::InvalidDate {
            input: input.to_owned(),
        })
    }
}

impl fmt::Display for FactDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}", self.year)?;
        if let Some(month) = self.month {
            write!(f, "-{month:02}")?;
        }
        if let Some(day) = self.day {
            write!(f, "-{day:02}")?;
        }
        Ok(())
    }
}

/// Lê `AAAA`, `AAAA-MM` ou `AAAA-MM-DD`, com os algarismos todos, e confere
/// se a data existe no calendário.
fn parse(input: &str) -> Option<FactDate> {
    let parts: Vec<&str> = input.split('-').collect();
    let date = match parts.as_slice() {
        [year] => FactDate {
            year: number(year, 4)?,
            month: None,
            day: None,
        },
        [year, month] => FactDate {
            year: number(year, 4)?,
            month: Some(two_digits(month)?),
            day: None,
        },
        [year, month, day] => FactDate {
            year: number(year, 4)?,
            month: Some(two_digits(month)?),
            day: Some(two_digits(day)?),
        },
        _ => return None,
    };

    let month_ok = date.month.is_none_or(|month| (1..=12).contains(&month));
    let day_ok = match (date.month, date.day) {
        (Some(month), Some(day)) => (1..=days_in_month(date.year, month)).contains(&day),
        _ => true,
    };
    (date.year >= 1000 && month_ok && day_ok).then_some(date)
}

/// Um número com exatamente `width` algarismos.
fn number(text: &str, width: usize) -> Option<u16> {
    let digits = text.len() == width && text.bytes().all(|byte| byte.is_ascii_digit());
    if digits { text.parse().ok() } else { None }
}

fn two_digits(text: &str) -> Option<u8> {
    u8::try_from(number(text, 2)?).ok()
}

pub(crate) fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Ano bissexto no calendário gregoriano: divisível por 4, menos os
/// centenários que não são divisíveis por 400 (1900 não foi, 2000 foi).
fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn date(input: &str) -> FactDate {
        input.parse().unwrap()
    }

    #[test]
    fn reads_the_three_precisions() {
        assert_eq!(date("1910").to_string(), "1910");
        assert_eq!(date("1942-12").to_string(), "1942-12");
        assert_eq!(date("1909-05-30").to_string(), "1909-05-30");
    }

    #[test]
    fn refuses_what_is_not_a_date() {
        for input in [
            "",
            "89",
            "1909-5-30",
            "1909-05-30-1",
            "1909/05/30",
            "1909-13",
            "1909-00",
            "1909-04-31",
            "1909-05-00",
            "0999",
            "1909-",
            "-1909",
            "abcd",
        ] {
            assert_eq!(
                input.parse::<FactDate>(),
                Err(Error::InvalidDate {
                    input: input.to_owned()
                }),
                "{input:?}"
            );
        }
    }

    #[test]
    fn knows_the_leap_years() {
        assert!("2024-02-29".parse::<FactDate>().is_ok());
        assert!("2000-02-29".parse::<FactDate>().is_ok());
        assert!("1900-02-29".parse::<FactDate>().is_err());
        assert!("2023-02-29".parse::<FactDate>().is_err());
    }

    #[test]
    fn the_less_precise_date_comes_first_in_the_same_year() {
        assert!(date("1909") < date("1909-05"));
        assert!(date("1909-05") < date("1909-05-30"));
        assert!(date("1909-05-30") < date("1910"));
    }

    proptest! {
        #[test]
        fn every_valid_date_comes_back_the_same(
            year in 1000_u16..=9999,
            month in 1_u8..=12,
            day in 1_u8..=31,
        ) {
            prop_assume!(day <= days_in_month(year, month));
            for text in [
                format!("{year}"),
                format!("{year}-{month:02}"),
                format!("{year}-{month:02}-{day:02}"),
            ] {
                prop_assert_eq!(date(&text).to_string(), text);
            }
        }

        #[test]
        fn any_text_either_reads_or_fails_without_panicking(input in ".{0,12}") {
            let _ = input.parse::<FactDate>();
        }
    }
}
