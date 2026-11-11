//! A linha de digitar, no rodapé: título novo, edição de campo, dia e
//! filtro.

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use vamo_core::{Status, TaskId};

use crate::Voice;

/// Pra que a linha de digitar foi aberta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Purpose {
    /// O título de uma tarefa nova.
    NewTask,
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

    /// O que aparece antes do texto.
    pub(crate) fn label(&self, voice: Voice) -> String {
        match &self.purpose {
            Purpose::NewTask => voice.prompt_new_task().to_owned(),
            Purpose::Filter => "filtro (área, status, encaixe ou palavra do título): ".to_owned(),
            Purpose::Title(_) => "título: ".to_owned(),
            Purpose::Area(_) => "área: ".to_owned(),
            Purpose::Notes(_) => "notas: ".to_owned(),
            Purpose::Day(_) => "dia (hoje, amanhã, 25/12): ".to_owned(),
            Purpose::Status(_, options) => {
                let options: Vec<String> = (1..)
                    .zip(options)
                    .map(|(n, status)| format!("{n} {status}"))
                    .collect();
                format!("status ({}): ", options.join(", "))
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
