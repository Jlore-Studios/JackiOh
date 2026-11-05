//! The picks an account makes for one card of its own (SPEC §14.2, SPEC §11 R1834000, R1834001): a foil, a
//! finish, a border and which of its full arts prints, if any (R1655000, R2655000), held on the account so
//! the server, which shows them to an opponent (R1834002), has them. Only the caller's own, keyed on the
//! verified token's profile and never on a field of the request.
//!
//! `GET /api/card-picks` answers `{ cards }` by card id, each id as it was written. `PUT` changes one
//! card's picks (`{ cardId, foil?, finish?, border?, fullArt?, augments? }`): a key left out keeps its pick,
//! `null` clears it. R1834001: an id must be one the account owns on that card. R3015005: a card wears one level
//! of one variant, named in `fullArt`: `false` for the normal face, `true` for Full art's first level on a
//! full-art card (R2655000), or a level's `version-…` id, each offered on the card and owned there (R2725001),
//! else 400 and nothing is written. A retired full art's id (`full-art-…`, `animated-…`) is a 400, and so is a
//! non-empty `augments`: the augments are retired, and `null` or `[]` only clears what a pick held from before.
//! Every write of `fullArt` clears the augments held, so a card's pick never names a level and an augment both.
//!
//! `POST /api/card-picks/upload` is a device's one-time upload of the picks it held before they moved to
//! the account. A card the account already holds a row for is never touched, so a newer device's pick
//! stands; an unknown id is kept (R1655000), and one that could never be worn is left out.

use std::sync::Arc;

use serde_json::{Map, Value, json};

use crate::api::collection::caller_profile;
use crate::api::http::{ApiError, ApiResult, Req, bad_request, now_ms, ok_of, str};
use crate::app::App;
use crate::config::PLAYER_SETTINGS_TEXT_MAX_LENGTH;
use crate::cosmetics::{
    self, ANIMATED_ART_ID_PREFIX, AUGMENT_ID_PREFIX, CosmeticKind, FULL_ART_ID_PREFIX, PickRefusal,
    VERSION_FULL_ART_ID, VERSION_ID_PREFIX, full_art_of, is_slug, level_of_pick, worn_level,
};
use crate::economy::store::{CardPicks, FullArtPick, OwnedCosmetics};
use crate::loadout::{AUGMENTS_KEY, CARD_SLOT_KINDS, CardPicksExt, FULL_ART_SETTING, setting_key};

/// A card a deck may hold: in the catalog, and no token.
pub(crate) fn is_playable(app: &App, card_id: &str) -> bool {
    app.catalog.card_ids.iter().any(|id| id == card_id) && !app.catalog.is_token(card_id)
}

/// R3015005: what a write of `card_id`'s pick of a level comes to against the account's ledger: `false` always
/// passes, `true` when the card is a full-art card (R2655000) offered and owning Full art's first level, and an
/// id when it is a level the card is offered and owns there (R2725001). A retired full art's id is no level.
fn check_full_art(card_id: &str, pick: &FullArtPick, owned: &OwnedCosmetics) -> Result<(), PickRefusal> {
    let registry = cosmetics::registry();
    let level = match pick {
        FullArtPick::On(false) => return Ok(()),
        FullArtPick::On(true) if full_art_of(registry, card_id).is_none() => return Err(PickRefusal::WrongKind),
        FullArtPick::On(true) => VERSION_FULL_ART_ID,
        FullArtPick::Id(id) => id,
    };
    cosmetics::check_pick(registry, owned, CosmeticKind::Version, level, Some(card_id))
}

/// R3015005: what a `PUT` naming a retired full art in `fullArt` answers: a card's pick names a level.
const RETIRED_REFUSAL: &str = "that cosmetic is retired: \"fullArt\" names a level";

/// R3015005: what a `PUT` listing augments answers: they are retired, each level a ladder's rung of its own.
const AUGMENTS_REFUSAL: &str = "\"augments\" are retired: a card wears one level, named in \"fullArt\"";

/// R2725000: whether `id` is an augment's well-shaped id, as a card pick's `augments` lists one.
fn is_augment_id(id: &str) -> bool {
    is_id_shape(id) && id.starts_with(AUGMENT_ID_PREFIX)
}

/// R2725000: an `augments` value as a `PUT` or an upload sends it: a list of distinct well-shaped augment ids.
fn augments_pick_of(value: &Value) -> Option<Vec<String>> {
    let mut ids: Vec<String> = Vec::new();
    for item in value.as_array()? {
        let id = item.as_str().filter(|id| is_augment_id(id))?;
        if ids.iter().any(|held| held == id) {
            return None;
        }
        ids.push(id.to_string());
    }
    Some(ids)
}

/// A `fullArt` value as a `PUT` or an upload sends it: a boolean, or a well-shaped full art's id,
/// `full-art-…` (R2655000) or `animated-…` (R2715000), or a version's, `version-…` (R2725000).
fn full_art_pick_of(value: &Value) -> Option<FullArtPick> {
    match value {
        Value::Bool(on) => Some(FullArtPick::On(*on)),
        Value::String(id)
            if is_id_shape(id)
                && [FULL_ART_ID_PREFIX, ANIMATED_ART_ID_PREFIX, VERSION_ID_PREFIX]
                    .iter()
                    .any(|prefix| id.starts_with(prefix)) =>
        {
            Some(FullArtPick::Id(id.clone()))
        }
        _ => None,
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
    let full_art = change_of(
        &req.body,
        FULL_ART_SETTING,
        full_art_pick_of,
        "a boolean or a full-art id",
    )?;
    let augments = change_of(
        &req.body,
        AUGMENTS_KEY,
        augments_pick_of,
        "an array of augment ids",
    )?;
    if matches!(&augments, Some(Some(ids)) if !ids.is_empty()) {
        return Err(bad_request(AUGMENTS_REFUSAL));
    }
    if matches!(&full_art, Some(Some(FullArtPick::Id(id))) if !id.starts_with(VERSION_ID_PREFIX)) {
        return Err(bad_request(RETIRED_REFUSAL));
    }

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
    if let Some(Some(pick)) = &full_art {
        check_full_art(&card_id, pick, &owned)
            .map_err(|refusal| bad_request(refusal.message(FULL_ART_SETTING)))?;
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
    // R3015005: a level written, or the augments cleared, leaves no augment held from before #3015.
    if let Some(pick) = full_art {
        picks.full_art = pick;
        picks.augments.clear();
    }
    if augments.is_some() {
        picks.augments.clear();
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

/// One entry of an upload, read whole or refused: a card id, and each pick a well-shaped id, the
/// full art's a boolean or a well-shaped id (R2655000), the augments a list of distinct augment ids (R2725000).
fn upload_entry(entry: &Value) -> Result<CardPicks, ApiError> {
    let refused = || {
        bad_request(
            "every entry of \"cards\" must be { cardId, foil?, finish?, border?, fullArt?, augments? }",
        )
    };
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
        Some(value) => picks.full_art = Some(full_art_pick_of(value).ok_or_else(refused)?),
    }
    match fields.get(AUGMENTS_KEY) {
        None | Some(Value::Null) => {}
        Some(value) => picks.augments = augments_pick_of(value).ok_or_else(refused)?,
    }
    Ok(picks)
}

/// R1834000: what of an uploaded entry can be kept: an id this build does not know stays, as the
/// device kept it; one it knows is kept only where the account owns it on the card, under its kind.
/// R3015005: the pick of a look is kept as the level it names ([`level_of_pick`], an older shape mapped), or the
/// highest below it, that the card is offered and owns ([`worn_level`]); a known id naming no level of the card is
/// dropped, `false`, `true` on a card with no full art and an unknown id stay as they are, and no augment stays.
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
    let card = picks.card_id.clone();
    let named = level_of_pick(registry, &card, picks.full_art.as_ref(), &picks.augments);
    picks.full_art = match named {
        Some(_) => worn_level(registry, named, |level| {
            cosmetics::check_pick(registry, owned, CosmeticKind::Version, &level.id, Some(&card)).is_ok()
        })
        .map(|level| FullArtPick::Id(level.id.clone())),
        None => picks.full_art.filter(
            |pick| !matches!(pick, FullArtPick::Id(id) if registry.iter().any(|cosmetic| cosmetic.id == *id)),
        ),
    };
    picks.augments.clear();
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
            full_art: Some(FullArtPick::On(false)),
            augments: Vec::new(),
        };
        let kept = keepable(picks.clone(), &none);
        assert_eq!(kept.foil.as_deref(), Some("foil-from-a-newer-client"));
        assert_eq!(kept.finish, None);
        assert_eq!(kept.border.as_deref(), Some("border-standard"));
        assert_eq!(kept.full_art, Some(FullArtPick::On(false)));
        // A full art switched on stays only where the card's full art is owned, or unknown here.
        let full_art = cosmetics::registry()
            .iter()
            .find(|cosmetic| cosmetic.kind == CosmeticKind::FullArt)
            .expect("the registry ships a full art");
        assert_eq!(full_art.rarity, crate::cosmetics::CosmeticRarity::Epic);
        let card = full_art.card.clone().expect("a full art names its card");
        let on = CardPicks {
            card_id: card.clone(),
            full_art: Some(FullArtPick::On(true)),
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
        assert_eq!(keepable(on, &owned).full_art, Some(FullArtPick::On(true)));
        let unknown_here = CardPicks {
            card_id: "core-999".to_string(),
            full_art: Some(FullArtPick::On(true)),
            ..CardPicks::default()
        };
        assert_eq!(
            keepable(unknown_here, &none).full_art,
            Some(FullArtPick::On(true))
        );
    }

    #[test]
    fn r2655000_an_upload_keeps_an_owned_or_unknown_full_art_id_and_drops_another_card_s_or_an_unowned_one() {
        let vanilla = |id: &str| CardPicks {
            card_id: "core-008".to_string(),
            full_art: Some(FullArtPick::Id(id.to_string())),
            ..CardPicks::default()
        };
        let owned = OwnedCosmetics {
            cards: vec![CardCosmetic {
                card_id: "core-008".to_string(),
                cosmetic_id: "full-art-mr-vanilla-signed".to_string(),
            }],
            account: Vec::new(),
        };
        let kept = |id: &str, owned: &OwnedCosmetics| keepable(vanilla(id), owned).full_art;
        let signed = Some(FullArtPick::Id("full-art-mr-vanilla-signed".to_string()));
        assert_eq!(kept("full-art-mr-vanilla-signed", &owned), signed);
        // Not owned, another card's or no full art at all: dropped.
        assert_eq!(
            kept("full-art-mr-vanilla-signed", &OwnedCosmetics::default()),
            None
        );
        assert_eq!(kept("full-art-mr-vanilla-animated", &owned), None);
        assert_eq!(kept("full-art-gary-the-gambler-signed", &owned), None);
        // An id this build does not know is kept, as the device kept it.
        assert_eq!(
            kept("full-art-from-a-newer-client", &OwnedCosmetics::default()),
            Some(FullArtPick::Id("full-art-from-a-newer-client".to_string()))
        );
        // The wire reads a boolean or a well-shaped id, and nothing else.
        assert_eq!(full_art_pick_of(&json!(true)), Some(FullArtPick::On(true)));
        assert_eq!(
            full_art_pick_of(&json!("full-art-mr-vanilla-signed")),
            Some(FullArtPick::Id("full-art-mr-vanilla-signed".to_string()))
        );
        assert_eq!(full_art_pick_of(&json!("Full Art")), None);
        assert_eq!(full_art_pick_of(&json!("foil-rainbow")), None);
        assert_eq!(full_art_pick_of(&json!(1)), None);
    }

    #[test]
    fn r2715000_an_animated_art_id_is_a_full_art_pick() {
        // The wire reads an Animated art's well-shaped id as a full art's.
        let animated = || Some(FullArtPick::Id("animated-mr-vanilla".to_string()));
        assert_eq!(full_art_pick_of(&json!("animated-mr-vanilla")), animated());
        assert_eq!(full_art_pick_of(&json!("animated-Mr")), None);
        assert_eq!(
            full_art_pick_of(&json!(format!(
                "animated-{}",
                "a".repeat(PLAYER_SETTINGS_TEXT_MAX_LENGTH)
            ))),
            None
        );
        assert_eq!(full_art_pick_of(&json!("animated")), None);
        // Owned on its card, it is a pick; `true` still needs the still Epic, which it is not.
        let owned = OwnedCosmetics {
            cards: vec![CardCosmetic {
                card_id: "core-008".to_string(),
                cosmetic_id: "animated-mr-vanilla".to_string(),
            }],
            account: Vec::new(),
        };
        let id = FullArtPick::Id("animated-mr-vanilla".to_string());
        assert_eq!(check_full_art("core-008", &id, &owned), Ok(()));
        assert_eq!(
            check_full_art("core-008", &FullArtPick::On(true), &owned),
            Err(PickRefusal::NotOwned)
        );
        assert_eq!(
            check_full_art("core-008", &id, &OwnedCosmetics::default()),
            Err(PickRefusal::NotOwned)
        );
        assert_eq!(
            check_full_art("core-004", &id, &owned),
            Err(PickRefusal::WrongKind)
        );
        // An upload keeps it owned and drops it unowned or on another card.
        let upload = |card: &str, owned: &OwnedCosmetics| {
            keepable(
                CardPicks {
                    card_id: card.to_string(),
                    full_art: animated(),
                    ..CardPicks::default()
                },
                owned,
            )
            .full_art
        };
        assert_eq!(upload("core-008", &owned), animated());
        assert_eq!(upload("core-008", &OwnedCosmetics::default()), None);
        assert_eq!(upload("core-004", &owned), None);
    }

    #[test]
    fn r2949000_an_upload_keeps_signature_only_beside_full_art_and_animated() {
        use crate::cosmetics::VERSION_FULL_ART_ID;
        // Carnivorous Cube, offered all three, owning each.
        let cube = "core-022";
        let row = |id: &str| CardCosmetic {
            card_id: cube.to_string(),
            cosmetic_id: id.to_string(),
        };
        let all = OwnedCosmetics {
            cards: vec![
                row(VERSION_FULL_ART_ID),
                row(AUGMENT_ANIMATED_ID),
                row(AUGMENT_SIGNATURE_ID),
            ],
            account: Vec::new(),
        };
        let kept = |full_art: FullArtPick, augments: &[&str], owned: &OwnedCosmetics| {
            keepable(
                CardPicks {
                    card_id: cube.to_string(),
                    full_art: Some(full_art),
                    augments: augments.iter().map(|id| (*id).to_string()).collect(),
                    ..CardPicks::default()
                },
                owned,
            )
            .augments
        };
        let version = || FullArtPick::Id(VERSION_FULL_ART_ID.to_string());
        let both = [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID];
        assert_eq!(kept(version(), &both, &all), both);
        assert!(kept(version(), &[AUGMENT_SIGNATURE_ID], &all).is_empty());
        assert_eq!(kept(FullArtPick::On(false), &both, &all), [AUGMENT_ANIMATED_ID]);
        // The version not owned: the version goes, and Signature with it.
        let no_version = OwnedCosmetics {
            cards: vec![row(AUGMENT_ANIMATED_ID), row(AUGMENT_SIGNATURE_ID)],
            account: Vec::new(),
        };
        assert_eq!(kept(version(), &both, &no_version), [AUGMENT_ANIMATED_ID]);
        // Animated not owned: Signature goes with it.
        let no_animated = OwnedCosmetics {
            cards: vec![row(VERSION_FULL_ART_ID), row(AUGMENT_SIGNATURE_ID)],
            account: Vec::new(),
        };
        assert!(kept(version(), &both, &no_animated).is_empty());
    }
}
