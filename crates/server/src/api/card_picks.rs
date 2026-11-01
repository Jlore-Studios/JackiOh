//! The picks an account makes for one card of its own (SPEC §14.2, SPEC §11 R1834000, R1834001): a
//! foil, finish, border, and full art (R1655000), shown to opponents (R1834002). Keyed on caller token.
//!
//! `GET /api/card-picks`: answers `{ cards }` for every card with picks.
//!
//! `PUT /api/card-picks`: updates `{ cardId, foil?, finish?, border?, fullArt? }`. Missing keys
//! keep existing picks, `null` clears. R1834001: ids and full art must be owned; otherwise 400.
//!
//! `POST /api/card-picks/upload`: device upload of picks. Existing account rows are preserved;
//! unknown ids stay (R1655000), unowned/invalid picks dropped, non-catalog cards skipped.

use std::sync::Arc;

use serde_json::{Map, Value, json};

use crate::api::collection::caller_profile;
use crate::api::http::{ApiError, ApiResult, Req, bad_request, now_ms, ok_of, str};
use crate::app::App;
use crate::config::PLAYER_SETTINGS_TEXT_MAX_LENGTH;
use crate::cosmetics::{self, PickRefusal, full_art_of, is_slug, owns};
use crate::economy::store::{CardPicks, OwnedCosmetics};
use crate::loadout::{CARD_SLOT_KINDS, FULL_ART_SETTING, setting_key};

/// A card a deck may hold: in the catalog, and no token.
fn is_playable(app: &App, card_id: &str) -> bool {
    app.catalog.card_ids.iter().any(|id| id == card_id) && !app.catalog.is_token(card_id)
}

/// What a write of `card_id`'s full art comes to against the account's ledger: `Ok` when the card
/// has a full art the account owns.
fn check_full_art(card_id: &str, owned: &OwnedCosmetics) -> Result<(), PickRefusal> {
    match full_art_of(cosmetics::registry(), card_id) {
        None => Err(PickRefusal::WrongKind),
        Some(full_art) if owns(full_art, Some(card_id), owned) => Ok(()),
        Some(_) => Err(PickRefusal::NotOwned),
    }
}

/// `{ cards }`: the account's card picks, by card id.
fn cards_answer(cards: &[CardPicks]) -> ApiResult {
    ok_of(&json!({ "cards": cards }))
}

/// `GET /api/card-picks` (`AuthLevel::Active`).
pub async fn get_card_picks(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let cards = tx.economy_card_picks_get(&profile.id).await?;
    tx.commit().await?;
    cards_answer(&cards)
}

/// One key of a `PUT`: left out (`None`), cleared (`Some(None)`) or set.
fn change_of<T>(
    body: &Value,
    key: &str,
    read: impl Fn(&Value) -> Option<T>,
    what: &str,
) -> Result<Option<Option<T>>, ApiError> {
    match body.get(key) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(value) => read(value)
            .map(|value| Some(Some(value)))
            .ok_or_else(|| bad_request(format!("\"{key}\" must be {what} or null"))),
    }
}

/// `PUT /api/card-picks` (`AuthLevel::Active`), in the account's economy turn (R2378000).
pub async fn put_card_pick(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let card_id = str(&req.body, "cardId")?;
    if !is_playable(app, &card_id) {
        return Err(bad_request("\"cardId\" is not a card of the catalog"));
    }
    let mut changes = Vec::with_capacity(CARD_SLOT_KINDS.len());
    for kind in CARD_SLOT_KINDS {
        let change = change_of(
            &req.body,
            setting_key(kind),
            |value| value.as_str().map(str::to_string),
            "a cosmetic id",
        )?;
        changes.push((kind, change));
    }
    let full_art = change_of(&req.body, FULL_ART_SETTING, Value::as_bool, "a boolean")?;

    // R2378000: the account's turn first, so a burst waits here holding no database
    // connection, not on the economy lock below.
    let _turn = app.spends.claim(&profile.id).await?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    // The row is read, changed and written back: two devices' writes take turns, so neither's
    // change is lost.
    tx.economy_lock_account(&profile.id).await?;
    let owned = tx.economy_cosmetics_get(&profile.id).await?;
    for (kind, change) in &changes {
        if let Some(Some(id)) = change {
            cosmetics::check_pick(cosmetics::registry(), &owned, *kind, id, Some(&card_id))
                .map_err(|refusal| bad_request(refusal.message(setting_key(*kind))))?;
        }
    }
    if full_art == Some(Some(true)) {
        check_full_art(&card_id, &owned).map_err(|refusal| bad_request(refusal.message(FULL_ART_SETTING)))?;
    }
    let held = tx.economy_card_picks_get(&profile.id).await?;
    let mut picks = held
        .into_iter()
        .find(|picks| picks.card_id == card_id)
        .unwrap_or_else(|| CardPicks {
            card_id: card_id.clone(),
            ..CardPicks::default()
        });
    for (kind, change) in changes {
        if let Some(pick) = change {
            picks.set(kind, pick);
        }
    }
    if let Some(pick) = full_art {
        picks.full_art = pick;
    }
    tx.economy_card_picks_put(&profile.id, &picks, now_ms()).await?;
    let cards = tx.economy_card_picks_get(&profile.id).await?;
    tx.commit().await?;
    cards_answer(&cards)
}

/// One uploaded id: a lower-case slug no longer than a settings text, as the table's checks hold it.
fn is_id_shape(id: &str) -> bool {
    is_slug(id) && id.len() <= PLAYER_SETTINGS_TEXT_MAX_LENGTH
}

/// One entry of an upload, read whole or refused: a card id, and each pick a well-shaped id or a
/// boolean for the full art.
fn upload_entry(entry: &Value) -> Result<CardPicks, ApiError> {
    let refused =
        || bad_request("every entry of \"cards\" must be { cardId, foil?, finish?, border?, fullArt? }");
    let fields: &Map<String, Value> = entry.as_object().ok_or_else(refused)?;
    let card_id = fields
        .get("cardId")
        .and_then(Value::as_str)
        .filter(|id| is_id_shape(id))
        .ok_or_else(refused)?;
    let mut picks = CardPicks {
        card_id: card_id.to_string(),
        ..CardPicks::default()
    };
    for kind in CARD_SLOT_KINDS {
        match fields.get(setting_key(kind)) {
            None | Some(Value::Null) => {}
            Some(Value::String(id)) if is_id_shape(id) => picks.set(kind, Some(id.clone())),
            Some(_) => return Err(refused()),
        }
    }
    match fields.get(FULL_ART_SETTING) {
        None | Some(Value::Null) => {}
        Some(Value::Bool(on)) => picks.full_art = Some(*on),
        Some(_) => return Err(refused()),
    }
    Ok(picks)
}

/// R1834000: what of an uploaded entry can be kept: an id this build does not know stays, as the
/// device kept it; one it knows is kept only where the account owns it on the card, under its kind.
/// A full art switched on stays unless the card's full art is known and not owned.
fn keepable(mut picks: CardPicks, owned: &OwnedCosmetics) -> CardPicks {
    let registry = cosmetics::registry();
    for kind in CARD_SLOT_KINDS {
        let Some(id) = picks.get(kind).map(str::to_string) else {
            continue;
        };
        let known = registry.iter().any(|cosmetic| cosmetic.id == id);
        if known && cosmetics::check_pick(registry, owned, kind, &id, Some(&picks.card_id)).is_err() {
            picks.set(kind, None);
        }
    }
    if picks.full_art == Some(true) && check_full_art(&picks.card_id, owned) == Err(PickRefusal::NotOwned) {
        picks.full_art = None;
    }
    picks
}

/// `POST /api/card-picks/upload` (`AuthLevel::Active`), in the account's economy turn (R2378000).
pub async fn upload_card_picks(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let Some(entries) = req.body.get("cards").and_then(Value::as_array) else {
        return Err(bad_request("\"cards\" must be a list of card picks"));
    };
    let most = app.catalog.card_ids.len();
    if entries.len() > most {
        return Err(bad_request(format!(
            "at most {most} cards' picks can be uploaded"
        )));
    }
    let uploaded = entries
        .iter()
        .map(upload_entry)
        .collect::<Result<Vec<CardPicks>, ApiError>>()?;

    // R2378000: the account's turn first, so a burst waits here holding no database
    // connection, not on the economy lock below.
    let _turn = app.spends.claim(&profile.id).await?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    // What the account owns is read and the rows it lacks are written: under the lock
    // `put_card_pick` takes, a `PUT` from another device takes its turn instead of writing over
    // an uploaded row, so neither's change is lost.
    tx.economy_lock_account(&profile.id).await?;
    let owned = tx.economy_cosmetics_get(&profile.id).await?;
    let kept: Vec<CardPicks> = uploaded
        .into_iter()
        .filter(|picks| is_playable(app, &picks.card_id))
        .map(|picks| keepable(picks, &owned))
        .collect();
    tx.economy_card_picks_upload(&profile.id, &kept, now_ms()).await?;
    let cards = tx.economy_card_picks_get(&profile.id).await?;
    tx.commit().await?;
    cards_answer(&cards)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cosmetics::CosmeticKind;
    use crate::economy::store::CardCosmetic;

    #[test]
    fn r1834000_an_upload_keeps_an_unknown_id_and_drops_one_it_cannot_wear() {
        let none = OwnedCosmetics::default();
        let picks = CardPicks {
            card_id: "core-001".to_string(),
            foil: Some("foil-from-a-newer-client".to_string()),
            // A known id of another kind.
            finish: Some("border-standard".to_string()),
            border: Some("border-standard".to_string()),
            full_art: Some(false),
        };
        let kept = keepable(picks.clone(), &none);
        assert_eq!(kept.foil.as_deref(), Some("foil-from-a-newer-client"));
        assert_eq!(kept.finish, None);
        assert_eq!(kept.border.as_deref(), Some("border-standard"));
        assert_eq!(kept.full_art, Some(false));
        // A full art switched on stays only where the card's full art is owned, or unknown here.
        let full_art = cosmetics::registry()
            .iter()
            .find(|cosmetic| cosmetic.kind == CosmeticKind::FullArt)
            .expect("the registry ships a full art");
        let card = full_art.card.clone().expect("a full art names its card");
        let on = CardPicks {
            card_id: card.clone(),
            full_art: Some(true),
            ..CardPicks::default()
        };
        assert_eq!(keepable(on.clone(), &none).full_art, None);
        let owned = OwnedCosmetics {
            cards: vec![CardCosmetic {
                card_id: card,
                cosmetic_id: full_art.id.clone(),
            }],
            account: Vec::new(),
        };
        assert_eq!(keepable(on, &owned).full_art, Some(true));
        let unknown_here = CardPicks {
            card_id: "core-999".to_string(),
            full_art: Some(true),
            ..CardPicks::default()
        };
        assert_eq!(keepable(unknown_here, &none).full_art, Some(true));
    }
}
