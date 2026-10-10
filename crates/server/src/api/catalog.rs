//! The card catalog the server checks loadouts and queue requests against (SPEC §9.4: "catalog is
//! static, versioned, shipped with the client; stale catalog version is rejected at save and
//! queue").
//!
//! It is `crates/cards/catalog.json`, compiled into the server through `jackioh_cards`
//! (`catalog_json()`), already a `defId -> CardDef` map, so a deployment cannot ship a server and a
//! catalog that disagree. Everything downstream depends only on `Catalog`, which `App.catalog` holds:
//! the defs, the version and the deployed commit `GET /api/catalog` reports.
//!
//! Not ported (SURFACE §11.3): `GET /api/catalog/:version`, which the client does not use.

use std::sync::Arc;

use axum::body::Body;
use axum::response::Response;
use jackioh_engine::{CardDef, CardDefs, Tag, set_ships};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::api::http::{ApiResult, Req};
use crate::app::App;

/// The patch list (R388, B4.2): `patches.json`, the list of patches in the order they were made.
/// Compiled in from the cards crate's copy, like the catalog itself.
const PATCHES_JSON: &str = include_str!("../../../cards/patches/patches.json");

/// Where the compiled-in catalog came from, for the error messages.
const CATALOG_SOURCE: &str = "crates/cards/catalog.json";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "the card catalog could not be read from {location}: {cause}. It is the 100 cards plus 9 tokens of SPEC §8; the server will not invent them."
)]
pub struct CatalogUnavailableError {
    pub location: String,
    pub cause: String,
}

impl CatalogUnavailableError {
    pub fn new(location: &str, cause: impl std::fmt::Display) -> CatalogUnavailableError {
        CatalogUnavailableError {
            location: location.to_string(),
            cause: cause.to_string(),
        }
    }
}

/// The catalog with the git commit this deploy runs, so `App` holds both in one place (SURFACE §11.2).
#[derive(Clone, Debug, PartialEq)]
pub struct Catalog {
    /// The version the client must match at save and at queue (§9.4).
    pub version: String,
    pub defs: CardDefs,
    pub card_ids: Vec<String>,
    /// `env.deployed_commit`: reported by `GET /api/catalog` in `x-deployed-commit`.
    pub commit: Option<String>,
    /// R164: the ban list, server state beside the catalog data. Empty: nothing is banned at launch.
    pub banned: Vec<String>,
    pub defs_json: String,
}

impl Catalog {
    pub fn is_token(&self, card_id: &str) -> bool {
        match self.defs.get(card_id) {
            None => false,
            Some(def) => def.token || def.tags.contains(&Tag::Token),
        }
    }

    // §9.4 L6: "every card exists in the current catalog version and is not banned".
    //
    // Proposed ruling for §11: a ban is server state, not catalog data (nothing in §8 is banned at
    // launch). A flag on the card would make
    // banning one card a new R105 version, and §9.4's stale-version rejection would then invalidate
    // every saved loadout at once. The shared validator reads bannedness through `Catalog`, never off a
    // `CardDef`; the list is empty until there is something to ban.
    //   Affects: §9.4 (L6), R105.
    pub fn is_banned(&self, card_id: &str) -> bool {
        self.banned.iter().any(|banned| banned == card_id)
    }

    pub fn defs_json(&self) -> &str {
        &self.defs_json
    }
}

fn is_card_def(value: &Value) -> bool {
    let Some(def) = value.as_object() else {
        return false;
    };
    def.get("id").is_some_and(Value::is_string)
        && def.get("name").is_some_and(Value::is_string)
        && def.get("tags").is_some_and(Value::is_array)
        && def.get("base").is_some_and(Value::is_object)
}

/// The FALLBACK version, for a caller that configures none (R105: an opaque string, compared for
/// equality only). `CATALOG_VERSION` is a required variable (`env.rs`) and `app.rs` always passes it;
/// this hash serves a direct `load_catalog` with no version (tests, tooling). Its `c1-` prefix cannot
/// collide with a configured one, and being derived from the bytes it cannot drift from them.
pub fn version_of(json: &str) -> String {
    let digest = Sha256::digest(json.as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("c1-{}", &hex[..12])
}

/// `Catalog` over a `defId -> CardDef` map.
pub fn catalog_from(defs: CardDefs, version: &str) -> Catalog {
    let card_ids = defs.keys().cloned().collect();
    // CardDef's serde shape is the catalog's own (field order and absences included).
    let defs_json =
        serde_json::to_string(&defs).unwrap_or_else(|error| panic!("CardDefs serialise: {error}"));
    Catalog {
        version: version.to_string(),
        defs,
        card_ids,
        commit: None,
        banned: Vec::new(),
        defs_json,
    }
}

/// `json` is the catalog text to read instead of the compiled-in `catalog.json` (tests hand it a
/// broken one).
#[derive(Clone, Debug, Default)]
pub struct LoadCatalogOptions {
    pub version: Option<String>,
    pub json: Option<String>,
}

/// The catalog, refused loudly when it is not one.
pub async fn load_catalog(options: LoadCatalogOptions) -> Result<Catalog, CatalogUnavailableError> {
    let json = options
        .json
        .unwrap_or_else(|| jackioh_cards::catalog_json().to_string());

    // Read as an ordered map first, so a bad entry is named and the catalog's own order survives.
    let parsed: Value =
        serde_json::from_str(&json).map_err(|cause| CatalogUnavailableError::new(CATALOG_SOURCE, cause))?;
    if !parsed.is_object() {
        return Err(CatalogUnavailableError::new(
            CATALOG_SOURCE,
            "expected a defId -> CardDef object",
        ));
    }
    let entries: indexmap::IndexMap<String, Value> =
        serde_json::from_str(&json).map_err(|cause| CatalogUnavailableError::new(CATALOG_SOURCE, cause))?;
    if entries.is_empty() {
        return Err(CatalogUnavailableError::new(
            CATALOG_SOURCE,
            "the catalog is empty",
        ));
    }
    let mut defs = CardDefs::new();
    for (key, value) in entries {
        if !is_card_def(&value) {
            return Err(CatalogUnavailableError::new(
                CATALOG_SOURCE,
                format!("\"{key}\" is not a CardDef"),
            ));
        }
        let def: CardDef = serde_json::from_value(value).map_err(|_| {
            CatalogUnavailableError::new(CATALOG_SOURCE, format!("\"{key}\" is not a CardDef"))
        })?;
        // R1420: a set the catalog holds that has not shipped is no card the server knows of: it is
        // in no deck, no statistic and no `GET /api/catalog` until `SHIPPED_SETS` lists it.
        if !set_ships(def.set) {
            continue;
        }
        defs.insert(key, def);
    }

    let version = options.version.unwrap_or_else(|| version_of(&json));
    Ok(catalog_from(defs, &version))
}

// The patch (R376, R388)

fn patch_list() -> Result<Value, String> {
    serde_json::from_str(PATCHES_JSON)
        .map_err(|cause| format!("the patch list could not be read from patches.json: {cause}"))
}

/// All patch versions from patches.json in release order.
pub async fn load_patch_versions() -> Vec<String> {
    let Ok(Value::Array(list)) = patch_list() else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|entry| entry.get("version").and_then(Value::as_str))
        .filter(|version| !version.is_empty())
        .map(str::to_string)
        .collect()
}

// Routes

/// The response header `GET /api/catalog` carries the deployed git commit in, when it is known.
pub const DEPLOYED_COMMIT_HEADER: &str = "x-deployed-commit";

/// `GET /api/catalog`.
///
/// Proposed ruling for §11: until the client ships a catalog, the server serves the same bytes here,
/// whole and unprojected. Whole, because §9.4 requires "one validator module shared by client and
/// server" and a trimmed card would be a second, weaker copy of the catalog. Unauthenticated: it is
/// the same bytes for everybody and names no profile, and §9.4's gate is about "no collection,
/// loadout, queue or match". It carries R105's version, so a stale client learns it is stale before
/// it builds a deck. The endpoint is the transport, never a second source of truth.
///   Affects: §9.1, §9.4, R105.
///
/// `apps/web/src/net/api.ts` already calls exactly this shape:
///
/// ```text
/// export type CatalogResponse = { version: string; defs: CardDefs };
/// ```
///
/// `commit` is the git commit this deploy runs (`env.rs` deployed_commit), sent as a header so the
/// body stays the same bytes for everybody (R163). `deploy-watch.yml` reads it, and Render's health
/// check probes this route too.
pub async fn get_catalog(app: &Arc<App>, _req: Req) -> ApiResult {
    let catalog = &app.catalog;
    let version = serde_json::to_string(&catalog.version).unwrap_or_else(|_| "\"\"".to_string());
    let body = format!("{{\"version\":{version},\"defs\":{}}}", catalog.defs_json());
    let mut builder = Response::builder()
        .status(200)
        .header("content-type", "application/json; charset=utf-8")
        .header("cache-control", "no-store");
    if let Some(commit) = &catalog.commit {
        builder = builder.header(DEPLOYED_COMMIT_HEADER, commit.as_str());
    }
    Ok(builder
        .body(Body::from(body))
        .unwrap_or_else(|error| panic!("a catalog response: {error}")))
}
