//! mei-config: resolves the Mei global config directory.
//!
//! Every Mei agent stores its data — sessions, provider auth, and so on — under
//! one directory chosen by the user through the `MEI_GLOBAL_CONFIG_DIR`
//! environment variable. There is no default: if it is unset, [`config_dir`]
//! errors, so the storage location is always explicit. The directory is created
//! if it does not exist yet.

use std::ffi::OsString;
use std::path::PathBuf;

use thiserror::Error;

/// The environment variable that points at the Mei config directory.
pub const ENV_VAR: &str = "MEI_GLOBAL_CONFIG_DIR";

/// Errors resolving the config directory.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("MEI_GLOBAL_CONFIG_DIR is not set; point it at a directory for Mei to store its data")]
    Unset,

    #[error("creating the config directory: {0}")]
    Io(#[from] std::io::Error),
}

/// The Mei config directory, created if it does not exist yet.
///
/// Reads [`ENV_VAR`]; returns [`ConfigError::Unset`] if it is not set.
pub fn config_dir() -> Result<PathBuf, ConfigError> {
    let dir = resolve(std::env::var_os(ENV_VAR))?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Resolves the directory from a raw env value. Kept separate from the env read
/// so the resolution rule can be tested without touching the process env.
fn resolve(raw: Option<OsString>) -> Result<PathBuf, ConfigError> {
    match raw {
        Some(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        _ => Err(ConfigError::Unset),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_a_set_path() {
        let dir = resolve(Some(OsString::from("/data/mei"))).expect("set");
        assert_eq!(dir, PathBuf::from("/data/mei"));
    }

    #[test]
    fn unset_is_an_error() {
        assert!(matches!(resolve(None), Err(ConfigError::Unset)));
    }

    #[test]
    fn empty_is_an_error() {
        assert!(matches!(
            resolve(Some(OsString::new())),
            Err(ConfigError::Unset)
        ));
    }
}
