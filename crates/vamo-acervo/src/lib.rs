//! O acervo do `vamo_fogao`: o museu de bolso.
//!
//! Os fatos da história do Botafogo, cada um com as suas fontes, o validador
//! que a CI roda e, a partir da fase 2, o sorteio da curiosidade do dia. Só
//! fato verificado aparece no app; a zoeira mora separada, na Resenha.
//!
//! # Os arquivos
//!
//! Cada verbete é um arquivo TOML: os fatos em `acervo/fatos/` e a zoeira em
//! `acervo/resenha/`. O nome do arquivo, sem o `.toml`, é a id do verbete. Um
//! fato tem esta cara:
//!
//! ```
//! use vamo_acervo::{Fact, Verification};
//!
//! let toml = r#"
//! texto = "Campeão da Libertadores no Monumental: 3 x 1 no Atlético-MG."
//! data = "2024-11-30"
//! status = "verificado"
//! tags = ["titulo:libertadores", "adversario:atletico-mg"]
//!
//! [[fontes]]
//! nome = "Trivela"
//! url = "https://trivela.com.br/america-do-sul/libertadores/"
//!
//! [[fontes]]
//! nome = "ESPN"
//! url = "https://global.espn.com/soccer/"
//! "#;
//!
//! let fact = Fact::from_toml("campeao-da-libertadores-de-2024", toml)?;
//! assert_eq!(fact.verification(), Verification::Verified);
//! # Ok::<(), vamo_acervo::Error>(())
//! ```
//!
//! # A régua
//!
//! [`Fact::from_toml`] só devolve o fato que passa na régua do acervo:
//!
//! - fato `verificado` tem duas fontes independentes, de domínios
//!   diferentes, e a Wikipédia em qualquer idioma conta como uma só;
//! - fato `a-conferir` tem pelo menos uma fonte e não aparece no app;
//! - recorde tem data, porque recorde cai;
//! - o texto cabe numa linha, com até [`ShortText::MAX_CHARS`] caracteres;
//! - campo que o formato não conhece é erro, porque quase sempre é erro de
//!   digitação.
//!
//! A régua não enxerga tudo: duas matérias de veículos diferentes podem
//! contar a mesma coisa copiada uma da outra. Isso fica com quem revisa o PR.
//!
//! Como o core, este crate não lê arquivo nem relógio: recebe o texto de cada
//! arquivo e devolve o verbete ou o erro. Quem lê a pasta é o
//! `vamo acervo validar`.

mod banter;
mod date;
mod document;
mod draw;
mod error;
mod fact;
mod slug;
mod source;
mod tag;
mod text;

pub use banter::Banter;
pub use date::FactDate;
pub use draw::{Curiosity, Day, banter_of_the_day, curiosity_of_the_day};
pub use error::Error;
pub use fact::{Fact, Kind, Verification};
pub use slug::Slug;
pub use source::{Origin, Source, SourceUrl};
pub use tag::Tag;
pub use text::ShortText;
