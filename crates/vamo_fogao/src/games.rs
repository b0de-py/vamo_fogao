//! Os jogos no binário: liga a seção `[jogos]` da config, o link e a cópia
//! local ao leitor de jogos do `vamo-agenda`.
//!
//! Como na agenda, quem baixa é o `vamo sync` (e o `vamo jogos`); a TUI, a
//! barra e os lembretes leem só a cópia local.

use color_eyre::eyre::{Result, WrapErr, eyre};
use jiff::tz::TimeZone;
use vamo_agenda::{Calendar, Freshness, IcsFixtures, IcsLink};
use vamo_core::{Game, Side};

use crate::config::GamesConfig;
use crate::paths;

/// O calendário de jogos lido, e de onde ele veio.
pub(crate) struct Loaded {
    pub(crate) fixtures: IcsFixtures,
    pub(crate) freshness: Freshness,
}

fn link(url: &str) -> Result<IcsLink> {
    Ok(IcsLink::new(url, &paths::games_cache()?))
}

fn parse(text: &str, config: &GamesConfig, zone: TimeZone) -> Result<IcsFixtures> {
    let calendar = Calendar::parse(text, zone)
        .wrap_err("o calendário de jogos não é um ICS que o vamo leia")?;
    Ok(IcsFixtures::new(calendar, config.team()))
}

/// Baixa o calendário de jogos (ou usa a cópia local, sem internet).
/// `Ok(None)` quando a config não tem a seção `[jogos]`.
pub(crate) fn load(config: &GamesConfig, zone: TimeZone) -> Result<Option<Loaded>> {
    let Some(url) = &config.ics else {
        return Ok(None);
    };
    let (text, freshness) = link(url)?.fetch()?;
    Ok(Some(Loaded {
        fixtures: parse(&text, config, zone)?,
        freshness,
    }))
}

/// O jogo como se escreve, com o mandante na frente: `Botafogo x Bangu`.
pub(crate) fn matchup(game: &Game, team: &str) -> String {
    match game.side {
        Side::Home => format!("{team} x {}", game.opponent),
        Side::Away => format!("{} x {team}", game.opponent),
    }
}

/// Uma linha do `vamo jogos`: `15/11 16:00  Botafogo x Bangu · Amistoso`.
pub(crate) fn line(game: &Game, team: &str) -> String {
    let competition = game
        .competition
        .as_ref()
        .map(|competition| format!(" · {competition}"))
        .unwrap_or_default();
    format!(
        "{}  {}{competition}",
        game.kickoff.strftime("%d/%m %H:%M"),
        matchup(game, team)
    )
}

/// O erro de quando a config não tem o calendário de jogos.
pub(crate) fn not_configured() -> color_eyre::Report {
    eyre!(
        "o calendário de jogos não está configurado: ponha o link ICS da tabela do time na \
         config, numa seção [jogos] com ics = \"https://...\""
    )
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    #[test]
    fn the_home_team_comes_first() {
        let mut game = Game {
            uid: "jogo@exemplo".to_owned(),
            opponent: "Bangu".to_owned(),
            kickoff: date(2026, 11, 15).at(16, 0, 0, 0),
            competition: Some("Amistoso".to_owned()),
            side: Side::Home,
        };
        assert_eq!(
            line(&game, "Botafogo"),
            "15/11 16:00  Botafogo x Bangu · Amistoso"
        );
        game.side = Side::Away;
        game.competition = None;
        assert_eq!(line(&game, "Botafogo"), "15/11 16:00  Bangu x Botafogo");
    }
}
