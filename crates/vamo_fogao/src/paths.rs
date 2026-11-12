//! Onde o `vamo` guarda as coisas, no padrão XDG.

use std::path::PathBuf;

use color_eyre::eyre::{Result, WrapErr};
use etcetera::BaseStrategy;

/// O nome da pasta do app dentro das pastas do XDG.
const APP_DIR: &str = "vamo_fogao";

/// O arquivo do banco: `$XDG_DATA_HOME/vamo_fogao/vamo.db`, que na maioria
/// das máquinas é `~/.local/share/vamo_fogao/vamo.db`.
pub(crate) fn database() -> Result<PathBuf> {
    let base = etcetera::choose_base_strategy()
        .wrap_err("não achei a pasta pessoal: a variável HOME está definida?")?;
    Ok(base.data_dir().join(APP_DIR).join("vamo.db"))
}

/// O arquivo de config: `$XDG_CONFIG_HOME/vamo_fogao/config.toml`, que na
/// maioria das máquinas é `~/.config/vamo_fogao/config.toml`.
pub(crate) fn config() -> Result<PathBuf> {
    let base = etcetera::choose_base_strategy()
        .wrap_err("não achei a pasta pessoal: a variável HOME está definida?")?;
    Ok(base.config_dir().join(APP_DIR).join("config.toml"))
}

/// A cópia local da agenda, na pasta de cache:
/// `$XDG_CACHE_HOME/vamo_fogao/agenda.ics`, que na maioria das máquinas é
/// `~/.cache/vamo_fogao/agenda.ics`. Dá pra apagar: o `vamo sync` baixa de
/// novo.
pub(crate) fn agenda_cache() -> Result<PathBuf> {
    let base = etcetera::choose_base_strategy()
        .wrap_err("não achei a pasta pessoal: a variável HOME está definida?")?;
    Ok(base.cache_dir().join(APP_DIR).join("agenda.ics"))
}

/// A cópia local do calendário de jogos, ao lado da agenda:
/// `$XDG_CACHE_HOME/vamo_fogao/jogos.ics`.
pub(crate) fn games_cache() -> Result<PathBuf> {
    Ok(agenda_cache()?.with_file_name("jogos.ics"))
}

#[cfg(feature = "google")]
/// A cópia local da agenda do Google, ao lado da do ICS:
/// `$XDG_CACHE_HOME/vamo_fogao/agenda-google.json`.
pub(crate) fn google_cache() -> Result<PathBuf> {
    Ok(agenda_cache()?.with_file_name("agenda-google.json"))
}

#[cfg(feature = "google")]
/// O token do Google, na pasta da config e só com permissão pra quem usa:
/// `$XDG_CONFIG_HOME/vamo_fogao/google-token.json`. Apagar o arquivo
/// desconecta.
pub(crate) fn google_token() -> Result<PathBuf> {
    Ok(config()?.with_file_name("google-token.json"))
}

/// Até onde o `vamo lembrar` já olhou, na pasta de estado:
/// `$XDG_STATE_HOME/vamo_fogao/lembretes`, que na maioria das máquinas é
/// `~/.local/state/vamo_fogao/lembretes`. Apagar só faz a próxima rodada
/// olhar os últimos minutos de novo.
pub(crate) fn reminders_state() -> Result<PathBuf> {
    let base = etcetera::choose_base_strategy()
        .wrap_err("não achei a pasta pessoal: a variável HOME está definida?")?;
    // Sem pasta de estado (fora do Linux), fica junto do banco.
    let dir = base.state_dir().unwrap_or_else(|| base.data_dir());
    Ok(dir.join(APP_DIR).join("lembretes"))
}
