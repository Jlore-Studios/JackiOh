//! The in-memory store (SURFACE §11.1, §11.2): one `FakeData` behind `Db::Fake` for the unit tests
//! and for `E2E=1`, ported from `apps/server/src/api/e2e-store.ts` (the end-to-end store) and
//! `apps/server/src/api/memory-stores.ts` (the halves both TS in-memory stores shared). TS had two
//! fakes, `src/api/e2e-store.ts` and `test/fakes/store.ts`, sharing `memory-stores.ts` so they could
//! not drift; Rust has one, so they cannot drift by construction. What only one of them did is an
//! option here: R111's launch grant runs when the store is built with a catalog (`E2E=1`,
//! [`create_e2e_store`]) and not otherwise (`FakeData::default()`, the unit tests' store, which
//! "carries no launch grant, so the existing tests keep seeing the store their assertions were
//! written against"); `on_call` is the unit fake's fault-injection seam; `seed_profile` and the
//! public `tables` are its test helpers.
//!
//! # From `e2e-store.ts`
//!
//! The `Store` BUILD M8's end-to-end mode runs on, held entirely in memory.
//!
//! WHY THIS EXISTS AND WHY IT IS HERE. R144 makes the server reseed its fixture accounts and invite
//! codes at boot, which needs somewhere to seed *into*, and the e2e suite has no Postgres. The real
//! implementation of this port is `src/db/pg.rs`; this one is a test fixture that happens to live in
//! `src/` because `src/app.rs` has to be able to choose it at boot. It is reachable only when
//! `env.E2E` is true, and `src/env.rs` refuses `E2E` together with `NODE_ENV=production`.
//!
//! It is deliberately strict everywhere Postgres is strict, because those are the invariants
//! SPEC §9.4 and §9.5 lean on and a laxer fixture would let a real bug pass the suite:
//!
//!  - a transaction snapshots every table at `begin` and restores it unless it is committed, so
//!    §9.4's "writes `collection` and `collection_grants` in one transaction" is really
//!    all-or-nothing, and runs one transaction at a time (the `tokio::sync::Mutex` behind
//!    `Db::Fake`, which hands itself out first come, first served: TS's `createTransactionQueue`),
//!    so a transaction that rolls back never takes another request's committed writes with it;
//!  - `redeem` runs §9.4's six steps in SPEC's order inside one transaction and answers the same
//!    seven result codes `app.redeem_invite_code` does, so the redemption the server calls here is
//!    the redemption it calls in Postgres;
//!  - `codes_claim`, `rooms_claim` and `tickets_claim_pair` are single-shot, so the second of two
//!    racing callers loses (§9.4 step 6, §9.5);
//!  - `matches_append_actions` refuses a `seq` that already exists (append-only, §9.3);
//!  - `results_insert` refuses a second row for the same match (§9.5);
//!  - `tickets_insert` refuses a second open ticket for one profile (`tickets_profile_queued_key`,
//!    which `queue.rs` relies on as the race-proof half of "not already queued");
//!  - `decks`, `trios`, `series`, `tutorial` and `game_records` are the memory-stores half below:
//!    the deck and trio caps, the owner check, `series_update`'s compare-and-set, the tutorial's
//!    grow-only merge and one game record per id (R250, R252, R263, R320, R376).
//!
//! R111 IS A DATABASE TRIGGER, so it is implemented here rather than in a handler. SPEC §11 R111:
//! "Becoming `active` grants one copy of every non-token card, written by a trigger on the
//! `pending → active` transition and idempotent, so a repeated redemption cannot double a
//! collection." Nothing in application code grants it — `codes.rs` only calls
//! `profiles_set_status(profile_id, Active)` — so `profiles_set_status` carries the trigger,
//! writing both tables exactly as `grant_cards` in `collection.rs` does and reading the same
//! `LAUNCH_COPIES` and `LAUNCH_GRANT_REASON` so the two can never drift.
//!
//! # From `memory-stores.ts`
//!
//! The in-memory halves of the stores R250–R263 added — saved decks, saved trios and the Conquest
//! series — R320's tutorial progress and R376's game records. TS's factories closed over a
//! `tables()` getter because both stores replaced their tables wholesale (a rolled-back `tx`,
//! `reset()`); here every function takes the transaction and reads the tables through it.
//!
//! Strict where Postgres is strict (migrations 0007, 0008, 0011):
//!  - `decks_upsert` / `trios_upsert` refuse an id owned by another profile (`not_owner`) and a
//!    create past the cap (`limit`), exactly as `app.upsert_deck` / `app.upsert_trio` do;
//!  - `trios_upsert` refuses a slot naming a deck that is not this profile's (`unknown_deck`, the
//!    composite foreign key) and one deck in two slots (the check constraint);
//!  - `decks_remove` empties every trio slot that named the deck (`on delete set null`);
//!  - `series_update` is compare-and-set on `version`;
//!  - `tutorial_merge` only ever grows the lessons and keeps the newest choice (0011, R320);
//!  - `player_settings_merge` replaces a group only with a strictly later one and caps the result's
//!    groups and bytes (0018, R633, R634);
//!  - `ranked_create_season` refuses an id that exists, `ranked_record_game` a second row for one
//!    game, and `ranked_note_peak_jlorious` only ever lowers a peak (R608, R609, R611; migration
//!    0022 carries the same tables on Postgres),
//!  - `game_records_insert` writes one record per id and refuses a second, and refuses a
//!    development record without a `dev:` id or a live one with one (0014, R376, R378).
//!
//! # Shape (SURFACE §11.2)
//!
//! One free function per store method, named `<substore>_<method>` (root `redeem`,
//! `purge_expired`), taking the transaction first and TS's arguments after it, in TS's order;
//! `db::store`'s `Tx` dispatches to them. They are synchronous: the transaction already holds the
//! one lock, so nothing here awaits. A TS `throw` is an `Err(StoreError)`, and the transaction it
//! happened in is then dropped without `commit`, which restores the snapshot (TS: "rolls back if
//! `fn` throws"). TS's nested `tx` joined the enclosing one; in Rust a caller that is already in a
//! transaction passes its `&mut Tx` on, so there is nothing to join.

use std::sync::Arc;

use indexmap::{IndexMap, IndexSet};
use serde_json::Value;
use tokio::sync::{Mutex, MutexGuard};

use jackioh_engine::wire::stats::{
    CardStatsFilter, DEV_RECORD_ID_PREFIX, GameRecord, GameSource, PilotFilter, record_matches,
};

use crate::api::collection::{LAUNCH_COPIES, LAUNCH_GRANT_REASON};
use crate::config::{
    CODE_ATTEMPT_WINDOW_SECONDS, CODE_ATTEMPTS_PER_IP_PER_HOUR, CODE_ATTEMPTS_PER_PROFILE_PER_HOUR,
    RATING_DEVIATION_START, RATING_START, RATING_VOLATILITY_START, USERNAME_DEFAULT_BASE,
};
use crate::db::store::{
    BotRating, CodeAttempt, CodeAttemptResult, CollectionEntry, CollectionGrant, Db, FavouriteCard, FunStats,
    GameRecordQuery, InviteCode, LastBoardEntry, LastBoardKind, MatchActionRow, MatchClocks, MatchRow,
    MatchStatus, PerMode, PlayerSettingsLimits, PlayerSettingsMergeInput, PlayerSettingsMergeOutcome,
    PlayerSettingsRow, PlayerStatsListOptions, PlayerStatsRow, Profile, ProfileCreateInput, ProfileRecord,
    ProfileStatus, PublicPlayerSummary, QueueMode, RatedGameRow, RedeemInviteCodeInput, RedeemResult,
    ResultRow, RetentionPurgeInput, RetentionPurgeResult, Room, SavedDeck, SavedTrio, Season, SeasonStanding,
    SeriesRow, SeriesStatus, StoreError, Ticket, TicketStatus, TrioUpsertOutcome, TutorialMergeInput,
    TutorialMergeOutcome, TutorialProgressRow, UpsertOutcome, UsernameClaim, UsernameClaimOutcome,
};
use crate::ranked::glicko2::Glicko;
use crate::ranked::ladder::SeasonRank;
use crate::ranked::season::{ResetChange, ResetPlayer};
use crate::username::username_key;

// ---------------------------------------------------------------------------
// The tables
// ---------------------------------------------------------------------------

/// One collection row: a profile's quantity of one card (`public.collection`).
#[derive(serde::Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CollectionRow {
    pub profile_id: String,
    pub card_id: String,
    pub quantity: i64,
}

/// One row of `public.last_boards` (migration 0017): one per profile and kind (R565).
#[derive(serde::Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LastBoardRow {
    pub profile_id: String,
    pub kind: LastBoardKind,
    pub board: Vec<LastBoardEntry>,
}

/// One row of `public.player_stats` (R639, R654).
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerStatsTableRow {
    pub profile_id: String,
    pub stats: IndexMap<String, Value>,
    pub is_private: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Every table, as both TS in-memory stores held them (TS `Tables`: the e2e store's own tables `&`
/// `DeckTables & TutorialTables & PlayerSettingsTables & RankedTables & LastBoardTables &
/// PlayerStatsTables & GameRecordTables`, one struct here). Public, so a test can assert on the
/// raw rows (TS `MemoryStore.tables`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FakeTables {
    pub profiles: Vec<Profile>,
    pub codes: Vec<InviteCode>,
    pub attempts: Vec<CodeAttempt>,
    pub collection: Vec<CollectionRow>,
    pub grants: Vec<CollectionGrant>,
    // DeckTables
    pub decks: Vec<SavedDeck>,
    pub trios: Vec<SavedTrio>,
    pub series: Vec<SeriesRow>,
    pub matches: Vec<MatchRow>,
    pub match_actions: Vec<MatchActionRow>,
    pub rooms: Vec<Room>,
    pub tickets: Vec<Ticket>,
    pub results: Vec<ResultRow>,
    // TutorialTables: the table R320 adds (`public.tutorial_progress`, migration 0011): one row per profile.
    pub tutorial: Vec<TutorialProgressRow>,
    // PlayerSettingsTables: the table R633 adds (`public.player_settings`, migration 0018): one row per profile.
    pub player_settings: Vec<PlayerSettingsRow>,
    // LastBoardTables: the table R565 adds (`public.last_boards`, migration 0017): one row per profile and kind.
    pub last_boards: Vec<LastBoardRow>,
    // PlayerStatsTables
    pub player_stats: Vec<PlayerStatsTableRow>,
    // GameRecordTables: the table R376 adds (`public.game_records`, migration 0014): one row per recorded game.
    pub game_records: Vec<GameRecord>,
    // RankedTables
    pub seasons: Vec<Season>,
    pub season_ranks: Vec<SeasonRank>,
    pub bots: Vec<BotRating>,
    pub rated_games: Vec<RatedGameRow>,
}

/// TS `emptyTables()` (with `emptyDeckTables`, `emptyTutorialTables`, `emptyPlayerSettingsTables`,
/// `emptyPlayerStatsTables` and `emptyRankedTables` folded in: one struct, every table empty).
pub fn empty_tables() -> FakeTables {
    FakeTables::default()
}

/// The clock a fake reads (TS: the runtime's `Timers.now`, epoch milliseconds; "never `Date.now()`
/// directly"). A test on a virtual clock hands its own.
pub type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;

/// The unit fake's fault-injection seam (TS `MemoryStore.onCall`): called with the port method's TS
/// name (`"decks.list"`, `"redeem"`, …) before each operation; an `Err` fails that method, and the
/// transaction it ran in rolls back when it is dropped.
pub type OnCall = Arc<dyn Fn(&str) -> Result<(), StoreError> + Send + Sync>;

/// What a `before_call` hook tells the method it ran before.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallOutcome {
    /// The method runs on the tables as the hook left them.
    Proceed,
    /// A compare-and-set (`series.update`) loses: it writes nothing and answers `false`, as TS's
    /// tests made it by replacing the method with `async () => false`. Any other method proceeds.
    Lose,
}

/// The unit fake's race seam, beside `OnCall`: called with the port method's TS name and the tables
/// it is about to read, before each operation, inside its transaction. It may change them — a rival
/// writer landing between a read and a write, as TS's tests did by wrapping `store.series.update`
/// — and may make a compare-and-set lose.
pub type BeforeCall = Arc<dyn Fn(&str, &mut FakeTables) -> CallOutcome + Send + Sync>;

/// The wall clock in epoch milliseconds: the default `now` of a store built without one (TS
/// `options.now ?? (() => Date.now())`).
pub fn system_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// What R111's launch grant reads from the catalog (TS `CatalogInfo`'s `cardIds`, `isToken` and
/// `isBanned`): R111 needs to know which ids are non-token and unbanned; §9.4 L3 and L6 define both.
#[derive(Clone)]
pub struct FakeCatalog {
    pub card_ids: Vec<String>,
    pub is_token: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    pub is_banned: Arc<dyn Fn(&str) -> bool + Send + Sync>,
}

/// Everything the fake holds: the tables, which a transaction snapshots, and what it was built with,
/// which a rollback leaves alone (TS kept `nextProfile` and the options in the factory's closure,
/// outside the snapshot, too).
pub struct FakeData {
    pub tables: FakeTables,
    /// `profile-<n>`'s next `n` (TS `nextProfile`).
    pub next_profile: u32,
    /// The clock `redeem` and R111's grant read.
    pub now: Clock,
    /// What the database would answer for §9.4's limits, kill switch and email verification. Public,
    /// so a test can flip it between calls (TS: hooks, "which is how the store contract drives it
    /// against both stores").
    pub redemption: RedemptionSettings,
    /// R111's launch grant runs on `pending → active` only when this is set (`E2E=1`).
    pub catalog: Option<FakeCatalog>,
    /// The unit tests' fault-injection hook; `None` charges nothing.
    pub on_call: Option<OnCall>,
    /// The unit tests' race hook (`BeforeCall`); `None` changes nothing.
    pub before_call: Option<BeforeCall>,
}

/// TS `E2EStoreOptions`.
pub struct E2eStoreOptions {
    /// R111 needs to know which ids are non-token and unbanned; §9.4 L3 and L6 define both.
    pub catalog: FakeCatalog,
    /// The `Timers.now` of the runtime this store belongs to; never `Date.now()` directly.
    pub now: Clock,
    /// What the database would answer for §9.4's limits, kill switch and email verification.
    pub redemption: Option<RedemptionSettings>,
}

impl std::fmt::Debug for FakeData {
    /// The rows and the profile counter; the hooks have nothing to print.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FakeData")
            .field("tables", &self.tables)
            .field("next_profile", &self.next_profile)
            .field(
                "catalog",
                &self.catalog.as_ref().map(|catalog| catalog.card_ids.len()),
            )
            .field("on_call", &self.on_call.is_some())
            .field("before_call", &self.before_call.is_some())
            .finish_non_exhaustive()
    }
}

/// TS `createMemoryStore()`: the unit tests' empty store, on the wall clock, with the default
/// redemption settings and no launch grant (`Db::fake()` wraps it).
impl Default for FakeData {
    fn default() -> FakeData {
        FakeData::new(Arc::new(system_now), default_redemption_settings(), None)
    }
}

impl FakeData {
    /// An empty store with the given clock, redemption settings and (for `E2E=1`) catalog.
    pub fn new(now: Clock, redemption: RedemptionSettings, catalog: Option<FakeCatalog>) -> FakeData {
        FakeData {
            tables: empty_tables(),
            next_profile: 1,
            now,
            redemption,
            catalog,
            on_call: None,
            before_call: None,
        }
    }

    /// R144: "the server reseeds its fixture accounts and invite codes at boot". The reseed is a
    /// wipe and rewrite rather than a merge, so a second run starts from the same rows as the first.
    pub fn reset(&mut self) {
        self.tables = empty_tables();
        self.next_profile = 1;
    }

    /// The append-only grant ledger (§9.4), which the collection store has no read for because no
    /// client path shows it. Exposed so the boot log and the R111 tests can see what the trigger
    /// wrote.
    pub fn grants_for(&self, profile_id: &str) -> Vec<CollectionGrant> {
        self.tables
            .grants
            .iter()
            .filter(|grant| grant.profile_id == profile_id)
            .cloned()
            .collect()
    }

    /// The fixed username an end-to-end fixture account carries (R1435): `base` bare, the prompt
    /// answered and no cooldown running, as `seed-accounts` names its accounts in Postgres. False
    /// when there is no such profile.
    pub fn name_fixture(&mut self, profile_id: &str, base: &str) -> bool {
        let Some(row) = profile_of(&mut self.tables, profile_id) else {
            return false;
        };
        row.username_base = base.to_string();
        row.username_key = username_key(base);
        row.username_tag = None;
        row.username_changed_at = None;
        row.username_prompted = true;
        true
    }

    /// Seeds a profile without going through the API (TS `MemoryStore.seedProfile`, which took
    /// `Partial<Profile> & { id }`): `input` is that object literal as JSON, every field but `id`
    /// optional, camelCase as TS wrote it. Defaults: user `user-<id>`, email `<id>@example.test`,
    /// `active`, a new player's rating, deviation and volatility, in no match, created at 0, and
    /// the username a new account gets, the lowest free `Player#n`, its prompt not yet answered
    /// (R1434). `username` names a base instead (stored as given, so a caller passes a stored
    /// form), with `usernameTag` as its tag or else the tag a claim of it would carry now;
    /// `usernamePrompted` and `usernameChangedAt` set the rest.
    pub fn seed_profile(&mut self, input: Value) -> Profile {
        let id = input
            .get("id")
            .and_then(Value::as_str)
            .expect("seed_profile needs an id")
            .to_string();
        let text = |key: &str| input.get(key).and_then(Value::as_str).map(str::to_string);
        let number = |key: &str| input.get(key).and_then(Value::as_f64);
        let status = input
            .get("status")
            .map(|value| serde_json::from_value::<ProfileStatus>(value.clone()).expect("a profile status"))
            .unwrap_or(ProfileStatus::Active);
        let (username_base, key, tag) = match text("username") {
            Some(base) => {
                let key = username_key(&base);
                let tag = match input.get("usernameTag").and_then(Value::as_i64) {
                    Some(tag) => Some(tag),
                    None => username_tag_for(&self.tables.profiles, &id, &key),
                };
                (base, key, tag)
            }
            None => default_username(&self.tables.profiles),
        };
        let profile = Profile {
            id: id.clone(),
            user_id: text("userId").unwrap_or_else(|| format!("user-{id}")),
            email: text("email").unwrap_or_else(|| format!("{id}@example.test")),
            username_base,
            username_key: key,
            username_tag: tag,
            username_changed_at: input.get("usernameChangedAt").and_then(Value::as_i64),
            username_prompted: input
                .get("usernamePrompted")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            status,
            rating: number("rating").unwrap_or(RATING_START),
            rating_deviation: number("ratingDeviation").unwrap_or(RATING_DEVIATION_START),
            rating_volatility: number("ratingVolatility").unwrap_or(RATING_VOLATILITY_START),
            in_match_id: text("inMatchId"),
            created_at: input.get("createdAt").and_then(Value::as_i64).unwrap_or(0),
        };
        self.tables.profiles.push(profile.clone());
        self.next_profile += 1;
        profile
    }
}

/// TS `createE2EStore(options)`: the `E2E=1` server's store, with R111's launch grant. The app keeps
/// the `Db::Fake` handle and reaches [`FakeData::reset`] and [`FakeData::grants_for`] through it.
pub fn create_e2e_store(options: E2eStoreOptions) -> Db {
    let redemption = options.redemption.unwrap_or_else(default_redemption_settings);
    Db::Fake(Arc::new(Mutex::new(FakeData::new(
        options.now,
        redemption,
        Some(options.catalog),
    ))))
}

// ---------------------------------------------------------------------------
// Transactions
// ---------------------------------------------------------------------------

/// One transaction over the fake: the lock, held until it ends (so transactions run one at a time,
/// first come first served), and the snapshot taken when it began. Dropped without [`commit`], it
/// puts the snapshot back: a rolled-back transaction leaves nothing behind.
pub struct FakeTx<'a> {
    guard: MutexGuard<'a, FakeData>,
    snapshot: Option<Box<FakeTables>>,
}

impl<'a> FakeTx<'a> {
    /// A transaction over an already-held lock; the tables as they are now are its snapshot.
    pub fn new(guard: MutexGuard<'a, FakeData>) -> FakeTx<'a> {
        let snapshot = Some(Box::new(guard.tables.clone()));
        FakeTx { guard, snapshot }
    }

    /// The whole store, for a test helper that runs inside a transaction.
    pub fn data(&mut self) -> &mut FakeData {
        &mut self.guard
    }

    /// The tables as this transaction sees them.
    pub fn tables(&mut self) -> &mut FakeTables {
        &mut self.guard.tables
    }
}

impl std::fmt::Debug for FakeTx<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FakeTx")
            .field("committed", &self.snapshot.is_none())
            .finish_non_exhaustive()
    }
}

impl Drop for FakeTx<'_> {
    fn drop(&mut self) {
        if let Some(snapshot) = self.snapshot.take() {
            self.guard.tables = *snapshot;
        }
    }
}

/// `Db::begin` for the fake: wait for the lock (TS `createTransactionQueue().run`: "a transaction
/// waits for the one before it"), then snapshot every table.
pub async fn begin(data: &Arc<Mutex<FakeData>>) -> FakeTx<'_> {
    FakeTx::new(data.lock().await)
}

/// `Tx::commit` for the fake: keep what the transaction wrote and let the next one in.
pub fn commit(mut f: FakeTx<'_>) {
    f.snapshot = None;
}

/// Charges `method` to the fault-injection hook, then runs the race hook, if either is set.
fn call(f: &mut FakeTx<'_>, method: &str) -> Result<(), StoreError> {
    call_cas(f, method).map(|_| ())
}

/// `call` for a compare-and-set method, which the race hook may make lose.
fn call_cas(f: &mut FakeTx<'_>, method: &str) -> Result<CallOutcome, StoreError> {
    if let Some(hook) = f.guard.on_call.clone() {
        hook(method)?;
    }
    Ok(match f.guard.before_call.clone() {
        Some(hook) => hook(method, &mut f.guard.tables),
        None => CallOutcome::Proceed,
    })
}

/// TS `new DuplicateResultError(matchId)`: `results_insert`'s refusal of a second row for one match.
fn duplicate_result(match_id: &str) -> StoreError {
    StoreError::Duplicate(match_id.to_string())
}

// ---------------------------------------------------------------------------
// SPEC §9.4's redemption, in memory (`redeem`)
// ---------------------------------------------------------------------------

/// The database state `app.redeem_invite_code` reads that no other port method can reach, and the
/// three numbers §9.4 writes into the function's body. Defaults come from `src/config.rs` — the
/// same constants `default_limits()` copies into `ApiLimits` — so the fixture and the deployment
/// agree without either restating a value.
#[derive(Clone)]
pub struct RedemptionSettings {
    /// §9.4 step 2: "more than 5 attempts in the last hour".
    pub attempts_per_profile_per_hour: i64,
    /// §9.4 step 3: "more than 20".
    pub attempts_per_ip_per_hour: i64,
    /// The window both counters are taken over.
    pub attempt_window_ms: i64,
    /// `app.settings.redemption_enabled` — the database-side kill switch the SQL function checks
    /// before the lookup and answers `circuit_open` from. A hook rather than a value so a caller can
    /// flip it between calls, which is how the store contract drives it against both stores.
    pub enabled: Arc<dyn Fn() -> bool + Send + Sync>,
    /// `auth.users.email_confirmed_at is not null` for this profile (§9.4 step 1). `public.profiles`
    /// has no such column and neither does `Profile`, so the SQL function joins `auth.users` for it
    /// and the fixture asks here. True by default: BUILD M8's three fixture accounts are all verified
    /// (`src/api/e2e.rs`), and the server refuses an unverified caller from the access token (R159)
    /// before this port is reached.
    pub email_verified: Arc<dyn Fn(&str) -> bool + Send + Sync>,
}

/// TS `defaultRedemptionSettings(overrides)`. TS's `overrides` is Rust's struct update:
/// `RedemptionSettings { enabled: …, ..default_redemption_settings() }`.
pub fn default_redemption_settings() -> RedemptionSettings {
    RedemptionSettings {
        attempts_per_profile_per_hour: CODE_ATTEMPTS_PER_PROFILE_PER_HOUR,
        attempts_per_ip_per_hour: CODE_ATTEMPTS_PER_IP_PER_HOUR,
        attempt_window_ms: CODE_ATTEMPT_WINDOW_SECONDS * 1000,
        enabled: Arc::new(|| true),
        email_verified: Arc::new(|_profile_id| true),
    }
}

/// §9.4 step 4's attempt row, written through the port (TS `redeemOne`'s `log`).
fn log_redeem_attempt(
    f: &mut FakeTx<'_>,
    input: &RedeemInviteCodeInput,
    now: i64,
    result: CodeAttemptResult,
    reason: &str,
) -> Result<(), StoreError> {
    codes_log_attempt(
        f,
        &CodeAttempt {
            profile_id: Some(input.profile_id.clone()),
            ip_hash: input.ip_hash.clone(),
            result,
            reason: reason.to_string(),
            at: now,
        },
    )
}

/// Steps 4 and 5's refusal (TS `redeemOne`'s `reject`): log the attempt, answer `invalid_code`.
fn reject_redeem(
    f: &mut FakeTx<'_>,
    input: &RedeemInviteCodeInput,
    now: i64,
    reason: &str,
) -> Result<RedeemResult, StoreError> {
    log_redeem_attempt(f, input, now, CodeAttemptResult::Rejected, reason)?;
    Ok(RedeemResult::InvalidCode)
}

/// SPEC §9.4's redemption, whole (TS `createInMemoryRedeem`'s `redeemOne`, and `Store.redeem`).
///
/// §9.4's six steps over this store's own methods, which is what makes this the fake half of
/// `app.redeem_invite_code` rather than a second reading of §9.4: the steps run in SPEC's order,
/// rejections are returned and never thrown, and every read and write goes back through the port so
/// the fake's own invariants (`codes_claim` is single-shot, `profiles_set_status` carries R111's
/// launch grant) hold here exactly as they do for any other caller.
///
/// CALLS RUN ONE AT A TIME. TS serialised redemptions through a promise chain, because without it
/// two redemptions by one pending profile both passed step 1 across an `await` and one account used
/// up two codes. Here a redemption runs inside a transaction, which holds the store's one lock, so
/// two of them cannot interleave at all.
///
/// That is STRICTER than Postgres, not the same. `app.redeem_invite_code` serialises two redemptions
/// only where they share something it locks: the profile row (step 1), the caller's IP hash (an
/// advisory lock before step 3's count, migration 0006) and the code row (step 5). Two redemptions by
/// different profiles from different addresses for different codes run side by side there. So a
/// race this fixture cannot lose still needs `tests/store/redeem_race.rs` against Postgres
/// (`tests/db/run.sh`).
pub fn redeem(f: &mut FakeTx<'_>, input: &RedeemInviteCodeInput) -> Result<RedeemResult, StoreError> {
    call(f, "redeem")?;
    let settings = f.guard.redemption.clone();
    // Read once the call holds the lock, as the database reads its clock inside the transaction.
    let now = (f.guard.now)();
    let profile_id = input.profile_id.as_str();

    // §9.4: "Redemption is one server-side transaction." In Postgres the whole of this is one
    // `select app.redeem_invite_code(...)`; here it is the caller's transaction, so a fault leaves
    // nothing behind.

    // ---- Step 1: "reject unless the account is pending with a verified email" --------------
    // One result for "no such profile", "banned" and "already active", exactly as the SQL's
    // `if not found or v_status is distinct from 'pending'` decides them together.
    let profile = profiles_get_by_id(f, profile_id)?;
    match profile {
        Some(profile) if profile.status == ProfileStatus::Pending => {}
        _ => return Ok(RedeemResult::NotPending),
    }
    if !(settings.email_verified)(profile_id) {
        return Ok(RedeemResult::EmailUnverified);
    }

    let since = now - settings.attempt_window_ms;

    // ---- Step 2: "reject if this profile made more than 5 attempts in the last hour" --------
    // Strictly `>`, and the count excludes the attempt being made because step 4 logs it below,
    // so the SEVENTH attempt is the first refused (`src/config.rs` carries the reasoning).
    if codes_count_attempts_by_profile(f, profile_id, since)? > settings.attempts_per_profile_per_hour {
        return Ok(RedeemResult::RateLimitedProfile);
    }

    // ---- Step 3: "reject if this IP hash made more than 20" ---------------------------------
    if codes_count_attempts_by_ip(f, &input.ip_hash, since)? > settings.attempts_per_ip_per_hour {
        return Ok(RedeemResult::RateLimitedIp);
    }

    // The database-side kill switch, checked where the SQL checks it: after the cheap refusals
    // and before the lookup. Unlike steps 2 and 3 this one DOES log the attempt, because the
    // caller got far enough to spend one.
    if !(settings.enabled)() {
        log_redeem_attempt(f, input, now, CodeAttemptResult::Rejected, "circuit_open")?;
        return Ok(RedeemResult::CircuitOpen);
    }

    // ---- Steps 4 and 5: log the attempt either way, then "look up by hash and reject if
    // revoked, expired or exhausted" ---------------------------------------------------------
    // The log is written with the outcome rather than ahead of it: the code store has only
    // `log_attempt`, and no way to flip a row's result afterwards the way the SQL updates its
    // own. Inside one transaction the two orders commit identically; what matters is that every
    // attempt past steps 2 and 3 leaves a row, whichever way the code turned out.

    // A code that could never exist (§9.4's alphabet and length) takes the identical path: no
    // oracle separates "well formed but unknown" from "not a code at all".
    let Some(code_hash) = input.code_hash.as_deref() else {
        return reject_redeem(f, input, now, "malformed");
    };

    let Some(code) = codes_find_by_hash(f, code_hash)? else {
        return reject_redeem(f, input, now, "missing");
    };
    if code.revoked {
        return reject_redeem(f, input, now, "revoked");
    }
    if code.expires_at.is_some_and(|expires_at| expires_at <= now) {
        return reject_redeem(f, input, now, "expired");
    }
    if code.uses >= code.max_uses {
        return reject_redeem(f, input, now, "exhausted");
    }

    // ---- Step 6: "increment uses and set the account active, atomically" --------------------
    // `claim` is the atomic increment, so a false return is a code that ran out between the
    // check above and here.
    if !codes_claim(f, &code.id, now)? {
        return reject_redeem(f, input, now, "exhausted");
    }
    profiles_set_status(f, profile_id, ProfileStatus::Active)?;
    log_redeem_attempt(f, input, now, CodeAttemptResult::Ok, "redeemed")?;
    Ok(RedeemResult::Ok)
}

// ---------------------------------------------------------------------------
// R111's trigger
// ---------------------------------------------------------------------------

/// TS `profileOf`.
fn profile_of<'t>(tables: &'t mut FakeTables, profile_id: &str) -> Option<&'t mut Profile> {
    tables.profiles.iter_mut().find(|row| row.id == profile_id)
}

/// R1434: the lowest tag from 1 that no profile but `profile_id` holds under `key`.
fn lowest_free_tag(profiles: &[Profile], profile_id: &str, key: &str) -> i64 {
    let held: IndexSet<i64> = profiles
        .iter()
        .filter(|profile| profile.id != profile_id && profile.username_key == key)
        .filter_map(|profile| profile.username_tag)
        .collect();
    (1..).find(|tag| !held.contains(tag)).unwrap_or(1)
}

/// R1434: the tag a claim of `key` by `profile_id` would carry: none while no other profile holds
/// the bare name, else the lowest free tag. Migration 0028's `USERNAME_TAG_FOR_SQL` in pg.rs.
fn username_tag_for(profiles: &[Profile], profile_id: &str, key: &str) -> Option<i64> {
    let bare_taken = profiles.iter().any(|profile| {
        profile.id != profile_id && profile.username_key == key && profile.username_tag.is_none()
    });
    bare_taken.then(|| lowest_free_tag(profiles, profile_id, key))
}

/// R1434: a new account's username, the lowest free `Player#n`, always tagged: migration 0028's
/// `app.assign_default_username`. The new row is not among `profiles` yet.
fn default_username(profiles: &[Profile]) -> (String, String, Option<i64>) {
    let key = username_key(USERNAME_DEFAULT_BASE);
    let tag = lowest_free_tag(profiles, "", &key);
    (USERNAME_DEFAULT_BASE.to_string(), key, Some(tag))
}

/// TS `collectionRow`.
fn collection_row<'t>(
    tables: &'t mut FakeTables,
    profile_id: &str,
    card_id: &str,
) -> Option<&'t mut CollectionRow> {
    tables
        .collection
        .iter_mut()
        .find(|row| row.profile_id == profile_id && row.card_id == card_id)
}

/// One copy of every non-token, unbanned card, idempotent. Written as a delta against what the
/// profile already owns — exactly what `grant_cards` does — so a second `pending → active`
/// transition computes zero for everything and writes no row at all: `collection_grants.delta`
/// carries `check (delta <> 0)` in migration `0002_collection.sql`, and an empty audit row would
/// be a lie in the ledger. A store built without a catalog (the unit tests') grants nothing.
fn apply_launch_grant(f: &mut FakeTx<'_>, profile_id: &str) {
    let Some(catalog) = f.guard.catalog.clone() else {
        return;
    };
    let at = (f.guard.now)();
    let tables = f.tables();
    let mut quantities: Vec<CollectionEntry> = Vec::new();
    let mut grants: Vec<CollectionGrant> = Vec::new();

    for card_id in &catalog.card_ids {
        if (catalog.is_token)(card_id) || (catalog.is_banned)(card_id) {
            continue;
        }
        let owned = collection_row(tables, profile_id, card_id)
            .map(|row| row.quantity)
            .unwrap_or(0);
        let delta = LAUNCH_COPIES - owned;
        if delta <= 0 {
            continue;
        }
        quantities.push(CollectionEntry {
            card_id: card_id.clone(),
            quantity: owned + delta,
        });
        grants.push(CollectionGrant {
            profile_id: profile_id.to_string(),
            card_id: card_id.clone(),
            delta,
            reason: LAUNCH_GRANT_REASON.to_string(),
            at,
        });
    }

    if quantities.is_empty() {
        return;
    }
    for entry in &quantities {
        match collection_row(tables, profile_id, &entry.card_id) {
            Some(row) => row.quantity = entry.quantity,
            None => tables.collection.push(CollectionRow {
                profile_id: profile_id.to_string(),
                card_id: entry.card_id.clone(),
                quantity: entry.quantity,
            }),
        }
    }
    tables.grants.extend(grants);
}

/// Migration 0013's retention purge (`src/api/retention.rs`), `Store.purgeExpired`.
pub fn purge_expired(
    f: &mut FakeTx<'_>,
    input: &RetentionPurgeInput,
) -> Result<RetentionPurgeResult, StoreError> {
    call(f, "purgeExpired")?;
    Ok(purge_expired_rows(f.tables(), input))
}

// ---------------------------------------------------------------------------
// Profiles
// ---------------------------------------------------------------------------

pub fn profiles_get_by_id(f: &mut FakeTx<'_>, profile_id: &str) -> Result<Option<Profile>, StoreError> {
    call(f, "profiles.getById")?;
    Ok(f.tables()
        .profiles
        .iter()
        .find(|row| row.id == profile_id)
        .cloned())
}

pub fn profiles_get_by_user_id(f: &mut FakeTx<'_>, user_id: &str) -> Result<Option<Profile>, StoreError> {
    call(f, "profiles.getByUserId")?;
    Ok(f.tables()
        .profiles
        .iter()
        .find(|profile| profile.user_id == user_id)
        .cloned())
}

pub fn profiles_get_many(f: &mut FakeTx<'_>, profile_ids: &[String]) -> Result<Vec<Profile>, StoreError> {
    call(f, "profiles.getMany")?;
    Ok(f.tables()
        .profiles
        .iter()
        .filter(|profile| profile_ids.contains(&profile.id))
        .cloned()
        .collect())
}

pub fn profiles_get_by_username(
    f: &mut FakeTx<'_>,
    key: &str,
    tag: Option<i64>,
) -> Result<Option<Profile>, StoreError> {
    call(f, "profiles.getByUsername")?;
    Ok(f.tables()
        .profiles
        .iter()
        .find(|profile| profile.username_key == key && profile.username_tag == tag)
        .cloned())
}

/// A new profile, at `rating` with a new player's deviation and volatility (R603).
pub fn profiles_create(f: &mut FakeTx<'_>, input: &ProfileCreateInput) -> Result<Profile, StoreError> {
    call(f, "profiles.create")?;
    if f.tables()
        .profiles
        .iter()
        .any(|profile| profile.user_id == input.user_id)
    {
        return Err(StoreError::from(format!(
            "profiles.user_id is unique: {} already has a profile",
            input.user_id
        )));
    }
    // §9.4: an account exists the moment auth says so and stays pending until a code is
    // redeemed, which is what `resolve_caller` in http.rs relies on.
    let n = f.guard.next_profile;
    let (username_base, key, tag) = default_username(&f.tables().profiles);
    let profile = Profile {
        id: format!("profile-{n}"),
        user_id: input.user_id.clone(),
        email: input.email.clone(),
        username_base,
        username_key: key,
        username_tag: tag,
        username_changed_at: None,
        username_prompted: false,
        status: ProfileStatus::Pending,
        rating: input.rating,
        rating_deviation: RATING_DEVIATION_START,
        rating_volatility: RATING_VOLATILITY_START,
        in_match_id: None,
        created_at: input.at,
    };
    f.guard.next_profile += 1;
    f.tables().profiles.push(profile.clone());
    Ok(profile)
}

pub fn profiles_set_status(
    f: &mut FakeTx<'_>,
    profile_id: &str,
    status: ProfileStatus,
) -> Result<(), StoreError> {
    call(f, "profiles.setStatus")?;
    let Some(row) = profile_of(f.tables(), profile_id) else {
        return Err(StoreError::from(format!("no profile {profile_id}")));
    };
    let was_pending = row.status == ProfileStatus::Pending;
    row.status = status;
    // R111: the trigger fires on the `pending → active` transition only.
    if was_pending && status == ProfileStatus::Active {
        apply_launch_grant(f, profile_id);
    }
    Ok(())
}

/// R603: one profile's hidden rating after a rated game.
pub fn profiles_set_glicko(f: &mut FakeTx<'_>, profile_id: &str, glicko: &Glicko) -> Result<(), StoreError> {
    call(f, "profiles.setGlicko")?;
    let Some(row) = profile_of(f.tables(), profile_id) else {
        return Err(StoreError::from(format!("no profile {profile_id}")));
    };
    row.rating = glicko.rating;
    row.rating_deviation = glicko.deviation;
    row.rating_volatility = glicko.volatility;
    Ok(())
}

// TS `setRating` (no caller) is not ported (SURFACE §11.2).

pub fn profiles_username_tag_for(
    f: &mut FakeTx<'_>,
    profile_id: &str,
    key: &str,
) -> Result<Option<i64>, StoreError> {
    call(f, "profiles.usernameTagFor")?;
    Ok(username_tag_for(&f.tables().profiles, profile_id, key))
}

/// R1434, R1435. The store's one lock is the lock on the key Postgres takes.
pub fn profiles_claim_username(
    f: &mut FakeTx<'_>,
    claim: &UsernameClaim,
) -> Result<UsernameClaimOutcome, StoreError> {
    call(f, "profiles.claimUsername")?;
    let profile_id = claim.profile_id.as_str();
    let Some(row) = f.tables().profiles.iter().find(|row| row.id == profile_id) else {
        return Err(StoreError::from(format!("no profile {profile_id}")));
    };
    if let Some(changed_at) = row.username_changed_at {
        let next_change_at = changed_at + claim.cooldown_ms;
        if next_change_at > claim.at {
            return Ok(UsernameClaimOutcome::Cooldown { next_change_at });
        }
    }
    let tag = username_tag_for(&f.tables().profiles, profile_id, &claim.key);
    if tag != claim.expected_tag {
        return Ok(UsernameClaimOutcome::Changed { tag });
    }
    let Some(row) = profile_of(f.tables(), profile_id) else {
        return Err(StoreError::from(format!("no profile {profile_id}")));
    };
    row.username_base = claim.base.clone();
    row.username_key = claim.key.clone();
    row.username_tag = tag;
    row.username_changed_at = Some(claim.at);
    row.username_prompted = true;
    Ok(UsernameClaimOutcome::Claimed { tag })
}

pub fn profiles_answer_username_prompt(f: &mut FakeTx<'_>, profile_id: &str) -> Result<(), StoreError> {
    call(f, "profiles.answerUsernamePrompt")?;
    let Some(row) = profile_of(f.tables(), profile_id) else {
        return Err(StoreError::from(format!("no profile {profile_id}")));
    };
    row.username_prompted = true;
    Ok(())
}

/// Pass `None` to clear. §9.5: every terminal reason clears both players'.
pub fn profiles_set_in_match(
    f: &mut FakeTx<'_>,
    profile_id: &str,
    match_id: Option<&str>,
) -> Result<(), StoreError> {
    call(f, "profiles.setInMatch")?;
    let Some(row) = profile_of(f.tables(), profile_id) else {
        return Err(StoreError::from(format!("no profile {profile_id}")));
    };
    row.in_match_id = match_id.map(str::to_string);
    Ok(())
}

/// Migration 0012's account deletion, [`remove_profile_rows`].
pub fn profiles_remove(f: &mut FakeTx<'_>, profile_id: &str) -> Result<bool, StoreError> {
    call(f, "profiles.remove")?;
    Ok(remove_profile_rows(f.tables(), profile_id))
}

// ---------------------------------------------------------------------------
// Invite codes (§9.4)
// ---------------------------------------------------------------------------

pub fn codes_insert(f: &mut FakeTx<'_>, code: &InviteCode) -> Result<(), StoreError> {
    call(f, "codes.insert")?;
    if f.tables().codes.iter().any(|row| row.code_hash == code.code_hash) {
        return Err(StoreError::from("invite_codes.code_hash is unique".to_string()));
    }
    f.tables().codes.push(code.clone());
    Ok(())
}

pub fn codes_find_by_hash(f: &mut FakeTx<'_>, code_hash: &str) -> Result<Option<InviteCode>, StoreError> {
    call(f, "codes.findByHash")?;
    Ok(f.tables()
        .codes
        .iter()
        .find(|code| code.code_hash == code_hash)
        .cloned())
}

/// §9.4 step 6: one atomic statement. Two callers cannot both win the last use.
pub fn codes_claim(f: &mut FakeTx<'_>, code_id: &str, now: i64) -> Result<bool, StoreError> {
    call(f, "codes.claim")?;
    let Some(row) = f.tables().codes.iter_mut().find(|code| code.id == code_id) else {
        return Ok(false);
    };
    if row.revoked {
        return Ok(false);
    }
    if row.expires_at.is_some_and(|expires_at| expires_at <= now) {
        return Ok(false);
    }
    if row.uses >= row.max_uses {
        return Ok(false);
    }
    row.uses += 1;
    Ok(true)
}

/// §9.4 step 4: the attempt is logged either way. Written by `redeem`.
pub fn codes_log_attempt(f: &mut FakeTx<'_>, attempt: &CodeAttempt) -> Result<(), StoreError> {
    call(f, "codes.logAttempt")?;
    f.tables().attempts.push(attempt.clone());
    Ok(())
}

pub fn codes_count_attempts_by_profile(
    f: &mut FakeTx<'_>,
    profile_id: &str,
    since: i64,
) -> Result<i64, StoreError> {
    call(f, "codes.countAttemptsByProfile")?;
    let count = f
        .tables()
        .attempts
        .iter()
        .filter(|a| a.profile_id.as_deref() == Some(profile_id) && a.at >= since)
        .count();
    Ok(count as i64)
}

/// When this profile's oldest attempt at or after `since` was made (epoch ms), or `None` when it
/// made none (R192).
pub fn codes_oldest_attempt_at_by_profile(
    f: &mut FakeTx<'_>,
    profile_id: &str,
    since: i64,
) -> Result<Option<i64>, StoreError> {
    call(f, "codes.oldestAttemptAtByProfile")?;
    Ok(f.tables()
        .attempts
        .iter()
        .filter(|a| a.profile_id.as_deref() == Some(profile_id) && a.at >= since)
        .map(|a| a.at)
        .min())
}

pub fn codes_count_attempts_by_ip(f: &mut FakeTx<'_>, ip_hash: &str, since: i64) -> Result<i64, StoreError> {
    call(f, "codes.countAttemptsByIp")?;
    let count = f
        .tables()
        .attempts
        .iter()
        .filter(|a| a.ip_hash == ip_hash && a.at >= since)
        .count();
    Ok(count as i64)
}

/// §9.4: the server-side circuit breaker's input — system-wide failures in a window (R106).
pub fn codes_count_failures(f: &mut FakeTx<'_>, since: i64) -> Result<i64, StoreError> {
    call(f, "codes.countFailures")?;
    let count = f
        .tables()
        .attempts
        .iter()
        .filter(|a| a.result == CodeAttemptResult::Rejected && a.at >= since)
        .count();
    Ok(count as i64)
}

// ---------------------------------------------------------------------------
// Collection (§9.4's entitlement ledger)
// ---------------------------------------------------------------------------

pub fn collection_get(f: &mut FakeTx<'_>, profile_id: &str) -> Result<Vec<CollectionEntry>, StoreError> {
    call(f, "collection.get")?;
    Ok(f.tables()
        .collection
        .iter()
        .filter(|row| row.profile_id == profile_id)
        .map(|row| CollectionEntry {
            card_id: row.card_id.clone(),
            quantity: row.quantity,
        })
        .collect())
}

/// Sets the absolute quantity, never adds to it (ports.ts).
pub fn collection_upsert_quantities(
    f: &mut FakeTx<'_>,
    profile_id: &str,
    entries: &[CollectionEntry],
) -> Result<(), StoreError> {
    call(f, "collection.upsertQuantities")?;
    let tables = f.tables();
    for entry in entries {
        match collection_row(tables, profile_id, &entry.card_id) {
            Some(row) => row.quantity = entry.quantity,
            None => tables.collection.push(CollectionRow {
                profile_id: profile_id.to_string(),
                card_id: entry.card_id.clone(),
                quantity: entry.quantity,
            }),
        }
    }
    Ok(())
}

/// Append-only (§9.4). Each row's `delta` is the change, so the ledger sums to the total.
pub fn collection_append_grants(f: &mut FakeTx<'_>, grants: &[CollectionGrant]) -> Result<(), StoreError> {
    call(f, "collection.appendGrants")?;
    for grant in grants {
        // `collection_grants.delta <> 0` in migration 0002.
        if grant.delta == 0 {
            return Err(StoreError::from(
                "collection_grants.delta must not be 0".to_string(),
            ));
        }
    }
    f.tables().grants.extend(grants.iter().cloned());
    Ok(())
}

// ---------------------------------------------------------------------------
// Matches (§9.3, §9.5)
// ---------------------------------------------------------------------------

pub fn matches_create(f: &mut FakeTx<'_>, row: &MatchRow) -> Result<(), StoreError> {
    call(f, "matches.create")?;
    if f.tables().matches.iter().any(|existing| existing.id == row.id) {
        return Err(StoreError::from(format!("matches.id is unique: {}", row.id)));
    }
    f.tables().matches.push(row.clone());
    Ok(())
}

pub fn matches_get(f: &mut FakeTx<'_>, match_id: &str) -> Result<Option<MatchRow>, StoreError> {
    call(f, "matches.get")?;
    Ok(f.tables().matches.iter().find(|row| row.id == match_id).cloned())
}

/// Append-only (§9.3). Rejects a seq that already exists.
pub fn matches_append_actions(f: &mut FakeTx<'_>, rows: &[MatchActionRow]) -> Result<(), StoreError> {
    call(f, "matches.appendActions")?;
    let tables = f.tables();
    for row in rows {
        let clash = tables
            .match_actions
            .iter()
            .any(|existing| existing.match_id == row.match_id && existing.seq == row.seq);
        if clash {
            return Err(StoreError::from(format!(
                "match_actions is append-only: seq {} exists",
                row.seq
            )));
        }
        tables.match_actions.push(row.clone());
    }
    Ok(())
}

pub fn matches_actions(f: &mut FakeTx<'_>, match_id: &str) -> Result<Vec<MatchActionRow>, StoreError> {
    call(f, "matches.actions")?;
    let mut rows: Vec<MatchActionRow> = f
        .tables()
        .match_actions
        .iter()
        .filter(|row| row.match_id == match_id)
        .cloned()
        .collect();
    rows.sort_by_key(|a| a.seq);
    Ok(rows)
}

pub fn matches_set_clocks(
    f: &mut FakeTx<'_>,
    match_id: &str,
    clocks: &MatchClocks,
) -> Result<(), StoreError> {
    call(f, "matches.setClocks")?;
    let Some(row) = f.tables().matches.iter_mut().find(|row| row.id == match_id) else {
        return Err(StoreError::from(format!("no match {match_id}")));
    };
    row.clocks = clocks.clone();
    Ok(())
}

pub fn matches_finish(f: &mut FakeTx<'_>, match_id: &str, at: i64) -> Result<(), StoreError> {
    call(f, "matches.finish")?;
    let Some(row) = f.tables().matches.iter_mut().find(|row| row.id == match_id) else {
        return Err(StoreError::from(format!("no match {match_id}")));
    };
    row.status = MatchStatus::Finished;
    row.finished_at = Some(at);
    Ok(())
}

/// For the reaper (§9.5).
pub fn matches_live(f: &mut FakeTx<'_>) -> Result<Vec<MatchRow>, StoreError> {
    call(f, "matches.live")?;
    Ok(f.tables()
        .matches
        .iter()
        .filter(|row| row.status == MatchStatus::Live)
        .cloned()
        .collect())
}

/// R257, R376: the mode a match was made in ([`match_mode_in`]).
pub fn matches_mode_of(f: &mut FakeTx<'_>, match_id: &str) -> Result<Option<QueueMode>, StoreError> {
    call(f, "matches.modeOf")?;
    Ok(match_mode_in(f.tables(), match_id))
}

/// No `open` rows here: a reserved match id is only an id until the registry creates it (R263).
pub fn matches_discard_open(f: &mut FakeTx<'_>, _match_id: &str) -> Result<(), StoreError> {
    call(f, "matches.discardOpen")?;
    Ok(())
}

/// R679: the voided match goes as if it never existed.
pub fn matches_forget_voided(f: &mut FakeTx<'_>, match_id: &str) -> Result<(), StoreError> {
    call(f, "matches.forgetVoided")?;
    forget_voided_rows(f.tables(), match_id);
    Ok(())
}

// ---------------------------------------------------------------------------
// Rooms (§9.5)
// ---------------------------------------------------------------------------

/// False when the code is already taken.
pub fn rooms_create(f: &mut FakeTx<'_>, room: &Room) -> Result<bool, StoreError> {
    call(f, "rooms.create")?;
    if f.tables().rooms.iter().any(|existing| existing.code == room.code) {
        return Ok(false);
    }
    f.tables().rooms.push(room.clone());
    Ok(true)
}

pub fn rooms_get(f: &mut FakeTx<'_>, code: &str) -> Result<Option<Room>, StoreError> {
    call(f, "rooms.get")?;
    Ok(f.tables().rooms.iter().find(|room| room.code == code).cloned())
}

/// The atomic single-claim: the loser of a join race gets `None`, never a second match.
pub fn rooms_claim(
    f: &mut FakeTx<'_>,
    code: &str,
    guest_profile_id: &str,
    match_id: &str,
    at: i64,
) -> Result<Option<Room>, StoreError> {
    call(f, "rooms.claim")?;
    let Some(row) = f.tables().rooms.iter_mut().find(|room| room.code == code) else {
        return Ok(None);
    };
    if row.guest_profile_id.is_some() {
        return Ok(None);
    }
    if row.expires_at <= at {
        return Ok(None);
    }
    row.guest_profile_id = Some(guest_profile_id.to_string());
    row.match_id = Some(match_id.to_string());
    Ok(Some(row.clone()))
}

// ---------------------------------------------------------------------------
// Tickets (§9.5)
// ---------------------------------------------------------------------------

pub fn tickets_insert(f: &mut FakeTx<'_>, ticket: &Ticket) -> Result<(), StoreError> {
    call(f, "tickets.insert")?;
    let queued = f
        .tables()
        .tickets
        .iter()
        .any(|row| row.profile_id == ticket.profile_id && row.status == TicketStatus::Open);
    if queued {
        return Err(StoreError::from(
            "tickets_profile_queued_key: this profile is already queued".to_string(),
        ));
    }
    f.tables().tickets.push(ticket.clone());
    Ok(())
}

pub fn tickets_get(f: &mut FakeTx<'_>, ticket_id: &str) -> Result<Option<Ticket>, StoreError> {
    call(f, "tickets.get")?;
    Ok(f.tables()
        .tickets
        .iter()
        .find(|ticket| ticket.id == ticket_id)
        .cloned())
}

pub fn tickets_open_for_profile(f: &mut FakeTx<'_>, profile_id: &str) -> Result<Option<Ticket>, StoreError> {
    call(f, "tickets.openForProfile")?;
    Ok(f.tables()
        .tickets
        .iter()
        .find(|ticket| ticket.profile_id == profile_id && ticket.status == TicketStatus::Open)
        .cloned())
}

pub fn tickets_list_open(f: &mut FakeTx<'_>) -> Result<Vec<Ticket>, StoreError> {
    call(f, "tickets.listOpen")?;
    let mut rows: Vec<Ticket> = f
        .tables()
        .tickets
        .iter()
        .filter(|ticket| ticket.status == TicketStatus::Open)
        .cloned()
        .collect();
    rows.sort_by_key(|a| a.enqueued_at);
    Ok(rows)
}

pub fn tickets_count_open(f: &mut FakeTx<'_>) -> Result<i64, StoreError> {
    call(f, "tickets.countOpen")?;
    Ok(f.tables()
        .tickets
        .iter()
        .filter(|ticket| ticket.status == TicketStatus::Open)
        .count() as i64)
}

/// R257: open tickets per mode, for the lobby's per-mode population. Every mode is present, in TS's
/// order (`bo1`, `bo3`, `random`).
pub fn tickets_count_open_by_mode(f: &mut FakeTx<'_>) -> Result<PerMode<i64>, StoreError> {
    call(f, "tickets.countOpenByMode")?;
    let mut counts: PerMode<i64> = PerMode::default();
    for ticket in &f.tables().tickets {
        if ticket.status == TicketStatus::Open {
            counts[ticket.mode] += 1;
        }
    }
    Ok(counts)
}

/// §9.5: "both tickets are claimed in one atomic statement".
pub fn tickets_claim_pair(
    f: &mut FakeTx<'_>,
    a_id: &str,
    b_id: &str,
    match_id: &str,
    _at: i64,
) -> Result<bool, StoreError> {
    call(f, "tickets.claimPair")?;
    if a_id == b_id {
        return Ok(false);
    }
    let tables = f.tables();
    let a = tables.tickets.iter().position(|ticket| ticket.id == a_id);
    let b = tables.tickets.iter().position(|ticket| ticket.id == b_id);
    let (Some(a), Some(b)) = (a, b) else {
        return Ok(false);
    };
    // §9.5: one atomic statement. Either both were open or nothing changes.
    if tables.tickets[a].status != TicketStatus::Open || tables.tickets[b].status != TicketStatus::Open {
        return Ok(false);
    }
    for at in [a, b] {
        let ticket = &mut tables.tickets[at];
        ticket.status = TicketStatus::Matched;
        ticket.match_id = Some(match_id.to_string());
    }
    Ok(true)
}

pub fn tickets_cancel(f: &mut FakeTx<'_>, ticket_id: &str, _at: i64) -> Result<(), StoreError> {
    call(f, "tickets.cancel")?;
    let Some(row) = f
        .tables()
        .tickets
        .iter_mut()
        .find(|ticket| ticket.id == ticket_id)
    else {
        return Ok(());
    };
    if row.status != TicketStatus::Open {
        return Ok(());
    }
    row.status = TicketStatus::Cancelled;
    Ok(())
}

// ---------------------------------------------------------------------------
// Results (§9.5)
// ---------------------------------------------------------------------------

/// One row per match (§9.5). Rejects a second row for the same match with `StoreError::Duplicate`
/// (TS `DuplicateResultError`).
pub fn results_insert(f: &mut FakeTx<'_>, row: &ResultRow) -> Result<(), StoreError> {
    call(f, "results.insert")?;
    if f.tables()
        .results
        .iter()
        .any(|existing| existing.match_id == row.match_id)
    {
        return Err(duplicate_result(&row.match_id));
    }
    f.tables().results.push(row.clone());
    Ok(())
}

pub fn results_get_by_match(f: &mut FakeTx<'_>, match_id: &str) -> Result<Option<ResultRow>, StoreError> {
    call(f, "results.getByMatch")?;
    Ok(f.tables()
        .results
        .iter()
        .find(|result| result.match_id == match_id)
        .cloned())
}

/// Counted as the Postgres store counts it: a winnerless row is a draw (§9.5).
pub fn results_record_for(f: &mut FakeTx<'_>, profile_id: &str) -> Result<ProfileRecord, StoreError> {
    call(f, "results.recordFor")?;
    let mine: Vec<&ResultRow> = f
        .tables()
        .results
        .iter()
        .filter(|row| row.players.0 == profile_id || row.players.1 == profile_id)
        .collect();
    let wins = mine
        .iter()
        .filter(|row| row.winner_profile_id.as_deref() == Some(profile_id))
        .count();
    let losses = mine
        .iter()
        .filter(|row| {
            row.winner_profile_id
                .as_deref()
                .is_some_and(|winner| winner != profile_id)
        })
        .count();
    let draws = mine.iter().filter(|row| row.winner_profile_id.is_none()).count();
    Ok(ProfileRecord {
        wins: wins as i64,
        losses: losses as i64,
        draws: draws as i64,
    })
}

// ===========================================================================
// memory-stores.ts: the halves both in-memory stores shared
// ===========================================================================

// ---------------------------------------------------------------------------
// Saved decks, trios and the Conquest series (R250–R263)
// ---------------------------------------------------------------------------

/// Oldest first, ties on id: the order `decks_list` and `trios_list` promise.
fn by_creation(a_created: i64, a_id: &str, b_created: i64, b_id: &str) -> std::cmp::Ordering {
    a_created.cmp(&b_created).then_with(|| a_id.cmp(b_id))
}

/// A profile's decks, oldest first: `created_at`, then `id`.
pub fn decks_list(f: &mut FakeTx<'_>, profile_id: &str) -> Result<Vec<SavedDeck>, StoreError> {
    call(f, "decks.list")?;
    let mut rows: Vec<SavedDeck> = f
        .tables()
        .decks
        .iter()
        .filter(|deck| deck.profile_id == profile_id)
        .cloned()
        .collect();
    rows.sort_by(|a, b| by_creation(a.created_at, &a.id, b.created_at, &b.id));
    Ok(rows)
}

/// One deck by id, whoever owns it; the caller checks `profile_id`.
pub fn decks_get(f: &mut FakeTx<'_>, deck_id: &str) -> Result<Option<SavedDeck>, StoreError> {
    call(f, "decks.get")?;
    Ok(f.tables().decks.iter().find(|deck| deck.id == deck_id).cloned())
}

/// Inserts a deck whose id is new, or replaces `name`, `cards`, `portrait`, `catalog_version` and
/// `updated_at` of the profile's own deck (its `created_at` is kept).
pub fn decks_upsert(
    f: &mut FakeTx<'_>,
    deck: &SavedDeck,
    max_decks: i64,
) -> Result<UpsertOutcome, StoreError> {
    let max_decks = usize::try_from(max_decks).unwrap_or(0);
    call(f, "decks.upsert")?;
    let rows = &mut f.tables().decks;
    if let Some(existing) = rows.iter_mut().find(|row| row.id == deck.id) {
        if existing.profile_id != deck.profile_id {
            return Ok(UpsertOutcome::NotOwner);
        }
        existing.name = deck.name.clone();
        existing.cards = deck.cards.clone();
        existing.portrait = deck.portrait.clone();
        existing.catalog_version = deck.catalog_version.clone();
        existing.updated_at = deck.updated_at;
        return Ok(UpsertOutcome::Updated);
    }
    let count = rows
        .iter()
        .filter(|row| row.profile_id == deck.profile_id)
        .count();
    if count >= max_decks {
        return Ok(UpsertOutcome::Limit);
    }
    rows.push(deck.clone());
    Ok(UpsertOutcome::Created)
}

/// Deletes the profile's own deck; every trio slot that named it becomes `None` (R252).
pub fn decks_remove(f: &mut FakeTx<'_>, profile_id: &str, deck_id: &str) -> Result<bool, StoreError> {
    call(f, "decks.remove")?;
    let t = f.tables();
    let Some(at) = t
        .decks
        .iter()
        .position(|deck| deck.id == deck_id && deck.profile_id == profile_id)
    else {
        return Ok(false);
    };
    t.decks.remove(at);
    // `on delete set null (deckN_id)`: the trio keeps its other slots and its place.
    for trio in t.trios.iter_mut() {
        if trio.profile_id != profile_id {
            continue;
        }
        for slot in [&mut trio.deck_ids.0, &mut trio.deck_ids.1, &mut trio.deck_ids.2] {
            if slot.as_deref() == Some(deck_id) {
                *slot = None;
            }
        }
    }
    Ok(true)
}

/// A profile's trios, oldest first: `created_at`, then `id`.
pub fn trios_list(f: &mut FakeTx<'_>, profile_id: &str) -> Result<Vec<SavedTrio>, StoreError> {
    call(f, "trios.list")?;
    let mut rows: Vec<SavedTrio> = f
        .tables()
        .trios
        .iter()
        .filter(|trio| trio.profile_id == profile_id)
        .cloned()
        .collect();
    rows.sort_by(|a, b| by_creation(a.created_at, &a.id, b.created_at, &b.id));
    Ok(rows)
}

pub fn trios_get(f: &mut FakeTx<'_>, trio_id: &str) -> Result<Option<SavedTrio>, StoreError> {
    call(f, "trios.get")?;
    Ok(f.tables().trios.iter().find(|trio| trio.id == trio_id).cloned())
}

/// As `decks_upsert`, plus `unknown_deck` when a non-null slot names a deck that is not this
/// profile's; one deck in two slots is refused by constraint.
pub fn trios_upsert(
    f: &mut FakeTx<'_>,
    trio: &SavedTrio,
    max_trios: i64,
) -> Result<TrioUpsertOutcome, StoreError> {
    let max_trios = usize::try_from(max_trios).unwrap_or(0);
    call(f, "trios.upsert")?;
    let t = f.tables();
    let existing = t.trios.iter().position(|row| row.id == trio.id);
    if let Some(at) = existing
        && t.trios[at].profile_id != trio.profile_id
    {
        return Ok(TrioUpsertOutcome::NotOwner);
    }

    let filled: Vec<&String> = [&trio.deck_ids.0, &trio.deck_ids.1, &trio.deck_ids.2]
        .into_iter()
        .flatten()
        .collect();
    let distinct: IndexSet<&String> = filled.iter().copied().collect();
    if distinct.len() != filled.len() {
        return Err(StoreError::from(
            "trios_decks_distinct: a trio cannot hold the same deck twice".to_string(),
        ));
    }
    let mine = |deck_id: &String| {
        t.decks
            .iter()
            .any(|deck| &deck.id == deck_id && deck.profile_id == trio.profile_id)
    };
    if !filled.iter().all(|&deck_id| mine(deck_id)) {
        return Ok(TrioUpsertOutcome::UnknownDeck);
    }

    if let Some(at) = existing {
        let row = &mut t.trios[at];
        row.name = trio.name.clone();
        row.deck_ids = trio.deck_ids.clone();
        row.updated_at = trio.updated_at;
        return Ok(TrioUpsertOutcome::Updated);
    }
    let count = t
        .trios
        .iter()
        .filter(|row| row.profile_id == trio.profile_id)
        .count();
    if count >= max_trios {
        return Ok(TrioUpsertOutcome::Limit);
    }
    t.trios.push(trio.clone());
    Ok(TrioUpsertOutcome::Created)
}

pub fn trios_remove(f: &mut FakeTx<'_>, profile_id: &str, trio_id: &str) -> Result<bool, StoreError> {
    call(f, "trios.remove")?;
    let t = f.tables();
    let Some(at) = t
        .trios
        .iter()
        .position(|trio| trio.id == trio_id && trio.profile_id == profile_id)
    else {
        return Ok(false);
    };
    t.trios.remove(at);
    Ok(true)
}

pub fn series_create(f: &mut FakeTx<'_>, row: &SeriesRow) -> Result<(), StoreError> {
    call(f, "series.create")?;
    if f.tables().series.iter().any(|existing| existing.id == row.id) {
        return Err(StoreError::from(format!("series.id is unique: {}", row.id)));
    }
    f.tables().series.push(row.clone());
    Ok(())
}

pub fn series_get(f: &mut FakeTx<'_>, series_id: &str) -> Result<Option<SeriesRow>, StoreError> {
    call(f, "series.get")?;
    Ok(f.tables()
        .series
        .iter()
        .find(|existing| existing.id == series_id)
        .cloned())
}

/// Compare-and-set: writes `next` only when the stored row's `version` is `next.version - 1`.
pub fn series_update(f: &mut FakeTx<'_>, next: &SeriesRow) -> Result<bool, StoreError> {
    if call_cas(f, "series.update")? == CallOutcome::Lose {
        return Ok(false);
    }
    let t = f.tables();
    let Some(current) = t.series.iter_mut().find(|existing| existing.id == next.id) else {
        return Ok(false);
    };
    if current.version != next.version - 1 {
        return Ok(false);
    }
    *current = next.clone();
    Ok(true)
}

/// The series whose game in play is this match (`status = 'playing'` and `next_match_id`).
pub fn series_by_match(f: &mut FakeTx<'_>, match_id: &str) -> Result<Option<SeriesRow>, StoreError> {
    call(f, "series.byMatch")?;
    Ok(f.tables()
        .series
        .iter()
        .find(|existing| existing.status == SeriesStatus::Playing && existing.next_match_id == match_id)
        .cloned())
}

/// The series one of whose games was played (or is being played) as this match, whatever its status.
pub fn series_with_game(f: &mut FakeTx<'_>, match_id: &str) -> Result<Option<SeriesRow>, StoreError> {
    call(f, "series.withGame")?;
    Ok(f.tables()
        .series
        .iter()
        .find(|existing| existing.games.iter().any(|game| game.match_id == match_id))
        .cloned())
}

/// The profile's series that is not over. A profile is in at most one.
pub fn series_active_for(f: &mut FakeTx<'_>, profile_id: &str) -> Result<Option<SeriesRow>, StoreError> {
    call(f, "series.activeFor")?;
    Ok(f.tables()
        .series
        .iter()
        .find(|existing| {
            existing.status != SeriesStatus::Over
                && (existing.sides.0.profile_id == profile_id || existing.sides.1.profile_id == profile_id)
        })
        .cloned())
}

/// Every series that is not over: the sweeper's input (R263), oldest first.
pub fn series_active(f: &mut FakeTx<'_>) -> Result<Vec<SeriesRow>, StoreError> {
    call(f, "series.active")?;
    let mut rows: Vec<SeriesRow> = f
        .tables()
        .series
        .iter()
        .filter(|existing| existing.status != SeriesStatus::Over)
        .cloned()
        .collect();
    rows.sort_by(|a, b| by_creation(a.created_at, &a.id, b.created_at, &b.id));
    Ok(rows)
}

// ---------------------------------------------------------------------------
// Tutorial progress on the account (SPEC §9.10, R320)
// ---------------------------------------------------------------------------

/// R320's merge, exactly as `app.merge_tutorial_progress` (0011) makes it: the lessons become the
/// union of the stored and the sent, each once in code-point order (`collate "C"` in Postgres), and
/// the stored choice is replaced only by a strictly newer one. `None` (TS `"limit"`) when the union
/// would pass `max_lessons`; nothing changes then.
pub fn merge_tutorial_row(
    existing: Option<&TutorialProgressRow>,
    input: &TutorialMergeInput,
    max_lessons: usize,
) -> Option<TutorialProgressRow> {
    let mut completed: Vec<String> = existing
        .map(|row| row.completed.clone())
        .unwrap_or_default()
        .into_iter()
        .chain(input.completed.iter().cloned())
        .collect::<IndexSet<String>>()
        .into_iter()
        .collect();
    completed.sort();
    if completed.len() > max_lessons {
        return None;
    }
    let stored = existing.and_then(|row| row.hidden_choice);
    let incoming = input.hidden_choice;
    let hidden_choice = match (incoming, stored) {
        (Some(incoming), None) => Some(incoming),
        (Some(incoming), Some(stored)) if incoming.at > stored.at => Some(incoming),
        (_, stored) => stored,
    };
    Some(TutorialProgressRow {
        profile_id: input.profile_id.clone(),
        completed,
        hidden_choice,
    })
}

/// The profile's row, or `None` before its first write. Laxer than Postgres in two places, both
/// listed in `src/db/pg.rs`'s KNOWN DIVERGENCES: it writes a row for a profile that is not active,
/// and it does not re-check an id's shape (the handler has).
pub fn tutorial_get(f: &mut FakeTx<'_>, profile_id: &str) -> Result<Option<TutorialProgressRow>, StoreError> {
    call(f, "tutorial.get")?;
    Ok(f.tables()
        .tutorial
        .iter()
        .find(|existing| existing.profile_id == profile_id)
        .cloned())
}

/// R320: one atomic merge ([`merge_tutorial_row`]). A profile with no row gets one.
pub fn tutorial_merge(
    f: &mut FakeTx<'_>,
    input: &TutorialMergeInput,
    max_lessons: i64,
) -> Result<TutorialMergeOutcome, StoreError> {
    let max_lessons = usize::try_from(max_lessons).unwrap_or(0);
    call(f, "tutorial.merge")?;
    let rows = &mut f.tables().tutorial;
    let at = rows
        .iter()
        .position(|existing| existing.profile_id == input.profile_id);
    let Some(merged) = merge_tutorial_row(at.map(|i| &rows[i]), input, max_lessons) else {
        return Ok(TutorialMergeOutcome::Limit);
    };
    match at {
        None => rows.push(merged.clone()),
        Some(i) => rows[i] = merged.clone(),
    }
    Ok(TutorialMergeOutcome::Merged { progress: merged })
}

// ---------------------------------------------------------------------------
// Player settings on the account (SPEC §9.1, R633, R634)
// ---------------------------------------------------------------------------

/// `JSON.stringify` of a JSON scalar, numbers as JavaScript prints them: an integral number without
/// a fraction (`1`, never serde's `1.0`), so the byte count is TS's.
fn js_scalar(node: &Value) -> String {
    match node {
        Value::Number(number) => {
            if let Some(i) = number.as_i64() {
                i.to_string()
            } else if let Some(u) = number.as_u64() {
                u.to_string()
            } else {
                let x = number.as_f64().unwrap_or(0.0);
                if x == 0.0 {
                    "0".to_string()
                } else if x.fract() == 0.0 && x.abs() < 1e21 {
                    format!("{x:.0}")
                } else {
                    x.to_string()
                }
            }
        }
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

/// The size Postgres measures for the byte cap: `octet_length(groups::text)`. jsonb prints an object
/// with its keys shorter first and then bytewise, a space after each colon and comma, so the
/// in-memory store prints the same text to count the same bytes.
pub fn jsonb_text_bytes(value: &Value) -> usize {
    fn text(node: &Value) -> String {
        match node {
            Value::Array(items) => {
                format!("[{}]", items.iter().map(text).collect::<Vec<String>>().join(", "))
            }
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.as_bytes().cmp(b.as_bytes())));
                let pairs: Vec<String> = keys
                    .iter()
                    .map(|key| {
                        format!(
                            "{}: {}",
                            serde_json::to_string(key).unwrap_or_default(),
                            text(&map[key.as_str()])
                        )
                    })
                    .collect();
                format!("{{{}}}", pairs.join(", "))
            }
            scalar => js_scalar(scalar),
        }
    }
    text(value).len()
}

/// R634's merge, exactly as `app.merge_player_settings` (0018) makes it: each group sent replaces
/// the stored group only when its time is strictly later (the stored one on a tie), and a group the
/// write does not name stays. `None` (TS `"limit"`) when the result would pass a cap; nothing
/// changes then.
pub fn merge_player_settings_row(
    existing: Option<&PlayerSettingsRow>,
    input: &PlayerSettingsMergeInput,
    limits: &PlayerSettingsLimits,
) -> Option<PlayerSettingsRow> {
    let mut groups = existing.map(|row| row.groups.clone()).unwrap_or_default();
    for (id, sent) in &input.groups {
        let replace = match groups.get(id) {
            None => true,
            Some(held) => sent.at > held.at,
        };
        if replace {
            groups.insert(id.clone(), sent.clone());
        }
    }
    let bytes = jsonb_text_bytes(&serde_json::to_value(&groups).unwrap_or(Value::Null));
    if groups.len() as i64 > limits.max_groups || bytes as i64 > limits.max_bytes {
        return None;
    }
    Some(PlayerSettingsRow {
        profile_id: input.profile_id.clone(),
        groups,
    })
}

/// The profile's row, or `None` before its first write. Laxer than Postgres in two places, both
/// listed in `src/db/pg.rs`'s KNOWN DIVERGENCES: it writes a row for a profile that is not active,
/// and it does not re-check a group's shape (the handler has).
pub fn player_settings_get(
    f: &mut FakeTx<'_>,
    profile_id: &str,
) -> Result<Option<PlayerSettingsRow>, StoreError> {
    call(f, "playerSettings.get")?;
    Ok(f.tables()
        .player_settings
        .iter()
        .find(|existing| existing.profile_id == profile_id)
        .cloned())
}

/// R634: one atomic merge ([`merge_player_settings_row`]). A profile with no row gets one.
pub fn player_settings_merge(
    f: &mut FakeTx<'_>,
    input: &PlayerSettingsMergeInput,
    limits: &PlayerSettingsLimits,
) -> Result<PlayerSettingsMergeOutcome, StoreError> {
    call(f, "playerSettings.merge")?;
    let rows = &mut f.tables().player_settings;
    let at = rows
        .iter()
        .position(|existing| existing.profile_id == input.profile_id);
    let Some(merged) = merge_player_settings_row(at.map(|i| &rows[i]), input, limits) else {
        return Ok(PlayerSettingsMergeOutcome::Limit);
    };
    match at {
        None => rows.push(merged.clone()),
        Some(i) => rows[i] = merged.clone(),
    }
    Ok(PlayerSettingsMergeOutcome::Merged { settings: merged })
}

// ---------------------------------------------------------------------------
// Last boards (C+ #29 Portal to the Past, R417, R565)
// ---------------------------------------------------------------------------

/// The profile's last board of this kind, or `None` before its first finished game of it.
pub fn last_boards_get(
    f: &mut FakeTx<'_>,
    profile_id: &str,
    kind: LastBoardKind,
) -> Result<Option<Vec<LastBoardEntry>>, StoreError> {
    call(f, "lastBoards.get")?;
    Ok(f.tables()
        .last_boards
        .iter()
        .find(|row| row.profile_id == profile_id && row.kind == kind)
        .map(|row| row.board.clone()))
}

/// R565: replace it, or write the first one, as a game of that kind ends.
pub fn last_boards_put(
    f: &mut FakeTx<'_>,
    profile_id: &str,
    kind: LastBoardKind,
    board: &[LastBoardEntry],
    _at: i64,
) -> Result<(), StoreError> {
    call(f, "lastBoards.put")?;
    let rows = &mut f.tables().last_boards;
    match rows
        .iter_mut()
        .find(|row| row.profile_id == profile_id && row.kind == kind)
    {
        Some(row) => row.board = board.to_vec(),
        None => rows.push(LastBoardRow {
            profile_id: profile_id.to_string(),
            kind,
            board: board.to_vec(),
        }),
    }
    Ok(())
}

/// R678: no randomness here (the in-memory stores draw nothing), so the first boards in table
/// order; Postgres draws them at random. The contract asserts only what both hold.
pub fn last_boards_sample_others(
    f: &mut FakeTx<'_>,
    exclude_profile_ids: &[String],
    count: i64,
) -> Result<Vec<Vec<LastBoardEntry>>, StoreError> {
    let count = usize::try_from(count).unwrap_or(0);
    call(f, "lastBoards.sampleOthers")?;
    let excluded: IndexSet<&String> = exclude_profile_ids.iter().collect();
    Ok(f.tables()
        .last_boards
        .iter()
        .filter(|row| {
            row.kind == LastBoardKind::Server && !row.board.is_empty() && !excluded.contains(&row.profile_id)
        })
        .take(count)
        .map(|row| row.board.clone())
        .collect())
}

// ---------------------------------------------------------------------------
// Game records for the card statistics (SPEC §9.11, R376)
// ---------------------------------------------------------------------------

/// The match store's `modeOf` for the in-memory store, as Postgres answers it: a game of a Conquest
/// series is `bo3`, a room's match has the room's mode, and a queue match its tickets' mode.
pub fn match_mode_in(tables: &FakeTables, match_id: &str) -> Option<QueueMode> {
    if tables
        .series
        .iter()
        .any(|row| row.games.iter().any(|game| game.match_id == match_id))
    {
        return Some(QueueMode::Bo3);
    }
    // R672: a rematch states its own mode, since no ticket, room or series made it.
    if let Some(own) = tables
        .matches
        .iter()
        .find(|row| row.id == match_id)
        .and_then(|row| row.mode)
    {
        return Some(own);
    }
    if let Some(room) = tables
        .rooms
        .iter()
        .find(|row| row.match_id.as_deref() == Some(match_id))
    {
        return Some(room.mode);
    }
    tables
        .tickets
        .iter()
        .find(|row| row.match_id.as_deref() == Some(match_id))
        .map(|row| row.mode)
}

/// R376: one record per id, as the primary key makes it. False, and nothing written, when a record
/// with this id exists.
pub fn game_records_insert(f: &mut FakeTx<'_>, record: &GameRecord) -> Result<bool, StoreError> {
    call(f, "gameRecords.insert")?;
    // R378: `game_records_dev_id_check` (0014). A development id begins "dev:", a live one never.
    if (record.source == GameSource::Dev) != record.id.starts_with(DEV_RECORD_ID_PREFIX) {
        return Err(StoreError::from(format!(
            "game_records_dev_id_check: a {} record cannot have the id {}",
            record.source, record.id
        )));
    }
    let rows = &mut f.tables().game_records;
    if rows.iter().any(|existing| existing.id == record.id) {
        return Ok(false);
    }
    rows.push(record.clone());
    Ok(true)
}

/// The records of the query's sources, mode and patch, in id order (code points), filtered exactly
/// as `card_stats` does.
pub fn game_records_list(f: &mut FakeTx<'_>, query: &GameRecordQuery) -> Result<Vec<GameRecord>, StoreError> {
    call(f, "gameRecords.list")?;
    let filter = CardStatsFilter {
        source: query.source,
        mode: query.mode,
        patch: query.patch.clone(),
        pilot: PilotFilter::Unified,
    };
    let mut rows: Vec<GameRecord> = f
        .tables()
        .game_records
        .iter()
        .filter(|record| record_matches(record, &filter))
        .cloned()
        .collect();
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(rows)
}

// ---------------------------------------------------------------------------
// Player statistics on the account (SPEC §9.11, R639, R654)
// ---------------------------------------------------------------------------

/// A counter as the summary counts it: a positive safe integer, or 0 (TS `typeof value === "number"
/// && Number.isSafeInteger(value) && value > 0`).
fn count_number(value: Option<&Value>) -> i64 {
    const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
    match value.and_then(Value::as_f64) {
        Some(x) if x.fract() == 0.0 && x.abs() <= MAX_SAFE_INTEGER && x > 0.0 => x as i64,
        _ => 0,
    }
}

/// `Object.entries` of a stats bag's member: an object's members, an array's items under their
/// index, anything else nothing (TS `typeof … === "object" && … !== null`).
fn object_entries(value: Option<&Value>) -> Vec<(String, &Value)> {
    match value {
        Some(Value::Object(map)) => map.iter().map(|(key, item)| (key.clone(), item)).collect(),
        Some(Value::Array(items)) => items
            .iter()
            .enumerate()
            .map(|(i, item)| (i.to_string(), item))
            .collect(),
        _ => Vec::new(),
    }
}

/// A profile's public summary, read off its stats bag (R654).
pub fn to_public_player_summary(
    profile_id: &str,
    username: &str,
    stats: &IndexMap<String, Value>,
    updated_at: i64,
) -> PublicPlayerSummary {
    let games = count_number(stats.get("games"));
    let wins = count_number(stats.get("wins"));
    let losses = count_number(stats.get("losses"));
    let draws = count_number(stats.get("draws"));
    let win_rate = if games == 0 {
        None
    } else {
        Some(wins as f64 / games as f64)
    };

    let mut played_counts: Vec<FavouriteCard> = Vec::new();
    let mut nemesis_counts: Vec<FavouriteCard> = Vec::new();
    let mut total_destroyed = 0;
    let mut total_defeated = 0;

    for (id, counters) in object_entries(stats.get("cards")) {
        if !(counters.is_object() || counters.is_array()) {
            continue;
        }
        let played = count_number(counters.get("played"));
        let played_against = count_number(counters.get("playedAgainst"));
        let destroyed = count_number(counters.get("destroyed"));
        let defeated = count_number(counters.get("defeated"));

        if played > 0 {
            played_counts.push(FavouriteCard {
                id: id.clone(),
                count: played,
            });
        }
        if played_against > 0 {
            nemesis_counts.push(FavouriteCard {
                id: id.clone(),
                count: played_against,
            });
        }
        total_destroyed += destroyed;
        total_defeated += defeated;
    }

    played_counts.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.id.cmp(&b.id)));
    nemesis_counts.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.id.cmp(&b.id)));

    PublicPlayerSummary {
        profile_id: profile_id.to_string(),
        username: username.to_string(),
        games,
        wins,
        losses,
        draws,
        win_rate,
        favourite_cards: played_counts.into_iter().take(3).collect(),
        fun_stats: FunStats {
            nemesis_card_id: nemesis_counts.first().map(|card| card.id.clone()),
            total_destroyed,
            total_defeated,
        },
        updated_at,
    }
}

pub fn player_stats_get(f: &mut FakeTx<'_>, profile_id: &str) -> Result<Option<PlayerStatsRow>, StoreError> {
    call(f, "playerStats.get")?;
    Ok(f.tables()
        .player_stats
        .iter()
        .find(|r| r.profile_id == profile_id)
        .map(|row| PlayerStatsRow {
            profile_id: row.profile_id.clone(),
            stats: row.stats.clone(),
            is_private: row.is_private,
            updated_at: row.updated_at,
        }))
}

pub fn player_stats_put(
    f: &mut FakeTx<'_>,
    profile_id: &str,
    stats: &IndexMap<String, Value>,
    is_private: bool,
    at: i64,
) -> Result<(), StoreError> {
    call(f, "playerStats.put")?;
    let rows = &mut f.tables().player_stats;
    match rows.iter().position(|r| r.profile_id == profile_id) {
        Some(index) => {
            let created_at = rows[index].created_at;
            rows[index] = PlayerStatsTableRow {
                profile_id: profile_id.to_string(),
                stats: stats.clone(),
                is_private,
                created_at,
                updated_at: at,
            };
        }
        None => rows.push(PlayerStatsTableRow {
            profile_id: profile_id.to_string(),
            stats: stats.clone(),
            is_private,
            created_at: at,
            updated_at: at,
        }),
    }
    Ok(())
}

pub fn player_stats_list_public(
    f: &mut FakeTx<'_>,
    options: &PlayerStatsListOptions,
) -> Result<Vec<PublicPlayerSummary>, StoreError> {
    call(f, "playerStats.listPublic")?;
    // R1436: matched by the key a name clashes on (R1434) plus its tag, as pg.rs matches it.
    let term = username_key(options.search.clone().unwrap_or_default().trim());
    let all = f.tables();
    // Each profile's username as shown, and what a search reads of it.
    let profile_map: IndexMap<&str, (String, String)> = all
        .profiles
        .iter()
        .map(|p| {
            let searched = match p.username_tag {
                Some(tag) => format!("{}#{tag}", p.username_key),
                None => p.username_key.clone(),
            };
            (p.id.as_str(), (p.username(), searched))
        })
        .collect();

    let mut public_rows: Vec<(&PlayerStatsTableRow, &str)> = all
        .player_stats
        .iter()
        .filter(|r| !r.is_private)
        .filter_map(|r| {
            let (username, searched) = profile_map.get(r.profile_id.as_str())?;
            (term.is_empty() || searched.contains(&term)).then_some((r, username.as_str()))
        })
        .collect();

    public_rows.sort_by(|a, b| {
        let games_a = count_number(a.0.stats.get("games"));
        let games_b = count_number(b.0.stats.get("games"));
        games_b
            .cmp(&games_a)
            .then_with(|| b.0.updated_at.cmp(&a.0.updated_at))
            .then_with(|| a.0.profile_id.cmp(&b.0.profile_id))
    });

    Ok(public_rows
        .into_iter()
        .skip(usize::try_from(options.offset).unwrap_or(usize::MAX))
        .take(usize::try_from(options.limit).unwrap_or(0))
        .map(|(row, username)| {
            to_public_player_summary(&row.profile_id, username, &row.stats, row.updated_at)
        })
        .collect())
}

// ---------------------------------------------------------------------------------------------
// Account deletion and the retention purge (migrations 0012 and 0013)
// ---------------------------------------------------------------------------------------------

/// Drops the rows `wanted` refuses, in place, and says how many went.
fn keep_only<T>(rows: &mut Vec<T>, wanted: impl FnMut(&T) -> bool) -> usize {
    let before = rows.len();
    rows.retain(wanted);
    before - rows.len()
}

/// The profile store's `remove`, as migration 0012 makes Postgres do it: the profile's own rows go,
/// its invite-code attempts lose their link to it, and a room it opened that nobody joined goes too.
/// Finished matches, results and series stay for the other player. Postgres empties this profile's
/// seat on them; here the id stays, since no port read of a finished match looks the seat up.
pub fn remove_profile_rows(tables: &mut FakeTables, profile_id: &str) -> bool {
    if keep_only(&mut tables.profiles, |row| row.id != profile_id) == 0 {
        return false;
    }
    for attempt in tables.attempts.iter_mut() {
        if attempt.profile_id.as_deref() == Some(profile_id) {
            attempt.profile_id = None;
        }
    }
    keep_only(&mut tables.collection, |row| row.profile_id != profile_id);
    keep_only(&mut tables.grants, |row| row.profile_id != profile_id);
    keep_only(&mut tables.decks, |row| row.profile_id != profile_id);
    keep_only(&mut tables.trios, |row| row.profile_id != profile_id);
    keep_only(&mut tables.tutorial, |row| row.profile_id != profile_id);
    keep_only(&mut tables.player_settings, |row| row.profile_id != profile_id);
    keep_only(&mut tables.last_boards, |row| row.profile_id != profile_id);
    keep_only(&mut tables.player_stats, |row| row.profile_id != profile_id);
    keep_only(&mut tables.tickets, |row| row.profile_id != profile_id);
    keep_only(&mut tables.rooms, |row| {
        !(row.host_profile_id == profile_id && row.guest_profile_id.is_none())
    });
    // 0014: a profile's season rows go with it; the record of its rated games stays for the other
    // player with this side's profile emptied, as Postgres's `on delete set null` does.
    keep_only(&mut tables.season_ranks, |row| row.profile_id != profile_id);
    for game in tables.rated_games.iter_mut() {
        for side in [&mut game.sides.0, &mut game.sides.1] {
            if side.profile_id.as_deref() == Some(profile_id) {
                side.profile_id = None;
            }
        }
    }
    true
}

/// The match store's `forgetVoided` (R679), as migration 0024's `app.forget_voided_match` makes
/// Postgres do it: a live match with no result goes with its log, a profile pointed at it is let go
/// and a ticket that paired it loses the link (`on delete set null`, 0004). Anything else is left
/// alone.
pub fn forget_voided_rows(tables: &mut FakeTables, match_id: &str) {
    let Some(row) = tables.matches.iter().find(|row| row.id == match_id) else {
        return;
    };
    if row.status != MatchStatus::Live {
        return;
    }
    if tables.results.iter().any(|row| row.match_id == match_id) {
        return;
    }
    keep_only(&mut tables.matches, |row| row.id != match_id);
    keep_only(&mut tables.match_actions, |row| row.match_id != match_id);
    for profile in tables.profiles.iter_mut() {
        if profile.in_match_id.as_deref() == Some(match_id) {
            profile.in_match_id = None;
        }
    }
    for ticket in tables.tickets.iter_mut() {
        if ticket.match_id.as_deref() == Some(match_id) {
            ticket.match_id = None;
        }
    }
}

/// The root `purgeExpired`: old attempts, and the logs of matches that ended before the cutoff.
pub fn purge_expired_rows(tables: &mut FakeTables, input: &RetentionPurgeInput) -> RetentionPurgeResult {
    let code_attempts = keep_only(&mut tables.attempts, |row| row.at >= input.code_attempts_before);
    let expired: IndexSet<String> = tables
        .matches
        .iter()
        .filter(|row| {
            row.status == MatchStatus::Finished
                && row
                    .finished_at
                    .is_some_and(|finished_at| finished_at < input.match_actions_ended_before)
        })
        .map(|row| row.id.clone())
        .collect();
    let match_actions = keep_only(&mut tables.match_actions, |row| !expired.contains(&row.match_id));
    RetentionPurgeResult {
        code_attempts: code_attempts as i64,
        match_actions: match_actions as i64,
    }
}

// ---------------------------------------------------------------------------------------------
// The ranked ladder (SPEC §9.12; migration 0022 carries it on Postgres). It reads `profiles` for
// the ratings a standing carries and the soft reset writes, as Postgres joins `public.profiles`.
// ---------------------------------------------------------------------------------------------

/// TS `byProfileId`.
fn by_profile_id(a: &str, b: &str) -> std::cmp::Ordering {
    a.cmp(b)
}

/// One process owns these tables: there is no second opener to serialize with (and the lock the
/// transaction holds serializes everything anyway).
pub fn ranked_lock_seasons(f: &mut FakeTx<'_>) -> Result<(), StoreError> {
    call(f, "ranked.lockSeasons")?;
    Ok(())
}

/// Every season, oldest first.
pub fn ranked_seasons(f: &mut FakeTx<'_>) -> Result<Vec<Season>, StoreError> {
    call(f, "ranked.seasons")?;
    let mut rows = f.tables().seasons.clone();
    rows.sort_by(|a, b| a.started_at.cmp(&b.started_at).then_with(|| a.id.cmp(&b.id)));
    Ok(rows)
}

/// False, writing nothing, when a season of that id exists already.
pub fn ranked_create_season(f: &mut FakeTx<'_>, season: &Season) -> Result<bool, StoreError> {
    call(f, "ranked.createSeason")?;
    if f.tables().seasons.iter().any(|row| row.id == season.id) {
        return Ok(false);
    }
    f.tables().seasons.push(season.clone());
    Ok(true)
}

/// R609: every profile that has played a rated game, with its hidden rating: the soft reset's
/// input. Bots are not profiles and are never in it.
pub fn ranked_rated_players(f: &mut FakeTx<'_>) -> Result<Vec<ResetPlayer>, StoreError> {
    call(f, "ranked.ratedPlayers")?;
    let tables = f.tables();
    let mut ids: IndexSet<String> = IndexSet::new();
    for game in &tables.rated_games {
        for side in [&game.sides.0, &game.sides.1] {
            if let (Some(profile_id), None) = (&side.profile_id, &side.bot_id) {
                ids.insert(profile_id.clone());
            }
        }
    }
    let mut players: Vec<ResetPlayer> = ids
        .into_iter()
        .filter_map(|profile_id| {
            let profile = tables.profiles.iter().find(|row| row.id == profile_id)?;
            Some(ResetPlayer {
                glicko: Glicko {
                    rating: profile.rating,
                    deviation: profile.rating_deviation,
                    volatility: profile.rating_volatility,
                },
                profile_id,
            })
        })
        .collect();
    players.sort_by(|a, b| by_profile_id(&a.profile_id, &b.profile_id));
    Ok(players)
}

/// R609: writes a soft reset's ratings.
pub fn ranked_reset_ratings(f: &mut FakeTx<'_>, changes: &[ResetChange]) -> Result<(), StoreError> {
    call(f, "ranked.resetRatings")?;
    let tables = f.tables();
    for change in changes {
        let Some(profile) = profile_of(tables, &change.profile_id) else {
            continue;
        };
        profile.rating = change.after.rating;
        profile.rating_deviation = change.after.deviation;
        profile.rating_volatility = change.after.volatility;
    }
    Ok(())
}

/// Every row of a season, each with the player's current rating, in profile-id order.
pub fn ranked_standings(f: &mut FakeTx<'_>, season_id: &str) -> Result<Vec<SeasonStanding>, StoreError> {
    call(f, "ranked.standings")?;
    let tables = f.tables();
    let mut rows: Vec<SeasonStanding> = tables
        .season_ranks
        .iter()
        .filter(|row| row.season_id == season_id)
        .filter_map(|row| {
            let profile = tables.profiles.iter().find(|p| p.id == row.profile_id)?;
            Some(SeasonStanding {
                rank: row.clone(),
                rating: profile.rating,
            })
        })
        .collect();
    rows.sort_by(|a, b| by_profile_id(&a.rank.profile_id, &b.rank.profile_id));
    Ok(rows)
}

pub fn ranked_rank(
    f: &mut FakeTx<'_>,
    season_id: &str,
    profile_id: &str,
) -> Result<Option<SeasonRank>, StoreError> {
    call(f, "ranked.rank")?;
    Ok(f.tables()
        .season_ranks
        .iter()
        .find(|rank| rank.season_id == season_id && rank.profile_id == profile_id)
        .cloned())
}

/// Every season row this profile has, oldest season first: the profile's badges (R607).
pub fn ranked_ranks_of(f: &mut FakeTx<'_>, profile_id: &str) -> Result<Vec<SeasonRank>, StoreError> {
    call(f, "ranked.ranksOf")?;
    let tables = f.tables();
    let order: IndexMap<&str, i64> = tables
        .seasons
        .iter()
        .map(|season| (season.id.as_str(), season.started_at))
        .collect();
    let started = |season_id: &str| order.get(season_id).copied().unwrap_or(0);
    let mut rows: Vec<SeasonRank> = tables
        .season_ranks
        .iter()
        .filter(|row| row.profile_id == profile_id)
        .cloned()
        .collect();
    rows.sort_by_key(|a| started(&a.season_id));
    Ok(rows)
}

/// Insert or replace one player's season row; `peak_jlorious` merges, keeping the better (lower).
pub fn ranked_put_rank(f: &mut FakeTx<'_>, row: &SeasonRank) -> Result<(), StoreError> {
    call(f, "ranked.putRank")?;
    let rows = &mut f.tables().season_ranks;
    match rows
        .iter_mut()
        .find(|rank| rank.season_id == row.season_id && rank.profile_id == row.profile_id)
    {
        None => rows.push(row.clone()),
        Some(existing) => {
            // Same merge as Postgres' upsert: a note_peak_jlorious that landed since the writer
            // read the row is not lost.
            let peak = existing.peak_jlorious;
            *existing = row.clone();
            existing.peak_jlorious = match (peak, row.peak_jlorious) {
                (None, written) => written,
                (Some(stored), None) => Some(stored),
                (Some(stored), Some(written)) => Some(stored.min(written)),
            };
        }
    }
    Ok(())
}

/// R608: records that a player has held this Jlorious position, keeping the best.
pub fn ranked_note_peak_jlorious(
    f: &mut FakeTx<'_>,
    season_id: &str,
    profile_id: &str,
    position: i64,
) -> Result<(), StoreError> {
    let position = i32::try_from(position).unwrap_or(i32::MAX);
    call(f, "ranked.notePeakJlorious")?;
    let Some(row) = f
        .tables()
        .season_ranks
        .iter_mut()
        .find(|rank| rank.season_id == season_id && rank.profile_id == profile_id)
    else {
        return Ok(());
    };
    row.peak_jlorious = Some(match row.peak_jlorious {
        None => position,
        Some(peak) => peak.min(position),
    });
    Ok(())
}

/// R610: a bot's rating, or `None` before its first rated game.
pub fn ranked_bot(f: &mut FakeTx<'_>, bot_id: &str) -> Result<Option<BotRating>, StoreError> {
    call(f, "ranked.bot")?;
    Ok(f.tables().bots.iter().find(|bot| bot.bot_id == bot_id).cloned())
}

pub fn ranked_put_bot(f: &mut FakeTx<'_>, bot: &BotRating) -> Result<(), StoreError> {
    call(f, "ranked.putBot")?;
    let rows = &mut f.tables().bots;
    match rows.iter().position(|row| row.bot_id == bot.bot_id) {
        None => rows.push(bot.clone()),
        Some(at) => rows[at] = bot.clone(),
    }
    Ok(())
}

/// R611: one row per rated game. Rejects a second row for the same id.
pub fn ranked_record_game(f: &mut FakeTx<'_>, row: &RatedGameRow) -> Result<(), StoreError> {
    call(f, "ranked.recordGame")?;
    if f.tables().rated_games.iter().any(|game| game.id == row.id) {
        return Err(StoreError::from(format!(
            "rated_games already holds a row for {}",
            row.id
        )));
    }
    f.tables().rated_games.push(row.clone());
    Ok(())
}

pub fn ranked_game(f: &mut FakeTx<'_>, id: &str) -> Result<Option<RatedGameRow>, StoreError> {
    call(f, "ranked.game")?;
    Ok(f.tables().rated_games.iter().find(|game| game.id == id).cloned())
}
