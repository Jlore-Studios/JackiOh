//! Saved decks and trios (SPEC §9.4, R250–R256), and what a queue ticket or a room is made with
//! (R253, R257, R264). Port of `apps/server/src/api/decks.ts`.
//!
//! This file replaces the single three-deck loadout. A profile keeps up to `MAX_SAVED_DECKS` named
//! decks and up to `MAX_SAVED_TRIOS` trios built from them, and both are DRAFTS (R250, R252): a
//! save checks structure only — D1–D5 for a deck (R641 added the portrait's), T1–T3 for a trio —
//! and legality is judged when a deck or a trio is queued (R253). So the save routes below never
//! call the L1–L6 validator, and the queue-time helpers at the bottom always do.
//!
//! No rule is written here. D1–D5, T1–T3, `normalize_name` and L1–L6 are
//! `jackioh_engine::validator`'s, called directly (TS reached them through `loadout-validator.ts`
//! and the `LoadoutValidator` port, neither of which is ported, SURFACE §11.3). Every refusal passes
//! the shared module's issues through untouched as `details`, with the first one's sentence as the
//! message, so the deck builder and the server say the same words (§9.4: "one validator module
//! shared by client and server").
//!
//! Ids are minted by the client (`crypto.randomUUID()`), which is what makes `PUT` an idempotent
//! upsert a dropped connection can simply retry (R256). The store decides create-or-update, the
//! cap and ownership in one statement under a lock on the profile (`app.upsert_deck`,
//! `app.upsert_trio`), so nothing here reads before it writes.
//!
//! R257's legacy queue body (no `mode`, or a `deckIndex` in place of `deckId`) is not ported
//! (SURFACE §11.3): `read_mode_choice` requires `mode`.

use std::sync::Arc;

use indexmap::{IndexMap, IndexSet};
use serde::Serialize;
use serde_json::{Map, Value, json};

use jackioh_engine::validator;
use jackioh_engine::wire::emotes::is_portrait_id;

use crate::api::collection::{caller_profile, owned_in};
use crate::api::http::{
    ApiError, ApiErrorCode, ApiResult, Req, bad_request, log_info, now_ms, ok_of, optional_str, str, string_list,
    to_json,
};
use crate::app::App;
use crate::config::{DECK_NAME_MAX_LENGTH, DRAFT_ISSUES_REPORTED_MAX, MAX_SAVED_DECKS, MAX_SAVED_TRIOS};
use crate::db::store::{
    FrozenDeck, FrozenTrio, SavedDeck, SavedTrio, TrioSlots, TrioUpsertOutcome, Tx, UpsertOutcome,
};

// ---------------------------------------------------------------------------
// Ids
// ---------------------------------------------------------------------------

/// A deck or trio id is a UUID (R256: the client mints it with `crypto.randomUUID()`). Checked
/// before any store call, because Postgres would answer a malformed one with a type error — a 500
/// for what is a malformed request. Case-insensitive on the way in and lower case from then on,
/// which is how Postgres prints a `uuid`, so both stores hold the same string for the same id.
///
/// TS `/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/iu`.
fn is_uuid_shape(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => *byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        })
}

fn saved_id_of(raw: Option<&Value>) -> Option<String> {
    let text = raw?.as_str()?;
    if is_uuid_shape(text) { Some(text.to_ascii_lowercase()) } else { None }
}

fn path_id_of(raw: Option<&String>, what: &str) -> Result<String, ApiError> {
    match raw.filter(|text| is_uuid_shape(text)) {
        Some(text) => Ok(text.to_ascii_lowercase()),
        None => Err(bad_request(format!("that is not a {what} id"))),
    }
}

// ---------------------------------------------------------------------------
// What the client sees
// ---------------------------------------------------------------------------

/// `SavedDeck` in `apps/web/src/net/api.ts`: the row without its owner, who is the caller.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeckView {
    pub id: String,
    pub name: String,
    pub cards: Vec<String>,
    pub portrait: Option<String>,
    pub catalog_version: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// `SavedTrio` in `apps/web/src/net/api.ts`: `deckIds` is the three slots, each an id or null.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrioView {
    pub id: String,
    pub name: String,
    pub deck_ids: Vec<Option<String>>,
    pub created_at: i64,
    pub updated_at: i64,
}

fn deck_view(deck: &SavedDeck) -> DeckView {
    DeckView {
        id: deck.id.clone(),
        name: deck.name.clone(),
        cards: deck.cards.clone(),
        portrait: deck.portrait.clone(),
        catalog_version: deck.catalog_version.clone(),
        created_at: deck.created_at,
        updated_at: deck.updated_at,
    }
}

/// A trio's three slots, in slot order.
fn slots_in(slots: &TrioSlots) -> Vec<Option<String>> {
    vec![slots.0.clone(), slots.1.clone(), slots.2.clone()]
}

fn trio_view(trio: &SavedTrio) -> TrioView {
    TrioView {
        id: trio.id.clone(),
        name: trio.name.clone(),
        deck_ids: slots_in(&trio.deck_ids),
        created_at: trio.created_at,
        updated_at: trio.updated_at,
    }
}

/// The caps the builder lives under (`DecksResponse.limits`): R250's and R252's numbers.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeckLimits {
    pub decks: i64,
    pub trios: i64,
    pub name_length: i64,
}

pub const LIMITS: DeckLimits = DeckLimits {
    decks: MAX_SAVED_DECKS as i64,
    trios: MAX_SAVED_TRIOS as i64,
    name_length: DECK_NAME_MAX_LENGTH as i64,
};

/// Another profile's id is answered as if it did not exist (the store's `not_owner`), so an id
/// reveals nothing about anyone else's decks. With client-minted UUIDs no one can guess one anyway;
/// this keeps the answer honest if one ever leaks.
///
/// A save refused by D1–D4 or T1–T3: the first issue's sentence, and the issues in `details`, at
/// most `DRAFT_ISSUES_REPORTED_MAX` of them so a body of junk cannot buy an answer many times its
/// own size.
fn draft_refused(message: &str, issues: &[Value]) -> ApiError {
    let reported: Vec<Value> = issues.iter().take(DRAFT_ISSUES_REPORTED_MAX as usize).cloned().collect();
    ApiError::with_details(ApiErrorCode::BadRequest, message, Value::Array(reported))
}

fn not_found(what: &str) -> ApiError {
    ApiError::new(ApiErrorCode::NotFound, format!("There is no such {what}."))
}

/// The issues a validator check returned, as the JSON the client reads.
fn issues_json<T: Serialize>(issues: &T) -> Result<Vec<Value>, ApiError> {
    match to_json(issues)? {
        Value::Array(entries) => Ok(entries),
        _ => Err(ApiError::internal("a validator check answered something other than a list of issues")),
    }
}

fn first_message(issues: &[Value]) -> Option<String> {
    issues.first()?.get("message")?.as_str().map(str::to_string)
}

// ---------------------------------------------------------------------------
// Body readers
// ---------------------------------------------------------------------------

/// A name must be a string, and that is all this checks: an empty or over-long one is D1's (or
/// T1's) to refuse, in the shared module's own sentence, so `str` — which refuses an empty string
/// with a sentence of its own — is not used here.
fn name_of(body: &Value) -> Result<String, ApiError> {
    match body.get("name") {
        Some(Value::String(value)) => Ok(value.clone()),
        _ => Err(bad_request("\"name\" must be a string")),
    }
}

/// A trio's slots: deck ids or `null`. How many there are is T2's to judge.
fn slots_of(body: &Value) -> Result<Vec<Option<String>>, ApiError> {
    let bad = || bad_request("\"deckIds\" must be a list of deck ids or null");
    let value = body.get("deckIds").and_then(Value::as_array).ok_or_else(bad)?;
    value
        .iter()
        .map(|entry| {
            if entry.is_null() {
                return Ok(None);
            }
            saved_id_of(Some(entry)).map(Some).ok_or_else(bad)
        })
        .collect()
}

/// §9.4: the catalog is "static, versioned, shipped with the client", and R253 checks the version a
/// client sends at save: a builder one release behind would otherwise save ids it cannot know are
/// gone. Checked before D3, which would call those ids undeckable when the real problem is the
/// client. BUILD M6-T2's acceptance item is literally 'a stale `catalogVersion` gets "update
/// required"', so that is the message.
fn assert_current_catalog(app: &App, catalog_version: &str) -> Result<(), ApiError> {
    if catalog_version != app.catalog.version {
        return Err(ApiError::with_details(
            ApiErrorCode::UpdateRequired,
            "update required",
            json!({ "expected": app.catalog.version, "received": catalog_version }),
        ));
    }
    Ok(())
}

/// R250 D3, R251: a deckable card is one the current catalog has and that is not a Token.
fn deckable_in(app: &App) -> impl Fn(&str) -> bool + '_ {
    let known: IndexSet<&str> = app.catalog.card_ids.iter().map(String::as_str).collect();
    move |card_id: &str| known.contains(card_id) && !app.catalog.is_token(card_id)
}

/// R250's D1–D4 (and R641's D5 when `with_portrait`): the shared module's `check_deck_draft`.
fn deck_draft_issues(
    app: &App,
    name: &str,
    cards: &[String],
    portrait: Option<&str>,
    with_portrait: bool,
) -> Result<Vec<Value>, ApiError> {
    let is_deckable = deckable_in(app);
    let is_portrait = |id: &str| is_portrait_id(&Value::from(id));
    let issues = validator::check_deck_draft(&validator::DeckDraftInput {
        name: name.to_string(),
        cards: cards.to_vec(),
        is_deckable: &is_deckable,
        name_max_length: DECK_NAME_MAX_LENGTH as usize,
        portrait: portrait.map(str::to_string),
        is_portrait: if with_portrait { Some(&is_portrait) } else { None },
    });
    issues_json(&issues)
}

/// R252's T1–T3: the shared module's `check_trio_draft`.
fn trio_draft_issues(name: &str, deck_ids: &[Option<String>]) -> Result<Vec<Value>, ApiError> {
    let input: validator::TrioDraftInput =
        serde_json::from_value(json!({ "name": name, "deckIds": deck_ids, "nameMaxLength": DECK_NAME_MAX_LENGTH }))?;
    issues_json(&validator::check_trio_draft(&input))
}

/// T2 has passed, so there are exactly three slots.
fn trio_slots_of(deck_ids: &[Option<String>]) -> Result<TrioSlots, ApiError> {
    match deck_ids {
        [first, second, third] => Ok((first.clone(), second.clone(), third.clone())),
        _ => Err(ApiError::internal(format!("T2 passed a trio of {} slots", deck_ids.len()))),
    }
}

// ---------------------------------------------------------------------------
// A trio import (R340, R341)
// ---------------------------------------------------------------------------

/// One deck of an imported trio, as the body carries it: its client-minted id, name and cards.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ImportedDeckInput {
    id: String,
    name: String,
    cards: Vec<String>,
}

/// The imported trio's own id and name (TS's anonymous `trio: { id, name }`).
#[derive(Clone, Debug, PartialEq, Eq)]
struct ImportedTrioInput {
    id: String,
    name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TrioImportInput {
    catalog_version: String,
    trio: ImportedTrioInput,
    /// Slot by slot; none is a slot the code left empty.
    slots: Vec<Option<ImportedDeckInput>>,
}

const IMPORT_SHAPE: &str =
    "A trio import is { catalogVersion, trio: { id, name }, slots: [3 × ({ id, name, cards } or null)] }";

fn record_of(value: Option<&Value>) -> Option<&Map<String, Value>> {
    value?.as_object()
}

/// R341: the body, read without trusting any of it. Shapes are checked here (ids are UUIDs, names
/// are strings, cards are lists of strings, exactly `TRIO_DECKS` slots); what the shapes hold is the
/// draft rules' to judge, in their own words.
fn import_of(body: &Value) -> Result<TrioImportInput, ApiError> {
    let catalog_version = str(body, "catalogVersion")?;
    let trio = record_of(body.get("trio"));
    let trio_id = trio.and_then(|trio| saved_id_of(trio.get("id")));
    let trio_name = trio.and_then(|trio| trio.get("name")).and_then(Value::as_str);
    let (Some(trio_id), Some(trio_name)) = (trio_id, trio_name) else {
        return Err(bad_request(IMPORT_SHAPE));
    };
    let raw_slots = match body.get("slots").and_then(Value::as_array) {
        Some(slots) if slots.len() == validator::TRIO_DECKS as usize => slots,
        _ => return Err(bad_request(IMPORT_SHAPE)),
    };
    let slots = raw_slots
        .iter()
        .map(|raw| -> Result<Option<ImportedDeckInput>, ApiError> {
            if raw.is_null() {
                return Ok(None);
            }
            let deck = record_of(Some(raw));
            let id = deck.and_then(|deck| saved_id_of(deck.get("id")));
            let name = deck.and_then(|deck| deck.get("name")).and_then(Value::as_str);
            let (Some(id), Some(name)) = (id, name) else {
                return Err(bad_request(IMPORT_SHAPE));
            };
            Ok(Some(ImportedDeckInput { id, name: name.to_string(), cards: string_list(raw, "cards")? }))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TrioImportInput {
        catalog_version,
        trio: ImportedTrioInput { id: trio_id, name: trio_name.to_string() },
        slots,
    })
}

/// "Deck 2 (“Aggro”)": which of the code's decks a refusal is about.
fn imported_deck_label(slot: usize, name: &str) -> String {
    format!("Deck {} (“{}”)", slot + 1, name)
}

/// R340: the room check, in the words the workshop shows, against what `t` holds now. Ids the
/// caller already owns are a retry of an import that landed (R256's idempotent upsert): they take no
/// new slot.
async fn assert_import_room(
    t: &mut Tx<'_>,
    profile_id: &str,
    deck_ids: &[String],
    trio_id: &str,
) -> Result<(), ApiError> {
    let decks = t.decks_list(profile_id).await?;
    let trios = t.trios_list(profile_id).await?;
    let held_decks: IndexSet<&str> = decks.iter().map(|deck| deck.id.as_str()).collect();
    let adding_decks = deck_ids.iter().filter(|id| !held_decks.contains(id.as_str())).count();
    let adding_trios = if trios.iter().any(|trio| trio.id == trio_id) { 0 } else { 1 };
    let input: validator::ImportRoomInput = serde_json::from_value(json!({
        "saved": { "decks": decks.len(), "trios": trios.len() },
        "limits": { "decks": MAX_SAVED_DECKS, "trios": MAX_SAVED_TRIOS },
        "adding": { "decks": adding_decks, "trios": adding_trios },
    }))?;
    let room = to_json(&validator::check_import_room(&input))?;
    if room.get("ok").and_then(Value::as_bool) == Some(true) {
        return Ok(());
    }
    let message = room.get("message").and_then(Value::as_str).unwrap_or_default();
    Err(ApiError::with_details(
        ApiErrorCode::Conflict,
        format!("{message} Nothing was imported."),
        json!({
            "decksShort": room.get("decksShort").cloned().unwrap_or(Value::Null),
            "triosShort": room.get("triosShort").cloned().unwrap_or(Value::Null),
            "limits": { "decks": MAX_SAVED_DECKS, "trios": MAX_SAVED_TRIOS },
        }),
    ))
}

/// Returned inside the import's transaction to roll back every write made before it.
fn import_outcome_refused(what: &str, outcome: &str) -> ApiError {
    if outcome == "not_owner" {
        return not_found(what);
    }
    // A cap reached between the room check and the write: another tab or device saved meanwhile.
    ApiError::new(
        ApiErrorCode::Conflict,
        "Your decks changed while this import was on its way. Nothing was imported; try again.",
    )
}

fn upsert_outcome_name(outcome: &UpsertOutcome) -> &'static str {
    match outcome {
        UpsertOutcome::Created => "created",
        UpsertOutcome::Updated => "updated",
        UpsertOutcome::Limit => "limit",
        UpsertOutcome::NotOwner => "not_owner",
    }
}

fn trio_outcome_name(outcome: &TrioUpsertOutcome) -> &'static str {
    match outcome {
        TrioUpsertOutcome::Created => "created",
        TrioUpsertOutcome::Updated => "updated",
        TrioUpsertOutcome::Limit => "limit",
        TrioUpsertOutcome::NotOwner => "not_owner",
        TrioUpsertOutcome::UnknownDeck => "unknown_deck",
    }
}

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

// TS's `createDeckRoutes()`, in its order, is six rows of `app.rs`'s `ROUTES`, every one
// `AuthLevel::Active`, so a pending account gets 403 from each (§9.4: "no collection, loadout,
// queue or match"):
//   GET /api/decks → list_decks;           PUT /api/decks/:id → put_deck;
//   DELETE /api/decks/:id → delete_deck;   PUT /api/trios/:id → put_trio;
//   POST /api/trios/import → import_trio;  DELETE /api/trios/:id → delete_trio.

/// `GET /api/decks`. Everything the builder opens on, oldest first, with the server's catalog
/// version so a stale client finds out before it builds rather than at save.
pub async fn list_decks(app: &Arc<App>, req: Req) -> ApiResult {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct DecksBody {
        catalog_version: String,
        decks: Vec<DeckView>,
        trios: Vec<TrioView>,
        limits: DeckLimits,
    }
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let decks = tx.decks_list(&profile.id).await?;
    let trios = tx.trios_list(&profile.id).await?;
    tx.commit().await?;
    ok_of(&DecksBody {
        catalog_version: app.catalog.version.clone(),
        decks: decks.iter().map(deck_view).collect(),
        trios: trios.iter().map(trio_view).collect(),
        limits: LIMITS,
    })
}

/// `PUT /api/decks/:id`. R250, R256: create or replace one deck. D1–D5 only: a draft may be
/// incomplete or unowned.
pub async fn put_deck(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let id = path_id_of(req.params.get("id"), "deck")?;
    let raw_name = name_of(&req.body)?;
    let cards = string_list(&req.body, "cards")?;
    let catalog_version = str(&req.body, "catalogVersion")?;
    let portrait = optional_str(&req.body, "portrait")?;

    assert_current_catalog(app, &catalog_version)?;
    let name = validator::normalize_name(&raw_name);
    let issues = deck_draft_issues(app, &name, &cards, portrait.as_deref(), true)?;
    if let Some(first) = first_message(&issues) {
        return Err(draft_refused(&first, &issues));
    }

    let now = now_ms();
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    // `created_at` is only read on a create; an update keeps the stored one (the store's contract).
    let outcome = tx
        .decks_upsert(
            &SavedDeck {
                id: id.clone(),
                profile_id: profile.id.clone(),
                name: name.clone(),
                cards: cards.clone(),
                portrait: portrait.clone(),
                catalog_version: catalog_version.clone(),
                created_at: now,
                updated_at: now,
            },
            MAX_SAVED_DECKS as i64,
        )
        .await?;
    match outcome {
        UpsertOutcome::NotOwner => return Err(not_found("deck")),
        UpsertOutcome::Limit => {
            return Err(ApiError::with_details(
                ApiErrorCode::Conflict,
                format!("You can keep {MAX_SAVED_DECKS} decks at most; delete one to save another."),
                json!({ "limit": MAX_SAVED_DECKS }),
            ));
        }
        UpsertOutcome::Created | UpsertOutcome::Updated => {}
    }

    // Read back rather than echo the request, so the answer carries the kept `created_at`.
    let saved = tx.decks_get(&id).await?;
    tx.commit().await?;
    // Deleted between the write and the read by the same player's other tab: say so plainly.
    let Some(saved) = saved.filter(|deck| deck.profile_id == profile.id) else {
        return Err(not_found("deck"));
    };
    log_info(
        "deck.saved",
        json!({ "profileId": profile.id, "deckId": id, "outcome": upsert_outcome_name(&outcome), "cards": cards.len() }),
    );
    ok_of(&json!({ "deck": deck_view(&saved) }))
}

/// `DELETE /api/decks/:id`. Idempotent: `deleted: false` when this profile has no deck with that
/// id, whoever else might. Every trio slot that named the deck becomes empty in the same statement
/// (R252).
pub async fn delete_deck(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let id = path_id_of(req.params.get("id"), "deck")?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let deleted = tx.decks_remove(&profile.id, &id).await?;
    tx.commit().await?;
    if deleted {
        log_info("deck.deleted", json!({ "profileId": profile.id, "deckId": id }));
    }
    ok_of(&json!({ "deleted": deleted }))
}

/// `PUT /api/trios/:id`. R252, R256: create or replace one trio. T1–T3 only: its decks may be
/// incomplete or share cards, which R253 judges at queue. A slot naming a deck this profile has not
/// saved (yet) is a conflict the client resolves by saving that deck first — an offline draft
/// syncing out of order.
pub async fn put_trio(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let id = path_id_of(req.params.get("id"), "trio")?;
    let raw_name = name_of(&req.body)?;
    let deck_ids = slots_of(&req.body)?;

    let name = validator::normalize_name(&raw_name);
    let issues = trio_draft_issues(&name, &deck_ids)?;
    if let Some(first) = first_message(&issues) {
        return Err(draft_refused(&first, &issues));
    }
    // T2 has passed, so there are exactly three slots.
    let slots = trio_slots_of(&deck_ids)?;

    let now = now_ms();
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let outcome = tx
        .trios_upsert(
            &SavedTrio {
                id: id.clone(),
                profile_id: profile.id.clone(),
                name,
                deck_ids: slots,
                created_at: now,
                updated_at: now,
            },
            MAX_SAVED_TRIOS as i64,
        )
        .await?;
    match outcome {
        TrioUpsertOutcome::NotOwner => return Err(not_found("trio")),
        TrioUpsertOutcome::Limit => {
            return Err(ApiError::with_details(
                ApiErrorCode::Conflict,
                format!("You can keep {MAX_SAVED_TRIOS} trios at most; delete one to save another."),
                json!({ "limit": MAX_SAVED_TRIOS }),
            ));
        }
        TrioUpsertOutcome::UnknownDeck => {
            return Err(ApiError::with_details(
                ApiErrorCode::Conflict,
                "This trio names a deck that is not saved yet; save the deck first.",
                json!({ "unknownDeck": true }),
            ));
        }
        TrioUpsertOutcome::Created | TrioUpsertOutcome::Updated => {}
    }

    let saved = tx.trios_get(&id).await?;
    tx.commit().await?;
    let Some(saved) = saved.filter(|trio| trio.profile_id == profile.id) else {
        return Err(not_found("trio"));
    };
    log_info("trio.saved", json!({ "profileId": profile.id, "trioId": id, "outcome": trio_outcome_name(&outcome) }));
    ok_of(&json!({ "trio": trio_view(&saved) }))
}

/// `POST /api/trios/import`. R340, R341: import a trio code's decks and the trio naming them, all
/// or nothing. The client decoded the code (R339) and minted every id; this checks all of it as any
/// save would — the catalog version, D1–D4 for each deck, T1–T3 for the trio — and then the room
/// under both caps (R340). A refusal names what to fix and nothing is written; otherwise every deck
/// and then the trio are upserted in one transaction, so a write that fails part-way rolls the
/// others back. Sending the same ids again (a retry after a dropped answer) updates what the first
/// attempt made and takes no new slot. Unowned cards and cards the decks share are kept: both are
/// judged at queue (R253), and the workshop marks them.
pub async fn import_trio(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let input = import_of(&req.body)?;
    assert_current_catalog(app, &input.catalog_version)?;

    let mut decks: Vec<Option<ImportedDeckInput>> = Vec::with_capacity(input.slots.len());
    for (slot, deck) in input.slots.iter().enumerate() {
        let Some(deck) = deck else {
            decks.push(None);
            continue;
        };
        let name = validator::normalize_name(&deck.name);
        let issues = deck_draft_issues(app, &name, &deck.cards, None, false)?;
        if let Some(first) = first_message(&issues) {
            return Err(draft_refused(&format!("{}: {}", imported_deck_label(slot, &name), first), &issues));
        }
        decks.push(Some(ImportedDeckInput { id: deck.id.clone(), name, cards: deck.cards.clone() }));
    }
    let deck_ids: Vec<Option<String>> = decks.iter().map(|deck| deck.as_ref().map(|deck| deck.id.clone())).collect();
    let trio_name = validator::normalize_name(&input.trio.name);
    let trio_issues = trio_draft_issues(&trio_name, &deck_ids)?;
    if let Some(first) = first_message(&trio_issues) {
        return Err(draft_refused(&first, &trio_issues));
    }
    let filled_ids: Vec<String> = deck_ids.iter().flatten().cloned().collect();
    let slots = trio_slots_of(&deck_ids)?;

    let now = now_ms();
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    assert_import_room(&mut tx, &profile.id, &filled_ids, &input.trio.id).await?;
    // Each deck one millisecond after the one before: the list is oldest first with ties broken on
    // the id, and the ids are random, so one instant for all three would list them in any order
    // rather than in their slots' (R341).
    let mut order = 0;
    for deck in decks.iter().flatten() {
        let at = now + order;
        order += 1;
        let outcome = tx
            .decks_upsert(
                &SavedDeck {
                    id: deck.id.clone(),
                    profile_id: profile.id.clone(),
                    name: deck.name.clone(),
                    cards: deck.cards.clone(),
                    portrait: None,
                    catalog_version: input.catalog_version.clone(),
                    created_at: at,
                    updated_at: at,
                },
                MAX_SAVED_DECKS as i64,
            )
            .await?;
        if !matches!(outcome, UpsertOutcome::Created | UpsertOutcome::Updated) {
            // Dropping `tx` rolls back every write made before this one.
            return Err(import_outcome_refused("deck", upsert_outcome_name(&outcome)));
        }
    }
    let outcome = tx
        .trios_upsert(
            &SavedTrio {
                id: input.trio.id.clone(),
                profile_id: profile.id.clone(),
                name: trio_name,
                deck_ids: slots,
                created_at: now,
                updated_at: now,
            },
            MAX_SAVED_TRIOS as i64,
        )
        .await?;
    if !matches!(outcome, TrioUpsertOutcome::Created | TrioUpsertOutcome::Updated) {
        return Err(import_outcome_refused("trio", trio_outcome_name(&outcome)));
    }
    tx.commit().await?;

    // Read back, as a single save does, so the answer carries the kept `created_at`s.
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let mut saved_decks: Vec<DeckView> = Vec::with_capacity(filled_ids.len());
    for id in &filled_ids {
        match tx.decks_get(id).await? {
            Some(saved) if saved.profile_id == profile.id => saved_decks.push(deck_view(&saved)),
            _ => return Err(not_found("deck")),
        }
    }
    let saved_trio = tx.trios_get(&input.trio.id).await?;
    tx.commit().await?;
    let Some(saved_trio) = saved_trio.filter(|trio| trio.profile_id == profile.id) else {
        return Err(not_found("trio"));
    };
    log_info(
        "trio.imported",
        json!({ "profileId": profile.id, "trioId": input.trio.id, "decks": filled_ids.len() }),
    );
    ok_of(&json!({ "decks": saved_decks, "trio": trio_view(&saved_trio) }))
}

/// `DELETE /api/trios/:id`. Idempotent, as the deck's. The decks it named are untouched.
pub async fn delete_trio(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let id = path_id_of(req.params.get("id"), "trio")?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let deleted = tx.trios_remove(&profile.id, &id).await?;
    tx.commit().await?;
    if deleted {
        log_info("trio.deleted", json!({ "profileId": profile.id, "trioId": id }));
    }
    ok_of(&json!({ "deleted": deleted }))
}

// ---------------------------------------------------------------------------
// The queue-time half: what a ticket or a room is made with (R253, R257, R264)
// ---------------------------------------------------------------------------

/// What `POST /api/queue`, `POST /api/rooms` and `POST /api/rooms/:code/join` were asked for, parsed
/// but not yet looked up (R257). The legacy `deckIndex` form is not ported (SURFACE §11.3).
#[derive(serde::Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ModeChoiceInput {
    Bo1 { deck_id: String },
    Bo3 { trio_id: String },
    Random,
}

/// What a ticket or a room freezes (§9.4: "Decks are frozen into the queue ticket").
#[derive(serde::Serialize, Clone)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum FrozenChoice {
    Bo1 { deck: FrozenDeck },
    Bo3 { trio: FrozenTrio },
    Random,
}

/// R257: `{ mode: "bo1", deckId } | { mode: "bo3", trioId } | { mode: "random" }`. Only the fields
/// the mode needs are read; anything malformed among them is a 400. A body with no `mode` (and
/// R257's legacy `deckIndex`) is refused like any other malformed mode (SURFACE §11.3).
pub fn read_mode_choice(body: &Value) -> Result<ModeChoiceInput, ApiError> {
    let mode = body.get("mode").and_then(Value::as_str);
    match mode {
        Some("random") => Ok(ModeChoiceInput::Random),
        Some("bo3") => match saved_id_of(body.get("trioId")) {
            Some(trio_id) => Ok(ModeChoiceInput::Bo3 { trio_id }),
            None => Err(bad_request("Conquest needs \"trioId\", the id of one of your trios")),
        },
        Some("bo1") => {
            let raw_deck_id = body.get("deckId").filter(|value| !value.is_null());
            let Some(raw_deck_id) = raw_deck_id else {
                return Err(bad_request("Best of 1 needs \"deckId\", the id of one of your decks"));
            };
            match saved_id_of(Some(raw_deck_id)) {
                Some(deck_id) => Ok(ModeChoiceInput::Bo1 { deck_id }),
                None => Err(bad_request("\"deckId\" must be the id of one of your decks")),
            }
        }
        _ => Err(bad_request("\"mode\" must be \"bo1\", \"bo3\" or \"random\"")),
    }
}

/// R165, generalised by R253: every refusal the player fixes in the deck builder is a
/// `loadout_invalid` (422), never a 404 — a missing deck and a deleted trio are both "go and build
/// one", and a 404 would send a client looking for a route that is working correctly.
const DECK_GONE: &str = "That deck is no longer saved; pick another.";
const TRIO_GONE: &str = "That trio is no longer saved; pick another.";

fn refused(message: &str) -> ApiError {
    ApiError::new(ApiErrorCode::LoadoutInvalid, message)
}

/// One of this profile's own decks by id, or none — another profile's deck is not theirs to play.
async fn own_deck(t: &mut Tx<'_>, profile_id: &str, deck_id: &str) -> Result<Option<SavedDeck>, ApiError> {
    let deck = t.decks_get(deck_id).await?;
    Ok(deck.filter(|deck| deck.profile_id == profile_id))
}

async fn chosen_deck(t: &mut Tx<'_>, profile_id: &str, deck_id: &str) -> Result<SavedDeck, ApiError> {
    own_deck(t, profile_id, deck_id).await?.ok_or_else(|| refused(DECK_GONE))
}

/// R253's two rule sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoadoutScope {
    /// A Best-of-1 deck: L2, L3, L5, L6 (`validate_deck`).
    Deck,
    /// A Conquest trio: L1–L6 (`validate_loadout`).
    Trio,
}

/// The single call to the shared validator at queue time (§9.4: "at save and again at queue").
/// R253: `scope` picks the rules — L2, L3, L5, L6 for one deck, L1–L6 for a trio — and the decks'
/// names let every sentence name the deck as the player named it. L5 needs the entitlements, so the
/// collection is read and handed in; L6 is checked against the CURRENT catalog, whatever version the
/// deck was saved under (R253: a saved deck's own version is no reason to refuse it).
///
/// The first issue's message becomes the error's and every issue rides along as `details`, exactly
/// as reported: no renumbering, no recomposed sentence. (This is TS's `sharedLoadoutValidator`, the
/// adapter `loadout-validator.ts` held, inlined.)
fn assert_legal(
    app: &App,
    owned: &IndexMap<String, i64>,
    decks: &[SavedDeck],
    scope: LoadoutScope,
) -> Result<(), ApiError> {
    let banned: Vec<&String> = app.catalog.card_ids.iter().filter(|card_id| app.catalog.is_banned(card_id)).collect();
    let catalog = json!({ "version": app.catalog.version, "cards": to_json(&app.catalog.defs)?, "banned": banned });
    let collection = to_json(owned)?;
    let deck_at = |deck: &SavedDeck| json!({ "name": deck.name, "cards": deck.cards });
    let result = match scope {
        LoadoutScope::Deck => {
            let deck = decks.first().map(deck_at).unwrap_or_else(|| json!({ "cards": [] }));
            let input: validator::DeckInput =
                serde_json::from_value(json!({ "deck": deck, "catalog": catalog, "collection": collection }))?;
            to_json(&validator::validate_deck(&input))?
        }
        LoadoutScope::Trio => {
            let input: validator::LoadoutInput = serde_json::from_value(json!({
                "decks": decks.iter().map(deck_at).collect::<Vec<_>>(),
                "catalog": catalog,
                "collection": collection,
            }))?;
            to_json(&validator::validate_loadout(&input))?
        }
    };
    if result.get("ok").and_then(Value::as_bool) == Some(true) {
        return Ok(());
    }
    let errors: Vec<Value> = result
        .get("errors")
        .and_then(Value::as_array)
        .map(|errors| {
            errors
                .iter()
                .map(|error| {
                    let mut issue = Map::new();
                    issue.insert("rule".to_string(), error.get("rule").cloned().unwrap_or(Value::Null));
                    issue.insert("message".to_string(), error.get("message").cloned().unwrap_or(Value::Null));
                    if let Some(deck) = error.get("deck").filter(|deck| !deck.is_null()) {
                        issue.insert("deck".to_string(), deck.clone());
                    }
                    if let Some(card_id) = error.get("cardId").filter(|card_id| !card_id.is_null()) {
                        issue.insert("cardId".to_string(), card_id.clone());
                    }
                    Value::Object(issue)
                })
                .collect()
        })
        .unwrap_or_default();
    let Some(first) = first_message(&errors) else {
        return Err(ApiError::internal("the validator refused a loadout without saying why"));
    };
    Err(ApiError::with_details(ApiErrorCode::LoadoutInvalid, first, Value::Array(errors)))
}

/// A copy, so a frozen deck can never alias the stored one.
fn freeze(deck: &SavedDeck) -> FrozenDeck {
    // R642: the portrait freezes with the deck, present (a saved deck always has one, or null).
    FrozenDeck { name: deck.name.clone(), cards: deck.cards.clone(), portrait: Some(deck.portrait.clone()) }
}

/// Loads the chosen deck or trio, checks it by R253 and returns the frozen copy the ticket or the
/// room keeps. Nothing after this reads the saved deck again, so editing it while queued (or while
/// a room waits) cannot change the game it becomes (§9.8: "Deck swapped after matchmaking").
///
/// An empty trio slot is not skipped: the filled decks go to the validator as they are, so a trio
/// with two decks fails L1 in the shared module's own words. All Random freezes nothing (R258).
pub async fn freeze_choice(app: &App, profile_id: &str, choice: &ModeChoiceInput) -> Result<FrozenChoice, ApiError> {
    let (deck_id, trio_id) = match choice {
        ModeChoiceInput::Random => return Ok(FrozenChoice::Random),
        ModeChoiceInput::Bo1 { deck_id } => (Some(deck_id), None),
        ModeChoiceInput::Bo3 { trio_id } => (None, Some(trio_id)),
    };
    let mut tx = app.db.begin(Some(profile_id)).await?;
    let owned = owned_in(&mut tx, profile_id).await?;

    if let Some(deck_id) = deck_id {
        let deck = chosen_deck(&mut tx, profile_id, deck_id).await?;
        tx.commit().await?;
        assert_legal(app, &owned, std::slice::from_ref(&deck), LoadoutScope::Deck)?;
        return Ok(FrozenChoice::Bo1 { deck: freeze(&deck) });
    }

    let trio_id = trio_id.map(String::as_str).unwrap_or_default();
    let trio = match tx.trios_get(trio_id).await? {
        Some(trio) if trio.profile_id == profile_id => trio,
        _ => return Err(refused(TRIO_GONE)),
    };
    let mut filled: Vec<SavedDeck> = Vec::new();
    for deck_id in slots_in(&trio.deck_ids).into_iter().flatten() {
        // A slot's deck is this profile's by the store's own constraint; `own_deck` says so again.
        if let Some(deck) = own_deck(&mut tx, profile_id, &deck_id).await? {
            filled.push(deck);
        }
    }
    tx.commit().await?;
    assert_legal(app, &owned, &filled, LoadoutScope::Trio)?;

    let [first, second, third] = filled.as_slice() else {
        // L1 passed, so the validator is not the shared module: a wiring fault, not a player's.
        return Err(ApiError::internal(format!("the validator passed trio {} with {} decks", trio.id, filled.len())));
    };
    Ok(FrozenChoice::Bo3 {
        trio: FrozenTrio { name: trio.name.clone(), decks: (freeze(first), freeze(second), freeze(third)) },
    })
}

/// R264: a profile in a series that is not over can neither queue nor create or join a room — its
/// next game is already decided by the series, and a second one would make it two places at once.
/// `already_in_match`, like being in a match, with the series to go back to in `details`.
pub async fn assert_not_in_series(app: &App, profile_id: &str) -> Result<(), ApiError> {
    let mut tx = app.db.begin(Some(profile_id)).await?;
    let series = tx.series_active_for(profile_id).await?;
    tx.commit().await?;
    if let Some(series) = series {
        return Err(ApiError::with_details(
            ApiErrorCode::AlreadyInMatch,
            "Finish your Conquest series first.",
            json!({ "seriesId": series.id }),
        ));
    }
    Ok(())
}
