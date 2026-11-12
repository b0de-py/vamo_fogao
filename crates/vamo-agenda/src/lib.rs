//! A agenda do `vamo_fogao`.
//!
//! Adaptador que lê a agenda pelo link iCal (ICS), sem login, e entrega ao
//! core os compromissos de cada dia, já com a recorrência resolvida e no
//! horário de quem usa. É a porta [`vamo_core::Agenda`].
//!
//! - [`IcsLink`]: baixa o ICS do link secreto da agenda e guarda uma cópia
//!   local, pra funcionar sem internet.
//! - [`Calendar`]: lê o texto do ICS e responde o que tem em cada dia.
//!
//! O leitor do ICS e da regra de recorrência são feitos aqui, no pedaço que
//! as agendas usam. A decisão está na ADR 0008.

mod calendar;
mod error;
mod ics;
mod link;
mod rrule;

pub use calendar::Calendar;
pub use error::Error;
pub use link::{Freshness, IcsLink};
