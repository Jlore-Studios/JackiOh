//! The collection: SPEC §9.4's entitlement ledger. Port of `apps/server/src/api/collection.ts`.
//!
//! "Collection is an entitlement ledger (`collection` plus append-only `collection_grants`) ...
//! Every collection change writes `collection` and `collection_grants` in one transaction, and no
//! client path writes either."
//!
//! Two consequences shape this file.
//!
//! 1. There is exactly one mutation path, `grant_cards`, and it makes both writes inside one
//!    transaction. The store exposes `collection_upsert_quantities` and `collection_append_grants`
//!    separately, so nothing stops a caller from making one without the other — which is why every
//!    caller goes through here instead, and why the fault-injection test asserts "both tables or
//!    neither" for a fault on either write (BUILD M6-T2).
//! 2. The route table holds a single `GET`. §9.8's first row ("Claiming unowned cards → the
//!    collection is server-owned") and M6-T2's acceptance item ("a direct insert attempt through
//!    the public API is impossible — no endpoint") are both satisfied by an absence, so the test
//!    asserts over the route table itself (`app::ROUTES`): a mutating route cannot be added without
//!    a test going red.
//!
//! The ledger's two writes mean two different numbers. `collection_upsert_quantities` takes the new
//! *absolute* quantity for each card (that is the projection the client reads), while a
//! `CollectionGrant`'s `delta` is the change this grant applied — the db agent's
//! `collection_grants.delta` column, which carries `check (delta <> 0)`. So a grant that would
//! change nothing writes no row at all.

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::json;

use crate::api::http::{ApiError, ApiResult, Req, bad_request, json};
use crate::app::App;
use crate::db::store::{CollectionEntry, CollectionGrant, Profile, StoreError, Tx};

/// SPEC §11 R111 fixes this: the launch grant is "one copy of every non-token card". The reason the
/// row gives for one — L3 caps a deck at `MAX_COPIES` and L4 forbids a card id in two decks, so one
/// copy already builds all three decks legally and a second would be unreachable — is R111's, not
/// this file's, and R141 then leans on it ("L5 is never a sole failure"). Nothing here re-derives
/// the quantity; this is the Rust half of a value SPEC already owns.
///
/// The db agent stores the same value as `app.settings('launch_quantity') = 1` in migration
/// `0002_collection.sql`, which cites R111 in the same words.
///
/// Public because R111's grant is a *database trigger* in production ("written by a trigger on
/// the `pending → active` transition"), so the end-to-end mode's in-memory store has to carry the
/// same trigger — see `db/fake.rs`. It reads this value rather than restating it, so the
/// application path and the trigger path cannot drift.
pub const LAUNCH_COPIES: i64 = 1;

/// Not in SPEC, and no R-row: this string is forced, not decided. `collection_grants.reason` carries
/// a closed-set CHECK constraint in migration `0002_collection.sql`
/// (`pack | craft | reward | refund | admin | launch`), so the launch path must use `"launch"` or
/// every insert fails at runtime, and that migration's own comment already cites R111 for why the
/// ledger distinguishes a launch grant from a later `admin` correction.
pub const LAUNCH_GRANT_REASON: &str = "launch";

/// TS `GrantInput`.
#[derive(Clone, Debug, PartialEq)]
pub struct GrantInput {
    pub profile_id: String,
    /// Each entry's `quantity` is a positive delta: how many copies to add.
    pub entries: Vec<CollectionEntry>,
    /// Why, for the append-only ledger (§9.4).
    pub reason: String,
}

/// The caller behind a route that declares `AuthLevel::Active`. `api::http::dispatch` has already
/// resolved and gated the profile by then (`assert_active`), so this only re-states that for the
/// type checker. Every handler that needs the caller's profile reads it through here.
pub fn caller_profile(req: &Req) -> Result<Profile, ApiError> {
    match &req.caller {
        Some(caller) => Ok(caller.profile.clone()),
        None => Err(bad_request("this endpoint needs a signed-in profile")),
    }
}

/// Sums repeated ids and rejects a delta that is not a positive whole number.
///
/// Not in SPEC, and no R-row: an input-shape check that mirrors a table constraint. §9.4 describes
/// grants and never revocation, and `collection.quantity >= 0` plus `collection_grants.delta <> 0`
/// are constraints migration `0002_collection.sql` already enforces, so a zero or negative delta
/// has no meaning this function could give it — rejecting it here only turns a constraint violation
/// into a named 400. Revocation, if it ever lands, is its own path with its own `reason`, and that
/// would be a ruling; this is not.
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
    // No delta, no write: `collection_grants.delta <> 0` forbids an empty audit row, and an empty
    // transaction would be a lie in the ledger.
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
            at,
        });
    }
    tx.collection_upsert_quantities(&input.profile_id, &quantities).await?;
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

/// BUILD M6-T2: "launch mode grants every card to every active profile." One copy of every
/// playable id in the catalog — tokens are skipped (§9.4 L3 bans them from a deck, so owning one
/// would be meaningless) and so are banned ids (L6).
///
/// Idempotent by topping up to `LAUNCH_COPIES` rather than adding: a second call computes a delta
/// of zero for everything already owned, those entries drop out, and neither table is touched.
/// `reason` is `LAUNCH_GRANT_REASON` unless the caller names another.
pub async fn grant_entire_catalog(app: &App, profile_id: &str, reason: Option<&str>) -> Result<(), ApiError> {
    let owned = owned_map(app, profile_id).await?;
    let mut entries: Vec<CollectionEntry> = Vec::new();
    for card_id in &app.catalog.card_ids {
        if app.catalog.is_token(card_id) || app.catalog.is_banned(card_id) {
            continue;
        }
        let missing = LAUNCH_COPIES - owned.get(card_id).copied().unwrap_or(0);
        if missing > 0 {
            entries.push(CollectionEntry { card_id: card_id.clone(), quantity: missing });
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

/// cardId → quantity owned. This is the `owned` input L5 is checked against ("copies across the
/// loadout never exceed the quantity owned"), so `decks.rs` builds the validator's input from
/// here and never reads `collection` itself.
pub async fn owned_map(app: &App, profile_id: &str) -> Result<IndexMap<String, i64>, ApiError> {
    let mut tx = app.db.begin(Some(profile_id)).await?;
    let owned = owned_in(&mut tx, profile_id).await?;
    tx.commit().await?;
    Ok(owned)
}

/// `GET /api/collection` (`AuthLevel::Active`).
///
/// §9.4: read-only, and deliberately the whole route table. There is no POST, PUT, PATCH or
/// DELETE: the ledger moves only through `grant_cards`, which no request reaches.
///
/// The response carries the catalog version the server is holding, because the client's catalog is
/// "static, versioned, shipped with the client" (§9.4) and a client one release behind needs to
/// find that out before it builds a loadout, not at save time. Only owned rows are sent; an id the
/// client does not see here is owned zero times.
pub async fn get_collection(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let mut entries = tx.collection_get(&profile.id).await?;
    tx.commit().await?;
    entries.sort_by(|a, b| a.card_id.cmp(&b.card_id));
    let entries: Vec<serde_json::Value> = entries
        .iter()
        .map(|entry| json!({ "cardId": entry.card_id, "quantity": entry.quantity }))
        .collect();
    Ok(json(200, json!({ "catalogVersion": app.catalog.version, "entries": entries })))
}
