//! Os comandos do `vamo`. Os de tarefa abrem o banco, pedem a regra pro
//! core e devolvem o texto pra mostrar; quem imprime é o `main`.

use std::path::Path;

use color_eyre::eyre::{Result, WrapErr, bail, eyre};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use uuid::Uuid;
use vamo_agenda::Freshness;
use vamo_core::{
    Area, Change, Event, EventId, EventStore, Fit, Machine, NotebookTitle, Projection, Skipped,
    Status, Task, TaskId,
};
use vamo_store::Store;

use crate::cli::{AcervoCommand, Command};
use crate::config::{AgendaConfig, Config};
use crate::import::{self, Plan};
use crate::paths;
use crate::{acervo, agenda};
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
            agenda::show(&config.agenda, day)
        }
        Command::Sync => Session::open(voice)?.sync(&config.agenda),
        Command::Tui => {
            let mut session = Session::open(voice)?;
            let acervo::Bundled { facts, banter } = acervo::bundled();
            let setup = vamo_tui::Setup {
                voice,
                acervo: vamo_tui::Acervo { facts, banter },
                time_zone: TimeZone::system(),
                animation: config.animation.unwrap_or_default(),
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

/// O banco aberto, o log lido e o estado das tarefas calculado a partir dele.
struct Session {
    store: Store,
    events: Vec<Event>,
    projection: Projection,
    voice: Voice,
}

impl Session {
    fn open(voice: Voice) -> Result<Self> {
        let path = paths::database()?;
        tracing::debug!(path = %path.display(), "abrindo o banco");
        let store = Store::open(&path)
            .wrap_err_with(|| format!("não deu pra abrir o banco em {}", path.display()))?;
        let events = store.events()?;
        let projection = events.iter().collect();
        Ok(Self {
            store,
            events,
            projection,
            voice,
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

    /// `vamo sync`: baixa a agenda e liga cada evento com rodapé `vamo:` à
    /// tarefa dele, que fica agendada no dia do evento.
    fn sync(&mut self, config: &AgendaConfig) -> Result<String> {
        let loaded = agenda::load(config)?.ok_or_else(agenda::not_configured)?;
        let today = jiff::Zoned::now().date();
        let (from, to) = agenda::sync_window(today)?;
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
        lines.push(self.voice.synced(appointments.len(), changed));
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
