//! The collection: SPEC §9.4's entitlement ledger (`collection` and append-only `collection_grants`).
//!
//! Every collection change writes both tables in one transaction (BUILD M6-T2). The only mutation
//! path is `grant_cards` (and dev tools, R1300000); the client API exposes only `GET` (§9.8).
//! `collection_upsert_quantities` takes absolute quantities, while `CollectionGrant::delta`
//! records non-zero changes.

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::json;

use crate::api::http::{ApiError, ApiResult, Req, bad_request, json};
use crate::app::App;
use crate::db::store::{CollectionEntry, CollectionGrant, Profile, StoreError, Tx};

/// SPEC §11 R111, R1310000, R141: launch/starter grant copy count (1 copy per granted card).
/// Public because the production database trigger on activation (`0002_collection.sql`) and
/// fake store mirror this setting.
pub const LAUNCH_COPIES: i64 = 1;

/// Not in SPEC, and no R-row: forced. `collection_grants.reason` has a closed-set CHECK in
/// migration `0002_collection.sql` (`pack | craft | reward | refund | admin | launch`), and that
/// migration's own comment cites R111 for why a launch grant differs from a later `admin` fix.
pub const LAUNCH_GRANT_REASON: &str = "launch";

/// Input to `grant_cards`.
#[derive(Clone, Debug, PartialEq)]
pub struct GrantInput {
    pub profile_id: String,
    /// Each entry's `quantity` is a positive delta: how many copies to add.
    pub entries: Vec<CollectionEntry>,
    /// Why, for the append-only ledger (§9.4).
    pub reason: String,
}

/// The caller behind a route that declares `AuthLevel::Active`; `api::http::dispatch` has already
/// resolved and gated the profile (`assert_active`), so this only satisfies the type checker.
pub fn caller_profile(req: &Req) -> Result<Profile, ApiError> {
    match &req.caller {
        Some(caller) => Ok(caller.profile.clone()),
        None => Err(bad_request("this endpoint needs a signed-in profile")),
    }
}

/// Sums repeated ids and rejects non-positive deltas, turning table constraint violations
/// into a 400 (§9.4).
fn deltas_from(entries: &[CollectionEntry]) -> Result<IndexMap<String, i64>, ApiError> {
    let mut deltas: IndexMap<String, i64> = IndexMap::new();
    for entry in entries {
        if entry.card_id.is_empty() {
            return Err(bad_request("a collection entry needs a card id"));
        }
        if entry.quantity <= 0 {
            return Err(bad_request(format!(
                "the grant for \"{}\" must be a positive whole number of copies",
                entry.card_id
            )));
        }
        *deltas.entry(entry.card_id.clone()).or_insert(0) += entry.quantity;
    }
    Ok(deltas)
}

/// §9.4's one collection mutation: reads the current quantities, adds the deltas and writes both
/// `collection` and `collection_grants` inside a single transaction. If either write fails, the
/// transaction rolls back (a `Tx` dropped without `commit`) and neither table moves.
pub async fn grant_cards(app: &App, input: GrantInput) -> Result<(), ApiError> {
    let deltas = deltas_from(&input.entries)?;
    // No delta, no write: `collection_grants.delta <> 0` forbids an empty audit row.
    if deltas.is_empty() {
        return Ok(());
    }

    let at = crate::app::now_ms();
    let mut tx = app.db.begin(Some(&input.profile_id)).await?;
    let current = owned_in(&mut tx, &input.profile_id).await?;
    let mut quantities: Vec<CollectionEntry> = Vec::new();
    let mut grants: Vec<CollectionGrant> = Vec::new();
    for (card_id, delta) in &deltas {
        quantities.push(CollectionEntry {
            card_id: card_id.clone(),
            quantity: current.get(card_id).copied().unwrap_or(0) + delta,
        });
        grants.push(CollectionGrant {
            profile_id: input.profile_id.clone(),
            card_id: card_id.clone(),
            delta: *delta,
            reason: input.reason.clone(),
            dev: false,
            at,
        });
    }
    tx.collection_upsert_quantities(&input.profile_id, &quantities)
        .await?;
    tx.collection_append_grants(&grants).await?;
    tx.commit().await?;

    tracing::info!(
        event = "collection.granted",
        "profileId" = %input.profile_id,
        cards = deltas.len(),
        reason = %input.reason,
    );
    Ok(())
}

/// BUILD M6-T2, §9.4 L3/L6, R1310000, R1310004: grants 1 copy of playable, unbanned catalog cards
/// (used for e2e fixtures and tests). Idempotent top-up to `LAUNCH_COPIES`.
pub async fn grant_entire_catalog(app: &App, profile_id: &str, reason: Option<&str>) -> Result<(), ApiError> {
    let owned = owned_map(app, profile_id).await?;
    let mut entries: Vec<CollectionEntry> = Vec::new();
    for card_id in &app.catalog.card_ids {
        if app.catalog.is_token(card_id) || app.catalog.is_banned(card_id) {
            continue;
        }
        let missing = LAUNCH_COPIES - owned.get(card_id).copied().unwrap_or(0);
        if missing > 0 {
            entries.push(CollectionEntry {
                card_id: card_id.clone(),
                quantity: missing,
            });
        }
    }
    if entries.is_empty() {
        return Ok(());
    }
    grant_cards(
        app,
        GrantInput {
            profile_id: profile_id.to_string(),
            entries,
            reason: reason.unwrap_or(LAUNCH_GRANT_REASON).to_string(),
        },
    )
    .await
}

/// Shared by `owned_map` and `grant_cards`, which needs the read inside its own transaction.
pub(crate) async fn owned_in(tx: &mut Tx<'_>, profile_id: &str) -> Result<IndexMap<String, i64>, StoreError> {
    let mut owned: IndexMap<String, i64> = IndexMap::new();
    for entry in tx.collection_get(profile_id).await? {
        *owned.entry(entry.card_id.clone()).or_insert(0) += entry.quantity;
    }
    Ok(owned)
}

/// cardId → quantity owned: the `owned` input L5 is checked against ("copies across the loadout
/// never exceed the quantity owned"). `decks.rs` builds the validator's input from here.
pub async fn owned_map(app: &App, profile_id: &str) -> Result<IndexMap<String, i64>, ApiError> {
    let mut tx = app.db.begin(Some(profile_id)).await?;
    let owned = owned_in(&mut tx, profile_id).await?;
    tx.commit().await?;
    Ok(owned)
}

/// `GET /api/collection` (`AuthLevel::Active`, §9.4). Read-only endpoint returning owned entries
/// and server's catalog version. Absent ids imply zero owned copies.
pub async fn get_collection(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let mut entries = tx.collection_get(&profile.id).await?;
    tx.commit().await?;
    // R1300000: a dev account's clear leaves its rows at 0, owned zero times like an absent id.
    entries.retain(|entry| entry.quantity > 0);
    entries.sort_by(|a, b| a.card_id.cmp(&b.card_id));
    let entries: Vec<serde_json::Value> = entries
        .iter()
        .map(|entry| json!({ "cardId": entry.card_id, "quantity": entry.quantity }))
        .collect();
    Ok(json(
        200,
        json!({ "catalogVersion": app.catalog.version, "entries": entries }),
    ))
}
