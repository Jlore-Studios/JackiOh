//! `jackioh_run`: the dungeon run (`docs/dungeon-run.md`), a seeded map of fights and tiles strung
//! around plain games, the way the engine's `duos` wraps two of them. No card and no rule of a game
//! knows about a run. `README.md` is this crate's contract.
//!
//! Every rule here is the plan's (its DR sections), every number a constant in `config.rs`, and the
//! mode has no spec section yet. The content (the pool packs and shops deal, the cards any starting
//! deck may hold, the enemies, artifacts, blessings, events and curses) is data the caller passes in;
//! `run_placeholder_content` is pseudo cards and temporary bosses until the designer's lists land.
//!
//! Like the engine's `reduce`, `run_reduce` takes no rng: it rebuilds it from `(seed, rng_cursor)` and
//! writes the cursor back, so a run file (`RunFile`: its setup, the unlocks it started from and its
//! log) folds to the same state anywhere. That is how a Campaign run is checked before it pays (DR1).

pub mod config;
mod fight;

#[cfg(test)]
mod fight_tests;
#[cfg(test)]
mod tests;

pub use fight::{RunFightSetup, run_catalog_content, run_fight_report, run_fight_setup};

use jackioh_engine::config::PERCENT;
use jackioh_engine::rng::Rng;
use jackioh_engine::wire::{CardType, PrintedRarity, SINGLE_PLAYER_SETS, SetName};
use serde::{Deserialize, Serialize};

use crate::config::{
    RUN_ACT_FIGHTS, RUN_ARTIFACT_LEVEL_MAX, RUN_ARTIFACT_PERCENT, RUN_ARTIFACT_PRICE, RUN_BUFF_TICKETS,
    RUN_CAMPFIRE_HEAL_PERCENT, RUN_CAMPFIRE_REMOVE, RUN_CARD_PRICE, RUN_CHEST_ARTIFACT_PERCENT,
    RUN_CHEST_COINS, RUN_CHEST_TICKETS, RUN_COINS, RUN_ELITE_PERCENT, RUN_ENEMY_DECK_SIZE, RUN_ENEMY_HEALTH,
    RUN_GHOST_PERCENT, RUN_KEEP, RUN_KEEP_MAX, RUN_LEVEL_EMBERS, RUN_LEVEL_TICKETS, RUN_MAX_HP,
    RUN_MINI_BOSS_DROP_TIER, RUN_PACK_SIZE, RUN_RADIANT_TICKETS, RUN_REMOVE_PRICE, RUN_ROW_WIDTH_MAX,
    RUN_ROW_WIDTH_MIN, RUN_SCORE_CARD_DRAWN, RUN_SCORE_CURSE_POINT_PERCENT, RUN_SCORE_DAMAGE,
    RUN_SCORE_FIGHT, RUN_SCORE_FINAL_BOSS_PERCENT, RUN_SCORE_HEALING, RUN_SCORE_MINI_BOSS_PERCENT,
    RUN_SCORE_PER_EMBER, RUN_SCORE_ROW, RUN_SEED_LENGTH, RUN_SHOP_ARTIFACTS, RUN_SHOP_CARDS,
    RUN_SHRINE_OFFERS, RUN_SLOT_LADDER, RUN_STARTER_ARTIFACT_POINTS, RUN_STARTER_ARTIFACTS_MAX,
    RUN_STARTER_SIZE, RUN_TICKETS, RUN_TIER_POINTS, RUN_TILE_WEIGHTS,
};

// ---------------------------------------------------------------------------------------------
// Content: data the caller passes in
// ---------------------------------------------------------------------------------------------

/// What a run deals from: the designer's lists, or `run_placeholder_content` until they land. A
/// Sandbox run's caller adds the player's custom cards to `pool` (DR1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunContent {
    /// Every card a run may meet: packs, shops, events and enemy decks deal from it, and a card of a
    /// single-player set (a boss card) only through its boss's deck.
    pub pool: Vec<RunPoolCard>,
    /// The cards any starting deck may hold; the unlocks' extracted cards join them (DR3).
    pub starter_access: Vec<String>,
    pub enemies: Vec<RunEnemyDef>,
    /// Artifacts, enemy artifacts and blessings.
    pub passives: Vec<RunPassiveDef>,
    pub events: Vec<RunEventDef>,
    pub curses: Vec<RunCurseDef>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunPoolCard {
    pub id: String,
    pub name: String,
    pub rarity: PrintedRarity,
    pub set: SetName,
    pub card_type: CardType,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunEnemyDef {
    pub id: String,
    pub name: String,
    /// `fight`, `elite`, `miniBoss` or `boss`.
    pub tier: RunNodeKind,
    /// Its own cards (a boss's boss cards); the pool deals the rest of `RUN_ENEMY_DECK_SIZE`.
    #[serde(default)]
    pub deck: Vec<String>,
    /// The artifacts it carries into the fight: Looming or Overwhelming Presence, a boss's own (DR5).
    #[serde(default)]
    pub artifacts: Vec<String>,
    /// A boss's drop (DR6). The final boss's is its own Unobtainable artifact. A mini-boss without
    /// one drops a random artifact of `RUN_MINI_BOSS_DROP_TIER` or better, chosen by the seed.
    #[serde(default)]
    pub drop: Option<String>,
}

/// DR3, DR5: what a passive is. A Combat artifact works inside a fight, an Economic one on the map;
/// an Enemy one is carried only by enemies; a blessing is a shrine's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum RunPassiveKind {
    Combat,
    Economic,
    Enemy,
    Blessing,
}

/// DR3: an artifact's strength, and so its points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum RunTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
    Diamond,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunPassiveDef {
    pub id: String,
    pub name: String,
    pub kind: RunPassiveKind,
    pub text: String,
    /// A player's artifact's tier; none for an enemy's artifact or a blessing.
    #[serde(default)]
    pub tier: Option<RunTier>,
    /// What it does on the map. A Combat or Enemy artifact has none: Looming and Overwhelming Presence
    /// act as `run_fight_setup`'s handicaps, and Combat artifacts change no fight yet (M4).
    #[serde(default)]
    pub effect: Option<RunEffect>,
    /// Any starting deck may take it; extracted artifacts join these.
    #[serde(default)]
    pub starter: bool,
    /// DR5: only its boss gives it, nothing else deals it, and it cannot be extracted.
    #[serde(default)]
    pub unobtainable: bool,
    /// A blessing's god.
    #[serde(default)]
    pub god: Option<String>,
}

/// A passive's effect on the map. An artifact's counts once per level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RunEffect {
    /// Keep more cards from each pack, up to `RUN_KEEP_MAX`.
    KeepMore {
        n: i32,
    },
    /// A pack may be kept empty, for Coins.
    Skip {
        coins: i32,
    },
    /// Packs show more cards.
    PackSize {
        n: i32,
    },
    /// Shop Coin prices fall by this percent.
    ShopDiscount {
        percent: i32,
    },
    CoinsPerFight {
        n: i32,
    },
    TicketsPerFight {
        n: i32,
    },
    MaxHp {
        n: i32,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunEventDef {
    pub id: String,
    pub text: String,
    pub options: Vec<RunEventOption>,
}

/// An event's choice and what it changes. A cost the run cannot pay, or HP it cannot spare, refuses it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunEventOption {
    pub text: String,
    #[serde(default)]
    pub hp: i32,
    #[serde(default)]
    pub coins: i32,
    #[serde(default)]
    pub tickets: i32,
    /// Adds a random pool card to the deck.
    #[serde(default)]
    pub card: bool,
    /// Gives a random artifact.
    #[serde(default)]
    pub artifact: bool,
}

/// DR8: a curse tier, taken at the start. Its points raise the score. It changes no fight yet (M4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunCurseDef {
    pub id: String,
    pub name: String,
    pub text: String,
    pub points: i32,
}

/// Pseudo cards and temporary bosses, artifacts, blessings, events and curses, until the designer's
/// lists replace them. Parsed on each call: a pure crate keeps no statics.
pub fn run_placeholder_content() -> RunContent {
    serde_json::from_str(include_str!("placeholder.json")).expect("placeholder.json is valid content")
}

// ---------------------------------------------------------------------------------------------
// The run's state
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum RunNodeKind {
    Fight,
    Elite,
    Ghost,
    MiniBoss,
    Boss,
    Shop,
    Campfire,
    Event,
    Chest,
    Shrine,
}

impl RunNodeKind {
    pub fn is_fight(self) -> bool {
        matches!(
            self,
            RunNodeKind::Fight
                | RunNodeKind::Elite
                | RunNodeKind::Ghost
                | RunNodeKind::MiniBoss
                | RunNodeKind::Boss
        )
    }

    pub fn is_boss(self) -> bool {
        matches!(self, RunNodeKind::MiniBoss | RunNodeKind::Boss)
    }
}

/// A number for each kind of fight (`config.rs`'s tables).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunByFight {
    pub fight: i32,
    pub elite: i32,
    pub ghost: i32,
    pub mini_boss: i32,
    pub boss: i32,
}

impl RunByFight {
    pub fn of(&self, kind: RunNodeKind) -> i32 {
        match kind {
            RunNodeKind::Fight => self.fight,
            RunNodeKind::Elite => self.elite,
            RunNodeKind::Ghost => self.ghost,
            RunNodeKind::MiniBoss => self.mini_boss,
            RunNodeKind::Boss => self.boss,
            _ => 0,
        }
    }
}

/// What carries between runs (DR3): the starting deck's level, the cards and artifacts extracted, and
/// in the Campaign the account's collection, since an extracted card enters a starting deck only if
/// the account owns it. The caller keeps it: the device in the Sandbox, the account in the Campaign.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunUnlocks {
    /// From 1: the row of `RUN_SLOT_LADDER` the starting deck's slots are.
    pub level: usize,
    pub cards: Vec<String>,
    pub artifacts: Vec<String>,
    /// The account's cards, in the Campaign; none in the Sandbox, which asks for no ownership.
    #[serde(default)]
    pub owned: Option<Vec<String>>,
}

impl Default for RunUnlocks {
    fn default() -> Self {
        RunUnlocks {
            level: 1,
            cards: Vec::new(),
            artifacts: Vec::new(),
            owned: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunSetup {
    /// `RUN_SEED_LENGTH` capital letters (`run_seed`, `run_daily_seed`).
    pub seed: String,
    /// `RUN_STARTER_SIZE` distinct cards that fit the level's slots.
    pub starter: Vec<String>,
    /// At most `RUN_STARTER_ARTIFACTS_MAX`, worth at most `RUN_STARTER_ARTIFACT_POINTS`.
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub curses: Vec<String>,
}

/// A whole run as one file: what it started from and every choice made. `run_fold` replays it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunFile {
    pub setup: RunSetup,
    pub unlocks: RunUnlocks,
    pub log: Vec<RunAction>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunNode {
    pub kind: RunNodeKind,
    #[serde(default)]
    pub enemy: Option<RunEnemy>,
    /// An Event node's event.
    #[serde(default)]
    pub event: Option<String>,
    /// A boss's preseeded drop, shown on the map (DR6).
    #[serde(default)]
    pub artifact: Option<String>,
}

/// An enemy as the map dealt it: what the fight's setup needs (`run_fight_setup`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunEnemy {
    pub id: String,
    pub name: String,
    pub hero_health: i32,
    pub deck: Vec<String>,
    pub artifacts: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunCard {
    pub id: String,
    /// Made Radiant at a campfire or a shop. A fight deals it Radiant (`run_fight_setup`'s marks).
    pub radiant: bool,
    /// Buffs from campfires and shops, the Buff keyword's, which a fight applies at its setup
    /// (`run_fight_setup`'s marks).
    pub buffs: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunArtifact {
    pub id: String,
    pub level: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RunPackKind {
    /// The cards of the deck just beaten, its boss cards included.
    Boss,
    Random,
    Set {
        set: SetName,
    },
    CardType {
        card_type: CardType,
    },
    Tag {
        tag: String,
    },
    Rarity {
        rarity: PrintedRarity,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RunShopGood {
    Card { id: String },
    Artifact { id: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunShopItem {
    pub good: RunShopGood,
    /// In Coins, before a discount.
    pub price: i32,
    pub sold: bool,
}

/// What the run waits on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "phase", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RunPhase {
    /// Choose a node of the next row (`Go`).
    Map,
    /// The node's fight is being played (`FightOver`).
    Fight,
    /// Take or leave the artifact first (`TakeArtifact`), then open one pack (`OpenPack`).
    Reward {
        artifact: Option<String>,
        packs: Vec<RunPackKind>,
    },
    /// Keep exactly `keep` of `cards` (`Keep`), or none with a Skip passive.
    Pack {
        cards: Vec<String>,
        keep: usize,
    },
    /// After a mini-boss: extract one card of the deck, or none (`Extract`).
    ExtractCard,
    /// After the final boss: extract one artifact held, never an Unobtainable one, or none.
    ExtractArtifact,
    Shop {
        stock: Vec<RunShopItem>,
    },
    /// `Heal`, `MakeRadiant`, `Buff` or `RemoveCards`, free.
    Campfire,
    Event {
        id: String,
    },
    Shrine {
        offers: Vec<String>,
    },
    Won,
    Dead,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RunAction {
    Go {
        col: usize,
    },
    /// The fight's result as the game ended: the hero's health and what the score reads.
    FightOver {
        won: bool,
        hp: i32,
        #[serde(default)]
        damage_dealt: i32,
        #[serde(default)]
        healing_done: i32,
        #[serde(default)]
        cards_drawn: i32,
        #[serde(default)]
        turns: i32,
    },
    TakeArtifact {
        take: bool,
    },
    OpenPack {
        index: usize,
    },
    Keep {
        cards: Vec<String>,
    },
    Extract {
        index: Option<usize>,
    },
    Buy {
        index: usize,
    },
    /// A shop's removal, for Coins.
    RemoveCard {
        index: usize,
    },
    /// A campfire's two removals.
    RemoveCards {
        indices: Vec<usize>,
    },
    MakeRadiant {
        index: usize,
    },
    Buff {
        index: usize,
    },
    LevelArtifact {
        index: usize,
    },
    Heal,
    /// An event's option.
    Choose {
        index: usize,
    },
    Bless {
        index: Option<usize>,
    },
    /// Leave a shop.
    Leave,
    Abandon,
}

/// Everything a leaderboard or a stats page reads from one run (DR9).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunStats {
    pub nodes_visited: i32,
    /// Every fight won, of every kind.
    pub fights_won: i32,
    pub elites_won: i32,
    pub ghosts_won: i32,
    pub mini_bosses_won: i32,
    pub bosses_won: i32,
    pub damage_dealt: i32,
    pub healing_done: i32,
    pub cards_drawn: i32,
    pub turns: i32,
    pub damage_taken: i32,
    pub healed: i32,
    pub coins_earned: i32,
    pub coins_spent: i32,
    pub tickets_earned: i32,
    pub tickets_spent: i32,
    pub cards_kept: i32,
    pub cards_bought: i32,
    pub cards_removed: i32,
    pub cards_made_radiant: i32,
    pub cards_buffed: i32,
    pub packs_skipped: i32,
    pub artifacts_taken: i32,
    pub artifacts_leveled: i32,
    pub blessings_taken: i32,
    pub events_seen: i32,
    pub chests_opened: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunState {
    pub seed: String,
    pub rng_cursor: u32,
    /// Rows of nodes; any node of the next row may be chosen.
    pub map: Vec<Vec<RunNode>>,
    /// The row stood on; none before the first step.
    pub row: Option<usize>,
    pub col: usize,
    pub hp: i32,
    pub deck: Vec<RunCard>,
    pub artifacts: Vec<RunArtifact>,
    pub blessings: Vec<String>,
    pub curses: Vec<String>,
    pub coins: i32,
    pub tickets: i32,
    /// What this run extracted, for the caller to add to the unlocks.
    pub extracted_cards: Vec<String>,
    pub extracted_artifacts: Vec<String>,
    pub phase: RunPhase,
    pub stats: RunStats,
}

// ---------------------------------------------------------------------------------------------
// Seeds
// ---------------------------------------------------------------------------------------------

const SEED_LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// DR2: a seed of `RUN_SEED_LENGTH` capital letters drawn from any text, such as a number the client
/// rolled at random.
pub fn run_seed(source: &str) -> String {
    let mut rng = Rng::new(source, 0);
    (0..RUN_SEED_LENGTH)
        .filter_map(|_| rng.pick(SEED_LETTERS).map(|b| char::from(*b)))
        .collect()
}

/// DR2: the daily run's seed: the date's (as the client writes it, `2026-11-01`) and the day's run
/// number, so everyone's nth run of a day shares one seed.
pub fn run_daily_seed(date: &str, run: u32) -> String {
    run_seed(&format!("daily:{date}:{run}"))
}

fn seed_is_valid(seed: &str) -> bool {
    seed.len() == RUN_SEED_LENGTH && seed.bytes().all(|b| SEED_LETTERS.contains(&b))
}

// ---------------------------------------------------------------------------------------------
// The starting deck
// ---------------------------------------------------------------------------------------------

/// The slots' names, from the lowest: `RUN_SLOT_LADDER`'s columns.
const SLOT_NAMES: [&str; 6] = ["Common", "Rare", "Epic", "Legendary", "Mythic", "Anything"];

fn card_def<'a>(content: &'a RunContent, id: &str) -> Option<&'a RunPoolCard> {
    content.pool.iter().find(|c| c.id == id)
}

fn passive_def<'a>(content: &'a RunContent, id: &str) -> Option<&'a RunPassiveDef> {
    content.passives.iter().find(|p| p.id == id)
}

fn is_boss_card(card: &RunPoolCard) -> bool {
    SINGLE_PLAYER_SETS.contains(&card.set)
}

/// The lowest slot a card fits (`SLOT_NAMES`): its rarity's, or Anything for a boss card.
fn slot_needed(card: &RunPoolCard) -> usize {
    if is_boss_card(card) {
        return SLOT_NAMES.len() - 1;
    }
    match card.rarity {
        PrintedRarity::Common => 0,
        PrintedRarity::Rare => 1,
        PrintedRarity::Epic => 2,
        PrintedRarity::Legendary => 3,
        PrintedRarity::Mythic => 4,
    }
}

fn by<K: PartialEq + Copy>(rows: &[(K, i32)], key: K) -> i32 {
    rows.iter().find(|(k, _)| *k == key).map_or(0, |(_, n)| *n)
}

/// DR3: an artifact's points, by its tier.
pub fn run_artifact_points(content: &RunContent, id: &str) -> i32 {
    passive_def(content, id)
        .and_then(|p| p.tier)
        .map_or(0, |t| by(&RUN_TIER_POINTS, t))
}

/// DR3: the starting deck is `RUN_STARTER_SIZE` distinct cards, each a starting card or one extracted
/// (and in the Campaign owned, a boss card excepted), that fit the level's slots; and at most
/// `RUN_STARTER_ARTIFACTS_MAX` artifacts worth at most `RUN_STARTER_ARTIFACT_POINTS`, each a starting
/// one or one extracted.
pub fn run_check_setup(setup: &RunSetup, unlocks: &RunUnlocks, content: &RunContent) -> Result<(), String> {
    if !seed_is_valid(&setup.seed) {
        return Err(format!(
            "a seed is {RUN_SEED_LENGTH} capital letters, not {:?}",
            setup.seed
        ));
    }
    if setup.starter.len() != RUN_STARTER_SIZE {
        return Err(format!(
            "a starting deck holds {RUN_STARTER_SIZE} cards, not {}",
            setup.starter.len()
        ));
    }
    let mut needs = Vec::new();
    for (i, id) in setup.starter.iter().enumerate() {
        if setup.starter[..i].contains(id) {
            return Err(format!("{id} is in the starting deck twice"));
        }
        let Some(card) = card_def(content, id) else {
            return Err(format!("{id} is no card of the run's pool"));
        };
        if !content.starter_access.contains(id) {
            if !unlocks.cards.contains(id) {
                return Err(format!("{id} is not unlocked: extract it at a mini-boss first"));
            }
            if let Some(owned) = &unlocks.owned
                && !is_boss_card(card)
                && !owned.contains(id)
            {
                return Err(format!("{id} is unlocked, but the account does not own it"));
            }
        }
        needs.push(slot_needed(card));
    }
    let slots = unlocks
        .level
        .checked_sub(1)
        .and_then(|i| RUN_SLOT_LADDER.get(i))
        .ok_or_else(|| format!("there is no level {}", unlocks.level))?;
    for (k, name) in SLOT_NAMES.iter().enumerate() {
        let cards = needs.iter().filter(|n| **n >= k).count();
        let open: usize = slots[k..].iter().sum();
        if cards > open {
            return Err(format!(
                "{cards} cards need a slot of {name} or better, and level {} has {open}",
                unlocks.level
            ));
        }
    }
    if setup.artifacts.len() > RUN_STARTER_ARTIFACTS_MAX {
        return Err(format!(
            "a run starts with at most {RUN_STARTER_ARTIFACTS_MAX} artifacts"
        ));
    }
    for (i, id) in setup.artifacts.iter().enumerate() {
        if setup.artifacts[..i].contains(id) {
            return Err(format!("{id} is taken twice"));
        }
        let Some(def) = passive_def(content, id) else {
            return Err(format!("{id} is no artifact"));
        };
        if !matches!(def.kind, RunPassiveKind::Combat | RunPassiveKind::Economic) || def.unobtainable {
            return Err(format!("{id} cannot start a run"));
        }
        if !def.starter && !unlocks.artifacts.contains(id) {
            return Err(format!("{id} is not unlocked: extract it at a final boss first"));
        }
    }
    let points: i32 = setup
        .artifacts
        .iter()
        .map(|id| run_artifact_points(content, id))
        .sum();
    if points > RUN_STARTER_ARTIFACT_POINTS {
        return Err(format!(
            "the artifacts are worth {points} points, over {RUN_STARTER_ARTIFACT_POINTS}"
        ));
    }
    for id in &setup.curses {
        if !content.curses.iter().any(|c| &c.id == id) {
            return Err(format!("{id} is no curse"));
        }
    }
    Ok(())
}

fn check_content(content: &RunContent) -> Result<(), String> {
    for tier in [
        RunNodeKind::Fight,
        RunNodeKind::Elite,
        RunNodeKind::MiniBoss,
        RunNodeKind::Boss,
    ] {
        if !content.enemies.iter().any(|e| e.tier == tier) {
            return Err(format!("the content has no {tier:?} enemy"));
        }
    }
    if content.events.is_empty() {
        return Err("the content has no event".into());
    }
    Ok(())
}

/// A run at its start: the setup checked, the map dealt from the seed.
pub fn run_create(setup: &RunSetup, unlocks: &RunUnlocks, content: &RunContent) -> Result<RunState, String> {
    run_check_setup(setup, unlocks, content)?;
    check_content(content)?;
    let mut rng = Rng::new(&setup.seed, 0);
    let held: Vec<&str> = setup.artifacts.iter().map(String::as_str).collect();
    let map = deal_map(content, &held, &mut rng);
    let mut state = RunState {
        seed: setup.seed.clone(),
        rng_cursor: 0,
        map,
        row: None,
        col: 0,
        hp: RUN_MAX_HP,
        deck: setup
            .starter
            .iter()
            .map(|id| RunCard {
                id: id.clone(),
                radiant: false,
                buffs: 0,
            })
            .collect(),
        artifacts: setup
            .artifacts
            .iter()
            .map(|id| RunArtifact {
                id: id.clone(),
                level: 1,
            })
            .collect(),
        blessings: Vec::new(),
        curses: setup.curses.clone(),
        coins: 0,
        tickets: 0,
        extracted_cards: Vec::new(),
        extracted_artifacts: Vec::new(),
        phase: RunPhase::Map,
        stats: RunStats::default(),
    };
    state.hp = run_max_hp(&state, content);
    state.rng_cursor = rng.cursor();
    Ok(state)
}

// ---------------------------------------------------------------------------------------------
// The map
// ---------------------------------------------------------------------------------------------

fn weighted<T: Copy>(rows: &[(T, i32)], rng: &mut Rng) -> T {
    let total: i32 = rows.iter().map(|(_, w)| w).sum();
    let mut roll = rng.int(total);
    for (item, w) in rows {
        if roll < *w {
            return *item;
        }
        roll -= w;
    }
    rows[rows.len() - 1].0
}

/// The pool's cards an enemy deck, a shop, an event or a pack other than a boss's may deal: no boss card.
fn open_pool(content: &RunContent) -> impl Iterator<Item = &RunPoolCard> {
    content.pool.iter().filter(|c| !is_boss_card(c))
}

/// `own` first, then cards of the open pool dealt to fill `RUN_ENEMY_DECK_SIZE`.
fn deal_deck(content: &RunContent, own: &[String], rng: &mut Rng) -> Vec<String> {
    let rest: Vec<String> = open_pool(content)
        .filter(|c| !own.contains(&c.id))
        .map(|c| c.id.clone())
        .collect();
    let fill = RUN_ENEMY_DECK_SIZE.saturating_sub(own.len());
    own.iter()
        .cloned()
        .chain(rng.shuffle(&rest).into_iter().take(fill))
        .collect()
}

/// An artifact any roll may deal: a player's (Combat or Economic), not Unobtainable, not held, of
/// `least` or better.
fn random_artifact(
    content: &RunContent,
    held: &[&str],
    least: Option<RunTier>,
    rng: &mut Rng,
) -> Option<String> {
    let open: Vec<&RunPassiveDef> = content
        .passives
        .iter()
        .filter(|p| matches!(p.kind, RunPassiveKind::Combat | RunPassiveKind::Economic))
        .filter(|p| !p.unobtainable && !held.contains(&p.id.as_str()))
        .filter(|p| least.is_none_or(|least| p.tier.is_some_and(|t| t >= least)))
        .collect();
    rng.pick(&open).map(|p| p.id.clone())
}

fn fight_node(content: &RunContent, kind: RunNodeKind, held: &[&str], rng: &mut Rng) -> RunNode {
    if kind == RunNodeKind::Ghost {
        // DR5: a ghost is a player's deck from a death near here; until snapshots exist, a random one.
        let enemy = RunEnemy {
            id: "ghost".into(),
            name: "Ghost".into(),
            hero_health: RUN_ENEMY_HEALTH.of(kind),
            deck: deal_deck(content, &[], rng),
            artifacts: Vec::new(),
        };
        return RunNode {
            kind,
            enemy: Some(enemy),
            event: None,
            artifact: None,
        };
    }
    let defs: Vec<&RunEnemyDef> = content.enemies.iter().filter(|e| e.tier == kind).collect();
    let def = *rng.pick(&defs).expect("check_content keeps every tier");
    let enemy = RunEnemy {
        id: def.id.clone(),
        name: def.name.clone(),
        hero_health: RUN_ENEMY_HEALTH.of(kind),
        deck: deal_deck(content, &def.deck, rng),
        artifacts: def.artifacts.clone(),
    };
    let artifact = match kind {
        RunNodeKind::Boss => def.drop.clone(),
        RunNodeKind::MiniBoss => def
            .drop
            .clone()
            .or_else(|| random_artifact(content, held, Some(RUN_MINI_BOSS_DROP_TIER), rng)),
        _ => None,
    };
    RunNode {
        kind,
        enemy: Some(enemy),
        event: None,
        artifact,
    }
}

fn tile_node(content: &RunContent, kind: RunNodeKind, rng: &mut Rng) -> RunNode {
    let event =
        (kind == RunNodeKind::Event).then(|| rng.pick(&content.events).expect("check_content").id.clone());
    RunNode {
        kind,
        enemy: None,
        event,
        artifact: None,
    }
}

fn row_width(rng: &mut Rng) -> i32 {
    RUN_ROW_WIDTH_MIN + rng.int(RUN_ROW_WIDTH_MAX - RUN_ROW_WIDTH_MIN + 1)
}

/// DR4: per act, its fight rows with a tile row between each two, a campfire, and the boss.
fn deal_map(content: &RunContent, held: &[&str], rng: &mut Rng) -> Vec<Vec<RunNode>> {
    let mut rows = Vec::new();
    for (act, fights) in RUN_ACT_FIGHTS.iter().enumerate() {
        for f in 0..*fights {
            if f > 0 {
                let width = row_width(rng);
                rows.push(
                    (0..width)
                        .map(|_| tile_node(content, weighted(&RUN_TILE_WEIGHTS, rng), rng))
                        .collect(),
                );
            }
            let width = row_width(rng);
            let row = (0..width)
                .map(|_| {
                    let kind = if rng.int(PERCENT) < RUN_ELITE_PERCENT {
                        RunNodeKind::Elite
                    } else if rng.int(PERCENT) < RUN_GHOST_PERCENT {
                        RunNodeKind::Ghost
                    } else {
                        RunNodeKind::Fight
                    };
                    fight_node(content, kind, held, rng)
                })
                .collect();
            rows.push(row);
        }
        rows.push(vec![tile_node(content, RunNodeKind::Campfire, rng)]);
        let boss = if act + 1 == RUN_ACT_FIGHTS.len() {
            RunNodeKind::Boss
        } else {
            RunNodeKind::MiniBoss
        };
        rows.push(vec![fight_node(content, boss, held, rng)]);
    }
    rows
}

// ---------------------------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------------------------

/// The node stood on.
pub fn run_node(state: &RunState) -> Option<&RunNode> {
    state.row.map(|r| &state.map[r][state.col])
}

/// The enemy of the fight being played, for the game's setup (`run_fight_setup`).
pub fn run_enemy(state: &RunState) -> Option<&RunEnemy> {
    match state.phase {
        RunPhase::Fight => run_node(state).and_then(|n| n.enemy.as_ref()),
        _ => None,
    }
}

/// The sum of one effect over the artifacts held (times their level) and the blessings.
fn passive_total(state: &RunState, content: &RunContent, read: impl Fn(RunEffect) -> Option<i32>) -> i32 {
    let artifacts = state.artifacts.iter().map(|a| (a.id.as_str(), a.level));
    let blessings = state.blessings.iter().map(|b| (b.as_str(), 1));
    artifacts
        .chain(blessings)
        .filter_map(|(id, times)| {
            passive_def(content, id)
                .and_then(|p| p.effect)
                .and_then(&read)
                .map(|n| n * times)
        })
        .sum()
}

pub fn run_max_hp(state: &RunState, content: &RunContent) -> i32 {
    RUN_MAX_HP
        + passive_total(state, content, |e| match e {
            RunEffect::MaxHp { n } => Some(n),
            _ => None,
        })
}

/// DR8: the run's curse points.
pub fn run_curse_points(state: &RunState, content: &RunContent) -> i32 {
    state
        .curses
        .iter()
        .filter_map(|id| content.curses.iter().find(|c| &c.id == id))
        .map(|c| c.points)
        .sum()
}

/// DR9: the score so far. Rows reached, fights won by kind and what the fights' reports dealt, healed
/// and drew; times `RUN_SCORE_MINI_BOSS_PERCENT` for each mini-boss beaten and
/// `RUN_SCORE_FINAL_BOSS_PERCENT` for the final boss; plus `RUN_SCORE_CURSE_POINT_PERCENT` for each
/// curse point.
pub fn run_score(state: &RunState, content: &RunContent) -> i64 {
    let s = &state.stats;
    let won = |kind: RunNodeKind, n: i32| i64::from(RUN_SCORE_FIGHT.of(kind)) * i64::from(n);
    let normal = s.fights_won - s.elites_won - s.ghosts_won - s.mini_bosses_won - s.bosses_won;
    let rows = state.row.map_or(0, |r| r + 1);
    let mut score = RUN_SCORE_ROW * i64::try_from(rows).unwrap_or(0)
        + won(RunNodeKind::Fight, normal)
        + won(RunNodeKind::Elite, s.elites_won)
        + won(RunNodeKind::Ghost, s.ghosts_won)
        + won(RunNodeKind::MiniBoss, s.mini_bosses_won)
        + won(RunNodeKind::Boss, s.bosses_won)
        + RUN_SCORE_DAMAGE * i64::from(s.damage_dealt)
        + RUN_SCORE_HEALING * i64::from(s.healing_done)
        + RUN_SCORE_CARD_DRAWN * i64::from(s.cards_drawn);
    let whole = i64::from(PERCENT);
    for _ in 0..s.mini_bosses_won {
        score = score * RUN_SCORE_MINI_BOSS_PERCENT / whole;
    }
    if s.bosses_won > 0 {
        score = score * RUN_SCORE_FINAL_BOSS_PERCENT / whole;
    }
    score * (whole + RUN_SCORE_CURSE_POINT_PERCENT * i64::from(run_curse_points(state, content))) / whole
}

/// DR9: the Embers a score pays.
pub fn run_embers(score: i64) -> i64 {
    score / RUN_SCORE_PER_EMBER
}

/// DR3, DR9: the Embers that raise the starting deck from `level` to the next; `None` at the ladder's
/// top or for a level that is none.
pub fn run_level_cost(level: usize) -> Option<i64> {
    level
        .checked_sub(1)
        .and_then(|step| RUN_LEVEL_EMBERS.get(step))
        .copied()
}

/// The run's numbers the screens show, so the client states none (CLAUDE.md rule 9): `config.rs`'s
/// own values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunConstants {
    /// `RUN_SLOT_LADDER`: each level's slots, from level 1.
    pub slot_ladder: Vec<[usize; 6]>,
    /// `RUN_STARTER_SIZE`.
    pub starter_size: usize,
    /// `RUN_STARTER_ARTIFACTS_MAX`.
    pub starter_artifacts_max: usize,
    /// `RUN_STARTER_ARTIFACT_POINTS`.
    pub starter_artifact_points: i32,
    /// `RUN_TIER_POINTS`: an artifact's points by its tier.
    pub tier_points: Vec<(RunTier, i32)>,
}

/// `RunConstants` from `config.rs`.
pub fn run_constants() -> RunConstants {
    RunConstants {
        slot_ladder: RUN_SLOT_LADDER.to_vec(),
        starter_size: RUN_STARTER_SIZE,
        starter_artifacts_max: RUN_STARTER_ARTIFACTS_MAX,
        starter_artifact_points: RUN_STARTER_ARTIFACT_POINTS,
        tier_points: RUN_TIER_POINTS.to_vec(),
    }
}

// ---------------------------------------------------------------------------------------------
// The reducer
// ---------------------------------------------------------------------------------------------

/// The run after one action, or why the action is refused (the state unchanged).
pub fn run_reduce(state: &RunState, content: &RunContent, action: &RunAction) -> Result<RunState, String> {
    let mut next = state.clone();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    apply(&mut next, content, action, &mut rng)?;
    next.rng_cursor = rng.cursor();
    Ok(next)
}

/// A run file replayed from its start; the error names the first refused step.
pub fn run_fold(file: &RunFile, content: &RunContent) -> Result<RunState, String> {
    let mut state = run_create(&file.setup, &file.unlocks, content)?;
    for (step, action) in file.log.iter().enumerate() {
        state = run_reduce(&state, content, action).map_err(|e| format!("step {step}: {e}"))?;
    }
    Ok(state)
}

fn held_ids(state: &RunState) -> Vec<&str> {
    state.artifacts.iter().map(|a| a.id.as_str()).collect()
}

fn in_deck(state: &RunState, id: &str) -> bool {
    state.deck.iter().any(|d| d.id == id)
}

fn spend_coins(state: &mut RunState, price: i32) -> Result<(), String> {
    if price > state.coins {
        return Err(format!("{price} Coins needed, {} held", state.coins));
    }
    state.coins -= price;
    state.stats.coins_spent += price;
    Ok(())
}

fn spend_tickets(state: &mut RunState, price: i32) -> Result<(), String> {
    if price > state.tickets {
        return Err(format!("{price} Tickets needed, {} held", state.tickets));
    }
    state.tickets -= price;
    state.stats.tickets_spent += price;
    Ok(())
}

fn earn(state: &mut RunState, coins: i32, tickets: i32) {
    state.coins += coins;
    state.tickets += tickets;
    state.stats.coins_earned += coins;
    state.stats.tickets_earned += tickets;
}

fn discounted(state: &RunState, content: &RunContent, price: i32) -> i32 {
    let off = passive_total(state, content, |e| match e {
        RunEffect::ShopDiscount { percent } => Some(percent),
        _ => None,
    });
    price * (PERCENT - off).max(0) / PERCENT
}

/// Runs `change`, then moves HP by what it changed the max by: a higher max heals that much, and HP
/// never stays above a lower one.
fn with_max_hp(state: &mut RunState, content: &RunContent, change: impl FnOnce(&mut RunState)) {
    let before = run_max_hp(state, content);
    change(state);
    let after = run_max_hp(state, content);
    state.hp = (state.hp + (after - before).max(0)).min(after);
}

fn gain_artifact(state: &mut RunState, content: &RunContent, id: String) {
    with_max_hp(state, content, |s| s.artifacts.push(RunArtifact { id, level: 1 }));
    state.stats.artifacts_taken += 1;
}

fn make_radiant(state: &mut RunState, index: usize) -> Result<(), String> {
    let card = state.deck.get_mut(index).ok_or("no such card")?;
    if card.radiant {
        return Err(format!("{} is Radiant already", card.id));
    }
    card.radiant = true;
    state.stats.cards_made_radiant += 1;
    Ok(())
}

fn buff(state: &mut RunState, index: usize) -> Result<(), String> {
    state.deck.get_mut(index).ok_or("no such card")?.buffs += 1;
    state.stats.cards_buffed += 1;
    Ok(())
}

/// Each value once, in first-seen order.
fn distinct<T: PartialEq + Clone>(items: impl Iterator<Item = T>) -> Vec<T> {
    let mut out: Vec<T> = Vec::new();
    for item in items {
        if !out.contains(&item) {
            out.push(item);
        }
    }
    out
}

/// DR6: the beaten enemy's pack, a random one, a set's and a type's, tag's or rarity's.
fn roll_packs(content: &RunContent, rng: &mut Rng) -> Vec<RunPackKind> {
    let sets = distinct(open_pool(content).map(|c| c.set));
    let types = distinct(open_pool(content).map(|c| c.card_type));
    let tags = distinct(open_pool(content).flat_map(|c| c.tags.iter().cloned()));
    let rarities = distinct(open_pool(content).map(|c| c.rarity));
    let set = rng.pick(&sets).map(|s| RunPackKind::Set { set: *s });
    let filters = [
        rng.pick(&types).map(|t| RunPackKind::CardType { card_type: *t }),
        rng.pick(&tags).map(|t| RunPackKind::Tag { tag: t.clone() }),
        rng.pick(&rarities).map(|r| RunPackKind::Rarity { rarity: *r }),
    ];
    let filter = rng.pick(&filters).cloned().flatten();
    vec![
        RunPackKind::Boss,
        RunPackKind::Random,
        set.unwrap_or(RunPackKind::Random),
        filter.unwrap_or(RunPackKind::Random),
    ]
}

fn open_pack(state: &RunState, content: &RunContent, kind: &RunPackKind, rng: &mut Rng) -> Vec<String> {
    let candidates: Vec<String> = match kind {
        RunPackKind::Boss => {
            let deck = run_node(state)
                .and_then(|n| n.enemy.as_ref())
                .map_or(&[][..], |e| &e.deck[..]);
            distinct(deck.iter().filter(|id| !in_deck(state, id)).cloned())
        }
        _ => open_pool(content)
            .filter(|c| !in_deck(state, &c.id))
            .filter(|c| match kind {
                RunPackKind::Set { set } => c.set == *set,
                RunPackKind::CardType { card_type } => c.card_type == *card_type,
                RunPackKind::Tag { tag } => c.tags.contains(tag),
                RunPackKind::Rarity { rarity } => c.rarity == *rarity,
                RunPackKind::Boss | RunPackKind::Random => true,
            })
            .map(|c| c.id.clone())
            .collect(),
    };
    let more = passive_total(state, content, |e| match e {
        RunEffect::PackSize { n } => Some(n),
        _ => None,
    });
    let size = RUN_PACK_SIZE + usize::try_from(more).unwrap_or(0);
    rng.shuffle(&candidates).into_iter().take(size).collect()
}

fn keep_count(state: &RunState, content: &RunContent, shown: usize) -> usize {
    let more = passive_total(state, content, |e| match e {
        RunEffect::KeepMore { n } => Some(n),
        _ => None,
    });
    (RUN_KEEP + usize::try_from(more).unwrap_or(0))
        .min(RUN_KEEP_MAX)
        .min(shown)
}

fn roll_shop(state: &RunState, content: &RunContent, rng: &mut Rng) -> Vec<RunShopItem> {
    let cards: Vec<&RunPoolCard> = open_pool(content).filter(|c| !in_deck(state, &c.id)).collect();
    let mut stock: Vec<RunShopItem> = rng
        .shuffle(&cards)
        .iter()
        .take(RUN_SHOP_CARDS)
        .map(|c| RunShopItem {
            good: RunShopGood::Card { id: c.id.clone() },
            price: by(&RUN_CARD_PRICE, c.rarity),
            sold: false,
        })
        .collect();
    let mut held: Vec<String> = held_ids(state).into_iter().map(String::from).collect();
    for _ in 0..RUN_SHOP_ARTIFACTS {
        let held_refs: Vec<&str> = held.iter().map(String::as_str).collect();
        let Some(id) = random_artifact(content, &held_refs, None, rng) else {
            break;
        };
        let price = passive_def(content, &id)
            .and_then(|p| p.tier)
            .map_or(0, |t| by(&RUN_ARTIFACT_PRICE, t));
        stock.push(RunShopItem {
            good: RunShopGood::Artifact { id: id.clone() },
            price,
            sold: false,
        });
        held.push(id);
    }
    stock
}

fn enter(state: &mut RunState, content: &RunContent, rng: &mut Rng) {
    let node = run_node(state).expect("enter follows a step").clone();
    state.stats.nodes_visited += 1;
    state.phase = match node.kind {
        kind if kind.is_fight() => RunPhase::Fight,
        RunNodeKind::Shop => RunPhase::Shop {
            stock: roll_shop(state, content, rng),
        },
        RunNodeKind::Campfire => RunPhase::Campfire,
        RunNodeKind::Event => {
            state.stats.events_seen += 1;
            RunPhase::Event {
                id: node.event.unwrap_or_default(),
            }
        }
        RunNodeKind::Chest => {
            earn(state, RUN_CHEST_COINS, RUN_CHEST_TICKETS);
            state.stats.chests_opened += 1;
            if rng.int(PERCENT) < RUN_CHEST_ARTIFACT_PERCENT
                && let Some(id) = random_artifact(content, &held_ids(state), None, rng)
            {
                gain_artifact(state, content, id);
            }
            RunPhase::Map
        }
        _ => {
            let open: Vec<String> = content
                .passives
                .iter()
                .filter(|p| p.kind == RunPassiveKind::Blessing && !state.blessings.contains(&p.id))
                .map(|p| p.id.clone())
                .collect();
            let offers: Vec<String> = rng.shuffle(&open).into_iter().take(RUN_SHRINE_OFFERS).collect();
            if offers.is_empty() {
                RunPhase::Map
            } else {
                RunPhase::Shrine { offers }
            }
        }
    };
}

/// What a won fight's `FightOver` reports, for the stats.
struct FightReport {
    hp: i32,
    damage_dealt: i32,
    healing_done: i32,
    cards_drawn: i32,
    turns: i32,
}

fn fight_won(state: &mut RunState, content: &RunContent, report: &FightReport, rng: &mut Rng) {
    let node = run_node(state).expect("a fight stands on a node").clone();
    let kind = node.kind;
    let hp = report.hp.clamp(1, run_max_hp(state, content));
    let s = &mut state.stats;
    s.damage_taken += (state.hp - hp).max(0);
    s.fights_won += 1;
    s.damage_dealt += report.damage_dealt;
    s.healing_done += report.healing_done;
    s.cards_drawn += report.cards_drawn;
    s.turns += report.turns;
    match kind {
        RunNodeKind::Elite => s.elites_won += 1,
        RunNodeKind::Ghost => s.ghosts_won += 1,
        RunNodeKind::MiniBoss => s.mini_bosses_won += 1,
        RunNodeKind::Boss => s.bosses_won += 1,
        _ => {}
    }
    state.hp = hp;
    let coins = RUN_COINS.of(kind)
        + passive_total(state, content, |e| match e {
            RunEffect::CoinsPerFight { n } => Some(n),
            _ => None,
        });
    let tickets = RUN_TICKETS.of(kind)
        + passive_total(state, content, |e| match e {
            RunEffect::TicketsPerFight { n } => Some(n),
            _ => None,
        });
    earn(state, coins, tickets);
    let artifact = match node.artifact {
        Some(id) => Some(id).filter(|id| !held_ids(state).contains(&id.as_str())),
        None if rng.int(PERCENT) < RUN_ARTIFACT_PERCENT.of(kind) => {
            random_artifact(content, &held_ids(state), None, rng)
        }
        None => None,
    };
    state.phase = RunPhase::Reward {
        artifact,
        packs: roll_packs(content, rng),
    };
}

/// After a fight's rewards: a boss's extraction, else the map.
fn after_rewards(state: &mut RunState) {
    state.phase = match run_node(state).map(|n| n.kind) {
        Some(RunNodeKind::MiniBoss) => RunPhase::ExtractCard,
        Some(RunNodeKind::Boss) => RunPhase::ExtractArtifact,
        _ => RunPhase::Map,
    };
}

fn apply(
    state: &mut RunState,
    content: &RunContent,
    action: &RunAction,
    rng: &mut Rng,
) -> Result<(), String> {
    if matches!(state.phase, RunPhase::Won | RunPhase::Dead) {
        return Err("the run is over".into());
    }
    if *action == RunAction::Abandon {
        state.phase = RunPhase::Dead;
        return Ok(());
    }
    match (state.phase.clone(), action) {
        (RunPhase::Map, RunAction::Go { col }) => {
            let row = state.row.map_or(0, |r| r + 1);
            if *col >= state.map.get(row).map_or(0, Vec::len) {
                return Err(format!("row {row} has no node {col}"));
            }
            state.row = Some(row);
            state.col = *col;
            enter(state, content, rng);
        }
        (
            RunPhase::Fight,
            RunAction::FightOver {
                won,
                hp,
                damage_dealt,
                healing_done,
                cards_drawn,
                turns,
            },
        ) => {
            if *won {
                let report = FightReport {
                    hp: *hp,
                    damage_dealt: *damage_dealt,
                    healing_done: *healing_done,
                    cards_drawn: *cards_drawn,
                    turns: *turns,
                };
                fight_won(state, content, &report, rng);
            } else {
                state.stats.damage_taken += state.hp;
                state.stats.damage_dealt += damage_dealt;
                state.stats.healing_done += healing_done;
                state.stats.cards_drawn += cards_drawn;
                state.stats.turns += turns;
                state.hp = 0;
                state.phase = RunPhase::Dead;
            }
        }
        (
            RunPhase::Reward {
                artifact: Some(id),
                packs,
            },
            RunAction::TakeArtifact { take },
        ) => {
            if *take {
                gain_artifact(state, content, id);
            }
            state.phase = RunPhase::Reward {
                artifact: None,
                packs,
            };
        }
        (
            RunPhase::Reward {
                artifact: Some(_), ..
            },
            RunAction::OpenPack { .. },
        ) => {
            return Err("take or leave the artifact first".into());
        }
        (
            RunPhase::Reward {
                artifact: None,
                packs,
            },
            RunAction::OpenPack { index },
        ) => {
            let kind = packs.get(*index).ok_or("no such pack")?;
            let cards = open_pack(state, content, kind, rng);
            let keep = keep_count(state, content, cards.len());
            state.phase = RunPhase::Pack { cards, keep };
        }
        (RunPhase::Pack { cards, keep }, RunAction::Keep { cards: kept }) => {
            let skip = passive_total(state, content, |e| match e {
                RunEffect::Skip { coins } => Some(coins),
                _ => None,
            });
            if kept.is_empty() && skip > 0 {
                earn(state, skip, 0);
                state.stats.packs_skipped += 1;
            } else {
                if kept.len() != keep {
                    return Err(format!("keep {keep} cards, not {}", kept.len()));
                }
                for (i, id) in kept.iter().enumerate() {
                    if !cards.contains(id) || kept[..i].contains(id) {
                        return Err(format!("{id} is not a card of the pack to keep"));
                    }
                }
                state.deck.extend(kept.iter().map(|id| RunCard {
                    id: id.clone(),
                    radiant: false,
                    buffs: 0,
                }));
                state.stats.cards_kept += i32::try_from(kept.len()).unwrap_or(0);
            }
            after_rewards(state);
        }
        (RunPhase::ExtractCard, RunAction::Extract { index }) => {
            if let Some(i) = index {
                if *i >= state.deck.len() || state.deck.len() == 1 {
                    return Err("no such card to extract".into());
                }
                let gone = state.deck.remove(*i);
                state.extracted_cards.push(gone.id);
            }
            state.phase = RunPhase::Map;
        }
        (RunPhase::ExtractArtifact, RunAction::Extract { index }) => {
            if let Some(i) = index {
                let id = state.artifacts.get(*i).ok_or("no such artifact")?.id.clone();
                if passive_def(content, &id).is_some_and(|p| p.unobtainable) {
                    return Err(format!("{id} is a boss's own and cannot be extracted"));
                }
                with_max_hp(state, content, |s| {
                    s.artifacts.remove(*i);
                });
                state.extracted_artifacts.push(id);
            }
            state.phase = RunPhase::Won;
        }
        (RunPhase::Shop { mut stock }, RunAction::Buy { index }) => {
            let item = stock.get_mut(*index).ok_or("no such item")?;
            if item.sold {
                return Err("sold already".into());
            }
            if let RunShopGood::Card { id } = &item.good
                && in_deck(state, id)
            {
                return Err(format!("{id} is in the deck already"));
            }
            spend_coins(state, discounted(state, content, item.price))?;
            item.sold = true;
            match item.good.clone() {
                RunShopGood::Card { id } => {
                    state.deck.push(RunCard {
                        id,
                        radiant: false,
                        buffs: 0,
                    });
                    state.stats.cards_bought += 1;
                }
                RunShopGood::Artifact { id } => gain_artifact(state, content, id),
            }
            state.phase = RunPhase::Shop { stock };
        }
        (RunPhase::Shop { .. }, RunAction::RemoveCard { index }) => {
            if *index >= state.deck.len() || state.deck.len() == 1 {
                return Err("no card to remove".into());
            }
            spend_coins(state, discounted(state, content, RUN_REMOVE_PRICE))?;
            state.deck.remove(*index);
            state.stats.cards_removed += 1;
        }
        (RunPhase::Shop { .. }, RunAction::MakeRadiant { index }) => {
            if state.deck.get(*index).is_some_and(|c| !c.radiant) {
                spend_tickets(state, RUN_RADIANT_TICKETS)?;
            }
            make_radiant(state, *index)?;
        }
        (RunPhase::Shop { .. }, RunAction::Buff { index }) => {
            if *index < state.deck.len() {
                spend_tickets(state, RUN_BUFF_TICKETS)?;
            }
            buff(state, *index)?;
        }
        (RunPhase::Shop { .. }, RunAction::LevelArtifact { index }) => {
            let level = state.artifacts.get(*index).ok_or("no such artifact")?.level;
            if level >= RUN_ARTIFACT_LEVEL_MAX {
                return Err("that artifact is at its highest level".into());
            }
            spend_tickets(state, RUN_LEVEL_TICKETS * level)?;
            with_max_hp(state, content, |s| s.artifacts[*index].level += 1);
            state.stats.artifacts_leveled += 1;
        }
        (RunPhase::Shop { .. }, RunAction::Leave) => state.phase = RunPhase::Map,
        (RunPhase::Campfire, RunAction::Heal) => {
            let max = run_max_hp(state, content);
            let healed = (max * RUN_CAMPFIRE_HEAL_PERCENT / PERCENT).min(max - state.hp);
            state.hp += healed;
            state.stats.healed += healed;
            state.phase = RunPhase::Map;
        }
        (RunPhase::Campfire, RunAction::MakeRadiant { index }) => {
            make_radiant(state, *index)?;
            state.phase = RunPhase::Map;
        }
        (RunPhase::Campfire, RunAction::Buff { index }) => {
            buff(state, *index)?;
            state.phase = RunPhase::Map;
        }
        (RunPhase::Campfire, RunAction::RemoveCards { indices }) => {
            let mut picked = distinct(indices.iter().copied());
            if picked.len() != RUN_CAMPFIRE_REMOVE || indices.len() != RUN_CAMPFIRE_REMOVE {
                return Err(format!("remove {RUN_CAMPFIRE_REMOVE} different cards"));
            }
            if picked.iter().any(|i| *i >= state.deck.len()) || state.deck.len() <= RUN_CAMPFIRE_REMOVE {
                return Err("no such cards to remove".into());
            }
            picked.sort_unstable_by(|a, b| b.cmp(a));
            for i in picked {
                state.deck.remove(i);
            }
            state.stats.cards_removed += i32::try_from(RUN_CAMPFIRE_REMOVE).unwrap_or(0);
            state.phase = RunPhase::Map;
        }
        (RunPhase::Event { id }, RunAction::Choose { index }) => {
            let def = content
                .events
                .iter()
                .find(|e| e.id == id)
                .ok_or("no such event")?;
            let option = def.options.get(*index).ok_or("no such option")?;
            if state.hp + option.hp < 1 {
                return Err("not enough HP".into());
            }
            if state.coins + option.coins < 0 || state.tickets + option.tickets < 0 {
                return Err("not enough to pay".into());
            }
            let before = state.hp;
            state.hp = (state.hp + option.hp).min(run_max_hp(state, content));
            state.stats.damage_taken += (before - state.hp).max(0);
            state.stats.healed += (state.hp - before).max(0);
            spend_coins(state, (-option.coins).max(0))?;
            spend_tickets(state, (-option.tickets).max(0))?;
            earn(state, option.coins.max(0), option.tickets.max(0));
            if option.card {
                let open: Vec<&RunPoolCard> = open_pool(content).filter(|c| !in_deck(state, &c.id)).collect();
                if let Some(card) = rng.pick(&open) {
                    state.deck.push(RunCard {
                        id: card.id.clone(),
                        radiant: false,
                        buffs: 0,
                    });
                }
            }
            if option.artifact
                && let Some(id) = random_artifact(content, &held_ids(state), None, rng)
            {
                gain_artifact(state, content, id);
            }
            state.phase = RunPhase::Map;
        }
        (RunPhase::Shrine { offers }, RunAction::Bless { index }) => {
            if let Some(i) = index {
                let id = offers.get(*i).ok_or("no such blessing")?.clone();
                with_max_hp(state, content, |s| s.blessings.push(id));
                state.stats.blessings_taken += 1;
            }
            state.phase = RunPhase::Map;
        }
        (phase, action) => return Err(format!("{action:?} is not an answer to {phase:?}")),
    }
    Ok(())
}
