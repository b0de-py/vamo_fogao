//! A paleta de comandos: uma caixa que abre com `:`, com busca difusa pelas
//! ações da tela aberta e pelas outras telas.
//!
//! Cada comando da paleta é uma tecla da tela com o nome dela por extenso.
//! Escolher um comando é apertar a tecla: o app fecha a paleta e manda a
//! tecla pro mesmo caminho de sempre. Assim a paleta não tem regra própria,
//! e uma ação nova da tela só precisa entrar na lista da [`Voice`].

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::Voice;
use crate::app::Tab;

/// Um comando da paleta: a tecla que ele aperta e o que ela faz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Command {
    /// A tecla que o comando aperta.
    pub(crate) key: KeyCode,
    /// O que a tecla faz, por extenso. É onde a busca procura.
    pub(crate) what: String,
}

impl Command {
    /// O nome da tecla, como aparece na ajuda.
    pub(crate) fn key_label(&self) -> String {
        match self.key {
            KeyCode::Char(' ') => "espaço".to_owned(),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Enter => "enter".to_owned(),
            KeyCode::Esc => "esc".to_owned(),
            KeyCode::Left => "←".to_owned(),
            KeyCode::Right => "→".to_owned(),
            other => format!("{other:?}").to_lowercase(),
        }
    }
}

/// Os comandos que valem na tela aberta: os dela, e depois os que valem em
/// toda tela (ir pras outras, a ajuda, sair).
pub(crate) fn commands(voice: Voice, tab: Tab, card: bool) -> Vec<Command> {
    let mut commands: Vec<Command> = voice
        .commands(tab, card)
        .into_iter()
        .map(|(key, what)| Command {
            key,
            what: what.to_owned(),
        })
        .collect();
    for (digit, other) in ('1'..='4').zip(Tab::ALL) {
        if other != tab || card {
            commands.push(Command {
                key: KeyCode::Char(digit),
                what: voice.go_to_tab(other),
            });
        }
    }
    commands.extend([
        Command {
            key: KeyCode::Char('?'),
            what: voice.open_help().to_owned(),
        },
        Command {
            key: KeyCode::Char('q'),
            what: voice.quit().to_owned(),
        },
    ]);
    commands
}

/// A paleta aberta: o que já foi digitado e o comando escolhido.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Palette {
    /// A busca.
    pub(crate) query: String,
    /// A posição do escolhido na lista filtrada.
    pub(crate) selected: usize,
}

/// O que a tecla fez com a paleta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Continua aberta.
    Open,
    /// Esc: fecha sem fazer nada.
    Close,
    /// Enter: fecha e aperta a tecla do comando escolhido.
    Run(KeyCode),
}

impl Palette {
    /// Reage a uma tecla, com a lista de comandos da tela.
    pub(crate) fn on_key(&mut self, key: KeyEvent, commands: &[Command]) -> Outcome {
        let found = search(&self.query, commands);
        match key.code {
            KeyCode::Esc => Outcome::Close,
            KeyCode::Enter => match found.get(self.selected) {
                Some(command) => Outcome::Run(command.key),
                // Sem nada achado, o enter não faz nada: a busca continua.
                None => Outcome::Open,
            },
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                Outcome::Open
            }
            KeyCode::Down => {
                if self.selected + 1 < found.len() {
                    self.selected += 1;
                }
                Outcome::Open
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.selected = 0;
                Outcome::Open
            }
            KeyCode::Char(c) => {
                self.query.push(c);
                self.selected = 0;
                Outcome::Open
            }
            _ => Outcome::Open,
        }
    }
}

/// Os comandos que casam com a busca, do melhor pro pior. Busca vazia
/// devolve todos, na ordem da lista.
///
/// A busca é difusa: as letras dela precisam aparecer no nome do comando
/// na mesma ordem, mas não precisam estar juntas (`tnv` acha "tarefa
/// nova"). Maiúscula e acento não contam. Entre os que casam, ganha o que
/// tem as letras mais juntas e, empatando, o que casa mais no começo.
pub(crate) fn search<'a>(query: &str, commands: &'a [Command]) -> Vec<&'a Command> {
    let query = fold(query.trim());
    let mut found: Vec<(Score, usize, &Command)> = commands
        .iter()
        .enumerate()
        .filter_map(|(index, command)| {
            score(&query, &fold(&command.what)).map(|score| (score, index, command))
        })
        .collect();
    // O índice desempata, pra lista não embaralhar entre dois iguais.
    found.sort_by_key(|&(score, index, _)| (score, index));
    found.into_iter().map(|(_, _, command)| command).collect()
}

/// A nota de um casamento: menos é melhor. Primeiro os buracos entre as
/// letras achadas, depois onde a primeira caiu.
type Score = (usize, usize);

/// Onde as letras de `query` aparecem em `text`, na ordem, do jeito mais
/// cedo possível. `None` quando alguma não aparece.
fn score(query: &[char], text: &[char]) -> Option<Score> {
    if query.is_empty() {
        return Some((0, 0));
    }
    let mut positions = Vec::with_capacity(query.len());
    let mut from = 0;
    for &wanted in query {
        let at = from + text.get(from..)?.iter().position(|&c| c == wanted)?;
        positions.push(at);
        from = at + 1;
    }
    let gaps = positions.windows(2).map(|pair| pair[1] - pair[0] - 1).sum();
    Some((gaps, positions[0]))
}

/// O texto em minúsculas e sem acento, letra por letra, pra busca não
/// depender de como se digitou.
fn fold(text: &str) -> Vec<char> {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'ô' | 'õ' => 'o',
            'ú' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn command(what: &str) -> Command {
        Command {
            key: KeyCode::Char('x'),
            what: what.to_owned(),
        }
    }

    fn names<'a>(found: &[&'a Command]) -> Vec<&'a str> {
        found.iter().map(|command| command.what.as_str()).collect()
    }

    #[test]
    fn an_empty_search_keeps_the_whole_list_in_order() {
        let list = [command("tarefa nova"), command("abre a ficha")];
        assert_eq!(names(&search("", &list)), ["tarefa nova", "abre a ficha"]);
    }

    #[test]
    fn the_letters_may_be_apart_but_in_order() {
        let list = [command("tarefa nova"), command("abre a ficha")];
        assert_eq!(names(&search("tnv", &list)), ["tarefa nova"]);
        assert_eq!(search("vnt", &list).len(), 0);
    }

    #[test]
    fn case_and_accents_do_not_matter() {
        let list = [command("edita o título"), command("vai pro Museu")];
        assert_eq!(names(&search("TITULO", &list)), ["edita o título"]);
        assert_eq!(names(&search("museu", &list)), ["vai pro Museu"]);
    }

    #[test]
    fn letters_together_win_over_letters_apart() {
        let list = [command("filtra a lista"), command("tarefa nova")];
        // "ta" está junta na segunda e separada por uma letra na primeira.
        assert_eq!(
            names(&search("ta", &list)),
            ["tarefa nova", "filtra a lista"]
        );
    }

    #[test]
    fn enter_runs_the_chosen_and_esc_closes() {
        let list = [command("tarefa nova"), command("abre a ficha")];
        let key = |code| KeyEvent::from(code);
        let mut palette = Palette::default();
        assert_eq!(palette.on_key(key(KeyCode::Down), &list), Outcome::Open);
        assert_eq!(palette.selected, 1);
        // Não passa do fim da lista.
        palette.on_key(key(KeyCode::Down), &list);
        assert_eq!(palette.selected, 1);
        assert_eq!(
            palette.on_key(key(KeyCode::Enter), &list),
            Outcome::Run(KeyCode::Char('x'))
        );
        assert_eq!(palette.on_key(key(KeyCode::Esc), &list), Outcome::Close);
    }

    #[test]
    fn typing_goes_back_to_the_first() {
        let list = [command("tarefa nova"), command("abre a ficha")];
        let mut palette = Palette {
            query: String::new(),
            selected: 1,
        };
        palette.on_key(KeyEvent::from(KeyCode::Char('a')), &list);
        assert_eq!(palette.query, "a");
        assert_eq!(palette.selected, 0);
        // Enter sem nada achado não roda nada.
        palette.on_key(KeyEvent::from(KeyCode::Char('z')), &list);
        palette.on_key(KeyEvent::from(KeyCode::Char('z')), &list);
        assert_eq!(
            palette.on_key(KeyEvent::from(KeyCode::Enter), &list),
            Outcome::Open
        );
    }

    proptest! {
        /// Todo comando se acha pelo próprio nome, e pelo nome com outra
        /// caixa.
        #[test]
        fn every_command_finds_itself(what in "[a-zà-ú ]{1,30}") {
            let list = [command(&what)];
            prop_assert_eq!(search(&what, &list).len(), 1);
            prop_assert_eq!(search(&what.to_uppercase(), &list).len(), 1);
        }

        /// O que a busca acha é sempre parte da lista, sem repetir.
        #[test]
        fn the_search_never_invents(query in "[a-z]{0,5}") {
            let list = [command("tarefa nova"), command("abre a ficha"), command("sai")];
            let found = search(&query, &list);
            prop_assert!(found.len() <= list.len());
        }
    }
}
