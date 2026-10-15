//! Custom cards on the account (SPEC §12.9, R915008; issue #918).
//!
//! An active account keeps up to `CUSTOM_CARDS_PER_ACCOUNT` custom cards (§12.1, R915000), private
//! to it: only the owner reads its rows (row-level security, as for saved decks) and only these
//! routes write them (migration 0037).
//!
//! - **A create** (`POST`) mints the card's id, `custom-` and 32 lowercase hex digits, and starts it
//!   at version 1. The id is the server's, never the client's, so no player can name a card into
//!   another's id space.
//! - **An update** (`PUT`) replaces the card whole and names the version it replaces: the store
//!   counts the save (that version and one) under the card's lock, and a version another save has
//!   moved past is a 409 that names the current one, so a stale tab or device never overwrites a
//!   newer save.
//! - **Every save** is judged by `jackioh_engine::custom::validate_custom_card` (R915001), the one
//!   check the creator, the practice worker and a game's setup make too, and refused with its
//!   issues as `details`. No limit is restated here.
//!
//! The server never compiles or runs a script: it does not link `jackioh-luau` (#902 V4), and a
//! source is text it stores, bounded by the validator's `CUSTOM_SOURCE_MAX_BYTES`. The creator checks
//! a script with the browser's Luau module before it saves it (§12.9). A deleted card leaves the
//! decks that name it as they are; it then fails their practice game's setup until it is replaced.

use std::sync::Arc;

use serde::Serialize;
use serde_json::{Value, json};

use jackioh_engine::config::CUSTOM_ID_PREFIX;
use jackioh_engine::custom::{custom_id_well_formed, validate_custom_card};
use jackioh_engine::{CustomCard, CustomFrame};

use crate::api::collection::caller_profile;
use crate::api::http::{
    ApiError, ApiErrorCode, ApiResult, Req, bad_request, json as json_response, log_info, now_ms, ok_of,
    to_json,
};
use crate::app::App;
use crate::config::{CUSTOM_CARDS_PER_ACCOUNT, DRAFT_ISSUES_REPORTED_MAX};
use crate::db::store::{CustomCardCreateOutcome, CustomCardUpdateOutcome, CustomCardWrite};

// What the client sees: each card as the engine's `CustomCard` JSON (`id`, `version`, `frame`,
// `source` and, when it has one, `builder`), the wire type the creator and a game read.

/// The cap the creator lives under (`GET /api/custom-cards`' `limits`).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CustomCardLimits {
    pub cards: i64,
}

pub const LIMITS: CustomCardLimits = CustomCardLimits {
    cards: CUSTOM_CARDS_PER_ACCOUNT as i64,
};

/// A create's status: the card is a new resource.
const CREATED: u16 = 201;

fn not_found() -> ApiError {
    ApiError::new(ApiErrorCode::NotFound, "There is no such custom card.")
}

// Reading a request

/// The `:id` of a card route: a well-formed custom id (R915000), or a 400 before any store call.
fn path_id_of(raw: Option<&String>) -> Result<String, ApiError> {
    match raw.filter(|text| custom_id_well_formed(text)) {
        Some(text) => Ok(text.clone()),
        None => Err(bad_request("that is not a custom card id")),
    }
}

/// `frame`, read as the engine's `CustomFrame`: a field of the wrong type is a 400 naming it.
/// Whether its values are within §12.2 is the validator's to say.
fn frame_of(body: &Value) -> Result<CustomFrame, ApiError> {
    let raw = body
        .get("frame")
        .ok_or_else(|| bad_request("\"frame\" must be a custom card's frame"))?;
    serde_json::from_value(raw.clone())
        .map_err(|error| bad_request(format!("\"frame\" is not a custom card's frame: {error}")))
}

/// `source`, the card-kit Luau: a string, empty (or absent) for a vanilla card (§12.2).
fn source_of(body: &Value) -> Result<String, ApiError> {
    match body.get("source") {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(text)) => Ok(text.clone()),
        Some(_) => Err(bad_request("\"source\" must be a string")),
    }
}

/// `builder`, the creator's own model of the card: any JSON, absent or `null` for none. Its size is
/// the validator's to judge.
fn builder_of(body: &Value) -> Option<Value> {
    body.get("builder").filter(|value| !value.is_null()).cloned()
}

/// An update's `version`: the version of the card the save replaces, a whole number from 1.
fn version_of(body: &Value) -> Result<i32, ApiError> {
    body.get("version")
        .and_then(Value::as_i64)
        .and_then(|version| i32::try_from(version).ok())
        .filter(|version| *version >= 1 && *version < i32::MAX)
        .ok_or_else(|| {
            bad_request(
                "\"version\" must be the version of the card this save replaces, a whole number from 1",
            )
        })
}

/// R915001: the validator's verdict on the card about to be saved. Refused with the first issue's
/// sentence and `details.issues`, the validator's `CustomIssue`s (`field`, `message`), at most
/// `DRAFT_ISSUES_REPORTED_MAX` of them.
fn refuse_invalid(card: &CustomCard) -> Result<(), ApiError> {
    let issues = validate_custom_card(card);
    let Some(first) = issues.first() else {
        return Ok(());
    };
    let reported = to_json(&issues.iter().take(DRAFT_ISSUES_REPORTED_MAX).collect::<Vec<_>>())?;
    Err(ApiError::with_details(
        ApiErrorCode::BadRequest,
        first.message.clone(),
        json!({ "issues": reported }),
    ))
}

/// R915000: `custom-` and the 32 lowercase hex digits of a random UUID: 128 random bits, well
/// inside `CUSTOM_ID_MAX_LENGTH`.
fn mint_id() -> String {
    format!("{CUSTOM_ID_PREFIX}{}", uuid::Uuid::new_v4().simple())
}

fn write_of(profile_id: &str, card: &CustomCard, at: i64) -> CustomCardWrite {
    CustomCardWrite {
        id: card.id.clone(),
        profile_id: profile_id.to_string(),
        frame: card.frame.clone(),
        source: card.source.clone(),
        builder: card.builder.clone(),
        at,
    }
}

// Routes

// All five routes are `AuthLevel::Active` rows of `app.rs`'s `ROUTES`: a pending account gets 403,
// as from its decks.

/// `GET /api/custom-cards`. The caller's cards, oldest first, and the cap.
pub async fn list_custom_cards(app: &Arc<App>, req: Req) -> ApiResult {
    #[derive(Serialize)]
    struct CustomCardsBody {
        cards: Vec<CustomCard>,
        limits: CustomCardLimits,
    }
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let cards = tx.custom_cards_list(&profile.id).await?;
    tx.commit().await?;
    ok_of(&CustomCardsBody {
        cards: cards.into_iter().map(|saved| saved.card).collect(),
        limits: LIMITS,
    })
}

/// `GET /api/custom-cards/:id`. One of the caller's cards; another profile's id answers as a
/// missing one, so an id reveals nothing about anyone else's cards.
pub async fn get_custom_card(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let id = path_id_of(req.params.get("id"))?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let saved = tx.custom_cards_get(&profile.id, &id).await?;
    tx.commit().await?;
    let saved = saved.ok_or_else(not_found)?;
    ok_of(&json!({ "card": saved.card }))
}

/// `POST /api/custom-cards` with `{ frame, source, builder? }`. A new card, 201: its id minted here,
/// at version 1; refused by the validator's issues (400) or past the cap (409, `details.limit`).
pub async fn create_custom_card(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let card = CustomCard {
        id: mint_id(),
        version: 1,
        frame: frame_of(&req.body)?,
        source: source_of(&req.body)?,
        builder: builder_of(&req.body),
    };
    refuse_invalid(&card)?;

    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let outcome = tx
        .custom_cards_create(
            &write_of(&profile.id, &card, now_ms()),
            CUSTOM_CARDS_PER_ACCOUNT as i64,
        )
        .await?;
    if outcome == CustomCardCreateOutcome::Limit {
        return Err(ApiError::with_details(
            ApiErrorCode::Conflict,
            format!(
                "You can keep {CUSTOM_CARDS_PER_ACCOUNT} custom cards at most; delete one to make another."
            ),
            json!({ "limit": CUSTOM_CARDS_PER_ACCOUNT }),
        ));
    }
    // Read back rather than echo the request, so the answer is what the store holds.
    let saved = tx.custom_cards_get(&profile.id, &card.id).await?;
    tx.commit().await?;
    let saved = saved.ok_or_else(|| ApiError::internal("a custom card just created did not read back"))?;
    log_info(
        "custom_card.created",
        json!({ "profileId": profile.id, "cardId": card.id, "sourceBytes": card.source.len() }),
    );
    Ok(json_response(CREATED, json!({ "card": to_json(&saved.card)? })))
}

/// `PUT /api/custom-cards/:id` with `{ version, frame, source, builder? }`, `version` the one this
/// save replaces. The card replaced whole at that version and one; refused by the validator's
/// issues (400), as a card the caller has not got (404), or when another save came first (409, its
/// current version as `details.version`).
pub async fn update_custom_card(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let id = path_id_of(req.params.get("id"))?;
    let replaces = version_of(&req.body)?;
    let card = CustomCard {
        id: id.clone(),
        version: replaces + 1,
        frame: frame_of(&req.body)?,
        source: source_of(&req.body)?,
        builder: builder_of(&req.body),
    };
    refuse_invalid(&card)?;

    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let outcome = tx
        .custom_cards_update(&write_of(&profile.id, &card, now_ms()), replaces)
        .await?;
    match outcome {
        CustomCardUpdateOutcome::Missing => return Err(not_found()),
        CustomCardUpdateOutcome::Stale => {
            let held = tx.custom_cards_get(&profile.id, &id).await?;
            return Err(ApiError::with_details(
                ApiErrorCode::Conflict,
                "This card was saved somewhere else since you opened it; open it again to edit its newest save.",
                json!({ "version": held.map(|saved| saved.card.version) }),
            ));
        }
        CustomCardUpdateOutcome::Updated => {}
    }
    let saved = tx.custom_cards_get(&profile.id, &id).await?;
    tx.commit().await?;
    let saved = saved.ok_or_else(not_found)?;
    log_info(
        "custom_card.updated",
        json!({ "profileId": profile.id, "cardId": id, "version": saved.card.version, "sourceBytes": saved.card.source.len() }),
    );
    ok_of(&json!({ "card": saved.card }))
}

/// `DELETE /api/custom-cards/:id`. Idempotent: `deleted: false` when the caller has no card with
/// that id, whoever else might. The decks that name it keep naming it (§12.9).
pub async fn delete_custom_card(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let id = path_id_of(req.params.get("id"))?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let deleted = tx.custom_cards_remove(&profile.id, &id).await?;
    tx.commit().await?;
    if deleted {
        log_info(
            "custom_card.deleted",
            json!({ "profileId": profile.id, "cardId": id }),
        );
    }
    ok_of(&json!({ "deleted": deleted }))
}
