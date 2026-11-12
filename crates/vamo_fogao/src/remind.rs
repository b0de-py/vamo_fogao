//! Os lembretes: o `vamo lembrar`, que um timer do systemd roda de minuto em
//! minuto (`extras/systemd/`), com ou sem a TUI aberta.
//!
//! A conta de quando avisar é do core ([`vamo_core::reminders`]). Aqui fica
//! o resto: a última vez que o timer olhou, guardada num arquivo, o texto
//! do aviso e o envio, pela porta [`Notifier`]. O envio de verdade é o
//! [`NotifySend`]; nos testes, um falso guarda o que recebeu.

use std::io;
use std::path::Path;
use std::process::Command;

use color_eyre::eyre::{Result, WrapErr};
use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use vamo_core::{Appointment, Notifier, Projection, ReminderWindow, reminders};
use vamo_tui::Voice;

/// As notificações do desktop pelo `notify-send`, da libnotify.
///
/// O `notify-send` fala com o serviço de notificações pelo D-Bus, o mesmo
/// que o Omarchy usa. Chamar o programa sai mais barato que trazer um
/// cliente de D-Bus pro binário: o porquê está na ADR 0011.
pub(crate) struct NotifySend;

impl Notifier for NotifySend {
    type Error = io::Error;

    fn notify(&mut self, summary: &str, body: &str) -> io::Result<()> {
        let status = Command::new("notify-send")
            .args(["--app-name=vamo_fogao", "--", summary, body])
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!("o notify-send saiu com {status}")))
        }
    }
}

/// Uma rodada: avisa o que venceu na janela e devolve quantos avisos saíram.
///
/// # Errors
///
/// Quando um aviso não sai. Os outros ainda são tentados; o erro que volta
/// é o primeiro.
pub(crate) fn round<N: Notifier>(
    notifier: &mut N,
    appointments: &[Appointment],
    projection: &Projection,
    lead: SignedDuration,
    window: ReminderWindow,
    voice: Voice,
) -> Result<usize, N::Error> {
    let due = reminders(appointments, projection, lead, window);
    let mut first_error = None;
    for reminder in &due {
        let time = reminder.start.strftime("%H:%M").to_string();
        // Sem antecedência (ou com a máquina acordando atrasada), o aviso
        // chega com o compromisso já começado.
        let body = if reminder.start <= window.until {
            voice.reminder_started(&time)
        } else {
            voice.reminder_ahead(&time)
        };
        if let Err(error) = notifier.notify(&reminder.title, &body) {
            tracing::warn!(%error, title = reminder.title, "o aviso não saiu");
            first_error.get_or_insert(error);
        }
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(due.len()),
    }
}

/// A última vez que o timer olhou, no horário local. Sem arquivo (a
/// primeira rodada) ou com o arquivo estragado, `None`: a janela volta só
/// uns minutos.
pub(crate) fn last_round(path: &Path, zone: &TimeZone) -> Option<DateTime> {
    let text = std::fs::read_to_string(path).ok()?;
    let at: Timestamp = text.trim().parse().ok()?;
    Some(at.to_zoned(zone.clone()).datetime())
}

/// Guarda até onde esta rodada olhou. O instante vai sem fuso (UTC), pra
/// uma troca de fuso não fazer o timer pular ou repetir avisos.
pub(crate) fn save_round(path: &Path, now: Timestamp) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .wrap_err_with(|| format!("não deu pra criar a pasta {}", dir.display()))?;
    }
    std::fs::write(path, format!("{now}\n"))
        .wrap_err_with(|| format!("não deu pra gravar {}", path.display()))
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use vamo_core::{DEFAULT_LEAD, When};

    use super::*;

    /// O falso: guarda cada aviso e falha quando mandam.
    #[derive(Default)]
    struct Fake {
        sent: Vec<(String, String)>,
        broken: bool,
    }

    impl Notifier for Fake {
        type Error = io::Error;

        fn notify(&mut self, summary: &str, body: &str) -> io::Result<()> {
            if self.broken {
                return Err(io::Error::other("sem serviço de notificação"));
            }
            self.sent.push((summary.to_owned(), body.to_owned()));
            Ok(())
        }
    }

    fn appointment(title: &str, hour: i8) -> Appointment {
        let day = date(2026, 11, 12);
        Appointment {
            uid: format!("{title}@exemplo"),
            title: title.to_owned(),
            when: When::Timed {
                start: day.at(hour, 0, 0, 0),
                end: day.at(hour + 1, 0, 0, 0),
            },
            recurring: false,
            task: None,
        }
    }

    fn window(hour: i8, minute: i8) -> ReminderWindow {
        let now = date(2026, 11, 12).at(hour, minute, 0, 0);
        ReminderWindow::since(Some(now.saturating_sub(SignedDuration::from_mins(1))), now)
    }

    #[test]
    fn what_is_due_is_sent() {
        let mut fake = Fake::default();
        let agenda = [appointment("Dentista", 15), appointment("Mercado", 18)];
        let sent = round(
            &mut fake,
            &agenda,
            &Projection::new(),
            DEFAULT_LEAD,
            window(14, 50),
            Voice::Game,
        )
        .unwrap();
        assert_eq!(sent, 1);
        assert_eq!(
            fake.sent,
            [("Dentista".to_owned(), "A bola rola às 15:00".to_owned())]
        );
    }

    #[test]
    fn the_neutral_voice_has_no_ball() {
        let mut fake = Fake::default();
        round(
            &mut fake,
            &[appointment("Dentista", 15)],
            &Projection::new(),
            DEFAULT_LEAD,
            window(14, 50),
            Voice::Neutral,
        )
        .unwrap();
        assert_eq!(fake.sent[0].1, "Começa às 15:00");
    }

    #[test]
    fn a_reminder_without_lead_says_it_started() {
        let mut fake = Fake::default();
        round(
            &mut fake,
            &[appointment("Dentista", 15)],
            &Projection::new(),
            SignedDuration::ZERO,
            window(15, 0),
            Voice::Neutral,
        )
        .unwrap();
        assert_eq!(fake.sent[0].1, "Começou às 15:00");
    }

    #[test]
    fn a_broken_notifier_is_an_error() {
        let mut fake = Fake {
            broken: true,
            ..Fake::default()
        };
        let result = round(
            &mut fake,
            &[appointment("Dentista", 15)],
            &Projection::new(),
            DEFAULT_LEAD,
            window(14, 50),
            Voice::Game,
        );
        assert!(result.is_err());
    }

    #[test]
    fn the_last_round_survives_a_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("estado/lembretes");
        let zone = TimeZone::get("America/Sao_Paulo").unwrap();
        assert_eq!(last_round(&path, &zone), None);

        let now: Timestamp = "2026-11-12T17:50:00Z".parse().unwrap();
        save_round(&path, now).unwrap();
        assert_eq!(
            last_round(&path, &zone),
            Some(date(2026, 11, 12).at(14, 50, 0, 0))
        );

        std::fs::write(&path, "não é um instante").unwrap();
        assert_eq!(last_round(&path, &zone), None);
    }
}
