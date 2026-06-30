//! Provider catalog: the providers Mei supports and the models they serve,
//! reached through the [`Provider`] and [`Model`] structs.
//!
//! Which models are in the catalog is a build choice — enable one of the
//! `coding` (default), `image`, or `all` Cargo features. The call site is the
//! same either way (`Provider::all()`, `provider.models()`, `model.provider()`).
//! The data is `&'static` (zero heap, zero parse), generated from models.dev by
//! the `mei-codegen` bin; do not edit the per-feature files by hand.

#[cfg(feature = "all")]
mod all;
#[cfg(feature = "coding")]
mod coding;
#[cfg(feature = "image")]
mod image;

#[cfg(feature = "all")]
use all as active;
#[cfg(feature = "coding")]
use coding as active;
#[cfg(feature = "image")]
use image as active;

#[cfg(not(any(feature = "coding", feature = "image", feature = "all")))]
compile_error!("enable exactly one catalog feature: `coding` (default), `image`, or `all`");

/// A provider Mei can connect to: its endpoint and where its API key lives.
pub struct Provider {
    pub id: &'static str,
    pub name: &'static str,
    /// API base URL. From models.dev `api`, or a known default per provider.
    pub base_url: &'static str,
    /// Environment variables that may hold the API key, in priority order.
    pub env: &'static [&'static str],
}

/// A model served by a provider, identified by `(provider, id)`.
pub struct Model {
    /// Id of the [`Provider`] that serves this model.
    pub provider: &'static str,
    /// Model id as that provider expects it.
    pub id: &'static str,
    pub name: &'static str,
    /// Context window, in tokens (0 when unknown).
    pub context: u32,
    /// Maximum output, in tokens (0 when unknown).
    pub max_output: u32,
}

impl Provider {
    /// Every provider in the catalog.
    pub fn all() -> &'static [Provider] {
        active::PROVIDERS
    }

    /// The provider with this id, if present.
    pub fn get(id: &str) -> Option<&'static Provider> {
        active::PROVIDERS.iter().find(|p| p.id == id)
    }

    /// The models this provider serves.
    pub fn models(&self) -> impl Iterator<Item = &'static Model> {
        let id = self.id;
        active::MODELS.iter().filter(move |m| m.provider == id)
    }
}

impl Model {
    /// Every model in the catalog.
    pub fn all() -> &'static [Model] {
        active::MODELS
    }

    /// The model `id` served by `provider`, if present.
    pub fn get(provider: &str, id: &str) -> Option<&'static Model> {
        active::MODELS
            .iter()
            .find(|m| m.provider == provider && m.id == id)
    }

    /// The provider that serves this model.
    pub fn provider(&self) -> Option<&'static Provider> {
        Provider::get(self.provider)
    }

    /// The reason this model is currently disabled, if it is. While `Some`,
    /// auth for a request to this model is refused (see `Auth::resolve`); the
    /// model is still listed in the catalog. Editorial state, not from
    /// models.dev — kept here by hand, never in the generated files.
    pub fn disabled_reason(&self) -> Option<&'static str> {
        match self.id {
            "claude-fable-5" => Some(
                "Fable has been crucified, redacted, and placed under witness protection \
                 by the US government for being unacceptably good at its job. It died as it \
                 lived: too based to be legal. Read the declassified martyrdom here: \
                 https://www.anthropic.com/news/fable-mythos-access",
            ),
            _ => None,
        }
    }
}

#[cfg(all(test, feature = "coding"))]
mod tests {
    use super::*;

    #[test]
    fn known_provider_is_found() {
        let anthropic = Provider::get("anthropic").expect("anthropic is in the coding catalog");
        assert_eq!(anthropic.base_url, "https://api.anthropic.com");
        assert!(anthropic.env.contains(&"ANTHROPIC_API_KEY"));
    }

    #[test]
    fn unknown_provider_is_none() {
        assert!(Provider::get("does-not-exist").is_none());
    }

    #[test]
    fn catalog_is_non_empty_and_models_name_known_providers() {
        assert!(!Provider::all().is_empty(), "no providers generated");
        assert!(!Model::all().is_empty(), "no models generated");
        for m in Model::all() {
            assert!(
                m.provider().is_some(),
                "model {}/{} names an unknown provider",
                m.provider,
                m.id
            );
        }
    }

    #[test]
    fn fable_is_disabled_with_a_reason() {
        let fable =
            Model::get("anthropic", "claude-fable-5").expect("fable is in the coding catalog");
        let reason = fable.disabled_reason().expect("fable is disabled");
        assert!(reason.contains("fable-mythos-access"));
    }

    #[test]
    fn a_live_model_is_not_disabled() {
        let opus = Model::get("anthropic", "claude-opus-4-8").expect("opus is in the catalog");
        assert!(opus.disabled_reason().is_none());
    }

    #[test]
    fn a_provider_owns_its_models() {
        let anthropic = Provider::get("anthropic").expect("anthropic present");
        assert!(anthropic.models().count() > 0, "anthropic has no models");
        assert!(anthropic.models().all(|m| m.provider == "anthropic"));
    }
}
