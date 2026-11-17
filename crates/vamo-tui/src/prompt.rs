//! A linha de digitar, no rodapé: título novo, edição de campo, dia e
//! filtro.

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use vamo_core::{Area, NotebookTitle, Status, TaskId};

use crate::Voice;
use crate::voice::Field;

/// Pra que a linha de digitar foi aberta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Purpose {
    /// O título de uma tarefa nova.
    NewTask,
    /// A área de uma tarefa nova, com o título já aceito. `known` são as
    /// áreas que já existem, da mais usada pra menos, pro tab completar.
    NewArea {
        title: NotebookTitle,
        known: Vec<Area>,
    },
    /// As notas de uma tarefa nova, com o título e a área já aceitos. É o
    /// último passo: o enter cria a tarefa.
    NewNotes { title: NotebookTitle, area: Area },
    /// O filtro do Banco.
    Filter,
    /// O título novo de uma tarefa.
    Title(TaskId),
    /// A área nova de uma tarefa.
    Area(TaskId),
    /// As notas novas de uma tarefa.
    Notes(TaskId),
    /// O dia pra agendar uma tarefa.
    Day(TaskId),
    /// A data em que vence o próximo passo de uma jogada, pedida ao fechar o
    /// passo da vez.
    StepDate(TaskId),
    /// A pronta do banco a escalar pra hoje, pelo ID ou por uma palavra do
    /// título.
    Pull,
    /// O número do item do passo da vez pra marcar ou desmarcar.
    Item(TaskId),
    /// A descartada que volta pro banco, pelo ID.
    Restore,
    /// O status novo, escolhido pelo número entre os que a regra deixa.
    Status(TaskId, Vec<Status>),
}

/// A linha de digitar aberta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Prompt {
    pub(crate) purpose: Purpose,
    pub(crate) text: String,
}

/// O que a tecla fez com a linha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    /// Continua digitando.
    Typing,
    /// Enter: confirma.
    Submit,
    /// Esc: desiste.
    Cancel,
}

impl Prompt {
    /// Abre a linha, já com um texto (o valor atual, quando é edição).
    pub(crate) fn new(purpose: Purpose, text: &str) -> Self {
        Self {
            purpose,
            text: text.to_owned(),
        }
    }

    /// Reage a uma tecla. Backspace apaga um caractere inteiro, mesmo
    /// acentuado.
    pub(crate) fn on_key(&mut self, key: KeyEvent) -> Step {
        match key.code {
            KeyCode::Enter => Step::Submit,
            KeyCode::Tab => {
                if let Some(rest) = self.completion() {
                    self.text.push_str(&rest);
                }
                Step::Typing
            }
            KeyCode::Esc => Step::Cancel,
            KeyCode::Backspace => {
                self.text.pop();
                Step::Typing
            }
            KeyCode::Char(c) => {
                self.text.push(c);
                Step::Typing
            }
            _ => Step::Typing,
        }
    }

    /// O pedaço que falta pro texto virar uma área que já existe, quando a
    /// linha pede a área de uma tarefa nova: com `ca` digitado e a área
    /// `casa` no banco, falta `sa`. Vale a mais usada entre as que começam
    /// assim.
    pub(crate) fn completion(&self) -> Option<String> {
        let Purpose::NewArea { known, .. } = &self.purpose else {
            return None;
        };
        let typed = self.text.trim();
        if typed.is_empty() {
            return None;
        }
        known
            .iter()
            .map(Area::as_str)
            .find(|area| area.starts_with(typed) && *area != typed)
            .map(|area| area[typed.len()..].to_owned())
    }

    /// O que aparece antes do texto.
    pub(crate) fn label(&self, voice: Voice) -> String {
        match &self.purpose {
            Purpose::NewTask => voice.prompt_new_task().to_owned(),
            Purpose::NewArea { .. } => voice.prompt_new_area().to_owned(),
            Purpose::NewNotes { .. } => voice.prompt_new_notes().to_owned(),
            Purpose::Filter => voice.prompt_filter().to_owned(),
            Purpose::Title(_) => format!("{}: ", voice.field(Field::Title)),
            Purpose::Area(_) => format!("{}: ", voice.field(Field::Area)),
            Purpose::Notes(_) => format!("{}: ", voice.field(Field::Notes)),
            Purpose::Pull => voice.prompt_pull().to_owned(),
            Purpose::Item(_) => "item (número): ".to_owned(),
            Purpose::Restore => voice.prompt_restore().to_owned(),
            Purpose::Day(_) | Purpose::StepDate(_) => voice.prompt_day().to_owned(),
            Purpose::Status(_, options) => {
                let options: Vec<String> = (1..)
                    .zip(options)
                    .map(|(n, status)| format!("{n} {status}"))
                    .collect();
                format!("{} ({}): ", voice.field(Field::Status), options.join(", "))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::key;

    #[test]
    fn typing_and_erasing_work_with_accents() {
        let mut prompt = Prompt::new(Purpose::NewTask, "");
        for c in "Pôr a mesá".chars() {
            assert_eq!(prompt.on_key(key(KeyCode::Char(c))), Step::Typing);
        }
        prompt.on_key(key(KeyCode::Backspace));
        assert_eq!(prompt.text, "Pôr a mes");
        assert_eq!(prompt.on_key(key(KeyCode::Enter)), Step::Submit);
        assert_eq!(prompt.on_key(key(KeyCode::Esc)), Step::Cancel);
    }

    #[test]
    fn tab_completes_the_most_used_area() {
        let areas = |names: &[&str]| names.iter().map(|name| name.parse().unwrap()).collect();
        let mut prompt = Prompt::new(
            Purpose::NewArea {
                title: "Lavar o carro".parse().unwrap(),
                known: areas(&["casa", "carreira", "saúde"]),
            },
            "ca",
        );
        assert_eq!(prompt.completion().as_deref(), Some("sa"));
        prompt.on_key(key(KeyCode::Tab));
        assert_eq!(prompt.text, "casa");
        // Completa, nada sobra.
        assert_eq!(prompt.completion(), None);
        // Sem nada digitado, não chuta.
        prompt.text.clear();
        assert_eq!(prompt.completion(), None);
        prompt.on_key(key(KeyCode::Tab));
        assert_eq!(prompt.text, "");
        // Com acento também.
        prompt.text = "sa".to_owned();
        assert_eq!(prompt.completion().as_deref(), Some("úde"));
    }

    #[test]
    fn the_status_label_numbers_the_options() {
        let prompt = Prompt::new(
            Purpose::Status(TaskId::FIRST, vec![Status::Refining, Status::Dropped]),
            "",
        );
        assert_eq!(
            prompt.label(Voice::Game),
            "status (1 destrinchando, 2 descartada): "
        );
    }
}
