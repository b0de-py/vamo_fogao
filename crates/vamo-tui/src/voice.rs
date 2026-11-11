//! Os textos da TUI, na linguagem de jogo ou na neutra.

use vamo_core::{Change, NotebookTitle, TaskId};

use crate::app::Tab;
use crate::today::Group;

/// A linguagem da interface.
///
/// A de jogo é a padrão: banco, escalação, gol, súmula. A neutra troca o
/// vocabulário do futebol por palavras comuns, pra quem não quer o tema.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Voice {
    /// A linguagem de jogo.
    #[default]
    Game,
    /// O modo neutro.
    Neutral,
}

#[allow(
    clippy::unused_self,
    reason = "todo texto passa pela Voice, até o que ainda é igual nas duas linguagens"
)]
impl Voice {
    /// O nome de uma aba.
    pub(crate) fn tab(self, tab: Tab) -> &'static str {
        match (self, tab) {
            (_, Tab::Today) => "Hoje",
            (_, Tab::Notebook) => "Caderno",
            (Self::Game, Tab::Bench) => "Banco",
            (Self::Neutral, Tab::Bench) => "Tarefas",
            (_, Tab::Museum) => "Museu",
        }
    }

    /// Os atalhos de sempre, no rodapé.
    pub(crate) fn hints(self) -> &'static str {
        match self {
            Self::Game => "tab troca de tela · ? ajuda · q encerra a partida",
            Self::Neutral => "tab troca de tela · ? ajuda · q sai",
        }
    }

    /// O título da tela Hoje.
    pub(crate) fn today_title(self) -> &'static str {
        "Hoje"
    }

    /// O placar do dia.
    pub(crate) fn score(self, done: usize, planned: usize) -> String {
        match self {
            Self::Game => format!("Placar: {done} de {planned}"),
            Self::Neutral => format!("Feitas: {done} de {planned}"),
        }
    }

    /// O título de um grupo da escalação.
    pub(crate) fn group(self, group: Group) -> &'static str {
        match (self, group) {
            (Self::Game, Group::Planned) => "Escalação",
            (Self::Neutral, Group::Planned) => "Pra hoje",
            (_, Group::Late) => "Atrasadas",
            (_, Group::Fixed) => "Fixas",
        }
    }

    /// O detalhe de uma tarefa atrasada: o dia pra que ela estava escalada.
    pub(crate) fn was_planned_for(self, day: &str) -> String {
        match self {
            Self::Game => format!("escalada pra {day}"),
            Self::Neutral => format!("era pra {day}"),
        }
    }

    /// Quando não tem nada no dia.
    pub(crate) fn empty_lineup(self) -> &'static str {
        match self {
            Self::Game => "Ninguém escalado hoje. No Banco, o e escala uma tarefa pro dia.",
            Self::Neutral => "Nada pra hoje. Em Tarefas, o e marca uma tarefa pro dia.",
        }
    }

    /// O aviso do gol.
    pub(crate) fn goal(self, title: &str) -> String {
        match self {
            Self::Game => format!("GOOOL! {title}"),
            Self::Neutral => format!("Concluída: {title}"),
        }
    }

    /// O título do Banco.
    pub(crate) fn bench_title(self) -> &'static str {
        match self {
            Self::Game => "Banco",
            Self::Neutral => "Tarefas",
        }
    }

    /// O que o Banco mostra sem filtro.
    pub(crate) fn bench_in_play(self) -> &'static str {
        match self {
            Self::Game => "em jogo (/ filtra, todas traz as encerradas)",
            Self::Neutral => "abertas (/ filtra, todas traz as encerradas)",
        }
    }

    /// Quando nenhuma tarefa passa no filtro.
    pub(crate) fn bench_empty(self) -> &'static str {
        match self {
            Self::Game => "Ninguém no banco com esse filtro. O n contrata uma tarefa nova.",
            Self::Neutral => "Nenhuma tarefa com esse filtro. O n cria uma nova.",
        }
    }

    /// O título do Caderno.
    pub(crate) fn notebook_title(self) -> &'static str {
        "Caderno"
    }

    /// Quando o caderno do dia está vazio.
    pub(crate) fn notebook_empty(self) -> &'static str {
        match self {
            Self::Game => "Página em branco: ninguém escalado hoje.",
            Self::Neutral => "Página em branco: nada pra hoje.",
        }
    }

    /// O começo da linha de digitar o título de uma tarefa nova.
    pub(crate) fn prompt_new_task(self) -> &'static str {
        match self {
            Self::Game => "contratar (verbo na frente, até cinco palavras): ",
            Self::Neutral => "nova tarefa (verbo na frente, até cinco palavras): ",
        }
    }

    /// Depois de criar uma tarefa.
    pub(crate) fn added(self, id: TaskId, title: &NotebookTitle) -> String {
        match self {
            Self::Game => format!("{id} entrou no banco: {title}"),
            Self::Neutral => format!("Tarefa criada: {id} {title}"),
        }
    }

    /// O título do histórico de uma tarefa.
    pub(crate) fn history(self) -> &'static str {
        match self {
            Self::Game => "Súmula",
            Self::Neutral => "Histórico",
        }
    }

    /// O que aconteceu num evento, numa linha do histórico.
    pub(crate) fn change(self, change: &Change) -> String {
        match change {
            Change::Created {
                title,
                area,
                fit,
                status,
            } => {
                let what = match self {
                    Self::Game => "entrou no banco",
                    Self::Neutral => "criada",
                };
                format!("{what}: {title} ({area}, {fit}, {status})")
            }
            Change::Title(title) => format!("título: {title}"),
            Change::Area(area) => format!("área: {area}"),
            Change::Fit(fit) => format!("encaixe: {fit}"),
            Change::Status(status) => format!("status: {status}"),
            Change::Day(Some(day)) => match self {
                Self::Game => format!("escalada pra {}", day.strftime("%d/%m/%Y")),
                Self::Neutral => format!("marcada pra {}", day.strftime("%d/%m/%Y")),
            },
            Change::Day(None) => match self {
                Self::Game => "saiu da escalação".to_owned(),
                Self::Neutral => "sem dia marcado".to_owned(),
            },
            Change::Check { day, done: true } => format!("feita em {}", day.strftime("%d/%m/%Y")),
            Change::Check { day, done: false } => {
                format!("desmarcada em {}", day.strftime("%d/%m/%Y"))
            }
            Change::Notes(notes) if notes.is_empty() => "notas apagadas".to_owned(),
            // No histórico, cada evento ocupa uma linha só.
            Change::Notes(notes) => {
                format!("notas: {}", notes.lines().collect::<Vec<_>>().join(" · "))
            }
        }
    }

    /// O título do quadro da curiosidade quando o fato é o da vez.
    pub(crate) fn curiosity_title(self) -> &'static str {
        match self {
            Self::Game => "Do baú do Glorioso",
            Self::Neutral => "Curiosidade do dia",
        }
    }

    /// O título do quadro quando o fato aconteceu neste dia.
    pub(crate) fn on_this_day(self, year: u16) -> String {
        format!("Neste dia, em {year}")
    }

    /// O começo da linha das fontes.
    pub(crate) fn sources(self) -> &'static str {
        "Fontes: "
    }

    /// O selo da resenha, que avisa que aquilo não é fato.
    pub(crate) fn banter_seal(self) -> &'static str {
        match self {
            Self::Game => "Resenha (é zoeira, não é fato)",
            Self::Neutral => "Piada (não é fato)",
        }
    }

    /// A dica da tecla que mostra a resenha.
    pub(crate) fn banter_hint(self) -> &'static str {
        match self {
            Self::Game => "r: a resenha do dia",
            Self::Neutral => "r: a piada do dia",
        }
    }

    /// A dica da tecla que volta pro fato.
    pub(crate) fn back_to_fact_hint(self) -> &'static str {
        "r: volta pro fato"
    }

    /// Quando o acervo não tem fato verificado.
    pub(crate) fn no_curiosity(self) -> &'static str {
        match self {
            Self::Game => "O baú está vazio: nenhum fato verificado ainda.",
            Self::Neutral => "Nenhum fato verificado ainda.",
        }
    }

    /// Quando a resenha está vazia.
    pub(crate) fn no_banter(self) -> &'static str {
        match self {
            Self::Game => "A resenha está vazia.",
            Self::Neutral => "Nenhuma piada no acervo.",
        }
    }

    /// O título da ajuda.
    pub(crate) fn help_title(self) -> &'static str {
        match self {
            Self::Game => "Prancheta",
            Self::Neutral => "Ajuda",
        }
    }

    /// As linhas da ajuda: as teclas da tela aberta e as de sempre.
    pub(crate) fn help(self, tab: Tab, card: bool) -> Vec<(&'static str, &'static str)> {
        let quit = match self {
            Self::Game => "encerra a partida",
            Self::Neutral => "sai",
        };
        let task_keys = [
            ("espaço", "marca o gol (ou o VAR anula)"),
            ("e", "escala pra hoje"),
            ("b", "tira do dia e devolve pro banco"),
            ("x", "descarta"),
        ];
        let mut rows: Vec<(&'static str, &'static str)> = match (tab, card) {
            (Tab::Today, _) => vec![
                ("j, k, setas", "sobe e desce na lista"),
                ("espaço, enter", "marca o gol (ou anula) e marca a fixa"),
                ("a", "adia a tarefa pra amanhã"),
                ("b", "tira do dia e devolve pro banco"),
                ("r", "troca a curiosidade pela resenha, e volta"),
            ],
            (Tab::Notebook | Tab::Museum, _) => vec![],
            (Tab::Bench, false) => {
                let mut rows = vec![
                    ("j, k, setas", "sobe e desce na lista"),
                    ("enter", "abre a ficha"),
                    ("n", "tarefa nova"),
                    ("/", "filtra: casa, foco, pronta, todas..."),
                    ("esc", "limpa o filtro"),
                ];
                rows.extend(task_keys);
                rows
            }
            (Tab::Bench, true) => {
                let mut rows = vec![
                    ("t, a, n", "edita o título, a área, as notas"),
                    ("f", "troca o encaixe"),
                    ("s", "muda o status"),
                    ("d", "agenda pra um dia"),
                ];
                rows.extend(task_keys);
                rows.push(("esc", "volta pro banco"));
                rows
            }
        };
        rows.extend([
            ("1 a 4", "vai direto pra tela"),
            ("tab, shift+tab", "próxima tela, tela anterior"),
            ("?", "abre e fecha esta ajuda"),
            ("q, ctrl+c", quit),
        ]);
        rows
    }
}
