//! O arquivo de config: `$XDG_CONFIG_HOME/vamo_fogao/config.toml`, que na
//! maioria das máquinas é `~/.config/vamo_fogao/config.toml`.
//!
//! O arquivo é opcional: sem ele, vale o padrão de cada campo. Um campo que
//! o `vamo` não conhece é erro, não silêncio, pra um erro de digitação não
//! passar batido.

use color_eyre::eyre::{Result, WrapErr, bail, eyre};
use toml::de::{DeTable, DeValue};
use vamo_tui::{Animation, Voice};

/// O que a config escolhe. Cada campo é `None` quando o arquivo não fala
/// dele, pra linha de comando e o ambiente poderem ter a palavra final.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Config {
    /// `linguagem = "jogo"` ou `"neutra"`.
    pub(crate) voice: Option<Voice>,
    /// `animacao = false` apaga o fogo da TUI.
    pub(crate) animation: Option<Animation>,
    /// A seção `[agenda]`.
    pub(crate) agenda: AgendaConfig,
}

/// A seção `[agenda]`: de onde a agenda vem.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AgendaConfig {
    /// `ics = "https://..."`: o link secreto da agenda (ou o caminho de um
    /// arquivo `.ics`). É segredo: fica só na config, nunca no repositório.
    pub(crate) ics: Option<String>,
}

/// Lê a config do lugar padrão. Sem arquivo, devolve a config vazia.
pub(crate) fn load() -> Result<Config> {
    let path = crate::paths::config()?;
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            parse(&text).wrap_err_with(|| format!("a config em {} tem erro", path.display()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(error) => Err(error).wrap_err_with(|| format!("não deu pra ler {}", path.display())),
    }
}

/// Lê a config a partir do texto do arquivo.
fn parse(text: &str) -> Result<Config> {
    let table =
        DeTable::parse(text).map_err(|error| eyre!("o TOML não fecha: {}", error.message()))?;
    let mut config = Config::default();
    for (key, value) in table.get_ref() {
        match key.get_ref().as_ref() {
            "linguagem" => {
                let DeValue::String(name) = value.get_ref() else {
                    bail!("`linguagem` é um texto entre aspas, como linguagem = \"neutra\"");
                };
                config.voice = Some(name.parse()?);
            }
            "animacao" => {
                let DeValue::Boolean(on) = value.get_ref() else {
                    bail!("`animacao` é true ou false, sem aspas, como animacao = false");
                };
                config.animation = Some(if *on { Animation::On } else { Animation::Off });
            }
            "agenda" => config.agenda = agenda(value.get_ref())?,
            other => bail!(
                "`{other}` não é um campo da config: os campos que existem são linguagem, \
                 animacao e a seção [agenda]"
            ),
        }
    }
    Ok(config)
}

/// Lê a seção `[agenda]`.
fn agenda(value: &DeValue<'_>) -> Result<AgendaConfig> {
    let DeValue::Table(table) = value else {
        bail!("`agenda` é uma seção: escreva [agenda] numa linha e os campos embaixo");
    };
    let mut agenda = AgendaConfig::default();
    for (key, value) in table {
        match key.get_ref().as_ref() {
            "ics" => {
                let DeValue::String(link) = value.get_ref() else {
                    bail!("`ics` é o link da agenda entre aspas, como ics = \"https://...\"");
                };
                agenda.ics = Some(link.to_string());
            }
            other => bail!("`{other}` não é um campo da seção [agenda]: o campo que existe é ics"),
        }
    }
    Ok(agenda)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_file_chooses_nothing() {
        assert_eq!(parse("").unwrap(), Config::default());
    }

    #[test]
    fn the_voice_comes_by_name() {
        let config = parse("linguagem = \"neutra\"\n").unwrap();
        assert_eq!(config.voice, Some(Voice::Neutral));
    }

    #[test]
    fn the_animation_can_be_turned_off() {
        let config = parse("linguagem = \"jogo\"\nanimacao = false\n").unwrap();
        assert_eq!(config.animation, Some(Animation::Off));
        assert_eq!(config.voice, Some(Voice::Game));
        assert_eq!(
            parse("animacao = true").unwrap().animation,
            Some(Animation::On)
        );
        assert!(parse("animacao = \"não\"").is_err());
    }

    #[test]
    fn an_unknown_voice_is_an_error() {
        let error = parse("linguagem = \"torcida\"").unwrap_err();
        assert_eq!(
            error.to_string(),
            "`torcida` não é uma linguagem: as linguagens são jogo e neutra"
        );
    }

    #[test]
    fn the_agenda_link_comes_in_its_section() {
        let config = parse("[agenda]\nics = \"https://agenda.exemplo/basic.ics\"\n").unwrap();
        assert_eq!(
            config.agenda.ics.as_deref(),
            Some("https://agenda.exemplo/basic.ics")
        );
        assert!(parse("agenda = 1").is_err());
        assert!(parse("[agenda]\nlink = \"x\"").is_err());
    }

    #[test]
    fn an_unknown_field_is_an_error() {
        let error = parse("lingaugem = \"neutra\"").unwrap_err();
        assert!(error.to_string().contains("`lingaugem`"), "{error}");
    }

    #[test]
    fn a_broken_file_is_an_error() {
        assert!(parse("linguagem = ").is_err());
        assert!(parse("linguagem = 1").is_err());
    }
}
