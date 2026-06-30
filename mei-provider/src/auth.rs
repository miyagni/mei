//! Resolving how to authenticate a request to a provider.

use crate::catalog::{Model, Provider};
use crate::credential::Credential;
use crate::error::AuthError;

/// How to authenticate a request to a provider: the value to send and the
/// endpoint to send it to.
pub struct Auth {
    /// The value to send: an API key, or an OAuth access token.
    pub key: String,
    /// The provider endpoint.
    pub base_url: &'static str,
}

impl Auth {
    /// Resolve auth for a request to `model` on `provider`, from a stored
    /// credential, falling back to the provider's env var.
    ///
    /// - `Err(ModelDisabled)` if the model is currently disabled — the request
    ///   is refused before any credential is even considered.
    /// - `Ok(None)` when nothing is configured (no credential, no env var).
    /// - `Ok(Some(auth))` otherwise: a stored credential wins — its API key, or
    ///   its OAuth access token; else the first of the provider's env vars set.
    pub fn resolve(
        provider: &Provider,
        model: &Model,
        credential: Option<&Credential>,
    ) -> Result<Option<Auth>, AuthError> {
        if let Some(reason) = model.disabled_reason() {
            return Err(AuthError::ModelDisabled(reason));
        }
        let key = match credential {
            Some(Credential::ApiKey(key)) => key.clone(),
            Some(Credential::OAuth(token)) => token.access_token.clone(),
            None => match provider.env.iter().find_map(|var| std::env::var(var).ok()) {
                Some(key) => key,
                None => return Ok(None),
            },
        };
        Ok(Some(Auth {
            key,
            base_url: provider.base_url,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credential::OAuthToken;

    fn test_provider(env: &'static [&'static str]) -> Provider {
        Provider {
            id: "test",
            name: "Test",
            base_url: "https://api.test",
            env,
        }
    }

    fn test_model(id: &'static str) -> Model {
        Model {
            provider: "test",
            id,
            name: "Test Model",
            context: 0,
            max_output: 0,
        }
    }

    #[test]
    fn api_key_credential_resolves_to_its_key() {
        let cred = Credential::ApiKey("sk-x".into());
        let auth = Auth::resolve(&test_provider(&[]), &test_model("live"), Some(&cred))
            .expect("not disabled")
            .expect("resolves");
        assert_eq!(auth.key, "sk-x");
        assert_eq!(auth.base_url, "https://api.test");
    }

    #[test]
    fn oauth_credential_resolves_to_the_access_token() {
        let cred = Credential::OAuth(OAuthToken {
            access_token: "at-123".into(),
            refresh_token: None,
            expires_at: None,
        });
        let auth = Auth::resolve(&test_provider(&[]), &test_model("live"), Some(&cred))
            .expect("not disabled")
            .expect("resolves");
        assert_eq!(auth.key, "at-123");
    }

    #[test]
    fn falls_back_to_the_env_var() {
        std::env::set_var("MEI_TEST_AUTH_KEY_FALLBACK", "sk-from-env");
        let auth = Auth::resolve(
            &test_provider(&["MEI_TEST_AUTH_KEY_FALLBACK"]),
            &test_model("live"),
            None,
        )
        .expect("not disabled")
        .expect("from env");
        assert_eq!(auth.key, "sk-from-env");
        std::env::remove_var("MEI_TEST_AUTH_KEY_FALLBACK");
    }

    #[test]
    fn nothing_configured_is_none() {
        let auth = Auth::resolve(
            &test_provider(&["MEI_TEST_AUTH_KEY_DEFINITELY_UNSET"]),
            &test_model("live"),
            None,
        )
        .expect("not disabled");
        assert!(auth.is_none());
    }

    #[test]
    fn a_disabled_model_is_refused_before_any_credential() {
        let cred = Credential::ApiKey("sk-x".into());
        match Auth::resolve(&test_provider(&[]), &test_model("claude-fable-5"), Some(&cred)) {
            Err(AuthError::ModelDisabled(reason)) => {
                assert!(reason.contains("fable-mythos-access"));
            }
            _ => panic!("expected ModelDisabled for fable"),
        }
    }
}
