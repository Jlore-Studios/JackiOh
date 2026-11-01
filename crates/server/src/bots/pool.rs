//! The bot pool (SPEC §9.12, R1454): active ladder bot count, identities, decks and trios.
//!
//! The pool holds `clamp(BOT_POOL_MIN, BOT_POOL_PER_WEEKLY_ACTIVE × weekly active people, BOT_POOL_MAX)`
//! active bots, sized at boot and every refresh. An excess retires the youngest, which finish their
//! match; a shortfall reinstates the latest retired first, so the ladder meets the same faces.
//! New bots draw strength (R610), persona (R645), schedule, and username (`bots::names`).
//! Bo1 decks draw from human decklists and random decks (R750000, R1453), each kept only when legal
//! and dealt again on a new catalog or deck size (R1705001), then the Conquest trio (R641000, R253).
//! Refreshes draw cosmetics (R1834005), kept on the `bots` row and never in the ledger, and manage
//! top-bot grinding (R1555000). The queue's sweeper runs `release_stale_starts` whatever
//! `LADDER_BOTS` says, as R1450's open-row sweep would hold a bot too long.

use std::sync::Arc;
use std::time::Duration;

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::validator::{self, CatalogSnapshot, DeckInput, LoadoutDeck, LoadoutInput};
use jackioh_engine::wire::ListTotal;
use jackioh_engine::{Rng, pick_portrait_from_seed};

use crate::actor::engine::{deal_random_deck_excluding_with_strength, deal_random_deck_with_strength};
use crate::api::catalog::Catalog;
use crate::api::queue::{new_seed, new_uuid};
use crate::api::ranked::{SeasonDeps, build_season_id};
use crate::app::{App, now_ms};
use crate::bots::cosmetics;
use crate::bots::fallback::{holds_deck_size, holds_decks, holds_trio};
use crate::bots::names::draw_username;
use crate::bots::persona::pick_bot_persona;
use crate::config::{
    BOT_ACCOUNT_AGE_MAX_DAYS, BOT_ACCOUNT_AGE_MIN_DAYS, BOT_ACTIVE_HOURS_MAX, BOT_ACTIVE_HOURS_MIN,
    BOT_COSMETIC_FITS, BOT_DECK_DRAW_ATTEMPTS, BOT_DECKS_MAX, BOT_DECKS_MIN, BOT_DEFAULT_USERNAME_SHARE,
    BOT_GRIND_GAP, BOT_GRIND_HYSTERESIS, BOT_GRIND_REFERENCE_RANK, BOT_GRINDERS, BOT_HOURS_PER_DAY,
    BOT_POOL_ACTIVE_WINDOW_DAYS, BOT_POOL_MAX, BOT_POOL_MIN, BOT_POOL_PER_WEEKLY_ACTIVE,
    BOT_POOL_REFRESH_INTERVAL_SECONDS, BOT_SPEED_MAX, BOT_SPEED_MIN, BOT_STALE_START_SECONDS, BOT_STRENGTHS,
    CARD_STATS_MIN_SAMPLE, USERNAME_CHANGE_COOLDOWN_MS,
};
use crate::db::store::{
    BotDeck, BotRow, FrozenDeck, FrozenTrio, NewBot, StoreError, UsernameClaim, UsernameClaimOutcome,
};
use crate::username::{normalize_username, username_key};

// Unit conversions, not configuration.
const MS_PER_DAY: i64 = 86_400_000;
const MS_PER_SECOND: i64 = 1000;

/// R641000: the name a bot's trio is frozen under; no client reads it (R336).
const TRIO_NAME: &str = "Conquest";

/// R1454: the pool's size for `weekly` people who finished a live game in the last
/// `BOT_POOL_ACTIVE_WINDOW_DAYS`.
pub fn pool_target(weekly: i64) -> usize {
    let wanted = (weekly.max(0) as f64 * BOT_POOL_PER_WEEKLY_ACTIVE).round() as usize;
    wanted.clamp(BOT_POOL_MIN, BOT_POOL_MAX)
}

/// R1454: what a refresh does to the pool.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PoolChange {
    /// Active bots to retire, the youngest accounts first.
    pub retire: Vec<String>,
    /// Retired bots to put back, the most recently retired first.
    pub reinstate: Vec<String>,
    /// How many new bots to make.
    pub create: usize,
}

/// R1454: the change that brings the pool's active bots to `target`.
pub fn plan_pool(bots: &[BotRow], target: usize) -> PoolChange {
    let mut active: Vec<&BotRow> = bots.iter().filter(|bot| bot.retired_at.is_none()).collect();
    let mut retired: Vec<&BotRow> = bots.iter().filter(|bot| bot.retired_at.is_some()).collect();
    if active.len() > target {
        active.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.profile_id.cmp(&a.profile_id))
        });
        return PoolChange {
            retire: active
                .iter()
                .take(active.len() - target)
                .map(|bot| bot.profile_id.clone())
                .collect(),
            ..PoolChange::default()
        };
    }
    let short = target - active.len();
    retired.sort_by(|a, b| {
        b.retired_at
            .cmp(&a.retired_at)
            .then_with(|| a.profile_id.cmp(&b.profile_id))
    });
    let reinstate: Vec<String> = retired
        .iter()
        .take(short)
        .map(|bot| bot.profile_id.clone())
        .collect();
    PoolChange {
        create: short - reinstate.len(),
        reinstate,
        retire: Vec::new(),
    }
}

/// R1555000: active bots that should grind when the reference-rank person exceeds the top bot by
/// `BOT_GRIND_GAP` (less `BOT_GRIND_HYSTERESIS` while some grind, so it does not flap), taking top
/// `BOT_GRINDERS` by rating.
pub fn plan_grind(bots: &[BotRow], people: &[f64]) -> Vec<String> {
    let mut active: Vec<&BotRow> = bots.iter().filter(|bot| bot.retired_at.is_none()).collect();
    active.sort_by(|a, b| {
        b.rating
            .total_cmp(&a.rating)
            .then_with(|| a.profile_id.cmp(&b.profile_id))
    });
    let Some(top) = active.first().map(|bot| bot.rating) else {
        return Vec::new();
    };
    let mut people = people.to_vec();
    people.sort_by(|a, b| b.total_cmp(a));
    let Some(reference) = BOT_GRIND_REFERENCE_RANK
        .checked_sub(1)
        .and_then(|index| people.get(index))
    else {
        return Vec::new();
    };
    let threshold = if active.iter().any(|bot| bot.grinding) {
        BOT_GRIND_GAP - BOT_GRIND_HYSTERESIS
    } else {
        BOT_GRIND_GAP
    };
    if reference - top <= threshold {
        return Vec::new();
    }
    active
        .into_iter()
        .take(BOT_GRINDERS)
        .map(|bot| bot.profile_id.clone())
        .collect()
}

pub(crate) fn int_in(rng: &mut Rng, low: i64, high: i64) -> i64 {
    low + i64::from(rng.int(i32::try_from(high - low + 1).unwrap_or(1)))
}

/// R1454: a new bot drawn from `seed` (its profile id `profile_id`, made at `now`), and the username
/// it claims, or none to keep the `Player#n` the store gives it. Its decks are dealt after
/// (`draw_decks`).
pub fn draw_new_bot(seed: &str, profile_id: &str, now: i64) -> (NewBot, Option<String>) {
    let mut rng = Rng::new(&format!("{seed}:bot"), 0);
    let strength = usize::try_from(rng.int(i32::try_from(BOT_STRENGTHS.len()).unwrap_or(1))).unwrap_or(0);
    let speed = BOT_SPEED_MIN + rng.next() * (BOT_SPEED_MAX - BOT_SPEED_MIN);
    let persona = pick_bot_persona(&mut || rng.next());
    let active_from_hour = rng.int(BOT_HOURS_PER_DAY);
    let active_hours = i32::try_from(int_in(
        &mut rng,
        i64::from(BOT_ACTIVE_HOURS_MIN),
        i64::from(BOT_ACTIVE_HOURS_MAX),
    ))
    .unwrap_or(BOT_ACTIVE_HOURS_MIN);
    let age_days = int_in(&mut rng, BOT_ACCOUNT_AGE_MIN_DAYS, BOT_ACCOUNT_AGE_MAX_DAYS);
    let into_day = i64::from(rng.int(i32::try_from(MS_PER_DAY).unwrap_or(1)));
    let name = if rng.chance(BOT_DEFAULT_USERNAME_SHARE) {
        None
    } else {
        draw_username(&mut rng)
    };
    let seed_rating = BOT_STRENGTHS
        .get(strength)
        .map_or(crate::config::RATING_START, |strength| strength.seed_rating);
    (
        NewBot {
            profile_id: profile_id.to_string(),
            strength,
            speed,
            persona,
            active_from_hour,
            active_hours,
            decks: Vec::new(),
            decks_catalog_version: None,
            created_at: now - age_days * MS_PER_DAY - into_day,
            seed_rating,
        },
        name,
    )
}

/// A collection of exactly the cards of `decks`, which is all a bot needs to own (R1454).
fn owning<'a>(decks: impl IntoIterator<Item = &'a [String]>) -> IndexMap<String, i32> {
    let mut owned: IndexMap<String, i32> = IndexMap::new();
    for card in decks.into_iter().flatten() {
        *owned.entry(card.clone()).or_insert(0) += 1;
    }
    owned
}

/// R1454: whether `cards` is a deck a player could queue for Best of 1 on this catalog (§9.4's L2,
/// L3, L5 and L6), against a collection of the deck's own cards.
fn deck_is_legal(input: &mut DeckInput, cards: &[String]) -> bool {
    input.deck = LoadoutDeck {
        name: None,
        cards: cards.to_vec(),
    };
    input.collection = owning([cards]);
    validator::validate_deck(input).ok
}

/// The validator's input for one deck on `catalog`, its deck and collection filled in per check.
fn deck_input(catalog: &Catalog) -> DeckInput {
    DeckInput {
        deck: LoadoutDeck {
            name: None,
            cards: Vec::new(),
        },
        catalog: CatalogSnapshot {
            version: catalog.version.clone(),
            cards: catalog.defs.clone(),
            banned: Some(catalog.banned.clone()),
        },
        collection: IndexMap::new(),
    }
}

/// R750000: how far a bot of strength index `strength` leans its random decks on winning cards
/// (`deal_random_deck_with_strength`), normalized across `BOT_STRENGTHS` seed ratings to [0.0, 1.0].
pub fn bot_target_strength(strength: usize) -> f64 {
    let Some(own) = BOT_STRENGTHS.get(strength) else {
        return 0.0;
    };
    let ratings = BOT_STRENGTHS.iter().map(|strength| strength.seed_rating);
    let low = ratings.clone().fold(f64::INFINITY, f64::min);
    let high = ratings.fold(f64::NEG_INFINITY, f64::max);
    if high <= low {
        return 0.0;
    }
    ((own.seed_rating - low) / (high - low)).clamp(0.0, 1.0)
}

/// R1454, R750000: distinct live Best-of-1 human decklists (R1044000) partitioned into `ranked` (at least
/// `CARD_STATS_MIN_SAMPLE` games, sorted by exact win rate (no float decides it), game count, and
/// cards) and `unranked`. Lists not of `DECK_SIZE` cards are excluded (R1705001): they fail L2, and
/// would take every one of `draw_decks`' attempts.
fn ranked_human_bo1_decklists(lists: &[ListTotal]) -> (Vec<Vec<String>>, Vec<Vec<String>>) {
    let (mut ranked, unranked): (Vec<&ListTotal>, Vec<&ListTotal>) = lists
        .iter()
        .filter(|list| holds_deck_size(&list.deck))
        .partition(|list| list.games >= CARD_STATS_MIN_SAMPLE);
    ranked.sort_by(|a, b| {
        (b.wins * a.games)
            .cmp(&(a.wins * b.games))
            .then_with(|| b.games.cmp(&a.games))
            .then_with(|| a.key.join(",").cmp(&b.key.join(",")))
    });
    (
        ranked.into_iter().map(|list| list.deck.clone()).collect(),
        unranked.into_iter().map(|list| list.deck.clone()).collect(),
    )
}

/// R750000: ranked lists ordered nearest to the bot's strength percentile (1.0 = best, 0.0 = worst),
/// ties to the better list; only the share is a float.
fn by_strength(ranked: Vec<Vec<String>>, strength: f64) -> Vec<Vec<String>> {
    let last = ranked.len().saturating_sub(1) as f64;
    let place = (1.0 - strength.clamp(0.0, 1.0)) * last;
    let mut placed: Vec<(usize, Vec<String>)> = ranked.into_iter().enumerate().collect();
    placed.sort_by(|(a, _), (b, _)| {
        (*a as f64 - place)
            .abs()
            .total_cmp(&(*b as f64 - place).abs())
            .then_with(|| a.cmp(b))
    });
    placed.into_iter().map(|(_, deck)| deck).collect()
}

/// R1454: draws legal Best-of-1 decks for a bot: human lists first, ranked by strength and the rest
/// shuffled (R750000), then random decks (R1453) leaning on winning cards by strength, up to
/// `BOT_DECK_DRAW_ATTEMPTS`. `lists` are live Best-of-1 human decklists (R1044000).
pub fn draw_decks(seed: &str, lists: &[ListTotal], catalog: &Catalog, strength: f64) -> Vec<BotDeck> {
    jackioh_cards::register_all();
    let mut rng = Rng::new(&format!("{seed}:decks"), 0);
    let wanted = usize::try_from(int_in(
        &mut rng,
        i64::try_from(BOT_DECKS_MIN).unwrap_or(0),
        i64::try_from(BOT_DECKS_MAX).unwrap_or(0),
    ))
    .unwrap_or(BOT_DECKS_MIN);
    let (ranked, unranked) = ranked_human_bo1_decklists(lists);
    let mut candidates = by_strength(ranked, strength);
    candidates.extend(rng.shuffle(&unranked));
    candidates.truncate(BOT_DECK_DRAW_ATTEMPTS);
    for index in candidates.len()..BOT_DECK_DRAW_ATTEMPTS {
        candidates.push(deal_random_deck_with_strength(
            &format!("{seed}:deck:{index}"),
            None,
            strength,
        ));
    }
    let mut input = deck_input(catalog);
    let mut decks: Vec<BotDeck> = Vec::new();
    for cards in candidates {
        if decks.len() >= wanted {
            break;
        }
        if !deck_is_legal(&mut input, &cards) {
            continue;
        }
        // R1834005: dealt with no slots, which the refresh's cosmetics step draws next.
        decks.push(BotDeck {
            portrait: pick_portrait_from_seed(&format!("{seed}:portrait:{}", decks.len())).to_string(),
            cards,
            slots: None,
        });
    }
    decks
}

/// R641000: draws a Conquest trio from disjoint decks in `own`, filling shortfalls with strength-weighted
/// random decks excluding used cards (R750000), portraits from seed (R642). Returns `None` if fewer than
/// 3 valid decks are found or if the trio fails rules (§9.4, R253): no Conquest until next refresh.
pub fn draw_trio(seed: &str, own: &[BotDeck], catalog: &Catalog, strength: f64) -> Option<FrozenTrio> {
    jackioh_cards::register_all();
    let mut input = deck_input(catalog);
    let mut kept: Vec<BotDeck> = Vec::new();
    let mut used: IndexSet<String> = IndexSet::new();
    let shares_none =
        |cards: &[String], used: &IndexSet<String>| !cards.iter().any(|card| used.contains(card));
    for deck in own {
        if kept.len() >= validator::TRIO_DECKS {
            break;
        }
        if shares_none(&deck.cards, &used) && deck_is_legal(&mut input, &deck.cards) {
            used.extend(deck.cards.iter().cloned());
            kept.push(deck.clone());
        }
    }
    for attempt in 0..BOT_DECK_DRAW_ATTEMPTS {
        if kept.len() >= validator::TRIO_DECKS {
            break;
        }
        let excluded: Vec<String> = used.iter().cloned().collect();
        let cards = deal_random_deck_excluding_with_strength(
            &format!("{seed}:trio-deck:{attempt}"),
            &excluded,
            strength,
        );
        if shares_none(&cards, &used) && deck_is_legal(&mut input, &cards) {
            used.extend(cards.iter().cloned());
            kept.push(BotDeck {
                portrait: pick_portrait_from_seed(&format!("{seed}:trio-portrait:{}", kept.len()))
                    .to_string(),
                cards,
                slots: None,
            });
        }
    }
    if kept.len() < validator::TRIO_DECKS {
        return None;
    }
    let checked = validator::validate_trio(&LoadoutInput {
        decks: kept
            .iter()
            .map(|deck| LoadoutDeck {
                name: None,
                cards: deck.cards.clone(),
            })
            .collect(),
        catalog: input.catalog,
        collection: owning(kept.iter().map(|deck| deck.cards.as_slice())),
    });
    if !checked.ok {
        return None;
    }
    let mut decks = kept.into_iter().enumerate().map(|(slot, deck)| FrozenDeck {
        name: format!("Deck {}", slot + 1),
        cards: deck.cards,
        portrait: Some(Some(deck.portrait)),
        // R1834005: a Best-of-1 deck the trio keeps keeps its slots; a deck dealt for it has none yet.
        slots: deck.slots,
    });
    let (Some(first), Some(second), Some(third)) = (decks.next(), decks.next(), decks.next()) else {
        return None;
    };
    Some(FrozenTrio {
        name: TRIO_NAME.to_string(),
        decks: (first, second, third),
    })
}

/// What one refresh did, for its log line and the tests.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PoolRefresh {
    pub target: usize,
    pub retired: usize,
    pub reinstated: usize,
    pub created: usize,
    /// Bots dealt decks (new, stale catalog, or invalid size, R1705001).
    pub dealt: usize,
    /// R641000: bots whose Conquest trio was dealt (R1705001).
    pub trios: usize,
    /// R1555000: bots that grind after it.
    pub grinding: usize,
    /// R1834005: bots whose cosmetics were drawn (new, redealt, or aged).
    pub dressed: usize,
}

/// R1454: claims `name` for a new bot via username route rules; falls back to `Player#n` on failure.
async fn claim_name(app: &App, profile_id: &str, name: &str, now: i64) -> Result<(), StoreError> {
    let Ok(base) = normalize_username(name) else {
        return Ok(());
    };
    let key = username_key(&base);
    let mut tx = app.db.begin(None).await?;
    let expected_tag = tx.profiles_username_tag_for(profile_id, &key).await?;
    let outcome = tx
        .profiles_claim_username(&UsernameClaim {
            profile_id: profile_id.to_string(),
            base,
            key,
            expected_tag,
            at: now,
            cooldown_ms: USERNAME_CHANGE_COOLDOWN_MS,
        })
        .await?;
    if matches!(outcome, UsernameClaimOutcome::Claimed { .. }) {
        tx.commit().await?;
    }
    Ok(())
}

/// R1454: releases bots and matched players whose start timed out after `BOT_STALE_START_SECONDS`,
/// reconciling the seat ledger immediately (R1523000).
pub async fn release_stale_starts(app: &App, now: i64) -> Result<usize, StoreError> {
    let mut tx = app.db.begin(None).await?;
    let released = tx
        .bots_release_stale_starts(now - BOT_STALE_START_SECONDS * MS_PER_SECOND)
        .await?;
    tx.commit().await?;
    for bot_id in &released {
        tracing::warn!(event = "bots.start.released", botId = %bot_id);
    }
    if !released.is_empty() {
        // R1523000: the bots and players let go here leave the seat ledger too, which a restart
        // filled from their flags.
        crate::seats::reconcile(app).await;
    }
    Ok(released.len())
}

/// R1454: adjusts bot pool size for active players, deals decks (R1705001) and trios (R641000) to
/// active bots needing them, draws eligible cosmetics (R1834005), and updates grinding bots (R1555000).
pub async fn refresh_pool(app: &App, seed: &str, now: i64) -> Result<PoolRefresh, StoreError> {
    let mut tx = app.db.begin(None).await?;
    let bots = tx.bots_list().await?;
    let weekly = tx
        .results_count_active_humans(now - BOT_POOL_ACTIVE_WINDOW_DAYS * MS_PER_DAY)
        .await?;
    tx.commit().await?;

    let target = pool_target(weekly);
    let change = plan_pool(&bots, target);
    let mut tx = app.db.begin(None).await?;
    for profile_id in &change.retire {
        tx.bots_set_retired(profile_id, Some(now)).await?;
    }
    for profile_id in &change.reinstate {
        tx.bots_set_retired(profile_id, None).await?;
    }
    let mut named: Vec<(String, String)> = Vec::new();
    for index in 0..change.create {
        let (bot, name) = draw_new_bot(&format!("{seed}:{index}"), &new_uuid(), now);
        tx.bots_create(&bot).await?;
        if let Some(name) = name {
            named.push((bot.profile_id.clone(), name));
        }
    }
    tx.commit().await?;
    for (profile_id, name) in &named {
        claim_name(app, profile_id, name, now).await?;
    }

    let mut tx = app.db.begin(None).await?;
    let bots = tx.bots_list().await?;
    let stale: Vec<BotRow> = bots
        .into_iter()
        .filter(|bot| bot.retired_at.is_none())
        // R1705001: a bot dealt decks of another size than `DECK_SIZE` is dealt again.
        .filter(|bot| !holds_decks(bot, &app.catalog.version))
        .collect();
    let lists = if stale.is_empty() {
        Vec::new()
    } else {
        tx.stats_decklists(&app.catalog.version).await?
    };
    tx.commit().await?;
    for bot in &stale {
        let decks = draw_decks(
            &format!("{seed}:{}", bot.profile_id),
            &lists,
            &app.catalog,
            bot_target_strength(bot.strength),
        );
        let mut tx = app.db.begin(None).await?;
        tx.bots_set_decks(&bot.profile_id, &decks, &app.catalog.version)
            .await?;
        tx.commit().await?;
    }

    // R641000: after the decks, since a trio starts from them.
    let mut tx = app.db.begin(None).await?;
    let bots = tx.bots_list().await?;
    tx.commit().await?;
    let mut trios = 0;
    for bot in bots.iter().filter(|bot| bot.retired_at.is_none()) {
        if holds_trio(bot, &app.catalog.version) {
            continue;
        }
        let Some(trio) = draw_trio(
            &format!("{seed}:{}", bot.profile_id),
            &bot.decks,
            &app.catalog,
            bot_target_strength(bot.strength),
        ) else {
            tracing::warn!(event = "bots.pool.trio_short", botId = %bot.profile_id);
            continue;
        };
        let mut tx = app.db.begin(None).await?;
        tx.bots_set_trio(&bot.profile_id, &trio, &app.catalog.version)
            .await?;
        tx.commit().await?;
        trios += 1;
    }

    // R1834005: after the decks and the trio, since a deck's slots and a card's picks follow them.
    let mut tx = app.db.begin(None).await?;
    let bots = tx.bots_list().await?;
    let owners = tx.economy_cosmetic_owners().await?;
    tx.commit().await?;
    let registry = crate::cosmetics::registry();
    let eligible = cosmetics::eligible(registry, &owners, now);
    let mut dressed = 0;
    for bot in bots.iter().filter(|bot| bot.retired_at.is_none()) {
        let Some(draw) = cosmetics::redraw(
            &format!("{seed}:{}", bot.profile_id),
            bot,
            BOT_COSMETIC_FITS,
            registry,
            &eligible,
            now,
        ) else {
            continue;
        };
        let mut tx = app.db.begin(None).await?;
        tx.bots_set_cosmetics(&bot.profile_id, &draw.cosmetics).await?;
        if let Some(decks) = &draw.decks {
            let version = bot
                .decks_catalog_version
                .as_deref()
                .unwrap_or(&app.catalog.version);
            tx.bots_set_decks(&bot.profile_id, decks, version).await?;
        }
        if let Some(trio) = &draw.trio {
            let version = bot
                .trio_catalog_version
                .as_deref()
                .unwrap_or(&app.catalog.version);
            tx.bots_set_trio(&bot.profile_id, trio, version).await?;
        }
        tx.commit().await?;
        dressed += 1;
    }

    // R1555000: after the pool is sized, so a retired bot stops and a new one may start.
    let mut tx = app.db.begin(None).await?;
    let bots = tx.bots_list().await?;
    let standings = tx
        .ranked_standings(&build_season_id(&SeasonDeps::current()))
        .await?;
    tx.commit().await?;
    let people: Vec<f64> = standings
        .iter()
        .filter(|standing| !standing.bot && standing.rank.ladder.is_some())
        .map(|standing| standing.rating)
        .collect();
    let grinders = plan_grind(&bots, &people);
    let mut tx = app.db.begin(None).await?;
    for bot in &bots {
        let grinding = grinders.contains(&bot.profile_id);
        if bot.grinding != grinding {
            tx.bots_set_grinding(&bot.profile_id, grinding).await?;
        }
    }
    tx.commit().await?;
    Ok(PoolRefresh {
        target,
        retired: change.retire.len(),
        reinstated: change.reinstate.len(),
        created: change.create,
        dealt: stale.len(),
        trios,
        grinding: grinders.len(),
        dressed,
    })
}

/// R1454: refreshes the pool at boot and every `BOT_POOL_REFRESH_INTERVAL_SECONDS`; a failed one is
/// logged and the next tries again. `app.rs` spawns it only when `LADDER_BOTS=on`.
pub async fn run_bot_pool(app: Arc<App>) {
    loop {
        match refresh_pool(&app, &new_seed(), now_ms()).await {
            Ok(done) => tracing::info!(
                event = "bots.pool.refreshed",
                target = done.target,
                retired = done.retired,
                reinstated = done.reinstated,
                created = done.created,
                dealt = done.dealt,
                trios = done.trios,
                grinding = done.grinding,
                dressed = done.dressed,
            ),
            Err(error) => tracing::warn!(event = "bots.pool.failed", message = %error),
        }
        tokio::time::sleep(Duration::from_secs(
            u64::try_from(BOT_POOL_REFRESH_INTERVAL_SECONDS).unwrap_or(0),
        ))
        .await;
    }
}
