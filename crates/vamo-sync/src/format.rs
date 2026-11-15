//! O formato de exportação: o contrato dos dados que saem do app.
//!
//! Cada tipo daqui vira uma linha de JSON (os eventos e as marcações, em
//! JSONL) ou um documento (a súmula e a rodada). Os nomes dos campos são em
//! português, como o resto do que quem usa vê, e cada linha carrega a
//! `versao` do formato.
//!
//! O JSON Schema de cada tipo sai destes mesmos tipos, com o `schemars`, e
//! fica guardado em `schema/v{VERSAO}/`. Um teste compara o schema gerado
//! com o guardado: mudar um tipo sem subir a [`VERSION`] quebra a CI.
//!
//! A regra de compatibilidade é uma só: **versão nova lê export antigo**.
//! Linha com versão maior que a [`VERSION`] é recusada (é de um vamo mais
//! novo, e ler pela metade perderia dado); linha com versão igual ou menor
//! é lida. Quando a versão 2 existir, a leitura da 1 continua aqui, e os
//! exemplos de cada versão em `tests/fixtures/` continuam passando.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use vamo_core::{
    Change, DaySummary, Event, EventId, HabitCount, Offside, Task, Unbeaten, WeekSummary,
};

use crate::Error;

/// A versão do formato que esta versão do vamo escreve. Sobe a cada mudança
/// em qualquer tipo deste módulo.
pub const VERSION: u32 = 2;

/// Uma linha do log de eventos exportado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EventLine {
    /// A versão do formato.
    pub versao: u32,
    /// O ID do evento, um UUID v7.
    pub id: String,
    /// Quando a mudança aconteceu, em RFC 3339 e UTC (`2026-11-11T12:00:00Z`).
    pub em: String,
    /// O nome da máquina de origem.
    pub maquina: String,
    /// A tarefa que mudou, como `T-041`.
    pub tarefa: String,
    /// O que mudou.
    pub mudanca: ChangeLine,
}

/// O que mudou numa tarefa, com o tipo da mudança no campo `tipo`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "tipo", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChangeLine {
    /// A tarefa nasceu, com os quatro campos de partida.
    Criada {
        /// O título, no padrão do caderno.
        titulo: String,
        /// A área.
        area: String,
        /// O encaixe: `foco`, `brecha` ou `com-alguém`.
        encaixe: String,
        /// O status de partida, como `pronta`.
        status: String,
    },
    /// O título mudou.
    Titulo {
        /// O título novo.
        titulo: String,
    },
    /// A área mudou.
    Area {
        /// A área nova.
        area: String,
    },
    /// O encaixe mudou.
    Encaixe {
        /// O encaixe novo.
        encaixe: String,
    },
    /// O status mudou.
    Status {
        /// O status novo.
        status: String,
    },
    /// As notas mudaram. Vazio apaga.
    Notas {
        /// As notas novas.
        notas: String,
    },
    /// A tarefa foi escalada pra um dia, ou saiu da escalação (`null`).
    Dia {
        /// O dia, como `2026-11-11`.
        dia: Option<String>,
    },
    /// O intervalo em que a tarefa volta depois do gol mudou (versão 2).
    Volta {
        /// O intervalo, como `1m` (todo mês) ou `2s` (a cada duas semanas).
        /// `null` tira: a tarefa deixa de voltar.
        intervalo: Option<String>,
    },
    /// Os passos da jogada em sequência mudaram (versão 2).
    Jogada {
        /// Um passo por linha, no formato `Título @48h + item`. `null` tira:
        /// a tarefa deixa de ser uma jogada.
        passos: Option<String>,
    },
    /// O ponto da jogada mudou (versão 2).
    Ponto {
        /// O passo da vez, os itens marcados e a espera, como `1,5,0`.
        ponto: String,
    },
    /// Uma marcação por tarefa (versões antigas; hoje a projeção ignora).
    Marcacao {
        /// O dia da marcação.
        dia: String,
        /// Se ficou marcada.
        feita: bool,
    },
}

impl From<&Event> for EventLine {
    fn from(event: &Event) -> Self {
        let mudanca = match &event.change {
            Change::Created {
                title,
                area,
                fit,
                status,
            } => ChangeLine::Criada {
                titulo: title.to_string(),
                area: area.to_string(),
                encaixe: fit.to_string(),
                status: status.to_string(),
            },
            Change::Title(title) => ChangeLine::Titulo {
                titulo: title.to_string(),
            },
            Change::Area(area) => ChangeLine::Area {
                area: area.to_string(),
            },
            Change::Fit(fit) => ChangeLine::Encaixe {
                encaixe: fit.to_string(),
            },
            Change::Status(status) => ChangeLine::Status {
                status: status.to_string(),
            },
            Change::Notes(notes) => ChangeLine::Notas {
                notas: notes.clone(),
            },
            Change::Day(day) => ChangeLine::Dia {
                dia: day.map(|day| day.to_string()),
            },
            Change::Plan(plan) => ChangeLine::Jogada {
                passos: plan.as_ref().map(ToString::to_string),
            },
            Change::Progress(progress) => ChangeLine::Ponto {
                ponto: progress.to_string(),
            },
            Change::Repeat(interval) => ChangeLine::Volta {
                intervalo: interval.map(|interval| interval.to_string()),
            },
            Change::Check { day, done } => ChangeLine::Marcacao {
                dia: day.to_string(),
                feita: *done,
            },
        };
        Self {
            versao: VERSION,
            id: event.id.to_string(),
            em: event.at.to_string(),
            maquina: event.machine.to_string(),
            tarefa: event.task.to_string(),
            mudanca,
        }
    }
}

impl TryFrom<EventLine> for Event {
    type Error = Error;

    /// Volta a linha pro evento do domínio, conferindo cada campo de novo
    /// pelo tipo dele.
    fn try_from(line: EventLine) -> Result<Self, Error> {
        check_version(line.versao)?;
        let change = match line.mudanca {
            ChangeLine::Criada {
                titulo,
                area,
                encaixe,
                status,
            } => Change::Created {
                title: titulo.parse()?,
                area: area.parse()?,
                fit: encaixe.parse()?,
                status: status.parse()?,
            },
            ChangeLine::Titulo { titulo } => Change::Title(titulo.parse()?),
            ChangeLine::Area { area } => Change::Area(area.parse()?),
            ChangeLine::Encaixe { encaixe } => Change::Fit(encaixe.parse()?),
            ChangeLine::Status { status } => Change::Status(status.parse()?),
            ChangeLine::Notas { notas } => Change::Notes(notas),
            ChangeLine::Dia { dia } => Change::Day(dia.as_deref().map(date).transpose()?),
            ChangeLine::Jogada { passos } => {
                Change::Plan(passos.as_deref().map(str::parse).transpose()?)
            }
            ChangeLine::Ponto { ponto } => Change::Progress(ponto.parse()?),
            ChangeLine::Volta { intervalo } => {
                Change::Repeat(intervalo.as_deref().map(str::parse).transpose()?)
            }
            ChangeLine::Marcacao { dia, feita } => Change::Check {
                day: date(&dia)?,
                done: feita,
            },
        };
        Ok(Self {
            id: line.id.parse::<EventId>()?,
            at: line.em.parse().map_err(Error::Time)?,
            machine: line.maquina.parse()?,
            task: line.tarefa.parse()?,
            change,
        })
    }
}

/// Uma marcação de atividade fixa da agenda: o evento `uid` marcado (ou
/// desmarcado) no dia. Na sincronia, a mais recente de cada `(dia, uid)`
/// ganha.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MarkLine {
    /// A versão do formato.
    pub versao: u32,
    /// O dia da marcação, como `2026-11-11`.
    pub dia: String,
    /// O UID do evento na agenda.
    pub uid: String,
    /// Se ficou marcada.
    pub feita: bool,
    /// Quando foi marcada ou desmarcada, em RFC 3339 e UTC.
    pub em: String,
}

/// Um reforço: uma tarefa que chega de fora (do Claude, de outro projeto)
/// pela branch `vamo-entrada` do backup, uma por linha do `reforcos.jsonl`.
/// Só a chave e o título são obrigatórios; o resto tem o padrão do `vamo
/// add`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReinforcementLine {
    /// A versão do formato.
    pub versao: u32,
    /// Uma chave única de quem mandou, como `claude-2026-11-14-filtro`. É
    /// ela que impede o mesmo reforço de virar duas tarefas.
    pub chave: String,
    /// O título, no padrão do caderno: verbo na frente e até cinco palavras.
    pub titulo: String,
    /// A área. Sem ela, `geral`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<String>,
    /// O encaixe: `foco`, `brecha` ou `com-alguém`. Sem ele, `brecha`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encaixe: Option<String>,
    /// O status de partida. Sem ele, `pronta`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// As notas: o detalhe que não cabe no título.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notas: Option<String>,
}

/// Uma tarefa num resumo: o ID e o título.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskRef {
    /// O ID, como `T-041`.
    pub id: String,
    /// O título.
    pub titulo: String,
}

impl From<&Task> for TaskRef {
    fn from(task: &Task) -> Self {
        Self {
            id: task.id().to_string(),
            titulo: task.title().to_string(),
        }
    }
}

/// Quanto de um hábito foi registrado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HabitLine {
    /// O nome do hábito.
    pub nome: String,
    /// A unidade, no singular.
    pub unidade: String,
    /// Quantas vezes foi registrado.
    pub quantidade: u32,
}

impl From<&HabitCount> for HabitLine {
    fn from(habit: &HabitCount) -> Self {
        Self {
            nome: habit.name.clone(),
            unidade: habit.unit.clone(),
            quantidade: habit.count,
        }
    }
}

/// A súmula de um dia que fechou.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SummaryDoc {
    /// A versão do formato.
    pub versao: u32,
    /// O dia, como `2026-11-11`.
    pub dia: String,
    /// Os gols: as tarefas do dia concluídas.
    pub gols: Vec<TaskRef>,
    /// As recorrentes marcadas no dia.
    pub fixas: Vec<TaskRef>,
    /// As que estavam no dia e saíram dele sem gol.
    pub voltaram_pro_banco: Vec<TaskRef>,
    /// As que ficaram no dia sem gol e venceram.
    pub acrescimos: Vec<TaskRef>,
    /// Os números dos hábitos do dia.
    pub habitos: Vec<HabitLine>,
}

impl From<&DaySummary> for SummaryDoc {
    fn from(summary: &DaySummary) -> Self {
        let tasks = |tasks: &[Task]| tasks.iter().map(TaskRef::from).collect();
        Self {
            versao: VERSION,
            dia: summary.day.to_string(),
            gols: tasks(&summary.goals),
            // Segue no formato v1 por compatibilidade: tarefa não é mais
            // marcada por dia (ADR 0014), então sai sempre vazio.
            fixas: Vec::new(),
            voltaram_pro_banco: tasks(&summary.benched),
            acrescimos: tasks(&summary.stoppage),
            habitos: summary.habits.iter().map(HabitLine::from).collect(),
        }
    }
}

/// Um dia dentro da rodada: a súmula sem os hábitos, que vão somados.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WeekDay {
    /// O dia, como `2026-11-11`.
    pub dia: String,
    /// Os gols do dia.
    pub gols: Vec<TaskRef>,
    /// As recorrentes marcadas no dia.
    pub fixas: Vec<TaskRef>,
    /// As que voltaram pro banco no dia.
    pub voltaram_pro_banco: Vec<TaskRef>,
    /// As que venceram no dia.
    pub acrescimos: Vec<TaskRef>,
}

/// Um impedimento: tarefa marcada na agenda fora das regras do tempo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OffsideLine {
    /// O dia do compromisso.
    pub dia: String,
    /// A tarefa ligada ao compromisso.
    pub tarefa: String,
    /// O título do compromisso.
    pub compromisso: String,
    /// A regra ferida, em texto.
    pub regra: String,
}

impl From<&Offside> for OffsideLine {
    fn from(offside: &Offside) -> Self {
        Self {
            dia: offside.day.to_string(),
            tarefa: offside.task.to_string(),
            compromisso: offside.appointment.clone(),
            regra: offside.violation.to_string(),
        }
    }
}

/// A invencibilidade.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UnbeatenLine {
    /// Os dias seguidos sem vencida até o fim da rodada.
    pub atual: u32,
    /// A maior sequência que já teve.
    pub recorde: u32,
    /// O recorde do clube: 52 jogos, em 1977 e 1978.
    pub recorde_do_clube: u32,
}

impl From<Unbeaten> for UnbeatenLine {
    fn from(unbeaten: Unbeaten) -> Self {
        Self {
            atual: unbeaten.current,
            recorde: unbeaten.record,
            recorde_do_clube: vamo_core::CLUB_RECORD,
        }
    }
}

/// A rodada: o resumo de uma semana, de segunda a domingo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WeekDoc {
    /// A versão do formato.
    pub versao: u32,
    /// A segunda-feira da semana.
    pub inicio: String,
    /// O domingo da semana.
    pub fim: String,
    /// Os gols da semana.
    pub gols: usize,
    /// As tarefas que voltaram pro banco na semana.
    pub voltaram_pro_banco: usize,
    /// As tarefas que venceram na semana.
    pub acrescimos: usize,
    /// Cada dia da semana, em ordem.
    pub dias: Vec<WeekDay>,
    /// Os impedimentos da semana.
    pub impedimentos: Vec<OffsideLine>,
    /// Os hábitos somados na semana.
    pub habitos: Vec<HabitLine>,
    /// A invencibilidade no fim da semana.
    pub invencibilidade: UnbeatenLine,
}

impl From<&WeekSummary> for WeekDoc {
    fn from(week: &WeekSummary) -> Self {
        let day = |pick: fn(&[DaySummary]) -> Option<&DaySummary>| {
            pick(&week.days)
                .map(|day| day.day.to_string())
                .unwrap_or_default()
        };
        let tasks = |tasks: &[Task]| tasks.iter().map(TaskRef::from).collect();
        Self {
            versao: VERSION,
            inicio: day(<[DaySummary]>::first),
            fim: day(<[DaySummary]>::last),
            gols: week.goals(),
            voltaram_pro_banco: week.benched(),
            acrescimos: week.stoppage(),
            dias: week
                .days
                .iter()
                .map(|day| WeekDay {
                    dia: day.day.to_string(),
                    gols: tasks(&day.goals),
                    fixas: Vec::new(),
                    voltaram_pro_banco: tasks(&day.benched),
                    acrescimos: tasks(&day.stoppage),
                })
                .collect(),
            impedimentos: week.offside.iter().map(OffsideLine::from).collect(),
            habitos: week.habits.iter().map(HabitLine::from).collect(),
            invencibilidade: week.unbeaten.into(),
        }
    }
}

/// Confere se a linha é de uma versão que esta versão do vamo lê.
///
/// # Errors
///
/// [`Error::NewerFormat`] se a versão for maior que a [`VERSION`].
pub fn check_version(found: u32) -> Result<(), Error> {
    if found > VERSION {
        return Err(Error::NewerFormat {
            found,
            known: VERSION,
        });
    }
    Ok(())
}

/// Um dia no formato `2026-11-11`.
fn date(text: &str) -> Result<jiff::civil::Date, Error> {
    text.parse().map_err(Error::Time)
}

/// Os schemas de cada tipo do formato, com o nome do arquivo de cada um em
/// `schema/v{VERSAO}/`.
pub fn schemas() -> Vec<(&'static str, schemars::Schema)> {
    vec![
        ("evento.json", schemars::schema_for!(EventLine)),
        ("marcacao.json", schemars::schema_for!(MarkLine)),
        ("reforco.json", schemars::schema_for!(ReinforcementLine)),
        ("resumo.json", schemars::schema_for!(SummaryDoc)),
        ("rodada.json", schemars::schema_for!(WeekDoc)),
    ]
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use uuid::Builder;
    use vamo_core::{Fit, Status};

    use super::*;

    fn event(n: u64, change: Change) -> Event {
        Event {
            id: EventId::new(Builder::from_unix_timestamp_millis(n, &[7; 10]).into_uuid()).unwrap(),
            at: jiff::Timestamp::from_millisecond(i64::try_from(n).unwrap()).unwrap(),
            machine: "notebook".parse().unwrap(),
            task: "T-041".parse().unwrap(),
            change,
        }
    }

    #[test]
    fn every_kind_of_change_goes_and_comes_back_the_same() {
        let changes = vec![
            Change::Created {
                title: "Trocar o filtro".parse().unwrap(),
                area: "casa".parse().unwrap(),
                fit: Fit::WithSomeone,
                status: Status::Ready,
            },
            Change::Title("Lavar o carro".parse().unwrap()),
            Change::Area("saúde".parse().unwrap()),
            Change::Fit(Fit::Focus),
            Change::Status(Status::Scheduled),
            Change::Notes("Comprar o filtro\nantes".to_owned()),
            Change::Day(Some(date(2026, 11, 11))),
            Change::Day(None),
            Change::Check {
                day: date(2026, 11, 11),
                done: true,
            },
            Change::Repeat(Some("1m".parse().unwrap())),
            Change::Repeat(None),
            Change::Plan(Some("Pedir @pedir + a + b\nFazer @48h".parse().unwrap())),
            Change::Plan(None),
            Change::Progress("1,3,1".parse().unwrap()),
        ];
        for (n, change) in (1..).zip(changes) {
            let original = event(n, change);
            let text = serde_json::to_string(&EventLine::from(&original)).unwrap();
            let line: EventLine = serde_json::from_str(&text).unwrap();
            assert_eq!(Event::try_from(line).unwrap(), original, "{text}");
        }
    }

    #[test]
    fn a_line_from_a_newer_version_is_refused() {
        let mut line = EventLine::from(&event(1, Change::Notes(String::new())));
        line.versao = VERSION + 1;
        assert!(matches!(
            Event::try_from(line),
            Err(Error::NewerFormat { found, .. }) if found == VERSION + 1
        ));
    }

    #[test]
    fn a_value_the_domain_rejects_is_an_error() {
        let mut line = EventLine::from(&event(1, Change::Status(Status::Done)));
        line.mudanca = ChangeLine::Status {
            status: "finalizada".to_owned(),
        };
        assert!(Event::try_from(line).is_err());
    }

    #[test]
    fn an_unknown_field_is_an_error_not_silence() {
        let text = r#"{"versao":1,"id":"0192f0c4-8a00-7000-8000-000000000001","em":"2026-11-11T12:00:00Z","maquina":"pc","tarefa":"T-001","mudanca":{"tipo":"notas","notas":""},"extra":1}"#;
        assert!(serde_json::from_str::<EventLine>(text).is_err());
    }
}
