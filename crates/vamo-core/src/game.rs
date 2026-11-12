//! Os jogos: quando o time joga, contra quem e valendo o quê.
//!
//! O calendário de jogos vem de fora, por uma porta ([`Fixtures`]), pra não
//! depender da agenda pessoal: quem usa assina a tabela do time e o `vamo`
//! fica sabendo do dia de jogo sozinho.

use jiff::SignedDuration;
use jiff::civil::{Date, DateTime};

use crate::{Area, Fit, Projection, Status, Task, Violation};

/// Quanto antes da bola rolar começa o "perto do jogo".
pub const BEFORE_KICKOFF: SignedDuration = SignedDuration::from_hours(2);

/// Quanto o jogo dura, com intervalo e acréscimos, pra regra do dia de jogo.
pub const MATCH_LENGTH: SignedDuration = SignedDuration::from_hours(2);

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

/// O dia de jogo: o jogo e a área das tarefas dele (como ver, com quem, o
/// que preparar).
///
/// A regra do dia de jogo mora aqui: perto do horário, só tarefa leve. De
/// [`BEFORE_KICKOFF`] antes da bola rolar até o fim do jogo
/// ([`MATCH_LENGTH`] depois), tarefa que pede foco ou outra pessoa fere a
/// regra. Brecha pode, e as tarefas do próprio jogo também.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameDay {
    /// O jogo.
    pub game: Game,
    /// A área das tarefas do jogo.
    pub area: Area,
}

impl GameDay {
    /// O trecho perto do jogo, onde só cabe tarefa leve: o começo e o fim.
    ///
    /// ```
    /// use jiff::civil::date;
    /// use vamo_core::{Game, GameDay, Side};
    ///
    /// let day = GameDay {
    ///     game: Game {
    ///         uid: "jogo-1@exemplo".to_owned(),
    ///         opponent: "Bangu".to_owned(),
    ///         kickoff: date(2026, 11, 15).at(21, 30, 0, 0),
    ///         competition: None,
    ///         side: Side::Home,
    ///     },
    ///     area: "jogo".parse()?,
    /// };
    /// assert_eq!(
    ///     day.light_only(),
    ///     (date(2026, 11, 15).at(19, 30, 0, 0), date(2026, 11, 15).at(23, 30, 0, 0))
    /// );
    /// # Ok::<(), vamo_core::Error>(())
    /// ```
    pub fn light_only(&self) -> (DateTime, DateTime) {
        (
            self.game.kickoff.saturating_sub(BEFORE_KICKOFF),
            self.game.kickoff.saturating_add(MATCH_LENGTH),
        )
    }

    /// Se a tarefa é leve o bastante pra perto do jogo: brecha, ou do
    /// próprio jogo.
    pub fn allows(&self, task: &Task) -> bool {
        task.fit() == Fit::Gap || task.area() == &self.area
    }

    /// A regra do dia de jogo que a tarefa fere, se for feita de `start` a
    /// `end`.
    pub fn check(&self, task: &Task, start: DateTime, end: DateTime) -> Option<Violation> {
        let (from, to) = self.light_only();
        (start < to && from < end && !self.allows(task)).then(|| Violation::NearGame {
            from: from.time(),
            to: to.time(),
        })
    }

    /// As tarefas do jogo em `day`: as da área do jogo que ainda estão em
    /// campo, escaladas pro dia ou sem dia marcado, e as que já saíram no
    /// dia.
    pub fn tasks(&self, projection: &Projection, day: Date) -> Vec<Task> {
        projection
            .tasks()
            .filter(|task| task.area() == &self.area)
            .filter(|task| match (task.status(), task.day()) {
                (Status::Dropped, _) => false,
                (Status::Done, task_day) => task_day == Some(day),
                (_, task_day) => task_day.is_none_or(|task_day| task_day == day),
            })
            .collect()
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

    fn game_day() -> GameDay {
        GameDay {
            game: game(15, 21),
            area: "jogo".parse().unwrap(),
        }
    }

    fn task(fit: Fit, area: &str) -> Task {
        Task::new(
            "T-041".parse().unwrap(),
            "Escrever o relatório".parse().unwrap(),
            area.parse().unwrap(),
            fit,
            Status::Scheduled,
        )
    }

    #[test]
    fn near_the_game_only_light_tasks() {
        let day = game_day();
        let at = |hour| date(2026, 11, 15).at(hour, 0, 0, 0);
        let violation = Some(Violation::NearGame {
            from: jiff::civil::time(19, 0, 0, 0),
            to: jiff::civil::time(23, 0, 0, 0),
        });
        // Foco que entra no trecho fere; o que acaba antes, não.
        assert_eq!(
            day.check(&task(Fit::Focus, "trampo"), at(18), at(20)),
            violation
        );
        assert_eq!(
            day.check(&task(Fit::WithSomeone, "casa"), at(22), at(23)),
            violation
        );
        assert_eq!(day.check(&task(Fit::Focus, "trampo"), at(17), at(19)), None);
        // Brecha e tarefa do próprio jogo podem.
        assert_eq!(day.check(&task(Fit::Gap, "casa"), at(20), at(21)), None);
        assert_eq!(
            day.check(&task(Fit::WithSomeone, "jogo"), at(20), at(23)),
            None
        );
    }

    #[test]
    fn the_game_tasks_are_the_ones_of_the_day_or_without_a_day() {
        use crate::{Change, Event, EventId, Machine, TaskId};
        use uuid::Builder;

        let mut projection = Projection::new();
        let mut n = 0;
        let mut record = |id: u32, change: Change| {
            n += 1;
            projection.apply(&Event {
                id: EventId::new(Builder::from_unix_timestamp_millis(n, &[0; 10]).into_uuid())
                    .unwrap(),
                at: jiff::Timestamp::from_second(i64::try_from(n).unwrap()).unwrap(),
                machine: Machine::new("notebook").unwrap(),
                task: TaskId::new(id).unwrap(),
                change,
            });
        };
        let created = |title: &str, area: &str| Change::Created {
            title: title.parse().unwrap(),
            area: area.parse().unwrap(),
            fit: Fit::WithSomeone,
            status: Status::Ready,
        };
        record(1, created("Combinar onde ver", "jogo"));
        record(2, created("Comprar o carvão", "jogo"));
        record(2, Change::Day(Some(date(2026, 11, 15))));
        record(3, created("Lavar a camisa", "jogo"));
        record(3, Change::Day(Some(date(2026, 11, 20))));
        record(4, created("Ligar pro encanador", "casa"));
        record(5, created("Separar a bandeira", "jogo"));
        record(5, Change::Day(Some(date(2026, 11, 15))));
        record(5, Change::Status(Status::Done));
        record(6, created("Chamar a turma", "jogo"));
        record(6, Change::Status(Status::Dropped));

        let ids: Vec<String> = game_day()
            .tasks(&projection, date(2026, 11, 15))
            .iter()
            .map(|task| task.id().to_string())
            .collect();
        assert_eq!(ids, ["T-001", "T-002", "T-005"]);
    }

    #[test]
    fn the_game_of_the_day_is_the_first_of_the_day() {
        let games = [game(15, 21), game(15, 11), game(14, 9)];
        assert_eq!(game_on(&games, date(2026, 11, 15)), Some(&games[1]));
    }
}
