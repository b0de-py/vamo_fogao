//! A leitura de um arquivo TOML do acervo: os campos de cada tabela, com a
//! linha de cada um, pra apontar onde está o problema.
//!
//! O TOML é lido campo a campo, sem o serde: assim cada mensagem sai em
//! português e com a linha certa.

use std::str::FromStr;

use toml::Spanned;
use toml::de::{DeTable, DeValue};

use crate::Error;

/// Um valor do arquivo e a linha onde ele está.
#[derive(Clone, Copy)]
pub(crate) struct Located<T> {
    pub(crate) value: T,
    pub(crate) line: usize,
}

impl Located<&str> {
    /// Lê o valor com o `FromStr` do tipo pedido e prende o erro, se houver,
    /// na linha do valor.
    pub(crate) fn parse<T: FromStr<Err = Error>>(&self) -> Result<T, Error> {
        self.value
            .parse()
            .map_err(|error| Error::at(self.line, error))
    }
}

/// Abre o arquivo. Erro de sintaxe vem com a linha.
pub(crate) fn parse(text: &str) -> Result<Spanned<DeTable<'_>>, Error> {
    DeTable::parse(text).map_err(|error| {
        let syntax = Error::Syntax {
            message: error.message().to_owned(),
        };
        match error.span() {
            Some(span) => Error::at(line_of(text, span.start), syntax),
            None => syntax,
        }
    })
}

/// Uma tabela do arquivo (o arquivo inteiro ou uma `[[fontes]]`) e o texto do
/// arquivo, pra achar a linha de cada campo.
pub(crate) struct Table<'a> {
    text: &'a str,
    fields: &'a DeTable<'a>,
    /// Onde a tabela começa. O arquivo inteiro não tem uma linha só.
    line: Option<usize>,
}

impl<'a> Table<'a> {
    /// A tabela do arquivo inteiro.
    pub(crate) fn root(text: &'a str, fields: &'a DeTable<'a>) -> Self {
        Self {
            text,
            fields,
            line: None,
        }
    }

    /// Confere que a tabela só tem os campos conhecidos.
    pub(crate) fn only(&self, known: &'static [&'static str]) -> Result<(), Error> {
        for (key, _) in self.fields {
            let field: &str = key.get_ref();
            if !known.contains(&field) {
                return Err(Error::at(
                    self.line_of(key.span().start),
                    Error::UnknownField {
                        field: field.to_owned(),
                        known,
                    },
                ));
            }
        }
        Ok(())
    }

    /// Um campo de texto, se estiver na tabela.
    pub(crate) fn string(&self, field: &'static str) -> Result<Option<Located<&'a str>>, Error> {
        let Some(value) = self.get(field) else {
            return Ok(None);
        };
        let line = self.line_of(value.span().start);
        match value.get_ref() {
            DeValue::String(text) => Ok(Some(Located {
                value: text.as_ref(),
                line,
            })),
            _ => Err(wrong_type(line, field, "um texto entre aspas")),
        }
    }

    /// Um campo de texto que não pode faltar.
    pub(crate) fn required_string(&self, field: &'static str) -> Result<Located<&'a str>, Error> {
        self.string(field)?.ok_or_else(|| {
            let missing = Error::MissingField { field };
            match self.line {
                Some(line) => Error::at(line, missing),
                None => missing,
            }
        })
    }

    /// Uma lista de textos, como `["a", "b"]`. Sem o campo, a lista vem vazia.
    pub(crate) fn strings(&self, field: &'static str) -> Result<Vec<Located<&'a str>>, Error> {
        const EXPECTED: &str = "uma lista de textos, como [\"a\", \"b\"]";
        let Some(value) = self.get(field) else {
            return Ok(Vec::new());
        };
        let DeValue::Array(items) = value.get_ref() else {
            return Err(wrong_type(
                self.line_of(value.span().start),
                field,
                EXPECTED,
            ));
        };
        items
            .iter()
            .map(|item| {
                let line = self.line_of(item.span().start);
                match item.get_ref() {
                    DeValue::String(text) => Ok(Located {
                        value: text.as_ref(),
                        line,
                    }),
                    _ => Err(wrong_type(line, field, EXPECTED)),
                }
            })
            .collect()
    }

    /// Uma lista de tabelas, como as `[[fontes]]`. Sem o campo, a lista vem
    /// vazia.
    pub(crate) fn tables(&self, field: &'static str) -> Result<Vec<Table<'a>>, Error> {
        const EXPECTED: &str = "uma lista de tabelas, cada uma começando com [[fontes]]";
        let Some(value) = self.get(field) else {
            return Ok(Vec::new());
        };
        let DeValue::Array(items) = value.get_ref() else {
            return Err(wrong_type(
                self.line_of(value.span().start),
                field,
                EXPECTED,
            ));
        };
        items
            .iter()
            .map(|item| {
                let line = self.line_of(item.span().start);
                match item.get_ref() {
                    DeValue::Table(fields) => Ok(Table {
                        text: self.text,
                        fields,
                        line: Some(line),
                    }),
                    _ => Err(wrong_type(line, field, EXPECTED)),
                }
            })
            .collect()
    }

    fn get(&self, field: &str) -> Option<&'a Spanned<DeValue<'a>>> {
        self.fields
            .iter()
            .find(|(key, _)| key.get_ref() == field)
            .map(|(_, value)| value)
    }

    fn line_of(&self, offset: usize) -> usize {
        line_of(self.text, offset)
    }
}

fn wrong_type(line: usize, field: &str, expected: &'static str) -> Error {
    Error::at(
        line,
        Error::WrongType {
            field: field.to_owned(),
            expected,
        },
    )
}

/// A linha de um ponto do texto, contando do 1.
fn line_of(text: &str, offset: usize) -> usize {
    text.get(..offset)
        .map_or(1, |before| before.matches('\n').count() + 1)
}
