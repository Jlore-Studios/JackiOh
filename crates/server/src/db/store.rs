//! The persistence port (SPEC §9.1, §9.2; `docs/v0.3.0/SURFACE.md` §4.3, §11.2, §11.3).
//! Everything under `api` and `actor` reads and writes the database
//! through the methods below and never through a driver, so the same handlers run against Postgres
//! (`pg`) and against the in-memory fake (`fake`) that the unit tests and `E2E=1` use.
//!
//! No traits: `Db` and `Tx` are enums with one variant per implementation, and every store
//! method is one method on `Tx`, named `<substore>_<method>`, whose body dispatches to `pg::<name>`
//! or `fake::<name>`. A transaction is
//!
//! ```ignore
//! let mut tx = app.db.begin(Some(profile_id)).await?;
//! // … tx.decks_upsert(&deck, MAX_SAVED_DECKS).await? …
//! tx.commit().await?;
//! ```
//!
//! and a `Tx` dropped without `commit` rolls back (Postgres: sqlx's `Transaction` drop; the fake:
//! `FakeTx` restores the snapshot it took at `begin`). A `Tx` is passed down, so nothing nests.
//!
//! Clocks and limits: `config.rs` (R79; §9.4, §9.5).
//!
//! Integers: epoch milliseconds, counts, caps and sequence numbers are `i64` (as Postgres's
//! `int8`/`count(*)` answer); ratings, deviations and volatilities are `f64` (migration
//! 0022 made them `double precision`).

use std::ops::{Index, IndexMut};
use std::sync::Arc;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use jackioh_engine::wire::{
    Action, ActionType, EmoteId, GameEventType, GameMode, GameOverReason, GameRecord, PerPlayer, PlayerId,
    PortraitId, SourceFilter, Winner,
};

use super::{fake, pg};
use crate::config::PersonaName;
use crate::ranked::glicko2::Glicko;
use crate::ranked::ladder::{RankTier, SeasonRank, VisibleRank};
use crate::ranked::season::{ResetChange, ResetPlayer};
use crate::username::render_username;

/// R417: one card of a last board — the card and its face, never its stats (C+ #29).
pub use jackioh_engine::state::LastBoardEntry;

/// R611: who played a side of a rated game: a person, or one of the AI bots (R610).
pub use jackioh_engine::wire::Pilot;

// Errors

/// What a store method can fail with. A refusal the port answers as a value (`RedeemResult`,
/// `UpsertOutcome`, a `false`, a `None`) is never an error.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// `results.insert`'s refusal of a second row for one match
    /// (§9.5), as its own variant so the writer can tell it from a real failure. A transaction whose
    /// `getByMatch` ran before a concurrent first writer committed only meets the duplicate here,
    /// and that collision is a clean no-op arriving the hard way — `results.rs` retries on it and
    /// finds the row the winner wrote. Postgres raises it as `unique_violation` (23505) on
    /// `results_pkey`. The field is the match id.
    #[error("result for match {0} is already recorded")]
    Duplicate(String),
    /// R776000: `matches.appendActions` was handed a seq the log already holds under another nonce, or
    /// a nonce it already holds at another seq, so the log has moved past the actor that wrote it (a
    /// write that committed and whose answer was lost, or a second writer). A nonce at its own seq is
    /// no clash: the log answers it with that row. The field is the message.
    #[error("{0}")]
    SeqClash(String),
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

// Serde helpers and the string unions

/// A field with three states — absent, `null`, a value — kept apart:
/// `None` is absent, `Some(None)` is `null`, `Some(Some(x))` is `x`. Used with
/// `#[serde(default, skip_serializing_if = "Option::is_none", with = "absent_or_null")]`.
pub(crate) mod absent_or_null {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer, T: Serialize>(
        value: &Option<Option<T>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
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

/// A string-literal union as a Rust enum: one unit variant per literal,
/// serialised as exactly that literal, with `as_str` (the text a SQL column or function holds),
/// `FromStr`, `Display` and `ALL` (declaration order).
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
            /// Every literal of the union, in declaration order.
            pub const ALL: &'static [$name] = &[ $( $name::$variant ),+ ];

            /// The literal itself, as the wire and the database hold it.
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

// Profiles

store_union! {
    pub enum ProfileStatus {
        Pending = "pending",
        Active = "active",
        Banned = "banned",
    }
}

store_union! {
    /// R1452: who holds a profile: a person, or a ladder bot. Server-only (`profiles.kind`, migration
    /// 0032, which no client role may read): no response carries it, so no read tells a bot's seat
    /// from a person's.
    #[derive(Default)]
    pub enum ProfileKind {
        #[default]
        Human = "human",
        Bot = "bot",
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    /// R1452: a person's profile, or a ladder bot's. Server-only, like the rating.
    #[serde(default)]
    pub kind: ProfileKind,
    /// The managed-auth user id.
    pub user_id: String,
    pub email: String,
    /// R1432, R1434: the username's base name, in its stored (NFKC) form: `Max` of `Max#3`. A new
    /// account's is `Player` (`USERNAME_DEFAULT_BASE`). Only ever written through the username
    /// routes; every read that shows a player joins it in from here (R1436).
    pub username_base: String,
    /// R1434: the key two base names clash on, `username::username_key` of the base.
    pub username_key: String,
    /// R1434: the tag, 1 or more, when the base was taken; none for a bare name.
    pub username_tag: Option<i64>,
    /// R1435: epoch ms of the last change of username, which starts the cooldown. None while the
    /// account still holds the default it was given.
    pub username_changed_at: Option<i64>,
    /// R1435: whether the prompt after activation has been answered, by a pick or a skip.
    pub username_prompted: bool,
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

impl Profile {
    /// The username as shown: `Max`, or `Max#3` when it carries a tag (R1432).
    pub fn username(&self) -> String {
        render_username(&self.username_base, self.username_tag)
    }
}

/// `profiles_create`'s argument. The new profile's username is the store's to give, the lowest
/// free `Player#n` (R1434).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileCreateInput {
    pub user_id: String,
    pub email: String,
    pub rating: f64,
    pub at: i64,
}

/// R1434, R1435: one claim of a username, `profiles_claim_username`'s argument. `base` is the
/// stored (NFKC) form `username::normalize_username` answered and `key` its `username_key`;
/// `expected_tag` is the tag the player's preview showed, so a claim whose outcome has changed
/// since is refused rather than handing the player a tag they were not shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsernameClaim {
    pub profile_id: String,
    pub base: String,
    pub key: String,
    pub expected_tag: Option<i64>,
    /// Epoch ms: when the claim is made, the time the cooldown runs from.
    pub at: i64,
    /// How long after a change the next is allowed (`USERNAME_CHANGE_COOLDOWN_MS`).
    pub cooldown_ms: i64,
}

/// What a claim of a username came to (R1434, R1435).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UsernameClaimOutcome {
    /// The name is the player's now, with this tag.
    Claimed { tag: Option<i64> },
    /// Nothing written: the name would carry this tag now, not the one the player was shown.
    Changed { tag: Option<i64> },
    /// Nothing written: the last change was too recent, and the next is allowed at this epoch ms.
    Cooldown { next_change_at: i64 },
}

// Invite codes (SPEC §9.4: codes stored hashed; §9.8: per-IP-hash limits)

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

/// §9.8's invite-cluster watch: what the attempt log holds for one IP hash in a window. `ip_hash` is
/// the stored (peppered) hash in full; only `jackioh-server invite-clusters` shortens it, and
/// nothing returns it to a client.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InviteCluster {
    pub ip_hash: String,
    /// Distinct profiles that made an attempt from this hash; attempts with no profile are not counted.
    pub accounts: i64,
    /// Every attempt from this hash, whoever made it and however it ended.
    pub attempts: i64,
}

store_union! {
    /// Exactly the strings `app.redeem_invite_code(uuid, text, text)` returns (migration 0001 §6).
    /// `not_pending` covers no such profile, banned and already active together: `codes.rs` tells
    /// them apart first (R145), so here it is a race, answered as a conflict (R170).
    /// `email_unverified` is the database's second opinion (R159); `circuit_open` is its half of
    /// §9.4's breaker, beside the server's (R106). Missing, revoked, expired and exhausted codes
    /// all answer `invalid_code` (§9.4).
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
    /// `None` when the string could never be a code (R104): it is passed down, not refused early,
    /// because §9.4 logs the attempt (step 4) before the lookup (step 5), so a malformed code still
    /// costs a row in `code_attempts`. Both implementations answer `invalid_code` for it.
    pub code_hash: Option<String>,
    pub ip_hash: String,
}

// The collection (§9.4: an entitlement ledger)

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

// Saved decks and trios (SPEC §9.4, R250–R256). They replace the single three-deck loadout: a
// profile keeps up to `MAX_SAVED_DECKS` named decks and builds up to `MAX_SAVED_TRIOS` trios from
// them. Both are drafts (R250, R252): a save checks structure only, and legality is judged when a
// deck or a trio is queued (R253).

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
pub type TrioSlots = (Option<String>, Option<String>, Option<String>);

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

// Queue modes (SPEC §9.5, R257) and what a ticket or a room freezes.

store_union! {
    /// R257: a ticket pairs only with a ticket of the same mode.
    pub enum QueueMode {
        Bo1 = "bo1",
        Bo3 = "bo3",
        Random = "random",
    }
}

/// One value per mode, every mode present (as `PerPlayer` is one per seat). Serialises as
/// `{ "bo1": …, "bo3": …, "random": … }`.
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
    pub decks: (FrozenDeck, FrozenDeck, FrozenDeck),
}

// Matches

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
    /// R768: `hash_state` of the final state, written with the result (`matches_record_final_hash`,
    /// migration 0035) and never by `matches_create`. Absent on a match that ended before it, on one
    /// the reaper resolved (R112) and on one not over yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_hash: Option<String>,
}

store_union! {
    /// R768: where a match stands, whatever its row's status: `open` while a claimed room or a queue
    /// pair holds its id and no actor plays it yet, `live` while one does, `over` once it ended.
    pub enum MatchPhase {
        Open = "open",
        Live = "live",
        Over = "over",
    }
}

/// R768: a match's phase and who holds a seat in it, an `open` one included (`matches_get` reads
/// only live and finished matches).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MatchSeats {
    pub phase: MatchPhase,
    /// The profiles seated so far, seat order; a seat not filled yet and a deleted account's are
    /// left out.
    pub players: Vec<String>,
}

/// R768: a finished match with a result, as its replay reads it: the row, the result and how many
/// actions its log still holds (none once the retention purge has taken it).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReplayRow {
    #[serde(rename = "match")]
    pub row: MatchRow,
    pub result: ResultRow,
    pub actions: i64,
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

// Rooms and tickets

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
    /// R1372: an All Random host's "More cards from the newest set", for the deck dealt to their seat
    /// when the room is joined (`matches.room_lean_newest`, migration 0027). False in the other modes.
    #[serde(default)]
    pub host_lean_newest: bool,
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
    /// R1372: an All Random ticket's "More cards from the newest set", for the deck dealt to its seat
    /// when it is paired (`tickets.lean_newest`, migration 0027). False in the other modes.
    #[serde(default)]
    pub lean_newest: bool,
    pub catalog_version: String,
    pub enqueued_at: i64,
    pub status: TicketStatus,
    pub match_id: Option<String>,
}

/// R640002: how long one claimed ticket waited, from its enqueue to its claim, in ms.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TicketWait {
    pub mode: QueueMode,
    pub waited_ms: i64,
}

// Results

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

// Game records for the card statistics (SPEC §9.11, R376–R378). Server-only (migration 0014).

/// R377, R378: which records a read returns. Pilots are per seat, so `card_stats` applies those.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameRecordQuery {
    pub source: SourceFilter,
    /// One match type, or null for every mode.
    pub mode: Option<GameMode>,
    /// One patch, or null for every patch.
    pub patch: Option<String>,
}

// The Conquest series (SPEC §9.5, R330–R338, R262–R264). One row per series, persisted so a series
// survives a server restart; every transition is a pure function in `api/series_rules.rs` written
// back with `series_update`, which is compare-and-set on `version`.

/// A series seat. Index 0 of `SeriesRow.sides` is `p1`; it is not the seat a game's match uses.
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
    /// Null while the game is being played.
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

// The ranked ladder (SPEC §9.12, R603–R612): seasons, each player's season on the ladder, the bots'
// ratings, and the record of every rated game. The rules are `ranked/*`, pure; `api/ranked.rs`
// reads and writes through this port.

/// R609: a season, named by the minor version of the game (`v0.2`), and the patch that opened it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Season {
    pub id: String,
    pub patch_version: String,
    pub started_at: i64,
}

/// One player's season row with their current hidden rating: what percentiles and Jlorious read.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeasonStanding {
    #[serde(flatten)]
    pub rank: SeasonRank,
    pub rating: f64,
    /// R640000: a ladder bot's row; never serialised when false. Jlorious and the percentiles read
    /// the rows without it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bot: bool,
}

/// R610: an AI bot's own Glicko-2 rating. Its ladder rank is a season row like a person's, read
/// against people's ratings and never in Jlorious (R640000).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BotRating {
    pub bot_id: String,
    pub glicko: Glicko,
    pub games: i64,
    pub updated_at: i64,
}

/// R1454: one Best-of-1 deck a ladder bot carries, and the portrait it plays it under (R642).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BotDeck {
    pub cards: Vec<String>,
    pub portrait: String,
}

/// R1453, R1454, R1452: a ladder bot as the queue, the actor and the pool read it: its `bots` row (migration
/// 0032), its profile's `created_at` and `current_match_id`, and its R610 rating. Server-only.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BotRow {
    /// The bot's profile, which is also its R610 bot id.
    pub profile_id: String,
    /// Its index in `BOT_STRENGTHS`.
    pub strength: usize,
    /// Its think times' multiplier (R1452).
    pub speed: f64,
    pub persona: PersonaName,
    /// Its schedule (R1454): on from this hour of the day (UTC) for `active_hours` hours.
    pub active_from_hour: i32,
    pub active_hours: i32,
    /// Its Best-of-1 decks; empty until the pool deals them.
    pub decks: Vec<BotDeck>,
    /// The catalog version its decks were dealt for, or none before they were.
    pub decks_catalog_version: Option<String>,
    /// R641000: its Conquest trio (three decks sharing no card, R253); none until the pool deals it.
    #[serde(default)]
    pub trio: Option<FrozenTrio>,
    /// R641000: the catalog version its trio was dealt for, or none before it was.
    #[serde(default)]
    pub trio_catalog_version: Option<String>,
    /// Epoch ms: when its account reads as made.
    pub created_at: i64,
    /// Epoch ms it was retired (R1454), or none while it is in the pool.
    pub retired_at: Option<i64>,
    /// Its R610 rating (`bot_ratings`), `RATING_START` before it has a row.
    pub rating: f64,
    /// The match it is in, as for a profile (§9.5).
    pub in_match_id: Option<String>,
}

/// R1454: a bot `bots_create` writes: its profile id, its `bots` row and the rating its
/// `bot_ratings` row starts at (R610), with a new player's deviation and volatility.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewBot {
    pub profile_id: String,
    pub strength: usize,
    pub speed: f64,
    pub persona: PersonaName,
    pub active_from_hour: i32,
    pub active_hours: i32,
    pub decks: Vec<BotDeck>,
    pub decks_catalog_version: Option<String>,
    pub created_at: i64,
    pub seed_rating: f64,
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

/// How a rated game ended: the match's reason, or the series' (R334), whose literals are disjoint.
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

// Tutorial progress on the account (SPEC §9.10, R320). The device keeps its own copy (R294) and the
// client merges the two (R321); this is the account's half, which only ever grows.

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

// Player settings on the account (SPEC §9.1, R633, R634). The device keeps its own copy and the
// client merges the two; this is the account's half, where a group is replaced only by a newer one.

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

// Last boards (R417, R565, R678)

store_union! {
    /// R417: "your last game" is your last finished game of the same kind; only `server` is
    /// written today.
    pub enum LastBoardKind {
        Server = "server",
        Practice = "practice",
    }
}

// Player statistics on the account (SPEC §9.11, R639, R654).

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStatsRow {
    pub profile_id: String,
    pub stats: IndexMap<String, Value>,
    pub is_private: bool,
    pub updated_at: i64,
}

// Consent on the account (SPEC §9.4, R757000, R757001, migration 0034).

store_union! {
    /// R757001: how an account came to agree to a version of the terms.
    pub enum TermsSource {
        /// While setting up the account: the sign-up box, or the consent screen of a pending one.
        Signup = "signup",
        /// Again, after the terms changed.
        Reaccept = "reaccept",
        /// Migration 0034's row for an account that existed before the box did.
        Baseline = "baseline",
    }
}

/// One version of the terms an account agreed to (`public.terms_acceptances`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TermsAcceptanceRow {
    pub version: String,
    pub source: TermsSource,
    pub accepted_at: i64,
}

/// What an account has agreed to: its age check (R757000) and every version of the terms
/// (R757001), oldest first. Never a birth date: none is stored.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConsentRow {
    pub profile_id: String,
    /// When the age check passed, or `None` when the account never answered it.
    pub age_checked_at: Option<i64>,
    /// The minimum age it was checked against, beside `age_checked_at`.
    pub age_min: Option<i32>,
    pub terms: Vec<TermsAcceptanceRow>,
}

/// One of `PublicPlayerSummary.favourite_cards`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FavouriteCard {
    pub id: String,
    pub count: i64,
}

/// `PublicPlayerSummary.fun_stats`.
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
    /// The player's username as shown (R1436), joined in from `profiles`, so a rename shows here
    /// at once with no stats row rewritten.
    pub username: String,
    pub games: i64,
    pub wins: i64,
    pub losses: i64,
    pub draws: i64,
    pub win_rate: Option<f64>,
    pub favourite_cards: Vec<FavouriteCard>,
    pub fun_stats: FunStats,
    pub updated_at: i64,
}

/// `player_stats_list_public`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStatsListOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

// The retention purge

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPurgeInput {
    pub code_attempts_before: i64,
    pub match_actions_ended_before: i64,
    /// R1442: the play telemetry of every match that ended before this goes.
    pub play_telemetry_ended_before: i64,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPurgeResult {
    pub code_attempts: i64,
    pub match_actions: i64,
    /// R1442: the rows of the three telemetry tables, together.
    pub play_telemetry: i64,
}

// Play telemetry (R1442, migration 0029). Server-only, keyed by match and seat, naming no profile.
// `seat` is always the seat the account BEGAN the match in (R677), the one `MatchRow.players` names.

/// R1442: one move a seat made, and how long it took (`public.action_timings`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActionTimingRow {
    pub match_id: String,
    pub seat: PlayerId,
    /// The move's `match_actions.seq`.
    pub seq: i64,
    pub action_kind: ActionType,
    /// How many actions the seat could have taken instead (`legal_actions` before the move).
    pub legal_count: i32,
    pub turn: i32,
    /// From the push of the view that made it the seat's move to the move, epoch ms apart.
    pub think_ms: i64,
    /// The seat's clock as the move arrived; absent when no clock was read (a backfilled row).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock_left_ms: Option<i64>,
    pub first_in_turn: bool,
    /// The player's ladder tier as the match began; absent when none was read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank_bucket: Option<RankTier>,
    pub pilot: Pilot,
}

/// R1442: one emote relayed before the result (`public.emote_events`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EmoteEventRow {
    pub match_id: String,
    pub seat: PlayerId,
    /// The emote's place among the match's emotes, from 0.
    pub ordinal: i32,
    pub emote_id: EmoteId,
    pub turn: i32,
    /// The last game event before the emote; absent when the match had none yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_event: Option<GameEventType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ms_since_trigger: Option<i64>,
    /// When the opponent emoted since this seat last did: how long after it this one came.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to_opponent_ms: Option<i64>,
    pub pilot: Pilot,
}

/// R1442: how the match ended for one seat (`public.match_signals`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MatchSignalRow {
    pub match_id: String,
    pub seat: PlayerId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conceded_turn: Option<i32>,
    /// How far behind the seat stood as it conceded: the AI's evaluation of the seat, negated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concede_eval_deficit: Option<f64>,
    pub draw_offers: i32,
    pub draw_accepted: bool,
    pub rematch_offered: bool,
    pub rematch_accepted: bool,
    pub timeouts: i32,
    pub pilot: Pilot,
}

/// R639000: one seat's signals beside how its match ended, what `timing-fit` fits the concessions,
/// draws, timeouts and rematches on.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignalOutcomeRow {
    pub signal: MatchSignalRow,
    /// Whether the account that began in the seat won (the match's seat, as the signals row's is,
    /// not the result's credited one after a Glitch's swap, R677); `None` when the match had no winner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub won: Option<bool>,
    pub reason: GameOverReason,
    /// The match's last turn, as its result holds it.
    pub turns: i64,
    /// Whether a rematch could be offered from the match (R672): it was ranked, as every match a
    /// ladder bot plays is, and no Conquest series holds it.
    pub rematchable: bool,
}

/// R1442: one match's telemetry, as the actor writes it with the result.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayTelemetry {
    pub action_timings: Vec<ActionTimingRow>,
    pub emote_events: Vec<EmoteEventRow>,
    pub match_signals: Vec<MatchSignalRow>,
}

// Live matches: what `queue.rs`, the room endpoints, the series and the rematch hand
// `actor::registry::Registry::start`.

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

// The store: Db (a pool or the fake's tables) and Tx (one transaction)

/// Migration 0001 §8, and the same closing note in 0002-0009: `service_role` is the role the API
/// server and the match actor hold. It is the only role granted EXECUTE on
/// `app.redeem_invite_code`, `app.upsert_deck`, `app.append_match_action`, `app.claim_ticket_pair`
/// and the rest, so running as it is not a formality — a call that gets it wrong fails with
/// `insufficient_privilege` instead of succeeding because the connection happened to own the table.
const ACTING_ROLE: &str = "service_role";

/// One statement, two `SET LOCAL`s. `set_config(name, value, true)` is `SET LOCAL name = value`,
/// and unlike `SET LOCAL` it takes parameters, so the profile id is bound rather than interpolated.
/// `request.jwt.claim.sub` is the GUC `auth.uid()` reads (`tests/sql/00_supabase_stub.sql`).
/// `service_role` bypasses RLS, but a policy, trigger or SECURITY INVOKER helper that consults the
/// caller then sees the right profile. `SET LOCAL` is a silent no-op outside a transaction, so
/// `Db::begin` is the only way to a `Tx`: `BEGIN` first, and no SQL runs outside a `Tx`.
const SESSION_SQL: &str =
    "select set_config('role', $1, true), set_config('request.jwt.claim.sub', $2, true)";

/// The store: Postgres behind a pool, or the fake's tables behind one lock (unit tests and
/// `E2E=1`). Cloning shares the pool or the tables.
#[derive(Clone)]
pub enum Db {
    Pg(sqlx::PgPool),
    Fake(Arc<tokio::sync::Mutex<fake::FakeData>>),
}

/// One transaction on the store. Every store method is a method here. Dropped without
/// `commit`, it rolls back.
pub enum Tx<'a> {
    Pg(sqlx::Transaction<'a, sqlx::Postgres>),
    Fake(fake::FakeTx<'a>),
}

impl Db {
    /// An empty fake store: the in-memory tables the unit tests and `E2E=1` run on.
    pub fn fake() -> Db {
        Db::Fake(Arc::new(tokio::sync::Mutex::new(fake::FakeData::default())))
    }

    /// Opens one transaction. Postgres: `BEGIN`, then the role switch to `service_role` and
    /// `request.jwt.claim.sub` set to `claim_sub` — the profile the call is about, or `""` for the
    /// calls that are about nobody (`tickets_list_open`, `matches_live`, the breaker's
    /// `codes_count_failures`) — in the one `SESSION_SQL` statement. The fake:
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

    /// Lets the pool's connections go, for a CLI about to exit. A no-op on the fake.
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

    // Root

    /// SPEC §9.4's redemption, whole: one transaction over `profiles`, `invite_codes` and
    /// `code_attempts`; in Postgres one statement, `app.redeem_invite_code` (migration 0001 §6),
    /// whose return values are the result type. Steps 2 and 3's limits run in the database, not a
    /// handler (`config.rs` has the fake's copy; `pg.rs`'s KNOWN DIVERGENCES, "redemption limits").
    /// Steps 2 and 3 reject BEFORE step 4, so a caller over the limit cannot pin their own counter
    /// by retrying, and a rejection is RETURNED, never thrown, so step 4's attempt row is never
    /// rolled back. §9.4's "identical time" is R107's response floor in `codes.rs`, not SQL's.
    pub async fn redeem(&mut self, input: &RedeemInviteCodeInput) -> StoreResult<RedeemResult> {
        dispatch!(self, redeem(input))
    }

    /// The retention purge (`api/retention.rs`): deletes `code_attempts` rows made before
    /// `code_attempts_before` and the action log of every match that ended before
    /// `match_actions_ended_before` (epoch ms), and the play telemetry of every match that ended
    /// before `play_telemetry_ended_before` (R1442). Results, and so ratings, are kept. Answers how
    /// many rows of each went. In Postgres this is `app.purge_expired_rows` (migration 0013, given
    /// its third cutoff by 0029), the one path the append-only guard on `match_actions` lets a
    /// delete through.
    pub async fn purge_expired(&mut self, input: &RetentionPurgeInput) -> StoreResult<RetentionPurgeResult> {
        dispatch!(self, purge_expired(input))
    }

    // profiles

    pub async fn profiles_get_by_id(&mut self, profile_id: &str) -> StoreResult<Option<Profile>> {
        dispatch!(self, profiles_get_by_id(profile_id))
    }

    pub async fn profiles_get_by_user_id(&mut self, user_id: &str) -> StoreResult<Option<Profile>> {
        dispatch!(self, profiles_get_by_user_id(user_id))
    }

    pub async fn profiles_get_many(&mut self, profile_ids: &[String]) -> StoreResult<Vec<Profile>> {
        dispatch!(self, profiles_get_many(profile_ids))
    }

    /// R1443: the one profile whose username has this key (`username::username_key` of its base)
    /// and tag, the pair R1434 holds unique, or none. A sign-in by username reads it for the address.
    pub async fn profiles_get_by_username(
        &mut self,
        key: &str,
        tag: Option<i64>,
    ) -> StoreResult<Option<Profile>> {
        dispatch!(self, profiles_get_by_username(key, tag))
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

    /// R1434: the tag a claim of the base whose key is `key` would carry now: none when no other
    /// profile holds the bare name, otherwise the lowest tag from 1 that no other profile holds.
    /// `profile_id`'s own name never counts as taken against it. A read: nothing is reserved.
    pub async fn profiles_username_tag_for(
        &mut self,
        profile_id: &str,
        key: &str,
    ) -> StoreResult<Option<i64>> {
        dispatch!(self, profiles_username_tag_for(profile_id, key))
    }

    /// R1434, R1435: claims a username for `claim.profile_id`, under a transaction-level lock on
    /// the key, so two claims of one name at the same moment get different tags. Refused while the
    /// cooldown runs (the first change away from the default is never refused), and refused when
    /// the tag the name would carry is not `claim.expected_tag`. A claim that lands starts the
    /// cooldown and answers the prompt.
    pub async fn profiles_claim_username(
        &mut self,
        claim: &UsernameClaim,
    ) -> StoreResult<UsernameClaimOutcome> {
        dispatch!(self, profiles_claim_username(claim))
    }

    /// R1435: answers the prompt after activation without changing the name ("Skip for now").
    pub async fn profiles_answer_username_prompt(&mut self, profile_id: &str) -> StoreResult<()> {
        dispatch!(self, profiles_answer_username_prompt(profile_id))
    }

    /// Pass `None` to clear. §9.5: every terminal reason clears both players'.
    pub async fn profiles_set_in_match(
        &mut self,
        profile_id: &str,
        match_id: Option<&str>,
    ) -> StoreResult<()> {
        dispatch!(self, profiles_set_in_match(profile_id, match_id))
    }

    /// Deletes the profile and everything that is only its own: collection and its ledger, decks,
    /// trios, tutorial progress, its age check and terms acceptances (R757000, R757001), queue
    /// tickets, rooms it opened that nobody joined, and its invite-code attempts' link to it. Finished matches, results and series stay for the other
    /// player, with this profile's seat left empty (migration 0012). False when there was no such
    /// profile. The caller refuses a profile in a live match or series first.
    pub async fn profiles_remove(&mut self, profile_id: &str) -> StoreResult<bool> {
        dispatch!(self, profiles_remove(profile_id))
    }

    // codes

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

    pub async fn codes_count_attempts_by_profile(
        &mut self,
        profile_id: &str,
        since: i64,
    ) -> StoreResult<i64> {
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

    /// §9.8: one row per IP hash with an attempt at or after `since`, ordered by `ip_hash`: how many
    /// distinct accounts and how many attempts it made. Read only by the operator's
    /// `invite-clusters` report; nothing acts on it.
    pub async fn codes_invite_clusters(&mut self, since: i64) -> StoreResult<Vec<InviteCluster>> {
        dispatch!(self, codes_invite_clusters(since))
    }

    /// §9.4: the server-side circuit breaker's input — system-wide failures in a window (R106).
    pub async fn codes_count_failures(&mut self, since: i64) -> StoreResult<i64> {
        dispatch!(self, codes_count_failures(since))
    }

    // collection (§9.4: an entitlement ledger. `collection_upsert_quantities` and
    // `collection_append_grants` are the two writes every mutation makes, and callers must make
    // them inside one transaction.)

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

    // decks

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

    // trios

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

    // matches

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

    /// R1450: the reaper's open-row sweep. Deletes every `open` row whose `ceiling_at` is before
    /// `before` (a room past its expiry, claimed or not, and a queue pairing's reservation claimed before
    /// it) whose match never started, freeing its room code (R110) and letting go of the in-match flags
    /// and ticket links that pointed at it. Never touches a live or finished match. Answers how many rows
    /// went.
    pub async fn matches_reap_open(&mut self, before: i64) -> StoreResult<u64> {
        dispatch!(self, matches_reap_open(before))
    }

    /// R768: the final state's hash, written with the result and only once: a no-op on a match
    /// that has one already, and on an unknown id.
    pub async fn matches_record_final_hash(&mut self, match_id: &str, final_hash: &str) -> StoreResult<()> {
        dispatch!(self, matches_record_final_hash(match_id, final_hash))
    }

    /// R768: the match's phase and its seated profiles, in every status, an `open` reservation
    /// included; `None` for an id no match, room or queue pair holds.
    pub async fn matches_seats(&mut self, match_id: &str) -> StoreResult<Option<MatchSeats>> {
        dispatch!(self, matches_seats(match_id))
    }

    // replays (R768)

    /// The finished matches `profile_id` held a seat in that have a result and a log the retention
    /// purge has not taken, newest ended first (the id breaking a tie), `limit` of them from
    /// `offset`.
    pub async fn replays_list(
        &mut self,
        profile_id: &str,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<ReplayRow>> {
        dispatch!(self, replays_list(profile_id, limit, offset))
    }

    /// One finished match with a result, whatever is left of its log; `None` for any other id.
    pub async fn replays_get(&mut self, match_id: &str) -> StoreResult<Option<ReplayRow>> {
        dispatch!(self, replays_get(match_id))
    }

    // rooms

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

    // tickets

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

    /// R257: open tickets per mode. Every mode is present. No route answers it (R640002).
    pub async fn tickets_count_open_by_mode(&mut self) -> StoreResult<PerMode<i64>> {
        dispatch!(self, tickets_count_open_by_mode())
    }

    /// R640002: the wait of every ticket claimed at or after `since` (epoch ms), whether a person's
    /// or a bot's claim, oldest claim first, ties on ticket id.
    pub async fn tickets_recent_waits(&mut self, since: i64) -> StoreResult<Vec<TicketWait>> {
        dispatch!(self, tickets_recent_waits(since))
    }

    /// §9.5: "both tickets are claimed in one atomic statement". Returns false unless both were
    /// still open, so two concurrent matchers cannot pair the same ticket twice.
    pub async fn tickets_claim_pair(
        &mut self,
        a_id: &str,
        b_id: &str,
        match_id: &str,
        at: i64,
    ) -> StoreResult<bool> {
        dispatch!(self, tickets_claim_pair(a_id, b_id, match_id, at))
    }

    pub async fn tickets_cancel(&mut self, ticket_id: &str, at: i64) -> StoreResult<()> {
        dispatch!(self, tickets_cancel(ticket_id, at))
    }

    // results

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

    // series

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

    // ranked

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

    pub async fn ranked_rank(
        &mut self,
        season_id: &str,
        profile_id: &str,
    ) -> StoreResult<Option<SeasonRank>> {
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

    // bots (R1453, R1454, R1451, R1452, migration 0032): the ladder bots

    /// R1454: every ladder bot, retired ones included, oldest profile first (ties on id).
    pub async fn bots_list(&mut self) -> StoreResult<Vec<BotRow>> {
        dispatch!(self, bots_list())
    }

    /// R1454: a new bot in one write: an `active` profile of kind `bot` made at `created_at`, with
    /// the default username a new account gets (the lowest free `Player#n`, R1434) and no auth user;
    /// its `bots` row; and its `bot_ratings` row at `seed_rating` with a new player's deviation and
    /// volatility and no games (R610).
    pub async fn bots_create(&mut self, bot: &NewBot) -> StoreResult<()> {
        dispatch!(self, bots_create(bot))
    }

    /// R1454: retires the bot at `at`, or puts it back in the pool with `None`. A retired bot
    /// finishes the match it is in.
    pub async fn bots_set_retired(&mut self, profile_id: &str, at: Option<i64>) -> StoreResult<()> {
        dispatch!(self, bots_set_retired(profile_id, at))
    }

    /// R1454: the bot's Best-of-1 decks, dealt for `catalog_version`.
    pub async fn bots_set_decks(
        &mut self,
        profile_id: &str,
        decks: &[BotDeck],
        catalog_version: &str,
    ) -> StoreResult<()> {
        dispatch!(self, bots_set_decks(profile_id, decks, catalog_version))
    }

    /// R641000: the bot's Conquest trio, dealt for `catalog_version`.
    pub async fn bots_set_trio(
        &mut self,
        profile_id: &str,
        trio: &FrozenTrio,
        catalog_version: &str,
    ) -> StoreResult<()> {
        dispatch!(self, bots_set_trio(profile_id, trio, catalog_version))
    }

    /// R1453: claims one open ticket for a bot, single-shot as `tickets_claim_pair` is: false unless
    /// the bot is free (in no match, a side of no series that is not over (R641000), and not
    /// retired) and the ticket still open. On true the ticket is matched to `match_id`, the `open`
    /// match skeleton is written and the bot is put in it; the ticket's owner is the caller's to put
    /// in the match.
    pub async fn tickets_claim_for_bot(
        &mut self,
        ticket_id: &str,
        bot_id: &str,
        match_id: &str,
        at: i64,
    ) -> StoreResult<bool> {
        dispatch!(self, tickets_claim_for_bot(ticket_id, bot_id, match_id, at))
    }

    /// R641000: `tickets_claim_for_bot` for a Conquest (`bo3`) ticket, but the bot is put in no
    /// match: `match_id` is game 1's reserved id, and the series the caller writes in the same
    /// transaction is what holds the bot between games. False for a ticket of another mode.
    pub async fn tickets_claim_for_bot_series(
        &mut self,
        ticket_id: &str,
        bot_id: &str,
        match_id: &str,
        at: i64,
    ) -> StoreResult<bool> {
        dispatch!(
            self,
            tickets_claim_for_bot_series(ticket_id, bot_id, match_id, at)
        )
    }

    /// R639000: claims a bot for the rematch `match_id` of a match `players` played, single-shot as
    /// `tickets_claim_for_bot` is, with no ticket: false unless the bot is free (in no match, a side
    /// of no series that is not over (R641000), and not retired), whatever its schedule. On true the
    /// `open` match skeleton is written for `players` and the bot is put in it; the person is the
    /// caller's to put in the match.
    pub async fn bots_claim_for_rematch(
        &mut self,
        bot_id: &str,
        players: &(String, String),
        match_id: &str,
        catalog_version: &str,
        at: i64,
    ) -> StoreResult<bool> {
        dispatch!(
            self,
            bots_claim_for_rematch(bot_id, players, match_id, catalog_version, at)
        )
    }

    /// R1454: lets go every bot still in a match it was claimed for before `before` (epoch ms) that
    /// never started (still the claim's `open` skeleton: a start a crash or a restart cut off). The
    /// skeleton goes, and with it the in-match state of both its profiles, as `matches_discard_open`
    /// lets a failed start's go; the ticket stays claimed. Answers the bots let go.
    pub async fn bots_release_stale_starts(&mut self, before: i64) -> StoreResult<Vec<String>> {
        dispatch!(self, bots_release_stale_starts(before))
    }

    /// R1453: the profiles `profile_id` faced in its last `limit` finished matches, newest first.
    pub async fn results_recent_opponents(
        &mut self,
        profile_id: &str,
        limit: usize,
    ) -> StoreResult<Vec<String>> {
        dispatch!(self, results_recent_opponents(profile_id, limit))
    }

    /// R1454: how many people (bots left out) finished a match at or after `since` (epoch ms).
    pub async fn results_count_active_humans(&mut self, since: i64) -> StoreResult<i64> {
        dispatch!(self, results_count_active_humans(since))
    }

    /// R640002: how many matches ended at or after `since` (epoch ms), games against a bot included.
    pub async fn results_count_since(&mut self, since: i64) -> StoreResult<i64> {
        dispatch!(self, results_count_since(since))
    }

    /// R611: one row per rated game. Rejects a second row for the same id.
    pub async fn ranked_record_game(&mut self, row: &RatedGameRow) -> StoreResult<()> {
        dispatch!(self, ranked_record_game(row))
    }

    pub async fn ranked_game(&mut self, id: &str) -> StoreResult<Option<RatedGameRow>> {
        dispatch!(self, ranked_game(id))
    }

    /// R748000: every rated game with a bot side that ended at or after `since` (epoch ms), oldest
    /// first, ties on id.
    pub async fn ranked_bot_games_since(&mut self, since: i64) -> StoreResult<Vec<RatedGameRow>> {
        dispatch!(self, ranked_bot_games_since(since))
    }

    // tutorial (R320: tutorial progress kept on the account)

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

    // playerSettings (R633: the player's game settings kept on the account)

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

    // lastBoards (R417, R565: each profile's last finished game's board, per kind, C+ #29)

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

    // gameRecords (R376: the card statistics' game records)

    /// R376: one record per id. False, and nothing written, when a record with this id exists.
    pub async fn game_records_insert(&mut self, record: &GameRecord) -> StoreResult<bool> {
        dispatch!(self, game_records_insert(record))
    }

    /// The records of the query's sources, mode and patch, in id order (code points).
    pub async fn game_records_list(&mut self, query: &GameRecordQuery) -> StoreResult<Vec<GameRecord>> {
        dispatch!(self, game_records_list(query))
    }

    // playerStats (R654: each profile's player statistics and privacy setting)

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

    // consent (R757000, R757001: the age check and the terms agreed to, migration 0034)

    /// The account's age check and terms acceptances; a row with neither when nothing is stored.
    pub async fn consent_get(&mut self, profile_id: &str) -> StoreResult<ConsentRow> {
        dispatch!(self, consent_get(profile_id))
    }

    /// R757000: records that the account passed the age check against `min_age`. The first record
    /// stands: a second call changes nothing.
    pub async fn consent_record_age(&mut self, profile_id: &str, min_age: i32, at: i64) -> StoreResult<()> {
        dispatch!(self, consent_record_age(profile_id, min_age, at))
    }

    /// R757001: records that the account agreed to `version`. Each version is recorded once; a
    /// second call for it changes nothing.
    pub async fn consent_record_terms(
        &mut self,
        profile_id: &str,
        version: &str,
        source: TermsSource,
        at: i64,
    ) -> StoreResult<()> {
        dispatch!(self, consent_record_terms(profile_id, version, source, at))
    }

    // playTelemetry (R1442: how each seat played a match, migration 0029)

    /// Writes one match's telemetry. A row whose key is already held is left as it stands, so a
    /// repeated write changes nothing, and a row of a seat whose account has been deleted is left
    /// out, so no write after the deletion puts back what it removed. Refused, with nothing written,
    /// when a row names a match the store does not hold (the foreign key on `matches`).
    pub async fn play_telemetry_insert(&mut self, telemetry: &PlayTelemetry) -> StoreResult<()> {
        dispatch!(self, play_telemetry_insert(telemetry))
    }

    /// R672: the account that began the match in `seat` offered a rematch; `made` when that offer
    /// made one. A no-op when the seat has no signals row yet.
    pub async fn play_telemetry_note_rematch(
        &mut self,
        match_id: &str,
        seat: PlayerId,
        made: bool,
    ) -> StoreResult<()> {
        dispatch!(self, play_telemetry_note_rematch(match_id, seat, made))
    }

    /// One match's telemetry: its timings by seq, its emotes by ordinal, its signals p1 first.
    pub async fn play_telemetry_of(&mut self, match_id: &str) -> StoreResult<PlayTelemetry> {
        dispatch!(self, play_telemetry_of(match_id))
    }

    /// Every action timing held, by match id then seq, for `timing-fit`.
    pub async fn play_telemetry_timings(&mut self) -> StoreResult<Vec<ActionTimingRow>> {
        dispatch!(self, play_telemetry_timings())
    }

    /// R639000: every emote event held, by match id then ordinal, for `timing-fit`.
    pub async fn play_telemetry_emotes(&mut self) -> StoreResult<Vec<EmoteEventRow>> {
        dispatch!(self, play_telemetry_emotes())
    }

    /// R639000: every signals row held beside its match's result, by match id then seat (p1 first),
    /// for `timing-fit`. A row whose match has no result yet is left out.
    pub async fn play_telemetry_outcomes(&mut self) -> StoreResult<Vec<SignalOutcomeRow>> {
        dispatch!(self, play_telemetry_outcomes())
    }

    /// The finished matches whose action log is still held, that have no action timing yet and
    /// whose accounts are not both deleted, by when they ended and then by id: what
    /// `timing-backfill` folds.
    pub async fn play_telemetry_unfolded(&mut self) -> StoreResult<Vec<String>> {
        dispatch!(self, play_telemetry_unfolded())
    }
}

// ---- Social: friends, presence, challenges, chat (#937, #938, #941) ----
// SPEC S7 (migration 0036): friend requests, friendships, blocks, presence, challenges and chat,
// and the spectator setting on the profile. The rules are `social/*`; these are their rows, and
// the methods below the one `impl Tx` they need.

/// B43: a pending friend request (`public.friend_requests`), one per direction and pair.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FriendRequestRow {
    pub id: String,
    pub from_profile: String,
    pub to_profile: String,
    pub created_at: i64,
}

/// B45: two friends (`public.friendships`), the pair in the store's order (`a < b`), and since when.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FriendshipRow {
    pub a: String,
    pub b: String,
    pub since: i64,
}

impl FriendshipRow {
    /// The friend of `profile_id` in this pair.
    pub fn other(&self, profile_id: &str) -> &str {
        if self.a == profile_id { &self.b } else { &self.a }
    }
}

/// B46: one block (`public.blocks`): `blocker` blocked `blocked`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BlockRow {
    pub blocker: String,
    pub blocked: String,
    pub created_at: i64,
}

store_union! {
    /// B48, B49: where a challenge stands (`challenges.state`). Only an open one may be answered.
    pub enum ChallengeState {
        Open = "open",
        Accepted = "accepted",
        Declined = "declined",
        Cancelled = "cancelled",
        Expired = "expired",
    }
}

/// B48: a challenge to a friend and the room it opened (`public.challenges`). The room expires
/// with the challenge (`expires_at`), and `mode` is the room's.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeRow {
    pub id: String,
    pub from_profile: String,
    pub to_profile: String,
    pub room_code: String,
    pub mode: QueueMode,
    pub created_at: i64,
    pub expires_at: i64,
    pub state: ChallengeState,
}

/// B53: one chat message (`public.chat_messages`), its body as stored (trimmed and filtered), and
/// when its recipient read it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageRow {
    pub id: String,
    pub from_profile: String,
    pub to_profile: String,
    pub body: String,
    pub created_at: i64,
    pub read_at: Option<i64>,
}

/// B53: one report of a chat message (`public.chat_reports`), one per message and reporter.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChatReportRow {
    pub id: String,
    pub message_id: String,
    pub reporter: String,
    pub reason: String,
    pub created_at: i64,
}

impl Tx<'_> {
    // friends (B43–B45)

    /// Every pending request `profile_id` sent or received, oldest first, the id breaking a tie.
    pub async fn friends_requests_of(&mut self, profile_id: &str) -> StoreResult<Vec<FriendRequestRow>> {
        dispatch!(self, friends_requests_of(profile_id))
    }

    /// One request, or `None` for an id no request holds.
    pub async fn friends_request_get(&mut self, request_id: &str) -> StoreResult<Option<FriendRequestRow>> {
        dispatch!(self, friends_request_get(request_id))
    }

    /// The request `from` sent `to`, if any (this direction only).
    pub async fn friends_request_between(
        &mut self,
        from: &str,
        to: &str,
    ) -> StoreResult<Option<FriendRequestRow>> {
        dispatch!(self, friends_request_between(from, to))
    }

    /// B43: writes the request. False, and nothing written, when its sender already has one to its
    /// recipient (the unique key).
    pub async fn friends_request_create(&mut self, row: &FriendRequestRow) -> StoreResult<bool> {
        dispatch!(self, friends_request_create(row))
    }

    /// B45: deletes one request; false when there was none.
    pub async fn friends_request_delete(&mut self, request_id: &str) -> StoreResult<bool> {
        dispatch!(self, friends_request_delete(request_id))
    }

    /// B46: deletes every request between the two, either way; answers how many went.
    pub async fn friends_requests_delete_between(&mut self, x: &str, y: &str) -> StoreResult<i64> {
        dispatch!(self, friends_requests_delete_between(x, y))
    }

    /// Every friendship `profile_id` is in, oldest first.
    pub async fn friends_of(&mut self, profile_id: &str) -> StoreResult<Vec<FriendshipRow>> {
        dispatch!(self, friends_of(profile_id))
    }

    /// B43: how many friends `profile_id` holds, for `MAX_FRIENDS`.
    pub async fn friends_count(&mut self, profile_id: &str) -> StoreResult<i64> {
        dispatch!(self, friends_count(profile_id))
    }

    /// The two's friendship, whichever order they are named in, or `None`.
    pub async fn friends_get(&mut self, x: &str, y: &str) -> StoreResult<Option<FriendshipRow>> {
        dispatch!(self, friends_get(x, y))
    }

    /// B44, B45: makes the two friends from `at` (a pair already friends keeps its row and its
    /// date) and deletes every request between them, either way. Answers the friendship.
    pub async fn friends_make(&mut self, x: &str, y: &str, at: i64) -> StoreResult<FriendshipRow> {
        dispatch!(self, friends_make(x, y, at))
    }

    /// B45, B46: ends the two's friendship; false when they were not friends.
    pub async fn friends_remove(&mut self, x: &str, y: &str) -> StoreResult<bool> {
        dispatch!(self, friends_remove(x, y))
    }

    // blocks (B46)

    /// The blocks `blocker` made, oldest first.
    pub async fn blocks_of(&mut self, blocker: &str) -> StoreResult<Vec<BlockRow>> {
        dispatch!(self, blocks_of(blocker))
    }

    /// B46: whether either of the two blocked the other.
    pub async fn blocks_between(&mut self, x: &str, y: &str) -> StoreResult<bool> {
        dispatch!(self, blocks_between(x, y))
    }

    /// B46: records the block; a block already made keeps its date.
    pub async fn blocks_add(&mut self, blocker: &str, blocked: &str, at: i64) -> StoreResult<()> {
        dispatch!(self, blocks_add(blocker, blocked, at))
    }

    /// Lifts `blocker`'s block of `blocked`; false when there was none.
    pub async fn blocks_remove(&mut self, blocker: &str, blocked: &str) -> StoreResult<bool> {
        dispatch!(self, blocks_remove(blocker, blocked))
    }

    // presence (B47)

    /// A heartbeat: `profile_id` was seen at `at` (a later beat never moves it back).
    pub async fn presence_touch(&mut self, profile_id: &str, at: i64) -> StoreResult<()> {
        dispatch!(self, presence_touch(profile_id, at))
    }

    /// Each named profile's last heartbeat, by profile id; one that never beat is left out.
    pub async fn presence_last_seen(&mut self, profile_ids: &[String]) -> StoreResult<IndexMap<String, i64>> {
        dispatch!(self, presence_last_seen(profile_ids))
    }

    // profiles (B46, B50)

    /// The profile a client named by id, or `None`: `profiles_get_by_id` for an id that came off
    /// the wire, which may be no id at all (Postgres answers `None` for a text that is not a UUID
    /// rather than failing the cast).
    pub async fn profiles_find(&mut self, profile_id: &str) -> StoreResult<Option<Profile>> {
        dispatch!(self, profiles_find(profile_id))
    }

    /// B50: whether friends may watch `profile_id`'s live unranked games: true unless they turned
    /// it off, and true for an id no profile holds (a read that finds no profile decides nothing).
    pub async fn profiles_allows_spectators(&mut self, profile_id: &str) -> StoreResult<bool> {
        dispatch!(self, profiles_allows_spectators(profile_id))
    }

    /// B50: `PUT /api/settings`' `allowSpectators`.
    pub async fn profiles_set_allow_spectators(&mut self, profile_id: &str, allow: bool) -> StoreResult<()> {
        dispatch!(self, profiles_set_allow_spectators(profile_id, allow))
    }

    // challenges (B48, B49)

    /// B48: writes the open challenge. A challenge of the same pair (this direction) still open
    /// but lapsed by `now` is marked expired first; one still open and live makes this false, with
    /// nothing written (`challenges_open_pair_key`).
    pub async fn challenges_create(&mut self, row: &ChallengeRow, now: i64) -> StoreResult<bool> {
        dispatch!(self, challenges_create(row, now))
    }

    /// One challenge, whatever its state, or `None`.
    pub async fn challenges_get(&mut self, challenge_id: &str) -> StoreResult<Option<ChallengeRow>> {
        dispatch!(self, challenges_get(challenge_id))
    }

    /// B48: the open challenges `profile_id` sent or received that have not lapsed by `now`,
    /// oldest first.
    pub async fn challenges_open_of(&mut self, profile_id: &str, now: i64) -> StoreResult<Vec<ChallengeRow>> {
        dispatch!(self, challenges_open_of(profile_id, now))
    }

    /// B49: the open challenge, not lapsed by `now`, that opened the room `code`, if any.
    pub async fn challenges_open_for_room(&mut self, code: &str, now: i64) -> StoreResult<Option<ChallengeRow>> {
        dispatch!(self, challenges_open_for_room(code, now))
    }

    /// B49: moves an open challenge to `state` (declined or cancelled) and answers it as it now
    /// stands; `None`, and nothing written, when it is not open.
    pub async fn challenges_close(
        &mut self,
        challenge_id: &str,
        state: ChallengeState,
    ) -> StoreResult<Option<ChallengeRow>> {
        dispatch!(self, challenges_close(challenge_id, state))
    }

    /// B46: moves every open challenge between the two, either way, to `state`; answers them.
    pub async fn challenges_close_between(
        &mut self,
        x: &str,
        y: &str,
        state: ChallengeState,
    ) -> StoreResult<Vec<ChallengeRow>> {
        dispatch!(self, challenges_close_between(x, y, state))
    }

    /// B49: the room `code` was joined by `guest`: the open challenge to `guest` that opened it,
    /// not lapsed by `now`, is accepted. False when no such challenge exists.
    pub async fn challenges_accept_room(&mut self, code: &str, guest: &str, now: i64) -> StoreResult<bool> {
        dispatch!(self, challenges_accept_room(code, guest, now))
    }

    /// B49: marks every open challenge lapsed by `now` expired; answers them.
    pub async fn challenges_expire(&mut self, now: i64) -> StoreResult<Vec<ChallengeRow>> {
        dispatch!(self, challenges_expire(now))
    }

    // rooms

    /// B49: frees an unjoined room: its code is open to the next room at once, and joining it is
    /// the miss an unknown code is. A room someone has joined is never touched. False when there
    /// was no unjoined room under the code.
    pub async fn rooms_release(&mut self, code: &str) -> StoreResult<bool> {
        dispatch!(self, rooms_release(code))
    }

    // chat (B53)

    pub async fn chat_insert(&mut self, row: &ChatMessageRow) -> StoreResult<()> {
        dispatch!(self, chat_insert(row))
    }

    /// One message, or `None`.
    pub async fn chat_get(&mut self, message_id: &str) -> StoreResult<Option<ChatMessageRow>> {
        dispatch!(self, chat_get(message_id))
    }

    /// B53: the newest `limit` messages between the two (either way) sent before `before` (epoch
    /// ms; every one when `None`), newest LAST, the id breaking a tie.
    pub async fn chat_page(
        &mut self,
        x: &str,
        y: &str,
        before: Option<i64>,
        limit: i64,
    ) -> StoreResult<Vec<ChatMessageRow>> {
        dispatch!(self, chat_page(x, y, before, limit))
    }

    /// B53: `reader` read these messages at `at`: each one sent to `reader` and not yet read.
    pub async fn chat_mark_read(&mut self, reader: &str, message_ids: &[String], at: i64) -> StoreResult<()> {
        dispatch!(self, chat_mark_read(reader, message_ids, at))
    }

    /// B53: how many unread messages each sender has sent `profile_id`, by sender id.
    pub async fn chat_unread(&mut self, profile_id: &str) -> StoreResult<IndexMap<String, i64>> {
        dispatch!(self, chat_unread(profile_id))
    }

    /// B53: stores the report; false, and nothing written, when its reporter already reported the
    /// message.
    pub async fn chat_report(&mut self, row: &ChatReportRow) -> StoreResult<bool> {
        dispatch!(self, chat_report(row))
    }

    /// B53: deletes every message sent before `before` (epoch ms), and their reports; answers how
    /// many messages went.
    pub async fn chat_purge(&mut self, before: i64) -> StoreResult<i64> {
        dispatch!(self, chat_purge(before))
    }
}
// ---- end Social ----
