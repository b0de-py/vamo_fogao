//! Os jogos: quando o time joga, contra quem e valendo o quê.
//!
//! O calendário de jogos vem de fora, por uma porta ([`Fixtures`]), pra não
//! depender da agenda pessoal: quem usa assina a tabela do time e o `vamo`
//! fica sabendo do dia de jogo sozinho.

use jiff::civil::{Date, DateTime};

/// Um jogo do time, já no horário local de quem usa.
///
/// ```
/// use jiff::civil::date;
/// use vamo_core::{Game, Side};
///
/// let game = Game {
///     uid: "jogo-1@exemplo".to_owned(),
///     opponent: "Bangu".to_owned(),
///     kickoff: date(2026, 11, 15).at(16, 0, 0, 0),
///     competition: Some("Amistoso".to_owned()),
///     side: Side::Home,
/// };
/// assert_eq!(game.day(), date(2026, 11, 15));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    /// O identificador do jogo no calendário.
    pub uid: String,
    /// O adversário, como está no calendário.
    pub opponent: String,
    /// A hora em que a bola rola.
    pub kickoff: DateTime,
    /// O campeonato, quando o calendário diz.
    pub competition: Option<String>,
    /// Se o time é o mandante ou o visitante.
    pub side: Side,
}

/// De que lado do jogo o time está.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// O mandante, o primeiro nome do "A x B".
    Home,
    /// O visitante.
    Away,
}

impl Game {
    /// O dia do jogo.
    pub fn day(&self) -> Date {
        self.kickoff.date()
    }
}

/// O jogo do dia: o primeiro que começa em `day`, se tiver.
///
/// ```
/// use jiff::civil::date;
/// use vamo_core::{Game, Side, game_on};
///
/// let game = Game {
///     uid: "jogo-1@exemplo".to_owned(),
///     opponent: "Bangu".to_owned(),
///     kickoff: date(2026, 11, 15).at(16, 0, 0, 0),
///     competition: None,
///     side: Side::Away,
/// };
/// assert_eq!(game_on(&[game.clone()], date(2026, 11, 15)), Some(&game));
/// assert_eq!(game_on(&[game], date(2026, 11, 16)), None);
/// ```
pub fn game_on(games: &[Game], day: Date) -> Option<&Game> {
    games
        .iter()
        .filter(|game| game.day() == day)
        .min_by_key(|game| game.kickoff)
}

/// A porta do calendário de jogos.
///
/// O `vamo-agenda` implementa lendo uma agenda ICS de jogos. Outra fonte
/// (uma API de futebol, por exemplo) só precisa devolver os mesmos
/// [`Game`]s.
pub trait Fixtures {
    /// O erro da leitura.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Os jogos que começam entre `from` e `to`, os dois dias incluídos, em
    /// ordem de começo.
    ///
    /// # Errors
    ///
    /// Quando o calendário não pode ser lido.
    fn games(&self, from: Date, to: Date) -> Result<Vec<Game>, Self::Error>;
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    fn game(day: i8, hour: i8) -> Game {
        Game {
            uid: format!("jogo-{day}-{hour}@exemplo"),
            opponent: "Bangu".to_owned(),
            kickoff: date(2026, 11, day).at(hour, 0, 0, 0),
            competition: None,
            side: Side::Home,
        }
    }

    #[test]
    fn the_game_of_the_day_is_the_first_of_the_day() {
        let games = [game(15, 21), game(15, 11), game(14, 9)];
        assert_eq!(game_on(&games, date(2026, 11, 15)), Some(&games[1]));
    }
}
