//! The bot pool (SPEC §9.12, R1449): how many ladder bots there are, who they are and what they play.
//!
//! The pool holds `clamp(BOT_POOL_MIN, BOT_POOL_PER_WEEKLY_ACTIVE × weekly active people,
//! BOT_POOL_MAX)` active bots, sized again every `BOT_POOL_REFRESH_INTERVAL_SECONDS` and once as the
//! server boots (`run_bot_pool`, spawned by `app.rs` when `LADDER_BOTS=on`). An excess retires the
//! active bots with the youngest accounts (the latest `created_at`, the account age drawn for each),
//! which finish the match they are in; a shortfall puts the most recently retired back first, so the
//! ladder keeps meeting the same faces, and only then makes new ones.
//!
//! A new bot is drawn from the pool's seed like a player is made: a strength (`BOT_STRENGTHS`, its
//! R610 rating seeded from it), a think speed, a persona (R645's weights), the hours it is on, an
//! account age, and a username drawn from two word lists or, for `BOT_DEFAULT_USERNAME_SHARE` of them,
//! the `Player#n` every new account is given. Its Best-of-1 decks are the patch's distinct decklists
//! people took into live Best-of-1 games, shuffled, then random decks dealt as a player's All Random
//! deck is (`deal_random_deck`: no shadow ban, no soft gate, R1448), each kept only when it is legal
//! (`validate_deck`, against a collection of its own cards: a bot owns nothing and needs nothing),
//! and dealt again whenever the catalog version changes.
//!
//! `release_stale_starts`, which the queue's sweeper runs every sweep whatever `LADDER_BOTS` says,
//! lets go any bot still held by a claim whose match never started (`BOT_STALE_START_SECONDS`): a
//! start a crash or a restart cut off, which nothing else reaps.

use std::sync::Arc;
use std::time::Duration;

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::validator::{self, CatalogSnapshot, DeckInput, LoadoutDeck};
use jackioh_engine::wire::{GameMode, GameRecord, GameSource, PLAYER_IDS, Pilot, SourceFilter};
use jackioh_engine::{Rng, pick_portrait_from_seed};

use crate::actor::engine::deal_random_deck;
use crate::api::catalog::Catalog;
use crate::api::queue::{new_seed, new_uuid};
use crate::app::{App, now_ms};
use crate::bots::persona::pick_persona;
use crate::config::{
    BOT_ACCOUNT_AGE_MAX_DAYS, BOT_ACCOUNT_AGE_MIN_DAYS, BOT_ACTIVE_HOURS_MAX, BOT_ACTIVE_HOURS_MIN,
    BOT_DECK_DRAW_ATTEMPTS, BOT_DECKS_MAX, BOT_DECKS_MIN, BOT_DEFAULT_USERNAME_SHARE, BOT_HOURS_PER_DAY,
    BOT_NAME_FIRST, BOT_NAME_SECOND, BOT_POOL_ACTIVE_WINDOW_DAYS, BOT_POOL_MAX, BOT_POOL_MIN,
    BOT_POOL_PER_WEEKLY_ACTIVE, BOT_POOL_REFRESH_INTERVAL_SECONDS, BOT_SPEED_MAX, BOT_SPEED_MIN,
    BOT_STALE_START_SECONDS, BOT_STRENGTHS, USERNAME_CHANGE_COOLDOWN_MS,
};
use crate::db::store::{
    BotDeck, BotRow, GameRecordQuery, NewBot, StoreError, UsernameClaim, UsernameClaimOutcome,
};
use crate::username::{normalize_username, username_key};

/// Unit conversion, not configuration: a day in milliseconds.
const MS_PER_DAY: i64 = 86_400_000;
/// Unit conversion, not configuration: a second in milliseconds.
const MS_PER_SECOND: i64 = 1000;

/// R1449: the pool's size for `weekly` people who finished a live game in the last
/// `BOT_POOL_ACTIVE_WINDOW_DAYS`.
pub fn pool_target(weekly: i64) -> usize {
    let wanted = (weekly.max(0) as f64 * BOT_POOL_PER_WEEKLY_ACTIVE).round() as usize;
    wanted.clamp(BOT_POOL_MIN, BOT_POOL_MAX)
}

/// R1449: what a refresh does to the pool.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PoolChange {
    /// Active bots to retire, the youngest accounts first.
    pub retire: Vec<String>,
    /// Retired bots to put back, the most recently retired first.
    pub reinstate: Vec<String>,
    /// How many new bots to make.
    pub create: usize,
}

/// R1449: the change that brings the pool's active bots to `target`.
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

/// `low + rng.int(high − low + 1)`: an integer in `[low, high]`.
fn int_in(rng: &mut Rng, low: i64, high: i64) -> i64 {
    low + i64::from(rng.int(i32::try_from(high - low + 1).unwrap_or(1)))
}

/// R1449: a new bot drawn from `seed` (its profile id `profile_id`, made at `now`), and the username
/// it claims, or none to keep the `Player#n` the store gives it. Its decks are dealt after
/// (`draw_decks`).
pub fn draw_new_bot(seed: &str, profile_id: &str, now: i64) -> (NewBot, Option<String>) {
    let mut rng = Rng::new(&format!("{seed}:bot"), 0);
    let strength = usize::try_from(rng.int(i32::try_from(BOT_STRENGTHS.len()).unwrap_or(1))).unwrap_or(0);
    let speed = BOT_SPEED_MIN + rng.next() * (BOT_SPEED_MAX - BOT_SPEED_MIN);
    let persona = pick_persona(&mut || rng.next());
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
        let first = rng.pick(BOT_NAME_FIRST).copied().unwrap_or_default();
        let second = rng.pick(BOT_NAME_SECOND).copied().unwrap_or_default();
        Some(format!("{first}{second}"))
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

/// R1449: whether `cards` is a deck a player could queue for Best of 1 on this catalog (§9.4's L2,
/// L3, L5 and L6), against a collection of the deck's own cards.
fn deck_is_legal(input: &mut DeckInput, cards: &[String]) -> bool {
    let mut owned: IndexMap<String, i32> = IndexMap::new();
    for card in cards {
        *owned.entry(card.clone()).or_insert(0) += 1;
    }
    input.deck = LoadoutDeck {
        name: None,
        cards: cards.to_vec(),
    };
    input.collection = owned;
    validator::validate_deck(input).ok
}

/// R1449: the patch's distinct decklists people took into live Best-of-1 games, in the records'
/// order (sorted copies compared, so a reordering is the same list).
fn human_bo1_decklists(records: &[GameRecord]) -> Vec<Vec<String>> {
    let mut seen: IndexSet<Vec<String>> = IndexSet::new();
    let mut lists: Vec<Vec<String>> = Vec::new();
    for record in records {
        if record.source != GameSource::Live || record.mode != GameMode::Bo1 {
            continue;
        }
        for seat in PLAYER_IDS {
            if record.pilots[seat] != Pilot::Human {
                continue;
            }
            let deck = record.game.seats[seat].deck.clone();
            let mut sorted = deck.clone();
            sorted.sort();
            if seen.insert(sorted) {
                lists.push(deck);
            }
        }
    }
    lists
}

/// R1449: two or three legal Best-of-1 decks for the bot seeded `seed`: the patch's human decklists
/// first (shuffled from the seed), then random decks dealt as a player's are (All Random's deal, with
/// no shadow ban and no soft gate, R1448), trying at most `BOT_DECK_DRAW_ATTEMPTS` in all, each with
/// a portrait dealt from the seed.
pub fn draw_decks(seed: &str, records: &[GameRecord], catalog: &Catalog) -> Vec<BotDeck> {
    jackioh_cards::register_all();
    let mut rng = Rng::new(&format!("{seed}:decks"), 0);
    let wanted = usize::try_from(int_in(
        &mut rng,
        i64::try_from(BOT_DECKS_MIN).unwrap_or(0),
        i64::try_from(BOT_DECKS_MAX).unwrap_or(0),
    ))
    .unwrap_or(BOT_DECKS_MIN);
    let mut candidates = rng.shuffle(&human_bo1_decklists(records));
    candidates.truncate(BOT_DECK_DRAW_ATTEMPTS);
    for index in candidates.len()..BOT_DECK_DRAW_ATTEMPTS {
        candidates.push(deal_random_deck(&format!("{seed}:deck:{index}"), None));
    }
    let mut input = DeckInput {
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
    };
    let mut decks: Vec<BotDeck> = Vec::new();
    for cards in candidates {
        if decks.len() >= wanted {
            break;
        }
        if !deck_is_legal(&mut input, &cards) {
            continue;
        }
        decks.push(BotDeck {
            portrait: pick_portrait_from_seed(&format!("{seed}:portrait:{}", decks.len())).to_string(),
            cards,
        });
    }
    decks
}

/// What one refresh did, for its log line and the tests.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PoolRefresh {
    pub target: usize,
    pub retired: usize,
    pub reinstated: usize,
    pub created: usize,
    /// Bots whose decks were dealt (new bots, and bots whose decks were for another catalog).
    pub dealt: usize,
}

/// R1449: claims `name` for the new bot as the username route claims one. A name the checks refuse,
/// or a claim the store refuses, leaves the bot its `Player#n`.
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

/// R1449: lets go every bot whose claimed match was still unstarted `BOT_STALE_START_SECONDS` after
/// the claim, and that match's player with it: a start a crash or a restart cut off. The queue's
/// sweeper runs it every sweep whatever `LADDER_BOTS` says (`run_matchmaker`), so neither is held
/// out of the queue for longer than about that. Answers how many bots it let go.
pub async fn release_stale_starts(app: &App, now: i64) -> Result<usize, StoreError> {
    let mut tx = app.db.begin(None).await?;
    let released = tx
        .bots_release_stale_starts(now - BOT_STALE_START_SECONDS * MS_PER_SECOND)
        .await?;
    tx.commit().await?;
    for bot_id in &released {
        tracing::warn!(event = "bots.start.released", botId = %bot_id);
    }
    Ok(released.len())
}

/// R1449: sizes the pool for the people active now, makes, retires and reinstates bots to fit, and
/// deals decks to every active bot that holds none for this catalog. `seed` seeds the new bots.
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
        .filter(|bot| {
            bot.decks.is_empty() || bot.decks_catalog_version.as_deref() != Some(app.catalog.version.as_str())
        })
        .collect();
    let records = if stale.is_empty() {
        Vec::new()
    } else {
        tx.game_records_list(&GameRecordQuery {
            source: SourceFilter::Live,
            mode: Some(GameMode::Bo1),
            patch: Some(app.catalog.version.clone()),
        })
        .await?
    };
    tx.commit().await?;
    for bot in &stale {
        let decks = draw_decks(&format!("{seed}:{}", bot.profile_id), &records, &app.catalog);
        let mut tx = app.db.begin(None).await?;
        tx.bots_set_decks(&bot.profile_id, &decks, &app.catalog.version)
            .await?;
        tx.commit().await?;
    }
    Ok(PoolRefresh {
        target,
        retired: change.retire.len(),
        reinstated: change.reinstate.len(),
        created: change.create,
        dealt: stale.len(),
    })
}

/// R1449: refreshes the pool as the server boots and every `BOT_POOL_REFRESH_INTERVAL_SECONDS`
/// after; a failed refresh is logged and the next one tries again. `app.rs` spawns it only when
/// `LADDER_BOTS=on`, and aborting the task stops it.
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
            ),
            Err(error) => tracing::warn!(event = "bots.pool.failed", message = %error),
        }
        tokio::time::sleep(Duration::from_secs(
            u64::try_from(BOT_POOL_REFRESH_INTERVAL_SECONDS).unwrap_or(0),
        ))
        .await;
    }
}
