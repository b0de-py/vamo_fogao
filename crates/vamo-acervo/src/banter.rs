//! A Resenha: a zoeira do acervo, com selo próprio.

use crate::document::{self, Table};
use crate::{Error, ShortText, Slug};

/// Um verbete da Resenha, a zoeira do acervo. Fica em `acervo/resenha/`,
/// aparece com selo de zoeira e nunca ocupa a linha da curiosidade do dia.
///
/// Zoeira não tem fonte, então a régua é outra: tem que ser absurda de cara,
/// pra ninguém confundir com fato. Isso o validador não enxerga; fica com
/// quem revisa o PR.
///
/// ```
/// use vamo_acervo::Banter;
///
/// let toml = r#"texto = "Em 4000 a.C., o time dos velociraptores levou 3 x 0 do Glorioso.""#;
/// let banter = Banter::from_toml("velociraptores", toml)?;
/// assert!(banter.text().as_str().starts_with("Em 4000 a.C."));
/// # Ok::<(), vamo_acervo::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Banter {
    id: Slug,
    text: ShortText,
    notes: String,
}

impl Banter {
    /// Os campos de um verbete da Resenha.
    const FIELDS: &[&str] = &["texto", "notas"];

    /// Lê um verbete da Resenha do TOML do arquivo.
    ///
    /// A `id` é o nome do arquivo, sem o `.toml`.
    ///
    /// # Errors
    ///
    /// O primeiro problema que aparece, com a linha quando dá pra saber:
    /// TOML que não abre, campo desconhecido, texto faltando ou fora do
    /// tamanho.
    pub fn from_toml(id: &str, toml: &str) -> Result<Self, Error> {
        let id: Slug = id.parse()?;
        let root = document::parse(toml)?;
        let table = Table::root(toml, root.get_ref());
        table.only(Self::FIELDS)?;

        let text = table.required_string("texto")?.parse()?;
        let notes = table
            .string("notas")?
            .map(|notes| notes.value.trim().to_owned())
            .unwrap_or_default();
        Ok(Self { id, text, notes })
    }

    /// A id: o nome do arquivo, sem o `.toml`.
    pub fn id(&self) -> &Slug {
        &self.id
    }

    /// A zoeira, numa linha.
    pub fn text(&self) -> &ShortText {
        &self.text
    }

    /// As notas de quem escreveu. Vazio quando não tem.
    pub fn notes(&self) -> &str {
        &self.notes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banter_has_no_sources_and_no_dates() {
        let toml = "texto = \"Uma zoeira.\"\ndata = \"4000\"";
        assert_eq!(
            Banter::from_toml("zoeira", toml),
            Err(Error::at(
                2,
                Error::UnknownField {
                    field: "data".to_owned(),
                    known: Banter::FIELDS,
                }
            ))
        );
    }

    #[test]
    fn banter_needs_text() {
        assert_eq!(
            Banter::from_toml("zoeira", "notas = \"só nota\""),
            Err(Error::MissingField { field: "texto" })
        );
    }
}
