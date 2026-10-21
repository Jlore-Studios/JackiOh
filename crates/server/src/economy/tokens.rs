//! The token bars and their tokens (SPEC §14.5; #1305): what a full bar pays, what a token
//! buys and what both read back.
//!
//! - [`settle_bar`]: the pure split of a bar's points into payouts and overflow (R1305000).
//! - [`redeem_card_token`]: a card token unlocks one chosen new card of a set that ships, in one
//!   transaction (R1305002).
//! - [`redeem_cosmetic_token`]: a Rare or Epic cosmetic token buys one cosmetic at its tier or
//!   below, in one transaction (R1305003).
//! - [`read_tokens`]: both bars' points and sizes, the cosmetic cycle's next step and the tokens
//!   held (R1305004).
//! - The routes, all `AuthLevel::Active`: `GET /api/tokens`, `POST /api/tokens/redeem-card`
//!   (`{ cardId }`) and `POST /api/tokens/redeem-cosmetic` (`{ token, cosmeticId, cardId? }`).
//!
//! A pack opening pays its bars through [`settle_bar`] in its own transaction
//! (`economy::packs::open_pack`, R1305000, R1305001); the cycle's stored position is the
//! `CosmeticTokenCycle` counter. Every redemption runs under the account's lock (R1301003), so a
//! token spent and a bar paying cannot interleave, and commits whole or not at all (§14.11): a
//! refusal returns before the commit and spends nothing.

use std::sync::Arc;

use indexmap::IndexMap;
use jackioh_engine::config::MAX_COPIES;
use jackioh_engine::set_ships;
use serde::{Deserialize, Serialize};

use crate::api::collection::{caller_profile, owned_in};
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, ok_of, optional_str, str};
use crate::app::App;
use crate::config::{CARD_TOKEN_BAR_SIZE, COSMETIC_TOKEN_BAR_SIZE, COSMETIC_TOKEN_CYCLE, CosmeticTokenStep};
use crate::cosmetics::{Cosmetic, CosmeticRarity};
use crate::db::store::{CollectionEntry, CollectionGrant};
use crate::economy::store::{
    CosmeticGrant, Counter, CounterEntry, EconomyReason, InventoryEntry, InventoryGrant, InventoryItem,
    OwnedCosmetics,
};

/// The collection ledger's reason for a card a card token unlocks: migration 0002's `craft`, one
/// of its closed set. A token unlocks a chosen card exactly as a craft does, and the closed set
/// takes no new reason without a migration (R1305002); the inventory row still says
/// `token_redeem`.
pub const CARD_TOKEN_GRANT_REASON: &str = "craft";

/// A token redemption's refusal: the choice the token cannot pay for. Always 409, and always
/// before the commit, so it spends nothing (§14.5).
pub(crate) fn conflict(message: impl Into<String>) -> ApiError {
    ApiError::new(ApiErrorCode::Conflict, message)
}

/// A bar's points split into payouts and overflow (R1305000): `pays` fills at `size`, and
/// `remainder` is what the bar keeps. A bar past its size pays for every size it holds, so a bar
/// that kept counting before bars paid (R1301002) settles whole.
pub fn settle_bar(start: i64, gained: i64, size: i64) -> (i64, i64) {
    let total = start + gained;
    (total / size, total % size)
}

/// The inventory key a cosmetic token of `token` is counted under: its rarity's name
/// (`InventoryItem::CosmeticToken` takes a rarity key).
fn token_key(token: CosmeticRarity) -> String {
    token.as_str().to_string()
}

/// One counter's value: a counter never written is 0. Only the account-wide row (`""`); pity
/// counters kept per kind of pack are another sub-issue's (R1298000).
fn counter_value(counters: &[CounterEntry], counter: Counter) -> i64 {
    counters
        .iter()
        .find(|entry| entry.counter == counter && entry.pack.is_empty())
        .map_or(0, |entry| entry.value)
}

/// How many of `item` under `key` the inventory rows hold.
pub(crate) fn held(inventory: &[InventoryEntry], item: InventoryItem, key: &str) -> i64 {
    inventory
        .iter()
        .find(|entry| entry.item == item && entry.key == key)
        .map_or(0, |entry| entry.count)
}

/// §14.5: a card token unlocks any new card of a set that ships that the account chooses, in one
/// transaction: the collection's two writes (reason [`CARD_TOKEN_GRANT_REASON`], R1305002), then
/// one `CardToken` spent (reason `token_redeem`), all with the same `at`; one commit. Refuses
/// with 409, writing nothing, when the catalog names no such card, it is a token, its set does
/// not ship, the account already owns it, or no card token is held.
pub async fn redeem_card_token(app: &App, profile_id: &str, card_id: &str) -> Result<(), ApiError> {
    let mut tx = app.db.begin(Some(profile_id)).await?;
    tx.economy_lock_account(profile_id).await?;
    let usable = match app.catalog.defs.get(card_id) {
        Some(def) => !def.is_token() && set_ships(def.set),
        None => false,
    };
    if !usable {
        return Err(conflict(
            "a card token unlocks only a new card of a set that ships",
        ));
    }
    let owned = owned_in(&mut tx, profile_id).await?;
    if owned.get(card_id).copied().unwrap_or(0) >= i64::from(MAX_COPIES) {
        return Err(conflict("that card is already owned"));
    }
    if held(
        &tx.economy_inventory_get(profile_id).await?,
        InventoryItem::CardToken,
        "",
    ) < 1
    {
        return Err(conflict("no card token is held"));
    }
    let at = crate::app::now_ms();
    tx.collection_upsert_quantities(
        profile_id,
        &[CollectionEntry {
            card_id: card_id.to_string(),
            quantity: owned.get(card_id).copied().unwrap_or(0) + 1,
        }],
    )
    .await?;
    tx.collection_append_grants(&[CollectionGrant {
        profile_id: profile_id.to_string(),
        card_id: card_id.to_string(),
        delta: 1,
        reason: CARD_TOKEN_GRANT_REASON.to_string(),
        dev: false,
        at,
    }])
    .await?;
    tx.economy_inventory_grant(&InventoryGrant {
        profile_id: profile_id.to_string(),
        item: InventoryItem::CardToken,
        key: String::new(),
        delta: -1,
        reason: EconomyReason::TokenRedeem,
        reference: None,
        dev: false,
        at,
    })
    .await?
    .ok_or_else(|| ApiError::internal("the inventory refused a held card token"))?;
    tx.commit().await?;
    tracing::info!(event = "tokens.card_redeemed", "profileId" = %profile_id, card = card_id);
    Ok(())
}

/// What a player's choice of one cosmetic, on one card or the account, comes to (R1305003,
/// R1306002).
pub(crate) struct Choice {
    /// The choice names a card the account owns (a full art, its own card alone, R1313000) for a
    /// per-card cosmetic, and no card for an account cosmetic.
    pub fits: bool,
    /// The account already owns the cosmetic there: a default or a starter, which §14.2 gives
    /// every account, or a ledger row.
    pub owned: bool,
}

/// [`Choice`] of `cosmetic` on `card_id`, over the copies `owned` and the account's `cosmetics`.
pub(crate) fn choice_of(
    cosmetic: &Cosmetic,
    card_id: Option<&str>,
    owned: &IndexMap<String, i64>,
    cosmetics: &OwnedCosmetics,
) -> Choice {
    let card_owns = |card_id: &str| owned.get(card_id).copied().unwrap_or(0) >= i64::from(MAX_COPIES);
    let already_owned = cosmetic.default
        || cosmetic.starter
        || match card_id {
            Some(card_id) => cosmetics
                .cards
                .iter()
                .any(|held| held.card_id == card_id && held.cosmetic_id == cosmetic.id),
            None => cosmetics.account.contains(&cosmetic.id),
        };
    let card_fits = match (cosmetic.kind.is_per_card(), card_id, cosmetic.card.as_deref()) {
        // A full art is owned on the one card it names (R1313000): only that card, owned, takes it.
        (true, Some(card_id), Some(names)) => card_id == names && card_owns(card_id),
        (true, Some(card_id), None) => card_owns(card_id),
        (true, _, _) => false,
        (false, None, _) => true,
        (false, Some(_), _) => false,
    };
    Choice {
        fits: card_fits,
        owned: already_owned,
    }
}

/// §14.5: a cosmetic token buys any cosmetic at its tier or below (§14.2's scale, Common to
/// Mythic; a token is a [`CosmeticRarity`] too, so the two compare on that scale alone): a per-card cosmetic for a card the account owns, or
/// an account cosmetic, never a chase, in one transaction: the cosmetics grant, then one
/// `CosmeticToken` of `token`'s rarity spent (reason `token_redeem`), all with the same `at`; one
/// commit. A token outside Rare and Epic is 400, as is an unknown cosmetic id. Refuses with 409,
/// writing nothing, when no such token is held, the cosmetic is a chase, it sits above the
/// token's tier (so Legendary and Mythic are unreachable: no token exceeds Epic), a per-card
/// cosmetic names no owned card (a full art names its own card alone, R1313000), an account
/// cosmetic is given one, or it is already owned (a ledger row, or a default or starter, which
/// §14.2 gives every account).
pub async fn redeem_cosmetic_token(
    app: &App,
    profile_id: &str,
    registry: &[Cosmetic],
    token: CosmeticRarity,
    cosmetic_id: &str,
    card_id: Option<&str>,
) -> Result<(), ApiError> {
    if !matches!(token, CosmeticRarity::Rare | CosmeticRarity::Epic) {
        return Err(bad_request(format!("no cosmetic token is {token}")));
    }
    let cosmetic = registry
        .iter()
        .find(|entry| entry.id == cosmetic_id)
        .ok_or_else(|| bad_request(format!("no cosmetic is named \"{cosmetic_id}\"")))?;
    let mut tx = app.db.begin(Some(profile_id)).await?;
    tx.economy_lock_account(profile_id).await?;
    let owned = owned_in(&mut tx, profile_id).await?;
    let cosmetics = tx.economy_cosmetics_get(profile_id).await?;
    let token_held = held(
        &tx.economy_inventory_get(profile_id).await?,
        InventoryItem::CosmeticToken,
        &token_key(token),
    );
    let choice = choice_of(cosmetic, card_id, &owned, &cosmetics);
    if token_held < 1 {
        return Err(conflict("no cosmetic token of that tier is held"));
    }
    if cosmetic.chase {
        return Err(conflict("no token pays for a chase cosmetic"));
    }
    if cosmetic.rarity > token {
        return Err(conflict("that cosmetic sits above the token's tier"));
    }
    if !choice.fits {
        return Err(conflict("that cosmetic fits no card held for it"));
    }
    if choice.owned {
        return Err(conflict("that cosmetic is already owned"));
    }
    let at = crate::app::now_ms();
    let granted = tx
        .economy_cosmetics_grant(&CosmeticGrant {
            profile_id: profile_id.to_string(),
            card_id: card_id.map(str::to_string),
            cosmetic_id: cosmetic_id.to_string(),
            delta: 1,
            reason: EconomyReason::TokenRedeem,
            reference: None,
            dev: false,
            at,
        })
        .await?;
    if !granted {
        return Err(conflict("that cosmetic is already owned"));
    }
    tx.economy_inventory_grant(&InventoryGrant {
        profile_id: profile_id.to_string(),
        item: InventoryItem::CosmeticToken,
        key: token_key(token),
        delta: -1,
        reason: EconomyReason::TokenRedeem,
        reference: None,
        dev: false,
        at,
    })
    .await?
    .ok_or_else(|| ApiError::internal("the inventory refused a held cosmetic token"))?;
    tx.commit().await?;
    tracing::info!(event = "tokens.cosmetic_redeemed", "profileId" = %profile_id, cosmetic = cosmetic_id);
    Ok(())
}

/// One token bar's points and size.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BarState {
    pub points: i64,
    pub size: i64,
}

/// The tokens held: a card token, a cosmetic token of each tier the cycle pays, and the unopened
/// cosmetic packs the cycle's last step pays (§14.6, opened by #1306).
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenHoldings {
    pub card: i64,
    pub cosmetic_rare: i64,
    pub cosmetic_epic: i64,
    pub cosmetic_pack: i64,
}

/// What `GET /api/tokens` reads back (R1305004): both bars' points and sizes, the cycle's next
/// step and the tokens held. A bar never written reads 0 points.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenBars {
    pub card_bar: BarState,
    pub cosmetic_bar: BarState,
    pub next_step: CosmeticTokenStep,
    pub tokens: TokenHoldings,
}

/// Reads [`TokenBars`] for `get_tokens`: the bars and the cycle's stored position from the
/// counters, the tokens from the inventory. Read-only, like the collection.
async fn read_tokens(app: &App, profile_id: &str) -> Result<TokenBars, ApiError> {
    let mut tx = app.db.begin(Some(profile_id)).await?;
    let counters = tx.economy_counters_get(profile_id).await?;
    let inventory = tx.economy_inventory_get(profile_id).await?;
    tx.commit().await?;
    let cycle = counter_value(&counters, Counter::CosmeticTokenCycle);
    let step = COSMETIC_TOKEN_CYCLE[(cycle % COSMETIC_TOKEN_CYCLE.len() as i64) as usize];
    Ok(TokenBars {
        card_bar: BarState {
            points: counter_value(&counters, Counter::CardTokenBar),
            size: CARD_TOKEN_BAR_SIZE,
        },
        cosmetic_bar: BarState {
            points: counter_value(&counters, Counter::CosmeticTokenBar),
            size: COSMETIC_TOKEN_BAR_SIZE,
        },
        next_step: step,
        tokens: TokenHoldings {
            card: held(&inventory, InventoryItem::CardToken, ""),
            cosmetic_rare: held(
                &inventory,
                InventoryItem::CosmeticToken,
                &token_key(CosmeticRarity::Rare),
            ),
            cosmetic_epic: held(
                &inventory,
                InventoryItem::CosmeticToken,
                &token_key(CosmeticRarity::Epic),
            ),
            cosmetic_pack: held(&inventory, InventoryItem::CosmeticPack, ""),
        },
    })
}

/// `GET /api/tokens` (`AuthLevel::Active`): [`read_tokens`].
pub async fn get_tokens(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    ok_of(&read_tokens(app, &profile.id).await?)
}

/// `POST /api/tokens/redeem-card` (`AuthLevel::Active`): `{ cardId }`; [`redeem_card_token`],
/// answering the tokens after it.
pub async fn post_redeem_card(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let card_id = str(&req.body, "cardId")?;
    redeem_card_token(app, &profile.id, &card_id).await?;
    ok_of(&read_tokens(app, &profile.id).await?)
}

/// `POST /api/tokens/redeem-cosmetic` (`AuthLevel::Active`): `{ token, cosmeticId, cardId? }`
/// over the shipped registry; [`redeem_cosmetic_token`], answering the tokens after it.
pub async fn post_redeem_cosmetic(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let token = str(&req.body, "token")?;
    let token = CosmeticRarity::named(&token)
        .ok_or_else(|| bad_request(format!("no cosmetic token is named \"{token}\"")))?;
    let cosmetic_id = str(&req.body, "cosmeticId")?;
    let card_id = optional_str(&req.body, "cardId")?;
    let registry = crate::cosmetics::shipped()
        .map_err(|error| ApiError::internal(format!("cosmetics.json: {error}")))?;
    redeem_cosmetic_token(
        app,
        &profile.id,
        &registry,
        token,
        &cosmetic_id,
        card_id.as_deref(),
    )
    .await?;
    ok_of(&read_tokens(app, &profile.id).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CARD_TOKEN_BAR_SIZE, COSMETIC_TOKEN_BAR_SIZE};

    #[test]
    fn r1305000_settle_bar_pays_once_and_keeps_the_overflow() {
        assert_eq!(
            settle_bar(CARD_TOKEN_BAR_SIZE - 1, 3, CARD_TOKEN_BAR_SIZE),
            (1, 2)
        );
        assert_eq!(
            settle_bar(COSMETIC_TOKEN_BAR_SIZE - 1, 3, COSMETIC_TOKEN_BAR_SIZE),
            (1, 2)
        );
    }

    #[test]
    fn r1305000_settle_bar_keeps_counting_past_its_size() {
        // A bar that kept counting before bars paid (R1301002) settles whole: every size it
        // holds pays, and the remainder keeps counting.
        assert_eq!(
            settle_bar(CARD_TOKEN_BAR_SIZE + 2, 1, CARD_TOKEN_BAR_SIZE),
            (1, 3)
        );
        assert_eq!(
            settle_bar(0, CARD_TOKEN_BAR_SIZE * 2 + 1, CARD_TOKEN_BAR_SIZE),
            (2, 1)
        );
        assert_eq!(settle_bar(7, 0, CARD_TOKEN_BAR_SIZE), (0, 7));
    }

    #[test]
    fn r1305003_a_rare_token_buys_at_its_tier_or_below() {
        // Common, Rare, Epic, Legendary, Mythic: what each token reaches.
        for (token, buys) in [
            (CosmeticRarity::Rare, [true, true, false, false, false]),
            (CosmeticRarity::Epic, [true, true, true, false, false]),
        ] {
            for (rarity, buys) in CosmeticRarity::ALL.into_iter().zip(buys) {
                assert_eq!(rarity <= token, buys, "a {token} token and a {rarity} cosmetic");
                assert_eq!(
                    rarity.tier() <= token.tier(),
                    buys,
                    "{token} and {rarity} by tier"
                );
            }
        }
    }
}
