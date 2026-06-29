use serde::{Deserialize, Serialize};

/// A stored credential for a provider: a raw API key or an OAuth token set.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Credential {
    /// A provider API key (e.g. the value of `ANTHROPIC_API_KEY`).
    ApiKey(String),
    /// OAuth tokens from a provider subscription login.
    OAuth(OAuthToken),
}

/// OAuth tokens obtained from a provider's login flow.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    /// Refresh token, when the provider issues one.
    pub refresh_token: Option<String>,
    /// Unix seconds at which the access token expires, when known.
    pub expires_at: Option<i64>,
}
