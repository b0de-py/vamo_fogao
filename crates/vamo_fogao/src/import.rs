//! A migração do banco de tarefas antigo, um markdown com uma tabela de
//! índice, pro `vamo_fogao`.
//!
//! O formato antigo é solto: área composta (`lazer / música`), status com
//! comentário (`agendada (qui 19h)`), encaixe duplo (`brecha + foco`) e
//! título comprido. Cada item passa por uma triagem que decide se ele migra,
//! fica arquivado no arquivo antigo ou precisa de alguém pra decidir. Nada
//! se perde: o que não cabe nos campos novos vai pras notas da tarefa.

use std::collections::BTreeSet;

use color_eyre::eyre::{Result, eyre};
use vamo_core::{Area, Fit, NotebookTitle, Status, TaskId};

use vamo_tui::Voice;

/// Um item do índice do banco antigo, com o texto de cada coluna.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct OldItem {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) area: String,
    pub(crate) status: String,
    pub(crate) fit: String,
    pub(crate) duration: String,
    pub(crate) frequency: String,
}

/// O que a triagem decidiu pra um item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Decision {
    /// Entra no banco com o mesmo ID. Tarefa concluída ou descartada entra
    /// arquivada: fica fora da lista, mas guarda o ID e o histórico.
    Migrate(Migration),
    /// Fica só no arquivo antigo; o motivo diz por quê.
    Leave { id: String, reason: String },
    /// Precisa de alguém decidir antes do import; o motivo diz o quê.
    Decide { id: String, reason: String },
}

/// Uma tarefa pronta pra entrar no banco.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Migration {
    pub(crate) id: TaskId,
    pub(crate) title: NotebookTitle,
    pub(crate) area: Area,
    pub(crate) fit: Fit,
    pub(crate) status: Status,
    /// Se o título foi cortado pra caber no caderno.
    pub(crate) title_was_cut: bool,
    /// O que não coube nos campos, uma informação por linha.
    pub(crate) notes: Vec<String>,
}

/// Lê a tabela do índice: a que tem a linha de cabeçalho começando com
/// `| ID |`. As colunas são achadas pelo nome, então a ordem não importa.
pub(crate) fn parse(markdown: &str) -> Result<Vec<OldItem>> {
    let mut lines = markdown.lines();
    let header = lines
        .by_ref()
        .map(cells)
        .find(|cells| cells.first().is_some_and(|cell| cell == "ID"))
        .ok_or_else(|| eyre!("não achei a tabela do índice: a linha que começa com `| ID |`"))?;

    let column = |name: &str| header.iter().position(|cell| cell == name);
    let required = |name: &str| {
        column(name).ok_or_else(|| eyre!("a tabela do índice não tem a coluna `{name}`"))
    };
    let id = required("ID")?;
    let title = required("Título")?;
    let area = required("Área")?;
    let status = required("Status")?;
    let fit = required("Encaixe")?;
    let duration = column("Duração");
    let frequency = column("Frequência");

    let mut items = Vec::new();
    for line in lines {
        // A tabela acaba na primeira linha que não começa com `|`.
        if !line.trim_start().starts_with('|') {
            break;
        }
        let row = cells(line);
        let is_separator = row
            .iter()
            .all(|cell| cell.chars().all(|c| matches!(c, '-' | ':')));
        if is_separator {
            continue;
        }
        let get = |index: usize| row.get(index).cloned().unwrap_or_default();
        items.push(OldItem {
            id: get(id),
            title: get(title),
            area: get(area),
            status: get(status),
            fit: get(fit),
            duration: duration.map(get).unwrap_or_default(),
            frequency: frequency.map(get).unwrap_or_default(),
        });
    }
    Ok(items)
}

/// As células de uma linha de tabela markdown, sem os espaços das pontas.
/// Linha que não é de tabela não tem célula.
fn cells(line: &str) -> Vec<String> {
    let Some(inner) = line.trim().strip_prefix('|') else {
        return Vec::new();
    };
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    inner
        .split('|')
        .map(|cell| cell.trim().to_owned())
        .collect()
}

/// A triagem de um item: migra, fica no arquivo antigo ou fica pra decidir.
///
/// Tarefa em aberto com algum campo que não dá pra converter fica pra
/// decidir, porque perder ela seria pior que parar o import. Já a tarefa
/// terminada com campo ruim fica só no arquivo antigo: não faz falta no app.
pub(crate) fn triage(item: &OldItem) -> Decision {
    if item.id.starts_with("H-") {
        return Decision::Leave {
            id: item.id.clone(),
            reason: "é um hábito, e os hábitos chegam na fase 5".to_owned(),
        };
    }
    let decide = |reason: String| Decision::Decide {
        id: item.id.clone(),
        reason,
    };
    let Ok(id) = item.id.parse::<TaskId>() else {
        return decide(format!("`{}` não é um ID no formato T-###", item.id));
    };
    let Some(status) = parse_status(&item.status) else {
        return decide(format!("o status `{}` não está claro", item.status));
    };
    match migration(item, id, status) {
        Ok(migration) => Decision::Migrate(migration),
        Err(reason) if is_finished(status) => Decision::Leave {
            id: item.id.clone(),
            reason,
        },
        Err(reason) => decide(reason),
    }
}

/// Se o status é de tarefa terminada, que entra arquivada.
pub(crate) fn is_finished(status: Status) -> bool {
    matches!(status, Status::Done | Status::Dropped)
}

/// Converte os campos do item. O erro é o motivo, pronto pra mostrar.
fn migration(item: &OldItem, id: TaskId, status: Status) -> Result<Migration, String> {
    let area =
        parse_area(&item.area).ok_or_else(|| format!("a área `{}` não está clara", item.area))?;
    let fit =
        parse_fit(&item.fit).ok_or_else(|| format!("o encaixe `{}` não está claro", item.fit))?;
    let (title, title_was_cut) = match NotebookTitle::new(&item.title) {
        Ok(title) => (title, false),
        Err(vamo_core::Error::TitleTooLong { suggestion, .. }) => (suggestion, true),
        Err(error) => return Err(error.to_string()),
    };

    let mut notes = Vec::new();
    if title_was_cut {
        notes.push(format!("Título no banco antigo: {}", item.title));
    }
    for (label, old, new) in [
        ("Status", &item.status, status.as_str()),
        ("Área", &item.area, area.as_str()),
        ("Encaixe", &item.fit, fit.as_str()),
    ] {
        if old != new {
            notes.push(format!("{label} no banco antigo: {old}"));
        }
    }
    for (label, value) in [("Duração", &item.duration), ("Frequência", &item.frequency)] {
        if !value.is_empty() && value != "?" {
            notes.push(format!("{label}: {value}"));
        }
    }

    Ok(Migration {
        id,
        title,
        area,
        fit,
        status,
        title_was_cut,
        notes,
    })
}

/// A triagem do arquivo inteiro, separada pro relatório.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Plan {
    /// Quantos itens o índice tem.
    pub(crate) total: usize,
    /// O que entra no banco, em aberto ou arquivado.
    pub(crate) migrate: Vec<Migration>,
    /// O que já estava no banco, de um import anterior.
    pub(crate) already: Vec<TaskId>,
    /// O que fica só no arquivo antigo, com o motivo.
    pub(crate) leave: Vec<(String, String)>,
    /// O que precisa de decisão antes do import, com o motivo.
    pub(crate) decide: Vec<(String, String)>,
}

impl Plan {
    /// Faz a triagem de cada item. O `in_the_bank` diz se um ID já está no
    /// banco: o que já entrou não entra de novo, então rodar o import duas
    /// vezes não duplica nada.
    pub(crate) fn new(items: &[OldItem], in_the_bank: impl Fn(TaskId) -> bool) -> Self {
        let mut plan = Self {
            total: items.len(),
            ..Self::default()
        };
        let mut seen = BTreeSet::new();
        for item in items {
            match triage(item) {
                Decision::Migrate(migration) => {
                    if !seen.insert(migration.id) {
                        plan.decide.push((
                            item.id.clone(),
                            "o ID aparece mais de uma vez no arquivo".to_owned(),
                        ));
                    } else if in_the_bank(migration.id) {
                        plan.already.push(migration.id);
                    } else {
                        plan.migrate.push(migration);
                    }
                }
                Decision::Leave { id, reason } => plan.leave.push((id, reason)),
                Decision::Decide { id, reason } => plan.decide.push((id, reason)),
            }
        }
        plan
    }

    /// O relatório da triagem. A última linha diz o que foi feito: nada, no
    /// ensaio, ou quantas tarefas entraram.
    pub(crate) fn render(&self, voice: Voice, dry_run: bool) -> String {
        let mut lines = vec![format!("Banco antigo: {} itens.", self.total)];

        let (finished, open): (Vec<&Migration>, Vec<&Migration>) =
            self.migrate.iter().partition(|m| is_finished(m.status));
        section(
            &mut lines,
            "Entram no banco",
            open.iter().map(|m| task_line(m)),
        );
        section(
            &mut lines,
            "Entram arquivadas",
            finished.iter().map(|m| task_line(m)),
        );
        section(
            &mut lines,
            "Já estavam no banco",
            self.already.iter().map(|id| format!("  {id}")),
        );
        section(
            &mut lines,
            "Ficam só no arquivo antigo",
            self.leave.iter().map(|(id, why)| format!("  {id}  {why}")),
        );
        section(
            &mut lines,
            "Pra decidir",
            self.decide.iter().map(|(id, why)| format!("  {id}  {why}")),
        );
        if !self.decide.is_empty() {
            lines.push(String::new());
            lines.push(
                "Ajuste esses itens no arquivo (ou apague a linha) e rode de novo: o import só \
                 grava quando não sobra nada pra decidir."
                    .to_owned(),
            );
        }

        lines.push(String::new());
        lines.push(if dry_run {
            "Ensaio: nada foi gravado.".to_owned()
        } else {
            voice.imported(self.migrate.len())
        });
        lines.join("\n")
    }
}

/// Uma seção do relatório, com o título e a contagem. Seção vazia some.
fn section(lines: &mut Vec<String>, title: &str, items: impl ExactSizeIterator<Item = String>) {
    if items.len() == 0 {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{title} ({}):", items.len()));
    lines.extend(items);
}

/// Uma tarefa que entra, numa linha do relatório.
fn task_line(m: &Migration) -> String {
    let cut = if m.title_was_cut {
        "  (título cortado; o original vai pras notas)"
    } else {
        ""
    };
    format!(
        "  {}  {}  ({}, {}, {}){cut}",
        m.id, m.title, m.area, m.fit, m.status
    )
}

/// O status é a primeira palavra (`agendada (qui 19h)` é agendada). Status
/// com interrogação (`pronta?`) não está decidido.
fn parse_status(text: &str) -> Option<Status> {
    let word: String = text
        .trim()
        .chars()
        .take_while(|&c| c.is_alphabetic() || c == '?')
        .collect();
    if word.ends_with('?') {
        return None;
    }
    word.parse().ok()
}

/// A área é a primeira de uma área composta (`lazer / música` é lazer).
fn parse_area(text: &str) -> Option<Area> {
    let first = text.split('/').next()?.trim();
    first.parse().ok()
}

/// O encaixe é o primeiro que aparecer no texto (`brecha + foco` é brecha,
/// `brecha (saída)` também).
fn parse_fit(text: &str) -> Option<Fit> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '-'))
        .find_map(|word| word.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, title: &str, area: &str, status: &str, fit: &str) -> OldItem {
        OldItem {
            id: id.to_owned(),
            title: title.to_owned(),
            area: area.to_owned(),
            status: status.to_owned(),
            fit: fit.to_owned(),
            ..OldItem::default()
        }
    }

    fn reason(decision: Decision) -> String {
        match decision {
            Decision::Decide { reason, .. } => reason,
            other => panic!("deveria ficar pra decidir, mas deu {other:?}"),
        }
    }

    #[test]
    fn reads_the_index_table_by_column_name() {
        let markdown = "\
# banco

| ID | Status | Título | Área | Encaixe | Duração |
|---|---|---|---|---|---|
| T-001 | pronta | Trocar o filtro | casa | brecha | ~10 min |
| T-002 | ideia | Ler o livro | estudos | foco | ? |

## Itens
| T-999 | fora | da | tabela | do índice |
";
        let items = parse(markdown).unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0],
            OldItem {
                id: "T-001".to_owned(),
                title: "Trocar o filtro".to_owned(),
                area: "casa".to_owned(),
                status: "pronta".to_owned(),
                fit: "brecha".to_owned(),
                duration: "~10 min".to_owned(),
                frequency: String::new(),
            }
        );
    }

    #[test]
    fn a_file_without_the_index_is_an_error() {
        let error = parse("# só um título\n\nnada de tabela").unwrap_err();
        assert!(error.to_string().contains("não achei a tabela do índice"));
    }

    #[test]
    fn a_missing_column_is_an_error() {
        let error = parse("| ID | Título | Área | Status |\n|---|---|---|---|\n").unwrap_err();
        assert_eq!(
            error.to_string(),
            "a tabela do índice não tem a coluna `Encaixe`"
        );
    }

    #[test]
    fn a_clean_item_migrates_as_it_is() {
        let decision = triage(&item(
            "T-001",
            "Trocar o filtro",
            "casa",
            "pronta",
            "brecha",
        ));
        assert_eq!(
            decision,
            Decision::Migrate(Migration {
                id: TaskId::new(1).unwrap(),
                title: "Trocar o filtro".parse().unwrap(),
                area: "casa".parse().unwrap(),
                fit: Fit::Gap,
                status: Status::Ready,
                title_was_cut: false,
                notes: Vec::new(),
            })
        );
    }

    #[test]
    fn what_does_not_fit_the_new_fields_goes_to_the_notes() {
        let mut old = item(
            "T-002",
            "Comprar ração e areia para o gato",
            "casa / família",
            "agendada (sáb de manhã)",
            "brecha + foco",
        );
        old.frequency = "uma vez".to_owned();

        let Decision::Migrate(migration) = triage(&old) else {
            panic!("deveria migrar");
        };

        assert_eq!(migration.title.as_str(), "Comprar ração e areia");
        assert!(migration.title_was_cut);
        assert_eq!(migration.area.as_str(), "casa");
        assert_eq!(migration.fit, Fit::Gap);
        assert_eq!(migration.status, Status::Scheduled);
        assert_eq!(
            migration.notes,
            [
                "Título no banco antigo: Comprar ração e areia para o gato",
                "Status no banco antigo: agendada (sáb de manhã)",
                "Área no banco antigo: casa / família",
                "Encaixe no banco antigo: brecha + foco",
                "Frequência: uma vez",
            ]
        );
    }

    #[test]
    fn finished_items_come_in_archived() {
        let Decision::Migrate(migration) = triage(&item(
            "T-003",
            "Pintar a sala",
            "casa",
            "concluída (02/03)",
            "foco",
        )) else {
            panic!("deveria migrar");
        };
        assert_eq!(migration.status, Status::Done);
        assert!(is_finished(migration.status));
    }

    #[test]
    fn finished_items_that_do_not_convert_stay_in_the_old_file() {
        assert_eq!(
            triage(&item(
                "T-004",
                "Reunião de pais",
                "casa",
                "descartada",
                "foco"
            )),
            Decision::Leave {
                id: "T-004".to_owned(),
                reason: "o título começa com `Reunião`, e no caderno a tarefa começa com verbo no \
                         infinitivo, como Ligar, Comprar ou Pôr"
                    .to_owned()
            }
        );
    }

    #[test]
    fn habits_stay_in_the_old_file() {
        assert!(matches!(
            triage(&item(
                "H-001",
                "Caminhar depois do almoço",
                "saúde",
                "recorrente",
                "brecha"
            )),
            Decision::Leave { .. }
        ));
    }

    #[test]
    fn unclear_items_are_left_for_a_human() {
        let ok = ("T-005", "Ler o livro", "estudos", "pronta", "foco");
        let cases = [
            (("X-1", ok.1, ok.2, ok.3, ok.4), "não é um ID"),
            ((ok.0, ok.1, ok.2, "pronta?", ok.4), "o status `pronta?`"),
            ((ok.0, ok.1, "?", ok.3, ok.4), "a área `?`"),
            (
                (ok.0, ok.1, ok.2, ok.3, "bloco fora das brechas"),
                "o encaixe",
            ),
            (
                (ok.0, "Reunião de pais", ok.2, ok.3, ok.4),
                "começa com `Reunião`",
            ),
        ];
        for ((id, title, area, status, fit), expected) in cases {
            let reason = reason(triage(&item(id, title, area, status, fit)));
            assert!(
                reason.contains(expected),
                "{reason:?} deveria ter {expected:?}"
            );
        }
    }

    #[test]
    fn the_plan_skips_what_is_already_in_the_bank_and_repeated_ids() {
        let items = [
            item("T-001", "Ler o livro", "estudos", "pronta", "foco"),
            item("T-002", "Pagar a luz", "casa", "pronta", "brecha"),
            item("T-002", "Pagar a água", "casa", "pronta", "brecha"),
        ];
        let first = TaskId::new(1).unwrap();

        let plan = Plan::new(&items, |id| id == first);

        assert_eq!(plan.total, 3);
        assert_eq!(plan.already, [first]);
        assert_eq!(plan.migrate.len(), 1);
        assert_eq!(
            plan.decide,
            [(
                "T-002".to_owned(),
                "o ID aparece mais de uma vez no arquivo".to_owned()
            )]
        );
    }

    #[test]
    fn the_status_is_the_first_word() {
        assert_eq!(parse_status("agendada"), Some(Status::Scheduled));
        assert_eq!(parse_status("agendada, em rampa"), Some(Status::Scheduled));
        assert_eq!(parse_status(" concluída (13/10)"), Some(Status::Done));
        assert_eq!(parse_status("pronta?"), None);
        assert_eq!(parse_status("?"), None);
        assert_eq!(parse_status(""), None);
    }

    #[test]
    fn the_fit_is_the_first_one_named() {
        assert_eq!(parse_fit("brecha (saída)"), Some(Fit::Gap));
        assert_eq!(parse_fit("foco / brecha"), Some(Fit::Focus));
        assert_eq!(parse_fit("com-alguém"), Some(Fit::WithSomeone));
        assert_eq!(parse_fit("bloco fora das brechas"), None);
    }
}
