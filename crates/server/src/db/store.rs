//! The persistence port (SPEC §9.1, §9.2; ← `apps/server/src/api/ports.ts`, the `Store` half;
//! docs/v0.3.0/SURFACE.md §11.2). Everything under `api` and `actor` reads and writes the database
//! through the methods below and never through a driver, so the same handlers run against Postgres
//! (`pg`, `src/db/store.ts` in TS) and against the in-memory fake (`fake`, TS's `e2e-store.ts` and
//! `memory-stores.ts`) that the unit tests and `E2E=1` use.
//!
//! No traits (SURFACE §11.3): `Db` and `Tx` are enums with one variant per implementation, and every
//! TS `Store` method is one method on `Tx`, named `<substore>_<method>` snake_cased, in TS's
//! argument order, whose body dispatches to `pg::<name>` or `fake::<name>`. TS's
//! `store.tx(async (t) => …)` becomes
//!
//! ```ignore
//! let mut tx = app.db.begin(Some(profile_id)).await?;
//! // … tx.decks_upsert(&deck, MAX_SAVED_DECKS).await? …
//! tx.commit().await?;
//! ```
//!
//! and a `Tx` dropped without `commit` rolls back (Postgres: sqlx's `Transaction` drop; the fake:
//! `FakeTx` restores the snapshot it took at `begin`), which is TS's "rolls back if `fn` throws".
//! TS's nested `tx` joined the enclosing transaction; a Rust `Tx` is passed down instead, so there is
//! nothing to nest.
//!
//! What `ports.ts` held that is not here, and where it went (SURFACE §11.3):
//!
//! - `Timers`/`systemTimers` → `tokio::time` (tests: `tokio::time::pause()` and `advance()`).
//! - `Logger` → `tracing` JSON lines with the same `event` names.
//! - `Ids`/`systemIds` → `uuid` and `getrandom`; `Hashes` → functions in `api/crypto.rs`.
//! - `ServerConfig` and `ApiLimits` → the constants of `config.rs` (R79's clocks, §9.4's redemption
//!   and breaker limits, §9.5's queue windows, `ROOM_CODE_TTL_SECONDS`).
//! - `CatalogInfo` → `api::catalog::Catalog`; `LoadoutIssue`/`LoadoutValidateInput`/
//!   `LoadoutValidator` → `jackioh_engine::validator`, which the handlers call directly.
//! - `AuthUser`, `AuthSession`, `AuthProvider` → `auth.rs` (the `Auth` enum).
//! - `MatchDirectory` → `actor::registry::Registry`; `ServerDeps` → `app::App`.
//! - `GameRecorder` → `jackioh_engine::summarize_game` and the catalog's newest patch, called
//!   directly where a finished match is filed (R376).
//! - `ProfileStore.setRating` (no caller) is not ported (SURFACE §11.2).
//!
//! Integers: epoch milliseconds, counts, caps and sequence numbers are `i64` (as TS stored them, and
//! as Postgres's `int8`/`count(*)` answer); ratings, deviations and volatilities are `f64` (migration
//! 0022 made them `double precision`).

use std::ops::{Index, IndexMut};
use std::sync::Arc;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use jackioh_engine::wire::{
    Action, GameMode, GameOverReason, GameRecord, PerPlayer, PlayerId, PortraitId, SourceFilter, Winner,
};

use super::{fake, pg};
use crate::ranked::glicko2::Glicko;
use crate::ranked::ladder::{SeasonRank, VisibleRank};
use crate::ranked::season::{ResetChange, ResetPlayer};

/// R417: one card of a last board — the card and its face, never its stats (C+ #29). `ports.ts`
/// declared the same `{ defId, radiant }` shape as the engine's; the engine's is the one type.
pub use jackioh_engine::state::LastBoardEntry;

/// R611: who played a side of a rated game: a person, or one of the AI bots (R610). `ports.ts`
/// declared the same `"human" | "ai"` as `packages/shared`'s `Pilot`; the wire's is the one type.
pub use jackioh_engine::wire::Pilot;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// What a store method can fail with. A refusal the port answers as a value (`RedeemResult`,
/// `UpsertOutcome`, a `false`, a `None`) is never an error.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// TS's `DuplicateResultError`: `results.insert`'s refusal of a second row for one match
    /// (§9.5), as its own variant so the writer can tell it from a real failure. A transaction whose
    /// `getByMatch` ran before a concurrent first writer committed only meets the duplicate here,
    /// and that collision is a clean no-op arriving the hard way — `results.rs` retries on it and
    /// finds the row the winner wrote. Postgres raises it as `unique_violation` (23505) on
    /// `results_pkey`. The field is the match id.
    #[error("result for match {0} is already recorded")]
    Duplicate(String),
    /// The driver's own failure: a lost connection, a constraint, a function's `raise`.
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    /// Anything else, with TS's message: a row that does not read back as the port's shape
    /// ("expected a frozen trio object"), or the fake's refusal of what Postgres would refuse by
    /// constraint ("duplicate match").
    #[error("{0}")]
    Other(String),
}

impl From<String> for StoreError {
    fn from(message: String) -> StoreError {
        StoreError::Other(message)
    }
}

impl From<&str> for StoreError {
    fn from(message: &str) -> StoreError {
        StoreError::Other(message.to_owned())
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(error: serde_json::Error) -> StoreError {
        StoreError::Other(error.to_string())
    }
}

/// Every store method's answer.
pub type StoreResult<T> = Result<T, StoreError>;

// ---------------------------------------------------------------------------
// Serde helpers and the string unions
// ---------------------------------------------------------------------------

/// A TS field `x?: T | null`, which has three states — absent, `null`, a value — kept apart:
/// `None` is absent, `Some(None)` is `null`, `Some(Some(x))` is `x`. Used with
/// `#[serde(default, skip_serializing_if = "Option::is_none", with = "absent_or_null")]`.
pub(crate) mod absent_or_null {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer, T: Serialize>(value: &Option<Option<T>>, serializer: S) -> Result<S::Ok, S::Error> {
        match value {
            Some(inner) => inner.serialize(serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
        deserializer: D,
    ) -> Result<Option<Option<T>>, D::Error> {
        Option::<T>::deserialize(deserializer).map(Some)
    }
}

/// A TS string-literal union as a Rust enum (SURFACE §4.3): one unit variant per literal,
/// serialised as exactly that literal, with `as_str` (the text a SQL column or function holds),
/// `FromStr`, `Display` and `ALL` (TS order).
macro_rules! store_union {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident = $text:literal ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum $name {
            $( $(#[$vmeta])* #[serde(rename = $text)] $variant ),+
        }

        impl $name {
            /// Every literal of the union, in the TS declaration order.
            pub const ALL: &'static [$name] = &[ $( $name::$variant ),+ ];

            /// The literal itself, as TS writes it and as the database stores it.
            pub fn as_str(self) -> &'static str {
                match self {
                    $( $name::$variant => $text ),+
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl std::str::FromStr for $name {
            type Err = StoreError;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                match text {
                    $( $text => Ok($name::$variant), )+
                    other => Err(StoreError::Other(format!(
                        "expected {}, got {}",
                        stringify!($name),
                        serde_json::to_string(other).unwrap_or_default()
                    ))),
                }
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Profiles
// ---------------------------------------------------------------------------

store_union! {
    pub enum ProfileStatus {
        Pending = "pending",
        Active = "active",
        Banned = "banned",
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    /// The managed-auth user id.
    pub user_id: String,
    pub email: String,
    /// TS `displayName?: string | null`: absent, cleared (`null`) or set.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "absent_or_null")]
    pub display_name: Option<Option<String>>,
    pub status: ProfileStatus,
    /// R603: the hidden Glicko-2 rating, its deviation and its volatility. Server-side only: no
    /// response carries any of the three (R612), the queue's rating window reads the first.
    pub rating: f64,
    pub rating_deviation: f64,
    pub rating_volatility: f64,
    /// Non-null while the profile is in a match (§9.5: every ending clears it).
    pub in_match_id: Option<String>,
    pub created_at: i64,
}

/// `ProfileStore.create`'s argument (TS's anonymous `{ userId, email, rating, at, displayName? }`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileCreateInput {
    pub user_id: String,
    pub email: String,
    pub rating: f64,
    pub at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "absent_or_null")]
    pub display_name: Option<Option<String>>,
}

// ---------------------------------------------------------------------------
// Invite codes (SPEC §9.4: codes stored hashed; §9.8: per-IP-hash limits)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InviteCode {
    pub id: String,
    pub code_hash: String,
    pub max_uses: i64,
    pub uses: i64,
    pub revoked: bool,
    /// Epoch ms, or null for "never expires".
    pub expires_at: Option<i64>,
    pub created_at: i64,
}

store_union! {
    pub enum CodeAttemptResult {
        Ok = "ok",
        Rejected = "rejected",
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CodeAttempt {
    pub profile_id: Option<String>,
    pub ip_hash: String,
    pub result: CodeAttemptResult,
    /// A coarse reason for operators; never returned to the client (§9.4).
    pub reason: String,
    pub at: i64,
}

store_union! {
    /// Exactly the strings `app.redeem_invite_code(uuid, text, text)` returns — migration 0001 §6
    /// lists them, and nothing else is a legal answer. The set is the SQL function's contract, so
    /// the port states it rather than a shape that would be pleasanter in Rust:
    ///
    ///  - `not_pending` covers "no such profile", "banned" and "already active" together, because
    ///    the function decides all three from one `select ... for update` and cannot tell a caller
    ///    which. R145 requires those three to be reported *distinctly* to the client, so `codes.rs`
    ///    decides them from the caller's own resolved profile before it calls here and treats this
    ///    result as the race it is: the account stopped being pending between the two. R170 fixes
    ///    what that race answers — a conflict, never a second 401 after authorization has already
    ///    passed.
    ///  - `email_unverified` is the function's own read of `auth.users.email_confirmed_at`, which
    ///    `public.profiles` does not carry. The server already knows the answer from the access
    ///    token (R159) and refuses first; this is the database's independent second opinion.
    ///  - `circuit_open` is the *database-side* half of §9.4's breaker
    ///    (`app.settings.redemption_enabled` plus the function's own failure count). It is not the
    ///    same object as the server's R106 breaker in `codes.rs`, which alerts, backs
    ///    `GET /api/codes/status` and is checked before this port is touched at all. Both answer 503.
    ///  - missing, revoked, expired and exhausted all collapse onto `invalid_code`, which is §9.4's
    ///    "Missing, expired and exhausted codes return an identical error" at the storage layer as
    ///    well as at the wire.
    pub enum RedeemResult {
        Ok = "ok",
        NotPending = "not_pending",
        EmailUnverified = "email_unverified",
        RateLimitedProfile = "rate_limited_profile",
        RateLimitedIp = "rate_limited_ip",
        CircuitOpen = "circuit_open",
        InvalidCode = "invalid_code",
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RedeemInviteCodeInput {
    pub profile_id: String,
    /// The keyed hash of the normalized code (`api::crypto`'s code hash); the store never sees
    /// plaintext.
    ///
    /// `None` means the caller has already established that this string could never be a code at
    /// all — wrong length, or a character outside §9.4's alphabet (R104). It is passed down rather
    /// than refused early because §9.4 orders the attempt log (step 4) *before* the lookup (step
    /// 5): a malformed code must still cost the caller a row in `code_attempts`, or it would be the
    /// one cheap probe in an interface built to make probing expensive. Both implementations answer
    /// `invalid_code` for it, indistinguishably from a code that was simply never minted.
    pub code_hash: Option<String>,
    pub ip_hash: String,
}

// ---------------------------------------------------------------------------
// The collection (§9.4: an entitlement ledger)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CollectionEntry {
    pub card_id: String,
    pub quantity: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CollectionGrant {
    pub profile_id: String,
    pub card_id: String,
    /// The signed change this grant applied, not the resulting total — the
    /// `collection_grants.delta` column, which is constrained `<> 0`. Named `delta` and not
    /// `quantity` on purpose: `CollectionEntry.quantity` next door is an absolute total, and one
    /// adapter reading the ledger as a total would double every grant.
    pub delta: i64,
    /// Constrained to a closed set in the schema: pack, craft, reward, refund, admin, launch.
    pub reason: String,
    pub at: i64,
}

// ---------------------------------------------------------------------------
// Saved decks and trios (SPEC §9.4, R250–R256). They replace the single three-deck loadout: a
// profile keeps up to `MAX_SAVED_DECKS` named decks and builds up to `MAX_SAVED_TRIOS` trios from
// them. Both are drafts (R250, R252): a save checks structure only, and legality is judged when a
// deck or a trio is queued (R253).
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SavedDeck {
    /// A UUID the client mints (R256), so a save is an idempotent upsert that can be retried.
    pub id: String,
    pub profile_id: String,
    /// 1..`DECK_NAME_MAX_LENGTH` characters, stored as `normalize_name` leaves it (R250).
    pub name: String,
    /// Catalog ids in the order the player put them in; at most `DECK_SIZE` (R250 D2).
    pub cards: Vec<String>,
    /// R641: the deck's hero portrait, or `null` — the default, `vanilla` (D5).
    pub portrait: Option<String>,
    /// The catalog version the client held at the last save. Informational: the queue re-validates
    /// (R253).
    pub catalog_version: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A trio's three slots, in order. `None` is an empty slot, which a saved trio may have (R252).
pub type TrioSlots = [Option<String>; 3];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SavedTrio {
    /// A UUID the client mints (R256).
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub deck_ids: TrioSlots,
    pub created_at: i64,
    pub updated_at: i64,
}

store_union! {
    /// What an upsert did:
    ///  - `created` / `updated` — written;
    ///  - `limit` — creating would take the profile past its cap (`MAX_SAVED_DECKS` or
    ///    `MAX_SAVED_TRIOS`); nothing was written;
    ///  - `not_owner` — the id is another profile's deck or trio; nothing was written, and the
    ///    caller answers exactly as for a missing id so an id reveals nothing about anyone else.
    pub enum UpsertOutcome {
        Created = "created",
        Updated = "updated",
        Limit = "limit",
        NotOwner = "not_owner",
    }
}

store_union! {
    /// A trio's upsert can also find a slot naming a deck that is not this profile's (or none at
    /// all): `UpsertOutcome | "unknown_deck"`.
    pub enum TrioUpsertOutcome {
        Created = "created",
        Updated = "updated",
        Limit = "limit",
        NotOwner = "not_owner",
        UnknownDeck = "unknown_deck",
    }
}

impl From<UpsertOutcome> for TrioUpsertOutcome {
    fn from(outcome: UpsertOutcome) -> TrioUpsertOutcome {
        match outcome {
            UpsertOutcome::Created => TrioUpsertOutcome::Created,
            UpsertOutcome::Updated => TrioUpsertOutcome::Updated,
            UpsertOutcome::Limit => TrioUpsertOutcome::Limit,
            UpsertOutcome::NotOwner => TrioUpsertOutcome::NotOwner,
        }
    }
}

// ---------------------------------------------------------------------------
// Queue modes (SPEC §9.5, R257) and what a ticket or a room freezes.
// ---------------------------------------------------------------------------

store_union! {
    /// R257: a ticket pairs only with a ticket of the same mode.
    pub enum QueueMode {
        Bo1 = "bo1",
        Bo3 = "bo3",
        Random = "random",
    }
}

/// TS's `Record<QueueMode, T>`: one value per mode, every mode present (as `PerPlayer` is for
/// `Record<PlayerId, T>`). Serialises as `{ "bo1": …, "bo3": …, "random": … }`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct PerMode<T> {
    pub bo1: T,
    pub bo3: T,
    pub random: T,
}

impl<T> Index<QueueMode> for PerMode<T> {
    type Output = T;

    fn index(&self, mode: QueueMode) -> &T {
        match mode {
            QueueMode::Bo1 => &self.bo1,
            QueueMode::Bo3 => &self.bo3,
            QueueMode::Random => &self.random,
        }
    }
}

impl<T> IndexMut<QueueMode> for PerMode<T> {
    fn index_mut(&mut self, mode: QueueMode) -> &mut T {
        match mode {
            QueueMode::Bo1 => &mut self.bo1,
            QueueMode::Bo3 => &mut self.bo3,
            QueueMode::Random => &mut self.random,
        }
    }
}

/// One deck as a match or a series freezes it: the cards and the name the player gave them.
/// `portrait` freezes the deck's hero portrait with them (R642: "the queued deck's portrait ...
/// frozen into the ticket or room with the deck"); absent on rows frozen before portraits existed,
/// so absence stays absent and reads as `vanilla` downstream.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FrozenDeck {
    pub name: String,
    pub cards: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "absent_or_null")]
    pub portrait: Option<Option<String>>,
}

/// R259: a Conquest player's trio, frozen at enqueue (or at room create/join).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FrozenTrio {
    pub name: String,
    pub decks: [FrozenDeck; 3],
}

// ---------------------------------------------------------------------------
// Matches
// ---------------------------------------------------------------------------

store_union! {
    pub enum MatchStatus {
        Live = "live",
        Finished = "finished",
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MatchRow {
    pub id: String,
    pub seed: String,
    /// Seat order: index 0 is p1, index 1 is p2.
    pub players: (String, String),
    /// The decks frozen into the tickets or the room (§9.4, §9.5).
    pub decks: (Vec<String>, Vec<String>),
    pub catalog_version: String,
    /// R604: a match the queue paired (or a game of a series it paired) is ranked; a room
    /// challenge is not. Only a ranked game moves a rating or a rank. Carried on `matches.ranked`
    /// since migration 0022; the store reads a false flag back as an absent one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ranked: Option<bool>,
    /// R672: the mode a rematch was made in, written when the rematch is created (migration 0023).
    /// Absent on every older row, whose mode `matches_mode_of` keeps deriving from its tickets,
    /// room or series exactly as before — no creation site but the rematch writes this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<QueueMode>,
    /// R672: a double-or-nothing rematch's stakes (migration 0023). Absent reads as 1, a normal
    /// game; only 2 is ever written, and only on a ranked rematch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stake: Option<i64>,
    pub status: MatchStatus,
    pub created_at: i64,
    pub finished_at: Option<i64>,
    /// Deadlines the clients render (§9.5: the grace countdown is stored on the match).
    pub clocks: MatchClocks,
    /// R417: each seat's last board as this match started (seat order, like `decks`), a
    /// `create_game` input frozen on the row so a rebuilt actor folds the same game. Absent when
    /// both are empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_boards: Option<(Vec<LastBoardEntry>, Vec<LastBoardEntry>)>,
    /// R678: the boards of two other players' last server games a Glitch may put on the field
    /// (seat order), sampled when the match is created and frozen on the row like `last_boards`, so
    /// a rebuilt actor folds the same game. Absent when no other player had a board to sample.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glitch_boards: Option<(Vec<LastBoardEntry>, Vec<LastBoardEntry>)>,
    /// R642: the hero portraits dealt to the seats, seat order like `decks`. Cosmetic only — it is
    /// sent in the `portraits` frame, never part of `PlayerView`. Absent on matches started before
    /// portraits existed; both seats then read as `vanilla`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portraits: Option<(PortraitId, PortraitId)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MatchClocks {
    /// Epoch ms the active player's turn clock expires, or null while it is paused.
    pub turn_deadline: Option<i64>,
    /// Epoch ms the open prompt's own clock expires (R79), or null.
    pub prompt_deadline: Option<i64>,
    /// Per-player disconnect grace deadlines (§9.5).
    pub grace_deadline: PerPlayer<Option<i64>>,
    /// Epoch ms the hard ceiling is reached (R79).
    pub ceiling_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MatchActionRow {
    pub match_id: String,
    /// 1-based, gapless, append-only.
    pub seq: i64,
    pub action: Action,
    pub at: i64,
}

// ---------------------------------------------------------------------------
// Rooms and tickets
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    pub code: String,
    pub host_profile_id: String,
    /// R264: the room's mode. A joiner plays it or is refused with it.
    pub mode: QueueMode,
    /// The host's frozen Best-of-1 deck; `[]` in the other two modes.
    pub host_deck: Vec<String>,
    /// R642: the host deck's portrait, frozen with it.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "absent_or_null")]
    pub host_portrait: Option<Option<String>>,
    /// The host's frozen trio in a Conquest room; null otherwise.
    pub host_trio: Option<FrozenTrio>,
    pub catalog_version: String,
    pub created_at: i64,
    pub expires_at: i64,
    /// Set once, atomically, by the first joiner.
    pub guest_profile_id: Option<String>,
    pub match_id: Option<String>,
}

store_union! {
    pub enum TicketStatus {
        Open = "open",
        Matched = "matched",
        Cancelled = "cancelled",
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Ticket {
    pub id: String,
    pub profile_id: String,
    pub rating: f64,
    /// R257: pairs only with a ticket of the same mode.
    pub mode: QueueMode,
    /// §9.4, §9.5: the Best-of-1 deck is frozen into the ticket; editing a saved deck later cannot
    /// change it. `[]` for a Conquest or an All Random ticket.
    pub deck: Vec<String>,
    /// R642: the Bo1 deck's portrait, frozen with it (R641's `null` — `vanilla` — otherwise).
    #[serde(default, skip_serializing_if = "Option::is_none", with = "absent_or_null")]
    pub portrait: Option<Option<String>>,
    /// R259: a Conquest ticket's frozen trio; null in the other two modes.
    pub trio: Option<FrozenTrio>,
    pub catalog_version: String,
    pub enqueued_at: i64,
    pub status: TicketStatus,
    pub match_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResultRow {
    pub match_id: String,
    /// Seat order, matching `MatchRow.players`.
    pub players: (String, String),
    /// null for a draw (R79: the ceiling and an accepted draw are draws).
    pub winner_profile_id: Option<String>,
    pub reason: GameOverReason,
    pub turns: i64,
    pub ended_at: i64,
    pub rating_before: (f64, f64),
    pub rating_after: (f64, f64),
}

/// A profile's finished-match record, counted from `results`.
///
/// A draw is a row with no winner — §9.5 makes the ceiling, a mutual hero death and an accepted
/// draw all winnerless — so wins + losses + draws is every match the profile has finished, and
/// nothing needs a separate "played" column to stay consistent with them.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProfileRecord {
    pub wins: i64,
    pub losses: i64,
    pub draws: i64,
}

// ---------------------------------------------------------------------------
// Game records for the card statistics (SPEC §9.11, R376–R378). Server-only (migration 0014).
// ---------------------------------------------------------------------------

/// R377, R378: which records a read returns (TS: `Pick<CardStatsFilter, "source" | "mode" |
/// "patch">`). Pilots are per seat, so `card_stats` applies those.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameRecordQuery {
    pub source: SourceFilter,
    /// One match type, or null for every mode.
    pub mode: Option<GameMode>,
    /// One patch, or null for every patch.
    pub patch: Option<String>,
}

// ---------------------------------------------------------------------------
// The Conquest series (SPEC §9.5, R330–R338, R262–R264). One row per series, persisted so a series
// survives a server restart; every transition is a pure function in `api/series_rules.rs` written
// back with `series_update`, which is compare-and-set on `version`.
// ---------------------------------------------------------------------------

/// A series seat. Index 0 of `SeriesRow.sides` is `p1`; it is not the seat a game's match uses.
/// TS's `"p1" | "p2"`, the same literals as `PlayerId`, so the one type.
pub type SeriesSeat = PlayerId;

store_union! {
    /// `picking` — both players are choosing the next game's deck (R331, R333);
    /// `playing` — the game `next_match_id` names is being played (or about to be started);
    /// `over` — decided, played out, forfeited or abandoned (R334).
    pub enum SeriesStatus {
        Picking = "picking",
        Playing = "playing",
        Over = "over",
    }
}

store_union! {
    /// Why a series ended (R334): a side won with every deck, `SERIES_WINS_NEEDED` wins
    /// (`decided`, R330); `SERIES_MAX_GAMES` were played without that (`exhausted`); a side left
    /// between games (`forfeit`); or neither side picked before the pick clock ran out, or a game
    /// could not be started (`abandoned`, unrated, R333, R263).
    pub enum SeriesEnd {
        Decided = "decided",
        Exhausted = "exhausted",
        Forfeit = "forfeit",
        Abandoned = "abandoned",
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeriesGame {
    /// 1-based.
    pub game_no: i64,
    pub match_id: String,
    /// The trio slot each side played, index 0 being series `p1`.
    pub slots: (i64, i64),
    /// Which side went first — was the match's `p1` (R335: odd games p1, even games p2).
    pub first: SeriesSeat,
    /// Null while the game is being played (TS `SeriesSeat | "draw" | null`).
    pub winner: Option<Winner>,
    pub reason: Option<GameOverReason>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeriesSide {
    pub profile_id: String,
    pub trio: FrozenTrio,
    /// Games this side has won. Each win locks the deck it was won with (R330), so this is also
    /// how many of its decks have won; which ones is read off `games`.
    pub wins: i64,
    /// The trio slot this side picked for the next game, or null. Sealed: final once in, and
    /// hidden from the other side until both have picked (R331): it leaves the server only in its
    /// owner's projection. Set by the server when one deck is left (R332).
    pub pick: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeriesRow {
    pub id: String,
    pub sides: (SeriesSide, SeriesSide),
    pub catalog_version: String,
    /// R604: a series the queue paired is ranked and moves the rating when it ends; a room's is
    /// not. Carried on `series.ranked` since migration 0022; the store reads a false flag back as
    /// an absent one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ranked: Option<bool>,
    /// Each game's seed is `{seed_base}:{game_no}` (R335). The server mints it; R143's e2e
    /// override feeds it.
    pub seed_base: String,
    pub status: SeriesStatus,
    pub games: Vec<SeriesGame>,
    /// The match id of the game being picked for or played. Minted when the pick phase opens — for
    /// game 1, when the series is made — so a restart finds the same id (R263).
    pub next_match_id: String,
    /// Epoch ms the pick phase closes (R333); null outside it.
    pub pick_deadline: Option<i64>,
    pub winner: Option<Winner>,
    pub end_reason: Option<SeriesEnd>,
    /// R262: the one rating move a series makes, recorded when it ends; null until then, when
    /// abandoned and for an unranked series (R604).
    pub rating_before: Option<(f64, f64)>,
    pub rating_after: Option<(f64, f64)>,
    pub created_at: i64,
    pub updated_at: i64,
    pub ended_at: Option<i64>,
    /// Optimistic concurrency: `series_update` writes only over the version before this one.
    pub version: i64,
}

// ---------------------------------------------------------------------------
// The ranked ladder (SPEC §9.12, R603–R612): seasons, each player's season on the ladder, the bots'
// ratings, and the record of every rated game. The rules are `ranked/*`, pure; `api/ranked.rs`
// reads and writes through this port.
// ---------------------------------------------------------------------------

/// R609: a season, named by the minor version of the game (`v0.2`), and the patch that opened it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Season {
    pub id: String,
    pub patch_version: String,
    pub started_at: i64,
}

/// One player's season row with their current hidden rating: what percentiles and Jlorious read
/// (TS: `SeasonRank & { rating: number }`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeasonStanding {
    #[serde(flatten)]
    pub rank: SeasonRank,
    pub rating: f64,
}

/// R610: an AI bot's own Glicko-2 rating. A bot has no ladder rank and no place on the leaderboard.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BotRating {
    pub bot_id: String,
    pub glicko: Glicko,
    pub games: i64,
    pub updated_at: i64,
}

/// R611: one side of a rated game, as recorded.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RatedSide {
    /// The player, or null for a bot (and for a player whose account was deleted since).
    pub profile_id: Option<String>,
    /// The bot, or null for a player.
    pub bot_id: Option<String>,
    pub pilot: Pilot,
    /// The hidden rating before and after the game.
    pub before: Glicko,
    pub after: Glicko,
    /// The visible rank before and after; null for a bot, which has none.
    pub rank_before: Option<VisibleRank>,
    pub rank_after: Option<VisibleRank>,
}

store_union! {
    /// `RatedGameRow.kind`: a ranked match, or a ranked series rated once as a whole (R262).
    pub enum RatedGameKind {
        Match = "match",
        Series = "series",
    }
}

/// How a rated game ended: the match's reason, or the series' (R334). TS:
/// `GameOverReason | SeriesEnd`, whose literals are disjoint.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum RatedReason {
    Game(GameOverReason),
    Series(SeriesEnd),
}

/// R611: the record of one rated game: a ranked match, or a ranked series, which is rated once as
/// a whole (R262). Kept for good (account deletion empties a side's `profile_id`, as for
/// `results`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RatedGameRow {
    /// The match's id, or the series' id.
    pub id: String,
    pub kind: RatedGameKind,
    pub season_id: String,
    /// The game's version, the newest patch (R375), and the catalog version it was played on.
    pub patch_version: String,
    pub catalog_version: String,
    pub sides: (RatedSide, RatedSide),
    /// The index of the side that won, or null for a draw.
    pub winner_side: Option<usize>,
    /// How it ended: the match's reason, or the series' (R334).
    pub reason: RatedReason,
    pub ended_at: i64,
}

// ---------------------------------------------------------------------------
// Tutorial progress on the account (SPEC §9.10, R320). The device keeps its own copy (R294) and the
// client merges the two (R321); this is the account's half, which only ever grows.
// ---------------------------------------------------------------------------

/// R320, R322: the player's newest explicit choice to hide or show the lesson path, and when it
/// was made (epoch ms, the choosing device's clock, never later than the server's when it arrived).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TutorialHiddenChoice {
    pub hidden: bool,
    pub at: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TutorialProgressRow {
    pub profile_id: String,
    /// Completed lesson ids, each once, in code-point order. Ids the client does not know are kept.
    pub completed: Vec<String>,
    /// Null until the player first hides or shows the path.
    pub hidden_choice: Option<TutorialHiddenChoice>,
}

/// What one write proposes: the device's progress, merged into the account's (R320).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TutorialMergeInput {
    pub profile_id: String,
    /// Already checked by the handler: lesson-id slugs, each once.
    pub completed: Vec<String>,
    pub hidden_choice: Option<TutorialHiddenChoice>,
    /// The server's clock when the write arrived. Postgres stamps it as the row's `updated_at`
    /// (and `created_at` for a new row), for operators; nothing reads it back through the port.
    pub at: i64,
}

/// `merged` with the row as it now stands, or `limit` when the union would pass the cap (nothing
/// written). Reachable only by a client that sends ids no lesson has.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TutorialMergeOutcome {
    Merged { progress: TutorialProgressRow },
    Limit,
}

// ---------------------------------------------------------------------------
// Player settings on the account (SPEC §9.1, R633, R634). The device keeps its own copy and the
// client merges the two; this is the account's half, where a group is replaced only by a newer one.
// ---------------------------------------------------------------------------

/// One setting's value. The server does not know the settings: it keeps flat booleans, numbers and
/// short texts. A number stays the JSON number it arrived as (`1` stays `1`, never `1.0`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum PlayerSettingValue {
    Bool(bool),
    Number(serde_json::Number),
    Text(String),
}

/// R633: one group of settings (the gameplay switches, the audio volumes, the effects, the card
/// display) and when it last changed on the device that wrote it (epoch ms, the writing device's
/// clock, never later than the server's when it arrived).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSettingsGroup {
    pub at: i64,
    pub values: IndexMap<String, PlayerSettingValue>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSettingsRow {
    pub profile_id: String,
    /// Group id to group. A group the client no longer has is kept and ignored by it.
    pub groups: IndexMap<String, PlayerSettingsGroup>,
}

/// What one write proposes: the groups a device has changed, merged into the account's (R634).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSettingsMergeInput {
    pub profile_id: String,
    /// Already checked by the handler: group ids, each a flat object of values.
    pub groups: IndexMap<String, PlayerSettingsGroup>,
    /// The server's clock when the write arrived. Postgres stamps it as the row's `updated_at`
    /// (and `created_at` for a new row), for operators; nothing reads it back through the port.
    pub at: i64,
}

/// The caller's caps on the result (`PLAYER_SETTINGS_GROUPS_MAX`, `PLAYER_SETTINGS_BYTES_MAX`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSettingsLimits {
    pub max_groups: i64,
    pub max_bytes: i64,
}

/// `merged` with the row as it now stands, or `limit` when the result would pass a cap (nothing
/// written). Reachable only by a client that sends groups no client has.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PlayerSettingsMergeOutcome {
    Merged { settings: PlayerSettingsRow },
    Limit,
}

// ---------------------------------------------------------------------------
// Last boards (R417, R565, R678)
// ---------------------------------------------------------------------------

store_union! {
    /// R417: "your last game" is your last finished game of the same kind; only `server` is
    /// written today.
    pub enum LastBoardKind {
        Server = "server",
        Practice = "practice",
    }
}

// ---------------------------------------------------------------------------
// Player statistics on the account (SPEC §9.11, R639, R654).
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStatsRow {
    pub profile_id: String,
    pub stats: IndexMap<String, Value>,
    pub is_private: bool,
    pub updated_at: i64,
}

/// One of `PublicPlayerSummary.favourite_cards` (TS's anonymous `{ id, count }`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FavouriteCard {
    pub id: String,
    pub count: i64,
}

/// `PublicPlayerSummary.fun_stats` (TS's anonymous object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FunStats {
    pub nemesis_card_id: Option<String>,
    pub total_destroyed: i64,
    pub total_defeated: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicPlayerSummary {
    pub profile_id: String,
    pub display_name: Option<String>,
    pub games: i64,
    pub wins: i64,
    pub losses: i64,
    pub draws: i64,
    pub win_rate: Option<f64>,
    pub favourite_cards: Vec<FavouriteCard>,
    pub fun_stats: FunStats,
    pub updated_at: i64,
}

/// `PlayerStatsStore.listPublic`'s argument (TS's anonymous `{ search?, limit, offset }`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStatsListOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

// ---------------------------------------------------------------------------
// The retention purge
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPurgeInput {
    pub code_attempts_before: i64,
    pub match_actions_ended_before: i64,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPurgeResult {
    pub code_attempts: i64,
    pub match_actions: i64,
}

// ---------------------------------------------------------------------------
// Live matches: what `queue.rs`, the room endpoints, the series and the rematch hand
// `actor::registry::Registry::start` (TS's `MatchDirectory.start` input).
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MatchSeat {
    pub profile_id: String,
    pub player: PlayerId,
    pub deck: Vec<String>,
    /// R642: the portrait dealt to this seat at match creation. Absent reads as `vanilla`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portrait: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StartMatchInput {
    pub match_id: String,
    pub seed: String,
    pub catalog_version: String,
    /// R604: true when the queue paired it, for the match row.
    pub ranked: bool,
    pub seats: (MatchSeat, MatchSeat),
    /// R672: the mode a rematch is made in, carried onto the match row. Absent everywhere else:
    /// the queue, the rooms and the series keep deriving the mode from what made the match
    /// (`matches_mode_of`), and only a rematch — which no ticket, room or series made — states it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<QueueMode>,
    /// R672: a double-or-nothing rematch's stakes, carried onto the match row. Absent reads as 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stake: Option<i64>,
}

// ---------------------------------------------------------------------------
// The store: Db (a pool or the fake's tables) and Tx (one transaction)
// ---------------------------------------------------------------------------

/// Migration 0001 §8, and the same closing note in 0002-0009: `service_role` is the role the API
/// server and the match actor hold. It is the only role granted EXECUTE on
/// `app.redeem_invite_code`, `app.upsert_deck`, `app.append_match_action`, `app.claim_ticket_pair`
/// and the rest, so running as it is not a formality — a call that gets it wrong fails with
/// `insufficient_privilege` instead of succeeding because the connection happened to own the table.
const ACTING_ROLE: &str = "service_role";

/// One statement, two `SET LOCAL`s. `set_config(name, value, true)` is `SET LOCAL name = value`,
/// and unlike `SET LOCAL` it takes parameters, so the profile id is bound rather than interpolated.
///
/// `request.jwt.claim.sub` is the GUC Supabase's `auth.uid()` reads (see
/// `tests/sql/00_supabase_stub.sql`, which stands the same function up for a plain Postgres). Every
/// RLS policy in 0002-0007 is `profile_id = app.current_profile_id()`, and
/// `app.profile_is_active()` reads `auth.uid()` directly, so a transaction that leaves it unset is
/// a transaction where those expressions silently see NULL. `service_role` carries BYPASSRLS, which
/// means this cannot change the result of anything today; it is set anyway so that the day a
/// policy, a trigger or a SECURITY INVOKER helper does consult the caller, it sees the profile the
/// call is about rather than nobody.
///
/// `SET LOCAL` is scoped to a transaction and is a silent no-op with a warning outside one, which
/// is exactly how a driver ends up quietly running as a superuser with RLS bypassed. `Db::begin`
/// is therefore the only way to a `Tx`: it issues `BEGIN` before it issues the role switch, and
/// nothing executes SQL outside a `Tx` (store.ts's rule 2).
const SESSION_SQL: &str = "select set_config('role', $1, true), set_config('request.jwt.claim.sub', $2, true)";

/// The store: Postgres behind a pool, or the fake's tables behind one lock (unit tests and
/// `E2E=1`). Cloning shares the pool or the tables.
#[derive(Clone)]
pub enum Db {
    Pg(sqlx::PgPool),
    Fake(Arc<tokio::sync::Mutex<fake::FakeData>>),
}

/// One transaction on the store. Every TS `Store` method is a method here. Dropped without
/// `commit`, it rolls back.
pub enum Tx<'a> {
    Pg(sqlx::Transaction<'a, sqlx::Postgres>),
    Fake(fake::FakeTx<'a>),
}

impl Db {
    /// An empty fake store: the in-memory tables the unit tests and `E2E=1` run on (TS
    /// `createMemoryStore()` / `createE2EStore()`).
    pub fn fake() -> Db {
        Db::Fake(Arc::new(tokio::sync::Mutex::new(fake::FakeData::default())))
    }

    /// Opens one transaction. Postgres: `BEGIN`, then the role switch to `service_role` and
    /// `request.jwt.claim.sub` set to `claim_sub` — the profile the call is about, or `""` for the
    /// calls that are about nobody (`tickets_list_open`, `matches_live`, the breaker's
    /// `codes_count_failures`) — in the one `SESSION_SQL` statement (store.ts's rule 2). The fake:
    /// takes the tables' lock and a snapshot to restore if the transaction is dropped uncommitted.
    pub async fn begin(&self, claim_sub: Option<&str>) -> Result<Tx<'_>, StoreError> {
        match self {
            Db::Pg(pool) => {
                let mut t = pool.begin().await?;
                sqlx::query(SESSION_SQL)
                    .bind(ACTING_ROLE)
                    .bind(claim_sub.unwrap_or(""))
                    .execute(&mut *t)
                    .await?;
                Ok(Tx::Pg(t))
            }
            Db::Fake(data) => Ok(Tx::Fake(fake::begin(data).await)),
        }
    }

    /// TS `PostgresStore.close()`: lets the pool's connections go, for a CLI about to exit. A no-op
    /// on the fake.
    pub async fn close(&self) {
        if let Db::Pg(pool) = self {
            pool.close().await;
        }
    }
}

/// One method body: the same call against whichever store this transaction is on. Postgres's
/// methods are async over the open `sqlx::Transaction`; the fake's are plain functions over the
/// locked tables.
macro_rules! dispatch {
    ($self:ident, $name:ident ( $($arg:expr),* $(,)? )) => {
        match $self {
            Tx::Pg(t) => pg::$name(t, $($arg),*).await,
            Tx::Fake(f) => fake::$name(f, $($arg),*),
        }
    };
}

impl Tx<'_> {
    /// Ends the transaction, keeping every write in it.
    pub async fn commit(self) -> Result<(), StoreError> {
        match self {
            Tx::Pg(t) => {
                t.commit().await?;
                Ok(())
            }
            Tx::Fake(f) => {
                fake::commit(f);
                Ok(())
            }
        }
    }

    // -----------------------------------------------------------------------
    // Root
    // -----------------------------------------------------------------------

    /// SPEC §9.4's redemption, whole: "Redemption is one server-side transaction: (1) reject
    /// unless the account is pending with a verified email; (2) reject if this profile made more
    /// than 5 attempts in the last hour; (3) reject if this IP hash made more than 20; (4) log the
    /// attempt either way; (5) look up by hash and reject if revoked, expired or exhausted; (6)
    /// increment uses and set the account active, atomically."
    ///
    /// It sits beside the transaction rather than under `codes` because it *is* a transaction and
    /// it writes three tables (`profiles`, `invite_codes`, `code_attempts`); it is not an operation
    /// on the code table. In Postgres it is one statement — `select app.redeem_invite_code(...)`,
    /// migration 0001 §6 — which is why the result type is that function's return values verbatim.
    ///
    /// Steps 2 and 3's limits and their window are ENFORCED BY THE STORE, NOT BY A HANDLER: they run
    /// where steps 4-6 run — inside the database, under the same lock. `config.rs` holds the
    /// deployment's copy of the numbers, which is also where the fake takes its default;
    /// `app.redeem_invite_code` writes §9.4's `5` and `20` into its own body (`pg.rs`'s KNOWN
    /// DIVERGENCES, "redemption limits").
    ///
    /// Two orderings the spec fixes and both implementations keep:
    ///  - steps 2 and 3 reject BEFORE step 4, so a caller already over the limit does not pin their
    ///    own counter by retrying and a flood is cheap to refuse;
    ///  - a rejection is RETURNED, never thrown, so the attempt row step 4 owes is never rolled back
    ///    by a later rejection in the same transaction.
    ///
    /// What it deliberately does not do is time: §9.4's "identical error in identical time" is
    /// R107's response floor, which is the server's job (`codes.rs`) and which SQL cannot deliver.
    pub async fn redeem(&mut self, input: &RedeemInviteCodeInput) -> StoreResult<RedeemResult> {
        dispatch!(self, redeem(input))
    }

    /// The retention purge (`api/retention.rs`): deletes `code_attempts` rows made before
    /// `code_attempts_before` and the action log of every match that ended before
    /// `match_actions_ended_before` (epoch ms). Results, and so ratings, are kept. Answers how many
    /// rows of each went. In Postgres this is `app.purge_expired_rows` (migration 0013), the one
    /// path the append-only guard on `match_actions` lets a delete through.
    pub async fn purge_expired(&mut self, input: &RetentionPurgeInput) -> StoreResult<RetentionPurgeResult> {
        dispatch!(self, purge_expired(input))
    }

    // -----------------------------------------------------------------------
    // profiles
    // -----------------------------------------------------------------------

    pub async fn profiles_get_by_id(&mut self, profile_id: &str) -> StoreResult<Option<Profile>> {
        dispatch!(self, profiles_get_by_id(profile_id))
    }

    pub async fn profiles_get_by_user_id(&mut self, user_id: &str) -> StoreResult<Option<Profile>> {
        dispatch!(self, profiles_get_by_user_id(user_id))
    }

    pub async fn profiles_get_many(&mut self, profile_ids: &[String]) -> StoreResult<Vec<Profile>> {
        dispatch!(self, profiles_get_many(profile_ids))
    }

    /// A new profile, at `rating` with a new player's deviation and volatility (R603).
    pub async fn profiles_create(&mut self, input: &ProfileCreateInput) -> StoreResult<Profile> {
        dispatch!(self, profiles_create(input))
    }

    pub async fn profiles_set_status(&mut self, profile_id: &str, status: ProfileStatus) -> StoreResult<()> {
        dispatch!(self, profiles_set_status(profile_id, status))
    }

    /// R603: one profile's hidden rating after a rated game.
    pub async fn profiles_set_glicko(&mut self, profile_id: &str, glicko: &Glicko) -> StoreResult<()> {
        dispatch!(self, profiles_set_glicko(profile_id, glicko))
    }

    pub async fn profiles_set_display_name(
        &mut self,
        profile_id: &str,
        display_name: Option<&str>,
    ) -> StoreResult<()> {
        dispatch!(self, profiles_set_display_name(profile_id, display_name))
    }

    /// Pass `None` to clear. §9.5: every terminal reason clears both players'.
    pub async fn profiles_set_in_match(&mut self, profile_id: &str, match_id: Option<&str>) -> StoreResult<()> {
        dispatch!(self, profiles_set_in_match(profile_id, match_id))
    }

    /// Deletes the profile and everything that is only its own: collection and its ledger, decks,
    /// trios, tutorial progress, queue tickets, rooms it opened that nobody joined, and its
    /// invite-code attempts' link to it. Finished matches, results and series stay for the other
    /// player, with this profile's seat left empty (migration 0012). False when there was no such
    /// profile. The caller refuses a profile in a live match or series first.
    pub async fn profiles_remove(&mut self, profile_id: &str) -> StoreResult<bool> {
        dispatch!(self, profiles_remove(profile_id))
    }

    // -----------------------------------------------------------------------
    // codes
    // -----------------------------------------------------------------------

    pub async fn codes_insert(&mut self, code: &InviteCode) -> StoreResult<()> {
        dispatch!(self, codes_insert(code))
    }

    pub async fn codes_find_by_hash(&mut self, code_hash: &str) -> StoreResult<Option<InviteCode>> {
        dispatch!(self, codes_find_by_hash(code_hash))
    }

    /// §9.4 step 6: one atomic statement. Increments `uses` only while the code is unrevoked,
    /// unexpired and unexhausted; returns false otherwise. Two concurrent callers cannot both win
    /// the last use.
    ///
    /// Redemption does not call this — `redeem` is the whole transaction. It stays because the
    /// fake builds step 6 out of it and the contract suite drives it directly.
    pub async fn codes_claim(&mut self, code_id: &str, now: i64) -> StoreResult<bool> {
        dispatch!(self, codes_claim(code_id, now))
    }

    /// §9.4 step 4: the attempt is logged either way. Written by `redeem`.
    pub async fn codes_log_attempt(&mut self, attempt: &CodeAttempt) -> StoreResult<()> {
        dispatch!(self, codes_log_attempt(attempt))
    }

    pub async fn codes_count_attempts_by_profile(&mut self, profile_id: &str, since: i64) -> StoreResult<i64> {
        dispatch!(self, codes_count_attempts_by_profile(profile_id, since))
    }

    /// When this profile's oldest attempt at or after `since` was made (epoch ms), or `None` when it
    /// made none. R192: `GET /api/codes/status` adds the window to it to say when an account that
    /// has used up §9.4 step 2's tries gets one back.
    pub async fn codes_oldest_attempt_at_by_profile(
        &mut self,
        profile_id: &str,
        since: i64,
    ) -> StoreResult<Option<i64>> {
        dispatch!(self, codes_oldest_attempt_at_by_profile(profile_id, since))
    }

    pub async fn codes_count_attempts_by_ip(&mut self, ip_hash: &str, since: i64) -> StoreResult<i64> {
        dispatch!(self, codes_count_attempts_by_ip(ip_hash, since))
    }

    /// §9.4: the server-side circuit breaker's input — system-wide failures in a window (R106).
    pub async fn codes_count_failures(&mut self, since: i64) -> StoreResult<i64> {
        dispatch!(self, codes_count_failures(since))
    }

    // -----------------------------------------------------------------------
    // collection (§9.4: an entitlement ledger. `collection_upsert_quantities` and
    // `collection_append_grants` are the two writes every mutation makes, and callers must make
    // them inside one transaction.)
    // -----------------------------------------------------------------------

    pub async fn collection_get(&mut self, profile_id: &str) -> StoreResult<Vec<CollectionEntry>> {
        dispatch!(self, collection_get(profile_id))
    }

    /// SETS each card's quantity to the absolute value given; it does not add to it. The caller has
    /// already read the current total inside the same transaction and computed the new one, so the
    /// Postgres side must `set quantity = excluded.quantity`, never `quantity + excluded`.
    pub async fn collection_upsert_quantities(
        &mut self,
        profile_id: &str,
        entries: &[CollectionEntry],
    ) -> StoreResult<()> {
        dispatch!(self, collection_upsert_quantities(profile_id, entries))
    }

    /// Append-only (§9.4). Each row's `delta` is the change, so the ledger sums to the total.
    pub async fn collection_append_grants(&mut self, grants: &[CollectionGrant]) -> StoreResult<()> {
        dispatch!(self, collection_append_grants(grants))
    }

    // -----------------------------------------------------------------------
    // decks
    // -----------------------------------------------------------------------

    /// A profile's decks, oldest first: `created_at`, then `id`.
    pub async fn decks_list(&mut self, profile_id: &str) -> StoreResult<Vec<SavedDeck>> {
        dispatch!(self, decks_list(profile_id))
    }

    /// One deck by id, whoever owns it; the caller checks `profile_id`.
    pub async fn decks_get(&mut self, deck_id: &str) -> StoreResult<Option<SavedDeck>> {
        dispatch!(self, decks_get(deck_id))
    }

    /// Inserts a deck whose id is new, or replaces `name`, `cards`, `catalog_version` and
    /// `updated_at` of the profile's own deck (its `created_at` is kept). The cap is checked under a
    /// lock on the profile, so two concurrent creates cannot both pass it (`app.upsert_deck`,
    /// migration 0007).
    pub async fn decks_upsert(&mut self, deck: &SavedDeck, max_decks: i64) -> StoreResult<UpsertOutcome> {
        dispatch!(self, decks_upsert(deck, max_decks))
    }

    /// Deletes the profile's own deck; every trio slot that named it becomes `null` in the same
    /// statement (R252). False when the profile has no deck with this id.
    pub async fn decks_remove(&mut self, profile_id: &str, deck_id: &str) -> StoreResult<bool> {
        dispatch!(self, decks_remove(profile_id, deck_id))
    }

    // -----------------------------------------------------------------------
    // trios
    // -----------------------------------------------------------------------

    /// A profile's trios, oldest first: `created_at`, then `id`.
    pub async fn trios_list(&mut self, profile_id: &str) -> StoreResult<Vec<SavedTrio>> {
        dispatch!(self, trios_list(profile_id))
    }

    pub async fn trios_get(&mut self, trio_id: &str) -> StoreResult<Option<SavedTrio>> {
        dispatch!(self, trios_get(trio_id))
    }

    /// As `decks_upsert`, plus `unknown_deck` when a non-null slot names a deck that is not this
    /// profile's. The caller has already checked T1–T3 (`check_trio_draft`); the store refuses a
    /// deck twice as well, by constraint.
    pub async fn trios_upsert(&mut self, trio: &SavedTrio, max_trios: i64) -> StoreResult<TrioUpsertOutcome> {
        dispatch!(self, trios_upsert(trio, max_trios))
    }

    pub async fn trios_remove(&mut self, profile_id: &str, trio_id: &str) -> StoreResult<bool> {
        dispatch!(self, trios_remove(profile_id, trio_id))
    }

    // -----------------------------------------------------------------------
    // matches
    // -----------------------------------------------------------------------

    pub async fn matches_create(&mut self, row: &MatchRow) -> StoreResult<()> {
        dispatch!(self, matches_create(row))
    }

    pub async fn matches_get(&mut self, match_id: &str) -> StoreResult<Option<MatchRow>> {
        dispatch!(self, matches_get(match_id))
    }

    /// Append-only (§9.3). Rejects a seq that already exists.
    pub async fn matches_append_actions(&mut self, rows: &[MatchActionRow]) -> StoreResult<()> {
        dispatch!(self, matches_append_actions(rows))
    }

    pub async fn matches_actions(&mut self, match_id: &str) -> StoreResult<Vec<MatchActionRow>> {
        dispatch!(self, matches_actions(match_id))
    }

    pub async fn matches_set_clocks(&mut self, match_id: &str, clocks: &MatchClocks) -> StoreResult<()> {
        dispatch!(self, matches_set_clocks(match_id, clocks))
    }

    pub async fn matches_finish(&mut self, match_id: &str, at: i64) -> StoreResult<()> {
        dispatch!(self, matches_finish(match_id, at))
    }

    /// For the reaper (§9.5).
    pub async fn matches_live(&mut self) -> StoreResult<Vec<MatchRow>> {
        dispatch!(self, matches_live())
    }

    /// R257, R376: the mode a match was made in, read off what made it: `bo3` for a game of a
    /// Conquest series (`series_with_game`), else the room's mode, else the mode of the queue
    /// tickets it paired. `None` for a match none of them made. The match row itself does not
    /// record it (a rematch's row does, R672, and that is read first).
    pub async fn matches_mode_of(&mut self, match_id: &str) -> StoreResult<Option<QueueMode>> {
        dispatch!(self, matches_mode_of(match_id))
    }

    /// R263: forget a match id that was reserved and never started — the first game of a Conquest
    /// series that ended (forfeit, abandoned) before it was played. In Postgres the reservation is
    /// an `open` row (`tickets_claim_pair`'s skeleton, or a claimed room), and dropping it releases
    /// a room code for reuse (R110). A no-op for an id with no such row, and never touches a live or
    /// finished match.
    pub async fn matches_discard_open(&mut self, match_id: &str) -> StoreResult<()> {
        dispatch!(self, matches_discard_open(match_id))
    }

    /// R679: a Glitch voided this live match, so it is removed as if it never existed: the row and
    /// its action log go, and any profile whose in-match flag points at it is let go (both players
    /// can queue again). Never touches a finished match or one with a result; a no-op for an
    /// unknown id.
    pub async fn matches_forget_voided(&mut self, match_id: &str) -> StoreResult<()> {
        dispatch!(self, matches_forget_voided(match_id))
    }

    // -----------------------------------------------------------------------
    // rooms
    // -----------------------------------------------------------------------

    /// False when the code is already taken.
    pub async fn rooms_create(&mut self, room: &Room) -> StoreResult<bool> {
        dispatch!(self, rooms_create(room))
    }

    pub async fn rooms_get(&mut self, code: &str) -> StoreResult<Option<Room>> {
        dispatch!(self, rooms_get(code))
    }

    /// Atomic single-claim: sets guest and match id only while the room is unclaimed and unexpired.
    /// Returns the claimed room, or `None` when someone else got there first.
    pub async fn rooms_claim(
        &mut self,
        code: &str,
        guest_profile_id: &str,
        match_id: &str,
        at: i64,
    ) -> StoreResult<Option<Room>> {
        dispatch!(self, rooms_claim(code, guest_profile_id, match_id, at))
    }

    // -----------------------------------------------------------------------
    // tickets
    // -----------------------------------------------------------------------

    pub async fn tickets_insert(&mut self, ticket: &Ticket) -> StoreResult<()> {
        dispatch!(self, tickets_insert(ticket))
    }

    pub async fn tickets_get(&mut self, ticket_id: &str) -> StoreResult<Option<Ticket>> {
        dispatch!(self, tickets_get(ticket_id))
    }

    pub async fn tickets_open_for_profile(&mut self, profile_id: &str) -> StoreResult<Option<Ticket>> {
        dispatch!(self, tickets_open_for_profile(profile_id))
    }

    pub async fn tickets_list_open(&mut self) -> StoreResult<Vec<Ticket>> {
        dispatch!(self, tickets_list_open())
    }

    pub async fn tickets_count_open(&mut self) -> StoreResult<i64> {
        dispatch!(self, tickets_count_open())
    }

    /// R257: open tickets per mode, for the lobby's per-mode population. Every mode is present.
    pub async fn tickets_count_open_by_mode(&mut self) -> StoreResult<PerMode<i64>> {
        dispatch!(self, tickets_count_open_by_mode())
    }

    /// §9.5: "both tickets are claimed in one atomic statement". Returns false unless both were
    /// still open, so two concurrent matchers cannot pair the same ticket twice.
    pub async fn tickets_claim_pair(&mut self, a_id: &str, b_id: &str, match_id: &str, at: i64) -> StoreResult<bool> {
        dispatch!(self, tickets_claim_pair(a_id, b_id, match_id, at))
    }

    pub async fn tickets_cancel(&mut self, ticket_id: &str, at: i64) -> StoreResult<()> {
        dispatch!(self, tickets_cancel(ticket_id, at))
    }

    // -----------------------------------------------------------------------
    // results
    // -----------------------------------------------------------------------

    /// One row per match (§9.5). Rejects a second row for the same match with
    /// `StoreError::Duplicate`.
    pub async fn results_insert(&mut self, row: &ResultRow) -> StoreResult<()> {
        dispatch!(self, results_insert(row))
    }

    pub async fn results_get_by_match(&mut self, match_id: &str) -> StoreResult<Option<ResultRow>> {
        dispatch!(self, results_get_by_match(match_id))
    }

    /// Every finished match this profile played, as wins/losses/draws.
    pub async fn results_record_for(&mut self, profile_id: &str) -> StoreResult<ProfileRecord> {
        dispatch!(self, results_record_for(profile_id))
    }

    // -----------------------------------------------------------------------
    // series
    // -----------------------------------------------------------------------

    pub async fn series_create(&mut self, series: &SeriesRow) -> StoreResult<()> {
        dispatch!(self, series_create(series))
    }

    pub async fn series_get(&mut self, series_id: &str) -> StoreResult<Option<SeriesRow>> {
        dispatch!(self, series_get(series_id))
    }

    /// Compare-and-set: writes `next` only when the stored row's `version` is `next.version - 1`.
    /// False when another writer got there first; the caller re-reads and re-applies its
    /// transition.
    pub async fn series_update(&mut self, next: &SeriesRow) -> StoreResult<bool> {
        dispatch!(self, series_update(next))
    }

    /// The series whose game in play is this match (`status = 'playing'` and `next_match_id`), or
    /// `None`.
    pub async fn series_by_match(&mut self, match_id: &str) -> StoreResult<Option<SeriesRow>> {
        dispatch!(self, series_by_match(match_id))
    }

    /// The series one of whose `games` was played (or is being played) as this match, whatever the
    /// series' status, or `None`. The board's series banner reads it after a game has ended.
    pub async fn series_with_game(&mut self, match_id: &str) -> StoreResult<Option<SeriesRow>> {
        dispatch!(self, series_with_game(match_id))
    }

    /// The profile's series that is not over, or `None`. A profile is in at most one.
    pub async fn series_active_for(&mut self, profile_id: &str) -> StoreResult<Option<SeriesRow>> {
        dispatch!(self, series_active_for(profile_id))
    }

    /// Every series that is not over: the sweeper's input (R263).
    pub async fn series_active(&mut self) -> StoreResult<Vec<SeriesRow>> {
        dispatch!(self, series_active())
    }

    // -----------------------------------------------------------------------
    // ranked
    // -----------------------------------------------------------------------

    /// Serializes season opens: `open_season_in_tx` takes it before reading `ranked_seasons()`, so
    /// two opens racing in different transactions — even under different season ids — run one
    /// after the other and the second sees the first's row (and never soft-resets off a snapshot
    /// that predates it). Released when the transaction ends; a no-op where one process owns the
    /// store.
    pub async fn ranked_lock_seasons(&mut self) -> StoreResult<()> {
        dispatch!(self, ranked_lock_seasons())
    }

    /// Every season, oldest first.
    pub async fn ranked_seasons(&mut self) -> StoreResult<Vec<Season>> {
        dispatch!(self, ranked_seasons())
    }

    /// False, writing nothing, when a season of that id exists already (another process opened it).
    pub async fn ranked_create_season(&mut self, season: &Season) -> StoreResult<bool> {
        dispatch!(self, ranked_create_season(season))
    }

    /// R609: every profile that has played a rated game, with its hidden rating: the soft reset's
    /// input. Bots are not profiles and are never in it.
    pub async fn ranked_rated_players(&mut self) -> StoreResult<Vec<ResetPlayer>> {
        dispatch!(self, ranked_rated_players())
    }

    /// R609: writes a soft reset's ratings, every one in one statement.
    pub async fn ranked_reset_ratings(&mut self, changes: &[ResetChange]) -> StoreResult<()> {
        dispatch!(self, ranked_reset_ratings(changes))
    }

    /// Every row of a season, each with the player's current rating, in profile-id order.
    pub async fn ranked_standings(&mut self, season_id: &str) -> StoreResult<Vec<SeasonStanding>> {
        dispatch!(self, ranked_standings(season_id))
    }

    pub async fn ranked_rank(&mut self, season_id: &str, profile_id: &str) -> StoreResult<Option<SeasonRank>> {
        dispatch!(self, ranked_rank(season_id, profile_id))
    }

    /// Every season row this profile has, oldest season first: the profile's badges (R607).
    pub async fn ranked_ranks_of(&mut self, profile_id: &str) -> StoreResult<Vec<SeasonRank>> {
        dispatch!(self, ranked_ranks_of(profile_id))
    }

    /// Insert or replace one player's season row. Only that player's own rated games call it.
    /// `peak_jlorious` merges rather than replaces — keeps the better (lower) of the stored and
    /// written positions — because a bystander's `ranked_note_peak_jlorious` can land between this
    /// writer's read of the row and its write.
    pub async fn ranked_put_rank(&mut self, row: &SeasonRank) -> StoreResult<()> {
        dispatch!(self, ranked_put_rank(row))
    }

    /// R608: records that a player has held this Jlorious position, keeping the best. One targeted
    /// write, so it cannot undo a game the same player has just finished elsewhere.
    pub async fn ranked_note_peak_jlorious(
        &mut self,
        season_id: &str,
        profile_id: &str,
        position: i64,
    ) -> StoreResult<()> {
        dispatch!(self, ranked_note_peak_jlorious(season_id, profile_id, position))
    }

    /// R610: a bot's rating, or `None` before its first rated game.
    pub async fn ranked_bot(&mut self, bot_id: &str) -> StoreResult<Option<BotRating>> {
        dispatch!(self, ranked_bot(bot_id))
    }

    pub async fn ranked_put_bot(&mut self, bot: &BotRating) -> StoreResult<()> {
        dispatch!(self, ranked_put_bot(bot))
    }

    /// R611: one row per rated game. Rejects a second row for the same id.
    pub async fn ranked_record_game(&mut self, row: &RatedGameRow) -> StoreResult<()> {
        dispatch!(self, ranked_record_game(row))
    }

    pub async fn ranked_game(&mut self, id: &str) -> StoreResult<Option<RatedGameRow>> {
        dispatch!(self, ranked_game(id))
    }

    // -----------------------------------------------------------------------
    // tutorial (R320: tutorial progress kept on the account)
    // -----------------------------------------------------------------------

    /// The profile's row, or `None` before its first write.
    pub async fn tutorial_get(&mut self, profile_id: &str) -> StoreResult<Option<TutorialProgressRow>> {
        dispatch!(self, tutorial_get(profile_id))
    }

    /// R320: one atomic merge. The stored lessons become the union of the stored and the sent (a
    /// write never removes one), and the stored choice is replaced only by a strictly newer one. A
    /// profile with no row gets one. `max_lessons` is the caller's cap on the union
    /// (`TUTORIAL_LESSONS_MAX`).
    pub async fn tutorial_merge(
        &mut self,
        input: &TutorialMergeInput,
        max_lessons: i64,
    ) -> StoreResult<TutorialMergeOutcome> {
        dispatch!(self, tutorial_merge(input, max_lessons))
    }

    // -----------------------------------------------------------------------
    // playerSettings (R633: the player's game settings kept on the account)
    // -----------------------------------------------------------------------

    /// The profile's row, or `None` before its first write.
    pub async fn player_settings_get(&mut self, profile_id: &str) -> StoreResult<Option<PlayerSettingsRow>> {
        dispatch!(self, player_settings_get(profile_id))
    }

    /// R634: one atomic merge. For each group sent, the stored group becomes the sent one only when
    /// the sent one's `at` is strictly later; a group the write does not name is left as it was. A
    /// profile with no row gets one.
    pub async fn player_settings_merge(
        &mut self,
        input: &PlayerSettingsMergeInput,
        limits: &PlayerSettingsLimits,
    ) -> StoreResult<PlayerSettingsMergeOutcome> {
        dispatch!(self, player_settings_merge(input, limits))
    }

    // -----------------------------------------------------------------------
    // lastBoards (R417, R565: each profile's last finished game's board, per kind, C+ #29)
    // -----------------------------------------------------------------------

    /// The profile's last board of this kind, or `None` before its first finished game of it.
    pub async fn last_boards_get(
        &mut self,
        profile_id: &str,
        kind: LastBoardKind,
    ) -> StoreResult<Option<Vec<LastBoardEntry>>> {
        dispatch!(self, last_boards_get(profile_id, kind))
    }

    /// R565: replace it, or write the first one, as a game of that kind ends (epoch ms `at`).
    pub async fn last_boards_put(
        &mut self,
        profile_id: &str,
        kind: LastBoardKind,
        board: &[LastBoardEntry],
        at: i64,
    ) -> StoreResult<()> {
        dispatch!(self, last_boards_put(profile_id, kind, board, at))
    }

    /// R678: up to `count` non-empty `server` boards of profiles NOT in `exclude_profile_ids`, each
    /// from a different profile, chosen at random (the fake takes them in table order). Fewer when
    /// fewer exist.
    pub async fn last_boards_sample_others(
        &mut self,
        exclude_profile_ids: &[String],
        count: i64,
    ) -> StoreResult<Vec<Vec<LastBoardEntry>>> {
        dispatch!(self, last_boards_sample_others(exclude_profile_ids, count))
    }

    // -----------------------------------------------------------------------
    // gameRecords (R376: the card statistics' game records)
    // -----------------------------------------------------------------------

    /// R376: one record per id. False, and nothing written, when a record with this id exists.
    pub async fn game_records_insert(&mut self, record: &GameRecord) -> StoreResult<bool> {
        dispatch!(self, game_records_insert(record))
    }

    /// The records of the query's sources, mode and patch, in id order (code points).
    pub async fn game_records_list(&mut self, query: &GameRecordQuery) -> StoreResult<Vec<GameRecord>> {
        dispatch!(self, game_records_list(query))
    }

    // -----------------------------------------------------------------------
    // playerStats (R654: each profile's player statistics and privacy setting)
    // -----------------------------------------------------------------------

    pub async fn player_stats_get(&mut self, profile_id: &str) -> StoreResult<Option<PlayerStatsRow>> {
        dispatch!(self, player_stats_get(profile_id))
    }

    pub async fn player_stats_put(
        &mut self,
        profile_id: &str,
        stats: &IndexMap<String, Value>,
        is_private: bool,
        at: i64,
    ) -> StoreResult<()> {
        dispatch!(self, player_stats_put(profile_id, stats, is_private, at))
    }

    pub async fn player_stats_list_public(
        &mut self,
        options: &PlayerStatsListOptions,
    ) -> StoreResult<Vec<PublicPlayerSummary>> {
        dispatch!(self, player_stats_list_public(options))
    }
}
