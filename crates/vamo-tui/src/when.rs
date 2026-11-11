//! O dia que quem usa digita pra agendar uma tarefa.

use jiff::civil::Date;

use crate::Voice;

/// Lê o dia digitado: `hoje`, `amanhã`, `dd/mm` (no ano de hoje),
/// `dd/mm/aaaa` ou `aaaa-mm-dd`.
///
/// # Errors
///
/// A mensagem pra mostrar no rodapé, com os formatos que valem.
pub(crate) fn parse_day(text: &str, today: Date, voice: Voice) -> Result<Date, String> {
    let text = text.trim().to_lowercase();
    let day = match text.as_str() {
        "hoje" => Some(today),
        "amanhã" | "amanha" => today.tomorrow().ok(),
        _ => match text.split('/').collect::<Vec<_>>().as_slice() {
            [day, month] => date(today.year(), month, day),
            [day, month, year] => year.parse().ok().and_then(|year| date(year, month, day)),
            _ => text.parse().ok(),
        },
    };
    day.ok_or_else(|| voice.bad_day(&text))
}

/// A data, se ela existir no calendário.
fn date(year: i16, month: &str, day: &str) -> Option<Date> {
    Date::new(year, month.parse().ok()?, day.parse().ok()?).ok()
}

#[cfg(test)]
mod tests {
    use jiff::civil::date as d;

    use super::*;

    const TODAY: Date = d(2026, 11, 11);

    #[test]
    fn the_formats_that_work() {
        for (text, expected) in [
            ("hoje", TODAY),
            ("Amanhã", d(2026, 11, 12)),
            ("amanha", d(2026, 11, 12)),
            ("25/12", d(2026, 12, 25)),
            (" 5/1/2027 ", d(2027, 1, 5)),
            ("2026-12-25", d(2026, 12, 25)),
        ] {
            assert_eq!(parse_day(text, TODAY, Voice::Game), Ok(expected), "{text}");
        }
    }

    #[test]
    fn days_that_do_not_exist_are_refused() {
        for text in ["", "ontem?", "30/02", "2026-13-01", "1/2/3/4"] {
            assert!(parse_day(text, TODAY, Voice::Game).is_err(), "{text}");
        }
    }
}
