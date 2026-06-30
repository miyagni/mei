//! Resolving how to authenticate a request to a provider.

use crate::catalog::Provider;
use crate::credential::Credential;

/// How to authenticate a request to a provider: the value to send and the
/// endpoint to send it to.
pub struct Auth {
    /// The value to send: an API key, or an OAuth access token.
    pub key: String,
    /// The provider endpoint.
    pub base_url: &'static str,
}

impl Auth {
    /// Resolve auth for `provider` from a stored credential, falling back to the
    /// provider's env var. Returns `None` when nothing is configured.
    ///
    /// Order: a stored credential wins — its API key, or its OAuth access token;
    /// otherwise the first of the provider's env vars that is set.
    pub fn resolve(provider: &Provider, credential: Option<&Credential>) -> Option<Auth> {
        let key = match credential {
            Some(Credential::ApiKey(key)) => key.clone(),
            Some(Credential::OAuth(token)) => token.access_token.clone(),
            None => provider
                .env
                .iter()
                .find_map(|var| std::env::var(var).ok())?,
        };
        Some(Auth {
            key,
            base_url: provider.base_url,
        })
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

    #[test]
    fn api_key_credential_resolves_to_its_key() {
        let cred = Credential::ApiKey("sk-x".into());
        let auth = Auth::resolve(&test_provider(&[]), Some(&cred)).expect("resolves");
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
        let auth = Auth::resolve(&test_provider(&[]), Some(&cred)).expect("resolves");
        assert_eq!(auth.key, "at-123");
    }

    #[test]
    fn falls_back_to_the_env_var() {
        std::env::set_var("MEI_TEST_AUTH_KEY_FALLBACK", "sk-from-env");
        let auth =
            Auth::resolve(&test_provider(&["MEI_TEST_AUTH_KEY_FALLBACK"]), None).expect("from env");
        assert_eq!(auth.key, "sk-from-env");
        std::env::remove_var("MEI_TEST_AUTH_KEY_FALLBACK");
    }

    #[test]
    fn nothing_configured_is_none() {
        let auth = Auth::resolve(
            &test_provider(&["MEI_TEST_AUTH_KEY_DEFINITELY_UNSET"]),
            None,
        );
        assert!(auth.is_none());
    }
}
