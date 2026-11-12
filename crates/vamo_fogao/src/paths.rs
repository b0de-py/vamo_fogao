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
