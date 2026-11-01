//! The economy's store (SPEC §14.11; migration 0045): every holding of §14 beside its append-only
//! ledger, over Postgres (`pg` below) or the in-memory fake (`fake` below, whose tables are
//! `FakeTables::economy`). Also here: migration 0047's practice seeds (§14.8, #1303), no holding and no
//! ledger, whose claim burns in place (`economy_practice_seed_claim`); migration 0067's idempotency keys
//! (R2124000), written in the spend's transaction and swept after `IDEMPOTENCY_KEY_TTL_HOURS`; and
//! migration 0071's claim, kept with its seed until a pay or a refusal settles it, once (R2422000,
//! `economy_practice_seed_settle`).
//!
//! Same shape as `duos::store`: each store method is a `Tx` method `economy_<table>_<verb>` dispatching to
//! this file's `pg::<name>` or `fake::<name>`. A grant writes its holding and ledger row together, in the
//! caller's transaction; one the holding cannot take answers `None` or `false`, writing nothing (§14.1,
//! §14.5); one the schema's checks refuse is refused here first, as `StoreError::Other("economy: …")`.
//! Lists come in their key's byte order (`collate "C"`), zero counts included; ledgers oldest first.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use jackioh_engine::config::Difficulty;
use jackioh_engine::wire::PrintedRarity;
use jackioh_engine::{Action, LastBoardEntry, LeanPick, PlayerId};

use crate::config::SET_TRACK_FIRST_LEVEL;
use crate::cosmetics::CosmeticRarity;
use crate::db::store::{StoreError, StoreResult, Tx};
use crate::economy::practice::ClaimedResult;

// The literals

/// Why a Jlore row was written (§14.11; `jlore_grants.reason`): a game's, a quest's or a track
/// level's pay (§14.7–§14.9), a pack bought (§14.3), a dev account's own grant or clear (§14.10),
/// or the launch's reset (§14.12).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JloreReason {
    Game,
    Quest,
    Track,
    PackPurchase,
    Dev,
    /// Migration 0050's reset of every existing account's economy (R1310002); no server path
    /// writes it.
    Launch,
}

/// Why a row of every other economy ledger was written (§14.11): the Jlore reasons, and a pack
/// opened (§14.3), a token bar paying (§14.5), a token redeemed (§14.5), the Chase Shard bar
/// buying (§14.6) and the launch's reset (§14.12).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EconomyReason {
    Game,
    Quest,
    Track,
    PackPurchase,
    PackOpen,
    TokenBar,
    TokenRedeem,
    ChaseShard,
    Dev,
    /// Migration 0050's reset of every existing account's economy (R1310002), and migration
    /// 0057's grant of the portraits (R1827000); no server path writes it.
    Launch,
}

/// What an inventory row counts (§14.3, §14.5, §14.6). A set's card pack is keyed by its set, the
/// global card pack and a cosmetic pack by nothing (`""`), a card token by nothing (any new card) or
/// a rarity, and a cosmetic token by its rarity.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InventoryItem {
    SetPack,
    GlobalPack,
    CosmeticPack,
    CardToken,
    CosmeticToken,
}

/// One of an account's counters (§14.4–§14.6): a pity counter per rarity, each token bar's points,
/// the cosmetic token cycle's steps paid, and the Chase Shard bar's cosmetic packs opened.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Counter {
    PityEpic,
    PityLegendary,
    PityMythic,
    CardTokenBar,
    CosmeticTokenBar,
    CosmeticTokenCycle,
    ChaseShardBar,
}

impl Counter {
    /// A pity counter (§14.4), the only kind a pack key may name: its account keeps one over all
    /// its card packs (`""`) or one per kind of pack, as the pity math's sub-issue rules.
    pub fn is_pity(self) -> bool {
        matches!(
            self,
            Counter::PityEpic | Counter::PityLegendary | Counter::PityMythic
        )
    }
}

/// What befell a held quest (§14.9; `quest_grants.change`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuestChange {
    Arrived,
    Progressed,
    Completed,
    Removed,
}

fn literal<T: Serialize>(value: &T) -> Result<String, StoreError> {
    match serde_json::to_value(value)? {
        serde_json::Value::String(text) => Ok(text),
        other => Err(StoreError::Other(format!("economy: {other} is not a literal"))),
    }
}

fn from_literal<T: DeserializeOwned>(text: String) -> Result<T, StoreError> {
    Ok(serde_json::from_value(serde_json::Value::String(text))?)
}

// The rows

/// One row of `public.inventory`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InventoryEntry {
    pub item: InventoryItem,
    pub key: String,
    pub count: i64,
}

/// One row of `public.economy_counters`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CounterEntry {
    pub counter: Counter,
    /// `""` but on a pity counter kept per kind of pack.
    pub pack: String,
    pub value: i64,
}

/// One row of `public.set_tracks` (§14.7).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SetTrack {
    pub set_id: String,
    pub xp: i64,
    /// From `SET_TRACK_FIRST_LEVEL`.
    pub level: i64,
    /// How many levels have paid their rewards: from 0 to `level`.
    pub rewards_claimed: i64,
}

/// One row of `public.held_quests` (§14.9).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HeldQuest {
    pub quest_id: String,
    pub progress: i64,
    pub arrived_at: i64,
}

/// An account's quests: the held ones, oldest first, and when the next is due (`None`: none is).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuestBook {
    pub held: Vec<HeldQuest>,
    pub next_due_at: Option<i64>,
}

/// One row of `public.card_cosmetics` (§14.2): a per-card cosmetic owned on one card.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CardCosmetic {
    pub card_id: String,
    pub cosmetic_id: String,
}

/// An account's cosmetics (§14.2): per card, and the account cosmetics by id. The starter
/// cosmetics, owned by every account, have no row.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OwnedCosmetics {
    pub cards: Vec<CardCosmetic>,
    pub account: Vec<String>,
}

/// R2655000: a card's own full-art pick: `false` (no full art), `true` (its Epic full art, as every
/// pick before #2655 named it) or a full art's registry id, which travels as the id's text. An id
/// is kept as written and judged where it is worn, as a foil's is (`cosmetics::full_art_picked`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum FullArtPick {
    On(bool),
    Id(String),
}

impl From<bool> for FullArtPick {
    fn from(on: bool) -> FullArtPick {
        FullArtPick::On(on)
    }
}

impl FullArtPick {
    /// Whether the pick prints a full art at all: `true` or any id.
    pub fn prints(&self) -> bool {
        !matches!(self, FullArtPick::On(false))
    }

    /// The id the pick names, if it names one rather than a switch.
    pub fn id(&self) -> Option<&str> {
        match self {
            FullArtPick::Id(id) => Some(id),
            FullArtPick::On(_) => None,
        }
    }

    /// The pick a row of `public.card_cosmetic_picks` holds: its `full_art` switch, and the id
    /// migration 0072 keeps beside a switch that is on.
    pub fn of_row(full_art: Option<bool>, id: Option<String>) -> Option<FullArtPick> {
        match (full_art, id) {
            (Some(true), Some(id)) => Some(FullArtPick::Id(id)),
            (Some(on), _) => Some(FullArtPick::On(on)),
            (None, _) => None,
        }
    }
}

/// R1834000: one row of `public.card_cosmetic_picks` (migration 0061): what one card of the account
/// wears of its own: a foil, a finish, a border and its full-art pick ([`FullArtPick`], R2655000). A kind
/// with no pick is absent and wears its default (R1953001; the Full-art cards setting, for `full_art`). An
/// id is kept as written, an unknown one too, and judged only where it is worn (`loadout::freeze_loadout`,
/// R1834002).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CardPicks {
    pub card_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foil: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_art: Option<FullArtPick>,
}

impl CardPicks {
    /// Whether the card has no pick of its own at all, which the store keeps as no row.
    pub fn is_empty(&self) -> bool {
        self.foil.is_none() && self.finish.is_none() && self.border.is_none() && self.full_art.is_none()
    }
}

/// R1834005: how many people own a cosmetic, on one card for a per-card one (`card_id`) or on the
/// account (`None`), and when one was first granted it (epoch ms; `None` when no grant row shows
/// it, as for a portrait migration 0057 handed out). A ladder bot's profile is never counted.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CosmeticOwners {
    pub cosmetic_id: String,
    pub card_id: Option<String>,
    pub owners: i64,
    pub first_granted_at: Option<i64>,
}

/// The human's deck of a practice game (§9.9), as its seed was issued for it: a list of card ids
/// (a preset's or a saved deck's), or the random deck the game deals from its seed, leaning on
/// `lean` (R1372).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PracticeDeck {
    Cards { cards: Vec<String> },
    Random { lean: Option<LeanPick> },
}

/// A practice game's setup, as the seed was issued for it (#1303): what a claim's log is folded
/// from, with the seed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PracticeSetup {
    /// The AI's tier, whose handicap the AI's seat plays under (R180).
    pub difficulty: Difficulty,
    /// The seat the human began in; the AI began in the other (R677).
    pub human_seat: PlayerId,
    pub deck: PracticeDeck,
    /// The human's last board, the server's (R1303003); empty for none.
    pub last_board: Vec<LastBoardEntry>,
    /// The catalog the game is played on (R767000).
    pub catalog_version: String,
}

/// A practice claim as it was sent (R2422000; `practice_seeds.claim`, migration 0071): the result
/// it says the human had and the game's log, kept with its seed from the burn until it settles.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PracticeClaim {
    pub result: ClaimedResult,
    pub log: Vec<Action>,
}

/// How a practice claim settled (R2422000; `practice_seeds.settled`): paid, or refused by its check.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PracticeOutcome {
    Paid,
    Refused,
}

/// A practice claim's mark (R2422000; `practice_seeds.settled` and `settled_at`): how and when it
/// settled.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PracticeSettled {
    pub outcome: PracticeOutcome,
    pub at: i64,
}

/// One row of `public.practice_seeds` (migration 0047): a seed issued for a practice game, and
/// when it was claimed (R1303001); with migration 0071, the claim kept until it settles and the
/// mark it settled with (R2422000).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PracticeSeed {
    pub seed: String,
    pub profile_id: String,
    pub setup: PracticeSetup,
    pub issued_at: i64,
    pub claimed_at: Option<i64>,
    /// The claim that burned the seed, until it settles; `None` before the burn, after the
    /// settle, and for a seed claimed before migration 0071.
    #[serde(default)]
    pub claim: Option<PracticeClaim>,
    /// The claim's mark, once it settled.
    #[serde(default)]
    pub settled: Option<PracticeSettled>,
}

/// One row of `public.idempotency_keys` (migration 0067, R2124000): a spend's key under its
/// account, the route and the keyed hash of the body it came with, and the answer it committed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IdempotencyRow {
    pub profile_id: String,
    /// A UUID, hyphenated and lower case.
    pub key: String,
    pub route: String,
    /// 64 lower-case hex digits.
    pub body_hash: String,
    /// A 2xx status.
    pub status: i64,
    pub body: serde_json::Value,
    pub created_at: i64,
}

// The ledgers' rows. Each says why (`reason`), what it was for (`reference`, the `ref` column), and
// whether a dev tool wrote it (`dev`, §14.10).

/// One row of `public.jlore_grants`: `delta` Jlore.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JloreGrant {
    pub profile_id: String,
    pub delta: i64,
    pub reason: JloreReason,
    pub reference: Option<String>,
    pub dev: bool,
    pub at: i64,
}

/// What a profile's `game` Jlore rows over one stretch of time add up to (R1483000), for the
/// daily cap: the whole stretch, and the share of it that one reference (a match) wrote.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameJloreTotals {
    pub day: i64,
    pub reference: i64,
}

/// One row of `public.inventory_grants`: `delta` of an item under its key.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InventoryGrant {
    pub profile_id: String,
    pub item: InventoryItem,
    pub key: String,
    pub delta: i64,
    pub reason: EconomyReason,
    pub reference: Option<String>,
    pub dev: bool,
    pub at: i64,
}

/// One row of `public.economy_counter_grants`: `delta` on a counter.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CounterGrant {
    pub profile_id: String,
    pub counter: Counter,
    pub pack: String,
    pub delta: i64,
    pub reason: EconomyReason,
    pub reference: Option<String>,
    pub dev: bool,
    pub at: i64,
}

/// One row of `public.set_track_grants`: the XP, levels and rewards paid a track gained, each
/// signed, at least one not zero.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrackGrant {
    pub profile_id: String,
    pub set_id: String,
    pub xp: i64,
    pub levels: i64,
    pub rewards: i64,
    pub reason: EconomyReason,
    pub reference: Option<String>,
    pub dev: bool,
    pub at: i64,
}

/// One row of `public.quest_grants`: a quest's change, with the progress it made when it
/// progressed (and 0 otherwise).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuestGrant {
    pub profile_id: String,
    pub quest_id: String,
    pub change: QuestChange,
    pub progress: i64,
    pub reason: EconomyReason,
    pub reference: Option<String>,
    pub dev: bool,
    pub at: i64,
}

/// One row of `public.cosmetic_grants`: a cosmetic owned (+1) or cleared (−1), on `card_id` or, when
/// that is `None`, on the account.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CosmeticGrant {
    pub profile_id: String,
    pub card_id: Option<String>,
    pub cosmetic_id: String,
    pub delta: i64,
    pub reason: EconomyReason,
    pub reference: Option<String>,
    pub dev: bool,
    pub at: i64,
}

/// An account's whole economy ledger, each table's rows oldest first.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EconomyLedger {
    pub jlore: Vec<JloreGrant>,
    pub inventory: Vec<InventoryGrant>,
    pub counters: Vec<CounterGrant>,
    pub tracks: Vec<TrackGrant>,
    pub quests: Vec<QuestGrant>,
    pub cosmetics: Vec<CosmeticGrant>,
}

/// The fake's tables (`FakeTables::economy`): each state table as `(profile_id, row)` pairs, each
/// ledger in the order it was written.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EconomyTables {
    pub wallets: Vec<(String, i64)>,
    pub inventory: Vec<(String, InventoryEntry)>,
    pub counters: Vec<(String, CounterEntry)>,
    pub tracks: Vec<(String, SetTrack)>,
    pub held_quests: Vec<(String, HeldQuest)>,
    pub quest_schedules: Vec<(String, Option<i64>)>,
    pub card_cosmetics: Vec<(String, CardCosmetic)>,
    pub account_cosmetics: Vec<(String, String)>,
    pub jlore_grants: Vec<JloreGrant>,
    pub inventory_grants: Vec<InventoryGrant>,
    pub counter_grants: Vec<CounterGrant>,
    pub track_grants: Vec<TrackGrant>,
    pub quest_grants: Vec<QuestGrant>,
    pub cosmetic_grants: Vec<CosmeticGrant>,
    /// Migration 0047's seeds, in the order they were issued.
    pub practice_seeds: Vec<PracticeSeed>,
    /// R1834000: migration 0061's per-card picks, one per profile and card.
    pub card_picks: Vec<(String, CardPicks)>,
    /// R2124000: migration 0067's idempotency keys, one per profile and key.
    pub idempotency_keys: Vec<IdempotencyRow>,
}

// What both stores refuse alike: migration 0045's checks

fn refused(why: String) -> StoreError {
    StoreError::Other(format!("economy: {why}"))
}

/// `<ledger>_dev_check`: a `dev` reason is always marked `dev`.
fn check_dev(is_dev_reason: bool, dev: bool) -> Result<(), StoreError> {
    if is_dev_reason && !dev {
        return Err(refused("a dev reason needs the dev mark (§14.10)".to_string()));
    }
    Ok(())
}

/// `delta <> 0`: a ledger row records a change.
fn check_delta(delta: i64) -> Result<(), StoreError> {
    if delta == 0 {
        return Err(refused("a grant of 0 changes nothing".to_string()));
    }
    Ok(())
}

/// `inventory_key_check`: the key each item takes. A card token's rarity key is a card's, a
/// cosmetic token's a cosmetic's (R1333000); both scales spell their rarities alike.
fn check_inventory_key(item: InventoryItem, key: &str) -> Result<(), StoreError> {
    let fits = match item {
        InventoryItem::SetPack => !key.is_empty(),
        InventoryItem::GlobalPack | InventoryItem::CosmeticPack => key.is_empty(),
        InventoryItem::CardToken => key.is_empty() || key.parse::<PrintedRarity>().is_ok(),
        InventoryItem::CosmeticToken => CosmeticRarity::named(key).is_some(),
    };
    if !fits {
        return Err(refused(format!("{item:?} takes no key {key:?}")));
    }
    Ok(())
}

fn check_jlore(grant: &JloreGrant) -> Result<(), StoreError> {
    check_delta(grant.delta)?;
    check_dev(grant.reason == JloreReason::Dev, grant.dev)
}

fn check_inventory(grant: &InventoryGrant) -> Result<(), StoreError> {
    check_delta(grant.delta)?;
    check_dev(grant.reason == EconomyReason::Dev, grant.dev)?;
    check_inventory_key(grant.item, &grant.key)
}

fn check_counter(grant: &CounterGrant) -> Result<(), StoreError> {
    check_delta(grant.delta)?;
    check_dev(grant.reason == EconomyReason::Dev, grant.dev)?;
    // `economy_counters_pack_check`.
    if !grant.counter.is_pity() && !grant.pack.is_empty() {
        return Err(refused(format!(
            "{:?} is kept per account, not per pack {:?}",
            grant.counter, grant.pack
        )));
    }
    Ok(())
}

fn check_track(grant: &TrackGrant) -> Result<(), StoreError> {
    // `set_track_grants_change_check`.
    if grant.xp == 0 && grant.levels == 0 && grant.rewards == 0 {
        return Err(refused("a track grant of nothing changes nothing".to_string()));
    }
    check_dev(grant.reason == EconomyReason::Dev, grant.dev)
}

fn check_quest(grant: &QuestGrant) -> Result<(), StoreError> {
    // `quest_grants_progress_check`: progress is made by progressing, and only so.
    if (grant.change == QuestChange::Progressed) != (grant.progress != 0) {
        return Err(refused(format!(
            "a quest {:?} with progress {}",
            grant.change, grant.progress
        )));
    }
    check_dev(grant.reason == EconomyReason::Dev, grant.dev)
}

fn check_cosmetic(grant: &CosmeticGrant) -> Result<(), StoreError> {
    // `cosmetic_grants.delta in (-1, 1)`: a cosmetic is owned or it is not.
    if grant.delta != 1 && grant.delta != -1 {
        return Err(refused(format!(
            "a cosmetic grant of {}, not 1 or -1",
            grant.delta
        )));
    }
    check_dev(grant.reason == EconomyReason::Dev, grant.dev)
}

/// Migration 0067's `status between 200 and 299`: only a success is kept.
const IDEMPOTENCY_STATUSES: std::ops::RangeInclusive<i64> = 200..=299;

/// Migration 0067's `body_hash ~ '^[0-9a-f]{64}$'`: a SHA-256 in hex.
const IDEMPOTENCY_BODY_HASH_DIGITS: usize = 64;

fn check_idempotency(row: &IdempotencyRow) -> Result<(), StoreError> {
    if !IDEMPOTENCY_STATUSES.contains(&row.status) {
        return Err(refused(format!(
            "an idempotency key keeps a 2xx answer, not {}",
            row.status
        )));
    }
    let hex = row.body_hash.len() == IDEMPOTENCY_BODY_HASH_DIGITS
        && row
            .body_hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !hex {
        return Err(refused(format!(
            "an idempotency key's body hash is 64 lower-case hex digits, not {:?}",
            row.body_hash
        )));
    }
    Ok(())
}

/// The track `grant` leaves, from `held` (a track not yet written is at its first level with no XP
/// and nothing paid): `None` when that track is no track (XP below 0, a level below the first, or
/// rewards paid below 0 or past the level reached), or a sum overflows.
fn track_after(held: Option<&SetTrack>, grant: &TrackGrant) -> Option<SetTrack> {
    let (xp, level, rewards_claimed) = match held {
        Some(track) => (track.xp, track.level, track.rewards_claimed),
        None => (0, SET_TRACK_FIRST_LEVEL, 0),
    };
    let next = SetTrack {
        set_id: grant.set_id.clone(),
        xp: xp.checked_add(grant.xp)?,
        level: level.checked_add(grant.levels)?,
        rewards_claimed: rewards_claimed.checked_add(grant.rewards)?,
    };
    let holds = next.xp >= 0
        && next.level >= SET_TRACK_FIRST_LEVEL
        && (0..=next.level).contains(&next.rewards_claimed);
    holds.then_some(next)
}

/// A count after `delta`: `None` below 0, or on overflow.
fn count_after(held: Option<i64>, delta: i64) -> Option<i64> {
    held.unwrap_or(0).checked_add(delta).filter(|count| *count >= 0)
}

// The store methods

macro_rules! economy_dispatch {
    ($self:ident, $name:ident ( $($arg:expr),* $(,)? )) => {
        match $self {
            Tx::Pg(t) => pg::$name(t, $($arg),*).await,
            Tx::Fake(f) => fake::$name(f, $($arg),*),
        }
    };
}

impl Tx<'_> {
    /// §14.1: the account's Jlore; 0 before its first grant.
    pub async fn economy_jlore_get(&mut self, profile_id: &str) -> StoreResult<i64> {
        economy_dispatch!(self, jlore_get(profile_id))
    }

    /// §14.1: adds `delta` Jlore and writes its ledger row. The new balance, or `None` for a spend
    /// the wallet cannot cover, which writes nothing.
    pub async fn economy_jlore_grant(&mut self, grant: &JloreGrant) -> StoreResult<Option<i64>> {
        check_jlore(grant)?;
        economy_dispatch!(self, jlore_grant(grant))
    }

    /// R1483000: the Jlore the profile's `game` rows add up to with `at` in `[from, until)`
    /// (milliseconds), and the share of it written under `reference`. Read under the account's
    /// economy lock, so the sum is the one a payout adds to.
    pub async fn economy_game_jlore_totals(
        &mut self,
        profile_id: &str,
        from: i64,
        until: i64,
        reference: &str,
    ) -> StoreResult<GameJloreTotals> {
        economy_dispatch!(self, game_jlore_totals(profile_id, from, until, reference))
    }

    /// §14.3, §14.5, §14.6: the account's packs and tokens, by item and key.
    pub async fn economy_inventory_get(&mut self, profile_id: &str) -> StoreResult<Vec<InventoryEntry>> {
        economy_dispatch!(self, inventory_get(profile_id))
    }

    /// Adds `delta` of an item and writes its ledger row. The new count, or `None` when the account
    /// holds fewer than it spends, which writes nothing.
    pub async fn economy_inventory_grant(&mut self, grant: &InventoryGrant) -> StoreResult<Option<i64>> {
        check_inventory(grant)?;
        economy_dispatch!(self, inventory_grant(grant))
    }

    /// §14.4–§14.6: the account's counters, by counter and pack.
    pub async fn economy_counters_get(&mut self, profile_id: &str) -> StoreResult<Vec<CounterEntry>> {
        economy_dispatch!(self, counters_get(profile_id))
    }

    /// Adds `delta` to a counter and writes its ledger row. The new value, or `None` below 0, which
    /// writes nothing.
    pub async fn economy_counters_grant(&mut self, grant: &CounterGrant) -> StoreResult<Option<i64>> {
        check_counter(grant)?;
        economy_dispatch!(self, counters_grant(grant))
    }

    /// §14.7: the account's set tracks, by set; a track not yet written has no row.
    pub async fn economy_tracks_get(&mut self, profile_id: &str) -> StoreResult<Vec<SetTrack>> {
        economy_dispatch!(self, tracks_get(profile_id))
    }

    /// Adds a grant's XP, levels and rewards paid to a track and writes its ledger row. The track
    /// after it, or `None` when that would be no track (see `track_after`), which writes nothing.
    pub async fn economy_tracks_grant(&mut self, grant: &TrackGrant) -> StoreResult<Option<SetTrack>> {
        check_track(grant)?;
        economy_dispatch!(self, tracks_grant(grant))
    }

    /// §14.9: the held quests, by when they arrived (then id), and when the next is due. The
    /// schedule's row stays locked to the end of the transaction, so a second read of the same
    /// account waits for it: a top-up reads, then writes what it read was due (R1304000).
    pub async fn economy_quests_get(&mut self, profile_id: &str) -> StoreResult<QuestBook> {
        economy_dispatch!(self, quests_get(profile_id))
    }

    /// Applies a quest's change and writes its ledger row: `Arrived` holds it (progress 0, arrived
    /// at the grant's `at`), `Progressed` adds its progress, `Completed` and `Removed` let it go.
    /// False, writing nothing, when an arrival is already held, or another change is not held or
    /// would bring the progress below 0.
    pub async fn economy_quests_change(&mut self, grant: &QuestGrant) -> StoreResult<bool> {
        check_quest(grant)?;
        economy_dispatch!(self, quests_change(grant))
    }

    /// §14.9: when the next quest is due (`None`: none is). A schedule, not a holding: no ledger row.
    pub async fn economy_quests_set_next_due(
        &mut self,
        profile_id: &str,
        next_due_at: Option<i64>,
    ) -> StoreResult<()> {
        economy_dispatch!(self, quests_set_next_due(profile_id, next_due_at))
    }

    /// R2210000: how many of the profile's quest rows are a `Removed` change with the quest reason
    /// and `at` in `[from, until)` (milliseconds): the quests it replaced then. A replace reads it
    /// under the account's economy lock.
    pub async fn economy_quests_removed_count(
        &mut self,
        profile_id: &str,
        from: i64,
        until: i64,
    ) -> StoreResult<i64> {
        economy_dispatch!(self, quests_removed_count(profile_id, from, until))
    }

    /// §14.2: the cosmetics the account owns, per card (by card, then cosmetic) and on the account.
    pub async fn economy_cosmetics_get(&mut self, profile_id: &str) -> StoreResult<OwnedCosmetics> {
        economy_dispatch!(self, cosmetics_get(profile_id))
    }

    /// Owns (+1) or clears (−1) a cosmetic, on a card or on the account, and writes its ledger row.
    /// False, writing nothing, when it is already owned or, for a clear, not owned.
    pub async fn economy_cosmetics_grant(&mut self, grant: &CosmeticGrant) -> StoreResult<bool> {
        check_cosmetic(grant)?;
        economy_dispatch!(self, cosmetics_grant(grant))
    }

    /// R1834000: the account's per-card picks, by card.
    pub async fn economy_card_picks_get(&mut self, profile_id: &str) -> StoreResult<Vec<CardPicks>> {
        economy_dispatch!(self, card_picks_get(profile_id))
    }

    /// R1834000: writes one card's picks whole, replacing what it held; picks with nothing in them
    /// delete its row. A current-state table with no ledger: a pick is no holding (§14.11).
    pub async fn economy_card_picks_put(
        &mut self,
        profile_id: &str,
        picks: &CardPicks,
        at: i64,
    ) -> StoreResult<()> {
        economy_dispatch!(self, card_picks_put(profile_id, picks, at))
    }

    /// R1834000: a device's one-time upload: writes each card's picks the account holds no row for
    /// and never touches one it holds, an empty entry writing nothing. Answers how many it wrote.
    pub async fn economy_card_picks_upload(
        &mut self,
        profile_id: &str,
        picks: &[CardPicks],
        at: i64,
    ) -> StoreResult<u64> {
        economy_dispatch!(self, card_picks_upload(profile_id, picks, at))
    }

    /// R1834005: every cosmetic some person owns, by cosmetic then card. Read by the ladder bots'
    /// wardrobes, never by a client.
    pub async fn economy_cosmetic_owners(&mut self) -> StoreResult<Vec<CosmeticOwners>> {
        economy_dispatch!(self, cosmetic_owners())
    }

    /// §14.11: every economy ledger row of the account, each table's oldest first.
    pub async fn economy_ledger_get(&mut self, profile_id: &str) -> StoreResult<EconomyLedger> {
        economy_dispatch!(self, ledger_get(profile_id))
    }

    pub async fn economy_practice_seed_insert(&mut self, seed: &PracticeSeed) -> StoreResult<()> {
        economy_dispatch!(self, practice_seed_insert(seed))
    }

    /// The profile's seed `seed`, or `None` when it has none by that name (another profile's
    /// included).
    pub async fn economy_practice_seed_get(
        &mut self,
        profile_id: &str,
        seed: &str,
    ) -> StoreResult<Option<PracticeSeed>> {
        economy_dispatch!(self, practice_seed_get(profile_id, seed))
    }

    /// R1303001: marks the profile's seed claimed at `at`, keeping `claim` with it until it settles
    /// (R2422000). False, writing nothing, when it has no such seed or it was claimed already, so
    /// two claims racing for one seed burn it once.
    pub async fn economy_practice_seed_claim(
        &mut self,
        profile_id: &str,
        seed: &str,
        at: i64,
        claim: &PracticeClaim,
    ) -> StoreResult<bool> {
        economy_dispatch!(self, practice_seed_claim(profile_id, seed, at, claim))
    }

    /// R2422000: marks the profile's claimed seed settled with `outcome` at `at`, and clears the
    /// claim it kept. False, writing nothing, when the profile has no such seed, it is unclaimed,
    /// or it settled already, so a pay that marks it first in its own transaction lands once
    /// whoever runs it.
    pub async fn economy_practice_seed_settle(
        &mut self,
        profile_id: &str,
        seed: &str,
        outcome: PracticeOutcome,
        at: i64,
    ) -> StoreResult<bool> {
        economy_dispatch!(self, practice_seed_settle(profile_id, seed, outcome, at))
    }

    /// R2422000: every profile's seeds that keep a claim not yet settled, claimed before
    /// `claimed_before`, oldest claim first (then by seed): what the retention loop settles.
    pub async fn economy_practice_seeds_unsettled(
        &mut self,
        claimed_before: i64,
    ) -> StoreResult<Vec<PracticeSeed>> {
        economy_dispatch!(self, practice_seeds_unsettled(claimed_before))
    }

    /// R2379000: deletes the profile's seed `seed` while it is unclaimed. False, writing nothing,
    /// when the profile has no such seed (another profile's included) or it was claimed.
    pub async fn economy_practice_seed_release(&mut self, profile_id: &str, seed: &str) -> StoreResult<bool> {
        economy_dispatch!(self, practice_seed_release(profile_id, seed))
    }

    /// R1483000: how many of the profile's seeds are unclaimed and issued at or after
    /// `issued_since`, the ones `economy_practice_seeds_prune` keeps.
    pub async fn economy_practice_seeds_open(
        &mut self,
        profile_id: &str,
        issued_since: i64,
    ) -> StoreResult<i64> {
        economy_dispatch!(self, practice_seeds_open(profile_id, issued_since))
    }

    /// R1303001: deletes the profile's seeds issued before `issued_before`, claimed or not, but a
    /// claim not yet settled, which stays until it settles (R2422000).
    pub async fn economy_practice_seeds_prune(
        &mut self,
        profile_id: &str,
        issued_before: i64,
    ) -> StoreResult<()> {
        economy_dispatch!(self, practice_seeds_prune(profile_id, issued_before))
    }

    /// R2124000: the profile's row for `key`, or `None` when it sent no spend with that key (another
    /// profile's key included).
    pub async fn economy_idempotency_get(
        &mut self,
        profile_id: &str,
        key: &str,
    ) -> StoreResult<Option<IdempotencyRow>> {
        economy_dispatch!(self, idempotency_get(profile_id, key))
    }

    /// R2124000: writes a spend's key and its answer, in the spend's transaction. A key the profile
    /// already holds is refused, as the primary key refuses it.
    pub async fn economy_idempotency_put(&mut self, row: &IdempotencyRow) -> StoreResult<()> {
        check_idempotency(row)?;
        economy_dispatch!(self, idempotency_put(row))
    }

    /// R2124000: deletes every profile's keys written before `created_before`; answers how many.
    pub async fn economy_idempotency_sweep(&mut self, created_before: i64) -> StoreResult<u64> {
        economy_dispatch!(self, idempotency_sweep(created_before))
    }

    /// R1301003: a lock on the account's economy until the transaction ends, so two pack openings
    /// at once run one after the other and neither rolls on counters the other is moving. A lock,
    /// not a holding: no ledger row.
    pub async fn economy_lock_account(&mut self, profile_id: &str) -> StoreResult<()> {
        economy_dispatch!(self, lock_account(profile_id))
    }
}

/// Migration 0045's `on delete cascade`, in the fake (`db::fake::remove_profile_rows`): every row of
/// a deleted profile goes, its ledgers' with it (0012's rule).
pub fn remove_economy_rows(tables: &mut EconomyTables, profile_id: &str) {
    tables.wallets.retain(|(owner, _)| owner != profile_id);
    tables.inventory.retain(|(owner, _)| owner != profile_id);
    tables.counters.retain(|(owner, _)| owner != profile_id);
    tables.tracks.retain(|(owner, _)| owner != profile_id);
    tables.held_quests.retain(|(owner, _)| owner != profile_id);
    tables.quest_schedules.retain(|(owner, _)| owner != profile_id);
    tables.card_cosmetics.retain(|(owner, _)| owner != profile_id);
    tables.account_cosmetics.retain(|(owner, _)| owner != profile_id);
    tables.jlore_grants.retain(|grant| grant.profile_id != profile_id);
    tables
        .inventory_grants
        .retain(|grant| grant.profile_id != profile_id);
    tables
        .counter_grants
        .retain(|grant| grant.profile_id != profile_id);
    tables.track_grants.retain(|grant| grant.profile_id != profile_id);
    tables.quest_grants.retain(|grant| grant.profile_id != profile_id);
    tables
        .cosmetic_grants
        .retain(|grant| grant.profile_id != profile_id);
    tables.practice_seeds.retain(|seed| seed.profile_id != profile_id);
    tables.card_picks.retain(|(owner, _)| owner != profile_id);
    tables.idempotency_keys.retain(|row| row.profile_id != profile_id);
}

// Postgres (migration 0045)

mod pg {
    use sqlx::Row;
    use sqlx::postgres::PgRow;
    use time::OffsetDateTime;

    use super::{
        CardCosmetic, CardPicks, CosmeticGrant, CosmeticOwners, CounterEntry, CounterGrant, EconomyLedger,
        EconomyReason, FullArtPick, GameJloreTotals, HeldQuest, IdempotencyRow, InventoryEntry,
        InventoryGrant, JloreGrant, JloreReason, OwnedCosmetics, PracticeClaim, PracticeOutcome,
        PracticeSeed, PracticeSettled, QuestBook, QuestChange, QuestGrant, SetTrack, TrackGrant, count_after,
        from_literal, literal, track_after,
    };
    use crate::auth::is_uuid;
    use crate::config::SET_TRACK_FIRST_LEVEL;
    // `db::pg`'s session and row helpers: every method re-stamps its subject first (`run_as`), so it
    // runs as `service_role` with `request.jwt.claim.sub` the profile the call is about.
    use crate::db::pg::{PgTx, ms_of, run_as};
    use crate::db::store::StoreError;

    fn at_of(row: &PgRow) -> Result<i64, StoreError> {
        Ok(ms_of(row.try_get("at")?))
    }

    pub async fn jlore_get(t: &mut PgTx<'_>, profile_id: &str) -> Result<i64, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let held: Option<i64> =
            sqlx::query_scalar("select jlore from public.wallets where profile_id = $1::uuid")
                .bind(profile_id)
                .fetch_optional(&mut **t)
                .await?;
        Ok(held.unwrap_or(0))
    }

    /// Three statements: the balance, locked; the refusal, decided here; then the wallet and the
    /// ledger row. An absent wallet is inserted rather than upserted over with a spend, since
    /// Postgres checks the proposed row (`jlore >= 0`) before the conflict.
    pub async fn jlore_grant(t: &mut PgTx<'_>, grant: &JloreGrant) -> Result<Option<i64>, StoreError> {
        run_as(t, Some(grant.profile_id.as_str())).await?;
        let held: Option<i64> =
            sqlx::query_scalar("select jlore from public.wallets where profile_id = $1::uuid for update")
                .bind(grant.profile_id.as_str())
                .fetch_optional(&mut **t)
                .await?;
        if count_after(held, grant.delta).is_none() {
            return Ok(None);
        }
        let jlore: i64 = if held.is_some() {
            sqlx::query_scalar(
                "update public.wallets set jlore = jlore + $2::bigint
                  where profile_id = $1::uuid
              returning jlore",
            )
        } else {
            sqlx::query_scalar(
                "insert into public.wallets (profile_id, jlore) values ($1::uuid, $2::bigint)
                 on conflict (profile_id) do update set jlore = public.wallets.jlore + $2::bigint
                 returning jlore",
            )
        }
        .bind(grant.profile_id.as_str())
        .bind(grant.delta)
        .fetch_one(&mut **t)
        .await?;
        sqlx::query(
            "insert into public.jlore_grants (profile_id, delta, reason, ref, dev, at)
             values ($1::uuid, $2::bigint, $3::text, $4::text, $5::boolean,
                     to_timestamp($6::double precision / 1000.0))",
        )
        .bind(grant.profile_id.as_str())
        .bind(grant.delta)
        .bind(literal(&grant.reason)?)
        .bind(grant.reference.as_deref())
        .bind(grant.dev)
        .bind(grant.at)
        .execute(&mut **t)
        .await?;
        Ok(Some(jlore))
    }

    /// One aggregate over the profile's rows (`jlore_grants_profile_idx` leads it).
    pub async fn game_jlore_totals(
        t: &mut PgTx<'_>,
        profile_id: &str,
        from: i64,
        until: i64,
        reference: &str,
    ) -> Result<GameJloreTotals, StoreError> {
        // A deleted seat reads back as the empty id (`db/pg.rs`, KNOWN DIVERGENCES, deleted
        // accounts), which has no rows and which Postgres would refuse as a uuid.
        if !is_uuid(profile_id) {
            return Ok(GameJloreTotals::default());
        }
        run_as(t, Some(profile_id)).await?;
        let (day, reference): (i64, i64) = sqlx::query_as(
            "select coalesce(sum(delta), 0)::bigint,
                    coalesce(sum(delta) filter (where ref = $5::text), 0)::bigint
               from public.jlore_grants
              where profile_id = $1::uuid and reason = $2::text
                and at >= to_timestamp($3::double precision / 1000.0)
                and at < to_timestamp($4::double precision / 1000.0)",
        )
        .bind(profile_id)
        .bind(literal(&JloreReason::Game)?)
        .bind(from)
        .bind(until)
        .bind(reference)
        .fetch_one(&mut **t)
        .await?;
        Ok(GameJloreTotals { day, reference })
    }

    pub async fn inventory_get(
        t: &mut PgTx<'_>,
        profile_id: &str,
    ) -> Result<Vec<InventoryEntry>, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let rows = sqlx::query(
            r#"select item, key, count from public.inventory
                where profile_id = $1::uuid
                order by item collate "C", key collate "C""#,
        )
        .bind(profile_id)
        .fetch_all(&mut **t)
        .await?;
        rows.iter()
            .map(|row| -> Result<InventoryEntry, StoreError> {
                Ok(InventoryEntry {
                    item: from_literal(row.try_get("item")?)?,
                    key: row.try_get("key")?,
                    count: row.try_get("count")?,
                })
            })
            .collect()
    }

    /// `jlore_grant`'s three statements, over `inventory`.
    pub async fn inventory_grant(
        t: &mut PgTx<'_>,
        grant: &InventoryGrant,
    ) -> Result<Option<i64>, StoreError> {
        run_as(t, Some(grant.profile_id.as_str())).await?;
        let item = literal(&grant.item)?;
        let held: Option<i64> = sqlx::query_scalar(
            "select count from public.inventory
              where profile_id = $1::uuid and item = $2::text and key = $3::text
                for update",
        )
        .bind(grant.profile_id.as_str())
        .bind(item.as_str())
        .bind(grant.key.as_str())
        .fetch_optional(&mut **t)
        .await?;
        if count_after(held, grant.delta).is_none() {
            return Ok(None);
        }
        let count: i64 = if held.is_some() {
            sqlx::query_scalar(
                "update public.inventory set count = count + $4::bigint
                  where profile_id = $1::uuid and item = $2::text and key = $3::text
              returning count",
            )
        } else {
            sqlx::query_scalar(
                "insert into public.inventory (profile_id, item, key, count)
                 values ($1::uuid, $2::text, $3::text, $4::bigint)
                 on conflict (profile_id, item, key)
                 do update set count = public.inventory.count + $4::bigint
                 returning count",
            )
        }
        .bind(grant.profile_id.as_str())
        .bind(item.as_str())
        .bind(grant.key.as_str())
        .bind(grant.delta)
        .fetch_one(&mut **t)
        .await?;
        sqlx::query(
            "insert into public.inventory_grants (profile_id, item, key, delta, reason, ref, dev, at)
             values ($1::uuid, $2::text, $3::text, $4::bigint, $5::text, $6::text, $7::boolean,
                     to_timestamp($8::double precision / 1000.0))",
        )
        .bind(grant.profile_id.as_str())
        .bind(item.as_str())
        .bind(grant.key.as_str())
        .bind(grant.delta)
        .bind(literal(&grant.reason)?)
        .bind(grant.reference.as_deref())
        .bind(grant.dev)
        .bind(grant.at)
        .execute(&mut **t)
        .await?;
        Ok(Some(count))
    }

    pub async fn counters_get(t: &mut PgTx<'_>, profile_id: &str) -> Result<Vec<CounterEntry>, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let rows = sqlx::query(
            r#"select counter, pack, value from public.economy_counters
                where profile_id = $1::uuid
                order by counter collate "C", pack collate "C""#,
        )
        .bind(profile_id)
        .fetch_all(&mut **t)
        .await?;
        rows.iter()
            .map(|row| -> Result<CounterEntry, StoreError> {
                Ok(CounterEntry {
                    counter: from_literal(row.try_get("counter")?)?,
                    pack: row.try_get("pack")?,
                    value: row.try_get("value")?,
                })
            })
            .collect()
    }

    /// `jlore_grant`'s three statements, over `economy_counters`.
    pub async fn counters_grant(t: &mut PgTx<'_>, grant: &CounterGrant) -> Result<Option<i64>, StoreError> {
        run_as(t, Some(grant.profile_id.as_str())).await?;
        let counter = literal(&grant.counter)?;
        let held: Option<i64> = sqlx::query_scalar(
            "select value from public.economy_counters
              where profile_id = $1::uuid and counter = $2::text and pack = $3::text
                for update",
        )
        .bind(grant.profile_id.as_str())
        .bind(counter.as_str())
        .bind(grant.pack.as_str())
        .fetch_optional(&mut **t)
        .await?;
        if count_after(held, grant.delta).is_none() {
            return Ok(None);
        }
        let value: i64 = if held.is_some() {
            sqlx::query_scalar(
                "update public.economy_counters set value = value + $4::bigint
                  where profile_id = $1::uuid and counter = $2::text and pack = $3::text
              returning value",
            )
        } else {
            sqlx::query_scalar(
                "insert into public.economy_counters (profile_id, counter, pack, value)
                 values ($1::uuid, $2::text, $3::text, $4::bigint)
                 on conflict (profile_id, counter, pack)
                 do update set value = public.economy_counters.value + $4::bigint
                 returning value",
            )
        }
        .bind(grant.profile_id.as_str())
        .bind(counter.as_str())
        .bind(grant.pack.as_str())
        .bind(grant.delta)
        .fetch_one(&mut **t)
        .await?;
        sqlx::query(
            "insert into public.economy_counter_grants (profile_id, counter, pack, delta, reason, ref, dev, at)
             values ($1::uuid, $2::text, $3::text, $4::bigint, $5::text, $6::text, $7::boolean,
                     to_timestamp($8::double precision / 1000.0))",
        )
        .bind(grant.profile_id.as_str())
        .bind(counter.as_str())
        .bind(grant.pack.as_str())
        .bind(grant.delta)
        .bind(literal(&grant.reason)?)
        .bind(grant.reference.as_deref())
        .bind(grant.dev)
        .bind(grant.at)
        .execute(&mut **t)
        .await?;
        Ok(Some(value))
    }

    fn track_of(row: &PgRow) -> Result<SetTrack, StoreError> {
        Ok(SetTrack {
            set_id: row.try_get("set_id")?,
            xp: row.try_get("xp")?,
            level: row.try_get("level")?,
            rewards_claimed: row.try_get("rewards_claimed")?,
        })
    }

    pub async fn tracks_get(t: &mut PgTx<'_>, profile_id: &str) -> Result<Vec<SetTrack>, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let rows = sqlx::query(
            r#"select set_id, xp, level, rewards_claimed from public.set_tracks
                where profile_id = $1::uuid
                order by set_id collate "C""#,
        )
        .bind(profile_id)
        .fetch_all(&mut **t)
        .await?;
        rows.iter().map(track_of).collect()
    }

    /// `jlore_grant`'s three statements, over `set_tracks`: the new track is checked here
    /// (`track_after`), and an absent track is inserted from its first level.
    pub async fn tracks_grant(t: &mut PgTx<'_>, grant: &TrackGrant) -> Result<Option<SetTrack>, StoreError> {
        run_as(t, Some(grant.profile_id.as_str())).await?;
        let held = sqlx::query(
            "select set_id, xp, level, rewards_claimed from public.set_tracks
              where profile_id = $1::uuid and set_id = $2::text
                for update",
        )
        .bind(grant.profile_id.as_str())
        .bind(grant.set_id.as_str())
        .fetch_optional(&mut **t)
        .await?
        .as_ref()
        .map(track_of)
        .transpose()?;
        if track_after(held.as_ref(), grant).is_none() {
            return Ok(None);
        }
        let row = if held.is_some() {
            sqlx::query(
                "update public.set_tracks
                    set xp = xp + $3::bigint, level = level + $4::bigint,
                        rewards_claimed = rewards_claimed + $5::bigint
                  where profile_id = $1::uuid and set_id = $2::text
              returning set_id, xp, level, rewards_claimed",
            )
            .bind(grant.profile_id.as_str())
            .bind(grant.set_id.as_str())
            .bind(grant.xp)
            .bind(grant.levels)
            .bind(grant.rewards)
            .fetch_one(&mut **t)
            .await?
        } else {
            sqlx::query(
                "insert into public.set_tracks (profile_id, set_id, xp, level, rewards_claimed)
                 values ($1::uuid, $2::text, $3::bigint, $6::bigint + $4::bigint, $5::bigint)
                 on conflict (profile_id, set_id) do update
                    set xp = public.set_tracks.xp + $3::bigint,
                        level = public.set_tracks.level + $4::bigint,
                        rewards_claimed = public.set_tracks.rewards_claimed + $5::bigint
                 returning set_id, xp, level, rewards_claimed",
            )
            .bind(grant.profile_id.as_str())
            .bind(grant.set_id.as_str())
            .bind(grant.xp)
            .bind(grant.levels)
            .bind(grant.rewards)
            .bind(SET_TRACK_FIRST_LEVEL)
            .fetch_one(&mut **t)
            .await?
        };
        sqlx::query(
            "insert into public.set_track_grants (profile_id, set_id, xp, levels, rewards, reason, ref, dev, at)
             values ($1::uuid, $2::text, $3::bigint, $4::bigint, $5::bigint, $6::text, $7::text,
                     $8::boolean, to_timestamp($9::double precision / 1000.0))",
        )
        .bind(grant.profile_id.as_str())
        .bind(grant.set_id.as_str())
        .bind(grant.xp)
        .bind(grant.levels)
        .bind(grant.rewards)
        .bind(literal(&grant.reason)?)
        .bind(grant.reference.as_deref())
        .bind(grant.dev)
        .bind(grant.at)
        .execute(&mut **t)
        .await?;
        Ok(Some(track_of(&row)?))
    }

    pub async fn quests_get(t: &mut PgTx<'_>, profile_id: &str) -> Result<QuestBook, StoreError> {
        run_as(t, Some(profile_id)).await?;
        // The schedule first, locked, so the held quests read after it are the ones a top-up
        // under way left when it committed (R1304000).
        let due: Option<Option<OffsetDateTime>> = sqlx::query_scalar(
            "select next_due_at from public.quest_schedules where profile_id = $1::uuid for update",
        )
        .bind(profile_id)
        .fetch_optional(&mut **t)
        .await?;
        let rows = sqlx::query(
            r#"select quest_id, progress, arrived_at from public.held_quests
                where profile_id = $1::uuid
                order by arrived_at, quest_id collate "C""#,
        )
        .bind(profile_id)
        .fetch_all(&mut **t)
        .await?;
        let held = rows
            .iter()
            .map(|row| -> Result<HeldQuest, StoreError> {
                Ok(HeldQuest {
                    quest_id: row.try_get("quest_id")?,
                    progress: row.try_get("progress")?,
                    arrived_at: ms_of(row.try_get("arrived_at")?),
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        Ok(QuestBook {
            held,
            next_due_at: due.flatten().map(ms_of),
        })
    }

    /// One statement for the change; the ledger row only when it touched the held quest.
    pub async fn quests_change(t: &mut PgTx<'_>, grant: &QuestGrant) -> Result<bool, StoreError> {
        run_as(t, Some(grant.profile_id.as_str())).await?;
        let done = match grant.change {
            QuestChange::Arrived => sqlx::query(
                "insert into public.held_quests (profile_id, quest_id, progress, arrived_at)
                 values ($1::uuid, $2::text, 0, to_timestamp($3::double precision / 1000.0))
                 on conflict (profile_id, quest_id) do nothing",
            )
            .bind(grant.profile_id.as_str())
            .bind(grant.quest_id.as_str())
            .bind(grant.at),
            QuestChange::Progressed => sqlx::query(
                "update public.held_quests set progress = progress + $3::bigint
                  where profile_id = $1::uuid and quest_id = $2::text
                    and progress + $3::bigint >= 0",
            )
            .bind(grant.profile_id.as_str())
            .bind(grant.quest_id.as_str())
            .bind(grant.progress),
            QuestChange::Completed | QuestChange::Removed => sqlx::query(
                "delete from public.held_quests where profile_id = $1::uuid and quest_id = $2::text",
            )
            .bind(grant.profile_id.as_str())
            .bind(grant.quest_id.as_str()),
        }
        .execute(&mut **t)
        .await?;
        if done.rows_affected() != 1 {
            return Ok(false);
        }
        sqlx::query(
            "insert into public.quest_grants (profile_id, quest_id, change, progress, reason, ref, dev, at)
             values ($1::uuid, $2::text, $3::text, $4::bigint, $5::text, $6::text, $7::boolean,
                     to_timestamp($8::double precision / 1000.0))",
        )
        .bind(grant.profile_id.as_str())
        .bind(grant.quest_id.as_str())
        .bind(literal(&grant.change)?)
        .bind(grant.progress)
        .bind(literal(&grant.reason)?)
        .bind(grant.reference.as_deref())
        .bind(grant.dev)
        .bind(grant.at)
        .execute(&mut **t)
        .await?;
        Ok(true)
    }

    pub async fn quests_set_next_due(
        t: &mut PgTx<'_>,
        profile_id: &str,
        next_due_at: Option<i64>,
    ) -> Result<(), StoreError> {
        run_as(t, Some(profile_id)).await?;
        sqlx::query(
            "insert into public.quest_schedules (profile_id, next_due_at)
             values ($1::uuid, case when $2::double precision is null then null
                                    else to_timestamp($2::double precision / 1000.0) end)
             on conflict (profile_id) do update set next_due_at = excluded.next_due_at",
        )
        .bind(profile_id)
        .bind(next_due_at)
        .execute(&mut **t)
        .await?;
        Ok(())
    }

    /// One aggregate over the profile's rows (`quest_grants_profile_idx` leads it).
    pub async fn quests_removed_count(
        t: &mut PgTx<'_>,
        profile_id: &str,
        from: i64,
        until: i64,
    ) -> Result<i64, StoreError> {
        run_as(t, Some(profile_id)).await?;
        Ok(sqlx::query_scalar(
            "select count(*)::bigint from public.quest_grants
              where profile_id = $1::uuid and change = $2::text and reason = $3::text
                and at >= to_timestamp($4::double precision / 1000.0)
                and at < to_timestamp($5::double precision / 1000.0)",
        )
        .bind(profile_id)
        .bind(literal(&QuestChange::Removed)?)
        .bind(literal(&EconomyReason::Quest)?)
        .bind(from)
        .bind(until)
        .fetch_one(&mut **t)
        .await?)
    }

    pub async fn cosmetics_get(t: &mut PgTx<'_>, profile_id: &str) -> Result<OwnedCosmetics, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let cards = sqlx::query(
            r#"select card_id, cosmetic_id from public.card_cosmetics
                where profile_id = $1::uuid
                order by card_id collate "C", cosmetic_id collate "C""#,
        )
        .bind(profile_id)
        .fetch_all(&mut **t)
        .await?
        .iter()
        .map(|row| -> Result<CardCosmetic, StoreError> {
            Ok(CardCosmetic {
                card_id: row.try_get("card_id")?,
                cosmetic_id: row.try_get("cosmetic_id")?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
        let account: Vec<String> = sqlx::query_scalar(
            r#"select cosmetic_id from public.account_cosmetics
                where profile_id = $1::uuid
                order by cosmetic_id collate "C""#,
        )
        .bind(profile_id)
        .fetch_all(&mut **t)
        .await?;
        Ok(OwnedCosmetics { cards, account })
    }

    /// One statement for the holding (on a card or on the account); the ledger row only when it
    /// owned or cleared one.
    pub async fn cosmetics_grant(t: &mut PgTx<'_>, grant: &CosmeticGrant) -> Result<bool, StoreError> {
        run_as(t, Some(grant.profile_id.as_str())).await?;
        let owns = grant.delta > 0;
        let done = match (grant.card_id.as_deref(), owns) {
            (Some(card_id), true) => sqlx::query(
                "insert into public.card_cosmetics (profile_id, card_id, cosmetic_id)
                 values ($1::uuid, $2::text, $3::text)
                 on conflict (profile_id, card_id, cosmetic_id) do nothing",
            )
            .bind(grant.profile_id.as_str())
            .bind(card_id)
            .bind(grant.cosmetic_id.as_str()),
            (Some(card_id), false) => sqlx::query(
                "delete from public.card_cosmetics
                  where profile_id = $1::uuid and card_id = $2::text and cosmetic_id = $3::text",
            )
            .bind(grant.profile_id.as_str())
            .bind(card_id)
            .bind(grant.cosmetic_id.as_str()),
            (None, true) => sqlx::query(
                "insert into public.account_cosmetics (profile_id, cosmetic_id)
                 values ($1::uuid, $2::text)
                 on conflict (profile_id, cosmetic_id) do nothing",
            )
            .bind(grant.profile_id.as_str())
            .bind(grant.cosmetic_id.as_str()),
            (None, false) => sqlx::query(
                "delete from public.account_cosmetics where profile_id = $1::uuid and cosmetic_id = $2::text",
            )
            .bind(grant.profile_id.as_str())
            .bind(grant.cosmetic_id.as_str()),
        }
        .execute(&mut **t)
        .await?;
        if done.rows_affected() != 1 {
            return Ok(false);
        }
        sqlx::query(
            "insert into public.cosmetic_grants (profile_id, card_id, cosmetic_id, delta, reason, ref, dev, at)
             values ($1::uuid, $2::text, $3::text, $4::bigint, $5::text, $6::text, $7::boolean,
                     to_timestamp($8::double precision / 1000.0))",
        )
        .bind(grant.profile_id.as_str())
        .bind(grant.card_id.as_deref())
        .bind(grant.cosmetic_id.as_str())
        .bind(grant.delta)
        .bind(literal(&grant.reason)?)
        .bind(grant.reference.as_deref())
        .bind(grant.dev)
        .bind(grant.at)
        .execute(&mut **t)
        .await?;
        Ok(true)
    }

    fn card_picks_of(row: &PgRow) -> Result<CardPicks, StoreError> {
        Ok(CardPicks {
            card_id: row.try_get("card_id")?,
            foil: row.try_get("foil")?,
            finish: row.try_get("finish")?,
            border: row.try_get("border")?,
            full_art: FullArtPick::of_row(row.try_get("full_art")?, row.try_get("full_art_id")?),
        })
    }

    pub async fn card_picks_get(t: &mut PgTx<'_>, profile_id: &str) -> Result<Vec<CardPicks>, StoreError> {
        run_as(t, Some(profile_id)).await?;
        sqlx::query(
            r#"select card_id, foil, finish, border, full_art, full_art_id from public.card_cosmetic_picks
                where profile_id = $1::uuid
                order by card_id collate "C""#,
        )
        .bind(profile_id)
        .fetch_all(&mut **t)
        .await?
        .iter()
        .map(card_picks_of)
        .collect()
    }

    pub async fn card_picks_put(
        t: &mut PgTx<'_>,
        profile_id: &str,
        picks: &CardPicks,
        at: i64,
    ) -> Result<(), StoreError> {
        run_as(t, Some(profile_id)).await?;
        if picks.is_empty() {
            sqlx::query(
                "delete from public.card_cosmetic_picks where profile_id = $1::uuid and card_id = $2::text",
            )
            .bind(profile_id)
            .bind(picks.card_id.as_str())
            .execute(&mut **t)
            .await?;
            return Ok(());
        }
        sqlx::query(
            "insert into public.card_cosmetic_picks (profile_id, card_id, foil, finish, border, full_art, updated_at, full_art_id)
             values ($1::uuid, $2::text, $3::text, $4::text, $5::text, $6::boolean,
                     to_timestamp($7::double precision / 1000.0), $8::text)
             on conflict (profile_id, card_id) do update
                set foil = excluded.foil, finish = excluded.finish, border = excluded.border,
                    full_art = excluded.full_art, full_art_id = excluded.full_art_id,
                    updated_at = excluded.updated_at",
        )
        .bind(profile_id)
        .bind(picks.card_id.as_str())
        .bind(picks.foil.as_deref())
        .bind(picks.finish.as_deref())
        .bind(picks.border.as_deref())
        .bind(picks.full_art.as_ref().map(FullArtPick::prints))
        .bind(at)
        .bind(picks.full_art.as_ref().and_then(FullArtPick::id))
        .execute(&mut **t)
        .await?;
        Ok(())
    }

    pub async fn card_picks_upload(
        t: &mut PgTx<'_>,
        profile_id: &str,
        picks: &[CardPicks],
        at: i64,
    ) -> Result<u64, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let mut written = 0;
        for card in picks.iter().filter(|card| !card.is_empty()) {
            written += sqlx::query(
                "insert into public.card_cosmetic_picks (profile_id, card_id, foil, finish, border, full_art, updated_at, full_art_id)
                 values ($1::uuid, $2::text, $3::text, $4::text, $5::text, $6::boolean,
                         to_timestamp($7::double precision / 1000.0), $8::text)
                 on conflict (profile_id, card_id) do nothing",
            )
            .bind(profile_id)
            .bind(card.card_id.as_str())
            .bind(card.foil.as_deref())
            .bind(card.finish.as_deref())
            .bind(card.border.as_deref())
            .bind(card.full_art.as_ref().map(FullArtPick::prints))
            .bind(at)
            .bind(card.full_art.as_ref().and_then(FullArtPick::id))
            .execute(&mut **t)
            .await?
            .rows_affected();
        }
        Ok(written)
    }

    pub async fn cosmetic_owners(t: &mut PgTx<'_>) -> Result<Vec<CosmeticOwners>, StoreError> {
        run_as(t, None).await?;
        sqlx::query(
            r#"with held as (
                 select c.profile_id, c.card_id, c.cosmetic_id from public.card_cosmetics c
                 union all
                 select a.profile_id, null, a.cosmetic_id from public.account_cosmetics a
               ), owners as (
                 select h.cosmetic_id, h.card_id, count(distinct h.profile_id)::bigint as owners
                   from held h
                   join public.profiles p on p.id = h.profile_id and p.kind = 'human'
                  group by h.cosmetic_id, h.card_id
               ), firsts as (
                 select g.cosmetic_id, g.card_id, min(g.at) as first_at
                   from public.cosmetic_grants g
                   join public.profiles p on p.id = g.profile_id and p.kind = 'human'
                  where g.delta > 0
                  group by g.cosmetic_id, g.card_id
               )
               select o.cosmetic_id, o.card_id, o.owners, f.first_at
                 from owners o
                 left join firsts f
                   on f.cosmetic_id = o.cosmetic_id and f.card_id is not distinct from o.card_id
                order by o.cosmetic_id collate "C", o.card_id collate "C" nulls first"#,
        )
        .fetch_all(&mut **t)
        .await?
        .iter()
        .map(|row| -> Result<CosmeticOwners, StoreError> {
            let first_at: Option<OffsetDateTime> = row.try_get("first_at")?;
            Ok(CosmeticOwners {
                cosmetic_id: row.try_get("cosmetic_id")?,
                card_id: row.try_get("card_id")?,
                owners: row.try_get("owners")?,
                first_granted_at: first_at.map(ms_of),
            })
        })
        .collect()
    }

    /// A transaction-scoped advisory lock on the account's id, as `db::pg::USERNAME_LOCK_SQL` takes
    /// one on a username: released by the commit or the rollback.
    pub async fn lock_account(t: &mut PgTx<'_>, profile_id: &str) -> Result<(), StoreError> {
        run_as(t, Some(profile_id)).await?;
        sqlx::query("select pg_advisory_xact_lock(hashtextextended('jackioh.economy:' || $1::text, 0))")
            .bind(profile_id)
            .execute(&mut **t)
            .await?;
        Ok(())
    }

    pub async fn ledger_get(t: &mut PgTx<'_>, profile_id: &str) -> Result<EconomyLedger, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let rows = |sql: &'static str| sqlx::query(sql).bind(profile_id.to_string());
        let jlore = rows(
            "select delta, reason, ref, dev, at from public.jlore_grants
              where profile_id = $1::uuid order by id",
        )
        .fetch_all(&mut **t)
        .await?
        .iter()
        .map(|row| -> Result<JloreGrant, StoreError> {
            Ok(JloreGrant {
                profile_id: profile_id.to_string(),
                delta: row.try_get("delta")?,
                reason: from_literal(row.try_get("reason")?)?,
                reference: row.try_get("ref")?,
                dev: row.try_get("dev")?,
                at: at_of(row)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
        let inventory = rows(
            "select item, key, delta, reason, ref, dev, at from public.inventory_grants
              where profile_id = $1::uuid order by id",
        )
        .fetch_all(&mut **t)
        .await?
        .iter()
        .map(|row| -> Result<InventoryGrant, StoreError> {
            Ok(InventoryGrant {
                profile_id: profile_id.to_string(),
                item: from_literal(row.try_get("item")?)?,
                key: row.try_get("key")?,
                delta: row.try_get("delta")?,
                reason: from_literal(row.try_get("reason")?)?,
                reference: row.try_get("ref")?,
                dev: row.try_get("dev")?,
                at: at_of(row)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
        let counters = rows(
            "select counter, pack, delta, reason, ref, dev, at from public.economy_counter_grants
              where profile_id = $1::uuid order by id",
        )
        .fetch_all(&mut **t)
        .await?
        .iter()
        .map(|row| -> Result<CounterGrant, StoreError> {
            Ok(CounterGrant {
                profile_id: profile_id.to_string(),
                counter: from_literal(row.try_get("counter")?)?,
                pack: row.try_get("pack")?,
                delta: row.try_get("delta")?,
                reason: from_literal(row.try_get("reason")?)?,
                reference: row.try_get("ref")?,
                dev: row.try_get("dev")?,
                at: at_of(row)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
        let tracks = rows(
            "select set_id, xp, levels, rewards, reason, ref, dev, at from public.set_track_grants
              where profile_id = $1::uuid order by id",
        )
        .fetch_all(&mut **t)
        .await?
        .iter()
        .map(|row| -> Result<TrackGrant, StoreError> {
            Ok(TrackGrant {
                profile_id: profile_id.to_string(),
                set_id: row.try_get("set_id")?,
                xp: row.try_get("xp")?,
                levels: row.try_get("levels")?,
                rewards: row.try_get("rewards")?,
                reason: from_literal(row.try_get("reason")?)?,
                reference: row.try_get("ref")?,
                dev: row.try_get("dev")?,
                at: at_of(row)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
        let quests = rows(
            "select quest_id, change, progress, reason, ref, dev, at from public.quest_grants
              where profile_id = $1::uuid order by id",
        )
        .fetch_all(&mut **t)
        .await?
        .iter()
        .map(|row| -> Result<QuestGrant, StoreError> {
            Ok(QuestGrant {
                profile_id: profile_id.to_string(),
                quest_id: row.try_get("quest_id")?,
                change: from_literal(row.try_get("change")?)?,
                progress: row.try_get("progress")?,
                reason: from_literal(row.try_get("reason")?)?,
                reference: row.try_get("ref")?,
                dev: row.try_get("dev")?,
                at: at_of(row)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
        let cosmetics = rows(
            "select card_id, cosmetic_id, delta, reason, ref, dev, at from public.cosmetic_grants
              where profile_id = $1::uuid order by id",
        )
        .fetch_all(&mut **t)
        .await?
        .iter()
        .map(|row| -> Result<CosmeticGrant, StoreError> {
            Ok(CosmeticGrant {
                profile_id: profile_id.to_string(),
                card_id: row.try_get("card_id")?,
                cosmetic_id: row.try_get("cosmetic_id")?,
                delta: row.try_get("delta")?,
                reason: from_literal(row.try_get("reason")?)?,
                reference: row.try_get("ref")?,
                dev: row.try_get("dev")?,
                at: at_of(row)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
        Ok(EconomyLedger {
            jlore,
            inventory,
            counters,
            tracks,
            quests,
            cosmetics,
        })
    }

    /// Migration 0047. `setup` crosses as text cast to `jsonb`, and reads back as text; so does
    /// migration 0071's `claim`.
    pub async fn practice_seed_insert(t: &mut PgTx<'_>, seed: &PracticeSeed) -> Result<(), StoreError> {
        run_as(t, Some(seed.profile_id.as_str())).await?;
        let claim = seed.claim.as_ref().map(serde_json::to_string).transpose()?;
        let outcome = seed
            .settled
            .as_ref()
            .map(|settled| literal(&settled.outcome))
            .transpose()?;
        sqlx::query(
            "insert into public.practice_seeds
                    (seed, profile_id, setup, issued_at, claimed_at, claim, settled, settled_at)
             values ($1::text, $2::uuid, $3::jsonb, to_timestamp($4::double precision / 1000.0),
                     case when $5::double precision is null then null
                          else to_timestamp($5::double precision / 1000.0) end,
                     $6::jsonb, $7::text,
                     case when $8::double precision is null then null
                          else to_timestamp($8::double precision / 1000.0) end)",
        )
        .bind(seed.seed.as_str())
        .bind(seed.profile_id.as_str())
        .bind(serde_json::to_string(&seed.setup)?)
        .bind(seed.issued_at)
        .bind(seed.claimed_at)
        .bind(claim)
        .bind(outcome)
        .bind(seed.settled.as_ref().map(|settled| settled.at))
        .execute(&mut **t)
        .await?;
        Ok(())
    }

    /// A row of `setup::text as setup, issued_at, claimed_at, claim::text as claim, settled,
    /// settled_at` as the seed `seed` of `profile_id`.
    fn practice_seed_of(row: &PgRow, seed: String, profile_id: String) -> Result<PracticeSeed, StoreError> {
        let setup: String = row.try_get("setup")?;
        let claimed_at: Option<OffsetDateTime> = row.try_get("claimed_at")?;
        let claim: Option<String> = row.try_get("claim")?;
        let outcome: Option<String> = row.try_get("settled")?;
        let settled_at: Option<OffsetDateTime> = row.try_get("settled_at")?;
        let settled = match (outcome, settled_at) {
            (Some(outcome), Some(at)) => Some(PracticeSettled {
                outcome: from_literal(outcome)?,
                at: ms_of(at),
            }),
            _ => None,
        };
        Ok(PracticeSeed {
            seed,
            profile_id,
            setup: serde_json::from_str(&setup)?,
            issued_at: ms_of(row.try_get("issued_at")?),
            claimed_at: claimed_at.map(ms_of),
            claim: claim
                .map(|claim| serde_json::from_str::<PracticeClaim>(&claim))
                .transpose()?,
            settled,
        })
    }

    pub async fn practice_seed_get(
        t: &mut PgTx<'_>,
        profile_id: &str,
        seed: &str,
    ) -> Result<Option<PracticeSeed>, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let row = sqlx::query(
            "select setup::text as setup, issued_at, claimed_at, claim::text as claim, settled, settled_at
               from public.practice_seeds
              where seed = $1::text and profile_id = $2::uuid",
        )
        .bind(seed)
        .bind(profile_id)
        .fetch_optional(&mut **t)
        .await?;
        row.map(|row| practice_seed_of(&row, seed.to_string(), profile_id.to_string()))
            .transpose()
    }

    /// One statement: the row changes only while it is unclaimed.
    pub async fn practice_seed_claim(
        t: &mut PgTx<'_>,
        profile_id: &str,
        seed: &str,
        at: i64,
        claim: &PracticeClaim,
    ) -> Result<bool, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let done = sqlx::query(
            "update public.practice_seeds
                set claimed_at = to_timestamp($3::double precision / 1000.0), claim = $4::jsonb
              where seed = $1::text and profile_id = $2::uuid and claimed_at is null",
        )
        .bind(seed)
        .bind(profile_id)
        .bind(at)
        .bind(serde_json::to_string(claim)?)
        .execute(&mut **t)
        .await?;
        Ok(done.rows_affected() == 1)
    }

    /// One statement: the row changes only while it is claimed and unsettled. A second settle of
    /// the row waits on the first's row lock, then finds it settled and changes nothing.
    pub async fn practice_seed_settle(
        t: &mut PgTx<'_>,
        profile_id: &str,
        seed: &str,
        outcome: PracticeOutcome,
        at: i64,
    ) -> Result<bool, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let done = sqlx::query(
            "update public.practice_seeds
                set settled = $3::text, settled_at = to_timestamp($4::double precision / 1000.0),
                    claim = null
              where seed = $1::text and profile_id = $2::uuid
                and claimed_at is not null and settled_at is null",
        )
        .bind(seed)
        .bind(profile_id)
        .bind(literal(&outcome)?)
        .bind(at)
        .execute(&mut **t)
        .await?;
        Ok(done.rows_affected() == 1)
    }

    /// Every profile's rows, so it runs as no subject: `service_role` bypasses RLS. Migration
    /// 0071's partial index `practice_seeds_unsettled` serves it.
    pub async fn practice_seeds_unsettled(
        t: &mut PgTx<'_>,
        claimed_before: i64,
    ) -> Result<Vec<PracticeSeed>, StoreError> {
        run_as(t, None).await?;
        let rows = sqlx::query(
            "select seed, profile_id::text as profile_id, setup::text as setup, issued_at, claimed_at,
                    claim::text as claim, settled, settled_at
               from public.practice_seeds
              where claim is not null and settled_at is null
                and claimed_at < to_timestamp($1::double precision / 1000.0)
              order by claimed_at, seed collate \"C\"",
        )
        .bind(claimed_before)
        .fetch_all(&mut **t)
        .await?;
        rows.iter()
            .map(|row| practice_seed_of(row, row.try_get("seed")?, row.try_get("profile_id")?))
            .collect()
    }

    /// One statement: the row goes only while it is unclaimed.
    pub async fn practice_seed_release(
        t: &mut PgTx<'_>,
        profile_id: &str,
        seed: &str,
    ) -> Result<bool, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let done = sqlx::query(
            "delete from public.practice_seeds where seed = $1::text and profile_id = $2::uuid and claimed_at is null",
        )
        .bind(seed)
        .bind(profile_id)
        .execute(&mut **t)
        .await?;
        Ok(done.rows_affected() == 1)
    }

    pub async fn practice_seeds_open(
        t: &mut PgTx<'_>,
        profile_id: &str,
        issued_since: i64,
    ) -> Result<i64, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let open: i64 = sqlx::query_scalar(
            "select count(*) from public.practice_seeds
              where profile_id = $1::uuid and claimed_at is null
                and issued_at >= to_timestamp($2::double precision / 1000.0)",
        )
        .bind(profile_id)
        .bind(issued_since)
        .fetch_one(&mut **t)
        .await?;
        Ok(open)
    }

    pub async fn practice_seeds_prune(
        t: &mut PgTx<'_>,
        profile_id: &str,
        issued_before: i64,
    ) -> Result<(), StoreError> {
        run_as(t, Some(profile_id)).await?;
        sqlx::query(
            "delete from public.practice_seeds
              where profile_id = $1::uuid and issued_at < to_timestamp($2::double precision / 1000.0)
                and (claim is null or settled_at is not null)",
        )
        .bind(profile_id)
        .bind(issued_before)
        .execute(&mut **t)
        .await?;
        Ok(())
    }

    /// Migration 0067. `body` crosses as text cast to `jsonb`, and reads back as text.
    pub async fn idempotency_get(
        t: &mut PgTx<'_>,
        profile_id: &str,
        key: &str,
    ) -> Result<Option<IdempotencyRow>, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let row = sqlx::query(
            "select route, body_hash, status::bigint as status, body::text as body, created_at
               from public.idempotency_keys
              where profile_id = $1::uuid and key = $2::uuid",
        )
        .bind(profile_id)
        .bind(key)
        .fetch_optional(&mut **t)
        .await?;
        row.map(|row| -> Result<IdempotencyRow, StoreError> {
            let body: String = row.try_get("body")?;
            Ok(IdempotencyRow {
                profile_id: profile_id.to_string(),
                key: key.to_string(),
                route: row.try_get("route")?,
                body_hash: row.try_get("body_hash")?,
                status: row.try_get("status")?,
                body: serde_json::from_str(&body)?,
                created_at: ms_of(row.try_get("created_at")?),
            })
        })
        .transpose()
    }

    pub async fn idempotency_put(t: &mut PgTx<'_>, row: &IdempotencyRow) -> Result<(), StoreError> {
        run_as(t, Some(row.profile_id.as_str())).await?;
        sqlx::query(
            "insert into public.idempotency_keys (profile_id, key, route, body_hash, status, body, created_at)
             values ($1::uuid, $2::uuid, $3::text, $4::text, $5::smallint, $6::jsonb,
                     to_timestamp($7::double precision / 1000.0))",
        )
        .bind(row.profile_id.as_str())
        .bind(row.key.as_str())
        .bind(row.route.as_str())
        .bind(row.body_hash.as_str())
        .bind(row.status)
        .bind(serde_json::to_string(&row.body)?)
        .bind(row.created_at)
        .execute(&mut **t)
        .await?;
        Ok(())
    }

    /// Every profile's rows, so it runs as no subject: `service_role` bypasses RLS.
    pub async fn idempotency_sweep(t: &mut PgTx<'_>, created_before: i64) -> Result<u64, StoreError> {
        run_as(t, None).await?;
        let done = sqlx::query(
            "delete from public.idempotency_keys
              where created_at < to_timestamp($1::double precision / 1000.0)",
        )
        .bind(created_before)
        .execute(&mut **t)
        .await?;
        Ok(done.rows_affected())
    }
}

// The in-memory fake (`FakeTables::economy`)

mod fake {
    use super::{
        CardCosmetic, CardPicks, CosmeticGrant, CosmeticOwners, CounterEntry, CounterGrant, EconomyLedger,
        EconomyReason, EconomyTables, GameJloreTotals, HeldQuest, IdempotencyRow, InventoryEntry,
        InventoryGrant, JloreGrant, JloreReason, OwnedCosmetics, PracticeClaim, PracticeOutcome,
        PracticeSeed, PracticeSettled, QuestBook, QuestChange, QuestGrant, SetTrack, TrackGrant, count_after,
        literal, track_after,
    };
    use crate::db::fake::FakeTx;
    use crate::db::store::ProfileKind;
    use crate::db::store::StoreError;

    /// The unit fake's two seams (`db::fake`'s `call`): its fault-injection hook, then its race hook,
    /// each told the method's name.
    fn call<'t>(f: &'t mut FakeTx<'_>, method: &str) -> Result<&'t mut EconomyTables, StoreError> {
        let data = f.data();
        if let Some(hook) = data.on_call.clone() {
            hook(method)?;
        }
        if let Some(hook) = data.before_call.clone() {
            hook(method, &mut data.tables);
        }
        Ok(&mut f.tables().economy)
    }

    /// `rows` sorted as Postgres's `order by … collate "C"` sorts them: by `key`'s bytes.
    fn sorted_by<T, K: Ord>(mut rows: Vec<T>, key: impl Fn(&T) -> K) -> Vec<T> {
        rows.sort_by_key(|row| key(row));
        rows
    }

    pub fn jlore_get(f: &mut FakeTx<'_>, profile_id: &str) -> Result<i64, StoreError> {
        let tables = call(f, "economy.wallets.get")?;
        Ok(tables
            .wallets
            .iter()
            .find(|(owner, _)| owner == profile_id)
            .map_or(0, |(_, jlore)| *jlore))
    }

    pub fn jlore_grant(f: &mut FakeTx<'_>, grant: &JloreGrant) -> Result<Option<i64>, StoreError> {
        let tables = call(f, "economy.wallets.grant")?;
        let held = tables
            .wallets
            .iter_mut()
            .find(|(owner, _)| *owner == grant.profile_id);
        let Some(jlore) = count_after(held.as_ref().map(|(_, jlore)| *jlore), grant.delta) else {
            return Ok(None);
        };
        match held {
            Some((_, held)) => *held = jlore,
            None => tables.wallets.push((grant.profile_id.clone(), jlore)),
        }
        tables.jlore_grants.push(grant.clone());
        Ok(Some(jlore))
    }

    pub fn game_jlore_totals(
        f: &mut FakeTx<'_>,
        profile_id: &str,
        from: i64,
        until: i64,
        reference: &str,
    ) -> Result<GameJloreTotals, StoreError> {
        let tables = call(f, "economy.wallets.gameTotals")?;
        let mut totals = GameJloreTotals::default();
        for grant in tables.jlore_grants.iter().filter(|grant| {
            grant.profile_id == profile_id
                && grant.reason == JloreReason::Game
                && (from..until).contains(&grant.at)
        }) {
            totals.day += grant.delta;
            if grant.reference.as_deref() == Some(reference) {
                totals.reference += grant.delta;
            }
        }
        Ok(totals)
    }

    pub fn inventory_get(f: &mut FakeTx<'_>, profile_id: &str) -> Result<Vec<InventoryEntry>, StoreError> {
        let tables = call(f, "economy.inventory.get")?;
        let mut rows = Vec::new();
        for (owner, entry) in &tables.inventory {
            if owner == profile_id {
                rows.push((literal(&entry.item)?, entry.clone()));
            }
        }
        Ok(sorted_by(rows, |(item, entry)| (item.clone(), entry.key.clone()))
            .into_iter()
            .map(|(_, entry)| entry)
            .collect())
    }

    pub fn inventory_grant(f: &mut FakeTx<'_>, grant: &InventoryGrant) -> Result<Option<i64>, StoreError> {
        let tables = call(f, "economy.inventory.grant")?;
        let held = tables.inventory.iter_mut().find(|(owner, entry)| {
            *owner == grant.profile_id && entry.item == grant.item && entry.key == grant.key
        });
        let Some(count) = count_after(held.as_ref().map(|(_, entry)| entry.count), grant.delta) else {
            return Ok(None);
        };
        match held {
            Some((_, entry)) => entry.count = count,
            None => tables.inventory.push((
                grant.profile_id.clone(),
                InventoryEntry {
                    item: grant.item,
                    key: grant.key.clone(),
                    count,
                },
            )),
        }
        tables.inventory_grants.push(grant.clone());
        Ok(Some(count))
    }

    pub fn counters_get(f: &mut FakeTx<'_>, profile_id: &str) -> Result<Vec<CounterEntry>, StoreError> {
        let tables = call(f, "economy.counters.get")?;
        let mut rows = Vec::new();
        for (owner, entry) in &tables.counters {
            if owner == profile_id {
                rows.push((literal(&entry.counter)?, entry.clone()));
            }
        }
        Ok(
            sorted_by(rows, |(counter, entry)| (counter.clone(), entry.pack.clone()))
                .into_iter()
                .map(|(_, entry)| entry)
                .collect(),
        )
    }

    pub fn counters_grant(f: &mut FakeTx<'_>, grant: &CounterGrant) -> Result<Option<i64>, StoreError> {
        let tables = call(f, "economy.counters.grant")?;
        let held = tables.counters.iter_mut().find(|(owner, entry)| {
            *owner == grant.profile_id && entry.counter == grant.counter && entry.pack == grant.pack
        });
        let Some(value) = count_after(held.as_ref().map(|(_, entry)| entry.value), grant.delta) else {
            return Ok(None);
        };
        match held {
            Some((_, entry)) => entry.value = value,
            None => tables.counters.push((
                grant.profile_id.clone(),
                CounterEntry {
                    counter: grant.counter,
                    pack: grant.pack.clone(),
                    value,
                },
            )),
        }
        tables.counter_grants.push(grant.clone());
        Ok(Some(value))
    }

    pub fn tracks_get(f: &mut FakeTx<'_>, profile_id: &str) -> Result<Vec<SetTrack>, StoreError> {
        let tables = call(f, "economy.tracks.get")?;
        let rows = tables
            .tracks
            .iter()
            .filter(|(owner, _)| owner == profile_id)
            .map(|(_, track)| track.clone())
            .collect();
        Ok(sorted_by(rows, |track: &SetTrack| track.set_id.clone()))
    }

    pub fn tracks_grant(f: &mut FakeTx<'_>, grant: &TrackGrant) -> Result<Option<SetTrack>, StoreError> {
        let tables = call(f, "economy.tracks.grant")?;
        let held = tables
            .tracks
            .iter_mut()
            .find(|(owner, track)| *owner == grant.profile_id && track.set_id == grant.set_id);
        let Some(next) = track_after(held.as_ref().map(|(_, track)| track), grant) else {
            return Ok(None);
        };
        match held {
            Some((_, track)) => *track = next.clone(),
            None => tables.tracks.push((grant.profile_id.clone(), next.clone())),
        }
        tables.track_grants.push(grant.clone());
        Ok(Some(next))
    }

    pub fn quests_get(f: &mut FakeTx<'_>, profile_id: &str) -> Result<QuestBook, StoreError> {
        let tables = call(f, "economy.quests.get")?;
        let held = tables
            .held_quests
            .iter()
            .filter(|(owner, _)| owner == profile_id)
            .map(|(_, quest)| quest.clone())
            .collect();
        let next_due_at = tables
            .quest_schedules
            .iter()
            .find(|(owner, _)| owner == profile_id)
            .and_then(|(_, due)| *due);
        Ok(QuestBook {
            held: sorted_by(held, |quest: &HeldQuest| {
                (quest.arrived_at, quest.quest_id.clone())
            }),
            next_due_at,
        })
    }

    pub fn quests_change(f: &mut FakeTx<'_>, grant: &QuestGrant) -> Result<bool, StoreError> {
        let tables = call(f, "economy.quests.change")?;
        let held = tables
            .held_quests
            .iter()
            .position(|(owner, quest)| *owner == grant.profile_id && quest.quest_id == grant.quest_id);
        match (grant.change, held) {
            (QuestChange::Arrived, None) => tables.held_quests.push((
                grant.profile_id.clone(),
                HeldQuest {
                    quest_id: grant.quest_id.clone(),
                    progress: 0,
                    arrived_at: grant.at,
                },
            )),
            (QuestChange::Progressed, Some(at)) => {
                let quest = &mut tables.held_quests[at].1;
                match quest.progress.checked_add(grant.progress) {
                    Some(progress) if progress >= 0 => quest.progress = progress,
                    _ => return Ok(false),
                }
            }
            (QuestChange::Completed | QuestChange::Removed, Some(at)) => {
                tables.held_quests.remove(at);
            }
            _ => return Ok(false),
        }
        tables.quest_grants.push(grant.clone());
        Ok(true)
    }

    pub fn quests_set_next_due(
        f: &mut FakeTx<'_>,
        profile_id: &str,
        next_due_at: Option<i64>,
    ) -> Result<(), StoreError> {
        let tables = call(f, "economy.quests.setNextDue")?;
        match tables
            .quest_schedules
            .iter_mut()
            .find(|(owner, _)| owner == profile_id)
        {
            Some((_, due)) => *due = next_due_at,
            None => tables.quest_schedules.push((profile_id.to_string(), next_due_at)),
        }
        Ok(())
    }

    pub fn quests_removed_count(
        f: &mut FakeTx<'_>,
        profile_id: &str,
        from: i64,
        until: i64,
    ) -> Result<i64, StoreError> {
        let tables = call(f, "economy.quests.removedCount")?;
        let count = tables
            .quest_grants
            .iter()
            .filter(|grant| {
                grant.profile_id == profile_id
                    && grant.change == QuestChange::Removed
                    && grant.reason == EconomyReason::Quest
                    && (from..until).contains(&grant.at)
            })
            .count();
        Ok(i64::try_from(count).unwrap_or(i64::MAX))
    }

    pub fn cosmetics_get(f: &mut FakeTx<'_>, profile_id: &str) -> Result<OwnedCosmetics, StoreError> {
        let tables = call(f, "economy.cosmetics.get")?;
        let cards = tables
            .card_cosmetics
            .iter()
            .filter(|(owner, _)| owner == profile_id)
            .map(|(_, owned)| owned.clone())
            .collect();
        let account = tables
            .account_cosmetics
            .iter()
            .filter(|(owner, _)| owner == profile_id)
            .map(|(_, cosmetic_id)| cosmetic_id.clone())
            .collect();
        Ok(OwnedCosmetics {
            cards: sorted_by(cards, |owned: &CardCosmetic| {
                (owned.card_id.clone(), owned.cosmetic_id.clone())
            }),
            account: sorted_by(account, String::clone),
        })
    }

    pub fn cosmetics_grant(f: &mut FakeTx<'_>, grant: &CosmeticGrant) -> Result<bool, StoreError> {
        let tables = call(f, "economy.cosmetics.grant")?;
        let owns = grant.delta > 0;
        let changed = match grant.card_id.as_deref() {
            Some(card_id) => {
                let held = tables.card_cosmetics.iter().position(|(owner, owned)| {
                    *owner == grant.profile_id
                        && owned.card_id == card_id
                        && owned.cosmetic_id == grant.cosmetic_id
                });
                match (owns, held) {
                    (true, None) => {
                        tables.card_cosmetics.push((
                            grant.profile_id.clone(),
                            CardCosmetic {
                                card_id: card_id.to_string(),
                                cosmetic_id: grant.cosmetic_id.clone(),
                            },
                        ));
                        true
                    }
                    (false, Some(at)) => {
                        tables.card_cosmetics.remove(at);
                        true
                    }
                    _ => false,
                }
            }
            None => {
                let held = tables.account_cosmetics.iter().position(|(owner, cosmetic_id)| {
                    *owner == grant.profile_id && *cosmetic_id == grant.cosmetic_id
                });
                match (owns, held) {
                    (true, None) => {
                        tables
                            .account_cosmetics
                            .push((grant.profile_id.clone(), grant.cosmetic_id.clone()));
                        true
                    }
                    (false, Some(at)) => {
                        tables.account_cosmetics.remove(at);
                        true
                    }
                    _ => false,
                }
            }
        };
        if changed {
            tables.cosmetic_grants.push(grant.clone());
        }
        Ok(changed)
    }

    pub fn card_picks_get(f: &mut FakeTx<'_>, profile_id: &str) -> Result<Vec<CardPicks>, StoreError> {
        let tables = call(f, "economy.cardPicks.get")?;
        let rows = tables
            .card_picks
            .iter()
            .filter(|(owner, _)| owner == profile_id)
            .map(|(_, picks)| picks.clone())
            .collect();
        Ok(sorted_by(rows, |picks: &CardPicks| picks.card_id.clone()))
    }

    pub fn card_picks_put(
        f: &mut FakeTx<'_>,
        profile_id: &str,
        picks: &CardPicks,
        _at: i64,
    ) -> Result<(), StoreError> {
        let tables = call(f, "economy.cardPicks.put")?;
        let held = tables
            .card_picks
            .iter()
            .position(|(owner, held)| owner == profile_id && held.card_id == picks.card_id);
        match (held, picks.is_empty()) {
            (Some(at), true) => {
                tables.card_picks.remove(at);
            }
            (Some(at), false) => tables.card_picks[at].1 = picks.clone(),
            (None, true) => {}
            (None, false) => tables.card_picks.push((profile_id.to_string(), picks.clone())),
        }
        Ok(())
    }

    pub fn card_picks_upload(
        f: &mut FakeTx<'_>,
        profile_id: &str,
        picks: &[CardPicks],
        _at: i64,
    ) -> Result<u64, StoreError> {
        let tables = call(f, "economy.cardPicks.upload")?;
        let mut written = 0;
        for card in picks.iter().filter(|card| !card.is_empty()) {
            let held = tables
                .card_picks
                .iter()
                .any(|(owner, held)| owner == profile_id && held.card_id == card.card_id);
            if !held {
                tables.card_picks.push((profile_id.to_string(), card.clone()));
                written += 1;
            }
        }
        Ok(written)
    }

    pub fn cosmetic_owners(f: &mut FakeTx<'_>) -> Result<Vec<CosmeticOwners>, StoreError> {
        let people: Vec<String> = f
            .tables()
            .profiles
            .iter()
            .filter(|profile| profile.kind == ProfileKind::Human)
            .map(|profile| profile.id.clone())
            .collect();
        let tables = call(f, "economy.cosmeticOwners")?;
        let held = tables
            .card_cosmetics
            .iter()
            .map(|(owner, owned)| (owner, Some(owned.card_id.clone()), owned.cosmetic_id.clone()))
            .chain(
                tables
                    .account_cosmetics
                    .iter()
                    .map(|(owner, cosmetic_id)| (owner, None, cosmetic_id.clone())),
            )
            .filter(|(owner, _, _)| people.contains(owner));
        let mut out: Vec<CosmeticOwners> = Vec::new();
        for (_, card_id, cosmetic_id) in held {
            match out
                .iter_mut()
                .find(|row| row.cosmetic_id == cosmetic_id && row.card_id == card_id)
            {
                Some(row) => row.owners += 1,
                None => {
                    let first_granted_at = tables
                        .cosmetic_grants
                        .iter()
                        .filter(|grant| {
                            grant.delta > 0
                                && grant.cosmetic_id == cosmetic_id
                                && grant.card_id == card_id
                                && people.contains(&grant.profile_id)
                        })
                        .map(|grant| grant.at)
                        .min();
                    out.push(CosmeticOwners {
                        cosmetic_id,
                        card_id,
                        owners: 1,
                        first_granted_at,
                    });
                }
            }
        }
        Ok(sorted_by(out, |row| {
            (row.cosmetic_id.clone(), row.card_id.clone())
        }))
    }

    /// The fake's transactions already run one at a time (`db::fake::FakeTx`), so its lock is
    /// only the seams.
    pub fn lock_account(f: &mut FakeTx<'_>, _profile_id: &str) -> Result<(), StoreError> {
        call(f, "economy.lock")?;
        Ok(())
    }

    pub fn ledger_get(f: &mut FakeTx<'_>, profile_id: &str) -> Result<EconomyLedger, StoreError> {
        let tables = call(f, "economy.ledger.get")?;
        fn mine<T: Clone>(rows: &[T], owner: impl Fn(&T) -> &str, profile_id: &str) -> Vec<T> {
            rows.iter()
                .filter(|row| owner(row) == profile_id)
                .cloned()
                .collect()
        }
        Ok(EconomyLedger {
            jlore: mine(&tables.jlore_grants, |row| &row.profile_id, profile_id),
            inventory: mine(&tables.inventory_grants, |row| &row.profile_id, profile_id),
            counters: mine(&tables.counter_grants, |row| &row.profile_id, profile_id),
            tracks: mine(&tables.track_grants, |row| &row.profile_id, profile_id),
            quests: mine(&tables.quest_grants, |row| &row.profile_id, profile_id),
            cosmetics: mine(&tables.cosmetic_grants, |row| &row.profile_id, profile_id),
        })
    }

    /// A seed already held is refused, as Postgres's primary key refuses it.
    pub fn practice_seed_insert(f: &mut FakeTx<'_>, seed: &PracticeSeed) -> Result<(), StoreError> {
        let tables = call(f, "economy.practiceSeeds.insert")?;
        if tables.practice_seeds.iter().any(|held| held.seed == seed.seed) {
            return Err(StoreError::Other(format!(
                "duplicate practice seed {}",
                seed.seed
            )));
        }
        tables.practice_seeds.push(seed.clone());
        Ok(())
    }

    pub fn practice_seed_get(
        f: &mut FakeTx<'_>,
        profile_id: &str,
        seed: &str,
    ) -> Result<Option<PracticeSeed>, StoreError> {
        let tables = call(f, "economy.practiceSeeds.get")?;
        Ok(tables
            .practice_seeds
            .iter()
            .find(|held| held.seed == seed && held.profile_id == profile_id)
            .cloned())
    }

    pub fn practice_seed_claim(
        f: &mut FakeTx<'_>,
        profile_id: &str,
        seed: &str,
        at: i64,
        claim: &PracticeClaim,
    ) -> Result<bool, StoreError> {
        let tables = call(f, "economy.practiceSeeds.claim")?;
        match tables
            .practice_seeds
            .iter_mut()
            .find(|held| held.seed == seed && held.profile_id == profile_id && held.claimed_at.is_none())
        {
            Some(held) => {
                held.claimed_at = Some(at);
                held.claim = Some(claim.clone());
                Ok(true)
            }
            None => Ok(false),
        }
    }

    pub fn practice_seed_settle(
        f: &mut FakeTx<'_>,
        profile_id: &str,
        seed: &str,
        outcome: PracticeOutcome,
        at: i64,
    ) -> Result<bool, StoreError> {
        let tables = call(f, "economy.practiceSeeds.settle")?;
        match tables.practice_seeds.iter_mut().find(|held| {
            held.seed == seed
                && held.profile_id == profile_id
                && held.claimed_at.is_some()
                && held.settled.is_none()
        }) {
            Some(held) => {
                held.settled = Some(PracticeSettled { outcome, at });
                held.claim = None;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    pub fn practice_seeds_unsettled(
        f: &mut FakeTx<'_>,
        claimed_before: i64,
    ) -> Result<Vec<PracticeSeed>, StoreError> {
        let tables = call(f, "economy.practiceSeeds.unsettled")?;
        let unsettled: Vec<PracticeSeed> = tables
            .practice_seeds
            .iter()
            .filter(|held| {
                held.claim.is_some()
                    && held.settled.is_none()
                    && held
                        .claimed_at
                        .is_some_and(|claimed_at| claimed_at < claimed_before)
            })
            .cloned()
            .collect();
        Ok(sorted_by(unsettled, |held| (held.claimed_at, held.seed.clone())))
    }

    pub fn practice_seed_release(
        f: &mut FakeTx<'_>,
        profile_id: &str,
        seed: &str,
    ) -> Result<bool, StoreError> {
        let tables = call(f, "economy.practiceSeeds.release")?;
        let before = tables.practice_seeds.len();
        tables.practice_seeds.retain(|held| {
            !(held.seed == seed && held.profile_id == profile_id && held.claimed_at.is_none())
        });
        Ok(tables.practice_seeds.len() < before)
    }

    pub fn practice_seeds_open(
        f: &mut FakeTx<'_>,
        profile_id: &str,
        issued_since: i64,
    ) -> Result<i64, StoreError> {
        let tables = call(f, "economy.practiceSeeds.open")?;
        let open = tables
            .practice_seeds
            .iter()
            .filter(|held| {
                held.profile_id == profile_id && held.claimed_at.is_none() && held.issued_at >= issued_since
            })
            .count();
        Ok(i64::try_from(open).unwrap_or(i64::MAX))
    }

    pub fn practice_seeds_prune(
        f: &mut FakeTx<'_>,
        profile_id: &str,
        issued_before: i64,
    ) -> Result<(), StoreError> {
        let tables = call(f, "economy.practiceSeeds.prune")?;
        tables.practice_seeds.retain(|held| {
            held.profile_id != profile_id
                || held.issued_at >= issued_before
                || (held.claim.is_some() && held.settled.is_none())
        });
        Ok(())
    }

    pub fn idempotency_get(
        f: &mut FakeTx<'_>,
        profile_id: &str,
        key: &str,
    ) -> Result<Option<IdempotencyRow>, StoreError> {
        let tables = call(f, "economy.idempotencyKeys.get")?;
        Ok(tables
            .idempotency_keys
            .iter()
            .find(|held| held.profile_id == profile_id && held.key == key)
            .cloned())
    }

    /// A key the profile already holds is refused, as Postgres's primary key refuses it.
    pub fn idempotency_put(f: &mut FakeTx<'_>, row: &IdempotencyRow) -> Result<(), StoreError> {
        let tables = call(f, "economy.idempotencyKeys.put")?;
        if tables
            .idempotency_keys
            .iter()
            .any(|held| held.profile_id == row.profile_id && held.key == row.key)
        {
            return Err(StoreError::Other(format!(
                "duplicate idempotency key {} for {}",
                row.key, row.profile_id
            )));
        }
        tables.idempotency_keys.push(row.clone());
        Ok(())
    }

    pub fn idempotency_sweep(f: &mut FakeTx<'_>, created_before: i64) -> Result<u64, StoreError> {
        let tables = call(f, "economy.idempotencyKeys.sweep")?;
        let before = tables.idempotency_keys.len();
        tables
            .idempotency_keys
            .retain(|held| held.created_at >= created_before);
        Ok(u64::try_from(before - tables.idempotency_keys.len()).unwrap_or(u64::MAX))
    }
}
