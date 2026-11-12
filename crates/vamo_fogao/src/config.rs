//! O arquivo de config: `$XDG_CONFIG_HOME/vamo_fogao/config.toml`, que na
//! maioria das máquinas é `~/.config/vamo_fogao/config.toml`.
//!
//! O arquivo é opcional: sem ele, vale o padrão de cada campo. Um campo que
//! o `vamo` não conhece é erro, não silêncio, pra um erro de digitação não
//! passar batido.

use std::collections::BTreeMap;

use color_eyre::eyre::{Result, WrapErr, bail, eyre};
use jiff::SignedDuration;
use jiff::civil::Weekday;
use toml::de::{DeTable, DeValue};
use vamo_core::{Block, TimeRange, TimeRules};
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
    /// A seção `[tempo]`: as regras do tempo.
    pub(crate) rules: TimeRules,
    /// `brecha_minima = 20`, na seção `[tempo]`: a menor brecha que vale
    /// mostrar, em minutos.
    pub(crate) min_gap: Option<SignedDuration>,
    /// A seção `[apelidos]`: o título curto de cada evento recorrente da
    /// agenda, pela chave do título do evento em minúsculas.
    pub(crate) aliases: BTreeMap<String, String>,
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
            "tempo" => (config.rules, config.min_gap) = rules(value.get_ref())?,
            "apelidos" => config.aliases = aliases(value.get_ref())?,
            other => bail!(
                "`{other}` não é um campo da config: os campos que existem são linguagem, \
                 animacao e as seções [agenda], [tempo] e [apelidos]"
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

/// Lê a seção `[apelidos]`, o título curto dos eventos recorrentes:
///
/// ```toml
/// [apelidos]
/// "Reunião diária da equipe de produto" = "Daily"
/// ```
fn aliases(value: &DeValue<'_>) -> Result<BTreeMap<String, String>> {
    let DeValue::Table(table) = value else {
        bail!("`apelidos` é uma seção: escreva [apelidos] numa linha e os apelidos embaixo");
    };
    let mut aliases = BTreeMap::new();
    for (key, value) in table {
        let title = key.get_ref().trim();
        let DeValue::String(alias) = value.get_ref() else {
            bail!("o apelido de \"{title}\" é um texto entre aspas, como \"{title}\" = \"Daily\"");
        };
        let words = alias.split_whitespace().count();
        if words == 0 || words > vamo_core::NotebookTitle::MAX_WORDS {
            bail!(
                "o apelido de \"{title}\" precisa ter de 1 a {} palavras, como uma linha do caderno",
                vamo_core::NotebookTitle::MAX_WORDS
            );
        }
        let alias = alias.split_whitespace().collect::<Vec<_>>().join(" ");
        aliases.insert(title.to_lowercase(), alias);
    }
    Ok(aliases)
}

/// Os dias do expediente quando a config não diz: de segunda a sexta.
const WEEKDAYS: [Weekday; 5] = [
    Weekday::Monday,
    Weekday::Tuesday,
    Weekday::Wednesday,
    Weekday::Thursday,
    Weekday::Friday,
];

/// Lê a seção `[tempo]`:
///
/// ```toml
/// [tempo]
/// expediente = "09:00-18:00"
/// dias_de_expediente = ["seg", "ter", "qua", "qui", "sex"]
/// teto_da_noite = "22:00"
/// brecha_minima = 15
///
/// [[tempo.protegido]]
/// nome = "Almoço"
/// horario = "12:00-13:00"
/// dias = ["seg", "ter", "qua", "qui", "sex"]
/// ```
fn rules(value: &DeValue<'_>) -> Result<(TimeRules, Option<SignedDuration>)> {
    let DeValue::Table(table) = value else {
        bail!("`tempo` é uma seção: escreva [tempo] numa linha e os campos embaixo");
    };
    let mut rules = TimeRules::default();
    let mut hours = None;
    let mut days = None;
    let mut min_gap = None;
    for (key, value) in table {
        let value = value.get_ref();
        match key.get_ref().as_ref() {
            "brecha_minima" => min_gap = Some(minutes("brecha_minima", value)?),
            "expediente" => hours = Some(range("expediente", value)?),
            "dias_de_expediente" => days = Some(weekdays("dias_de_expediente", value)?),
            "teto_da_noite" => {
                let DeValue::String(text) = value else {
                    bail!("`teto_da_noite` é uma hora entre aspas, como teto_da_noite = \"22:00\"");
                };
                rules.night_cap = Some(vamo_core::time_of_day(text)?);
            }
            "protegido" => {
                let DeValue::Array(blocks) = value else {
                    bail!("cada bloco protegido é uma seção [[tempo.protegido]]");
                };
                for block in blocks {
                    rules.protected.push(protected(block.get_ref())?);
                }
            }
            other => bail!(
                "`{other}` não é um campo da seção [tempo]: os campos que existem são \
                 expediente, dias_de_expediente, teto_da_noite, brecha_minima e \
                 [[tempo.protegido]]"
            ),
        }
    }
    match (hours, days) {
        (Some(hours), days) => {
            rules.workday = Some(Block {
                name: "expediente".to_owned(),
                hours,
                days: days.unwrap_or_else(|| WEEKDAYS.to_vec()),
            });
        }
        (None, Some(_)) => {
            bail!("`dias_de_expediente` precisa do `expediente`, como \"09:00-18:00\"")
        }
        (None, None) => {}
    }
    Ok((rules, min_gap))
}

/// Um número de minutos, inteiro e maior que zero.
fn minutes(field: &str, value: &DeValue<'_>) -> Result<SignedDuration> {
    let wrong = || eyre!("`{field}` é um número de minutos, sem aspas, como {field} = 15");
    let DeValue::Integer(number) = value else {
        return Err(wrong());
    };
    let minutes: i64 = i64::from_str_radix(number.as_str(), number.radix()).map_err(|_| wrong())?;
    if !(1..=24 * 60).contains(&minutes) {
        return Err(wrong());
    }
    Ok(SignedDuration::from_mins(minutes))
}

/// Um `[[tempo.protegido]]`.
fn protected(value: &DeValue<'_>) -> Result<Block> {
    let DeValue::Table(table) = value else {
        bail!("cada bloco protegido é uma seção [[tempo.protegido]]");
    };
    let mut name = None;
    let mut hours = None;
    let mut days = Vec::new();
    for (key, value) in table {
        let value = value.get_ref();
        match key.get_ref().as_ref() {
            "nome" => {
                let DeValue::String(text) = value else {
                    bail!(
                        "`nome` do bloco protegido é um texto entre aspas, como nome = \"Almoço\""
                    );
                };
                name = Some(text.trim().to_owned());
            }
            "horario" => hours = Some(range("horario", value)?),
            "dias" => days = weekdays("dias", value)?,
            other => bail!(
                "`{other}` não é um campo do bloco protegido: os campos são nome, horario e dias"
            ),
        }
    }
    let hours = hours
        .ok_or_else(|| eyre!("todo bloco protegido precisa do `horario`, como \"12:00-13:00\""))?;
    Ok(Block {
        name: name.unwrap_or_else(|| "bloco protegido".to_owned()),
        hours,
        days,
    })
}

/// Um horário como `"09:00-18:00"`.
fn range(field: &str, value: &DeValue<'_>) -> Result<TimeRange> {
    let DeValue::String(text) = value else {
        bail!("`{field}` é um horário entre aspas, como {field} = \"09:00-18:00\"");
    };
    Ok(text.parse()?)
}

/// Uma lista de dias como `["seg", "qua"]`.
fn weekdays(field: &str, value: &DeValue<'_>) -> Result<Vec<Weekday>> {
    let wrong = || {
        eyre!(
            "`{field}` é uma lista de dias, como {field} = [\"seg\", \"qua\", \"sex\"] (os dias \
             são seg, ter, qua, qui, sex, sab e dom)"
        )
    };
    let DeValue::Array(items) = value else {
        return Err(wrong());
    };
    items
        .iter()
        .map(|item| match item.get_ref() {
            DeValue::String(day) => weekday(day).ok_or_else(wrong),
            _ => Err(wrong()),
        })
        .collect()
}

/// `seg`, `segunda`, `segunda-feira`, `sáb`, `sabado`...
fn weekday(text: &str) -> Option<Weekday> {
    let text = text.trim().to_lowercase();
    let name = text.strip_suffix("-feira").unwrap_or(&text);
    Some(match name {
        "seg" | "segunda" => Weekday::Monday,
        "ter" | "terça" | "terca" => Weekday::Tuesday,
        "qua" | "quarta" => Weekday::Wednesday,
        "qui" | "quinta" => Weekday::Thursday,
        "sex" | "sexta" => Weekday::Friday,
        "sab" | "sáb" | "sábado" | "sabado" => Weekday::Saturday,
        "dom" | "domingo" => Weekday::Sunday,
        _ => return None,
    })
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
    fn aliases_are_keyed_by_the_lowercase_title() {
        let config =
            parse("[apelidos]\n\"Reunião Diária da Equipe\" = \"Daily  da equipe\"\n").unwrap();
        assert_eq!(
            config
                .aliases
                .get("reunião diária da equipe")
                .map(String::as_str),
            Some("Daily da equipe")
        );
        assert!(parse("[apelidos]\nAcademia = 1\n").is_err());
        assert!(parse("[apelidos]\nAcademia = \"um dois três quatro cinco seis\"\n").is_err());
    }

    #[test]
    fn the_minimum_gap_is_in_minutes() {
        let config = parse("[tempo]\nbrecha_minima = 20\n").unwrap();
        assert_eq!(config.min_gap, Some(SignedDuration::from_mins(20)));
        for wrong in ["\"20\"", "0", "-5", "1.5"] {
            assert!(
                parse(&format!("[tempo]\nbrecha_minima = {wrong}\n")).is_err(),
                "{wrong}"
            );
        }
    }

    #[test]
    fn the_time_rules_come_in_their_section() {
        let config = parse(
            "[tempo]\n\
             expediente = \"9h-18h\"\n\
             teto_da_noite = \"22:00\"\n\
             [[tempo.protegido]]\n\
             nome = \"Almoço\"\n\
             horario = \"12:00-13:00\"\n\
             [[tempo.protegido]]\n\
             nome = \"Escola\"\n\
             horario = \"17:00-17:40\"\n\
             dias = [\"seg\", \"quarta\"]\n",
        )
        .unwrap();
        let rules = config.rules;
        let workday = rules.workday.unwrap();
        assert_eq!(workday.hours.to_string(), "09:00–18:00");
        assert_eq!(workday.days, WEEKDAYS);
        assert_eq!(rules.night_cap, Some(jiff::civil::time(22, 0, 0, 0)));
        assert_eq!(rules.protected.len(), 2);
        assert_eq!(rules.protected[0].days, []);
        assert_eq!(
            rules.protected[1].days,
            [Weekday::Monday, Weekday::Wednesday]
        );
    }

    #[test]
    fn broken_time_rules_say_what_is_wrong() {
        for text in [
            "[tempo]\nexpediente = \"18:00-09:00\"",
            "[tempo]\nteto_da_noite = 22",
            "[tempo]\ndias_de_expediente = [\"seg\"]",
            "[tempo]\nexpediente = \"09:00-18:00\"\ndias_de_expediente = [\"segundou\"]",
            "[[tempo.protegido]]\nnome = \"Almoço\"",
            "[tempo]\nalmoco = \"12:00-13:00\"",
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
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
