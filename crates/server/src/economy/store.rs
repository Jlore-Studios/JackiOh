//! The economy's store (SPEC §14.11; migration 0045): every holding of §14 beside its append-only
//! ledger, over Postgres (`pg` below) or the in-memory fake (`fake` below, whose tables are
//! `FakeTables::economy`). Migration 0047's practice seeds (§14.8, #1303) live here too: a seed is
//! no holding and has no ledger, and a claim burns it in place (`economy_practice_seed_claim`).
//!
//! Same shape as `duos::store`: every store method is one method on `Tx`, named
//! `economy_<table>_<verb>`, whose body dispatches to this file's `pg::<name>` or `fake::<name>`.
//! A grant writes its holding and its ledger row together, in the caller's transaction, and a grant
//! the holding cannot take (a spend the wallet cannot cover, a quest not held, a cosmetic already
//! owned) answers `None` or `false` and writes nothing (§14.1, §14.5). A grant the schema refuses
//! by its checks (a zero change, a `dev` reason without the `dev` mark, a key its item does not
//! take) is refused here first, before either store, as `StoreError::Other("economy: …")`, so both
//! answer it alike.
//!
//! Lists come in their key's byte order (Postgres's `collate "C"`), a count that fell to zero
//! included; ledgers come oldest first. Integers are `i64`, times epoch milliseconds, as everywhere
//! in the store.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use jackioh_engine::config::Difficulty;
use jackioh_engine::wire::PrintedRarity;
use jackioh_engine::{LastBoardEntry, LeanPick, PlayerId};

use crate::config::SET_TRACK_FIRST_LEVEL;
use crate::cosmetics::CosmeticRarity;
use crate::db::store::{StoreError, StoreResult, Tx};

// ---------------------------------------------------------------------------
// The literals
// ---------------------------------------------------------------------------

/// Why a Jlore row was written (§14.11; `jlore_grants.reason`): a game's, a quest's or a track
/// level's pay (§14.7–§14.9), a pack bought (§14.3), or a dev account's own grant or clear (§14.10).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JloreReason {
    Game,
    Quest,
    Track,
    PackPurchase,
    Dev,
}

/// Why a row of every other economy ledger was written (§14.11): the Jlore reasons, and a pack
/// opened (§14.3), a token bar paying (§14.5), a token redeemed (§14.5) and the Chase Shard bar
/// buying (§14.6).
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

/// A literal's text, as its column holds it.
fn literal<T: Serialize>(value: &T) -> Result<String, StoreError> {
    match serde_json::to_value(value)? {
        serde_json::Value::String(text) => Ok(text),
        other => Err(StoreError::Other(format!("economy: {other} is not a literal"))),
    }
}

/// A literal read back from its column.
fn from_literal<T: DeserializeOwned>(text: String) -> Result<T, StoreError> {
    Ok(serde_json::from_value(serde_json::Value::String(text))?)
}

// ---------------------------------------------------------------------------
// The rows
// ---------------------------------------------------------------------------

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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OwnedCosmetics {
    pub cards: Vec<CardCosmetic>,
    pub account: Vec<String>,
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

/// One row of `public.practice_seeds` (migration 0047): a seed issued for a practice game, and
/// when it was claimed (R1303001).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PracticeSeed {
    pub seed: String,
    pub profile_id: String,
    pub setup: PracticeSetup,
    pub issued_at: i64,
    pub claimed_at: Option<i64>,
}

// ---------------------------------------------------------------------------
// The ledgers' rows. Each says why (`reason`), what it was for (`reference`, the `ref` column), and
// whether a dev tool wrote it (`dev`, §14.10).
// ---------------------------------------------------------------------------

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
}

// ---------------------------------------------------------------------------
// What both stores refuse alike: migration 0045's checks
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// The store methods
// ---------------------------------------------------------------------------

/// `economy_dispatch!(self, name(args))`: the same call against whichever store this transaction
/// is on.
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

    /// Adds a grant's XP, levels and rewards paid to a track (from its first level with no XP when
    /// it has no row) and writes its ledger row. The track after it, or `None` when that would be no
    /// track (see `track_after`), which writes nothing.
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

    /// §14.11: every economy ledger row of the account, each table's oldest first.
    pub async fn economy_ledger_get(&mut self, profile_id: &str) -> StoreResult<EconomyLedger> {
        economy_dispatch!(self, ledger_get(profile_id))
    }

    /// §14.8, #1303: writes a seed just issued for a practice game.
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

    /// R1303001: marks the profile's seed claimed at `at`. False, writing nothing, when it has no
    /// such seed or it was claimed already, so two claims racing for one seed burn it once.
    pub async fn economy_practice_seed_claim(
        &mut self,
        profile_id: &str,
        seed: &str,
        at: i64,
    ) -> StoreResult<bool> {
        economy_dispatch!(self, practice_seed_claim(profile_id, seed, at))
    }

    /// R1303001: deletes the profile's seeds issued before `issued_before`, claimed or not.
    pub async fn economy_practice_seeds_prune(
        &mut self,
        profile_id: &str,
        issued_before: i64,
    ) -> StoreResult<()> {
        economy_dispatch!(self, practice_seeds_prune(profile_id, issued_before))
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
}

// ---------------------------------------------------------------------------
// Postgres (migration 0045)
// ---------------------------------------------------------------------------

mod pg {
    use sqlx::Row;
    use sqlx::postgres::PgRow;
    use time::OffsetDateTime;

    use super::{
        CardCosmetic, CosmeticGrant, CounterEntry, CounterGrant, EconomyLedger, HeldQuest, InventoryEntry,
        InventoryGrant, JloreGrant, OwnedCosmetics, PracticeSeed, QuestBook, QuestChange, QuestGrant,
        SetTrack, TrackGrant, count_after, from_literal, literal, track_after,
    };
    use crate::config::SET_TRACK_FIRST_LEVEL;
    use crate::db::pg::PgTx;
    use crate::db::store::StoreError;

    /// `db::pg`'s `SESSION_SQL` and acting role: every method re-stamps its subject first, so it
    /// runs as `service_role` with `request.jwt.claim.sub` the profile the call is about.
    const SESSION_SQL: &str =
        "select set_config('role', $1, true), set_config('request.jwt.claim.sub', $2, true)";
    const ACTING_ROLE: &str = "service_role";

    /// Nanoseconds in a millisecond.
    const NANOS_PER_MS: i128 = 1_000_000;

    async fn run_as(t: &mut PgTx<'_>, subject: Option<&str>) -> Result<(), StoreError> {
        sqlx::query(SESSION_SQL)
            .bind(ACTING_ROLE)
            .bind(subject.unwrap_or(""))
            .execute(&mut **t)
            .await?;
        Ok(())
    }

    fn ms_of(value: OffsetDateTime) -> i64 {
        (value.unix_timestamp_nanos() / NANOS_PER_MS) as i64
    }

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

    /// Migration 0047. `setup` crosses as text cast to `jsonb`, and reads back as text.
    pub async fn practice_seed_insert(t: &mut PgTx<'_>, seed: &PracticeSeed) -> Result<(), StoreError> {
        run_as(t, Some(seed.profile_id.as_str())).await?;
        sqlx::query(
            "insert into public.practice_seeds (seed, profile_id, setup, issued_at, claimed_at)
             values ($1::text, $2::uuid, $3::jsonb, to_timestamp($4::double precision / 1000.0),
                     case when $5::double precision is null then null
                          else to_timestamp($5::double precision / 1000.0) end)",
        )
        .bind(seed.seed.as_str())
        .bind(seed.profile_id.as_str())
        .bind(serde_json::to_string(&seed.setup)?)
        .bind(seed.issued_at)
        .bind(seed.claimed_at)
        .execute(&mut **t)
        .await?;
        Ok(())
    }

    pub async fn practice_seed_get(
        t: &mut PgTx<'_>,
        profile_id: &str,
        seed: &str,
    ) -> Result<Option<PracticeSeed>, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let row = sqlx::query(
            "select setup::text as setup, issued_at, claimed_at from public.practice_seeds
              where seed = $1::text and profile_id = $2::uuid",
        )
        .bind(seed)
        .bind(profile_id)
        .fetch_optional(&mut **t)
        .await?;
        row.map(|row| -> Result<PracticeSeed, StoreError> {
            let setup: String = row.try_get("setup")?;
            let claimed_at: Option<OffsetDateTime> = row.try_get("claimed_at")?;
            Ok(PracticeSeed {
                seed: seed.to_string(),
                profile_id: profile_id.to_string(),
                setup: serde_json::from_str(&setup)?,
                issued_at: ms_of(row.try_get("issued_at")?),
                claimed_at: claimed_at.map(ms_of),
            })
        })
        .transpose()
    }

    /// One statement: the row changes only while it is unclaimed.
    pub async fn practice_seed_claim(
        t: &mut PgTx<'_>,
        profile_id: &str,
        seed: &str,
        at: i64,
    ) -> Result<bool, StoreError> {
        run_as(t, Some(profile_id)).await?;
        let done = sqlx::query(
            "update public.practice_seeds set claimed_at = to_timestamp($3::double precision / 1000.0)
              where seed = $1::text and profile_id = $2::uuid and claimed_at is null",
        )
        .bind(seed)
        .bind(profile_id)
        .bind(at)
        .execute(&mut **t)
        .await?;
        Ok(done.rows_affected() == 1)
    }

    pub async fn practice_seeds_prune(
        t: &mut PgTx<'_>,
        profile_id: &str,
        issued_before: i64,
    ) -> Result<(), StoreError> {
        run_as(t, Some(profile_id)).await?;
        sqlx::query(
            "delete from public.practice_seeds
              where profile_id = $1::uuid and issued_at < to_timestamp($2::double precision / 1000.0)",
        )
        .bind(profile_id)
        .bind(issued_before)
        .execute(&mut **t)
        .await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The in-memory fake (`FakeTables::economy`)
// ---------------------------------------------------------------------------

mod fake {
    use super::{
        CardCosmetic, CosmeticGrant, CounterEntry, CounterGrant, EconomyLedger, EconomyTables, HeldQuest,
        InventoryEntry, InventoryGrant, JloreGrant, OwnedCosmetics, PracticeSeed, QuestBook, QuestChange,
        QuestGrant, SetTrack, TrackGrant, count_after, literal, track_after,
    };
    use crate::db::fake::FakeTx;
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
    ) -> Result<bool, StoreError> {
        let tables = call(f, "economy.practiceSeeds.claim")?;
        match tables
            .practice_seeds
            .iter_mut()
            .find(|held| held.seed == seed && held.profile_id == profile_id && held.claimed_at.is_none())
        {
            Some(held) => {
                held.claimed_at = Some(at);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    pub fn practice_seeds_prune(
        f: &mut FakeTx<'_>,
        profile_id: &str,
        issued_before: i64,
    ) -> Result<(), StoreError> {
        let tables = call(f, "economy.practiceSeeds.prune")?;
        tables
            .practice_seeds
            .retain(|held| held.profile_id != profile_id || held.issued_at >= issued_before);
        Ok(())
    }
}
