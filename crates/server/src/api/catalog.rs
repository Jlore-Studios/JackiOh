//! The card catalog the server checks loadouts and queue requests against (SPEC §9.4: "catalog is
//! static, versioned, shipped with the client; stale catalog version is rejected at save and
//! queue"). Port of `apps/server/src/api/catalog.ts`.
//!
//! It is `crates/cards/catalog.json`, compiled into the server through `jackioh_cards`
//! (`catalog_json()`), which is already a `defId -> CardDef` map — i.e. exactly `CardDefs`. TS read
//! the file at boot; Rust holds the same bytes from the build, so a deployment cannot ship a server
//! and a catalog that disagree.
//!
//! Everything downstream depends only on `Catalog` (TS `CatalogInfo`), which `App.catalog` holds:
//! the defs, the version and the deployed commit `GET /api/catalog` reports.
//!
//! Not ported (SURFACE §11.3): `GET /api/catalog/:version` (unused by the client) and the snapshot
//! reader behind it (`catalogUrl`, `patchesUrl`, `readSnapshot`, `catalogAtVersion`, the
//! `PATCH_VERSION` pattern and the snapshot cache).

use axum::body::Body;
use axum::response::Response;
use jackioh_engine::{CardDef, CardDefs, Tag};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::api::http::{ApiResult, Req};
use crate::app::App;

/// The patch list (R388, B4.2): `patches.json`, the list of patches in the order they were made.
/// Compiled in from the cards crate's copy, like the catalog itself.
const PATCHES_JSON: &str = include_str!("../../../cards/patches/patches.json");

/// Where the compiled-in catalog came from, for the error messages.
const CATALOG_SOURCE: &str = "crates/cards/catalog.json";

/// TS `CatalogUnavailableError`.
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
        CatalogUnavailableError { location: location.to_string(), cause: cause.to_string() }
    }
}

/// TS `CatalogInfo`, plus the git commit this deploy runs (TS `createCatalogRoutes({ commit })`),
/// so `App` holds both in one place (SURFACE §11.2).
#[derive(Clone, Debug, PartialEq)]
pub struct Catalog {
    /// The version the client must match at save and at queue (§9.4).
    pub version: String,
    pub defs: CardDefs,
    pub card_ids: Vec<String>,
    /// `env.deployed_commit`: reported by `GET /api/catalog` in `x-deployed-commit`.
    pub commit: Option<String>,
    /// `JSON.stringify(defs)`, once: `GET /api/catalog` serves the same bytes to everybody (R163).
    defs_json: String,
}

impl Catalog {
    /// TS `isToken`.
    pub fn is_token(&self, card_id: &str) -> bool {
        match self.defs.get(card_id) {
            None => false,
            Some(def) => def.token || def.tags.contains(&Tag::Token),
        }
    }

    // §9.4 L6: "every card exists in the current catalog version and is not banned".
    //
    // NOT IN SPEC, and no R-row yet — PROPOSED RULING for §11:
    //   Topic: Where L6's ban list lives
    //   Ruling: A ban is server state, not catalog data. Nothing in §8 is banned at launch and a
    //     `CardDef` carries no ban flag, so L6's two halves are answered from two places: catalog
    //     membership from the catalog both sides ship, and the ban list from the server alone. The
    //     alternative — a flag on the card — would put a ban inside the catalog data itself, so
    //     banning one card would mean a new R105 version, and §9.4's stale-version rejection would
    //     then turn every saved loadout in the game invalid at once. It would also hand the client
    //     a copy of a list it has no business being able to disagree with. The shared validator
    //     therefore reads bannedness through `Catalog` and never off a `CardDef`, and the list
    //     is empty until there is something to ban.
    //   Affects: §9.4 (L6), R105; `api/catalog.ts`, `packages/validator`.
    pub fn is_banned(&self, _card_id: &str) -> bool {
        false
    }

    /// `JSON.stringify(defs)`, as `GET /api/catalog` sends it.
    pub fn defs_json(&self) -> &str {
        &self.defs_json
    }
}

fn is_card_def(value: &Value) -> bool {
    let Some(def) = value.as_object() else { return false };
    def.get("id").is_some_and(Value::is_string)
        && def.get("name").is_some_and(Value::is_string)
        && def.get("tags").is_some_and(Value::is_array)
        && def.get("base").is_some_and(Value::is_object)
}

/// The FALLBACK version, for a caller that configures none. SPEC §11 R105 already fixes what a
/// catalog version is — "a short opaque string stamped on every `cards` row and mirrored in the
/// server's settings", compared for equality only, never parsed or ordered, and `core-1` for the
/// Core set — so nothing here re-decides it. `CATALOG_VERSION` is a required variable (`env.rs`)
/// and `app.rs` always passes it, which is the path R105 describes and the value migration
/// `0001_profiles_and_invites.sql` seeds into `app.settings`.
///
/// This hash exists only for a direct `load_catalog` with no version — tests and tooling. Its
/// `c1-` prefix cannot collide with a configured one, and being derived from the bytes it describes
/// it cannot drift from them. It is opaque and equality-compared like any other R105 version.
pub fn version_of(json: &str) -> String {
    let digest = Sha256::digest(json.as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("c1-{}", &hex[..12])
}

/// `Catalog` over a `defId -> CardDef` map.
pub fn catalog_from(defs: CardDefs, version: &str) -> Catalog {
    let card_ids = defs.keys().cloned().collect();
    // CardDef's serde shape is the catalog's own (field order and absences included), so this is
    // TS's `JSON.stringify` of the parsed file.
    let defs_json = serde_json::to_string(&defs).unwrap_or_else(|error| panic!("CardDefs serialise: {error}"));
    Catalog { version: version.to_string(), defs, card_ids, commit: None, defs_json }
}

/// TS `loadCatalog`'s options. `json` stands where TS's `url` stood: the catalog text to read
/// instead of the compiled-in `catalog.json` (tests hand it a broken one).
#[derive(Clone, Debug, Default)]
pub struct LoadCatalogOptions {
    pub version: Option<String>,
    pub json: Option<String>,
}

/// TS `loadCatalog`: the catalog, refused loudly when it is not one.
pub async fn load_catalog(options: LoadCatalogOptions) -> Result<Catalog, CatalogUnavailableError> {
    let json = options.json.unwrap_or_else(|| jackioh_cards::catalog_json().to_string());

    // Read as an ordered map first, so a bad entry is named (TS's `isCardDef` pass) and the
    // catalog's own order survives.
    let parsed: Value =
        serde_json::from_str(&json).map_err(|cause| CatalogUnavailableError::new(CATALOG_SOURCE, cause))?;
    if !parsed.is_object() {
        return Err(CatalogUnavailableError::new(CATALOG_SOURCE, "expected a defId -> CardDef object"));
    }
    let entries: indexmap::IndexMap<String, Value> =
        serde_json::from_str(&json).map_err(|cause| CatalogUnavailableError::new(CATALOG_SOURCE, cause))?;
    if entries.is_empty() {
        return Err(CatalogUnavailableError::new(CATALOG_SOURCE, "the catalog is empty"));
    }
    let mut defs = CardDefs::new();
    for (key, value) in entries {
        if !is_card_def(&value) {
            return Err(CatalogUnavailableError::new(CATALOG_SOURCE, format!("\"{key}\" is not a CardDef")));
        }
        let def: CardDef = serde_json::from_value(value)
            .map_err(|_| CatalogUnavailableError::new(CATALOG_SOURCE, format!("\"{key}\" is not a CardDef")))?;
        defs.insert(key, def);
    }

    let version = options.version.unwrap_or_else(|| version_of(&json));
    Ok(catalog_from(defs, &version))
}

// ---------------------------------------------------------------------------
// The patch (R376, R388)
// ---------------------------------------------------------------------------

fn patch_list() -> Result<Value, String> {
    serde_json::from_str(PATCHES_JSON).map_err(|cause| format!("the patch list could not be read from patches.json: {cause}"))
}

/// R376: the version every live game record is filed under — the newest patch of R388's list, which
/// is the cards this build plays (the newest snapshot always equals `catalog.json`). Read as data,
/// like the catalog above: the list's order is the order of versions, never a comparison of strings.
pub async fn load_current_patch() -> Result<String, String> {
    let parsed = patch_list()?;
    let version = parsed
        .as_array()
        .and_then(|list| list.last())
        .and_then(|newest| newest.get("version"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if version.is_empty() {
        return Err("the patch list at patches.json names no newest version (R388)".to_string());
    }
    Ok(version.to_string())
}

/// All patch versions from patches.json in release order.
pub async fn load_patch_versions() -> Vec<String> {
    let Ok(Value::Array(list)) = patch_list() else { return Vec::new() };
    list.iter()
        .filter_map(|entry| entry.get("version").and_then(Value::as_str))
        .filter(|version| !version.is_empty())
        .map(str::to_string)
        .collect()
}

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

/// The response header `GET /api/catalog` carries the deployed git commit in, when it is known.
pub const DEPLOYED_COMMIT_HEADER: &str = "x-deployed-commit";

/// `GET /api/catalog`.
///
/// NOT IN SPEC, and no R-row yet — PROPOSED RULING for §11:
///   Topic: The catalog a client that ships none can read
///   Ruling: §9.4's "static, versioned, shipped with the client" describes the end state, and until
///     the client ships one the server serves the same bytes from `GET /api/catalog`, whole and
///     unprojected. Whole, because §9.4 requires "one validator module shared by client and
///     server" and `@jackioh/validator`'s `CatalogSnapshot.cards` *is* a `CardDefs`: a trimmed card
///     would be a second, weaker copy of the catalog, and the deckbuilder's verdict (UX) would stop
///     being the verdict the save runs (law). Unauthenticated, like the file it stands in for: it
///     is the same bytes for everybody, it names no profile, and §9.4's gate on a pending account
///     is about "no collection, loadout, queue or match" — card art is none of those. The endpoint
///     carries R105's version, so a stale client learns it is stale before it builds a deck rather
///     than at save time. The endpoint is the transport, never a second source of truth: the day
///     the client ships its own catalog this route may go away without a rule changing.
///   Affects: §9.1, §9.4, R105; `api/catalog.ts`, `apps/web/src/net/api.ts`, `packages/validator`.
///
/// `apps/web/src/net/api.ts` already calls exactly this shape:
///
///     export type CatalogResponse = { version: string; defs: CardDefs };
///
/// `commit` is the git commit this deploy runs (`env.rs` deployed_commit), sent as a header so the
/// body stays the same bytes for everybody (R163). `deploy-watch.yml` reads it to tell a server that
/// Render redeployed from one it left alone. Render's health check probes this route too.
pub async fn get_catalog(app: &App, _req: Req) -> ApiResult {
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
    Ok(builder.body(Body::from(body)).unwrap_or_else(|error| panic!("a catalog response: {error}")))
}
