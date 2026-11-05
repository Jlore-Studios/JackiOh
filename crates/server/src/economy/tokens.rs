//! The token bars and their tokens (SPEC §14.5; #1305): what a full bar pays, what a token
//! buys and what both read back.
//!
//! A pack opening pays its bars through [`settle_bar`] and [`settle_cosmetic_bar`] (step by step,
//! each step at its own size, R2728000) in its own transaction (`economy::packs::open_pack`,
//! R1305000, R1305001); the cycle's stored position is the `CosmeticTokenCycle` counter. Every
//! redemption runs under the account's lock (R1301003), so a token spent and a bar paying cannot
//! interleave, and commits whole or not at all (§14.11).

use std::sync::Arc;

use indexmap::IndexMap;
use jackioh_engine::config::MAX_COPIES;
use jackioh_engine::set_ships;
use jackioh_engine::wire::PrintedRarity;
use serde::{Deserialize, Serialize};

use crate::api::collection::{caller_profile, owned_in};
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, ok_of, optional_str, str};
use crate::app::App;
use crate::config::{
    CARD_TOKEN_BAR_SIZE, CosmeticTokenStep, TOKEN_BAR_POINTS_BY_CARD_RARITY,
    TOKEN_BAR_POINTS_BY_COSMETIC_RARITY,
};
use crate::cosmetics::{Cosmetic, CosmeticKind, CosmeticRarity, offered, row_owns};
use crate::db::store::{CollectionEntry, CollectionGrant, Tx};
use crate::economy::idempotency::{RequestKey, Spent, keep, replayed};
use crate::economy::packs::{bar_before, cycle_at};
use crate::economy::store::{
    CosmeticGrant, Counter, EconomyReason, InventoryEntry, InventoryGrant, InventoryItem, OwnedCosmetics,
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

/// R2728000: the cosmetic token bar settled step by step from the cycle's stored `position`: each
/// step pays when the points reach its own size (`CosmeticTokenStep::bar_size`), taken off before
/// the next step's size is compared, so one gain can pay several steps of sizes that differ, and
/// a bar past its step's size settles whole (R1305000). The steps paid, in order, and the points
/// kept.
pub fn settle_cosmetic_bar(start: i64, gained: i64, position: i64) -> (Vec<CosmeticTokenStep>, i64) {
    let mut points = start + gained;
    let mut paid = Vec::new();
    loop {
        let step = cycle_at(position + paid.len() as i64);
        if points < step.bar_size() {
            return (paid, points);
        }
        points -= step.bar_size();
        paid.push(step);
    }
}

/// The points each opened slot of a card pack adds to each token bar, whatever it grants: its
/// slot's card rarity's, pity's force included, in `TOKEN_BAR_POINTS_BY_CARD_RARITY` (R1953000,
/// R1999000).
pub fn card_bar_points(rarity: PrintedRarity) -> i64 {
    let points = TOKEN_BAR_POINTS_BY_CARD_RARITY;
    match rarity {
        PrintedRarity::Common => points.common,
        PrintedRarity::Rare => points.rare,
        PrintedRarity::Epic => points.epic,
        PrintedRarity::Legendary => points.legendary,
        PrintedRarity::Mythic => points.mythic,
    }
}

/// The points each opened slot of a cosmetic pack adds to each token bar, whatever it grants: its
/// rolled cosmetic rarity's, not the one it fell to, in `TOKEN_BAR_POINTS_BY_COSMETIC_RARITY`
/// (R1306001, R1953000, R1999000). A cosmetic rarity is never read as a card's (R1333000), so it
/// has a table of its own.
pub fn cosmetic_bar_points(rarity: CosmeticRarity) -> i64 {
    let points = TOKEN_BAR_POINTS_BY_COSMETIC_RARITY;
    match rarity {
        CosmeticRarity::Common => points.common,
        CosmeticRarity::Rare => points.rare,
        CosmeticRarity::Epic => points.epic,
        CosmeticRarity::Legendary => points.legendary,
        CosmeticRarity::Mythic => points.mythic,
    }
}

/// The inventory key a cosmetic token of `token` is counted under: its rarity's name
/// (`InventoryItem::CosmeticToken` takes a rarity key).
fn token_key(token: CosmeticRarity) -> String {
    token.as_str().to_string()
}

pub(crate) fn held(inventory: &[InventoryEntry], item: InventoryItem, key: &str) -> i64 {
    inventory
        .iter()
        .find(|entry| entry.item == item && entry.key == key)
        .map_or(0, |entry| entry.count)
}

/// §14.5: a card token unlocks any new card of a set that ships that the account chooses, in one
/// transaction, every row with the same `at`, under an `Idempotency-Key` (R2124000) when `request`
/// has one, answering the tokens after it, read in its transaction: right after the account's
/// lock, before the token held is read (a retry finds the token its first try spent gone), a key
/// already committed answers what it kept, and a redemption's answer is kept in its own
/// transaction.
pub async fn redeem_card_token_once(
    app: &App,
    profile_id: &str,
    card_id: &str,
    request: Option<&RequestKey>,
) -> Result<Spent<TokenBars>, ApiError> {
    let mut tx = app.db.begin(Some(profile_id)).await?;
    tx.economy_lock_account(profile_id).await?;
    if let Some(before) = replayed(&mut tx, profile_id, request).await? {
        return Ok(before);
    }
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
    let bars = tokens_in(&mut tx, profile_id).await?;
    keep(&mut tx, profile_id, request, &bars).await?;
    tx.commit().await?;
    tracing::info!(event = "tokens.card_redeemed", "profileId" = %profile_id, card = card_id);
    Ok(Spent::Now(bars))
}

/// What a player's choice of one cosmetic, on one card or the account, comes to (R1305003,
/// R1306002).
pub(crate) struct Choice {
    /// The choice names a card the account owns (a full art, its own card alone, R1313000) for a
    /// per-card cosmetic, and no card for an account cosmetic.
    pub fits: bool,
    /// The account already owns the cosmetic there: a default or a starter, which §14.2 gives
    /// every account, or a ledger row, a retired full art's owning what it implies (R2725002).
    pub owned: bool,
}

/// R2725000, R2725001: why no token or Chase Shard bar buys `cosmetic` on `card_id`, a 400's words: a retired
/// full art, which nothing sells, or a version or an augment the card is not offered. `None` when either may.
pub(crate) fn unsold(cosmetic: &Cosmetic, card_id: Option<&str>) -> Option<&'static str> {
    if cosmetic.kind == CosmeticKind::FullArt {
        Some("that cosmetic is retired")
    } else if cosmetic.kind.is_offered() && !offered(cosmetic, card_id) {
        Some("that cosmetic is not offered on that card")
    } else {
        None
    }
}

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
                .any(|held| held.card_id == card_id && row_owns(&held.cosmetic_id, &cosmetic.id)),
            None => cosmetics.account.contains(&cosmetic.id),
        };
    let card_fits = match (cosmetic.kind.is_per_card(), card_id) {
        // A per-card cosmetic fits an owned card it is offered (R2725001): a full art the one card it names
        // (R1313000), a version or an augment a card its offer takes.
        (true, Some(card_id)) => offered(cosmetic, Some(card_id)) && card_owns(card_id),
        (true, None) => false,
        (false, None) => true,
        (false, Some(_)) => false,
    };
    Choice {
        fits: card_fits,
        owned: already_owned,
    }
}

/// §14.5: a cosmetic token buys any cosmetic at its tier or below on §14.2's scale (a chase is Legendary or
/// Mythic, R1333000, so a Legendary token reaches a Legendary chase and no token a Mythic, R2728000): a
/// per-card cosmetic for a card the account owns, or an account cosmetic ([`Choice`]), in one transaction,
/// every row with the same `at`. A token outside Rare, Epic and Legendary is 400, as is an unknown cosmetic
/// id. With no key, [`redeem_cosmetic_token_once`]'s redemption.
pub async fn redeem_cosmetic_token(
    app: &App,
    profile_id: &str,
    registry: &[Cosmetic],
    token: CosmeticRarity,
    cosmetic_id: &str,
    card_id: Option<&str>,
) -> Result<(), ApiError> {
    redeem_cosmetic_token_once(app, profile_id, registry, token, cosmetic_id, card_id, None)
        .await?
        .now()?;
    Ok(())
}

/// [`redeem_cosmetic_token`] under an `Idempotency-Key` (R2124000), as [`redeem_card_token_once`] runs a
/// card token's: the key is read in its transaction right after the account's lock, before anything held
/// is read, and a redemption's answer is kept in its own transaction.
pub async fn redeem_cosmetic_token_once(
    app: &App,
    profile_id: &str,
    registry: &[Cosmetic],
    token: CosmeticRarity,
    cosmetic_id: &str,
    card_id: Option<&str>,
    request: Option<&RequestKey>,
) -> Result<Spent<TokenBars>, ApiError> {
    if !matches!(
        token,
        CosmeticRarity::Rare | CosmeticRarity::Epic | CosmeticRarity::Legendary
    ) {
        return Err(bad_request(format!("no cosmetic token is {token}")));
    }
    let cosmetic = registry
        .iter()
        .find(|entry| entry.id == cosmetic_id)
        .ok_or_else(|| bad_request(format!("no cosmetic is named \"{cosmetic_id}\"")))?;
    if let Some(why) = unsold(cosmetic, card_id) {
        return Err(bad_request(why));
    }
    let mut tx = app.db.begin(Some(profile_id)).await?;
    tx.economy_lock_account(profile_id).await?;
    if let Some(before) = replayed(&mut tx, profile_id, request).await? {
        return Ok(before);
    }
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
    let bars = tokens_in(&mut tx, profile_id).await?;
    keep(&mut tx, profile_id, request, &bars).await?;
    tx.commit().await?;
    tracing::info!(event = "tokens.cosmetic_redeemed", "profileId" = %profile_id, cosmetic = cosmetic_id);
    Ok(Spent::Now(bars))
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BarState {
    pub points: i64,
    pub size: i64,
}

/// The tokens held: a card token, a cosmetic token of each tier the cycle pays (the Legendary one
/// #2728's, R2728000), and the unopened cosmetic packs the cycle's last step pays (§14.6, opened by
/// #1306).
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenHoldings {
    pub card: i64,
    pub cosmetic_rare: i64,
    pub cosmetic_epic: i64,
    pub cosmetic_legendary: i64,
    pub cosmetic_pack: i64,
}

/// What `GET /api/tokens` reads back (R1305004): both bars' points and sizes, the cycle's next
/// step and the tokens held. The cosmetic bar's size is its next step's (R2728000). A bar never
/// written reads 0 points.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenBars {
    pub card_bar: BarState,
    pub cosmetic_bar: BarState,
    pub next_step: CosmeticTokenStep,
    pub tokens: TokenHoldings,
}

/// Reads [`TokenBars`] for `get_tokens`, in a transaction of its own ([`tokens_in`]). Read-only,
/// like the collection.
async fn read_tokens(app: &App, profile_id: &str) -> Result<TokenBars, ApiError> {
    let mut tx = app.db.begin(Some(profile_id)).await?;
    let bars = tokens_in(&mut tx, profile_id).await?;
    tx.commit().await?;
    Ok(bars)
}

/// [`TokenBars`] as `tx` reads them: the bars and the cycle's stored position from the counters,
/// the tokens from the inventory. A redemption reads its answer through it before its commit.
async fn tokens_in(tx: &mut Tx<'_>, profile_id: &str) -> Result<TokenBars, ApiError> {
    let counters = tx.economy_counters_get(profile_id).await?;
    let inventory = tx.economy_inventory_get(profile_id).await?;
    let cycle = bar_before(&counters, Counter::CosmeticTokenCycle);
    let step = cycle_at(cycle);
    Ok(TokenBars {
        card_bar: BarState {
            points: bar_before(&counters, Counter::CardTokenBar),
            size: CARD_TOKEN_BAR_SIZE,
        },
        cosmetic_bar: BarState {
            points: bar_before(&counters, Counter::CosmeticTokenBar),
            size: step.bar_size(),
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
            cosmetic_legendary: held(
                &inventory,
                InventoryItem::CosmeticToken,
                &token_key(CosmeticRarity::Legendary),
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

/// `POST /api/tokens/redeem-card` (`AuthLevel::Active`): `{ cardId }`, with an optional
/// `Idempotency-Key` (R2124000); [`redeem_card_token_once`], in the account's spend turn
/// (R2378000).
pub async fn post_redeem_card(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let card_id = str(&req.body, "cardId")?;
    let request = RequestKey::of(app, &req, "/api/tokens/redeem-card")?;
    let _turn = app.spends.claim(&profile.id).await?;
    redeem_card_token_once(app, &profile.id, &card_id, request.as_ref())
        .await?
        .answer()
}

/// `POST /api/tokens/redeem-cosmetic` (`AuthLevel::Active`): `{ token, cosmeticId, cardId? }`
/// over the shipped registry, with an optional `Idempotency-Key` (R2124000);
/// [`redeem_cosmetic_token_once`], in the account's spend turn (R2378000).
pub async fn post_redeem_cosmetic(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let token = str(&req.body, "token")?;
    let token = CosmeticRarity::named(&token)
        .ok_or_else(|| bad_request(format!("no cosmetic token is named \"{token}\"")))?;
    let cosmetic_id = str(&req.body, "cosmeticId")?;
    let card_id = optional_str(&req.body, "cardId")?;
    let request = RequestKey::of(app, &req, "/api/tokens/redeem-cosmetic")?;
    let registry = crate::cosmetics::shipped()
        .map_err(|error| ApiError::internal(format!("cosmetics.json: {error}")))?;
    let _turn = app.spends.claim(&profile.id).await?;
    redeem_cosmetic_token_once(
        app,
        &profile.id,
        &registry,
        token,
        &cosmetic_id,
        card_id.as_deref(),
        request.as_ref(),
    )
    .await?
    .answer()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CosmeticTokenStep::{CosmeticPack, Epic, Legendary, Rare};
    use crate::config::{CARD_TOKEN_BAR_SIZE, COSMETIC_TOKEN_BAR_SIZE_RARE, COSMETIC_TOKEN_CYCLE};

    #[test]
    fn r1305000_settle_bar_pays_once_and_keeps_the_overflow() {
        assert_eq!(
            settle_bar(CARD_TOKEN_BAR_SIZE - 1, 3, CARD_TOKEN_BAR_SIZE),
            (1, 2)
        );
        assert_eq!(
            settle_cosmetic_bar(COSMETIC_TOKEN_BAR_SIZE_RARE - 1, 3, 0),
            (vec![Rare], 2)
        );
    }

    #[test]
    fn r1305000_settle_bar_keeps_counting_past_its_size() {
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
    fn r1953000_points_follow_the_rarity_and_a_bar_at_195_gaining_10_pays_once_keeping_5() {
        let cards = [
            PrintedRarity::Common,
            PrintedRarity::Rare,
            PrintedRarity::Epic,
            PrintedRarity::Legendary,
            PrintedRarity::Mythic,
        ]
        .map(card_bar_points);
        let cosmetics = CosmeticRarity::ALL.map(cosmetic_bar_points);
        assert_eq!(cards, [1, 2, 3, 5, 10]);
        assert_eq!(cosmetics, [1, 2, 3, 5, 10]);
        assert_eq!(cards.iter().sum::<i64>(), 21);
        assert_eq!(settle_bar(195, 10, CARD_TOKEN_BAR_SIZE), (1, 5));
        assert_eq!(
            settle_cosmetic_bar(COSMETIC_TOKEN_BAR_SIZE_RARE - 5, 10, 0),
            (vec![Rare], 5)
        );
    }

    #[test]
    fn r2728000_each_step_pays_at_its_own_size() {
        assert_eq!(
            [Rare, Epic, Legendary, CosmeticPack].map(CosmeticTokenStep::bar_size),
            [50, 75, 100, 100]
        );
        assert_eq!(
            COSMETIC_TOKEN_CYCLE,
            [Rare, Epic, Legendary, Rare, Epic, CosmeticPack]
        );
        // One cycle: 450 points, down from #1953's 1,200.
        assert_eq!(
            COSMETIC_TOKEN_CYCLE
                .iter()
                .map(|step| step.bar_size())
                .sum::<i64>(),
            450
        );
        // At every place of the cycle, a point short pays nothing and the last point pays that
        // place's step, keeping nothing; the place is counted from the stored position, so a
        // second lap pays as the first.
        for position in 0..2 * COSMETIC_TOKEN_CYCLE.len() as i64 {
            let step = COSMETIC_TOKEN_CYCLE[position as usize % COSMETIC_TOKEN_CYCLE.len()];
            let short = step.bar_size() - 1;
            assert_eq!(
                settle_cosmetic_bar(short, 0, position),
                (vec![], short),
                "{position}"
            );
            assert_eq!(
                settle_cosmetic_bar(short, 1, position),
                (vec![step], 0),
                "{position}"
            );
        }
    }

    #[test]
    fn r2728000_overflow_carries_into_the_next_step_at_its_own_size() {
        // (start, gained, position) → (steps paid, points kept).
        assert_eq!(settle_cosmetic_bar(45, 10, 0), (vec![Rare], 5));
        // At the Epic step, 230 points pay Epic (75), Legendary (100) and Rare (50), keeping 5.
        assert_eq!(settle_cosmetic_bar(70, 160, 1), (vec![Epic, Legendary, Rare], 5));
        // The cycle wraps: the pack, then the next lap's Rare.
        assert_eq!(settle_cosmetic_bar(100, 50, 5), (vec![CosmeticPack, Rare], 0));
        // A bar the 200-point bar left at 199 (R1953000) settles whole at the Rare step: Rare and
        // Epic, keeping 74, short of Legendary's 100.
        assert_eq!(settle_cosmetic_bar(199, 0, 0), (vec![Rare, Epic], 74));
        // Two cycles' points pay both laps in order and keep nothing.
        let two_laps: Vec<CosmeticTokenStep> = COSMETIC_TOKEN_CYCLE
            .iter()
            .chain(COSMETIC_TOKEN_CYCLE.iter())
            .copied()
            .collect();
        assert_eq!(settle_cosmetic_bar(0, 900, 0), (two_laps, 0));
        assert_eq!(settle_cosmetic_bar(7, 0, 3), (vec![], 7));
    }

    #[test]
    fn r1305003_a_rare_token_buys_at_its_tier_or_below() {
        // Common, Rare, Epic, Legendary, Mythic: what each token reaches.
        for (token, buys) in [
            (CosmeticRarity::Rare, [true, true, false, false, false]),
            (CosmeticRarity::Epic, [true, true, true, false, false]),
            // R2728000: the Legendary token, a Legendary chase included, never a Mythic.
            (CosmeticRarity::Legendary, [true, true, true, true, false]),
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
