//! The dev tools (SPEC §14.10, R1300000): a dev account, one whose profile id `DEV_PROFILE_IDS`
//! lists (`env.rs`), moves its own economy by hand, so each part of §14 can be tried on staging and
//! production. Every route acts on the caller's own account, named by the token, and none takes a
//! profile id.
//!
//! The server decides (CLAUDE.md rule 7): `GET /api/auth/me` says whether the caller is a dev
//! account (`isDev`), and every route here checks the list itself and answers anyone else 403
//! `forbidden`, writing nothing. The client only chooses whether to show its panel.
//!
//! Each action is one transaction, and every row it writes is a ledger row of reason `dev` marked
//! `dev` (§14.10, §14.11): the collection's `collection_grants` under the reason
//! [`DEV_GRANT_REASON`] (migration 0046), each economy ledger under `JloreReason::Dev` or
//! `EconomyReason::Dev`. A clear is an offsetting row per holding, never a deleted one (§14.11).
//!
//! - `POST /api/dev/xp` `{ setId, xp }`: XP to the track of a set that ships, through the level
//!   curve (§14.7), its level stopping at `SET_TRACK_LEVELS`. No level's rewards are paid: the
//!   reward tables are the tracks' sub-issue's.
//! - `POST /api/dev/jlore` `{ jlore }`: Jlore (§14.1).
//! - `POST /api/dev/fill`: every collectable card (non-token, unbanned, of a set that ships) up to
//!   `MAX_COPIES`, and every cosmetic of the registry but each kind's default, which every account
//!   owns with no row (§14.2): a per-card kind on each of those cards, a full art on the card it
//!   names (R1313000), an account kind once.
//! - `POST /api/dev/clear/:part`: `cards`, `cosmetics`, `jlore`, `counters` (the bars and pity
//!   counters), `tracks`, or `everything`, all five in one transaction. Packs, tokens and quests are
//!   no part of any.

use std::sync::Arc;

use serde_json::{Value, json};

use jackioh_engine::config::MAX_COPIES;
use jackioh_engine::{SetName, set_ships};

use crate::api::collection::{caller_profile, owned_in};
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, json, str};
use crate::app::{App, now_ms};
use crate::config::{SET_TRACK_FIRST_LEVEL, SET_TRACK_LEVELS};
use crate::cosmetics::CosmeticKind;
use crate::db::store::{CollectionEntry, CollectionGrant, Profile, Tx};
use crate::economy::math::level_for_xp;
use crate::economy::store::{
    CosmeticGrant, CounterGrant, EconomyReason, JloreGrant, JloreReason, SetTrack, TrackGrant,
};

/// R1300000: `collection_grants.reason` for a dev account's own card grant or clear (migration
/// 0046), which its `collection_grants_dev_check` writes only with the `dev` mark.
pub const DEV_GRANT_REASON: &str = "dev";

/// R1300000: the 403 every dev route answers an account the list does not name.
const DEV_ONLY_MESSAGE: &str = "Only a dev account may use the dev tools.";

/// A grant the store answers `None` to: a sum past what a holding's column holds.
const TOO_MUCH_MESSAGE: &str = "That is more than this account can hold.";

/// R1300000: whether `DEV_PROFILE_IDS` lists `profile_id`.
pub fn is_dev(app: &App, profile_id: &str) -> bool {
    app.env.dev_profile_ids.iter().any(|id| id == profile_id)
}

/// The caller, when it is a dev account; 403 `forbidden` otherwise. Every handler here asks first.
fn dev_caller(app: &App, req: &Req) -> Result<Profile, ApiError> {
    let profile = caller_profile(req)?;
    if !is_dev(app, &profile.id) {
        return Err(ApiError::new(ApiErrorCode::Forbidden, DEV_ONLY_MESSAGE));
    }
    Ok(profile)
}

/// `body[key]` as a whole number above 0.
fn amount_of(body: &Value, key: &str) -> Result<i64, ApiError> {
    match body.get(key).and_then(Value::as_i64) {
        Some(amount) if amount > 0 => Ok(amount),
        _ => Err(bad_request(format!("\"{key}\" must be a positive whole number"))),
    }
}

/// §14: the cards a grant may hold: no token, nothing banned (§9.4 L3, L6), no set that has not
/// shipped (R1420), in catalog order.
fn collectable_cards(app: &App) -> Vec<&str> {
    app.catalog
        .card_ids
        .iter()
        .filter(|card_id| {
            !app.catalog.is_token(card_id)
                && !app.catalog.is_banned(card_id)
                && app
                    .catalog
                    .defs
                    .get(*card_id)
                    .is_some_and(|def| set_ships(def.set))
        })
        .map(String::as_str)
        .collect()
}

/// §14.2: a cosmetic owned for one card (a foil, a finish, a border), not for the whole account.
fn per_card(kind: CosmeticKind) -> bool {
    matches!(
        kind,
        CosmeticKind::Foil | CosmeticKind::Finish | CosmeticKind::Border
    )
}

/// `POST /api/dev/xp` with `{ setId, xp }`: 200 `{ track }`, the track after the grant.
pub async fn give_xp(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = dev_caller(app, &req)?;
    let set_id = str(&req.body, "setId")?;
    if !set_id.parse::<SetName>().is_ok_and(set_ships) {
        return Err(bad_request(format!(
            "\"setId\" must name a set that ships (got {})",
            crate::env::quoted(&set_id)
        )));
    }
    let xp = amount_of(&req.body, "xp")?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let (held_xp, held_level) = tx
        .economy_tracks_get(&profile.id)
        .await?
        .into_iter()
        .find(|track| track.set_id == set_id)
        .map_or((0, SET_TRACK_FIRST_LEVEL), |track| (track.xp, track.level));
    let xp_after = held_xp
        .checked_add(xp)
        .ok_or_else(|| bad_request(TOO_MUCH_MESSAGE))?;
    // §14.7: a track has `SET_TRACK_LEVELS` levels; a gift of XP never lowers one.
    let level_after = i64::from(level_for_xp(xp_after).min(SET_TRACK_LEVELS)).max(held_level);
    let grant = TrackGrant {
        profile_id: profile.id.clone(),
        set_id,
        xp,
        levels: level_after - held_level,
        rewards: 0,
        reason: EconomyReason::Dev,
        reference: None,
        dev: true,
        at: now_ms(),
    };
    let track: SetTrack = tx
        .economy_tracks_grant(&grant)
        .await?
        .ok_or_else(|| bad_request(TOO_MUCH_MESSAGE))?;
    tx.commit().await?;
    log_dev(&profile, "xp", 1);
    Ok(json(200, json!({ "track": track })))
}

/// `POST /api/dev/jlore` with `{ jlore }`: 200 `{ jlore }`, the balance after the grant.
pub async fn give_jlore(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = dev_caller(app, &req)?;
    let delta = amount_of(&req.body, "jlore")?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let balance = tx
        .economy_jlore_grant(&JloreGrant {
            profile_id: profile.id.clone(),
            delta,
            reason: JloreReason::Dev,
            reference: None,
            dev: true,
            at: now_ms(),
        })
        .await?
        .ok_or_else(|| bad_request(TOO_MUCH_MESSAGE))?;
    tx.commit().await?;
    log_dev(&profile, "jlore", 1);
    Ok(json(200, json!({ "jlore": balance })))
}

/// `POST /api/dev/fill`: 200 `{ cards, cosmetics }`, the rows written to the collection's ledger
/// and to the cosmetics'. What is owned already is left as it is, so a second fill writes nothing.
pub async fn fill(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = dev_caller(app, &req)?;
    let cosmetics = crate::cosmetics::shipped().map_err(ApiError::internal)?;
    let cards = collectable_cards(app);
    let at = now_ms();
    let max_copies = i64::from(MAX_COPIES);
    let mut tx = app.db.begin(Some(&profile.id)).await?;

    let owned = owned_in(&mut tx, &profile.id).await?;
    let mut quantities: Vec<CollectionEntry> = Vec::new();
    let mut grants: Vec<CollectionGrant> = Vec::new();
    for card_id in &cards {
        let held = owned.get(*card_id).copied().unwrap_or(0);
        if held >= max_copies {
            continue;
        }
        quantities.push(CollectionEntry {
            card_id: card_id.to_string(),
            quantity: max_copies,
        });
        grants.push(dev_card_grant(&profile.id, card_id, max_copies - held, at));
    }
    if !grants.is_empty() {
        tx.collection_upsert_quantities(&profile.id, &quantities).await?;
        tx.collection_append_grants(&grants).await?;
    }

    let mut cosmetics_written = 0;
    for cosmetic in cosmetics.iter().filter(|cosmetic| !cosmetic.default) {
        let targets: Vec<Option<&str>> = if per_card(cosmetic.kind) {
            cards.iter().copied().map(Some).collect()
        } else if cosmetic.kind == CosmeticKind::FullArt {
            // R1313000: a full art is owned on the one card it names.
            vec![cosmetic.card.as_deref()]
        } else {
            vec![None]
        };
        for card_id in targets {
            let granted = tx
                .economy_cosmetics_grant(&dev_cosmetic_grant(&profile.id, card_id, &cosmetic.id, 1, at))
                .await?;
            cosmetics_written += usize::from(granted);
        }
    }
    tx.commit().await?;
    log_dev(&profile, "fill", grants.len() + cosmetics_written);
    Ok(json(
        200,
        json!({ "cards": grants.len(), "cosmetics": cosmetics_written }),
    ))
}

/// What a clear names (§14.10): each holding apart, or all of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Holding {
    Cards,
    Cosmetics,
    Jlore,
    Counters,
    Tracks,
}

impl Holding {
    const ALL: [Holding; 5] = [
        Holding::Cards,
        Holding::Cosmetics,
        Holding::Jlore,
        Holding::Counters,
        Holding::Tracks,
    ];
}

/// The holdings a clear's `:part` names, or `None` for no clear.
fn holdings_of(part: &str) -> Option<&'static [Holding]> {
    Some(match part {
        "cards" => &[Holding::Cards],
        "cosmetics" => &[Holding::Cosmetics],
        "jlore" => &[Holding::Jlore],
        "counters" => &[Holding::Counters],
        "tracks" => &[Holding::Tracks],
        "everything" => &Holding::ALL,
        _ => return None,
    })
}

/// `POST /api/dev/clear/:part`: 200 `{ written }`, the ledger rows the clear wrote; 404 for a part
/// that names no clear.
pub async fn clear(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = dev_caller(app, &req)?;
    let part = req.params.get("part").map(String::as_str).unwrap_or("");
    let Some(holdings) = holdings_of(part) else {
        return Err(ApiError::new(ApiErrorCode::NotFound, "no such clear"));
    };
    let at = now_ms();
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let mut written = 0;
    for holding in holdings {
        written += match holding {
            Holding::Cards => clear_cards(&mut tx, &profile.id, at).await?,
            Holding::Cosmetics => clear_cosmetics(&mut tx, &profile.id, at).await?,
            Holding::Jlore => clear_jlore(&mut tx, &profile.id, at).await?,
            Holding::Counters => clear_counters(&mut tx, &profile.id, at).await?,
            Holding::Tracks => clear_tracks(&mut tx, &profile.id, at).await?,
        };
    }
    tx.commit().await?;
    log_dev(&profile, part, written);
    Ok(json(200, json!({ "written": written })))
}

/// Every card owned to 0, each by its offsetting row.
async fn clear_cards(tx: &mut Tx<'_>, profile_id: &str, at: i64) -> Result<usize, ApiError> {
    let owned: Vec<CollectionEntry> = tx
        .collection_get(profile_id)
        .await?
        .into_iter()
        .filter(|entry| entry.quantity > 0)
        .collect();
    if owned.is_empty() {
        return Ok(0);
    }
    let grants: Vec<CollectionGrant> = owned
        .iter()
        .map(|entry| dev_card_grant(profile_id, &entry.card_id, -entry.quantity, at))
        .collect();
    let emptied: Vec<CollectionEntry> = owned
        .into_iter()
        .map(|entry| CollectionEntry {
            card_id: entry.card_id,
            quantity: 0,
        })
        .collect();
    tx.collection_upsert_quantities(profile_id, &emptied).await?;
    tx.collection_append_grants(&grants).await?;
    Ok(grants.len())
}

/// Every cosmetic owned, on a card or on the account, cleared.
async fn clear_cosmetics(tx: &mut Tx<'_>, profile_id: &str, at: i64) -> Result<usize, ApiError> {
    let owned = tx.economy_cosmetics_get(profile_id).await?;
    let mut written = 0;
    let on_cards = owned
        .cards
        .iter()
        .map(|owned| (Some(owned.card_id.as_str()), owned.cosmetic_id.as_str()));
    let on_account = owned
        .account
        .iter()
        .map(|cosmetic_id| (None, cosmetic_id.as_str()));
    for (card_id, cosmetic_id) in on_cards.chain(on_account) {
        let cleared = tx
            .economy_cosmetics_grant(&dev_cosmetic_grant(profile_id, card_id, cosmetic_id, -1, at))
            .await?;
        written += usize::from(cleared);
    }
    Ok(written)
}

/// The wallet to 0.
async fn clear_jlore(tx: &mut Tx<'_>, profile_id: &str, at: i64) -> Result<usize, ApiError> {
    let balance = tx.economy_jlore_get(profile_id).await?;
    if balance <= 0 {
        return Ok(0);
    }
    let cleared = tx
        .economy_jlore_grant(&JloreGrant {
            profile_id: profile_id.to_string(),
            delta: -balance,
            reason: JloreReason::Dev,
            reference: None,
            dev: true,
            at,
        })
        .await?;
    Ok(usize::from(cleared.is_some()))
}

/// Every pity counter, bar and cycle (§14.4–§14.6) to 0.
async fn clear_counters(tx: &mut Tx<'_>, profile_id: &str, at: i64) -> Result<usize, ApiError> {
    let mut written = 0;
    for entry in tx.economy_counters_get(profile_id).await? {
        if entry.value <= 0 {
            continue;
        }
        let cleared = tx
            .economy_counters_grant(&CounterGrant {
                profile_id: profile_id.to_string(),
                counter: entry.counter,
                pack: entry.pack,
                delta: -entry.value,
                reason: EconomyReason::Dev,
                reference: None,
                dev: true,
                at,
            })
            .await?;
        written += usize::from(cleared.is_some());
    }
    Ok(written)
}

/// Every track back to its start: its first level, no XP, nothing paid (§14.7).
async fn clear_tracks(tx: &mut Tx<'_>, profile_id: &str, at: i64) -> Result<usize, ApiError> {
    let mut written = 0;
    for track in tx.economy_tracks_get(profile_id).await? {
        let grant = TrackGrant {
            profile_id: profile_id.to_string(),
            set_id: track.set_id,
            xp: -track.xp,
            levels: SET_TRACK_FIRST_LEVEL - track.level,
            rewards: -track.rewards_claimed,
            reason: EconomyReason::Dev,
            reference: None,
            dev: true,
            at,
        };
        if grant.xp == 0 && grant.levels == 0 && grant.rewards == 0 {
            continue;
        }
        let cleared = tx.economy_tracks_grant(&grant).await?;
        written += usize::from(cleared.is_some());
    }
    Ok(written)
}

/// A `collection_grants` row of a dev tool: `delta` copies of `card_id`, reason `dev`, marked.
fn dev_card_grant(profile_id: &str, card_id: &str, delta: i64, at: i64) -> CollectionGrant {
    CollectionGrant {
        profile_id: profile_id.to_string(),
        card_id: card_id.to_string(),
        delta,
        reason: DEV_GRANT_REASON.to_string(),
        dev: true,
        at,
    }
}

/// A `cosmetic_grants` row of a dev tool: `cosmetic_id` owned (+1) or cleared (−1), on `card_id` or
/// on the account.
fn dev_cosmetic_grant(
    profile_id: &str,
    card_id: Option<&str>,
    cosmetic_id: &str,
    delta: i64,
    at: i64,
) -> CosmeticGrant {
    CosmeticGrant {
        profile_id: profile_id.to_string(),
        card_id: card_id.map(str::to_string),
        cosmetic_id: cosmetic_id.to_string(),
        delta,
        reason: EconomyReason::Dev,
        reference: None,
        dev: true,
        at,
    }
}

fn log_dev(profile: &Profile, tool: &str, written: usize) {
    tracing::info!(
        event = "dev.tool",
        "profileId" = %profile.id,
        tool = %tool,
        written,
    );
}
