//! The bot pool's decisions (SPEC §9.12, R1454): how many ladder bots are active, which to retire,
//! reinstate or make, who grinds, a new bot's identity, its Best-of-1 decks and its Conquest trio. Pure
//! over plain records: the host reads its store into [`BotRecord`]s (or [`BotFacts`]), asks, and writes
//! what comes out (the server's `bots/pool.rs`, a Roblox DataStore script); nothing here reads a store,
//! a clock or the OS.
//!
//! The pool holds `clamp(BOT_POOL_MIN, BOT_POOL_PER_WEEKLY_ACTIVE × weekly active people, BOT_POOL_MAX)`
//! active bots, sized at boot and every refresh. An excess retires the youngest, which finish their
//! match; a shortfall reinstates the latest retired first, so the ladder meets the same faces.
//! New bots draw strength (R610), persona (R645), schedule, and username (`bots::names`).
//! Bo1 decks draw from human decklists and random decks (R750000, R1453), each kept only when legal
//! and dealt again on a new catalog or deck size (R1705001), then the Conquest trio (R641000, R253).
//! The random decks lean on the win-rate table the host hands the deal, the server's snapshot of its
//! cached card stats, and on nothing without one (R3049002).

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use jackioh_engine::validator::{self, CatalogSnapshot, DeckInput, LoadoutDeck, LoadoutInput};
use jackioh_engine::wire::ListTotal;
use jackioh_engine::{DECK_SIZE, Rng, WinRateTable, pick_portrait_from_seed};

use crate::bots::cosmetics::BotCosmetics;
use crate::bots::fallback::holds_deck_size;
use crate::bots::int_in;
use crate::bots::names::draw_username;
use crate::bots::persona::pick_bot_persona;
use crate::config::{
    BOT_ACCOUNT_AGE_MAX_DAYS, BOT_ACCOUNT_AGE_MIN_DAYS, BOT_ACTIVE_HOURS_MAX, BOT_ACTIVE_HOURS_MIN,
    BOT_DECK_DRAW_ATTEMPTS, BOT_DECKS_MAX, BOT_DECKS_MIN, BOT_DEFAULT_USERNAME_SHARE, BOT_GRIND_GAP,
    BOT_GRIND_HYSTERESIS, BOT_GRIND_REFERENCE_RANK, BOT_GRINDERS, BOT_HOURS_PER_DAY, BOT_POOL_MAX,
    BOT_POOL_MIN, BOT_POOL_PER_WEEKLY_ACTIVE, BOT_SPEED_MAX, BOT_SPEED_MIN, BOT_STRENGTHS,
    CARD_STATS_MIN_SAMPLE, MS_PER_DAY, PersonaName,
};
use crate::deck::{DeckSlots, FrozenDeck, FrozenTrio};

/// R641000: the name a bot's trio is frozen under. No client reads it: a series shows a player only
/// which of the opponent's slots have won (R336).
const TRIO_NAME: &str = "Conquest";

/// R2778000: the two ladders a bot is rated on.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum BotLadder {
    Constructed,
    Random,
}

/// R2778000: a bot's two ladders are the ranked ladder's (`ranked::ladder::Ladder`), which the server's store
/// rows and rated games name, under the names the pool reads; the server converts with `into()`.
impl From<crate::ranked::ladder::Ladder> for BotLadder {
    fn from(ladder: crate::ranked::ladder::Ladder) -> Self {
        match ladder {
            crate::ranked::ladder::Ladder::Constructed => BotLadder::Constructed,
            crate::ranked::ladder::Ladder::Random => BotLadder::Random,
        }
    }
}

/// R1454: one Best-of-1 deck a ladder bot carries, and the portrait it plays it under (R642).
/// R1834005: and the slots it wears it with, drawn as a person's deck's (`bots::cosmetics`); absent
/// on a deck dealt since the bot last drew its loadout, which the pool's next refresh draws.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct BotDeck {
    pub cards: Vec<String>,
    pub portrait: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub slots: Option<DeckSlots>,
}

/// R1454: a bot `bots_create` writes: its profile id, its `bots` row and the rating its
/// `bot_ratings` row starts at (R610), with a new player's deviation and volatility.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct NewBot {
    pub profile_id: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub strength: usize,
    pub speed: f64,
    pub persona: PersonaName,
    pub active_from_hour: i32,
    pub active_hours: i32,
    pub decks: Vec<BotDeck>,
    pub decks_catalog_version: Option<String>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub created_at: i64,
    pub seed_rating: f64,
}

/// What the pool and the fallback read of a ladder bot (R1454, R1453, R1555000): a host's row of it
/// implements this, or it is a [`BotFacts`].
pub trait BotRecord {
    /// The bot's profile, which is also its R610 bot id.
    fn profile_id(&self) -> &str;
    /// Epoch ms: when its account reads as made.
    fn created_at(&self) -> i64;
    /// Epoch ms it was retired (R1454), or none while it is in the pool.
    fn retired_at(&self) -> Option<i64>;
    /// R1555000: whether it grinds now.
    fn grinding(&self) -> bool;
    /// R2778000: its rating on `ladder`.
    fn rating_for(&self, ladder: BotLadder) -> f64;
    /// Its schedule (R1454): on from this hour of the day (UTC) for `active_hours` hours.
    fn active_from_hour(&self) -> i32;
    fn active_hours(&self) -> i32;
    /// The match it is in, if it is in one (§9.5).
    fn in_match_id(&self) -> Option<&str>;
    /// Its Best-of-1 decks; empty until the pool deals them.
    fn decks(&self) -> &[BotDeck];
    /// The catalog version its decks were dealt for, or none before they were.
    fn decks_catalog_version(&self) -> Option<&str>;
    /// R641000: its Conquest trio, or none until the pool deals it.
    fn trio(&self) -> Option<&FrozenTrio>;
    /// R641000: the catalog version its trio was dealt for, or none before it was.
    fn trio_catalog_version(&self) -> Option<&str>;
    /// R1834005: its wardrobe and the picks it wears, drawn by the pool (`bots::cosmetics::redraw`); none
    /// before the first draw.
    fn cosmetics(&self) -> Option<&BotCosmetics>;
}

/// A bot as a plain record, for a host that holds no row type of its own (a Roblox script, the
/// boundary's JSON): the fields [`BotRecord`] reads.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct BotFacts {
    pub profile_id: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub created_at: i64,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub retired_at: Option<i64>,
    pub grinding: bool,
    /// Its rating on the Constructed ladder.
    pub rating: f64,
    /// Its rating on the All Random ladder.
    pub random_rating: f64,
    pub active_from_hour: i32,
    pub active_hours: i32,
    pub in_match_id: Option<String>,
    pub decks: Vec<BotDeck>,
    pub decks_catalog_version: Option<String>,
    pub trio: Option<FrozenTrio>,
    pub trio_catalog_version: Option<String>,
    /// R1834005: its wardrobe and the picks it wears; none before the pool first draws them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cosmetics: Option<BotCosmetics>,
}

impl BotRecord for BotFacts {
    fn profile_id(&self) -> &str {
        &self.profile_id
    }
    fn created_at(&self) -> i64 {
        self.created_at
    }
    fn retired_at(&self) -> Option<i64> {
        self.retired_at
    }
    fn grinding(&self) -> bool {
        self.grinding
    }
    fn rating_for(&self, ladder: BotLadder) -> f64 {
        match ladder {
            BotLadder::Constructed => self.rating,
            BotLadder::Random => self.random_rating,
        }
    }
    fn active_from_hour(&self) -> i32 {
        self.active_from_hour
    }
    fn active_hours(&self) -> i32 {
        self.active_hours
    }
    fn in_match_id(&self) -> Option<&str> {
        self.in_match_id.as_deref()
    }
    fn decks(&self) -> &[BotDeck] {
        &self.decks
    }
    fn decks_catalog_version(&self) -> Option<&str> {
        self.decks_catalog_version.as_deref()
    }
    fn trio(&self) -> Option<&FrozenTrio> {
        self.trio.as_ref()
    }
    fn trio_catalog_version(&self) -> Option<&str> {
        self.trio_catalog_version.as_deref()
    }
    fn cosmetics(&self) -> Option<&BotCosmetics> {
        self.cosmetics.as_ref()
    }
}

/// R1454: the pool's size for `weekly` people who finished a live game in the last
/// `BOT_POOL_ACTIVE_WINDOW_DAYS`.
pub fn pool_target(weekly: i64) -> usize {
    let wanted = (weekly.max(0) as f64 * BOT_POOL_PER_WEEKLY_ACTIVE).round() as usize;
    wanted.clamp(BOT_POOL_MIN, BOT_POOL_MAX)
}

/// R1454: what a refresh does to the pool.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PoolChange {
    /// Active bots to retire, the youngest accounts first.
    pub retire: Vec<String>,
    /// Retired bots to put back, the most recently retired first.
    pub reinstate: Vec<String>,
    /// How many new bots to make.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub create: usize,
}

/// R1454: the change that brings the pool's active bots to `target`.
pub fn plan_pool<B: BotRecord>(bots: &[B], target: usize) -> PoolChange {
    let mut active: Vec<&B> = bots.iter().filter(|bot| bot.retired_at().is_none()).collect();
    let mut retired: Vec<&B> = bots.iter().filter(|bot| bot.retired_at().is_some()).collect();
    if active.len() > target {
        active.sort_by(|a, b| {
            b.created_at()
                .cmp(&a.created_at())
                .then_with(|| b.profile_id().cmp(a.profile_id()))
        });
        return PoolChange {
            retire: active
                .iter()
                .take(active.len() - target)
                .map(|bot| bot.profile_id().to_string())
                .collect(),
            ..PoolChange::default()
        };
    }
    let short = target - active.len();
    retired.sort_by(|a, b| {
        b.retired_at()
            .cmp(&a.retired_at())
            .then_with(|| a.profile_id().cmp(b.profile_id()))
    });
    let reinstate: Vec<String> = retired
        .iter()
        .take(short)
        .map(|bot| bot.profile_id().to_string())
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
/// R2778000: on `ladder`, its people's ratings against the bots' there; a refresh plans each ladder.
pub fn plan_grind<B: BotRecord>(bots: &[B], people: &[f64], ladder: BotLadder) -> Vec<String> {
    let mut active: Vec<&B> = bots.iter().filter(|bot| bot.retired_at().is_none()).collect();
    active.sort_by(|a, b| {
        b.rating_for(ladder)
            .total_cmp(&a.rating_for(ladder))
            .then_with(|| a.profile_id().cmp(b.profile_id()))
    });
    let Some(top) = active.first().map(|bot| bot.rating_for(ladder)) else {
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
    let threshold = if active.iter().any(|bot| bot.grinding()) {
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
        .map(|bot| bot.profile_id().to_string())
        .collect()
}

/// R1454: a new bot drawn from `seed` (its profile id `profile_id`, made at `now`), and the username
/// it claims, or none to keep the `Player#n` the store gives it. Its decks are dealt after
/// (`draw_decks`). `accepts` is the host's judgement of a drawn name (`names::draw_username`).
pub fn draw_new_bot(
    seed: &str,
    profile_id: &str,
    now: i64,
    accepts: &mut dyn FnMut(&str) -> Option<String>,
) -> (NewBot, Option<String>) {
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
        draw_username(&mut rng, accepts)
    };
    let seed_rating = BOT_STRENGTHS
        .get(strength)
        .map_or(0.0, |strength| strength.seed_rating);
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
fn deck_input(catalog: &CatalogSnapshot) -> DeckInput {
    DeckInput {
        deck: LoadoutDeck {
            name: None,
            cards: Vec::new(),
        },
        catalog: catalog.clone(),
        collection: IndexMap::new(),
    }
}

/// R750000: how far a bot of strength index `strength` leans its random decks on winning cards
/// (`build_ai_deck`'s `strength`), normalized across `BOT_STRENGTHS` seed ratings to [0.0, 1.0].
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

/// `build_ai_deck`'s deal for a ladder bot (#750, R1454): the engine's All Random deal exactly at strength
/// 0.0, leaning on the cards that win as far as `strength` (0 to 1) says, by `win_rates` (R3049002: the
/// host's snapshot, none leaning on nothing); `excluded` cards are banned from it (a trio's decks share
/// none, R253). Reads the registered catalog.
pub fn deal_bot_deck(
    seed: &str,
    excluded: Option<&[String]>,
    strength: f64,
    win_rates: Option<&WinRateTable>,
) -> Vec<String> {
    // R258: `banned: []` is practice's "random deck for a human" (no shadow-ban: R186's list shapes
    // the AI's own decks, not a player's), and `DECK_SIZE` is the size L2 asks of every deck.
    jackioh_ai::build_ai_deck(
        &mut Rng::new(seed, 0),
        DECK_SIZE,
        &jackioh_ai::AiDeckOptions {
            banned: Some(excluded.map_or_else(Vec::new, <[String]>::to_vec)),
            lean_set: None,
            gated_sets: Some(vec![]),
            strength: Some(strength),
            win_rates: win_rates.cloned(),
            ..jackioh_ai::AiDeckOptions::default()
        },
    )
}

/// R1454: draws legal Best-of-1 decks for a bot: human lists first, ranked by strength and the rest
/// shuffled (R750000), then random decks (R1453) leaning on winning cards by strength, by `win_rates`
/// (R3049002), up to `BOT_DECK_DRAW_ATTEMPTS`. `lists` are live Best-of-1 human decklists (R1044000).
/// The host registers the catalog the deal reads (`jackioh_cards::register_all`).
pub fn draw_decks(
    seed: &str,
    lists: &[ListTotal],
    catalog: &CatalogSnapshot,
    strength: f64,
    win_rates: Option<&WinRateTable>,
) -> Vec<BotDeck> {
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
        candidates.push(deal_bot_deck(
            &format!("{seed}:deck:{index}"),
            None,
            strength,
            win_rates,
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
/// random decks excluding used cards (R750000, by `win_rates`, R3049002), portraits from seed (R642). Returns `None` if fewer than
/// 3 valid decks are found or if the trio fails rules (§9.4, R253): no Conquest until next refresh.
pub fn draw_trio(
    seed: &str,
    own: &[BotDeck],
    catalog: &CatalogSnapshot,
    strength: f64,
    win_rates: Option<&WinRateTable>,
) -> Option<FrozenTrio> {
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
        let cards = deal_bot_deck(
            &format!("{seed}:trio-deck:{attempt}"),
            Some(&excluded),
            strength,
            win_rates,
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

#[cfg(test)]
mod tests {
    use jackioh_engine::wire::FirstSeen;

    use super::*;

    /// A Best-of-1 list people took into `games` games and won `wins` of, of `size` cards of its own.
    fn list(name: &str, size: usize, wins: i64, games: i64) -> ListTotal {
        let deck: Vec<String> = (0..size).map(|card| format!("{name}-{card:02}")).collect();
        ListTotal {
            key: deck.clone(),
            deck,
            games,
            wins,
            first: FirstSeen {
                record: name.to_string(),
                seat: 0,
            },
        }
    }

    /// R750000, R1705001: lists of at least CARD_STATS_MIN_SAMPLE games are ranked, the best win rate
    /// first, in whatever order they come; the rest are unranked; a list not of DECK_SIZE cards is
    /// neither.
    #[test]
    fn r750000_lists_with_their_games_rank_by_win_rate_and_the_rest_stay_unranked() {
        let size = usize::try_from(DECK_SIZE).expect("a deck size");
        let games = CARD_STATS_MIN_SAMPLE * 2;
        let worse = list("worse", size, games / 4, games);
        let better = list("better", size, games * 3 / 4, games);
        let thin = list("thin", size, CARD_STATS_MIN_SAMPLE - 1, CARD_STATS_MIN_SAMPLE - 1);
        let short = list("short", size - 1, games, games);
        for lists in [
            [worse.clone(), better.clone(), thin.clone(), short.clone()],
            [better.clone(), worse.clone(), short.clone(), thin.clone()],
        ] {
            assert_eq!(
                ranked_human_bo1_decklists(&lists),
                (
                    vec![better.deck.clone(), worse.deck.clone()],
                    vec![thin.deck.clone()]
                )
            );
        }
    }

    /// R750000: ranked lists are tried nearest the bot's strength first, the strongest bot from the best
    /// list and the weakest from the worst, a tie going to the better list.
    #[test]
    fn r750000_ranked_lists_are_tried_nearest_the_bots_strength_first() {
        let ranked: Vec<Vec<String>> = (0..5).map(|place| vec![format!("list-{place}")]).collect();
        let order = |strength: f64| -> Vec<String> {
            by_strength(ranked.clone(), strength)
                .into_iter()
                .flatten()
                .collect()
        };
        let lists = |places: [usize; 5]| -> Vec<String> {
            places.iter().map(|place| format!("list-{place}")).collect()
        };
        assert_eq!(order(1.0), lists([0, 1, 2, 3, 4]));
        assert_eq!(order(0.0), lists([4, 3, 2, 1, 0]));
        assert_eq!(order(0.5), lists([2, 1, 3, 0, 4]));
        assert_eq!(order(0.75), lists([1, 0, 2, 3, 4]));
    }
}
