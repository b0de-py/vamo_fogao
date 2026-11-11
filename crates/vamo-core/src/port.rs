//! As portas: o que o core precisa do mundo de fora, escrito como trait.
//!
//! Quem implementa é um adaptador. O core só conhece a trait, então dá pra
//! trocar o adaptador, ou usar um de mentira num teste, sem mexer nas regras.

use crate::Event;

/// Onde o log de eventos fica guardado. O `vamo-store` implementa com SQLite.
pub trait EventStore {
    /// O erro do armazenamento.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Guarda os eventos no fim do log e diz quantos eram novos.
    ///
    /// Evento que já está guardado (mesmo ID) fica de fora, então guardar o
    /// mesmo evento duas vezes não duplica nada. É o que a sincronia precisa
    /// pra juntar logs sem se preocupar com o que o outro lado já tem.
    ///
    /// # Errors
    ///
    /// Quando o armazenamento falha. Aí nenhum dos eventos entra: ou entram
    /// todos, ou nenhum.
    fn append(&mut self, events: &[Event]) -> Result<usize, Self::Error>;

    /// Todos os eventos, na ordem em que foram guardados.
    ///
    /// # Errors
    ///
    /// Quando o armazenamento falha ou devolve algo que não é um evento
    /// válido.
    fn events(&self) -> Result<Vec<Event>, Self::Error>;
}
