//! Os comandos do `vamo`. Os de tarefa abrem o banco, pedem a regra pro
//! core e devolvem o texto pra mostrar; quem imprime é o `main`.

use std::collections::BTreeSet;
use std::path::Path;

use color_eyre::eyre::{Result, WrapErr, bail, eyre};
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use uuid::Uuid;
use vamo_agenda::{Footer, Freshness};
use vamo_core::{
    Area, Change, Event, EventId, EventStore, Fit, Machine, NotebookTitle, Notifier, Projection,
    ReminderWindow, Skipped, Status, Task, TaskId,
};
use vamo_store::Store;

use crate::cli::{AcervoCommand, Command};
use crate::config::Config;
use crate::import::{self, Plan};
use crate::paths;
use crate::{acervo, agenda, bar, games, remind};
use vamo_tui::Voice;

/// Roda um comando e devolve o que mostrar na saída.
pub(crate) fn run(command: Command, voice: Voice, config: &Config) -> Result<String> {
    match command {
        Command::Add {
            title,
            area,
            fit,
            status,
            notes,
        } => Session::open(voice)?.add(&title.join(" "), area, fit, status, notes),
        Command::List { all, area, status } => {
            Ok(Session::open(voice)?.list(all, area.as_ref(), status))
        }
        Command::Done { id } => Session::open(voice)?.done(id),
        Command::Show { id } => Session::open(voice)?.show(id),
        Command::Import { file, dry_run } => Session::open(voice)?.import(&file, dry_run),
        Command::Curiosidade { date, banter } => {
            let date = date.unwrap_or_else(|| jiff::Zoned::now().date());
            acervo::curiosity(date, banter, voice)
        }
        Command::Agenda { date } => {
            let day = date.unwrap_or_else(|| jiff::Zoned::now().date());
            Session::open(voice)?.agenda(config, day)
        }
        Command::Sync => Session::open(voice)?.sync(config),
        Command::Games { date } => {
            games_list(config, date.unwrap_or_else(|| jiff::Zoned::now().date()))
        }
        Command::Inbox => Ok(Session::open(voice)?.inbox()),
        Command::Bar { json } => Ok(Session::open(voice)?.bar(config, json)),
        Command::Remind { test } => Session::open(voice)?.remind(config, test),
        Command::ConnectGoogle => agenda::google::connect(&config.agenda),
        Command::Tui => {
            let mut session = Session::open(voice)?;
            let acervo::Bundled { facts, banter } = acervo::bundled();
            let now = jiff::Zoned::now();
            let mut schedule = agenda::schedule(config, &now);
            schedule.game = games::today(&config.games, &now, voice);
            let setup = vamo_tui::Setup {
                voice,
                acervo: vamo_tui::Acervo { facts, banter },
                time_zone: TimeZone::system(),
                animation: config.animation.unwrap_or_default(),
                schedule,
            };
            vamo_tui::run(&mut session, setup).wrap_err("a TUI não conseguiu usar o terminal")?;
            Ok(String::new())
        }
        // O acervo não mexe nas tarefas, então nem abre o banco.
        Command::Acervo {
            command: AcervoCommand::Validate { folder },
        } => acervo::validate(&folder, voice),
    }
}

/// `vamo jogos`: baixa o calendário de jogos e mostra os dos 60 dias a
/// partir de `today`. Não mexe nas tarefas, então nem abre o banco.
fn games_list(config: &Config, today: Date) -> Result<String> {
    let zone = TimeZone::system();
    let loaded = games::load(&config.games, zone)?.ok_or_else(games::not_configured)?;
    let (found, warnings) = loaded
        .fixtures
        .read(today, today.checked_add(jiff::ToSpan::days(60))?);
    let mut lines: Vec<String> = warnings.iter().map(|w| format!("aviso: {w}")).collect();
    if loaded.freshness == Freshness::Cached {
        lines.push("aviso: os jogos não baixaram agora; esta é a cópia local".to_owned());
    }
    if found.is_empty() {
        lines.push("Nenhum jogo nos próximos 60 dias.".to_owned());
    }
    let team = config.games.team();
    lines.extend(found.iter().map(|game| games::line(game, team)));
    Ok(lines.join("\n"))
}

/// O banco aberto, o log lido e o estado das tarefas calculado a partir dele.
struct Session {
    store: Store,
    events: Vec<Event>,
    projection: Projection,
    voice: Voice,
    /// As atividades fixas da agenda marcadas: a tabela inteira cabe na
    /// memória, e a TUI lê a cada quadro.
    agenda_checks: BTreeSet<(Date, String)>,
}

impl Session {
    fn open(voice: Voice) -> Result<Self> {
        let path = paths::database()?;
        tracing::debug!(path = %path.display(), "abrindo o banco");
        let store = Store::open(&path)
            .wrap_err_with(|| format!("não deu pra abrir o banco em {}", path.display()))?;
        let events = store.events()?;
        let projection = events.iter().collect();
        let agenda_checks = store.agenda_checks()?;
        Ok(Self {
            store,
            events,
            projection,
            voice,
            agenda_checks,
        })
    }

    /// `vamo add`: cria a tarefa com o próximo ID livre.
    fn add(
        &mut self,
        title: &str,
        area: Area,
        fit: Fit,
        status: Status,
        notes: Option<String>,
    ) -> Result<String> {
        let title: NotebookTitle = title.parse()?;
        let id = self
            .projection
            .next_id()
            .ok_or_else(|| eyre!("os números de tarefa acabaram"))?;

        let mut changes = vec![Change::Created {
            title,
            area,
            fit,
            status,
        }];
        if let Some(notes) = notes {
            changes.push(Change::Notes(notes));
        }
        self.record(id, changes)?;

        let task = self.task(id)?;
        Ok(self.voice.added(task.id(), task.title()))
    }

    /// `vamo list`: as tarefas em jogo, ou as do filtro pedido.
    fn list(&self, all: bool, area: Option<&Area>, status: Option<Status>) -> String {
        let tasks: Vec<Task> = self
            .projection
            .tasks()
            .filter(|task| match status {
                Some(status) => task.status() == status,
                None => all || !matches!(task.status(), Status::Done | Status::Dropped),
            })
            .filter(|task| area.is_none_or(|area| task.area() == area))
            .collect();

        if tasks.is_empty() {
            self.voice.nothing_to_list().to_owned()
        } else {
            table(&tasks)
        }
    }

    /// `vamo done`: conclui a tarefa, se a regra de status deixar.
    fn done(&mut self, id: TaskId) -> Result<String> {
        // O gol conta no dia em que saiu.
        let changes = self.projection.finish(id, jiff::Zoned::now().date())?;
        self.record(id, changes)?;
        let task = self.task(id)?;
        Ok(self.voice.goal(task.id(), task.title()))
    }

    /// `vamo caixa`: as tarefas prontas sem dia, cada uma com o encaixe, a
    /// estimativa (se tiver) e o rodapé pra colar na descrição do evento.
    fn inbox(&self) -> String {
        let tasks = self.projection.inbox();
        if tasks.is_empty() {
            return self.voice.empty_inbox().to_owned();
        }
        let mut lines = vec![self.voice.inbox(tasks.len())];
        for task in &tasks {
            let estimate = task
                .estimate()
                .map(|estimate| format!(" · ~{}", agenda::length(estimate)))
                .unwrap_or_default();
            lines.push(String::new());
            lines.push(format!(
                "{}  {} · {} · {}{estimate}",
                task.id(),
                task.title(),
                task.area(),
                task.fit()
            ));
            lines.push(format!("       {}", Footer::of(task)));
        }
        lines.join("\n")
    }

    /// `vamo barra`: o placar do dia, em texto ou no JSON do módulo. A
    /// agenda vem só da cópia local: a barra pergunta a cada meio minuto e
    /// não pode esperar a internet.
    fn bar(&self, config: &Config, json: bool) -> String {
        let now = jiff::Zoned::now();
        let schedule = agenda::schedule(config, &now);
        let game = games::today(&config.games, &now, self.voice);
        let bar = bar::Bar::new(&bar::Day {
            projection: &self.projection,
            appointments: &schedule.appointments,
            fixed: &schedule.fixed,
            checks: &self.agenda_checks,
            game: game.as_ref(),
            team: config.games.team(),
            now: now.datetime(),
        });
        if json {
            bar.json(self.voice).to_string()
        } else {
            bar.text(self.voice)
        }
    }

    /// `vamo lembrar`: uma rodada dos lembretes. Olha da última rodada até
    /// agora, manda o que venceu e guarda até onde olhou. Sem nada pra
    /// avisar, não escreve nada: o timer roda de minuto em minuto, e o
    /// diário do systemd não precisa de uma linha por minuto.
    fn remind(&self, config: &Config, test: bool) -> Result<String> {
        let mut notifier = remind::NotifySend;
        if test {
            let (summary, body) = self.voice.test_reminder();
            notifier
                .notify(summary, body)
                .wrap_err("a notificação de teste não saiu: o notify-send está instalado?")?;
            return Ok(self.voice.reminders_sent(1));
        }
        let zone = TimeZone::system();
        let now = Timestamp::now();
        let path = paths::reminders_state()?;
        let window = ReminderWindow::since(
            remind::last_round(&path, &zone),
            now.to_zoned(zone.clone()).datetime(),
        );
        let lead = config.reminder_lead.unwrap_or(vamo_core::DEFAULT_LEAD);
        // O aviso das 23:55 pode ser de um compromisso de amanhã cedo.
        let last_day = window.until.saturating_add(lead).date();
        let appointments = agenda::cached_appointments(config, zone, window.after.date(), last_day);
        let sent = remind::round(
            &mut notifier,
            &appointments,
            &self.projection,
            lead,
            window,
            self.voice,
        );
        // A rodada fica guardada mesmo com aviso que falhou: repetir a
        // janela mandaria de novo os que saíram.
        remind::save_round(&path, now)?;
        let sent = sent.wrap_err("um lembrete não saiu: o notify-send está instalado?")?;
        Ok(if sent == 0 {
            String::new()
        } else {
            self.voice.reminders_sent(sent)
        })
    }

    /// `vamo agenda`: os compromissos do dia, baixados agora, com o aviso
    /// das tarefas marcadas fora das regras do tempo.
    fn agenda(&self, config: &Config, day: jiff::civil::Date) -> Result<String> {
        let loaded = agenda::load(&config.agenda, day, day)?.ok_or_else(agenda::not_configured)?;
        let (appointments, warnings) = loaded.calendar.read(day, day);
        let mut lines: Vec<String> = warnings.iter().map(|w| format!("aviso: {w}")).collect();
        if loaded.freshness == Freshness::Cached {
            lines.push("aviso: a agenda não baixou agora; esta é a cópia local".to_owned());
        }
        if appointments.is_empty() {
            lines.push(format!("Nada na agenda em {}.", day.strftime("%d/%m")));
        }
        let game = games::on(&config.games, TimeZone::system(), day, self.voice);
        for appointment in &appointments {
            lines.push(agenda::line(appointment, day));
            for violation in
                agenda::violations(appointment, &self.projection, &config.rules, game.as_ref())
            {
                lines.push(format!("             ⚠ {violation}"));
            }
        }
        lines.extend(agenda::gap_lines(
            config,
            &appointments,
            &self.projection,
            day,
        ));
        Ok(lines.join("\n"))
    }

    /// `vamo sync`: baixa a agenda e liga cada evento com rodapé `vamo:` à
    /// tarefa dele, que fica agendada no dia do evento.
    fn sync(&mut self, config: &Config) -> Result<String> {
        let today = jiff::Zoned::now().date();
        let (from, to) = agenda::sync_window(today)?;
        let loaded = agenda::load(&config.agenda, from, to)?.ok_or_else(agenda::not_configured)?;
        let (appointments, warnings) = loaded.calendar.read(from, to);

        let mut lines: Vec<String> = warnings.iter().map(|w| format!("aviso: {w}")).collect();
        if loaded.freshness == Freshness::Cached {
            lines.push("aviso: a agenda não baixou agora; esta é a cópia local".to_owned());
        }
        let plan = self.projection.link(&appointments, today);
        for skipped in &plan.skipped {
            lines.push(match skipped {
                Skipped::Unknown(id) => {
                    format!("aviso: um evento aponta pra {id}, que não existe no banco")
                }
                Skipped::NotReady(id, status) => format!(
                    "aviso: {id} está como {status} e não dá pra agendar: deixe ela pronta antes"
                ),
            });
        }
        let changed = plan.changes.len();
        for (id, changes) in plan.changes {
            let what: Vec<String> = changes.iter().map(|c| self.voice.change(c)).collect();
            self.record(id, changes)?;
            let task = self.task(id)?;
            lines.push(format!("{id} {}: {}", task.title(), what.join(", ")));
        }
        // As tarefas marcadas fora das regras, de hoje em diante, com a
        // regra do dia de jogo (só na linguagem de jogo). A tabela é lida
        // uma vez só, da cópia local.
        let found = match self.voice {
            Voice::Game => games::cached(&config.games, TimeZone::system(), today, to),
            Voice::Neutral => Vec::new(),
        };
        for appointment in appointments.iter().filter(|a| a.day() >= today) {
            let game = games::pick(&found, &config.games, appointment.day());
            for violation in
                agenda::violations(appointment, &self.projection, &config.rules, game.as_ref())
            {
                lines.push(format!(
                    "⚠ {} ({}): {violation}",
                    appointment.title,
                    appointment.day().strftime("%d/%m")
                ));
            }
        }
        lines.push(self.voice.synced(appointments.len(), changed));
        // A tabela de jogos vem junto, mas jogo que não baixou não derruba o
        // sync da agenda.
        match games::load(&config.games, TimeZone::system()) {
            Ok(Some(loaded)) if loaded.freshness == Freshness::Cached => {
                lines.push("aviso: os jogos não baixaram agora; fica a cópia local".to_owned());
            }
            Ok(_) => {}
            Err(error) => lines.push(format!("aviso: os jogos não vieram: {error:#}")),
        }
        Ok(lines.join("\n"))
    }

    /// `vamo show`: a ficha da tarefa e o histórico dela.
    fn show(&self, id: TaskId) -> Result<String> {
        let task = self.task(id)?;
        let mut lines = vec![
            format!("{} {}", task.id(), task.title()),
            String::new(),
            format!("área     {}", task.area()),
            format!("encaixe  {}", task.fit()),
            format!("status   {}", task.status()),
        ];
        // Nota de várias linhas fica alinhada embaixo da primeira.
        let mut notes = task.notes().lines();
        if let Some(first) = notes.next() {
            lines.push(format!("notas    {first}"));
            lines.extend(notes.map(|line| format!("         {line}")));
        }

        lines.push(String::new());
        lines.push(self.voice.history().to_owned());
        // O log está na ordem de gravação; o histórico vai na ordem do
        // tempo, que é a mesma regra da projeção.
        let mut history: Vec<&Event> = self.events.iter().filter(|e| e.task == id).collect();
        history.sort_by_key(|event| (event.at, event.id));
        let zone = TimeZone::system();
        for event in history {
            let when = event.at.to_zoned(zone.clone()).strftime("%d/%m/%Y %H:%M");
            lines.push(format!(
                "{when}  {}  {}",
                event.machine,
                self.voice.change(&event.change)
            ));
        }
        Ok(lines.join("\n"))
    }

    /// `vamo import`: a triagem do banco antigo e, fora do ensaio, a
    /// gravação do que migra. Com item pra decidir, nada é gravado.
    fn import(&mut self, path: &Path, dry_run: bool) -> Result<String> {
        let markdown = std::fs::read_to_string(path)
            .wrap_err_with(|| format!("não deu pra ler o banco antigo em {}", path.display()))?;
        let items = import::parse(&markdown)?;
        let plan = Plan::new(&items, |id| self.projection.task(id).is_some());

        if !dry_run {
            if !plan.decide.is_empty() {
                bail!(
                    "ainda tem {} itens pra decidir, e o import só grava quando não sobra nenhum: \
                     veja quais com `vamo import --dry-run {}`",
                    plan.decide.len(),
                    path.display()
                );
            }
            for migration in &plan.migrate {
                let mut changes = vec![Change::Created {
                    title: migration.title.clone(),
                    area: migration.area.clone(),
                    fit: migration.fit,
                    status: migration.status,
                }];
                if !migration.notes.is_empty() {
                    changes.push(Change::Notes(migration.notes.join("\n")));
                }
                self.record(migration.id, changes)?;
            }
        }
        Ok(plan.render(self.voice, dry_run))
    }

    /// A tarefa com esse ID, ou o erro de tarefa que não existe.
    fn task(&self, id: TaskId) -> Result<Task> {
        Ok(self
            .projection
            .task(id)
            .ok_or(vamo_core::Error::UnknownTask { id })?)
    }

    /// Grava as mudanças como eventos (ID novo, o instante de agora e o nome
    /// desta máquina) e aplica na projeção.
    fn record(&mut self, task: TaskId, changes: Vec<Change>) -> Result<()> {
        let machine = machine()?;
        let at = Timestamp::now();
        let events: Vec<Event> = changes
            .into_iter()
            .map(|change| Event {
                id: EventId::new(Uuid::now_v7())
                    .expect("o Uuid::now_v7 sempre gera um UUID da versão 7"),
                at,
                machine: machine.clone(),
                task,
                change,
            })
            .collect();

        let added = self.store.append(&events)?;
        tracing::debug!(added, "eventos gravados");
        for event in &events {
            self.projection.apply(event);
        }
        self.events.extend(events);
        Ok(())
    }
}

/// A TUI lê e grava as tarefas pela mesma sessão da CLI.
impl vamo_tui::Ledger for Session {
    type Error = color_eyre::Report;

    fn projection(&self) -> &Projection {
        &self.projection
    }

    fn events(&self) -> &[Event] {
        &self.events
    }

    fn record(&mut self, task: TaskId, changes: Vec<Change>) -> Result<()> {
        Session::record(self, task, changes)
    }

    fn agenda_checks(&self) -> &BTreeSet<(Date, String)> {
        &self.agenda_checks
    }

    fn check_agenda(&mut self, day: Date, uid: &str, done: bool) -> Result<()> {
        self.store.set_agenda_check(day, uid, done)?;
        if done {
            self.agenda_checks.insert((day, uid.to_owned()));
        } else {
            self.agenda_checks.remove(&(day, uid.to_owned()));
        }
        Ok(())
    }
}

/// As tarefas em colunas: ID, status, encaixe, área e título.
fn table(tasks: &[Task]) -> String {
    let rows: Vec<[String; 5]> = tasks
        .iter()
        .map(|task| {
            [
                task.id().to_string(),
                task.status().to_string(),
                task.fit().to_string(),
                task.area().to_string(),
                task.title().to_string(),
            ]
        })
        .collect();

    // A largura de cada coluna é a do maior valor nela, contada em
    // caracteres (o "í" de "concluída" ocupa dois bytes, mas um espaço).
    let mut widths = [0; 4];
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }

    let [id_width, status_width, fit_width, area_width] = widths;
    rows.iter()
        .map(|[id, status, fit, area, title]| {
            format!(
                "{id:<id_width$}  {status:<status_width$}  {fit:<fit_width$}  \
                 {area:<area_width$}  {title}"
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// O nome desta máquina pros eventos: a variável `VAMO_MAQUINA`, se tiver,
/// ou o hostname. Um hostname que não serve como nome vira `desconhecida`.
fn machine() -> Result<Machine> {
    match std::env::var("VAMO_MAQUINA") {
        Ok(name) => name
            .parse()
            .wrap_err("o VAMO_MAQUINA não serve como nome de máquina"),
        Err(std::env::VarError::NotPresent) => Ok(hostname()),
        Err(error) => Err(error).wrap_err("o VAMO_MAQUINA não é um texto válido"),
    }
}

fn hostname() -> Machine {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .or_else(|_| std::fs::read_to_string("/etc/hostname"))
        .ok()
        .and_then(|name| name.trim().parse().ok())
        .unwrap_or_else(|| Machine::new("desconhecida").expect("`desconhecida` é um nome válido"))
}
