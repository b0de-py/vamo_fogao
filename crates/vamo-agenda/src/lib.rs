//! A agenda do `vamo_fogao`.
//!
//! Adaptador que lê a agenda pelo link iCal (ICS), sem login, e, como opção,
//! pela API do Google Agenda com OAuth. Os dois ficam atrás da mesma trait.
//! Também lê e escreve o rodapé que liga um evento a uma tarefa:
//!
//! ```text
//! vamo: T-041 | Ligar na fisio | saúde | brecha
//! ```
