//! An account's card proficiency and the stamp each card wears (SPEC §14.14; R3013000, R3013001).
//! Only the caller's own, keyed on the verified token's profile and never on a field of the request.
//!
//! `GET /api/proficiency` answers `{ cards }`, one entry per card the account has played in a game that
//! counts (`{ cardId, plays, level, stamp }`, by card id); a card it never played has no entry, at level 0.
//! The server alone writes plays and levels, with a game's pay (`economy::proficiency`).
//!
//! `PUT /api/proficiency/stamp` (`{ cardId, stamp }`) picks the one stamp a card wears, or none with
//! `null`. R3013001: level n unlocks stamp n, so a stamp above the card's level, a card never played
//! included, is a 400 and writes nothing; so are a card that is no card of the catalog, a token and a
//! stamp that is none of `1..=PROFICIENCY_LEVELS`. It takes the account's economy turn (R2378000), as a
//! card pick does, and answers `{ cards }` as it now stands.

use std::sync::Arc;

use serde_json::{Value, json};

use crate::api::card_picks::is_playable;
use crate::api::collection::caller_profile;
use crate::api::http::{ApiResult, Req, bad_request, now_ms, ok_of, str};
use crate::app::App;
use crate::config::PROFICIENCY_LEVELS;
use crate::economy::store::CardProficiency;

/// `{ cards }`: the account's card proficiency, by card id.
fn cards_answer(cards: &[CardProficiency]) -> ApiResult {
    ok_of(&json!({ "cards": cards }))
}

/// `GET /api/proficiency` (`AuthLevel::Active`).
pub async fn get_proficiency(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let cards = tx.economy_card_proficiency_get(&profile.id).await?;
    tx.commit().await?;
    cards_answer(&cards)
}

/// A `stamp` as a `PUT` sends it: `null` for none, or a stamp of `1..=PROFICIENCY_LEVELS`.
fn stamp_of(body: &Value) -> Option<Option<i64>> {
    match body.get("stamp")? {
        Value::Null => Some(None),
        value => value
            .as_i64()
            .filter(|stamp| {
                usize::try_from(*stamp).is_ok_and(|stamp| (1..=PROFICIENCY_LEVELS).contains(&stamp))
            })
            .map(Some),
    }
}

/// `PUT /api/proficiency/stamp` (`AuthLevel::Active`), in the account's economy turn (R2378000).
pub async fn put_proficiency_stamp(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let card_id = str(&req.body, "cardId")?;
    if !is_playable(app, &card_id) {
        return Err(bad_request("\"cardId\" is not a card of the catalog"));
    }
    let Some(stamp) = stamp_of(&req.body) else {
        return Err(bad_request(format!(
            "\"stamp\" must be a whole number from 1 to {PROFICIENCY_LEVELS}, or null"
        )));
    };

    // R2378000: the account's turn first, so a burst waits here holding no database connection, not
    // on the economy lock below.
    let _turn = app.spends.claim(&profile.id).await?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    // The level is read and the stamp written under the lock a game's pay takes, so the two take turns.
    tx.economy_lock_account(&profile.id).await?;
    let held = tx.economy_card_proficiency_get(&profile.id).await?;
    let row = held.iter().find(|row| row.card_id == card_id);
    // R3013001: level n unlocks stamp n; a card never played is at level 0.
    let level = row.map_or(0, |row| row.level);
    if let Some(stamp) = stamp.filter(|stamp| *stamp > level) {
        return Err(bad_request(format!("\"stamp\" needs level {stamp} of this card")));
    }
    let cards = if row.is_some() {
        tx.economy_card_proficiency_stamp(&profile.id, &card_id, stamp, now_ms())
            .await?;
        tx.economy_card_proficiency_get(&profile.id).await?
    } else {
        // Only `null` gets here without a row: there is no stamp to clear.
        held
    };
    tx.commit().await?;
    cards_answer(&cards)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r3013001_a_stamp_is_null_or_one_of_the_levels() {
        assert_eq!(stamp_of(&json!({ "stamp": null })), Some(None));
        assert_eq!(stamp_of(&json!({ "stamp": 1 })), Some(Some(1)));
        let top = i64::try_from(PROFICIENCY_LEVELS).expect("a few levels");
        assert_eq!(stamp_of(&json!({ "stamp": top })), Some(Some(top)));
        for refused in [
            json!({}),
            json!({ "stamp": 0 }),
            json!({ "stamp": top + 1 }),
            json!({ "stamp": -1 }),
            json!({ "stamp": 1.5 }),
            json!({ "stamp": "1" }),
            json!({ "stamp": true }),
        ] {
            assert_eq!(stamp_of(&refused), None, "{refused}");
        }
    }
}
