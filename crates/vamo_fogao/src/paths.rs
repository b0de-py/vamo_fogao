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
