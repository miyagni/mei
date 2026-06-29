use std::ffi::OsString;
use std::path::PathBuf;

use thiserror::Error;

/// The environment variable pointing at Mei's global config directory, where
/// sessions, config and credentials live. The user is expected to set it.
pub const CONFIG_DIR_ENV: &str = "MEI_GLOBAL_CONFIG_DIR";

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("{CONFIG_DIR_ENV} is not set — point it at a directory for Mei's data")]
    MissingConfigDir,
}

/// Resolves the global config directory from `MEI_GLOBAL_CONFIG_DIR`. There is
/// no implicit default: if the variable is unset, that is an error.
pub fn config_dir() -> Result<PathBuf, ConfigError> {
    resolve(std::env::var_os(CONFIG_DIR_ENV))
}

fn resolve(value: Option<OsString>) -> Result<PathBuf, ConfigError> {
    match value {
        Some(dir) if !dir.is_empty() => Ok(PathBuf::from(dir)),
        _ => Err(ConfigError::MissingConfigDir),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_set_path_resolves() {
        let dir = resolve(Some(OsString::from("/data/mei"))).expect("set path resolves");
        assert_eq!(dir, PathBuf::from("/data/mei"));
    }

    #[test]
    fn unset_or_empty_is_an_error() {
        assert!(matches!(resolve(None), Err(ConfigError::MissingConfigDir)));
        assert!(matches!(
            resolve(Some(OsString::new())),
            Err(ConfigError::MissingConfigDir)
        ));
    }
}
