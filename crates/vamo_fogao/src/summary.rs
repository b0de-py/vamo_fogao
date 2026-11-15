//! A súmula do dia no terminal e no JSON, e os hábitos do habit-tracker.
//!
//! A regra (o que é gol, o que voltou pro banco) é do core, no
//! [`DaySummary`]. Aqui fica o que depende do mundo de fora: o fuso, que
//! diz onde o dia começa e acaba, a config do habit-tracker e o texto.

use color_eyre::eyre::{Result, WrapErr, eyre};
use jiff::Zoned;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use serde_json::Value;
use vamo_core::{CLUB_RECORD, DaySummary, DayWindow, Event, HabitCount, Unbeaten, WeekSummary};
use vamo_store::{Habit, HabitTracker};
use vamo_tui::Voice;

use crate::config::HabitsConfig;

/// A hora do apito final: a súmula de um dia fecha às 4h do dia seguinte.
const FINAL_WHISTLE_HOUR: i8 = 4;

/// O último dia que já fechou em `now`: até as 4h, a noite de ontem ainda
/// está em jogo, e o último dia fechado é anteontem.
pub(crate) fn last_closed_day(now: &Zoned) -> Result<Date> {
    let shifted = now.datetime() - jiff::SignedDuration::from_hours(FINAL_WHISTLE_HOUR.into());
    Ok(shifted.date().yesterday()?)
}

/// O trecho do relógio que conta como `day` no fuso dado: da meia-noite às
/// 4h do dia seguinte.
pub(crate) fn window(day: Date, zone: &TimeZone) -> Result<DayWindow> {
    let start = day.to_zoned(zone.clone())?.timestamp();
    let end = day
        .tomorrow()?
        .at(FINAL_WHISTLE_HOUR, 0, 0, 0)
        .to_zoned(zone.clone())?
        .timestamp();
    Ok(DayWindow { start, end })
}

/// Os dias de `first` a `last`, os dois inclusos, cada um com o trecho do
/// relógio dele no fuso.
pub(crate) fn days(first: Date, last: Date, zone: &TimeZone) -> Result<Vec<(Date, DayWindow)>> {
    first
        .series(jiff::ToSpan::days(1))
        .take_while(|day| *day <= last)
        .map(|day| Ok((day, window(day, zone)?)))
        .collect()
}

/// A invencibilidade no fim de `last`, que precisa ser um dia fechado:
/// conta do dia do primeiro evento do log até ele.
pub(crate) fn unbeaten(events: &[Event], zone: &TimeZone, last: Date) -> Result<Unbeaten> {
    let Some(first) = events.iter().map(|event| event.at).min() else {
        return Ok(Unbeaten::default());
    };
    let first = first.to_zoned(zone.clone()).date();
    Ok(Unbeaten::count(events, days(first, last, zone)?))
}

/// O habit-tracker da config, se a seção `[habitos]` existir.
///
/// # Errors
///
/// Quando a config do habit-tracker não abre ou não está no formato dele.
pub(crate) fn habit_tracker(config: &HabitsConfig) -> Result<Option<HabitTracker>> {
    let (Some(database), Some(settings)) = (&config.database, &config.config) else {
        return Ok(None);
    };
    let database = crate::paths::expand_home(database)?;
    let settings = crate::paths::expand_home(settings)?;
    let text = std::fs::read_to_string(&settings).wrap_err_with(|| {
        format!(
            "não deu pra ler a config do habit-tracker em {}",
            settings.display()
        )
    })?;
    let (habits, turn) = parse_habits(&text).wrap_err_with(|| {
        format!(
            "a config do habit-tracker em {} não está no formato dele",
            settings.display()
        )
    })?;
    Ok(Some(HabitTracker::new(&database, habits, turn)))
}

/// Lê a config do habit-tracker: os hábitos em `habitos`, cada um com
/// `nome` e `unidade`, e a hora da virada do dia em `hora_virada_dia`.
fn parse_habits(text: &str) -> Result<(Vec<Habit>, u8)> {
    let config: Value = serde_json::from_str(text)?;
    let turn = match config.get("hora_virada_dia") {
        None => 0,
        Some(hour) => hour
            .as_u64()
            .and_then(|hour| u8::try_from(hour).ok())
            .filter(|hour| *hour < 24)
            .ok_or_else(|| eyre!("`hora_virada_dia` é uma hora de 0 a 23"))?,
    };
    let list = config
        .get("habitos")
        .and_then(Value::as_object)
        .ok_or_else(|| eyre!("falta o `habitos`, com um campo por hábito"))?;
    let habits = list
        .iter()
        .map(|(key, habit)| {
            let field = |name: &str| {
                habit
                    .get(name)
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .ok_or_else(|| eyre!("o hábito `{key}` está sem o `{name}`"))
            };
            Ok(Habit {
                key: key.clone(),
                name: field("nome")?,
                unit: field("unidade")?,
            })
        })
        .collect::<Result<_>>()?;
    Ok((habits, turn))
}

/// A súmula em texto, pro terminal.
pub(crate) fn text(summary: &DaySummary, voice: Voice) -> String {
    let mut lines = vec![voice.summary_title(
        summary.day.weekday(),
        &summary.day.strftime("%d/%m").to_string(),
    )];
    let [goals, benched, stoppage, _] = voice.summary_parts();
    if summary.is_empty() {
        lines.push(String::new());
        lines.push(voice.summary_empty().to_owned());
    }
    for (title, tasks) in [
        (goals, &summary.goals),
        (benched, &summary.benched),
        (stoppage, &summary.stoppage),
    ] {
        if tasks.is_empty() {
            continue;
        }
        lines.push(String::new());
        lines.push(format!("{title} ({})", tasks.len()));
        lines.extend(
            tasks
                .iter()
                .map(|task| format!("  {}  {}", task.id(), task.title())),
        );
    }
    if !summary.habits.is_empty() {
        lines.push(String::new());
        lines.extend(habit_lines(&summary.habits, voice));
    }
    lines.join("\n")
}

/// A rodada em texto, pro terminal: uma linha por dia, os impedimentos, os
/// hábitos da semana e a invencibilidade.
pub(crate) fn week_text(week: &WeekSummary, voice: Voice) -> String {
    let (Some(first), Some(last)) = (week.days.first(), week.days.last()) else {
        return String::new();
    };
    let short = |day: Date| day.strftime("%d/%m").to_string();
    let mut lines = vec![
        voice.week_title(&short(first.day), &short(last.day)),
        String::new(),
        voice.summary_counts(week.goals(), week.benched(), week.stoppage()),
        String::new(),
    ];
    for day in &week.days {
        lines.push(format!(
            "  {:<7} {}  {}",
            voice.weekday(day.day.weekday()),
            short(day.day),
            voice.summary_counts(day.goals.len(), day.benched.len(), day.stoppage.len())
        ));
    }
    if !week.offside.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "{} ({})",
            voice.offside_title(),
            week.offside.len()
        ));
        lines.extend(week.offside.iter().map(|offside| {
            format!(
                "  {}  {} {}: {}",
                short(offside.day),
                offside.task,
                offside.appointment,
                offside.violation
            )
        }));
    }
    if !week.habits.is_empty() {
        lines.push(String::new());
        lines.extend(habit_lines(&week.habits, voice));
    }
    lines.push(String::new());
    lines.push(voice.week_unbeaten(week.unbeaten.current, week.unbeaten.record, CLUB_RECORD));
    lines.join("\n")
}

/// O título e uma linha por hábito, com os nomes alinhados.
fn habit_lines(habits: &[HabitCount], voice: Voice) -> Vec<String> {
    let [.., title] = voice.summary_parts();
    let width = habits
        .iter()
        .map(|habit| habit.name.chars().count())
        .max()
        .unwrap_or(0);
    let mut lines = vec![title.to_owned()];
    lines.extend(
        habits
            .iter()
            .map(|habit| format!("  {:<width$}  {} {}", habit.name, habit.count, unit(habit))),
    );
    lines
}

/// A rodada em JSON, no formato de exportação, pro MCP.
pub(crate) fn week_json(week: &WeekSummary) -> String {
    let doc = vamo_sync::format::WeekDoc::from(week);
    serde_json::to_string(&doc).expect("a rodada só tem texto e número, e vira JSON sempre")
}

/// A unidade no plural quando precisa. O plural é o do português mais
/// simples, com um `s` no fim: serve pras unidades de hábito de sempre
/// (copo, xícara, página), e uma unidade irregular pode vir já no plural
/// na config.
fn unit(habit: &HabitCount) -> String {
    if habit.count == 1 || habit.unit.ends_with('s') {
        habit.unit.clone()
    } else {
        format!("{}s", habit.unit)
    }
}

/// A súmula em JSON, no formato de exportação: é o que fica guardado no
/// banco e o que o MCP lê.
pub(crate) fn json(summary: &DaySummary) -> String {
    let doc = vamo_sync::format::SummaryDoc::from(summary);
    serde_json::to_string(&doc).expect("a súmula só tem texto e número, e vira JSON sempre")
}

#[cfg(test)]
pub(crate) mod tests {
    use jiff::Timestamp;
    use jiff::civil::date;
    use uuid::Builder;
    use vamo_core::{Change, Event, EventId, Fit, Projection, Status, TaskId};

    use super::*;

    /// Um log de exemplo, com tarefas inventadas e o relógio andando na mão.
    pub(crate) struct Log {
        pub(crate) events: Vec<Event>,
        pub(crate) projection: Projection,
        now: Timestamp,
        n: u64,
    }

    impl Log {
        pub(crate) fn new() -> Self {
            Self {
                events: Vec::new(),
                projection: Projection::new(),
                now: "2026-11-01T12:00:00Z".parse().unwrap(),
                n: 0,
            }
        }

        /// Leva o relógio pro instante dado, em UTC.
        pub(crate) fn at(&mut self, instant: &str) {
            self.now = instant.parse().unwrap();
        }

        pub(crate) fn record(&mut self, id: TaskId, changes: Vec<Change>) {
            for change in changes {
                self.n += 1;
                let event = Event {
                    id: EventId::new(
                        Builder::from_unix_timestamp_millis(self.n, &[0; 10]).into_uuid(),
                    )
                    .unwrap(),
                    at: self.now,
                    machine: "notebook".parse().unwrap(),
                    task: id,
                    change,
                };
                self.projection.apply(&event);
                self.events.push(event);
            }
        }

        pub(crate) fn add(&mut self, title: &str, status: Status) -> TaskId {
            let id = self.projection.next_id().unwrap();
            self.record(
                id,
                vec![Change::Created {
                    title: title.parse().unwrap(),
                    area: "casa".parse().unwrap(),
                    fit: Fit::Gap,
                    status,
                }],
            );
            id
        }

        pub(crate) fn schedule(&mut self, id: TaskId, day: Date) {
            let changes = self.projection.schedule(id, day).unwrap();
            self.record(id, changes);
        }

        pub(crate) fn finish(&mut self, id: TaskId, day: Date) {
            let changes = self.projection.finish(id, day).unwrap();
            self.record(id, changes);
        }
    }

    /// O dia de exemplo, uma quarta, com uma tarefa em cada lugar.
    pub(crate) fn example_day() -> (Log, Date) {
        let day = date(2026, 11, 11);
        let mut log = Log::new();
        let filter = log.add("Trocar o filtro da água", Status::Ready);
        let bill = log.add("Pagar a conta de luz", Status::Ready);
        let car = log.add("Lavar o carro", Status::Ready);
        for id in [filter, bill, car] {
            log.schedule(id, day);
        }
        log.at("2026-11-11T13:00:00Z");
        log.finish(filter, day);
        log.at("2026-11-11T22:00:00Z");
        log.schedule(bill, date(2026, 11, 13));
        (log, day)
    }

    fn habits() -> Vec<HabitCount> {
        vec![
            HabitCount {
                name: "Água".to_owned(),
                unit: "copo".to_owned(),
                count: 6,
            },
            HabitCount {
                name: "Leitura".to_owned(),
                unit: "capítulo".to_owned(),
                count: 1,
            },
        ]
    }

    fn pretty(json: &str) -> String {
        serde_json::to_string_pretty(&serde_json::from_str::<Value>(json).unwrap()).unwrap()
    }

    #[test]
    fn the_summary_of_an_example_day() {
        let (log, day) = example_day();
        let summary = DaySummary::close(
            &log.events,
            day,
            window(day, &TimeZone::UTC).unwrap(),
            habits(),
        );
        insta::assert_snapshot!(text(&summary, Voice::Game));
        insta::assert_snapshot!("neutra", text(&summary, Voice::Neutral));
        insta::assert_snapshot!("json", pretty(&json(&summary)));
    }

    /// Uma semana de exemplo, de 09/11 a 15/11, com o dia de exemplo na
    /// quarta, um gol na segunda, a conta adiada pra sexta que venceu lá e um
    /// impedimento.
    fn example_week() -> WeekSummary {
        let (mut log, _) = example_day();
        let monday = date(2026, 11, 9);
        log.at("2026-11-08T12:00:00Z");
        let reading = log.add("Ler o capítulo novo", Status::Ready);
        log.schedule(reading, monday);
        log.at("2026-11-09T15:00:00Z");
        log.finish(reading, monday);
        let offside = vec![vamo_core::Offside {
            day: date(2026, 11, 13),
            task: "T-003".parse().unwrap(),
            appointment: "Lavar o carro".to_owned(),
            violation: vamo_core::Violation::AfterNightCap {
                cap: jiff::civil::time(22, 0, 0, 0),
            },
        }];
        WeekSummary::close(
            &log.events,
            days(monday, date(2026, 11, 15), &TimeZone::UTC).unwrap(),
            offside,
            vec![HabitCount {
                name: "Água".to_owned(),
                unit: "copo".to_owned(),
                count: 38,
            }],
            Unbeaten {
                current: 0,
                record: 3,
            },
        )
    }

    #[test]
    fn the_summary_of_an_example_week() {
        let week = example_week();
        insta::assert_snapshot!(week_text(&week, Voice::Game));
        insta::assert_snapshot!("semana_neutra", week_text(&week, Voice::Neutral));
        insta::assert_snapshot!("semana_json", pretty(&week_json(&week)));
    }

    #[test]
    fn a_day_without_tasks_says_so() {
        let day = date(2026, 11, 11);
        let summary = DaySummary::close(&[], day, window(day, &TimeZone::UTC).unwrap(), Vec::new());
        assert_eq!(
            text(&summary, Voice::Game),
            "Súmula de quarta, 11/11\n\nDia sem jogo: nenhuma tarefa escalada, nenhum gol."
        );
    }

    #[test]
    fn the_last_closed_day_turns_at_four() {
        let zoned = |text: &str| text.parse::<Zoned>().unwrap();
        assert_eq!(
            last_closed_day(&zoned("2026-11-12T03:59[America/Sao_Paulo]")).unwrap(),
            date(2026, 11, 10)
        );
        assert_eq!(
            last_closed_day(&zoned("2026-11-12T04:00[America/Sao_Paulo]")).unwrap(),
            date(2026, 11, 11)
        );
        assert_eq!(
            last_closed_day(&zoned("2026-11-12T18:00[America/Sao_Paulo]")).unwrap(),
            date(2026, 11, 11)
        );
    }

    #[test]
    fn the_window_goes_from_midnight_to_four_in_the_zone() {
        let zone = TimeZone::get("America/Sao_Paulo").unwrap();
        let window = window(date(2026, 11, 11), &zone).unwrap();
        assert_eq!(window.start.to_string(), "2026-11-11T03:00:00Z");
        assert_eq!(window.end.to_string(), "2026-11-12T07:00:00Z");
    }

    #[test]
    fn the_unbeaten_streak_runs_until_the_last_closed_day() {
        let (log, _) = example_day();
        let now: Zoned = "2026-11-12T10:00[UTC]".parse().unwrap();
        // O dia 11 teve uma vencida; os dias de antes não tiveram gol.
        assert_eq!(
            unbeaten(&log.events, &TimeZone::UTC, last_closed_day(&now).unwrap()).unwrap(),
            Unbeaten::default()
        );
        assert_eq!(
            unbeaten(&[], &TimeZone::UTC, last_closed_day(&now).unwrap()).unwrap(),
            Unbeaten::default()
        );
    }

    #[test]
    fn the_habit_tracker_config_gives_names_units_and_the_turn() {
        let (habits, turn) = parse_habits(
            r#"{"hora_virada_dia": 3, "habitos": {
                "agua": {"nome": "Água", "emoji": "x", "unidade": "copo"},
                "leitura": {"nome": "Leitura", "unidade": "capítulo"}
            }}"#,
        )
        .unwrap();
        assert_eq!(turn, 3);
        assert_eq!(
            habits.iter().map(|h| h.key.as_str()).collect::<Vec<_>>(),
            ["agua", "leitura"]
        );
        assert_eq!(habits[0].unit, "copo");
        assert!(parse_habits(r#"{"habitos": {"agua": {"nome": "Água"}}}"#).is_err());
        assert!(parse_habits(r#"{"hora_virada_dia": 30, "habitos": {}}"#).is_err());
    }
}
