//! O link ICS: baixa a agenda e guarda uma cópia local.
//!
//! O link secreto do Google Agenda (e de quase toda agenda) devolve o ICS
//! inteiro, sem login. A cópia local é o que deixa o app funcionar sem
//! internet: o `vamo sync` baixa e grava; a TUI e o resto só leem a cópia.

use std::path::{Path, PathBuf};
use std::time::Duration;

use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

use crate::Error;

/// Quanto esperar a agenda responder antes de desistir e usar a cópia.
const TIMEOUT: Duration = Duration::from_secs(20);

/// O link da agenda e onde a cópia dele fica.
#[derive(Debug, Clone)]
pub struct IcsLink {
    url: String,
    cache: PathBuf,
}

/// De onde veio o texto da agenda.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// Baixado agora.
    Fresh,
    /// A cópia local: a internet ou a agenda falharam.
    Cached,
}

impl IcsLink {
    /// O link `url`, com a cópia em `cache`.
    pub fn new(url: &str, cache: &Path) -> Self {
        Self {
            url: url.trim().to_owned(),
            cache: cache.to_owned(),
        }
    }

    /// Baixa a agenda e atualiza a cópia. Se a internet falhar e tiver
    /// cópia, devolve a cópia e diz que ela é velha.
    ///
    /// # Errors
    ///
    /// - [`Error::Download`] quando não deu pra baixar e não tem cópia.
    /// - [`Error::Cache`] quando não deu pra gravar a cópia.
    pub fn fetch(&self) -> Result<(String, Freshness), Error> {
        match self.download() {
            Ok(text) => {
                self.save(&text)?;
                Ok((text, Freshness::Fresh))
            }
            Err(error) => match self.cached()? {
                Some(text) => {
                    tracing::warn!(%error, "a agenda não baixou; usando a cópia local");
                    Ok((text, Freshness::Cached))
                }
                None => Err(error),
            },
        }
    }

    /// A cópia local, se já tiver uma.
    ///
    /// # Errors
    ///
    /// [`Error::Cache`] quando a cópia existe mas não dá pra ler.
    pub fn cached(&self) -> Result<Option<String>, Error> {
        match std::fs::read_to_string(&self.cache) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(Error::Cache {
                path: self.cache.clone(),
                source,
            }),
        }
    }

    fn download(&self) -> Result<String, Error> {
        let failed = |problem: String| Error::Download { problem };
        // Um arquivo `.ics` na máquina também serve: é só copiar.
        if let Some(path) = self.local_file() {
            return std::fs::read_to_string(path)
                .map_err(|_| failed(format!("não deu pra ler o arquivo {}", path.display())));
        }
        // O `webcal://` é o mesmo link, só que com o nome que abre a agenda
        // no navegador.
        let url = match self.url.strip_prefix("webcal://") {
            Some(rest) => format!("https://{rest}"),
            None => self.url.clone(),
        };
        // A TLS do sistema (OpenSSL no Linux), com os certificados do
        // sistema: é o que mantém as licenças dentro do `deny.toml`.
        let tls = TlsConfig::builder()
            .provider(TlsProvider::NativeTls)
            .root_certs(RootCerts::PlatformVerifier)
            .build();
        let agent = ureq::Agent::config_builder()
            .tls_config(tls)
            .timeout_global(Some(TIMEOUT))
            .build()
            .new_agent();
        let mut response = agent
            .get(&url)
            .call()
            // O erro do ureq traz a URL, e a URL é secreta: fica só o tipo.
            .map_err(|error| failed(describe(&error)))?;
        response
            .body_mut()
            .with_config()
            // Uma agenda de anos cabe folgada em 20 MB.
            .limit(20 * 1024 * 1024)
            .read_to_string()
            .map_err(|error| failed(describe(&error)))
    }

    /// O caminho, quando o "link" é um arquivo da máquina
    /// (`/caminho/agenda.ics` ou `file:///caminho/agenda.ics`).
    fn local_file(&self) -> Option<&Path> {
        if let Some(path) = self.url.strip_prefix("file://") {
            return Some(Path::new(path));
        }
        (!self.url.contains("://")).then(|| Path::new(&self.url))
    }

    fn save(&self, text: &str) -> Result<(), Error> {
        let failed = |source| Error::Cache {
            path: self.cache.clone(),
            source,
        };
        if let Some(folder) = self.cache.parent() {
            std::fs::create_dir_all(folder).map_err(failed)?;
        }
        std::fs::write(&self.cache, text).map_err(failed)
    }
}

/// O motivo da falha, sem a URL (que tem o segredo da agenda).
fn describe(error: &ureq::Error) -> String {
    match error {
        ureq::Error::StatusCode(code) => format!("a agenda respondeu com o código {code}"),
        ureq::Error::Timeout(_) => "a agenda demorou demais pra responder".to_owned(),
        ureq::Error::HostNotFound => "o endereço da agenda não foi achado".to_owned(),
        ureq::Error::Io(_) | ureq::Error::ConnectionFailed => {
            "não deu pra conectar na agenda (sem internet?)".to_owned()
        }
        ureq::Error::BodyExceedsLimit(_) => "a agenda passou de 20 MB".to_owned(),
        _ => "a agenda não baixou".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_internet_the_copy_is_used() {
        let folder = tempfile::tempdir().unwrap();
        let cache = folder.path().join("agenda.ics");
        std::fs::write(&cache, "BEGIN:VCALENDAR\nEND:VCALENDAR\n").unwrap();
        // Uma porta local fechada: a conexão falha na hora, sem rede.
        let link = IcsLink::new("http://127.0.0.1:9/agenda.ics", &cache);
        let (text, freshness) = link.fetch().unwrap();
        assert_eq!(freshness, Freshness::Cached);
        assert!(text.starts_with("BEGIN:VCALENDAR"));
    }

    #[test]
    fn a_file_on_the_machine_works_as_a_link() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("minha.ics");
        std::fs::write(&file, "BEGIN:VCALENDAR\nEND:VCALENDAR\n").unwrap();
        let cache = folder.path().join("cache").join("agenda.ics");
        let link = IcsLink::new(file.to_str().unwrap(), &cache);
        assert_eq!(link.fetch().unwrap().1, Freshness::Fresh);
        assert!(cache.exists());
    }

    #[test]
    fn without_internet_and_without_copy_it_is_an_error() {
        let folder = tempfile::tempdir().unwrap();
        let link = IcsLink::new(
            "http://127.0.0.1:9/agenda.ics",
            &folder.path().join("x.ics"),
        );
        let error = link.fetch().unwrap_err();
        assert!(!error.to_string().contains("127.0.0.1"), "{error}");
    }
}
