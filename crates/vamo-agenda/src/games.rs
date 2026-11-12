//! O calendário de jogos lido de uma agenda ICS.
//!
//! As tabelas de jogos que se assinam como agenda põem o jogo no título do
//! evento, cada uma de um jeito: `Botafogo x Bangu`, `Bangu vs Botafogo`,
//! `⚽ Botafogo x Bangu - Carioca`, `Carioca: Bangu x Botafogo`,
//! `Botafogo x Bangu (Copa do Brasil)`. O [`IcsFixtures`] reconhece essas
//! formas, acha de que lado o time está e entrega ao core os [`Game`]s. O
//! resto da agenda (um evento que não é jogo do time) fica de fora.

use std::convert::Infallible;

use jiff::civil::Date;
use vamo_core::{Fixtures, Game, Side, When};

use crate::Calendar;

/// Os separadores entre os dois times, com espaço em volta pra não cortar
/// um nome no meio.
const VERSUS: [&str; 5] = [" x ", " X ", " × ", " vs. ", " vs "];

/// Os separadores entre o jogo e o campeonato.
const COMPETITION: [&str; 4] = [" - ", " – ", " — ", " | "];

/// Os jogos de um time, lidos de uma agenda ICS.
///
/// ```
/// use jiff::civil::date;
/// use jiff::tz::TimeZone;
/// use vamo_agenda::{Calendar, IcsFixtures};
/// use vamo_core::{Fixtures, Side};
///
/// let ics = "BEGIN:VCALENDAR\n\
///            BEGIN:VEVENT\n\
///            UID:jogo-1@exemplo\n\
///            DTSTART:20261115T190000Z\n\
///            DTEND:20261115T210000Z\n\
///            SUMMARY:Bangu x Botafogo - Amistoso\n\
///            END:VEVENT\n\
///            END:VCALENDAR\n";
/// let fixtures = IcsFixtures::new(Calendar::parse(ics, TimeZone::UTC)?, "Botafogo");
/// let games = fixtures.games(date(2026, 11, 15), date(2026, 11, 15)).unwrap();
/// assert_eq!(games[0].opponent, "Bangu");
/// assert_eq!(games[0].side, Side::Away);
/// assert_eq!(games[0].competition.as_deref(), Some("Amistoso"));
/// # Ok::<(), vamo_agenda::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct IcsFixtures {
    calendar: Calendar,
    team: String,
}

impl IcsFixtures {
    /// Os jogos de `team` (como `Botafogo`) no `calendar`.
    pub fn new(calendar: Calendar, team: &str) -> Self {
        Self {
            calendar,
            team: team.trim().to_owned(),
        }
    }

    /// Os jogos de `from` a `to` e os avisos da leitura da agenda.
    pub fn read(&self, from: Date, to: Date) -> (Vec<Game>, Vec<String>) {
        let (appointments, warnings) = self.calendar.read(from, to);
        let games = appointments
            .into_iter()
            .filter_map(|appointment| {
                // Jogo sem hora marcada ainda não tem quando a bola rola.
                let When::Timed { start, .. } = appointment.when else {
                    return None;
                };
                let title = parse_title(&appointment.title, &self.team)?;
                Some(Game {
                    uid: appointment.uid,
                    opponent: title.opponent,
                    kickoff: start,
                    competition: title.competition,
                    side: title.side,
                })
            })
            .collect();
        (games, warnings)
    }
}

impl Fixtures for IcsFixtures {
    // A agenda já foi lida no `Calendar::parse`; daqui pra frente não falha.
    type Error = Infallible;

    fn games(&self, from: Date, to: Date) -> Result<Vec<Game>, Self::Error> {
        Ok(self.read(from, to).0)
    }
}

/// O que o título de um evento diz do jogo.
#[derive(Debug, PartialEq, Eq)]
struct Title {
    opponent: String,
    competition: Option<String>,
    side: Side,
}

/// Lê o título de um evento como jogo de `team`. `None` quando não é um
/// jogo, ou não é do time.
fn parse_title(title: &str, team: &str) -> Option<Title> {
    // Emoji e símbolo na frente (`⚽ `, `🏆 `) não são do nome do time.
    let title = title
        .trim_start_matches(|c: char| !c.is_alphanumeric())
        .trim();
    let (matchup, competition) = split_competition(title);
    let (home, away) = VERSUS
        .iter()
        .find_map(|versus| matchup.split_once(versus))?;
    let (home, away) = (home.trim(), away.trim());
    let (opponent, side) = match (is_team(home, team), is_team(away, team)) {
        (true, false) => (away, Side::Home),
        (false, true) => (home, Side::Away),
        _ => return None,
    };
    if opponent.is_empty() {
        return None;
    }
    Some(Title {
        opponent: opponent.to_owned(),
        competition: competition.map(str::to_owned),
        side,
    })
}

/// Separa o jogo do campeonato, que pode vir depois de um traço ou de uma
/// barra, entre parênteses no fim ou antes de dois-pontos no começo.
fn split_competition(title: &str) -> (&str, Option<&str>) {
    let has_versus = |text: &str| VERSUS.iter().any(|versus| text.contains(versus));
    for separator in COMPETITION {
        if let Some((left, right)) = title.split_once(separator) {
            let (matchup, competition) = if has_versus(left) {
                (left, right)
            } else {
                (right, left)
            };
            return (matchup.trim(), non_empty(competition));
        }
    }
    if let Some(open) = title.rfind(" (")
        && let Some(inside) = title[open + 2..].strip_suffix(')')
    {
        return (title[..open].trim(), non_empty(inside));
    }
    if let Some((competition, matchup)) = title.split_once(": ")
        && has_versus(matchup)
    {
        return (matchup.trim(), non_empty(competition));
    }
    (title, None)
}

fn non_empty(text: &str) -> Option<&str> {
    let text = text.trim();
    (!text.is_empty()).then_some(text)
}

/// Se um lado do jogo é o time. O nome no calendário pode vir com sufixo
/// (`Botafogo-RJ`, `Botafogo FR`), então vale o começo, sem ligar pra
/// maiúscula.
fn is_team(side: &str, team: &str) -> bool {
    let side = side.to_lowercase();
    let team = team.to_lowercase();
    side == team
        || side
            .strip_prefix(&team)
            .is_some_and(|rest| rest.starts_with([' ', '-', '/']))
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use jiff::tz::TimeZone;
    use proptest::prelude::*;

    use super::*;

    fn title(text: &str) -> Option<Title> {
        parse_title(text, "Botafogo")
    }

    fn game(opponent: &str, competition: Option<&str>, side: Side) -> Title {
        Title {
            opponent: opponent.to_owned(),
            competition: competition.map(str::to_owned),
            side,
        }
    }

    #[test]
    fn the_common_titles_are_read() {
        let cases = [
            ("Botafogo x Bangu", game("Bangu", None, Side::Home)),
            ("Bangu vs Botafogo", game("Bangu", None, Side::Away)),
            ("Bangu vs. Botafogo", game("Bangu", None, Side::Away)),
            ("Botafogo × Bangu", game("Bangu", None, Side::Home)),
            (
                "⚽ Botafogo x Bangu - Carioca",
                game("Bangu", Some("Carioca"), Side::Home),
            ),
            (
                "Carioca | Bangu x Botafogo",
                game("Bangu", Some("Carioca"), Side::Away),
            ),
            (
                "Carioca: Bangu x Botafogo",
                game("Bangu", Some("Carioca"), Side::Away),
            ),
            (
                "Botafogo x Bangu (Copa do Brasil)",
                game("Bangu", Some("Copa do Brasil"), Side::Home),
            ),
            (
                "Botafogo-RJ x São Paulo",
                game("São Paulo", None, Side::Home),
            ),
            ("botafogo fr x Bangu", game("Bangu", None, Side::Home)),
        ];
        for (text, expected) in cases {
            assert_eq!(title(text), Some(expected), "{text}");
        }
    }

    #[test]
    fn what_is_not_a_game_of_the_team_is_left_out() {
        for text in [
            "Reunião da equipe",
            "Bangu x Madureira",
            "Botafogo x Botafogo",
            "Botafoguense x Bangu",
            "Botafogo x ",
            "Festa x",
        ] {
            assert_eq!(title(text), None, "{text}");
        }
    }

    #[test]
    fn the_example_calendar_has_the_games_of_the_team() {
        let text = std::fs::read_to_string(format!(
            "{}/tests/fixtures/jogos.ics",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let zone = TimeZone::get("America/Sao_Paulo").unwrap();
        let fixtures = IcsFixtures::new(Calendar::parse(&text, zone).unwrap(), "Botafogo");
        let games = fixtures
            .games(date(2026, 11, 1), date(2026, 11, 30))
            .unwrap();
        let summary: Vec<(String, String, Option<String>, Side)> = games
            .iter()
            .map(|game| {
                (
                    game.kickoff.to_string(),
                    game.opponent.clone(),
                    game.competition.clone(),
                    game.side,
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                (
                    "2026-11-15T16:00:00".to_owned(),
                    "Bangu".to_owned(),
                    Some("Amistoso".to_owned()),
                    Side::Home
                ),
                (
                    "2026-11-19T21:30:00".to_owned(),
                    "Madureira".to_owned(),
                    Some("Copa Exemplo".to_owned()),
                    Side::Away
                ),
                (
                    "2026-11-26T19:00:00".to_owned(),
                    "Olaria".to_owned(),
                    None,
                    Side::Home
                ),
            ]
        );
    }

    proptest! {
        /// Qualquer adversário que não tenha o separador no nome volta
        /// inteiro, dos dois lados do jogo.
        #[test]
        fn any_opponent_comes_back_whole(opponent in "[A-Za-zÀ-ÖØ-öø-ú][A-Za-zÀ-ÖØ-öø-ú ]{0,20}[A-Za-zÀ-ÖØ-öø-ú]") {
            prop_assume!(!opponent.to_lowercase().starts_with("botafogo"));
            prop_assume!(!VERSUS.iter().any(|versus| opponent.contains(versus.trim())));
            let home = title(&format!("Botafogo x {opponent}")).unwrap();
            prop_assert_eq!(&home.opponent, &opponent);
            prop_assert_eq!(home.side, Side::Home);
            let away = title(&format!("{opponent} x Botafogo")).unwrap();
            prop_assert_eq!(&away.opponent, &opponent);
            prop_assert_eq!(away.side, Side::Away);
        }
    }
}
