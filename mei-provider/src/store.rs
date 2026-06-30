use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;

use tempfile::NamedTempFile;

use crate::credential::Credential;
use crate::error::AuthError;

/// Per-provider credential store, persisted as `auth.json` under the Mei config
/// directory. The harness just calls [`AuthStore::open`]; the store finds its
/// own file via `mei-config`. [`AuthStore::open_in`] takes an explicit directory.
pub struct AuthStore {
    dir: PathBuf,
    credentials: BTreeMap<String, Credential>,
}

impl AuthStore {
    /// Opens the store at `<config>/auth.json`, where `<config>` comes from
    /// [`mei_config::config_dir`] (`MEI_GLOBAL_CONFIG_DIR`). A missing file is a
    /// fresh, empty store.
    pub fn open() -> Result<Self, AuthError> {
        Self::open_in(mei_config::config_dir()?)
    }

    /// Opens the store at `<dir>/auth.json`. For tests and callers that choose
    /// their own location.
    pub fn open_in(dir: impl Into<PathBuf>) -> Result<Self, AuthError> {
        let dir = dir.into();
        let credentials = match std::fs::read_to_string(dir.join("auth.json")) {
            Ok(json) => serde_json::from_str(&json)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self { dir, credentials })
    }

    /// The credential stored for `provider`, if any.
    pub fn get(&self, provider: &str) -> Option<&Credential> {
        self.credentials.get(provider)
    }

    /// The providers that currently have a stored credential.
    pub fn providers(&self) -> impl Iterator<Item = &str> {
        self.credentials.keys().map(String::as_str)
    }

    /// Stores (or replaces) the credential for `provider` and persists.
    pub fn set(
        &mut self,
        provider: impl Into<String>,
        credential: Credential,
    ) -> Result<(), AuthError> {
        self.credentials.insert(provider.into(), credential);
        self.persist()
    }

    /// Removes a provider's credential and persists. Returns whether one existed.
    pub fn remove(&mut self, provider: &str) -> Result<bool, AuthError> {
        if self.credentials.remove(provider).is_none() {
            return Ok(false);
        }
        self.persist()?;
        Ok(true)
    }

    /// Writes `auth.json` atomically (sibling temp file + rename). On Unix the
    /// temp file is created with mode 0600, so the credential file is not
    /// world-readable.
    fn persist(&self) -> Result<(), AuthError> {
        std::fs::create_dir_all(&self.dir)?;
        let json = serde_json::to_string_pretty(&self.credentials)?;
        let mut tmp = NamedTempFile::new_in(&self.dir)?;
        tmp.write_all(json.as_bytes())?;
        tmp.persist(self.dir.join("auth.json"))
            .map_err(|e| e.error)?;
        Ok(())
    }
}
