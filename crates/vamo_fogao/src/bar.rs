//! A barra: o `vamo barra`, o placar do dia numa linha, pro módulo QML do
//! Omarchy (`extras/omarchy/vamo_fogao.qml`).
//!
//! O `--json` é um contrato com o módulo, e o módulo instalado pode ser
//! mais velho ou mais novo que o binário. Por isso a saída tem número de
//! versão ([`FORMAT_VERSION`]) e um teste de snapshot que quebra se o
//! formato mudar sem querer. Os campos `text`, `tooltip` e `class` seguem o
//! padrão dos módulos de comando da barra (o mesmo do Waybar), então até um
//! módulo genérico mostra o placar.

use std::collections::BTreeSet;

use jiff::civil::{Date, DateTime};
use serde_json::{Value, json};
use vamo_core::{Appointment, FixedActivity, GameDay, Lineup, Projection, Side, Status};
use vamo_tui::Voice;

/// A versão do formato do `vamo barra --json`.
///
/// Campo novo não muda a versão: o módulo ignora o que não conhece. Muda
/// quando um campo some, troca de tipo ou de sentido.
pub(crate) const FORMAT_VERSION: u32 = 1;

/// Quantas tarefas em aberto a dica lista antes do "e mais".
const OPEN_IN_TOOLTIP: usize = 5;

/// O que a barra precisa saber do dia. Quem monta é o comando, com o banco
/// e a cópia local da agenda.
pub(crate) struct Day<'a> {
    /// As tarefas.
    pub(crate) projection: &'a Projection,
    /// Os compromissos de hoje.
    pub(crate) appointments: &'a [Appointment],
    /// As atividades fixas da agenda de hoje.
    pub(crate) fixed: &'a [FixedActivity],
    /// As fixas da agenda marcadas.
    pub(crate) checks: &'a BTreeSet<(Date, String)>,
    /// O dia de jogo, se hoje tem jogo (e a linguagem é a de jogo).
    pub(crate) game: Option<&'a GameDay>,
    /// O nome do time, pro "Botafogo x Bangu".
    pub(crate) team: &'a str,
    /// Agora, no horário local.
    pub(crate) now: DateTime,
}

/// O resumo do dia que a barra mostra.
#[derive(Debug)]
pub(crate) struct Bar {
    day: Date,
    done: usize,
    planned: usize,
    late: usize,
    /// O próximo compromisso com horário: quando começa e o título.
    next: Option<(DateTime, String)>,
    /// As tarefas escaladas pro dia que ainda não saíram, com o ID na frente.
    open: Vec<String>,
    /// O jogo do dia, se tiver.
    game: Option<Matchday>,
}

/// O jogo do dia, como a barra mostra.
#[derive(Debug)]
struct Matchday {
    opponent: String,
    matchup: String,
    kickoff: DateTime,
    competition: Option<String>,
    side: Side,
    /// O trecho perto do jogo, onde só cabe tarefa leve.
    light_only: (DateTime, DateTime),
    /// As tarefas do jogo que ainda não saíram.
    open_tasks: usize,
}

/// O estado do placar, que vira a classe do módulo (e a cor na barra).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Nada previsto pro dia.
    Empty,
    /// Tem coisa em aberto.
    Playing,
    /// Tudo previsto foi feito.
    Closed,
}

impl Bar {
    /// Resume o dia.
    pub(crate) fn new(day: &Day<'_>) -> Self {
        let today = day.now.date();
        let lineup = Lineup::new(day.projection, today);
        let score = lineup.score_with(day.fixed, day.checks);
        let next = day
            .appointments
            .iter()
            .filter_map(|appointment| {
                let (start, _) = appointment.span()?;
                (start > day.now).then(|| (start, appointment.title.clone()))
            })
            .min_by_key(|(start, _)| *start);
        let open = lineup
            .planned
            .iter()
            .filter(|task| task.status() != Status::Done)
            .map(|task| format!("{} {}", task.id(), task.title()))
            .collect();
        let game = day.game.map(|game_day| {
            let game = &game_day.game;
            Matchday {
                opponent: game.opponent.clone(),
                matchup: match game.side {
                    Side::Home => format!("{} x {}", day.team, game.opponent),
                    Side::Away => format!("{} x {}", game.opponent, day.team),
                },
                kickoff: game.kickoff,
                competition: game.competition.clone(),
                side: game.side,
                light_only: game_day.light_only(),
                open_tasks: game_day
                    .tasks(day.projection, today)
                    .iter()
                    .filter(|task| task.status() != Status::Done)
                    .count(),
            }
        });
        Self {
            day: today,
            done: score.done,
            planned: score.planned,
            late: lineup.late.len(),
            next,
            open,
            game,
        }
    }

    fn state(&self) -> State {
        if self.planned == 0 {
            State::Empty
        } else if self.done >= self.planned {
            State::Closed
        } else {
            State::Playing
        }
    }

    /// O texto curto, o que aparece na barra.
    pub(crate) fn text(&self, voice: Voice) -> String {
        let score = voice.bar_score(self.done, self.planned);
        match &self.game {
            Some(game) => format!("{score} · {}", voice.bar_game(&hm(game.kickoff))),
            None => score,
        }
    }

    /// A dica que aparece com o mouse em cima: o dia, o placar, o atraso, o
    /// próximo compromisso e o que falta.
    pub(crate) fn tooltip(&self, voice: Voice) -> String {
        let mut lines = vec![
            format!(
                "Hoje · {}, {}",
                voice.weekday(self.day.weekday()),
                self.day.strftime("%d/%m")
            ),
            voice.score(self.done, self.planned),
        ];
        if let Some(game) = &self.game {
            let competition = game
                .competition
                .as_ref()
                .map(|competition| format!(" · {competition}"))
                .unwrap_or_default();
            lines.push(format!(
                "{}: {}, {}{competition}",
                voice.game_day(),
                game.matchup,
                hm(game.kickoff)
            ));
            lines.push(voice.light_only(&hm(game.light_only.0), &hm(game.light_only.1)));
        }
        if self.late > 0 {
            lines.push(voice.late_count(self.late));
        }
        if let Some((start, title)) = &self.next {
            lines.push(voice.next_appointment(&hm(*start), title));
        }
        if !self.open.is_empty() {
            lines.push(String::new());
            lines.push(voice.still_open().to_owned());
            lines.extend(self.open.iter().take(OPEN_IN_TOOLTIP).cloned());
            let more = self.open.len().saturating_sub(OPEN_IN_TOOLTIP);
            if more > 0 {
                lines.push(format!("e mais {more}"));
            }
        }
        lines.join("\n")
    }

    /// As classes do módulo: uma do estado do placar (`vazio`, `andando` ou
    /// `fechado`) e `atraso` quando tem tarefa atrasada.
    fn classes(&self) -> Vec<&'static str> {
        let mut classes = vec![match self.state() {
            State::Empty => "vazio",
            State::Playing => "andando",
            State::Closed => "fechado",
        }];
        if self.late > 0 {
            classes.push("atraso");
        }
        if self.game.is_some() {
            classes.push("jogo");
        }
        classes
    }

    /// A saída do `vamo barra --json`.
    pub(crate) fn json(&self, voice: Voice) -> Value {
        json!({
            "versao": FORMAT_VERSION,
            "dia": self.day.to_string(),
            "text": self.text(voice),
            "tooltip": self.tooltip(voice),
            "class": self.classes(),
            "placar": {
                "feitas": self.done,
                "previstas": self.planned,
            },
            "atrasadas": self.late,
            "proximo": self.next.as_ref().map(|(start, title)| json!({
                "inicio": hm(*start),
                "titulo": title,
            })),
            "jogo": self.game.as_ref().map(|game| json!({
                "adversario": game.opponent,
                "inicio": hm(game.kickoff),
                "campeonato": game.competition,
                "mando": match game.side {
                    Side::Home => "casa",
                    Side::Away => "fora",
                },
                "so_tarefa_leve": {
                    "de": hm(game.light_only.0),
                    "ate": hm(game.light_only.1),
                },
                "tarefas_em_aberto": game.open_tasks,
            })),
        })
    }
}

/// Uma hora como se escreve: `21:30`.
fn hm(at: DateTime) -> String {
    at.strftime("%H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use uuid::Uuid;
    use vamo_core::{Change, Event, EventId, Fit, Machine, TaskId, When};

    use super::*;

    fn today() -> Date {
        date(2026, 11, 12)
    }

    /// Um log de exemplo: duas tarefas escaladas pra hoje (uma feita), uma
    /// atrasada e uma fixa que ninguém marcou.
    fn projection() -> Projection {
        let mut projection = Projection::new();
        let mut n = 0;
        let mut record = |id: u32, change: Change| {
            n += 1;
            projection.apply(&Event {
                id: EventId::new(Uuid::from_u128(
                    0x0192_f0c4_8a00_7000_8000_0000_0000_0000 + n,
                ))
                .unwrap(),
                at: jiff::Timestamp::from_second(i64::try_from(n).unwrap()).unwrap(),
                machine: Machine::new("notebook").unwrap(),
                task: TaskId::new(id).unwrap(),
                change,
            });
        };
        let tasks = [
            (1, "Revisar o orçamento", Status::Ready),
            (2, "Ligar pro encanador", Status::Ready),
            (3, "Separar roupa pra doação", Status::Ready),
            (4, "Regar as plantas", Status::Recurring),
        ];
        for (id, title, status) in tasks {
            record(
                id,
                Change::Created {
                    title: title.parse().unwrap(),
                    area: "casa".parse().unwrap(),
                    fit: Fit::Gap,
                    status,
                },
            );
        }
        record(1, Change::Day(Some(today())));
        record(2, Change::Day(Some(today())));
        record(2, Change::Status(Status::Done));
        record(3, Change::Day(Some(date(2026, 11, 10))));
        projection
    }

    fn appointments() -> Vec<Appointment> {
        let timed = |uid: &str, title: &str, hour| Appointment {
            uid: uid.to_owned(),
            title: title.to_owned(),
            when: When::Timed {
                start: today().at(hour, 0, 0, 0),
                end: today().at(hour + 1, 0, 0, 0),
            },
            recurring: false,
            task: None,
        };
        vec![
            timed("cedo", "Café com a vizinha", 8),
            timed("tarde", "Reunião de condomínio", 17),
            timed("almoco", "Almoço de família", 12),
        ]
    }

    fn bar(projection: &Projection, appointments: &[Appointment]) -> Bar {
        Bar::new(&Day {
            projection,
            appointments,
            fixed: &[],
            checks: &BTreeSet::new(),
            game: None,
            team: "Botafogo",
            now: today().at(10, 30, 0, 0),
        })
    }

    fn game_day() -> GameDay {
        GameDay {
            game: vamo_core::Game {
                uid: "jogo-1@exemplo".to_owned(),
                opponent: "Bangu".to_owned(),
                kickoff: today().at(21, 30, 0, 0),
                competition: Some("Amistoso".to_owned()),
                side: Side::Away,
            },
            area: "jogo".parse().unwrap(),
        }
    }

    #[test]
    fn the_json_format_is_stable() {
        let bar = bar(&projection(), &appointments());
        let json = serde_json::to_string_pretty(&bar.json(Voice::Game)).unwrap();
        insta::assert_snapshot!(json);
    }

    #[test]
    fn the_game_day_goes_in_the_json() {
        let projection = projection();
        let game_day = game_day();
        let bar = Bar::new(&Day {
            projection: &projection,
            appointments: &[],
            fixed: &[],
            checks: &BTreeSet::new(),
            game: Some(&game_day),
            team: "Botafogo",
            now: today().at(10, 30, 0, 0),
        });
        assert_eq!(bar.text(Voice::Game), "⚽ 1 x 2 · jogo às 21:30");
        assert_eq!(bar.classes(), ["andando", "atraso", "jogo"]);
        let json = serde_json::to_string_pretty(&bar.json(Voice::Game)).unwrap();
        insta::assert_snapshot!(json);
    }

    #[test]
    fn the_neutral_voice_has_no_ball() {
        let bar = bar(&projection(), &appointments());
        assert_eq!(bar.text(Voice::Game), "⚽ 1 x 2");
        assert_eq!(bar.text(Voice::Neutral), "1/3");
    }

    #[test]
    fn the_next_appointment_is_the_first_that_did_not_start() {
        let bar = bar(&projection(), &appointments());
        assert_eq!(bar.next.unwrap().1, "Almoço de família");
    }

    #[test]
    fn an_empty_day_is_empty() {
        let bar = bar(&Projection::new(), &[]);
        assert_eq!(bar.classes(), ["vazio"]);
        assert_eq!(bar.json(Voice::Game)["proximo"], Value::Null);
    }

    #[test]
    fn a_closed_score_is_closed() {
        let bar = Bar {
            day: today(),
            done: 3,
            planned: 3,
            late: 0,
            next: None,
            open: Vec::new(),
            game: None,
        };
        assert_eq!(bar.classes(), ["fechado"]);
    }

    #[test]
    fn the_tooltip_stops_listing_after_a_few() {
        let bar = Bar {
            day: today(),
            done: 0,
            planned: 7,
            late: 0,
            next: None,
            game: None,
            open: (1..=7)
                .map(|n| format!("T-00{n} Fazer a coisa {n}"))
                .collect(),
        };
        let tooltip = bar.tooltip(Voice::Game);
        assert!(tooltip.contains("T-005"));
        assert!(!tooltip.contains("T-006"));
        assert!(tooltip.ends_with("e mais 2"));
    }
}
