//! O dicionário: todo texto de interface do `vamo`, na linguagem de jogo ou
//! na neutra.
//!
//! A TUI e a CLI escrevem pela mesma [`Voice`], então trocar de linguagem é
//! trocar a `Voice`, sem caçar string pelo código.

use std::fmt;
use std::str::FromStr;

use jiff::civil::Weekday;
use vamo_acervo::{Banter, Curiosity, Tag};
use vamo_core::{Change, NotebookTitle, TaskId};

use crate::Animation;
use crate::app::Tab;
use crate::museum::Shelf;
use crate::today::Group;

/// Um campo de tarefa, pros rótulos da Ficha, do Banco e da linha de
/// digitar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Field {
    Id,
    Title,
    Area,
    Fit,
    Status,
    Day,
    Notes,
}

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

impl Voice {
    /// As duas linguagens, na ordem da ajuda.
    pub const ALL: [Self; 2] = [Self::Game, Self::Neutral];

    /// O nome da linguagem na linha de comando e na config.
    pub fn name(self) -> &'static str {
        match self {
            Self::Game => "jogo",
            Self::Neutral => "neutra",
        }
    }
}

impl fmt::Display for Voice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// O nome que não é de nenhuma linguagem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownVoice(pub String);

impl fmt::Display for UnknownVoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` não é uma linguagem: as linguagens são jogo e neutra",
            self.0
        )
    }
}

impl std::error::Error for UnknownVoice {}

impl FromStr for Voice {
    type Err = UnknownVoice;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|voice| voice.name() == s)
            .ok_or_else(|| UnknownVoice(s.to_owned()))
    }
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
    pub fn score(self, done: usize, planned: usize) -> String {
        match self {
            Self::Game => format!("Placar: {done} de {planned}"),
            Self::Neutral => format!("Feitas: {done} de {planned}"),
        }
    }

    /// O placar curto da barra: `⚽ 2 x 3` (duas feitas, três em aberto) na
    /// de jogo, `2/5` na neutra.
    pub fn bar_score(self, done: usize, planned: usize) -> String {
        match self {
            Self::Game => format!("⚽ {done} x {}", planned.saturating_sub(done)),
            Self::Neutral => format!("{done}/{planned}"),
        }
    }

    /// Quantas tarefas estão atrasadas, na dica da barra.
    pub fn late_count(self, count: usize) -> String {
        match (self, count) {
            (_, 1) => "1 atrasada".to_owned(),
            (_, count) => format!("{count} atrasadas"),
        }
    }

    /// O próximo compromisso da agenda, na dica da barra.
    pub fn next_appointment(self, time: &str, title: &str) -> String {
        format!("Próximo: {time} {title}")
    }

    /// O título da lista do que falta no dia, na dica da barra.
    pub fn still_open(self) -> &'static str {
        match self {
            Self::Game => "Em campo:",
            Self::Neutral => "Falta:",
        }
    }

    /// O texto do lembrete de um compromisso que ainda vai começar.
    pub fn reminder_ahead(self, time: &str) -> String {
        match self {
            Self::Game => format!("A bola rola às {time}"),
            Self::Neutral => format!("Começa às {time}"),
        }
    }

    /// O texto do lembrete de um compromisso que já começou.
    pub fn reminder_started(self, time: &str) -> String {
        match self {
            Self::Game => format!("A bola rolou às {time}"),
            Self::Neutral => format!("Começou às {time}"),
        }
    }

    /// Quantos lembretes uma rodada do `vamo lembrar` mandou.
    pub fn reminders_sent(self, count: usize) -> String {
        match count {
            1 => "1 lembrete enviado".to_owned(),
            count => format!("{count} lembretes enviados"),
        }
    }

    /// A notificação de teste do `vamo lembrar --teste`: o título e o texto.
    pub fn test_reminder(self) -> (&'static str, &'static str) {
        match self {
            Self::Game => (
                "vamo_fogao",
                "Teste de lembrete: se você está lendo isto, o aviso chega na hora do jogo.",
            ),
            Self::Neutral => (
                "vamo_fogao",
                "Teste de lembrete: se você está lendo isto, os avisos estão funcionando.",
            ),
        }
    }

    /// O título de um grupo da escalação.
    pub(crate) fn group(self, group: Group) -> &'static str {
        match (self, group) {
            (Self::Game, Group::Planned) => "Escalação",
            (Self::Neutral, Group::Planned) => "Pra hoje",
            (_, Group::Late) => "Atrasadas",
            (_, Group::Fixed) => "Fixas",
            (_, Group::Agenda) => "Fixas da agenda",
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

    /// O título do `vamo caixa`.
    pub fn inbox(self, count: usize) -> String {
        let tasks = if count == 1 { "tarefa" } else { "tarefas" };
        match self {
            Self::Game => format!(
                "A marcar: {count} {tasks} no aquecimento, esperando horário. Cole o rodapé na descrição do evento."
            ),
            Self::Neutral => format!(
                "A marcar: {count} {tasks} esperando horário. Cole o rodapé na descrição do evento."
            ),
        }
    }

    /// O `vamo caixa` vazio.
    pub fn empty_inbox(self) -> &'static str {
        match self {
            Self::Game => "Ninguém no aquecimento: toda tarefa pronta já tem dia.",
            Self::Neutral => "Nada a marcar: toda tarefa pronta já tem dia.",
        }
    }

    /// O título das brechas do dia na tela Hoje.
    pub(crate) fn gaps(self) -> &'static str {
        match self {
            Self::Game => "Contra-ataques (brechas)",
            Self::Neutral => "Brechas",
        }
    }

    /// Quando o expediente de hoje não tem brecha nenhuma.
    pub(crate) fn no_gaps(self) -> &'static str {
        match self {
            Self::Game => "Sem espaço pra contra-atacar hoje.",
            Self::Neutral => "Nenhuma brecha hoje.",
        }
    }

    /// O aviso do gol: depois do `vamo done` e no rodapé da TUI.
    pub fn goal(self, id: TaskId, title: &NotebookTitle) -> String {
        match self {
            Self::Game => format!("GOOOL! {id} {title}"),
            Self::Neutral => format!("Concluída: {id} {title}"),
        }
    }

    /// O título do Banco.
    /// O grito que pisca por cima do fogo quando uma tarefa é concluída.
    pub(crate) fn goal_shout(self) -> &'static str {
        self.pick(" G O O O L ! ", " C O N C L U Í D A ")
    }

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

    /// O título do Museu.
    pub(crate) fn museum_title(self) -> &'static str {
        "Museu"
    }

    /// O selo do fato que aconteceu neste dia.
    pub(crate) fn on_this_day_badge(self) -> &'static str {
        "Neste dia"
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

    /// Depois de criar uma tarefa, no `vamo add` e na TUI.
    pub fn added(self, id: TaskId, title: &NotebookTitle) -> String {
        match self {
            Self::Game => format!("{id} entrou no banco: {title}"),
            Self::Neutral => format!("Tarefa criada: {id} {title}"),
        }
    }

    /// O título do histórico de uma tarefa, no `vamo show` e na Ficha.
    pub fn history(self) -> &'static str {
        match self {
            Self::Game => "Súmula",
            Self::Neutral => "Histórico",
        }
    }

    /// O que aconteceu num evento, numa linha do histórico.
    pub fn change(self, change: &Change) -> String {
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

    /// O resumo do `vamo sync`: quantos compromissos a agenda tem na janela
    /// e quantas tarefas mudaram por causa do rodapé `vamo:`.
    pub fn synced(self, appointments: usize, changed: usize) -> String {
        let appointments = match appointments {
            1 => "1 compromisso".to_owned(),
            n => format!("{n} compromissos"),
        };
        let moved = match (self, changed) {
            (Self::Game, 1) => "1 tarefa escalada".to_owned(),
            (Self::Game, n) => format!("{n} tarefas escaladas"),
            (Self::Neutral, 1) => "1 tarefa marcada".to_owned(),
            (Self::Neutral, n) => format!("{n} tarefas marcadas"),
        };
        format!("Agenda lida: {appointments}, {moved} pela agenda.")
    }

    /// Quando a lista do `vamo list` não tem nenhuma tarefa.
    pub fn nothing_to_list(self) -> &'static str {
        match self {
            Self::Game => "Ninguém no banco. Pra contratar: vamo add <título>",
            Self::Neutral => "Nenhuma tarefa. Pra criar uma: vamo add <título>",
        }
    }

    /// Depois de `vamo import`, com quantas tarefas entraram.
    pub fn imported(self, count: usize) -> String {
        match (self, count) {
            (Self::Game, 0) => "Nenhum reforço entrou no banco.".to_owned(),
            (Self::Game, 1) => "1 reforço entrou no banco.".to_owned(),
            (Self::Game, n) => format!("{n} reforços entraram no banco."),
            (Self::Neutral, 0) => "Nenhuma tarefa importada.".to_owned(),
            (Self::Neutral, 1) => "1 tarefa importada.".to_owned(),
            (Self::Neutral, n) => format!("{n} tarefas importadas."),
        }
    }

    /// Depois de `vamo acervo validar`, quando o acervo inteiro passa.
    pub fn acervo_checked(self, verified: usize, to_check: usize, banter: usize) -> String {
        let facts = match verified {
            1 => "1 fato verificado".to_owned(),
            n => format!("{n} fatos verificados"),
        };
        match self {
            Self::Game => format!(
                "O VAR conferiu o acervo: {facts}, {to_check} a conferir e {banter} da resenha."
            ),
            Self::Neutral => {
                format!("Acervo validado: {facts}, {to_check} a conferir e {banter} de piada.")
            }
        }
    }

    /// A curiosidade do dia no `vamo curiosidade`, com as fontes embaixo.
    pub fn curiosity(self, curiosity: Curiosity<'_>) -> String {
        let fact = curiosity.fact();
        let heading = match (curiosity, fact.date()) {
            (Curiosity::OnThisDay(_), Some(date)) => self.on_this_day(date.year()),
            _ => self.curiosity_title().to_owned(),
        };
        let sources: Vec<&str> = fact
            .sources()
            .iter()
            .map(vamo_acervo::Source::name)
            .collect();
        format!(
            "{heading}\n\n{}\n\n{}{}",
            fact.text(),
            self.sources(),
            sources.join(" · ")
        )
    }

    /// A resenha do dia no `vamo curiosidade --resenha`, com o selo.
    pub fn banter(self, banter: &Banter) -> String {
        format!("{}\n\n{}", self.banter_seal(), banter.text())
    }

    /// O que vem antes da mensagem de um erro.
    pub fn error(self) -> &'static str {
        match self {
            Self::Game => "Deu ruim:",
            Self::Neutral => "Erro:",
        }
    }

    /// O rótulo de um campo de tarefa.
    pub(crate) fn field(self, field: Field) -> &'static str {
        match field {
            Field::Id => "ID",
            Field::Title => "título",
            Field::Area => "área",
            Field::Fit => "encaixe",
            Field::Status => "status",
            Field::Day => "dia",
            Field::Notes => "notas",
        }
    }

    /// O rótulo do filtro do Banco, com o filtro.
    pub(crate) fn filter(self, filter: &str) -> String {
        format!("filtro: {filter}")
    }

    /// O começo da linha de digitar o filtro.
    pub(crate) fn prompt_filter(self) -> &'static str {
        "filtro (área, status, encaixe ou palavra do título): "
    }

    /// O começo da linha de digitar o dia.
    pub(crate) fn prompt_day(self) -> &'static str {
        "dia (hoje, amanhã, 25/12): "
    }

    /// Quando o dia digitado não é um dia.
    pub(crate) fn bad_day(self, text: &str) -> String {
        format!("`{text}` não é um dia: vale hoje, amanhã, 25/12, 25/12/2026 ou 2026-12-25")
    }

    /// Quando o número digitado não é de nenhuma opção.
    pub(crate) fn bad_option(self, text: &str) -> String {
        format!("`{text}` não é uma das opções: digite o número do status")
    }

    /// O dia da semana.
    pub fn weekday(self, day: Weekday) -> &'static str {
        match day {
            Weekday::Monday => "segunda",
            Weekday::Tuesday => "terça",
            Weekday::Wednesday => "quarta",
            Weekday::Thursday => "quinta",
            Weekday::Friday => "sexta",
            Weekday::Saturday => "sábado",
            Weekday::Sunday => "domingo",
        }
    }

    /// O tipo de um tema do acervo.
    pub(crate) fn tag_kind(self, tag: &Tag) -> &'static str {
        match tag {
            Tag::Era(_) => "época",
            Tag::Idol(_) => "ídolo",
            Tag::Title(_) => "título",
            Tag::Opponent(_) => "adversário",
            Tag::Sport(_) => "esporte",
        }
    }

    /// O grupo dos fatos sem tema.
    pub(crate) fn no_theme(self) -> &'static str {
        "sem tema"
    }

    /// Como o Museu está arrumado, com as teclas pra trocar.
    pub(crate) fn shelf(self, arrangement: Shelf) -> &'static str {
        match (self, arrangement) {
            (Self::Game, Shelf::ByDate) => "por data (o: por tema · z: resenha)",
            (Self::Game, Shelf::ByTheme) => "por tema (o: por data · z: resenha)",
            (Self::Neutral, Shelf::ByDate) => "por data (o: por tema · z: piadas)",
            (Self::Neutral, Shelf::ByTheme) => "por tema (o: por data · z: piadas)",
            (Self::Game, Shelf::Banter) => "resenha (z: volta pros fatos)",
            (Self::Neutral, Shelf::Banter) => "piadas (z: volta pros fatos)",
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
    pub fn no_curiosity(self) -> &'static str {
        match self {
            Self::Game => "O baú está vazio: nenhum fato verificado ainda.",
            Self::Neutral => "Nenhum fato verificado ainda.",
        }
    }

    /// Quando a resenha está vazia.
    pub fn no_banter(self) -> &'static str {
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

    /// Escolhe entre o texto de jogo e o neutro.
    fn pick(self, game: &'static str, neutral: &'static str) -> &'static str {
        match self {
            Self::Game => game,
            Self::Neutral => neutral,
        }
    }

    /// As linhas da ajuda: as teclas da tela aberta e as de sempre. Com a
    /// animação ligada, entra também o clique que atiça o fogo.
    pub(crate) fn help(
        self,
        tab: Tab,
        card: bool,
        animation: Animation,
    ) -> Vec<(&'static str, &'static str)> {
        let back_to_bench = self.pick(
            "tira do dia e devolve pro banco",
            "tira do dia e devolve pra lista",
        );
        let task_keys = [
            (
                "espaço",
                self.pick("marca o gol (ou o VAR anula)", "conclui (ou desfaz)"),
            ),
            ("e", self.pick("escala pra hoje", "marca pra hoje")),
            ("b", back_to_bench),
            ("x", "descarta"),
        ];
        let mut rows: Vec<(&'static str, &'static str)> = match (tab, card) {
            (Tab::Today, _) => vec![
                ("j, k, setas", "sobe e desce na lista"),
                (
                    "espaço, enter",
                    self.pick(
                        "marca o gol (ou anula) e marca a fixa",
                        "conclui (ou desfaz) e marca a fixa",
                    ),
                ),
                ("a", "adia a tarefa pra amanhã"),
                ("b", back_to_bench),
                (
                    "r",
                    self.pick(
                        "troca a curiosidade pela resenha, e volta",
                        "troca a curiosidade pela piada, e volta",
                    ),
                ),
            ],
            (Tab::Notebook, _) => vec![],
            (Tab::Museum, _) => vec![
                ("j, k, setas", "sobe e desce na lista"),
                ("o", "ordena por data ou por tema"),
                (
                    "z",
                    self.pick(
                        "mostra a resenha, e volta pros fatos",
                        "mostra as piadas, e volta pros fatos",
                    ),
                ),
            ],
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
                rows.push(("esc", self.pick("volta pro banco", "volta pra lista")));
                rows
            }
        };
        rows.extend([
            ("1 a 4", "vai direto pra tela"),
            ("tab, shift+tab", "próxima tela, tela anterior"),
            ("?", "abre e fecha esta ajuda"),
            ("q, ctrl+c", self.pick("encerra a partida", "sai")),
        ]);
        if animation == Animation::On {
            rows.push(("clique", "atiça o fogo do fundo"));
        }
        rows
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use ratatui::crossterm::event::KeyCode;

    use super::*;
    use crate::app::tests::{draw, key, new_app};
    use crate::today::tests::example_day;

    #[test]
    fn the_name_goes_and_comes_back() {
        for voice in Voice::ALL {
            assert_eq!(voice.name().parse(), Ok(voice));
        }
        assert_eq!(
            "torcida".parse::<Voice>(),
            Err(UnknownVoice("torcida".to_owned()))
        );
    }

    /// Cada tela do app, desenhada na linguagem dada, uma embaixo da outra.
    /// Cada passo é a tecla que leva à tela e o nome dela.
    fn every_screen(voice: Voice) -> String {
        let steps = [
            (None, "Hoje"),
            (Some('r'), "Hoje com a piada"),
            (Some('2'), "Caderno"),
            (Some('3'), "Tarefas"),
            (Some('\n'), "Ficha"),
            (Some('4'), "Museu"),
            (Some('o'), "Museu por tema"),
            (Some('?'), "Ajuda"),
        ];
        let mut ledger = example_day();
        let mut app = new_app(voice);
        let mut screens = Vec::new();
        for (step, name) in steps {
            match step {
                Some('\n') => app.on_key(key(KeyCode::Enter), &mut ledger),
                Some(c) => app.on_key(key(KeyCode::Char(c)), &mut ledger),
                None => {}
            }
            screens.push(format!("── {name}\n{}", draw(&app, &ledger, 100, 14)));
        }
        screens.join("\n")
    }

    #[test]
    fn every_screen_in_the_neutral_voice() {
        let screens = every_screen(Voice::Neutral);
        // Nenhuma palavra do futebol escapa do modo neutro.
        for word in [
            "GOOOL", "GOL", "banco", "Banco", "escala", "Escala", "súmula", "Súmula", "placar",
            "Glorioso", "resenha", "Resenha", "VAR", "contrat",
        ] {
            assert!(
                !screens.contains(word),
                "`{word}` no modo neutro:\n{screens}"
            );
        }
        assert_snapshot!(screens);
    }
}
