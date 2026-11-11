//! As fontes de um fato e a régua de independência.

use std::fmt;
use std::str::FromStr;

use crate::Error;

/// Os rótulos que vêm logo antes do domínio de um país e não identificam
/// ninguém sozinhos, como o `com` do `com.br` e o `co` do `co.uk`. Com um
/// deles, o domínio registrável tem três partes (`lance.com.br`); sem, duas
/// (`globo.com`).
const SECOND_LEVEL: &[&str] = &[
    "ac", "art", "blog", "co", "com", "edu", "esp", "gob", "gov", "jor", "jus", "leg", "mp", "mus",
    "net", "nom", "org", "tur",
];

/// Os domínios da Wikimedia e os espelhos da Wikipédia: tudo conta como uma
/// fonte só, a Wikipédia.
const WIKIPEDIA: &[&str] = &[
    "wikipedia.org",
    "wikimedia.org",
    "wikidata.org",
    "wikiwand.com",
];

/// Uma fonte de um fato: o nome (o veículo, o livro, o documento) e o link,
/// quando a fonte está na internet.
///
/// ```
/// use vamo_acervo::Source;
///
/// let lance = Source::new("Lance!", Some("https://www.lance.com.br/botafogo".parse()?))?;
/// assert_eq!(lance.origin().as_str(), "lance.com.br");
///
/// // Fonte fora da internet não tem link, e a origem é o próprio nome.
/// let minutes = Source::new("Ata de fundação do clube", None)?;
/// assert_eq!(minutes.origin().as_str(), "ata de fundação do clube");
/// # Ok::<(), vamo_acervo::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    name: String,
    url: Option<SourceUrl>,
}

impl Source {
    /// Monta uma fonte. O espaço nas pontas do nome sai.
    ///
    /// # Errors
    ///
    /// [`Error::EmptySourceName`] se o nome estiver vazio.
    pub fn new(name: &str, url: Option<SourceUrl>) -> Result<Self, Error> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::EmptySourceName);
        }
        Ok(Self {
            name: name.to_owned(),
            url,
        })
    }

    /// O nome da fonte.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// O link, se a fonte está na internet.
    pub fn url(&self) -> Option<&SourceUrl> {
        self.url.as_ref()
    }

    /// De onde a fonte vem, pra régua de independência: o domínio
    /// registrável do link ou, pra fonte sem link, o próprio nome.
    pub fn origin(&self) -> Origin {
        match &self.url {
            Some(url) => Origin(registrable_domain(url.host())),
            None => Origin(self.name.to_lowercase()),
        }
    }
}

/// O link de uma fonte: `https://` (ou `http://`) e um domínio com ponto,
/// sem espaço.
///
/// ```
/// use vamo_acervo::SourceUrl;
///
/// let url: SourceUrl = "https://pt.wikipedia.org/wiki/Botafogo".parse()?;
/// assert_eq!(url.host(), "pt.wikipedia.org");
///
/// assert!("pt.wikipedia.org/wiki/Botafogo".parse::<SourceUrl>().is_err());
/// # Ok::<(), vamo_acervo::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUrl {
    url: String,
    host: String,
}

impl SourceUrl {
    /// O link inteiro, como veio.
    pub fn as_str(&self) -> &str {
        &self.url
    }

    /// O domínio do link, em minúsculas e sem a porta.
    pub fn host(&self) -> &str {
        &self.host
    }
}

impl FromStr for SourceUrl {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Error> {
        match host_of(input) {
            Some(host) => Ok(Self {
                url: input.to_owned(),
                host,
            }),
            None => Err(Error::InvalidUrl {
                input: input.to_owned(),
            }),
        }
    }
}

impl fmt::Display for SourceUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.url)
    }
}

/// De onde vem uma fonte, pra régua de independência: duas fontes da mesma
/// origem contam como uma só.
///
/// ```
/// use vamo_acervo::Source;
///
/// let english = Source::new(
///     "Wikipédia em inglês",
///     Some("https://en.wikipedia.org/wiki/Botafogo_de_Futebol_e_Regatas".parse()?),
/// )?;
/// let portuguese = Source::new(
///     "Wikipédia",
///     Some("https://pt.wikipedia.org/wiki/Botafogo_de_Futebol_e_Regatas".parse()?),
/// )?;
/// assert_eq!(english.origin(), portuguese.origin());
/// # Ok::<(), vamo_acervo::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Origin(String);

impl Origin {
    /// A origem como texto: um domínio, como `lance.com.br`, ou o nome da
    /// fonte, em minúsculas.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// O domínio de um link, em minúsculas, ou `None` se o link não serve.
fn host_of(url: &str) -> Option<String> {
    if url.chars().any(char::is_whitespace) {
        return None;
    }
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    // O domínio vai até a primeira barra, interrogação ou cerquilha.
    let authority = rest.split(['/', '?', '#']).next()?;
    // Link com usuário (`https://alguem@site.com`) não é link de fonte.
    if authority.contains('@') {
        return None;
    }
    let host = match authority.split_once(':') {
        Some((host, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => host,
        Some(_) => return None,
        None => authority,
    };
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    let labels_ok = host.split('.').count() >= 2
        && host.split('.').all(|label| {
            !label.is_empty()
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        });
    labels_ok.then_some(host)
}

/// O domínio registrável de um host, o pedaço que alguém registra e
/// controla: `lance.com.br` pra `www.lance.com.br`, `globo.com` pra
/// `ge.globo.com`. A Wikipédia, em qualquer idioma ou espelho, vira
/// `wikipedia.org`.
///
/// É uma aproximação da lista de sufixos públicos (a Public Suffix List). Ela
/// dá conta dos sites de imprensa e de arquivo que o acervo usa, sem trazer
/// uma dependência com a lista inteira. Quando erra, erra pro lado da
/// cautela: dois sites num sufixo que ela não conhece parecem o mesmo, e o
/// fato pede mais uma fonte.
fn registrable_domain(host: &str) -> String {
    let labels: Vec<&str> = host.split('.').collect();
    let keep = match labels.as_slice() {
        [.., second, country] if country.len() == 2 && SECOND_LEVEL.contains(second) => 3,
        _ => 2,
    };
    let domain = labels[labels.len().saturating_sub(keep)..].join(".");
    if WIKIPEDIA.contains(&domain.as_str()) {
        "wikipedia.org".to_owned()
    } else {
        domain
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn origin(url: &str) -> String {
        let url: SourceUrl = url.parse().unwrap();
        Source::new("fonte", Some(url)).unwrap().origin().0
    }

    #[test]
    fn reads_the_host_of_a_link() {
        for (url, host) in [
            ("https://www.lance.com.br/botafogo", "www.lance.com.br"),
            ("http://rsssf.org", "rsssf.org"),
            ("https://Trivela.com.br:443/x?y=1#z", "trivela.com.br"),
            ("https://lance.com.br./x", "lance.com.br"),
        ] {
            assert_eq!(url.parse::<SourceUrl>().unwrap().host(), host, "{url}");
        }
    }

    #[test]
    fn refuses_what_is_not_a_link() {
        for input in [
            "",
            "lance.com.br",
            "ftp://lance.com.br",
            "https://",
            "https://localhost/x",
            "https://lance .com.br",
            "https://alguem@lance.com.br",
            "https://lance.com.br:porta",
            "https://lance..com.br",
            "https://lancê.com.br",
        ] {
            assert_eq!(
                input.parse::<SourceUrl>(),
                Err(Error::InvalidUrl {
                    input: input.to_owned()
                }),
                "{input:?}"
            );
        }
    }

    #[test]
    fn finds_the_registrable_domain() {
        for (url, expected) in [
            ("https://www.lance.com.br/x", "lance.com.br"),
            ("https://lance.com.br/x", "lance.com.br"),
            ("https://ge.globo.com/x", "globo.com"),
            ("https://oglobo.globo.com/x", "globo.com"),
            ("https://global.espn.com/x", "espn.com"),
            ("https://www.bbc.co.uk/x", "bbc.co.uk"),
            ("https://bndigital.bn.gov.br/x", "bn.gov.br"),
            ("http://memoria.bn.br/x", "bn.br"),
            ("https://www.rsssf.org/x", "rsssf.org"),
            ("https://trivela.co/x", "trivela.co"),
        ] {
            assert_eq!(origin(url), expected, "{url}");
        }
    }

    #[test]
    fn the_wikipedia_in_any_language_or_mirror_is_one_origin() {
        for url in [
            "https://en.wikipedia.org/wiki/Botafogo_de_Futebol_e_Regatas",
            "https://pt.wikipedia.org/wiki/Botafogo_de_Futebol_e_Regatas",
            "https://pt.m.wikipedia.org/wiki/Botafogo_de_Futebol_e_Regatas",
            "https://commons.wikimedia.org/wiki/Category:Botafogo",
            "https://www.wikidata.org/wiki/Q208226",
            "https://www.wikiwand.com/pt/Botafogo",
        ] {
            assert_eq!(origin(url), "wikipedia.org", "{url}");
        }
    }

    #[test]
    fn a_source_without_link_is_its_own_origin() {
        let book = Source::new("  Ata de Fundação  ", None).unwrap();
        assert_eq!(book.name(), "Ata de Fundação");
        assert_eq!(book.origin().as_str(), "ata de fundação");
    }

    #[test]
    fn a_source_needs_a_name() {
        assert_eq!(Source::new(" ", None), Err(Error::EmptySourceName));
    }

    proptest! {
        #[test]
        fn subdomains_never_make_a_new_origin(
            subdomain in "[a-z]{1,8}(\\.[a-z]{1,8}){0,2}",
            domain in prop::sample::select(vec!["lance.com.br", "globo.com", "wikipedia.org"]),
        ) {
            let url = format!("https://{subdomain}.{domain}/x");
            prop_assert_eq!(origin(&url), domain);
        }

        #[test]
        fn any_text_either_reads_as_a_link_or_fails_without_panicking(input in "\\PC{0,40}") {
            let _ = input.parse::<SourceUrl>();
        }
    }
}
