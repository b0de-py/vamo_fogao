//! A agenda do `vamo_fogao`.
//!
//! Adaptador que lê a agenda pelo link iCal (ICS), sem login, e entrega ao
//! core os compromissos de cada dia, já com a recorrência resolvida e no
//! horário de quem usa. É a porta [`vamo_core::Agenda`].
//!
//! - [`IcsLink`]: baixa o ICS do link secreto da agenda e guarda uma cópia
//!   local, pra funcionar sem internet.
//! - [`Calendar`]: lê o texto do ICS e responde o que tem em cada dia.
//! - [`IcsFixtures`]: os jogos do time, lidos de uma agenda ICS de jogos.
//!   É a porta [`vamo_core::Fixtures`].
//! - [`Footer`]: o rodapé `vamo:` que liga um evento a uma tarefa:
//!
//! ```text
//! vamo: T-041 | Trocar o filtro da água | casa | brecha
//! ```
//!
//! Com a feature `google` (desligada por padrão), o Google Agenda também
//! entra, pela API e com login: [`GoogleLink`] e [`GoogleCalendar`]. O
//! resto do app não sabe de qual porta a agenda veio.
//!
//! O leitor do ICS e da regra de recorrência são feitos aqui, no pedaço que
//! as agendas usam. A decisão está na ADR 0008.

mod calendar;
mod error;
mod footer;
mod games;
#[cfg(feature = "google")]
mod google;
mod ics;
mod link;
mod rrule;

pub use calendar::Calendar;
pub use error::Error;
pub use footer::Footer;
pub use games::IcsFixtures;
#[cfg(feature = "google")]
pub use google::{GoogleCalendar, GoogleLink, OAuthClient};
pub use link::{Freshness, IcsLink};
