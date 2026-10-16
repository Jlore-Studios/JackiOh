//! The bot pool (SPEC §9.12, R1454): how many ladder bots there are, who they are and what they play.
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
//! people took into live Best-of-1 games, those with enough games in the order its strength takes
//! them and the rest shuffled (R750000), then random decks dealt as a player's All Random deck is (no shadow ban, no soft gate,
//! R1453) leaning on the cards that win as far as its strength says (`deal_random_deck_with_strength`,
//! R750000), each kept only when it is legal (`validate_deck`, against a collection of its own cards:
//! a bot owns nothing and needs nothing), and dealt again whenever the catalog version changes. Its
//! Conquest trio (R641000) is dealt after them, for the same catalog version: its own decks first, in order, each that shares no card with
//! those taken before it, then decks dealt with every card already taken left out, until it holds
//! three that pass the trio's rules (`validate_trio`, R253).
//!
//! `release_stale_starts`, which the queue's sweeper runs every sweep whatever `LADDER_BOTS` says,
//! lets go any bot still held by a claim whose match never started (`BOT_STALE_START_SECONDS`): a
//! start a crash or a restart cut off. The reaper's open-row sweep (R1450) would delete that
//! skeleton too, but only after `OPEN_MATCH_REAP_GRACE_SECONDS`, and a bot is not held that long.

use std::sync::Arc;
use std::time::Duration;

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::validator::{self, CatalogSnapshot, DeckInput, LoadoutDeck, LoadoutInput};
use jackioh_engine::wire::ListTotal;
use jackioh_engine::{Rng, pick_portrait_from_seed};

use crate::actor::engine::{deal_random_deck_excluding_with_strength, deal_random_deck_with_strength};
use crate::api::catalog::Catalog;
use crate::api::queue::{new_seed, new_uuid};
use crate::app::{App, now_ms};
use crate::bots::persona::pick_bot_persona;
use crate::config::{
    BOT_ACCOUNT_AGE_MAX_DAYS, BOT_ACCOUNT_AGE_MIN_DAYS, BOT_ACTIVE_HOURS_MAX, BOT_ACTIVE_HOURS_MIN,
    BOT_DECK_DRAW_ATTEMPTS, BOT_DECKS_MAX, BOT_DECKS_MIN, BOT_DEFAULT_USERNAME_SHARE, BOT_HOURS_PER_DAY,
    BOT_NAME_FIRST, BOT_NAME_SECOND, BOT_POOL_ACTIVE_WINDOW_DAYS, BOT_POOL_MAX, BOT_POOL_MIN,
    BOT_POOL_PER_WEEKLY_ACTIVE, BOT_POOL_REFRESH_INTERVAL_SECONDS, BOT_SPEED_MAX, BOT_SPEED_MIN,
    BOT_STALE_START_SECONDS, BOT_STRENGTHS, CARD_STATS_MIN_SAMPLE, USERNAME_CHANGE_COOLDOWN_MS,
};
use crate::db::store::{
    BotDeck, BotRow, FrozenDeck, FrozenTrio, NewBot, StoreError, UsernameClaim, UsernameClaimOutcome,
};
use crate::username::{normalize_username, username_key};

/// Unit conversion, not configuration: a day in milliseconds.
const MS_PER_DAY: i64 = 86_400_000;
/// Unit conversion, not configuration: a second in milliseconds.
const MS_PER_SECOND: i64 = 1000;

/// R641000: the name a bot's trio is frozen under. No client reads it: a series shows a player only
/// which of the opponent's slots have won (R336).
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

/// `low + rng.int(high − low + 1)`: an integer in `[low, high]`.
fn int_in(rng: &mut Rng, low: i64, high: i64) -> i64 {
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

/// R750000: how far the bot of strength index `strength` leans its random decks on the cards that
/// win (`deal_random_deck_with_strength`): its seed rating's place between the weakest and the
/// strongest of `BOT_STRENGTHS`, 0.0 for the weakest and 1.0 for the strongest. 0.0 for an index
/// `BOT_STRENGTHS` does not hold, or when every strength seeds one rating.
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

/// R1454, R750000: the patch's distinct decklists people took into live Best-of-1 games (sorted
/// copies compared, so a reordering is the same list), as the totals hold them (R1044000), in the
/// order the records first held them, split in two. `ranked` holds those with at least
/// `CARD_STATS_MIN_SAMPLE` games, the best exact win rate first (`wins_a × games_b` against
/// `wins_b × games_a`, so no float decides it, as `win_rate_leader` does), then more games, then the
/// ids joined, ascending; `unranked` holds the rest in the records' order. A draw is a game not won, as
/// `card_stats` counts it, and a list needs its 20 games before it is ranked at all, so a list that
/// won its one game never goes ahead of one that won 90 of 200.
fn ranked_human_bo1_decklists(lists: &[ListTotal]) -> (Vec<Vec<String>>, Vec<Vec<String>>) {
    let (mut ranked, unranked): (Vec<&ListTotal>, Vec<&ListTotal>) =
        lists.iter().partition(|list| list.games >= CARD_STATS_MIN_SAMPLE);
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

/// R750000: the ranked lists (best first) in the order a bot of strength share `strength` takes them:
/// nearest first to its place among them, the best list for 1.0 and the worst for 0.0, a tie going to
/// the better list. Ordered by index distance, so only the share, never a rate, is a float here.
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

/// R1454: two or three legal Best-of-1 decks for the bot seeded `seed`: the patch's human decklists
/// first, those with enough games in the order `strength` takes them (`by_strength`) and the rest
/// shuffled from the seed (R750000), then
/// random decks dealt as a player's are (All Random's deal, with no shadow ban and no soft gate,
/// R1453) leaning on the cards that win as far as `strength` says (`bot_target_strength`, R750000),
/// trying at most `BOT_DECK_DRAW_ATTEMPTS` in all, each with a portrait dealt from the seed. `lists`
/// are the patch's live Best-of-1 human decklists in the order the records first held them
/// (`Tx::stats_decklists`, R1044000).
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
        decks.push(BotDeck {
            portrait: pick_portrait_from_seed(&format!("{seed}:portrait:{}", decks.len())).to_string(),
            cards,
        });
    }
    decks
}

/// R641000: the Conquest trio for the bot seeded `seed`, which holds the Best-of-1 decks `own`: its
/// own decks first, in order, each kept when it shares no card with the decks kept before it and a
/// player could queue it; then, while the trio is short, decks dealt as All Random deals a player's
/// with every card kept so far left out, leaning on the cards that win as far as `strength` says
/// (`deal_random_deck_excluding_with_strength`, R750000), kept on the same terms,
/// trying at most `BOT_DECK_DRAW_ATTEMPTS`, each with a portrait dealt from the seed (R642). `None`
/// when that finds no three, or the three fail the trio's own rules (§9.4's L1–L6, R253, against a
/// collection of their own cards), so the bot plays no Conquest until the next refresh.
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
    /// Bots whose decks were dealt (new bots, and bots whose decks were for another catalog).
    pub dealt: usize,
    /// R641000: bots whose Conquest trio was dealt (bots that held none, and bots whose trio was for
    /// another catalog).
    pub trios: usize,
}

/// R1454: claims `name` for the new bot as the username route claims one. A name the checks refuse,
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

/// R1454: lets go every bot whose claimed match was still unstarted `BOT_STALE_START_SECONDS` after
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

/// R1454: sizes the pool for the people active now, makes, retires and reinstates bots to fit, and
/// deals decks to every active bot that holds none for this catalog, then (R641000) a trio to every
/// active bot that holds none for it. `seed` seeds the new bots.
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
        if bot.trio.is_some() && bot.trio_catalog_version.as_deref() == Some(app.catalog.version.as_str()) {
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
    Ok(PoolRefresh {
        target,
        retired: change.retire.len(),
        reinstated: change.reinstate.len(),
        created: change.create,
        dealt: stale.len(),
        trios,
    })
}

/// R1454: refreshes the pool as the server boots and every `BOT_POOL_REFRESH_INTERVAL_SECONDS`
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
                trios = done.trios,
            ),
            Err(error) => tracing::warn!(event = "bots.pool.failed", message = %error),
        }
        tokio::time::sleep(Duration::from_secs(
            u64::try_from(BOT_POOL_REFRESH_INTERVAL_SECONDS).unwrap_or(0),
        ))
        .await;
    }
}
