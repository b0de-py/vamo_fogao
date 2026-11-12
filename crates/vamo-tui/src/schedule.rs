//! A agenda do dia na tela Hoje: os compromissos, as regras que algum
//! deles fere (em amarelo plasma) e as brechas com as tarefas que cabem.

use jiff::SignedDuration;
use jiff::civil::{Date, DateTime, Time};
use ratatui::text::{Line, Span};
use vamo_core::{
    Appointment, FixedActivity, GameDay, Gap, Projection, Task, TimeRules, When, fitting, gaps,
};

use crate::{Voice, theme};

/// A agenda que a TUI recebe de quem a chama. A TUI não lê a agenda: quem
/// liga o leitor é o binário, que entrega só os compromissos.
#[derive(Debug, Clone, Default)]
pub struct Schedule {
    /// Os compromissos do dia, já no fuso de quem usa. Os recorrentes
    /// ocupam as brechas, mas na tela aparecem como atividades fixas.
    pub appointments: Vec<Appointment>,
    /// As atividades fixas do dia, com o título curto: itens de marcar na
    /// escalação, que contam no placar sem virar tarefa.
    pub fixed: Vec<FixedActivity>,
    /// As regras do tempo da config.
    pub rules: TimeRules,
    /// O tamanho mínimo de uma brecha.
    pub min_gap: SignedDuration,
    /// A hora em que a TUI abriu: brecha que já passou não aparece.
    pub now: Option<Time>,
    /// O dia de jogo, se hoje tem jogo. No modo neutro, quem chama deixa
    /// vazio: a TUI não fala de futebol.
    pub game: Option<GameDay>,
    /// O nome do time, pro card do dia de jogo.
    pub team: String,
}

/// Quantas tarefas sugerir por brecha, pra linha não virar parágrafo.
const SUGGESTIONS: usize = 3;

impl Schedule {
    /// Se tem alguma coisa pra mostrar: sem agenda e sem expediente, a tela
    /// Hoje fica como era.
    fn is_empty(&self) -> bool {
        self.appointments.is_empty() && self.rules.workday.is_none()
    }

    /// As linhas da agenda e das brechas, pra ir embaixo da escalação.
    pub(crate) fn lines(
        &self,
        projection: &Projection,
        today: Date,
        width: usize,
        voice: Voice,
    ) -> Vec<Line<'static>> {
        if self.is_empty() {
            return Vec::new();
        }
        let mut lines = vec![Line::default(), Line::styled("Agenda", theme::heading())];
        let once: Vec<&Appointment> = self
            .appointments
            .iter()
            .filter(|appointment| !appointment.recurring)
            .collect();
        if once.is_empty() {
            lines.push(Line::styled("nada marcado além das fixas", theme::muted()));
        }
        for appointment in once {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<11}  ", when(appointment.when, today)),
                    theme::soft(),
                ),
                // O horário ocupa 13 colunas; o título fica com o resto.
                Span::styled(
                    fit(&appointment.title, width.saturating_sub(13)),
                    theme::text(),
                ),
            ]));
            for warning in self.violations(appointment, projection) {
                lines.push(Line::styled(
                    format!("{:13}⚠ {warning}", ""),
                    theme::warning(),
                ));
            }
        }

        let found = gaps(
            today,
            &self.rules,
            &self.appointments,
            self.min_gap,
            self.now,
        );
        if self.rules.workday.is_some() {
            lines.push(Line::default());
            lines.push(Line::styled(voice.gaps(), theme::heading()));
            if found.is_empty() {
                lines.push(Line::styled(voice.no_gaps(), theme::muted()));
            }
            let tasks: Vec<Task> = projection.tasks().collect();
            for gap in found {
                // Perto do jogo, a brecha só sugere tarefa leve.
                let near_game = self.game.as_ref().filter(|game_day| {
                    let (from, to) = game_day.light_only();
                    gap.start < to && from < gap.end
                });
                let light: Vec<Task>;
                let tasks = match near_game {
                    Some(game_day) => {
                        light = tasks
                            .iter()
                            .filter(|t| game_day.allows(t))
                            .cloned()
                            .collect();
                        &light
                    }
                    None => &tasks,
                };
                lines.push(gap_line(gap, today, tasks, &self.appointments));
            }
        }
        lines
    }

    /// As regras que a tarefa ligada ao compromisso fere.
    fn violations(&self, appointment: &Appointment, projection: &Projection) -> Vec<String> {
        let (Some(id), Some((start, end))) = (appointment.task, appointment.span()) else {
            return Vec::new();
        };
        let Some(task) = projection.task(id) else {
            return Vec::new();
        };
        let game = self
            .game
            .as_ref()
            .and_then(|game_day| game_day.check(&task, start, end));
        self.rules
            .check(&task, start, end)
            .iter()
            .chain(game.iter())
            .map(|violation| format!("{id}: {violation}"))
            .collect()
    }
}

/// `10:00–12:00  2h        T-004 Comprar o gesso · T-007 Ligar pro síndico`
fn gap_line(gap: Gap, today: Date, tasks: &[Task], agenda: &[Appointment]) -> Line<'static> {
    let mut spans = vec![
        Span::styled(
            format!("{}–{}  ", hm(gap.start), hm(gap.end)),
            theme::soft(),
        ),
        Span::styled(format!("{:<8}", length(gap.length())), theme::muted()),
    ];
    let fits = fitting(gap, today, tasks, agenda);
    for (index, task) in fits.iter().take(SUGGESTIONS).enumerate() {
        if index > 0 {
            spans.push(Span::styled(" · ", theme::muted()));
        }
        spans.push(Span::styled(format!("{} ", task.id()), theme::muted()));
        spans.push(Span::styled(task.title().to_string(), theme::text()));
    }
    if fits.len() > SUGGESTIONS {
        spans.push(Span::styled(
            format!(" +{}", fits.len() - SUGGESTIONS),
            theme::muted(),
        ));
    }
    Line::from(spans)
}

/// O horário do compromisso no dia: `09:00–09:30`, ou `dia inteiro`.
pub(crate) fn when(when: When, today: Date) -> String {
    match when {
        When::AllDay(_) => "dia inteiro".to_owned(),
        When::Timed { start, end } => {
            // O que começou ontem aparece desde 00:00; o que vai até amanhã,
            // até 24:00.
            let begin = if start.date() < today {
                "00:00".to_owned()
            } else {
                hm(start)
            };
            let finish = if end.date() > today {
                "24:00".to_owned()
            } else {
                hm(end)
            };
            format!("{begin}–{finish}")
        }
    }
}

fn hm(at: DateTime) -> String {
    at.strftime("%H:%M").to_string()
}

/// O texto inteiro se couber em `width` colunas; se não, cortado com `…`
/// no fim. Conta caracteres, que é o que o terminal mostra numa coluna cada
/// (emoji ocupa duas, e aí o corte sobra uma coluna, sem estragar a linha).
pub(crate) fn fit(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_owned();
    }
    let kept: String = text.chars().take(width.saturating_sub(1)).collect();
    format!("{}…", kept.trim_end())
}

/// `45 min`, `2h`, `1h30`.
fn length(span: SignedDuration) -> String {
    let minutes = span.as_mins();
    match (minutes / 60, minutes % 60) {
        (0, minutes) => format!("{minutes} min"),
        (hours, 0) => format!("{hours}h"),
        (hours, minutes) => format!("{hours}h{minutes:02}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_read_like_people_write() {
        assert_eq!(length(SignedDuration::from_mins(45)), "45 min");
        assert_eq!(length(SignedDuration::from_mins(120)), "2h");
        assert_eq!(length(SignedDuration::from_mins(90)), "1h30");
        assert_eq!(length(SignedDuration::from_mins(65)), "1h05");
    }

    #[test]
    fn titles_are_cut_only_when_they_do_not_fit() {
        assert_eq!(fit("Caminhada no parque", 30), "Caminhada no parque");
        assert_eq!(fit("Caminhada no parque", 19), "Caminhada no parque");
        assert_eq!(fit("Caminhada no parque", 12), "Caminhada n…");
        assert_eq!(fit("Caminhada", 0), "…");
    }
}
