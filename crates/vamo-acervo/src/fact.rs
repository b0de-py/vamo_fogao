//! O fato do acervo e a régua que ele precisa passar.

use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use crate::document::{self, Located, Table};
use crate::{Error, FactDate, ShortText, Slug, Source, SourceUrl, Tag};

/// Um fato do acervo, já conferido pela régua.
///
/// Só nasce pelo [`Fact::from_toml`], então todo `Fact` que existe passou
/// na régua: fato verificado tem duas fontes independentes, recorde tem
/// data, e assim por diante.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    id: Slug,
    text: ShortText,
    date: Option<FactDate>,
    until: Option<FactDate>,
    kind: Kind,
    verification: Verification,
    tags: Vec<Tag>,
    sources: Vec<Source>,
    notes: String,
}

impl Fact {
    /// Os campos de um fato.
    const FIELDS: &[&str] = &[
        "texto", "data", "ate", "tipo", "status", "tags", "notas", "fontes",
    ];

    /// Os campos de cada `[[fontes]]`.
    const SOURCE_FIELDS: &[&str] = &["nome", "url"];

    /// Lê um fato do TOML do arquivo e confere a régua.
    ///
    /// A `id` é o nome do arquivo, sem o `.toml`.
    ///
    /// ```
    /// use vamo_acervo::{Error, Fact};
    ///
    /// let toml = r#"
    /// texto = "Um recorde qualquer."
    /// tipo = "recorde"
    /// status = "a-conferir"
    ///
    /// [[fontes]]
    /// nome = "Lance!"
    /// url = "https://www.lance.com.br/botafogo"
    /// "#;
    /// assert_eq!(Fact::from_toml("um-recorde", toml), Err(Error::RecordWithoutDate));
    /// ```
    ///
    /// # Errors
    ///
    /// O primeiro problema que a régua acha, com a linha quando dá pra saber
    /// (veja [`Error::line`]): TOML que não abre, campo desconhecido,
    /// faltando ou com valor fora do formato, recorde sem data, fato sem
    /// fonte e fato verificado sem duas fontes independentes.
    pub fn from_toml(id: &str, toml: &str) -> Result<Self, Error> {
        let id: Slug = id.parse()?;
        let root = document::parse(toml)?;
        let table = Table::root(toml, root.get_ref());
        table.only(Self::FIELDS)?;

        let text = table.required_string("texto")?.parse()?;
        let date = optional(table.string("data")?)?;
        // O `ate` guarda a linha pra regra do período, lá embaixo.
        let until = table
            .string("ate")?
            .map(|field| field.parse().map(|until: FactDate| (until, field.line)))
            .transpose()?;
        let kind = optional(table.string("tipo")?)?.unwrap_or_default();
        let verification = table.required_string("status")?.parse()?;
        let tags = table
            .strings("tags")?
            .iter()
            .map(Located::parse)
            .collect::<Result<Vec<Tag>, _>>()?;
        let notes = table
            .string("notas")?
            .map(|notes| notes.value.trim().to_owned())
            .unwrap_or_default();
        let sources = table
            .tables("fontes")?
            .iter()
            .map(read_source)
            .collect::<Result<Vec<Source>, _>>()?;

        // As regras que cruzam campos.
        if let Some((until, line)) = until {
            let Some(date) = date else {
                return Err(Error::at(line, Error::UntilWithoutDate));
            };
            if until <= date {
                return Err(Error::at(line, Error::UntilNotAfterDate { date, until }));
            }
        }
        if kind == Kind::Record && date.is_none() {
            return Err(Error::RecordWithoutDate);
        }
        check_sources(verification, &sources)?;

        Ok(Self {
            id,
            text,
            date,
            until: until.map(|(until, _line)| until),
            kind,
            verification,
            tags,
            sources,
            notes,
        })
    }

    /// A id do fato: o nome do arquivo, sem o `.toml`.
    pub fn id(&self) -> &Slug {
        &self.id
    }

    /// O fato, numa linha.
    pub fn text(&self) -> &ShortText {
        &self.text
    }

    /// Quando aconteceu, se as fontes dizem. Num período, é o começo.
    pub fn date(&self) -> Option<FactDate> {
        self.date
    }

    /// O fim do período, quando o fato dura mais que uma data.
    pub fn until(&self) -> Option<FactDate> {
        self.until
    }

    /// Se é um fato comum ou um recorde.
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Se o fato já tem as duas fontes independentes.
    pub fn verification(&self) -> Verification {
        self.verification
    }

    /// Se o fato pode aparecer no app: só o verificado aparece.
    pub fn is_verified(&self) -> bool {
        self.verification == Verification::Verified
    }

    /// As tags, na ordem do arquivo.
    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }

    /// As fontes, na ordem do arquivo.
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }

    /// As notas de quem pesquisou: divergência entre as fontes ou o que
    /// falta pra conferir. Vazio quando não tem.
    pub fn notes(&self) -> &str {
        &self.notes
    }
}

/// O tipo de um fato. No arquivo, o campo `tipo`; sem ele, vale `fato`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Kind {
    /// Um fato comum (`fato`).
    #[default]
    Fact,
    /// Um recorde (`recorde`), que precisa de data, porque recorde cai.
    Record,
}

impl FromStr for Kind {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Error> {
        match input {
            "fato" => Ok(Self::Fact),
            "recorde" => Ok(Self::Record),
            _ => Err(Error::UnknownKind {
                input: input.to_owned(),
            }),
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Fact => "fato",
            Self::Record => "recorde",
        })
    }
}

/// Se o fato já tem as duas fontes independentes. No arquivo, o campo
/// `status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verification {
    /// Tem duas fontes independentes e aparece no app (`verificado`).
    Verified,
    /// Ainda falta fonte, e não aparece no app (`a-conferir`).
    ToCheck,
}

impl FromStr for Verification {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Error> {
        match input {
            "verificado" => Ok(Self::Verified),
            "a-conferir" => Ok(Self::ToCheck),
            _ => Err(Error::UnknownVerification {
                input: input.to_owned(),
            }),
        }
    }
}

impl fmt::Display for Verification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Verified => "verificado",
            Self::ToCheck => "a-conferir",
        })
    }
}

/// Lê um campo opcional com o `FromStr` do tipo.
fn optional<T: FromStr<Err = Error>>(field: Option<Located<&str>>) -> Result<Option<T>, Error> {
    field.as_ref().map(Located::parse).transpose()
}

/// Lê uma `[[fontes]]`.
fn read_source(table: &Table<'_>) -> Result<Source, Error> {
    table.only(Fact::SOURCE_FIELDS)?;
    let name = table.required_string("nome")?;
    let url: Option<SourceUrl> = optional(table.string("url")?)?;
    Source::new(name.value, url).map_err(|error| Error::at(name.line, error))
}

/// A régua das fontes: todo fato tem fonte, e o verificado tem duas de
/// origens diferentes.
fn check_sources(verification: Verification, sources: &[Source]) -> Result<(), Error> {
    if sources.is_empty() {
        return Err(Error::NoSource);
    }
    if verification == Verification::ToCheck {
        return Ok(());
    }
    let origins: BTreeSet<_> = sources.iter().map(Source::origin).collect();
    if origins.len() >= 2 {
        return Ok(());
    }
    if sources.len() == 1 {
        return Err(Error::SingleSource);
    }
    // Mais de uma fonte, todas da mesma origem.
    match origins.into_iter().next() {
        Some(origin) => Err(Error::SameOrigin { origin }),
        None => Err(Error::NoSource),
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// Um fato verificado que passa na régua, pra cada teste mexer num campo.
    const VALID: &str = r#"
texto = "24 x 0 no Mangueira, pelo Campeonato Carioca."
data = "1909-05-30"
tipo = "recorde"
status = "verificado"
tags = ["adversario:mangueira"]
notas = "As fontes divergem sobre o local."

[[fontes]]
nome = "Lance!"
url = "https://www.lance.com.br/lancepedia/maior-goleada-da-historia-do-botafogo.html"

[[fontes]]
nome = "Wikipédia em inglês: Sport Club Mangueira"
url = "https://en.wikipedia.org/wiki/Sport_Club_Mangueira"
"#;

    fn fact(toml: &str) -> Result<Fact, Error> {
        Fact::from_toml("um-fato", toml)
    }

    #[test]
    fn reads_every_field() {
        let fact = fact(VALID).unwrap();
        assert_eq!(fact.id().as_str(), "um-fato");
        assert_eq!(
            fact.text().as_str(),
            "24 x 0 no Mangueira, pelo Campeonato Carioca."
        );
        assert_eq!(fact.date(), Some("1909-05-30".parse().unwrap()));
        assert_eq!(fact.until(), None);
        assert_eq!(fact.kind(), Kind::Record);
        assert!(fact.is_verified());
        assert_eq!(fact.tags(), ["adversario:mangueira".parse().unwrap()]);
        assert_eq!(fact.notes(), "As fontes divergem sobre o local.");
        let names: Vec<&str> = fact.sources().iter().map(Source::name).collect();
        assert_eq!(
            names,
            ["Lance!", "Wikipédia em inglês: Sport Club Mangueira"]
        );
    }

    #[test]
    fn the_optional_fields_can_stay_out() {
        let toml = r#"
texto = "Um fato sem data."
status = "a-conferir"

[[fontes]]
nome = "Ata de fundação do clube"
"#;
        let fact = fact(toml).unwrap();
        assert_eq!(fact.date(), None);
        assert_eq!(fact.kind(), Kind::Fact);
        assert!(!fact.is_verified());
        assert_eq!(fact.tags(), []);
        assert_eq!(fact.notes(), "");
        assert_eq!(fact.sources()[0].url(), None);
    }

    #[test]
    fn a_verified_fact_with_a_single_source_fails() {
        let toml = r#"
texto = "Um fato."
status = "verificado"

[[fontes]]
nome = "Lance!"
url = "https://www.lance.com.br/botafogo"
"#;
        assert_eq!(fact(toml), Err(Error::SingleSource));
    }

    #[test]
    fn two_wikipedias_are_still_one_source() {
        let toml = r#"
texto = "Um fato."
status = "verificado"

[[fontes]]
nome = "Wikipédia em inglês"
url = "https://en.wikipedia.org/wiki/Botafogo_de_Futebol_e_Regatas"

[[fontes]]
nome = "Wikipédia"
url = "https://pt.wikipedia.org/wiki/Botafogo_de_Futebol_e_Regatas"
"#;
        assert!(
            matches!(fact(toml), Err(Error::SameOrigin { origin }) if origin.as_str() == "wikipedia.org")
        );
    }

    #[test]
    fn a_fact_to_check_can_have_a_single_source() {
        let single = VALID
            .replace("verificado", "a-conferir")
            .replace("nome = \"Lance!\"", "nome = \"Wikipédia em português\"")
            .replace(
                "https://www.lance.com.br/lancepedia/maior-goleada-da-historia-do-botafogo.html",
                "https://pt.wikipedia.org/wiki/Sport_Club_Mangueira",
            );
        assert!(fact(&single).is_ok());
    }

    #[test]
    fn every_fact_needs_a_source() {
        let toml = "texto = \"Um fato.\"\nstatus = \"a-conferir\"\n";
        assert_eq!(fact(toml), Err(Error::NoSource));
    }

    #[test]
    fn a_record_needs_a_date() {
        let without_date = VALID.replace("data = \"1909-05-30\"\n", "");
        assert_eq!(fact(&without_date), Err(Error::RecordWithoutDate));
    }

    #[test]
    fn a_period_ends_after_it_starts() {
        let period = VALID.replace("data = \"1909-05-30\"", "data = \"1977\"\nate = \"1978\"");
        let fact_with_period = fact(&period).unwrap();
        assert_eq!(fact_with_period.until(), Some("1978".parse().unwrap()));

        let backwards = VALID.replace("data = \"1909-05-30\"", "data = \"1978\"\nate = \"1977\"");
        assert_eq!(
            fact(&backwards),
            Err(Error::at(
                4,
                Error::UntilNotAfterDate {
                    date: "1978".parse().unwrap(),
                    until: "1977".parse().unwrap(),
                }
            ))
        );

        let without_start = VALID.replace("data = \"1909-05-30\"", "ate = \"1978\"");
        assert_eq!(
            fact(&without_start),
            Err(Error::at(3, Error::UntilWithoutDate))
        );
    }

    #[test]
    fn problems_point_to_their_line() {
        // Cada caso troca um pedaço do fato válido. As linhas contam a
        // primeira, vazia, que vem logo depois do `r#"`.
        let cases = [
            (
                "texto = \"24 x 0 no Mangueira, pelo Campeonato Carioca.\"",
                "texto = 3",
                Error::at(
                    2,
                    Error::WrongType {
                        field: "texto".to_owned(),
                        expected: "um texto entre aspas",
                    },
                ),
            ),
            (
                "data = \"1909-05-30\"",
                "data = \"30/05/1909\"",
                Error::at(
                    3,
                    Error::InvalidDate {
                        input: "30/05/1909".to_owned(),
                    },
                ),
            ),
            (
                "tipo = \"recorde\"",
                "tipo = \"recorde",
                Error::at(
                    4,
                    Error::Syntax {
                        message: "invalid basic string, expected `\"`".to_owned(),
                    },
                ),
            ),
            (
                "tags = [\"adversario:mangueira\"]",
                "tags = [\n  \"adversario:mangueira\",\n  \"jogador:didi\",\n]",
                Error::at(
                    8,
                    Error::InvalidTag {
                        input: "jogador:didi".to_owned(),
                    },
                ),
            ),
            (
                "notas =",
                "nota =",
                Error::at(
                    7,
                    Error::UnknownField {
                        field: "nota".to_owned(),
                        known: Fact::FIELDS,
                    },
                ),
            ),
        ];
        for (from, to, expected) in cases {
            assert_eq!(fact(&VALID.replace(from, to)), Err(expected), "{to:?}");
        }
    }

    #[test]
    fn problems_inside_a_source_point_to_their_line() {
        let unknown = VALID.replace("nome = \"Lance!\"", "titulo = \"Lance!\"");
        assert_eq!(
            fact(&unknown),
            Err(Error::at(
                10,
                Error::UnknownField {
                    field: "titulo".to_owned(),
                    known: Fact::SOURCE_FIELDS,
                }
            ))
        );

        let bad_url = VALID.replace("https://www.lance.com.br", "www.lance.com.br");
        assert_eq!(fact(&bad_url).unwrap_err().line(), Some(11));

        // Sem o `nome`, a falta aponta pra linha onde a fonte começa.
        let without_name = VALID.replace("nome = \"Lance!\"\n", "");
        assert_eq!(
            fact(&without_name),
            Err(Error::at(9, Error::MissingField { field: "nome" }))
        );
    }

    #[test]
    fn a_missing_field_in_the_file_has_no_line() {
        let without_status = VALID.replace("status = \"verificado\"\n", "");
        assert_eq!(
            fact(&without_status),
            Err(Error::MissingField { field: "status" })
        );
    }

    #[test]
    fn the_id_is_a_slug() {
        assert_eq!(
            Fact::from_toml("Um Fato", VALID),
            Err(Error::InvalidSlug {
                input: "Um Fato".to_owned()
            })
        );
    }

    proptest! {
        #[test]
        fn any_text_either_reads_or_fails_without_panicking(toml in "\\PC{0,200}") {
            let _ = fact(&toml);
        }

        #[test]
        fn a_verified_fact_never_passes_with_sources_from_one_domain(
            subdomains in prop::collection::vec("[a-z]{1,8}", 1..4),
        ) {
            let sources: Vec<String> = subdomains
                .iter()
                .map(|subdomain| {
                    format!("[[fontes]]\nnome = \"Lance!\"\nurl = \"https://{subdomain}.lance.com.br/x\"")
                })
                .collect();
            let toml = format!(
                "texto = \"Um fato.\"\nstatus = \"verificado\"\n{}",
                sources.join("\n")
            );
            prop_assert!(fact(&toml).is_err());
        }
    }
}
