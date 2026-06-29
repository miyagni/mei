//! Generates mei-provider's per-feature catalogs from models.dev.
//!
//! Run: `cargo run -p mei-provider --bin mei-codegen --features codegen`.
//! Writes `src/catalog/{coding,image,all}.rs`. A curated provider missing from
//! models.dev is skipped, so the catalogs track models.dev.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde::Deserialize;

const CATALOG_URL: &str = "https://models.dev/catalog.json";

/// Coding families per provider — the lineages the providers' own coding tools
/// use (Codex / Claude Code / Gemini CLI). The generator keeps the latest model
/// of each. Researched from the providers' model docs, June 2026; notably Codex
/// uses `gpt-codex-spark` (not the deprecated `gpt-codex`).
const CODING_FAMILIES: &[(&str, &str)] = &[
    ("anthropic", "claude-opus"),
    ("anthropic", "claude-sonnet"),
    ("anthropic", "claude-haiku"),
    ("anthropic", "claude-fable"),
    ("openai", "gpt"),
    ("openai", "gpt-mini"),
    ("openai", "gpt-codex-spark"),
    ("google", "gemini-pro"),
    ("google", "gemini-flash"),
];

/// Curated providers, with the base URL to use when models.dev `api` is null.
const CURATED: &[(&str, &str)] = &[
    ("anthropic", "https://api.anthropic.com"),
    ("openai", "https://api.openai.com/v1"),
    ("google", "https://generativelanguage.googleapis.com/v1beta"),
];

#[derive(Deserialize)]
struct Catalog {
    providers: BTreeMap<String, ProviderJson>,
}

#[derive(Deserialize)]
struct ProviderJson {
    name: String,
    #[serde(default)]
    env: Vec<String>,
    #[serde(default)]
    api: Option<String>,
    #[serde(default)]
    models: BTreeMap<String, ModelJson>,
}

#[derive(Deserialize)]
struct ModelJson {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    tool_call: bool,
    #[serde(default)]
    family: Option<String>,
    #[serde(default)]
    limit: Limit,
    #[serde(default)]
    modalities: Modalities,
    #[serde(default)]
    release_date: Option<String>,
    #[serde(default)]
    last_updated: Option<String>,
}

#[derive(Deserialize, Default)]
struct Limit {
    #[serde(default)]
    context: u32,
    #[serde(default)]
    output: u32,
}

#[derive(Deserialize, Default)]
struct Modalities {
    #[serde(default)]
    output: Vec<String>,
}

fn month_ordinal(date: &str) -> Option<u32> {
    let year: u32 = date.get(0..4)?.parse().ok()?;
    let month: u32 = date.get(5..7)?.parse().ok()?;
    (1..=12).contains(&month).then_some(year * 12 + (month - 1))
}

fn date_ordinal(m: &ModelJson) -> Option<u32> {
    m.release_date
        .as_deref()
        .or(m.last_updated.as_deref())
        .and_then(month_ordinal)
}

fn is_dated_snapshot(id: &str) -> bool {
    matches!(id.rsplit_once('-'), Some((_, tail)) if tail.len() == 8 && tail.bytes().all(|b| b.is_ascii_digit()))
}

fn outputs_image(m: &ModelJson) -> bool {
    m.modalities.output.iter().any(|o| o.as_str() == "image")
}

/// Display name without models.dev's `(latest)` alias marker.
fn clean_name(name: &str) -> String {
    name.trim_end_matches(" (latest)").trim().to_owned()
}

/// A model kept for a category, ready to emit.
struct Kept {
    provider: &'static str,
    id: String,
    name: String,
    context: u32,
    max_output: u32,
}

impl Kept {
    fn of(provider: &'static str, id: &str, m: &ModelJson) -> Self {
        Kept {
            provider,
            id: id.to_owned(),
            name: clean_name(m.name.as_deref().unwrap_or(id)),
            context: m.limit.context,
            max_output: m.limit.output,
        }
    }
}

/// Coding: for each coding family (those the providers' coding tools use), the
/// latest tool-capable text model, preferring the clean alias over a snapshot.
fn coding_models(catalog: &Catalog) -> Vec<Kept> {
    struct Cand {
        id: String,
        name: String,
        context: u32,
        max_output: u32,
        ordinal: u32,
        dated: bool,
    }

    let mut by_family: BTreeMap<(&'static str, String), Vec<Cand>> = BTreeMap::new();
    for &(id, _) in CURATED {
        let Some(p) = catalog.providers.get(id) else {
            continue;
        };
        for (mid, m) in &p.models {
            if !m.tool_call || outputs_image(m) {
                continue;
            }
            let Some(family) = m.family.as_deref() else {
                continue;
            };
            if !CODING_FAMILIES.contains(&(id, family)) {
                continue;
            }
            let Some(ordinal) = date_ordinal(m) else {
                continue;
            };
            by_family
                .entry((id, family.to_owned()))
                .or_default()
                .push(Cand {
                    id: mid.clone(),
                    name: clean_name(m.name.as_deref().unwrap_or(mid)),
                    context: m.limit.context,
                    max_output: m.limit.output,
                    ordinal,
                    dated: is_dated_snapshot(mid),
                });
        }
    }

    let mut winners: Vec<(&'static str, Cand)> = by_family
        .into_iter()
        .map(|((provider, _family), cands)| {
            let best = cands
                .into_iter()
                .max_by_key(|c| (c.ordinal, !c.dated, Reverse(c.id.len())))
                .expect("family has at least one candidate");
            (provider, best)
        })
        .collect();
    winners.sort_by(|a, b| (a.0, &a.1.id).cmp(&(b.0, &b.1.id)));

    winners
        .into_iter()
        .map(|(provider, c)| Kept {
            provider,
            id: c.id,
            name: c.name,
            context: c.context,
            max_output: c.max_output,
        })
        .collect()
}

/// Image: every model of a curated provider that outputs images.
fn image_models(catalog: &Catalog) -> Vec<Kept> {
    let mut kept = Vec::new();
    for &(id, _) in CURATED {
        let Some(p) = catalog.providers.get(id) else {
            continue;
        };
        for (mid, m) in &p.models {
            if outputs_image(m) {
                kept.push(Kept::of(id, mid, m));
            }
        }
    }
    kept
}

/// All: every tool-capable model of a curated provider (no recency filter).
fn all_models(catalog: &Catalog) -> Vec<Kept> {
    let mut kept = Vec::new();
    for &(id, _) in CURATED {
        let Some(p) = catalog.providers.get(id) else {
            continue;
        };
        for (mid, m) in &p.models {
            if m.tool_call {
                kept.push(Kept::of(id, mid, m));
            }
        }
    }
    kept
}

fn write_category(catalog: &Catalog, name: &str, kept: &[Kept]) {
    let used: BTreeSet<&str> = kept.iter().map(|k| k.provider).collect();

    let mut providers = String::new();
    for &(id, default_base) in CURATED {
        if !used.contains(id) {
            continue;
        }
        let p = &catalog.providers[id];
        let base_url = p.api.as_deref().unwrap_or(default_base);
        let env = p
            .env
            .iter()
            .map(|e| format!("{e:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            providers,
            "    Provider {{ id: {id:?}, name: {:?}, base_url: {base_url:?}, env: &[{env}] }},",
            p.name,
        )
        .unwrap();
    }

    let mut models = String::new();
    for k in kept {
        writeln!(
            models,
            "    Model {{ provider: {:?}, id: {:?}, name: {:?}, context: {}, max_output: {} }},",
            k.provider, k.id, k.name, k.context, k.max_output,
        )
        .unwrap();
    }

    let out = format!(
        "// @generated by mei-codegen ({name}) from {CATALOG_URL} — do not edit by hand.\n\
         use super::{{Model, Provider}};\n\n\
         pub(super) static PROVIDERS: &[Provider] = &[\n{providers}];\n\n\
         pub(super) static MODELS: &[Model] = &[\n{models}];\n"
    );
    let path = format!("{}/src/catalog/{name}.rs", env!("CARGO_MANIFEST_DIR"));
    std::fs::write(&path, out).expect("write category file");
    eprintln!("{name}: {} providers, {} models", used.len(), kept.len());
}

fn main() {
    let body = ureq::get(CATALOG_URL)
        .call()
        .expect("fetch catalog.json")
        .into_string()
        .expect("read catalog.json body");
    let catalog: Catalog = serde_json::from_str(&body).expect("parse catalog.json");

    write_category(&catalog, "coding", &coding_models(&catalog));
    write_category(&catalog, "image", &image_models(&catalog));
    write_category(&catalog, "all", &all_models(&catalog));
}
