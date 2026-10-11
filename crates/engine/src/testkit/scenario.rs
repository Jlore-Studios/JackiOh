//! The card test harness (BUILD M4-T3; SURFACE §8). Port of `packages/cards/test/_harness.ts`, with
//! `packages/cards/test/_harness.test.ts` as this file's `mod tests` (part 5).
//!
//! In TS, importing the harness registered the real catalog and every script (`registerAll()`). The
//! engine crate cannot name `jackioh_cards` (it depends on the engine), so a card test calls
//! `jackioh_cards::register_all()` first, or a test with fixture cards installs them with this
//! file's `register_catalog`/`register_scripts`: the thread-local override that
//! `catalog::registered_catalog()` and `scripts::script_of()` consult first under the `testkit`
//! feature (SURFACE §8). Each `#[test]` runs on its own thread, so an override is that test's alone.
//!
//! A scenario is built out of the engine's OWN functions — `create_game`, `new_instance`,
//! `place_on_field`, `move_to_zone`, `create_in_hand`, `refresh_mana`, `state_check` — never out of a
//! hand-written state literal, so a state built here always has every field the engine adds later.
//!
//! ---------------------------------------------------------------------------------------------
//! DEFAULTS  (pass `seed`, `turn`, `active` to change them)
//! ---------------------------------------------------------------------------------------------
//!   seed    "jackioh-harness"
//!   active  "p1"
//!   turn    9  — a mid-game board: the active side has started ceil(9/2) = 5 turns and the other
//!               floor(9/2) = 4, so both sides sit at MAX_MANA (4/4) with no asymmetry to reason
//!               about. Turn counts player-turns (§2.5): turn 1 is p1's first, 2 is p2's first.
//!               `turnsStarted` always follows `active`/`turn` by that rule, NOT by the parity of
//!               `turn`, so `{ "active": "p2", "turn": 9 }` gives p2 five started turns and p1 four.
//!   mana    each side's `mana.max` is whatever `refresh_mana` computes from `turnsStarted`, and
//!           `mana.current` equals it. `SideSetup.mana` sets `current` only — §2.3 lets current
//!           exceed max, so the harness never raises max.
//!   health  HERO_HEALTH (30); armor 0.
//!
//! The scenario starts in `phase: "main"` with the mulligan, `begin_game` and the opening `start_turn`
//! all skipped: setup draws nothing and fires no trigger. Units placed by `field` are NOT summoning
//! sick (`summonedTurn` is left unset) and have both exertions unspent.
//!
//! ---------------------------------------------------------------------------------------------
//! SIDE SETUP  (what `p1`/`p2` accept)
//! ---------------------------------------------------------------------------------------------
//!   hand, library, graveyard, exile   (string | { def?, defId?, radiant?, costMod?, costOverride? })[]
//!   field, backrow                    (string | { def?, defId?, radiant?, row?, lane?, stack?,
//!                                                 position?, damage?, counters?, faceUp?,
//!                                                 statsOverride?, costMod?, costOverride? })[]
//!   health, mana, armor               number
//! `def` and `defId` are aliases: exactly one is required, and both with different values throws
//! naming both. A bare string is `{ def: <string> }`. `radiant: true` sets the instance's Radiant
//! flag in whatever zone the entry names, a library or a hand included (#21, #23 need a Radiant
//! card sitting on top of a library). `statsOverride` is §10.4 layer 1 / R41's X/X token.
//! `counters` seeds §10.1's instance counters — #91's `plague`, #93's `grade`. `costMod` and
//! `costOverride` seed §2.3's cost layers, which R78 keeps in every zone, so a ruling about the cost
//! at resolution rather than the printed cost (R65, R66) can be set up in a hand or a library.
//! `faceUp` is written exactly as given, so `faceUp: false` reads back `Some(false)` rather than
//! `None` (R33: a Trap is face-down until it fires).
//!
//! `stack: true` builds a §3.2 Stack pile, in a unit zone or a backrow zone (B5 E21): the entry
//! buries the card already in its lane instead of taking a lane of its own, so the `lane` may be
//! repeated, and with no `lane` it lands on the entry before it in its row. Later entries go on top,
//! the way a play would put them, so the pile reads top-first and the list reads bottom-first:
//!   "field": ["core-043", { "def": "core-092", "stack": true }]
//! is a Felinor Fiender on top of a Big Felinor in lane 1, the Big Felinor dormant (R13).
//!
//! `field` and `backrow` describe ONE board and take the same entries: an entry's row comes from
//! its def's type — a Unit to the unit zones, a Field Spell / Trap / Field Trap to the backrow —
//! and `row: "units" | "backrow"` overrides that, so `field: [{ defId: MANA_WELL, row: "backrow" }]`
//! and `backrow: [{ defId: MANA_WELL }]` are the same setup — and a card filed under the wrong list
//! is routed by its type rather than refused. Only an explicit `row` that contradicts the type is an
//! error, and a Spell is refused in either list: §3.2 never puts one on the field.
//! Lanes are handed out per row over the whole board, `field` entries
//! first: an entry naming a `lane` keeps it, and the rest take the leftmost zone still free, in
//! list order. Instances are created in list order either way, so the ids follow the literal.
//!
//! ---------------------------------------------------------------------------------------------
//! STRING REFERENCES  (`play("43", …)`, `attack("Big Felinor", "hero")`, `expect_in_zone("c7", "gone")`)
//! ---------------------------------------------------------------------------------------------
//! A `string` is resolved in this order, first match wins:
//!   1. an instance id that exists right now (`"c7"`);
//!   2. a catalog id (`"core-043"`);
//!   3. a SPEC §5 index (`"43"`, `"51.1"`, `"T-rush"`);
//!   4. a card name, exact (`"Big Felinor"`), then case-insensitive.
//!      Steps 2-4 give a defId, and the instance is then the first one found scanning
//!      the ACTIVE player first, then the opponent, and within a side:
//!      hand → unit zones (lane 1..5, top of a Stack pile before the cards dormant under it) →
//!      backrow (lane 1..5, then the cards dormant under its tops) → graveyard → exile → library →
//!      resolving.
//!      A method that needs the card somewhere particular narrows the search to that place first:
//!      `play` looks in hands only, `attack`/`switch_position` on the field only. Nothing matching panics
//!      naming the string and listing what was there instead. Holding several copies of one def?
//!      Use the `CardInstance` the setup or `s.card(...)` handed back; the string form is first-match.
//!
//! ---------------------------------------------------------------------------------------------
//! INSTANCES ARE SNAPSHOTS
//! ---------------------------------------------------------------------------------------------
//! `reduce` returns a new state, so a `CardInstance` you captured before a step is a stale copy after
//! it. Every harness method re-resolves a `CardInstance` argument by its `id` against the live
//! state, so passing a stale instance is fine — but never read `.damage`/`.buffs` off one you are
//! holding; read it back with `s.card(inst)`, `s.unit(p, lane)` or `s.expect_stats(inst, …)`.
//! `unit`, `backrow`, `hand` and `pile` hand back copies for that reason (and so that a copy can be
//! passed straight into the next step: `s.attack("core-025", s.unit("p2", 1).unwrap())`).
//!
//! ---------------------------------------------------------------------------------------------
//! start_turn() vs end_turn()  — the two easiest things to get wrong
//! ---------------------------------------------------------------------------------------------
//! `start_turn()` runs the engine's `start_turn(sink, state.active)` for the player who is active NOW.
//!   It does not pass the turn: `state.turn` and that player's `turnsStarted` go up by one, mana
//!   refreshes, delayed and start-of-turn effects fire, exertion resets, the player draws one card,
//!   and the phase lands back in `main`. This is what an "at the start of your turn" card test wants.
//! `end_turn()` is the `endTurn` ACTION: end-of-turn triggers and delayed effects fire, the turn log
//!   closes, "this turn" modifiers expire — and then `turn.rs` starts the OPPONENT's turn (their
//!   draw, their triggers). So one `end_turn()` hands the turn over; two come back around to your own
//!   next turn. It is not `start_turn()` twice: the opponent really takes a turn in between.
//!   WARNING (engine `reduce`, §2.5): after any action, a turn with nothing meaningful left on it
//!   auto-ends by itself and emits `turnAutoEnded`. A side with an empty hand and no unit that could
//!   switch position has nothing meaningful, so `end_turn()` can cascade several turns forward. Give
//!   each side a card in hand or a unit on the board when a test crosses a turn boundary.
//!
//! ---------------------------------------------------------------------------------------------
//! ZONES, EVENTS, ASSERTIONS
//! ---------------------------------------------------------------------------------------------
//! `expect_in_zone(card, "field")` means a unit zone or the backrow, a card dormant under a Stack pile
//!   included. `"gone"` means in no pile at all: SPEC §3.2 / R11's unit token that ceased to exist
//!   (`move_to_zone` answers "vanished" and leaves it nowhere). To assert `"gone"` pass the
//!   `CardInstance` or its instance id — a def reference has nothing left to find.
//! `expect_stats` reads the engine's `unit_view` (`layers.rs`), so every §10.4 layer is already in the
//!   number: never `def.base`, never raw `buffs`. `stats(card)` returns that whole `UnitView`
//!   (attack, maxHealth, health, keywords, armor, position) for a test that needs to read a keyword
//!   or an Armor total — a `CardInstance` has none of those, they are computed on every read.
//! `events()` is the cumulative log of every event produced since setup finished, oldest first; the
//!   events of the setup itself (including a death from a unit placed at lethal damage) are not in
//!   it. `last_events()` is just the most recent step. `expect_events(...)` searches `events` for those
//!   types as a SUBSEQUENCE: in that relative order, not necessarily adjacent.
//! Every refusal — an illegal play, an illegal attack, an answer that matches no option — panics
//!   with the engine's own message, which is the mechanism the M4-T4 table's "Play refused
//!   without a tribute" / "locked zone rejects play" / "Uncastable at 4 mana" rows use:
//!   `s.expect_refused_with(|s| s.play(x, json!({})), "tribute")` (TS `toThrow(/tribute/)`).
//!
//! TS's fallbacks for `reduce`'s long-gone placeholder refusals ("combat arrives with M2", "prompts
//! arrive with M3") are not ported: `attack` and `answer` are plain actions, as every other step is.
//! `view()` calls `view_for(state, player)` (§10.8) directly.

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::IndexMap;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::config::{BACKROW_ZONES, DECK_SIZE, UNIT_ZONES};
use crate::layers::{UnitView, unit_view};
use crate::own_library::show_to_owner;
use crate::rng::Rng;
use crate::script::{CardScripts, EngineSink};
use crate::state::{
    CardInstance, CreateGameOptions, GameState, LastBoardInput, PendingChoice, PromptOption, create_game,
    find_instance, find_instance_mut, new_instance,
};
use crate::wire::{
    Action, ActionBody, AttackHealth, CardDef, CardDefs, CardType, Counters, GameEvent, PLAYER_IDS, Phase,
    PlayerId, PlayerView, Position, RevealAt, Row, Selection, Zone, ZoneChoice, opponent_of,
};
use crate::zones::{
    LibraryPosition, MoveResult, MoveToZoneOptions, OffFieldZone, PlaceOnFieldOptions, ZoneSlot,
};

pub const DEFAULT_SEED: &str = "jackioh-harness";
/// A mid-game board: both sides at MAX_MANA. See the file header.
pub const DEFAULT_TURN: i32 = 9;

// ---------------------------------------------------------------------------------------------
// Test-only registries (SURFACE §8)
// ---------------------------------------------------------------------------------------------

// The one exception to SURFACE §3's "no mutable statics": a per-thread override of the two
// registries, for tests only (this module is compiled under the `testkit` feature alone). Each holds
// a leaked `&'static`, so `registered_catalog()` can hand it out as it hands out the `OnceLock`'s; a
// test registers a handful of times, so the leak is bounded by the test run.
thread_local! {
    static CATALOG_OVERRIDE: Cell<Option<&'static CardDefs>> = const { Cell::new(None) };
    static CATALOG_VERSION_OVERRIDE: Cell<Option<&'static str>> = const { Cell::new(None) };
    static SCRIPTS_OVERRIDE: Cell<Option<&'static IndexMap<String, CardScripts>>> = const { Cell::new(None) };
    static WIN_RATES_OVERRIDE: Cell<Option<&'static crate::win_rates::WinRateTable>> =
        const { Cell::new(None) };
}

/// TS `registerCatalog(defs)` for this thread: the catalog `catalog::registered_catalog()` answers
/// with until the thread ends or `clear_overrides()` runs. Its version is TS's default, `"test"`.
pub fn register_catalog(defs: CardDefs) {
    register_catalog_as(defs, "test");
}

/// TS `registerCatalog(defs, catalogVersion)` for this thread.
pub fn register_catalog_as(defs: CardDefs, catalog_version: &str) {
    let defs: &'static CardDefs = Box::leak(Box::new(defs));
    let version: &'static str = Box::leak(catalog_version.to_string().into_boxed_str());
    CATALOG_OVERRIDE.with(|cell| cell.set(Some(defs)));
    CATALOG_VERSION_OVERRIDE.with(|cell| cell.set(Some(version)));
}

/// TS `registerScripts(scripts)` for this thread: the scripts `scripts::script_of()` reads until the
/// thread ends or `clear_overrides()` runs. Like TS, it replaces the whole registry, never merges.
pub fn register_scripts(scripts: IndexMap<String, CardScripts>) {
    let scripts: &'static IndexMap<String, CardScripts> = Box::leak(Box::new(scripts));
    SCRIPTS_OVERRIDE.with(|cell| cell.set(Some(scripts)));
}

/// The catalog this thread registered, if any: what `catalog::registered_catalog()` answers first.
pub fn catalog_override() -> Option<&'static CardDefs> {
    CATALOG_OVERRIDE.with(Cell::get)
}

/// The catalog version this thread registered with its catalog, if any (`catalog::catalog_version()`).
pub fn catalog_version_override() -> Option<&'static str> {
    CATALOG_VERSION_OVERRIDE.with(Cell::get)
}

/// The scripts this thread registered, if any: what `scripts::script_of()` reads first.
pub fn scripts_override() -> Option<&'static IndexMap<String, CardScripts>> {
    SCRIPTS_OVERRIDE.with(Cell::get)
}

/// ME-STATS: the win-rate table this thread registered, if any: what
/// `catalog::registered_win_rates()` answers first. A leaked `&'static`, like the catalog's.
pub fn register_win_rates(table: crate::win_rates::WinRateTable) {
    let table: &'static crate::win_rates::WinRateTable = Box::leak(Box::new(table));
    WIN_RATES_OVERRIDE.with(|cell| cell.set(Some(table)));
}

/// The win-rate table this thread registered, if any.
pub fn win_rates_override() -> Option<&'static crate::win_rates::WinRateTable> {
    WIN_RATES_OVERRIDE.with(Cell::get)
}

/// Back to the production registries (`jackioh_cards::register_all()`'s) for this thread: TS's
/// `registerAll()` after a fixture registration.
pub fn clear_overrides() {
    CATALOG_OVERRIDE.with(|cell| cell.set(None));
    CATALOG_VERSION_OVERRIDE.with(|cell| cell.set(None));
    SCRIPTS_OVERRIDE.with(|cell| cell.set(None));
    WIN_RATES_OVERRIDE.with(|cell| cell.set(None));
}

// ---------------------------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------------------------

/// TS `CardRef = string | CardInstance`. A `CardInstance` is held by its id (and def id, for the
/// message when it is gone), so a copy handed back by a read can go straight into the next step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CardRef {
    Text(String),
    Instance { id: String, def_id: String },
}

impl From<&str> for CardRef {
    fn from(text: &str) -> CardRef {
        CardRef::Text(text.to_string())
    }
}

impl From<String> for CardRef {
    fn from(text: String) -> CardRef {
        CardRef::Text(text)
    }
}

impl From<&String> for CardRef {
    fn from(text: &String) -> CardRef {
        CardRef::Text(text.clone())
    }
}

impl From<CardInstance> for CardRef {
    fn from(card: CardInstance) -> CardRef {
        CardRef::Instance {
            id: card.id,
            def_id: card.def_id,
        }
    }
}

impl From<&CardInstance> for CardRef {
    fn from(card: &CardInstance) -> CardRef {
        CardRef::Instance {
            id: card.id.clone(),
            def_id: card.def_id.clone(),
        }
    }
}

/// A seat as a harness read or assertion names it: `PlayerId::P1`, `"p1"`, or `()`/`None` for the
/// active player (TS's omitted `player?` argument).
pub trait SeatRef {
    fn seat_or(self, active: PlayerId) -> PlayerId;
}

impl SeatRef for PlayerId {
    fn seat_or(self, _active: PlayerId) -> PlayerId {
        self
    }
}

impl SeatRef for &str {
    fn seat_or(self, _active: PlayerId) -> PlayerId {
        match self {
            "p1" => PlayerId::P1,
            "p2" => PlayerId::P2,
            other => panic!("not a seat: {other:?} (a seat is \"p1\" or \"p2\")"),
        }
    }
}

impl SeatRef for Option<PlayerId> {
    fn seat_or(self, active: PlayerId) -> PlayerId {
        self.unwrap_or(active)
    }
}

impl SeatRef for () {
    fn seat_or(self, active: PlayerId) -> PlayerId {
        active
    }
}

/// Every setup entry names its card with `def` OR `defId` — the two are aliases, exactly one is
/// required, and giving both with different values throws. A bare string is the same as `{ def }`.
#[derive(Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DefRef {
    #[serde(default)]
    pub def: Option<String>,
    #[serde(default)]
    pub def_id: Option<String>,
}

/// R78: "`costMod`, `costOverride` and `radiant` persist in every zone", so a setup may seed the two
/// cost layers wherever the card sits — which is what a ruling about the cost *at resolution* rather
/// than the printed cost (R65, R66) needs in order to be testable at all.
#[derive(Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CostSetup {
    /// R65: added to the printed cost, in every zone (§2.3's cost layers).
    #[serde(default)]
    pub cost_mod: Option<i32>,
    /// R65: the cost this card has instead of its printed one, modifiers still applying over it.
    #[serde(default)]
    pub cost_override: Option<i32>,
}

/// TS `DefRef & CostSetup & { radiant? }`: a hand, library, graveyard or exile entry's object form.
#[derive(Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PileEntry {
    #[serde(flatten)]
    pub card: DefRef,
    #[serde(flatten)]
    pub cost: CostSetup,
    #[serde(default)]
    pub radiant: Option<bool>,
    /// MD-B6, R943: opt a setup card into Created (`new_instance` mints Created; the setup
    /// defaults to dealt, so only `created: true` keeps the mark).
    #[serde(default)]
    pub created: Option<bool>,
}

/// TS `PileSetup = string | (DefRef & CostSetup & { radiant? })`.
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum PileSetup {
    Name(String),
    Entry(PileEntry),
}

/// One card on the field. `field` and `backrow` take the same shape: the row comes from the def's
/// type (Units to the unit zones, Field Spells and Traps to the backrow) unless `row` names it.
#[derive(Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FieldEntry {
    #[serde(flatten)]
    pub card: DefRef,
    #[serde(flatten)]
    pub cost: CostSetup,
    #[serde(default)]
    pub radiant: Option<bool>,
    /// Overrides the row the def's type implies.
    #[serde(default)]
    pub row: Option<Row>,
    /// 1-based lane; omitted takes the leftmost zone still free in that row.
    #[serde(default)]
    pub lane: Option<i32>,
    /// §3.2 Stack: this entry buries the card already in its lane instead of taking a lane of its
    /// own, so the lane may be repeated. Later entries go on top, as a play would put them: the pile
    /// reads top-first, which is the reverse of the list. A unit zone or a backrow zone (B5 E21), and
    /// the buried card must come earlier in the list (or be pinned there with the same `lane`).
    #[serde(default)]
    pub stack: Option<bool>,
    #[serde(default)]
    pub position: Option<Position>,
    #[serde(default)]
    pub damage: Option<i32>,
    /// §10.1 counters: #91's plague and #93's grade.
    #[serde(default)]
    pub counters: Option<Counters>,
    /// A backrow card whose identity is public (§10.8, R33); `false` is written as `false`.
    #[serde(default)]
    pub face_up: Option<bool>,
    /// §10.4 layer 1 / R41: a token summoned X/X.
    #[serde(default)]
    pub stats_override: Option<AttackHealth>,
    /// MD-B6, R943: opt a setup card into Created (as on `PileEntry`).
    #[serde(default)]
    pub created: Option<bool>,
}

/// TS `FieldSetup = string | FieldEntry`.
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum FieldSetup {
    Name(String),
    Entry(FieldEntry),
}

/// TS `BackrowSetup = string | FieldEntry`.
pub type BackrowSetup = FieldSetup;

#[derive(Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SideSetup {
    #[serde(default)]
    pub hand: Option<Vec<PileSetup>>,
    #[serde(default)]
    pub field: Option<Vec<FieldSetup>>,
    #[serde(default)]
    pub backrow: Option<Vec<BackrowSetup>>,
    #[serde(default)]
    pub library: Option<Vec<PileSetup>>,
    #[serde(default)]
    pub graveyard: Option<Vec<PileSetup>>,
    #[serde(default)]
    pub exile: Option<Vec<PileSetup>>,
    #[serde(default)]
    pub health: Option<i32>,
    #[serde(default)]
    pub mana: Option<i32>,
    #[serde(default)]
    pub armor: Option<i32>,
}

#[derive(Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayOptions {
    /// 1-based lane; the row comes from the def's type. Omitted means R64's leftmost free zone.
    #[serde(default)]
    pub zone: Option<i32>,
    /// The row `zone` names when it is not the def's own: a Unit topping a carrier (C+ #33, R446).
    #[serde(default)]
    pub row: Option<Row>,
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub embiggen: Option<bool>,
    /// R1086: play this Magnetic card onto the zone's host Unit.
    #[serde(default)]
    pub magnetic: Option<bool>,
    #[serde(default)]
    pub targets: Option<Vec<Selection>>,
    #[serde(default)]
    pub modes: Option<Vec<String>>,
    /// Units sacrificed for a Tribute cost; each entry is any card reference on the field.
    #[serde(default)]
    pub tributes: Option<Vec<String>>,
    /// ME-ALTPLAY, R1040, R1044: play face-down as a Trap revealing at this timing.
    #[serde(default)]
    pub face_down: Option<RevealAt>,
}

/// B3.2, R384: an Activate ability's choices travel in the action as a play's do (R81): `ability`
/// names one when a card has several, `modes` and `targets` are its declared choices, and `tributes`
/// pays a Tribute its cost names (card references on the field, as `play`'s). With none of `ability`,
/// `modes` or `tributes` the harness sends `activatePower`, the alias every old log carries, which
/// the engine routes exactly as `activate` (R43, R384).
#[derive(Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActivateOptions {
    #[serde(default)]
    pub targets: Option<Vec<Selection>>,
    #[serde(default)]
    pub ability: Option<String>,
    #[serde(default)]
    pub modes: Option<Vec<String>>,
    #[serde(default)]
    pub tributes: Option<Vec<String>>,
}

#[derive(Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioOptions {
    #[serde(default)]
    pub seed: Option<String>,
    #[serde(default)]
    pub p1: Option<SideSetup>,
    #[serde(default)]
    pub p2: Option<SideSetup>,
    /// A number, so a fraction or a negative is refused by name as TS refused it.
    #[serde(default)]
    pub turn: Option<f64>,
    #[serde(default)]
    pub active: Option<PlayerId>,
    /// R417: each seat's last board, the `create_game` input C+ #29 reads (seat order).
    #[serde(default)]
    pub last_boards: Option<LastBoardInput>,
    /// R678: the two other games' boards a Glitch may lay down, the `create_game` input (seat order).
    #[serde(default)]
    pub glitch_boards: Option<LastBoardInput>,
}

// ---------------------------------------------------------------------------------------------
// Card references
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Where {
    Any,
    Hand,
    Field,
}

/// TS `INSTANCE_ID = /^c\d+$/`.
fn is_instance_id(text: &str) -> bool {
    text.len() > 1 && text.starts_with('c') && text[1..].bytes().all(|byte| byte.is_ascii_digit())
}

fn registered() -> &'static CardDefs {
    crate::catalog::registered_catalog()
}

fn describe_instance(state: &GameState, card: &CardInstance) -> String {
    let name = registered()
        .get(&card.def_id)
        .map(|def| def.name.as_str())
        .or_else(|| {
            state
                .transient_defs
                .get(&card.def_id)
                .map(|def| def.name.as_str())
        })
        .unwrap_or("?");
    format!(
        "{} {} \"{}\"{}",
        card.id,
        card.def_id,
        name,
        if card.radiant { " (radiant)" } else { "" }
    )
}

/// Every def the engine can see: the registered catalog plus this state's transient defs (Fuse).
fn visible_defs(state: &GameState) -> Vec<&CardDef> {
    registered()
        .values()
        .chain(state.transient_defs.values())
        .collect()
}

/// Steps 2-4 of the header's resolution order: catalog id, then §5 index, then name (exact, then
/// case-insensitive). Returns every defId that matched at the first matching step.
fn def_ids_for(state: &GameState, text: &str) -> Vec<String> {
    let defs = visible_defs(state);
    let lower = text.to_lowercase();
    let steps: [&dyn Fn(&CardDef) -> bool; 4] = [
        &|def: &CardDef| def.id == text,
        &|def: &CardDef| def.index == text,
        &|def: &CardDef| def.name == text,
        &|def: &CardDef| def.name.to_lowercase() == lower,
    ];
    for step in steps {
        let hit: Vec<String> = defs
            .iter()
            .filter(|&&def| step(def))
            .map(|def| def.id.clone())
            .collect();
        if !hit.is_empty() {
            return hit;
        }
    }
    Vec::new()
}

/// A side's instances in the header's documented scan order.
fn side_order(state: &GameState, player: PlayerId, scope: Where) -> Vec<&CardInstance> {
    let side = &state.players[player];
    let units: Vec<&CardInstance> = side.units.iter().flatten().flatten().collect();
    // B5 E21, R446: a carrier's Unit and the cards dormant in a backrow pile are on the field too.
    let mut backrow: Vec<&CardInstance> = side.backrow.iter().flatten().collect();
    backrow.extend(side.carried.iter().flatten().flatten());
    backrow.extend(side.backrow_piles.iter().flatten().flatten());
    match scope {
        Where::Hand => side.hand.iter().collect(),
        Where::Field => units.into_iter().chain(backrow).collect(),
        Where::Any => side
            .hand
            .iter()
            .chain(units)
            .chain(backrow)
            .chain(side.graveyard.iter())
            .chain(side.exile.iter())
            .chain(side.library.iter())
            .chain(side.resolving.iter())
            .collect(),
    }
}

fn search_order(state: &GameState, scope: Where) -> Vec<&CardInstance> {
    let active = state.active;
    let mut out = side_order(state, active, scope);
    out.extend(side_order(state, opponent_of(active), scope));
    out
}

fn where_label(scope: Where) -> &'static str {
    match scope {
        Where::Hand => "a hand",
        Where::Field => "the field",
        Where::Any => "any zone",
    }
}

// ---------------------------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------------------------

/// `create_game` validates decks (§2.6: DECK_SIZE cards, no duplicates, no tokens), so the filler
/// deck is the first DECK_SIZE non-token catalog ids in §5 index order — TS took them straight from
/// `query({})`, whose filter (no token, R674's Glitch in no pool) and order this repeats. Both
/// libraries are emptied again right afterwards and `nextId` is reset, so the ids a scenario hands
/// out start at `c1` and follow the setup's own order.
fn filler_deck() -> Result<Vec<String>, String> {
    let defs: Vec<&CardDef> = crate::catalog::query(&crate::catalog::CatalogQueryArgs::default());
    let size = DECK_SIZE.max(0) as usize;
    if defs.len() < size {
        return Err(format!(
            "the registered catalog holds only {} non-token cards; create_game needs {DECK_SIZE} for its \
             filler deck (did jackioh_cards::register_all() run?)",
            defs.len()
        ));
    }
    Ok(defs.into_iter().take(size).map(|def| def.id.clone()).collect())
}

/// TS `defOf(state, defId)` (`catalog::find_def`: transient defs first), as the harness's error.
fn def_of(state: &GameState, def_id: &str) -> Result<CardDef, String> {
    crate::catalog::find_def(Some(state), def_id)
        .cloned()
        .ok_or_else(|| format!("unknown defId \"{def_id}\": register the catalog first"))
}

fn setup_def_id(state: &GameState, text: &str, label: &str) -> Result<String, String> {
    match def_ids_for(state, text).into_iter().next() {
        Some(first) => Ok(first),
        None => Err(format!(
            "{label}: no catalog card matches \"{text}\" — use a catalog id (\"core-043\"), a SPEC §5 index \
             (\"43\", \"T-rush\") or a name (\"Big Felinor\")"
        )),
    }
}

/// A setup entry as `ref_of` reads it: a bare string or its `def`/`defId` pair.
enum EntryRef<'a> {
    Name(&'a str),
    Fields(&'a DefRef),
}

/// The one place `def`/`defId`/a bare string become a single name. Exactly one of the two keys is
/// required; both with different values is a typo worth naming.
fn ref_of(entry: EntryRef<'_>, label: &str) -> Result<String, String> {
    let fields = match entry {
        EntryRef::Name(name) => return Ok(name.to_string()),
        EntryRef::Fields(fields) => fields,
    };
    if let (Some(def), Some(def_id)) = (&fields.def, &fields.def_id)
        && def != def_id
    {
        return Err(format!(
            "{label}: `def` is \"{def}\" but `defId` is \"{def_id}\"; they are aliases, so give one"
        ));
    }
    match fields.def.as_ref().or(fields.def_id.as_ref()) {
        Some(reference) => Ok(reference.clone()),
        None => Err(format!(
            "{label}: no card named — an entry needs `def` or `defId` (or be a plain string)"
        )),
    }
}

fn pile_ref(entry: &PileSetup) -> EntryRef<'_> {
    match entry {
        PileSetup::Name(name) => EntryRef::Name(name),
        PileSetup::Entry(fields) => EntryRef::Fields(&fields.card),
    }
}

fn field_ref(entry: &FieldSetup) -> EntryRef<'_> {
    match entry {
        FieldSetup::Name(name) => EntryRef::Name(name),
        FieldSetup::Entry(fields) => EntryRef::Fields(&fields.card),
    }
}

/// One resolved card to place, with its row decided and (later) its lane assigned.
#[derive(Clone, Debug)]
struct Placement {
    entry: FieldEntry,
    def_id: String,
    row: Row,
    label: String,
    lane: Option<i32>,
}

fn row_size_of(row: Row) -> i32 {
    if row == Row::Units {
        UNIT_ZONES
    } else {
        BACKROW_ZONES
    }
}

/// §3.2: a Unit goes in the unit zones and a Field Spell, Trap or Field Trap in the backrow, so the
/// def's type picks the row and `row` on the entry overrides it. `field` and `backrow` therefore
/// accept the same entries: naming the list is a convenience, not a second rule.
fn normalize_placement(
    state: &GameState,
    entry: &FieldSetup,
    fallback: Row,
    label: &str,
) -> Result<Placement, String> {
    let fields: FieldEntry = match entry {
        FieldSetup::Name(name) => FieldEntry {
            card: DefRef {
                def: Some(name.clone()),
                def_id: None,
            },
            ..FieldEntry::default()
        },
        FieldSetup::Entry(fields) => fields.clone(),
    };
    let def_id = setup_def_id(state, &ref_of(field_ref(entry), label)?, label)?;
    let def = def_of(state, &def_id)?;
    let implied = if def.type_ == CardType::Unit {
        Row::Units
    } else {
        Row::Backrow
    };
    let row = fields.row.unwrap_or(if def.type_ == CardType::Spell {
        fallback
    } else {
        implied
    });

    if def.type_ == CardType::Spell {
        return Err(format!(
            "{label}: \"{}\" is a Spell; a Spell is never on the field (§3.2)",
            def.name
        ));
    }
    if row == Row::Units && def.type_ != CardType::Unit {
        return Err(format!(
            "{label}: \"{}\" is a {}; the unit zones hold Units only",
            def.name,
            def.type_.as_str()
        ));
    }
    if row == Row::Backrow && def.type_ == CardType::Unit {
        return Err(format!(
            "{label}: \"{}\" is a Unit; the backrow holds Field Spells and Traps",
            def.name
        ));
    }

    Ok(Placement {
        lane: fields.lane,
        entry: fields,
        def_id,
        row,
        label: label.to_string(),
    })
}

/// §3.2 lane assignment for a setup row: entries that name a `lane` keep it, and the rest take the
/// leftmost lane still free, in list order. Instances are still created in list order, so the ids
/// follow the setup literal even when a later entry pinned an earlier lane.
///
/// §3.2 Stack: an entry with `stack: true` does not take a lane of its own — it buries the card
/// already in one, so it may repeat a `lane` an earlier entry pinned, and with no `lane` of its own
/// it lands on the entry before it in the list.
///
/// Each entry is its `(lane, stack)`.
fn assign_lanes(entries: &[(Option<i32>, bool)], size: i32, label: &str) -> Result<Vec<i32>, String> {
    let mut taken: Vec<i32> = Vec::new();
    for (at, &(lane, stack)) in entries.iter().enumerate() {
        let Some(lane) = lane else {
            continue;
        };
        if lane < 1 || lane > size {
            return Err(format!(
                "{label}: lane {lane} is out of range; lanes are 1..{size}"
            ));
        }
        if taken.contains(&lane) && !stack {
            return Err(format!(
                "{label}: two cards were given lane {lane} — add `stack: true` to entry [{at}] for a §3.2 Stack pile"
            ));
        }
        if !taken.contains(&lane) {
            taken.push(lane);
        }
    }

    let mut lanes: Vec<i32> = Vec::new();
    let mut next = 1;
    for (at, &(lane, stack)) in entries.iter().enumerate() {
        if stack {
            let under = lane.or_else(|| at.checked_sub(1).and_then(|before| lanes.get(before).copied()));
            match under {
                Some(under) if lanes[..at].contains(&under) => {
                    lanes.push(under);
                    continue;
                }
                _ => {
                    let shown = under.map_or_else(|| "?".to_string(), |under| under.to_string());
                    return Err(format!(
                        "{label}: entry [{at}] has `stack: true` but lane {shown} holds nothing yet — the card \
                         it buries goes EARLIER in the list, the pile reading top-first"
                    ));
                }
            }
        }
        if let Some(lane) = lane {
            lanes.push(lane);
            continue;
        }
        while taken.contains(&next) {
            next += 1;
        }
        if next > size {
            return Err(format!(
                "{label}: the row holds {size} zones and the setup asks for more"
            ));
        }
        taken.push(next);
        lanes.push(next);
    }
    Ok(lanes)
}

/// R78: `costMod` and `costOverride` persist in every zone, so every setup entry may seed them.
fn apply_cost_setup(card: &mut CardInstance, cost: Option<&CostSetup>) {
    let Some(cost) = cost else {
        return;
    };
    if let Some(cost_mod) = cost.cost_mod {
        card.cost_mod = cost_mod;
    }
    if let Some(cost_override) = cost.cost_override {
        card.cost_override = Some(cost_override);
    }
}

/// Place one resolved entry, in the list order the ids follow.
fn place_one(sink: &mut EngineSink<'_>, player: PlayerId, entry: &Placement) -> Result<(), String> {
    let row = entry.row;
    // `assign_lanes` gave every placement its lane.
    let lane = entry.lane.unwrap_or_default();
    let def = def_of(sink.state, &entry.def_id)?;

    let mut card = new_instance(
        &mut *sink.state,
        &entry.def_id,
        player,
        Zone::Field { player, row, lane },
    );
    if entry.entry.radiant == Some(true) {
        card.radiant = true;
    }
    // MD-B6, R943: the setup defaults to dealt; only `created: true` keeps the mark.
    card.created = entry.entry.created.filter(|created| *created);
    if let Some(stats) = entry.entry.stats_override {
        card.stats_override = Some(stats);
    }
    if let Some(counters) = entry.entry.counters {
        card.counters = counters;
    }
    apply_cost_setup(&mut card, Some(&entry.entry.cost));
    let id = card.id.clone();

    // §3.2 Stack: `stack: true` buries whatever is in the lane already — `place_on_field` puts the
    // arriving card on top, so the pile reads top-first and the list reads bottom-first.
    let slot = ZoneSlot { player, row, lane };
    let options = PlaceOnFieldOptions {
        stack: if entry.entry.stack == Some(true) {
            Some(true)
        } else {
            None
        },
    };
    if !crate::zones::place_on_field(&mut *sink.state, &mut card, slot, options) {
        return Err(format!(
            "{}: {} {} lane {lane} would not take \"{}\"",
            entry.label,
            player.as_str(),
            row.as_str(),
            def.name
        ));
    }

    // TS went on writing the object it had placed; here it is found again by its id.
    if let Some(card) = find_instance_mut(&mut *sink.state, &id) {
        if row == Row::Units {
            card.position = Some(entry.entry.position.unwrap_or(Position::Atk));
            if let Some(damage) = entry.entry.damage {
                card.damage = damage;
            }
        }
        // R33: a Trap is face-down, so `faceUp: false` is a state a test asserts — write it as given
        // rather than leaving the flag unset, which would read `None`.
        if row == Row::Backrow
            && let Some(face_up) = entry.entry.face_up
        {
            card.face_up = Some(face_up);
        }
    }
    Ok(())
}

fn place_pile(
    sink: &mut EngineSink<'_>,
    player: PlayerId,
    zone: OffFieldZone,
    refs: &[PileSetup],
    label: &str,
) -> Result<(), String> {
    for (at, entry) in refs.iter().enumerate() {
        let at_ = format!("{label}[{at}]");
        let def_id = setup_def_id(sink.state, &ref_of(pile_ref(entry), &at_)?, &at_)?;
        let (radiant, cost, created) = match entry {
            PileSetup::Name(_) => (false, None, None),
            PileSetup::Entry(fields) => (
                fields.radiant == Some(true),
                Some(&fields.cost),
                fields.created.filter(|created| *created),
            ),
        };

        if zone == OffFieldZone::Hand {
            let _ = crate::setup::create_in_hand(&mut *sink, player, &def_id);
            // `create_in_hand` pushes the new card onto the end of the hand.
            if let Some(card) = sink.state.players[player].hand.last_mut() {
                if radiant {
                    card.radiant = true;
                }
                // MD-B6, R943: the setup defaults to dealt; only `created: true` keeps the mark.
                card.created = created;
                apply_cost_setup(card, cost);
            }
            continue;
        }

        let mut card = new_instance(&mut *sink.state, &def_id, player, zone.zone_for(player));
        if radiant {
            card.radiant = true;
        }
        // MD-B6, R943: the setup defaults to dealt; only `created: true` keeps the mark.
        card.created = created;
        apply_cost_setup(&mut card, cost);
        let id = card.id.clone();
        // `position: "bottom"` keeps list order, so `library[0]` is the next card drawn (draw.rs).
        let result = crate::zones::move_to_zone(
            &mut *sink.state,
            &mut card,
            zone,
            MoveToZoneOptions {
                position: Some(LibraryPosition::Bottom),
                ..MoveToZoneOptions::default()
            },
        );
        // R311: a scenario's library stands for its owner's deck, which they know card by card. A deck
        // is dealt on its base face, so a card set Radiant here became Radiant where nobody saw it
        // (#28, #42): what its owner was shown is the base face.
        if zone == OffFieldZone::Library
            && result == MoveResult::Moved
            && let Some(card) = find_instance_mut(&mut *sink.state, &id)
        {
            show_to_owner(card);
            if let Some(known) = card.known_as.as_mut() {
                known.radiant = false;
            }
        }
        if result == MoveResult::Vanished {
            let name = def_of(sink.state, &def_id)?.name;
            return Err(format!(
                "{at_}: \"{name}\" is a unit token, and R11 makes one cease to exist on the way to a {}; put it \
                 in a hand or a library instead",
                zone.as_str()
            ));
        }
    }
    Ok(())
}

fn place_side(sink: &mut EngineSink<'_>, player: PlayerId, setup: &SideSetup) -> Result<(), String> {
    let seat = player.as_str();
    place_pile(
        sink,
        player,
        OffFieldZone::Hand,
        setup.hand.as_deref().unwrap_or(&[]),
        &format!("{seat}.hand"),
    )?;

    // `field` and `backrow` are one board: each entry's row comes from its def's type (or its own
    // `row`), and lanes are then handed out per row over the whole board, `field` entries first.
    let mut placements: Vec<Placement> = Vec::new();
    for (at, entry) in setup.field.iter().flatten().enumerate() {
        placements.push(normalize_placement(
            sink.state,
            entry,
            Row::Units,
            &format!("{seat}.field[{at}]"),
        )?);
    }
    for (at, entry) in setup.backrow.iter().flatten().enumerate() {
        placements.push(normalize_placement(
            sink.state,
            entry,
            Row::Backrow,
            &format!("{seat}.backrow[{at}]"),
        )?);
    }
    for row in [Row::Units, Row::Backrow] {
        let in_row: Vec<usize> = (0..placements.len())
            .filter(|&at| placements[at].row == row)
            .collect();
        let entries: Vec<(Option<i32>, bool)> = in_row
            .iter()
            .map(|&at| (placements[at].lane, placements[at].entry.stack == Some(true)))
            .collect();
        let lanes = assign_lanes(&entries, row_size_of(row), &format!("{seat}.{}", row.as_str()))?;
        for (at, &index) in in_row.iter().enumerate() {
            placements[index].lane = lanes.get(at).copied();
        }
    }
    for entry in &placements {
        place_one(sink, player, entry)?;
    }

    place_pile(
        sink,
        player,
        OffFieldZone::Library,
        setup.library.as_deref().unwrap_or(&[]),
        &format!("{seat}.library"),
    )?;
    place_pile(
        sink,
        player,
        OffFieldZone::Graveyard,
        setup.graveyard.as_deref().unwrap_or(&[]),
        &format!("{seat}.graveyard"),
    )?;
    place_pile(
        sink,
        player,
        OffFieldZone::Exile,
        setup.exile.as_deref().unwrap_or(&[]),
        &format!("{seat}.exile"),
    )?;
    Ok(())
}

fn setup_of(opts: &ScenarioOptions, player: PlayerId) -> SideSetup {
    match player {
        PlayerId::P1 => opts.p1.clone().unwrap_or_default(),
        PlayerId::P2 => opts.p2.clone().unwrap_or_default(),
    }
}

fn build_state(opts: &ScenarioOptions) -> Result<GameState, String> {
    let seed = opts.seed.clone().unwrap_or_else(|| DEFAULT_SEED.to_string());
    let deck = filler_deck()?;
    let mut state = create_game(&CreateGameOptions {
        seed,
        decks: (deck.clone(), deck),
        last_boards: opts.last_boards.clone(),
        glitch_boards: opts.glitch_boards.clone(),
        ..CreateGameOptions::default()
    });

    // The filler libraries exist only to satisfy §2.6's deck validation.
    for player in PLAYER_IDS {
        let library = state.players[player].library.clone();
        for card in &library {
            crate::zones::remove_from_any_zone(&mut state, &mut card.clone());
        }
    }
    state.next_id = 1;

    let active = opts.active.unwrap_or(PlayerId::P1);
    let turn = match opts.turn {
        None => DEFAULT_TURN,
        Some(turn) if turn.is_finite() && turn.fract() == 0.0 && turn >= 0.0 => turn as i32,
        Some(turn) => {
            return Err(format!(
                "scenario: turn must be a non-negative integer, got {turn}"
            ));
        }
    };
    state.active = active;
    state.turn = turn;
    state.phase = Phase::Main;
    state.players[active].turns_started = ((turn + 1) / 2).max(0);
    state.players[opponent_of(active)].turns_started = (turn / 2).max(0);

    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);

    // p1 then p2, and within a side hand → field → backrow → library → graveyard → exile, so the
    // instance ids a scenario hands out are a function of the setup literal alone.
    {
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        for player in PLAYER_IDS {
            place_side(&mut sink, player, &setup_of(opts, player))?;
        }
    }

    for player in PLAYER_IDS {
        let setup = setup_of(opts, player);
        let side = &mut state.players[player];
        crate::mana::refresh_mana(side);
        if let Some(mana) = setup.mana {
            side.mana.current = mana;
        }
        if let Some(health) = setup.health {
            side.hero.health = health;
        }
        if let Some(armor) = setup.armor {
            side.hero.armor = armor;
        }
    }

    // §4.5: settle the board once, so a unit placed at lethal damage dies like it would in play.
    {
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        crate::state_check::state_check(&mut sink);
    }
    state.rng_cursor = rng.cursor();
    Ok(state)
}

// ---------------------------------------------------------------------------------------------
// Refusals (TS `expect(() => …).toThrow(/text/)`)
// ---------------------------------------------------------------------------------------------

/// A panic's message, as `toThrow` read an Error's.
fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<String>() {
        return text.clone();
    }
    if let Some(text) = payload.downcast_ref::<&str>() {
        return (*text).to_string();
    }
    String::from("(a panic with no message)")
}

/// What `run` panicked with, or `None` when it ran through.
fn refusal_of(run: impl FnOnce()) -> Option<String> {
    match catch_unwind(AssertUnwindSafe(run)) {
        Ok(()) => None,
        Err(payload) => Some(panic_text(payload.as_ref())),
    }
}

/// TS `expect(() => …).toThrow()` for a call that is not a scenario step (`scenario(…)` itself).
pub fn expect_throw(run: impl FnOnce()) {
    if refusal_of(run).is_none() {
        panic!("expect_throw: the call ran through; it should have been refused");
    }
}

/// TS `expect(() => …).toThrow(/text/)`: the refusal's message contains `text`.
pub fn expect_throw_with(run: impl FnOnce(), text: &str) {
    match refusal_of(run) {
        None => {
            panic!("expect_throw_with: the call ran through; it should have been refused with \"{text}\"")
        }
        Some(message) if !message.contains(text) => {
            panic!("expect_throw_with: the refusal \"{message}\" does not contain \"{text}\"")
        }
        Some(_) => {}
    }
}

fn or_fail<T>(result: Result<T, String>) -> T {
    match result {
        Ok(value) => value,
        Err(message) => panic!("{message}"),
    }
}

/// An options literal (`json!({ … })`, or `Value::Null` for none) as the step's typed options.
fn options_of<T: DeserializeOwned + Default>(opts: Value, what: &str) -> T {
    if opts.is_null() {
        return T::default();
    }
    match serde_json::from_value::<T>(opts.clone()) {
        Ok(typed) => typed,
        Err(error) => panic!("{what}: the options {opts} do not read: {error}"),
    }
}

fn json_text<T: serde::Serialize + ?Sized>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

// ---------------------------------------------------------------------------------------------
// The scenario
// ---------------------------------------------------------------------------------------------

/// TS `Scenario` (the `Harness` class): the live state, the event log, and the steps, reads and
/// assertions. Every step and assertion answers `&mut Scenario`, so they chain as TS's did.
pub struct Scenario {
    current: GameState,
    log: Vec<GameEvent>,
    last: Vec<GameEvent>,
    step: u32,
}

impl Scenario {
    /// TS `new Harness(opts)`.
    pub fn new(opts: &ScenarioOptions) -> Scenario {
        Scenario {
            current: or_fail(build_state(opts)),
            log: Vec::new(),
            last: Vec::new(),
            step: 0,
        }
    }

    /// The live state; it follows every step (`reduce` returns a new state, and this tracks it).
    pub fn state(&self) -> &GameState {
        &self.current
    }

    /// The live state, for a test that edits the board between steps (TS wrote `s.state.…` directly).
    pub fn state_mut(&mut self) -> &mut GameState {
        &mut self.current
    }

    /// Every event since setup finished, oldest first.
    pub fn events(&self) -> &[GameEvent] {
        &self.log
    }

    /// The events of the most recent step only.
    pub fn last_events(&self) -> &[GameEvent] {
        &self.last
    }

    // --- refs -----------------------------------------------------------------------------------

    /// A reference as the live instance's id, searched where `scope` says.
    fn resolve(&self, reference: &CardRef, scope: Where, what: &str) -> Result<String, String> {
        let state = &self.current;

        let text = match reference {
            CardRef::Instance { id, def_id } => {
                let Some(live) = find_instance(state, id) else {
                    return Err(format!("{what}: {id} ({def_id}) is in no zone any more"));
                };
                let pool = search_order(state, scope);
                if !pool.iter().any(|card| card.id == live.id) {
                    return Err(format!(
                        "{what}: {} is not in {} — it is in {}",
                        describe_instance(state, live),
                        where_label(scope),
                        self.locate(&live.id)
                    ));
                }
                return Ok(live.id.clone());
            }
            CardRef::Text(text) => text,
        };

        let pool = search_order(state, scope);
        if is_instance_id(text)
            && let Some(by_id) = pool.iter().find(|card| card.id == *text)
        {
            return Ok(by_id.id.clone());
        }

        let def_ids = def_ids_for(state, text);
        if let Some(by_def) = pool.iter().find(|card| def_ids.contains(&card.def_id)) {
            return Ok(by_def.id.clone());
        }

        let listing: Vec<String> = pool.iter().map(|card| describe_instance(state, card)).collect();
        let listing = if listing.is_empty() {
            String::from("nothing")
        } else {
            listing.join(", ")
        };
        let known = if def_ids.is_empty() {
            String::new()
        } else {
            format!(" (that names {}, of which no copy is there)", def_ids.join(", "))
        };
        Err(format!(
            "{what}: nothing matching \"{text}\" is in {}{known}; found {listing}",
            where_label(scope)
        ))
    }

    /// TS `Located`: "hand" | "library" | "graveyard" | "exile" | "resolving" | "field" | "gone".
    fn locate(&self, id: &str) -> &'static str {
        for player in PLAYER_IDS {
            let side = &self.current.players[player];
            if side.hand.iter().any(|card| card.id == id) {
                return "hand";
            }
            if side.library.iter().any(|card| card.id == id) {
                return "library";
            }
            if side.graveyard.iter().any(|card| card.id == id) {
                return "graveyard";
            }
            if side.exile.iter().any(|card| card.id == id) {
                return "exile";
            }
            if side.resolving.iter().any(|card| card.id == id) {
                return "resolving";
            }
            if side.units.iter().flatten().flatten().any(|card| card.id == id) {
                return "field";
            }
            if side.backrow.iter().flatten().any(|card| card.id == id) {
                return "field";
            }
            if side
                .backrow_piles
                .iter()
                .flatten()
                .flatten()
                .any(|card| card.id == id)
            {
                return "field";
            }
        }
        "gone"
    }

    /// An instance id for a reference, even when the instance has ceased to exist (R11).
    fn ref_id(&self, reference: &CardRef, zone: &str) -> Result<String, String> {
        let text = match reference {
            CardRef::Instance { id, .. } => return Ok(id.clone()),
            CardRef::Text(text) => text,
        };
        match self.resolve(reference, Where::Any, "expectInZone") {
            Ok(id) => Ok(id),
            Err(_) if is_instance_id(text) => Ok(text.clone()),
            Err(message) => {
                let hint = if zone == "gone" {
                    " — to assert \"gone\" pass the CardInstance you were handed, or its instance id (\"c7\")"
                } else {
                    ""
                };
                Err(format!("{message}{hint}"))
            }
        }
    }

    /// The live instance a resolved id names.
    fn live(&self, id: &str) -> &CardInstance {
        match find_instance(&self.current, id) {
            Some(card) => card,
            None => panic!("{id} is in no zone any more"),
        }
    }

    pub fn card(&self, card: impl Into<CardRef>) -> &CardInstance {
        let id = or_fail(self.resolve(&card.into(), Where::Any, "card"));
        self.live(&id)
    }

    /// The live instance, for a test that edits a card between steps (TS wrote through `s.card(…)`'s
    /// live object). Resolved as `card()` resolves it.
    pub fn card_mut(&mut self, card: impl Into<CardRef>) -> &mut CardInstance {
        let id = or_fail(self.resolve(&card.into(), Where::Any, "card"));
        match find_instance_mut(&mut self.current, &id) {
            Some(card) => card,
            None => panic!("{id} is in no zone any more"),
        }
    }

    /// §10.4's computed view: attack, maxHealth, health, keywords, armor, position.
    pub fn stats(&self, card: impl Into<CardRef>) -> UnitView {
        let id = or_fail(self.resolve(&card.into(), Where::Any, "stats"));
        unit_view(&self.current, self.live(&id))
    }

    // --- engine plumbing ------------------------------------------------------------------------

    /// An action through `reduce`, which derives its rng from (seed, cursor) itself. Every action gets
    /// its own deterministic nonce, or `reduce`'s dedupe would replay the last result instead of
    /// applying this one.
    fn try_action(&mut self, body: ActionBody, player: PlayerId) -> Option<String> {
        self.step += 1;
        let action = Action::new(body, player, format!("h{}", self.step));
        let result = crate::reduce::reduce(&self.current, &action);
        if let Some(error) = result.error {
            return Some(error);
        }
        self.current = result.state;
        self.last = result.events.clone();
        self.log.extend(result.events);
        None
    }

    fn action(&mut self, body: ActionBody, player: PlayerId, what: &str) {
        if let Some(error) = self.try_action(body, player) {
            panic!("{error} — {what}");
        }
    }

    /// A direct engine call, threading the rng cursor the way `reduce` does. The state is changed in
    /// place (no copy), and nothing is merged into the log if `run` panics.
    fn direct(&mut self, run: impl FnOnce(&mut EngineSink<'_>)) {
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&self.current.seed, self.current.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut self.current, &mut events, &mut rng);
            run(&mut sink);
        }
        self.current.rng_cursor = rng.cursor();
        self.last = events.clone();
        self.log.extend(events);
    }

    // --- steps ----------------------------------------------------------------------------------

    /// `opts`: `json!({ "zone"?, "row"?, "x"?, "embiggen"?, "magnetic"?, "targets"?, "modes"?,
    /// "tributes"?, "faceDown"? })`.
    pub fn play(&mut self, card: impl Into<CardRef>, opts: Value) -> &mut Scenario {
        let opts: PlayOptions = options_of(opts, "play");
        let id = or_fail(self.resolve(&card.into(), Where::Hand, "play"));
        let inst = self.live(&id).clone();
        let def = or_fail(def_of(&self.current, &inst.def_id));
        let what = format!("play {}", describe_instance(&self.current, &inst));

        let mut zone: Option<ZoneChoice> = None;
        if let Some(lane) = opts.zone {
            if def.type_ == CardType::Spell && opts.face_down.is_none() {
                panic!("{what}: a Spell takes no zone, and zone {lane} was given");
            }
            let row = opts.row.unwrap_or(if def.type_ == CardType::Unit {
                Row::Units
            } else {
                Row::Backrow
            });
            let size = row_size_of(row);
            if lane < 1 || lane > size {
                panic!(
                    "{what}: zone {lane} is out of range; {} lanes are 1..{size}",
                    row.as_str()
                );
            }
            zone = Some(ZoneChoice { row, lane });
        }

        let tributes: Option<Vec<String>> = opts.tributes.as_ref().map(|refs| {
            refs.iter()
                .map(|reference| {
                    or_fail(self.resolve(
                        &CardRef::from(reference),
                        Where::Field,
                        &format!("{what} (tribute)"),
                    ))
                })
                .collect()
        });

        self.action(
            ActionBody::Play {
                instance_id: inst.id.clone(),
                zone,
                x: opts.x,
                embiggen: opts.embiggen,
                magnetic: opts.magnetic,
                tributes,
                targets: opts.targets,
                modes: opts.modes,
                plague: None,
                face_down: opts.face_down,
            },
            inst.controller,
            &what,
        );
        self
    }

    /// `target` is a card reference or `"hero"`, the enemy hero (`hero-<player>`).
    pub fn attack(&mut self, attacker: impl Into<CardRef>, target: impl Into<CardRef>) -> &mut Scenario {
        let source_id = or_fail(self.resolve(&attacker.into(), Where::Field, "attack"));
        let source = self.live(&source_id).clone();
        let enemy = opponent_of(source.controller);
        let target_id = match target.into() {
            CardRef::Text(text) if text == "hero" => format!("hero-{}", enemy.as_str()),
            other => or_fail(self.resolve(&other, Where::Field, "attack (target)")),
        };
        let what = format!(
            "attack {} → {target_id}",
            describe_instance(&self.current, &source)
        );

        self.action(
            ActionBody::Attack {
                attacker_id: source.id.clone(),
                target_id,
            },
            source.controller,
            &what,
        );
        self
    }

    /// `selection`: a `Selection[]` (`json!([{ "pick": "none" }])`), a string or a list of strings
    /// matched against the open prompt's options (`to_selections`).
    pub fn answer(&mut self, selection: Value) -> &mut Scenario {
        let Some(pending) = self.current.pending.clone() else {
            panic!(
                "answer({}): no prompt is open (phase {}, turn {}, active {})",
                json_text(&selection),
                self.current.phase.as_str(),
                self.current.turn,
                self.current.active.as_str()
            );
        };
        let picks = or_fail(to_selections(&self.current, &selection, &pending));
        let who = pending.player_id;
        let what = format!(
            "answer {} prompt {} with {}",
            pending.kind.as_str(),
            pending.id,
            json_text(&picks)
        );
        self.action(
            ActionBody::Answer {
                choice_id: pending.id.clone(),
                selection: picks,
            },
            who,
            &what,
        );
        self
    }

    /// A raw engine action, for the actions with no step helper — the `Emote` action (MD-D29,
    /// R1127). Panics when `reduce` refuses it, like every other step.
    pub fn act(&mut self, body: ActionBody, player: PlayerId, what: &str) -> &mut Scenario {
        self.action(body, player, what);
        self
    }

    pub fn end_turn(&mut self) -> &mut Scenario {
        let active = self.current.active;
        self.action(
            ActionBody::EndTurn,
            active,
            &format!("endTurn for {}", active.as_str()),
        );
        self
    }

    pub fn start_turn(&mut self) -> &mut Scenario {
        self.direct(|sink| {
            let active = sink.state.active;
            crate::turn::start_turn(sink, active);
        });
        self
    }

    pub fn switch_position(&mut self, card: impl Into<CardRef>) -> &mut Scenario {
        let id = or_fail(self.resolve(&card.into(), Where::Field, "switchPosition"));
        let unit = self.live(&id).clone();
        let what = format!("switchPosition {}", describe_instance(&self.current, &unit));
        self.action(
            ActionBody::SwitchPosition {
                instance_id: unit.id.clone(),
            },
            unit.controller,
            &what,
        );
        self
    }

    /// `opts`: `json!({ "targets"?, "ability"?, "modes"?, "tributes"? })`.
    pub fn activate(&mut self, card: impl Into<CardRef>, opts: Value) -> &mut Scenario {
        let opts: ActivateOptions = options_of(opts, "activate");
        let id = or_fail(self.resolve(&card.into(), Where::Field, "activate"));
        let source = self.live(&id).clone();
        let who = source.controller;
        let what = format!("activate {}", describe_instance(&self.current, &source));
        if opts.ability.is_some() || opts.modes.is_some() || opts.tributes.is_some() {
            let tributes: Option<Vec<String>> = opts.tributes.as_ref().map(|refs| {
                refs.iter()
                    .map(|reference| {
                        or_fail(self.resolve(
                            &CardRef::from(reference),
                            Where::Field,
                            &format!("{what} (tribute)"),
                        ))
                    })
                    .collect()
            });
            self.action(
                ActionBody::Activate {
                    instance_id: source.id.clone(),
                    ability: opts.ability,
                    targets: opts.targets,
                    modes: opts.modes,
                    tributes,
                },
                who,
                &what,
            );
            return self;
        }
        // R752: `activatePower`, the alias that names a Heroic Power's rolled power, or a card's only ability.
        self.action(
            ActionBody::ActivatePower {
                instance_id: source.id.clone(),
                targets: opts.targets,
            },
            who,
            &what,
        );
        self
    }

    // --- reads ----------------------------------------------------------------------------------

    pub fn view(&self, player: impl SeatRef) -> PlayerView {
        let viewer = player.seat_or(self.current.active);
        crate::view_for::view_for(&self.current, viewer)
    }

    pub fn unit(&self, player: impl SeatRef, lane: i32) -> Option<CardInstance> {
        Scenario::check_lane("unit", Row::Units, lane);
        let player = player.seat_or(self.current.active);
        crate::zones::card_at(
            &self.current,
            ZoneSlot {
                player,
                row: Row::Units,
                lane,
            },
        )
        .cloned()
    }

    pub fn backrow(&self, player: impl SeatRef, lane: i32) -> Option<CardInstance> {
        Scenario::check_lane("backrow", Row::Backrow, lane);
        let player = player.seat_or(self.current.active);
        crate::zones::card_at(
            &self.current,
            ZoneSlot {
                player,
                row: Row::Backrow,
                lane,
            },
        )
        .cloned()
    }

    pub fn hand(&self, player: impl SeatRef) -> Vec<CardInstance> {
        self.current.players[player.seat_or(self.current.active)]
            .hand
            .clone()
    }

    /// `zone`: "hand", "library", "graveyard" or "exile" (TS `PileName`).
    pub fn pile(&self, player: impl SeatRef, zone: &str) -> Vec<CardInstance> {
        let side = &self.current.players[player.seat_or(self.current.active)];
        match zone {
            "hand" => side.hand.clone(),
            "library" => side.library.clone(),
            "graveyard" => side.graveyard.clone(),
            "exile" => side.exile.clone(),
            other => {
                panic!("pile(): \"{other}\" is not a pile; piles are hand, library, graveyard and exile")
            }
        }
    }

    fn check_lane(what: &str, row: Row, lane: i32) {
        let size = row_size_of(row);
        if lane < 1 || lane > size {
            panic!(
                "{what}(): lane {lane} is out of range; {} lanes are 1..{size}",
                row.as_str()
            );
        }
    }

    // --- assertions -----------------------------------------------------------------------------

    /// `zone`: "hand", "library", "graveyard", "exile", "field" or "gone" (TS `ZoneName`).
    pub fn expect_in_zone(&mut self, card: impl Into<CardRef>, zone: &str) -> &mut Scenario {
        let id = or_fail(self.ref_id(&card.into(), zone));
        let actual = self.locate(&id);
        let who = match find_instance(&self.current, &id) {
            None => id.clone(),
            Some(known) => describe_instance(&self.current, known),
        };
        if actual != zone {
            panic!("expectInZone: {who} should be in {zone} but is in {actual}");
        }
        self
    }

    /// `stats`: `json!({ "attack"?, "health"?, "maxHealth"? })`.
    pub fn expect_stats(&mut self, card: impl Into<CardRef>, stats: Value) -> &mut Scenario {
        let id = or_fail(self.resolve(&card.into(), Where::Any, "expectStats"));
        let inst = self.live(&id);
        let view = unit_view(&self.current, inst);
        let who = describe_instance(&self.current, inst);
        let seen = format!(
            "attack {}, health {}/{}, damage {}, buffs +{}/+{}",
            view.attack, view.health, view.max_health, inst.damage, inst.buffs.attack, inst.buffs.health
        );
        for (key, actual) in [
            ("attack", view.attack),
            ("health", view.health),
            ("maxHealth", view.max_health),
        ] {
            let Some(want) = stats.get(key).filter(|want| !want.is_null()) else {
                continue;
            };
            if want.as_i64() != Some(i64::from(actual)) {
                panic!("expectStats: {who} {key} should be {want} but is {actual} — {seen}");
            }
        }
        self
    }

    /// `types`: `json!(["cardPlayed", "summoned"])` (or one type as a string), a subsequence of the log.
    pub fn expect_events(&mut self, types: Value) -> &mut Scenario {
        let wanted: Vec<String> = match &types {
            Value::String(one) => vec![one.clone()],
            Value::Array(items) => items
                .iter()
                .map(|item| match item.as_str() {
                    Some(text) => text.to_string(),
                    None => panic!("expectEvents: {item} is not an event type"),
                })
                .collect(),
            other => panic!("expectEvents: {other} is not a list of event types"),
        };
        let seen: Vec<&str> = self.log.iter().map(|event| event.event_type().as_str()).collect();
        let mut at = 0;
        let mut missing: Vec<&str> = Vec::new();
        for kind in &wanted {
            match seen[at..].iter().position(|seen_kind| seen_kind == kind) {
                Some(found) => at += found + 1,
                None => {
                    missing.push(kind);
                    break;
                }
            }
        }
        if !missing.is_empty() {
            let log = if seen.is_empty() {
                String::from("nothing")
            } else {
                seen.join(", ")
            };
            panic!(
                "expectEvents: {} is not a subsequence of the log — the log holds {log}",
                wanted.join(" → ")
            );
        }
        self
    }

    pub fn expect_health(&mut self, player: impl SeatRef, health: i32) -> &mut Scenario {
        let player = player.seat_or(self.current.active);
        let actual = self.current.players[player].hero.health;
        if actual != health {
            panic!(
                "expectHealth: {}'s hero should be at {health} but is at {actual}",
                player.as_str()
            );
        }
        self
    }

    pub fn expect_mana(&mut self, player: impl SeatRef, current: i32) -> &mut Scenario {
        let player = player.seat_or(self.current.active);
        let mana = self.current.players[player].mana;
        if mana.current != current {
            panic!(
                "expectMana: {} should have {current} mana but has {} of {}",
                player.as_str(),
                mana.current,
                mana.max
            );
        }
        self
    }

    /// TS `expect(() => s.step(…)).toThrow()`: the step is refused (it panics), and the scenario goes
    /// on from the state the refusal left — a refused action leaves it untouched.
    pub fn expect_refused(
        &mut self,
        step: impl for<'a> FnOnce(&'a mut Scenario) -> &'a mut Scenario,
    ) -> &mut Scenario {
        if refusal_of(|| {
            step(&mut *self);
        })
        .is_none()
        {
            panic!("expect_refused: the step ran through; it should have been refused");
        }
        self
    }

    /// TS `expect(() => s.step(…)).toThrow(/text/)`: refused, with `text` in the message.
    pub fn expect_refused_with(
        &mut self,
        step: impl for<'a> FnOnce(&'a mut Scenario) -> &'a mut Scenario,
        text: &str,
    ) -> &mut Scenario {
        match refusal_of(|| {
            step(&mut *self);
        }) {
            None => panic!(
                "expect_refused_with: the step ran through; it should have been refused with \"{text}\""
            ),
            Some(message) if !message.contains(text) => {
                panic!("expect_refused_with: the refusal \"{message}\" does not contain \"{text}\"")
            }
            Some(_) => {}
        }
        self
    }
}

/// §10.6: a prompt's answer is a `Selection[]`. A string (or a list of strings) is matched against
/// `state.pending.options` by `key`, then `label`, then a mode option, then the option's
/// `instanceId`, then that instance's `defId`, then the label case-insensitively, and finally by
/// resolving the string the way every other harness reference is resolved.
fn to_selections(
    state: &GameState,
    selection: &Value,
    pending: &PendingChoice,
) -> Result<Vec<Selection>, String> {
    match selection {
        Value::String(text) => Ok(vec![match_option(state, text, pending)?]),
        Value::Array(items) if items.is_empty() => Ok(Vec::new()),
        Value::Array(items) if items[0].is_string() => items
            .iter()
            .map(|item| match item.as_str() {
                Some(text) => match_option(state, text, pending),
                None => Err(format!("answer: {item} is not a string, as the first pick was")),
            })
            .collect(),
        Value::Array(_) => serde_json::from_value::<Vec<Selection>>(selection.clone())
            .map_err(|error| format!("answer({selection}): not a list of selections: {error}")),
        other => Err(format!(
            "answer({other}): an answer is a list of selections, a string or a list of strings"
        )),
    }
}

fn match_option(state: &GameState, text: &str, pending: &PendingChoice) -> Result<Selection, String> {
    let instance_id_of = |pick: &Selection| -> Option<String> {
        match pick {
            Selection::Instance { instance_id } => Some(instance_id.clone()),
            _ => None,
        }
    };
    let def_id_of = |pick: &Selection| -> Option<String> {
        instance_id_of(pick).and_then(|id| find_instance(state, &id).map(|card| card.def_id.clone()))
    };
    let lower = text.to_lowercase();

    let tests: [&dyn Fn(&PromptOption) -> bool; 7] = [
        &|option: &PromptOption| option.key == text,
        &|option: &PromptOption| option.label == text,
        &|option: &PromptOption| matches!(&option.selection, Selection::Mode { option: mode } if mode == text),
        &|option: &PromptOption| instance_id_of(&option.selection).as_deref() == Some(text),
        &|option: &PromptOption| def_id_of(&option.selection).as_deref() == Some(text),
        &|option: &PromptOption| option.label.to_lowercase() == lower,
        &|option: &PromptOption| {
            let def_ids = def_ids_for(state, text);
            def_id_of(&option.selection).is_some_and(|def_id| def_ids.contains(&def_id))
        },
    ];

    for test in tests {
        if let Some(hit) = pending.options.iter().find(|&option| test(option)) {
            return Ok(hit.selection.clone());
        }
    }

    let listing: Vec<String> = pending
        .options
        .iter()
        .map(|option| format!("{} ({})", option.key, option.label))
        .collect();
    let listing = if listing.is_empty() {
        String::from("nothing")
    } else {
        listing.join(", ")
    };
    Err(format!(
        "answer(\"{text}\"): the {} prompt {} has no such option. Keys and labels on offer: {listing}. Pick {}..{}.",
        pending.kind.as_str(),
        pending.id,
        pending.min,
        pending.max
    ))
}

/// Build a scenario from its options literal (`json!({ "p1": {…}, "p2": {…}, "seed"?, "turn"?,
/// "active"?, "lastBoards"?, "glitchBoards"? })`, or `json!({})`). See the file header for every
/// default and every documented decision.
pub fn scenario(opts: Value) -> Scenario {
    let opts: ScenarioOptions = options_of(opts, "scenario");
    Scenario::new(&opts)
}

#[cfg(test)]
mod tests {
    //! The harness proves itself (BUILD M4-T3). Every case leans on cards whose scripts do not
    //! matter to it and asserts ENGINE behaviour, never card text:
    //!
    //!   core-025 "4-mana 7/7"  Unit, cost 4, 7/7 Armor 7 / radiant 7/7 Indestructible — keywords only
    //!   core-056 "Jilliax"     Unit, cost 2, 3/2 Rush Taunt Lifesteal Divine Shield — keywords only
    //!   core-043 "Big Felinor" Unit, cost 3, 3/10 / radiant 6/20 — the radiant stat difference
    //!   core-010 "Rapid Replenish" Spell, cost 0 — a play that ends in the graveyard
    //!   core-041 "Sheepish"    Trap, and core-073 "Anti-oneshot Armor" Field Spell — backrow placement
    //!
    //! These tests run inside the engine crate, which cannot link `jackioh_cards` (it would be a
    //! second copy of the engine), so they install the real catalog through the thread-local
    //! override and run with no card scripts: no case here plays a card whose script would act
    //! (core-025 and core-056 have none; core-010's draw needs three earlier plays; the rest are only
    //! placed or drawn). `activate()` has no case here: Heroic Power (#98) is its only user and that
    //! card's own test covers it. A prompt CHAIN needs a card script that opens one, which is a card
    //! file's test.
    //!
    //! TURN HAZARD: `reduce` auto-ends a turn with nothing meaningful left on it (§2.5), so scenarios
    //! that cross a turn boundary give both sides something to do. See the harness header.

    use super::*;
    use crate::config::{HAND_CAP, MAX_MANA};
    use crate::mana::effective_cost;
    use crate::wire::{KeywordKind, ZoneName};
    use serde_json::json;

    /// The real catalog, as `registerAll()` registered it for the TS harness.
    fn install_catalog() {
        if catalog_override().is_none() {
            let defs: CardDefs = serde_json::from_str(include_str!("../../../cards/catalog.json"))
                .expect("crates/cards/catalog.json parses as CardDefs");
            register_catalog(defs);
        }
    }

    fn scenario(opts: Value) -> Scenario {
        install_catalog();
        super::scenario(opts)
    }

    /// A unit each side can always act with, so `reduce` never auto-ends a turn under a test.
    fn anchor() -> Value {
        json!({ "def": "core-056", "lane": 5 })
    }

    /// The same job as `anchor()` for a test that owns the board: #10 Rapid Replenish is a 0-cost Spell
    /// and §8 row 10 says "always playable", so holding one is a legal action for R82's purposes and the
    /// turn cannot auto-end. A combat test wants this rather than `anchor()`, because `anchor()` is a
    /// second `core-056` and these tests name their attacker by def id (`s.attack("core-056", …)`),
    /// which is a first-match lookup.
    const HAND_ANCHOR: &str = "core-010";

    /// A whole §3.2 Stack pile, top-first, which `unit()` cannot give: it answers with the card on top.
    /// `state.players[p].units[lane - 1]` is the pile itself (`Pile = Vec<CardInstance>`, §10.1).
    fn pile_at(s: &Scenario, player: PlayerId, lane: usize) -> Vec<CardInstance> {
        s.state().players[player].units[lane - 1]
            .clone()
            .unwrap_or_default()
    }

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    fn def_id_at(card: Option<CardInstance>) -> Option<String> {
        card.map(|card| card.def_id)
    }

    fn some(id: &str) -> Option<String> {
        Some(id.to_string())
    }

    mod harness_setup {
        use super::*;

        #[test]
        fn places_hand_field_backrow_library_graveyard_and_exile_cards_in_the_right_zones() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": ["core-025", "core-010"],
                    "field": [{ "def": "core-043", "radiant": true }, { "def": "core-025", "position": "DEF", "damage": 3, "lane": 4 }],
                    "backrow": ["core-041", { "def": "core-073", "faceUp": true, "lane": 3 }],
                    "library": ["core-056", "core-002"],
                    "graveyard": ["core-005"],
                    "exile": ["core-016"],
                },
                "p2": { "field": ["core-056"] },
            }));

            assert_eq!(def_ids(&s.hand("p1")), ["core-025", "core-010"]);
            assert_eq!(def_ids(&s.pile("p1", "library")), ["core-056", "core-002"]);
            assert_eq!(def_ids(&s.pile("p1", "graveyard")), ["core-005"]);
            assert_eq!(def_ids(&s.pile("p1", "exile")), ["core-016"]);

            // §3.2: an entry with no `lane` takes the leftmost zone still free, so the pinned lane 4 stands.
            assert_eq!(def_id_at(s.unit("p1", 1)), some("core-043"));
            assert!(s.unit("p1", 2).is_none());
            assert_eq!(def_id_at(s.unit("p1", 4)), some("core-025"));
            assert_eq!(def_id_at(s.unit("p2", 1)), some("core-056"));

            s.expect_in_zone("core-010", "hand")
                .expect_in_zone("core-043", "field")
                .expect_in_zone("core-005", "graveyard")
                .expect_in_zone("core-016", "exile")
                .expect_in_zone("core-002", "library");
        }

        #[test]
        fn honours_radiant_position_damage_and_face_up() {
            let mut s = scenario(json!({
                "p1": {
                    "field": [{ "def": "core-043", "radiant": true }, { "def": "core-025", "position": "DEF", "damage": 3, "lane": 4 }],
                    "backrow": ["core-041", { "def": "core-073", "faceUp": true, "lane": 3 }],
                },
            }));

            // §10.4 layer 1: the radiant face is the printed face once the instance is Radiant.
            assert_eq!(s.unit("p1", 1).map(|card| card.radiant), Some(true));
            s.expect_stats("core-043", json!({ "attack": 6, "maxHealth": 20, "health": 20 }));

            let seven = s.unit("p1", 4).expect("a unit in lane 4");
            assert_eq!(seven.position, Some(Position::Def));
            assert_eq!(seven.damage, 3);
            // §4.1: Defense Position grants Taunt and Armor +1 on top of the printed Armor 7.
            s.expect_stats(&seven, json!({ "attack": 7, "maxHealth": 7, "health": 4 }));
            assert_eq!(s.stats(&seven).armor, 8);
            assert!(
                s.stats(&seven)
                    .keywords
                    .iter()
                    .any(|keyword| keyword.kind() == KeywordKind::Taunt)
            );
            assert_eq!(s.stats(&seven).position, Position::Def);

            assert_eq!(def_id_at(s.backrow("p1", 1)), some("core-041"));
            assert_eq!(s.backrow("p1", 1).and_then(|card| card.face_up), None);
            assert_eq!(def_id_at(s.backrow("p1", 3)), some("core-073"));
            assert_eq!(s.backrow("p1", 3).and_then(|card| card.face_up), Some(true));
            assert!(s.backrow("p1", 2).is_none());
        }

        #[test]
        fn library_0_is_the_next_card_drawn() {
            let mut s =
                scenario(json!({ "p1": { "field": [anchor()], "library": ["core-056", "core-002"] } }));
            s.start_turn();
            assert_eq!(def_ids(&s.hand("p1")), ["core-056"]);
            assert_eq!(def_ids(&s.pile("p1", "library")), ["core-002"]);
        }

        #[test]
        fn starts_in_the_main_phase_with_the_asked_for_active_player_turn_mana_health_and_armor() {
            let mut s = scenario(
                json!({ "turn": 5, "active": "p2", "p1": { "health": 12, "armor": 2 }, "p2": { "mana": 1 } }),
            );

            assert_eq!(s.state().phase, Phase::Main);
            assert_eq!(s.state().active, PlayerId::P2);
            assert_eq!(s.state().turn, 5);
            // Turn 5 with p2 active: p2 has started ceil(5/2) = 3 turns, p1 floor(5/2) = 2 (§2.3).
            assert_eq!(s.state().players.p2.turns_started, 3);
            assert_eq!(s.state().players.p1.turns_started, 2);
            assert_eq!(s.state().players.p2.mana.max, 3);
            assert_eq!(s.state().players.p1.mana.max, 2);

            // §2.3: `mana` sets current only, so current may sit below (or above) max.
            s.expect_mana("p2", 1)
                .expect_mana("p1", 2)
                .expect_health("p1", 12)
                .expect_health("p2", 30);
            assert_eq!(s.state().players.p1.hero.armor, 2);
        }

        #[test]
        fn defaults_to_a_mid_game_board_with_both_sides_at_max_mana() {
            let mut s = scenario(json!({}));
            assert_eq!(s.state().turn, DEFAULT_TURN);
            assert_eq!(s.state().active, PlayerId::P1);
            s.expect_mana("p1", MAX_MANA).expect_mana("p2", MAX_MANA);
            assert_eq!(s.state().players.p1.mana.max, MAX_MANA);
            assert_eq!(s.state().players.p2.mana.max, MAX_MANA);
            // No mulligan, no opening draw, no start-of-turn: setup deals nothing.
            assert!(s.hand("p1").is_empty());
            assert!(s.events().is_empty());
            assert_eq!(s.state().rng_cursor, 0);
        }

        #[test]
        fn settles_the_board_once_so_a_unit_placed_at_lethal_damage_is_already_dead_s4_5() {
            let s = scenario(json!({ "p1": { "field": [{ "def": "core-043", "damage": 10 }, anchor()] } }));
            assert!(s.unit("p1", 1).is_none());
            assert_eq!(def_ids(&s.pile("p1", "graveyard")), ["core-043"]);
            // The setup's own events are not in the log.
            assert!(s.events().is_empty());
        }

        #[test]
        fn throws_a_named_error_for_an_unknown_card_a_duplicate_lane_a_spell_and_a_wrong_row_card() {
            expect_throw_with(
                || {
                    scenario(json!({ "p1": { "hand": ["core-999"] } }));
                },
                "no catalog card matches \"core-999\"",
            );
            expect_throw_with(
                || {
                    scenario(
                        json!({ "p1": { "field": [{ "def": "core-025", "lane": 2 }, { "def": "core-056", "lane": 2 }] } }),
                    );
                },
                "two cards were given lane 2",
            );
            // §3.2: a Spell is never on the field.
            expect_throw_with(
                || {
                    scenario(json!({ "p1": { "field": ["core-010"] } }));
                },
                "is a Spell; a Spell is never on the field",
            );
            // Only an explicit `row` can contradict the def's type, and then it is an error.
            expect_throw_with(
                || {
                    scenario(json!({ "p1": { "backrow": [{ "def": "core-025", "row": "backrow" }] } }));
                },
                "the backrow holds Field Spells and Traps",
            );
            expect_throw_with(
                || {
                    scenario(json!({ "p1": { "field": [{ "def": "core-041", "row": "units" }] } }));
                },
                "the unit zones hold Units only",
            );
            // A card filed under the wrong list is simply routed by its type, not refused.
            assert_eq!(
                def_id_at(scenario(json!({ "p1": { "backrow": ["core-025"] } })).unit("p1", 1)),
                some("core-025")
            );
            expect_throw_with(
                || {
                    scenario(json!({ "p1": { "field": [{ "def": "core-025", "lane": 9 }] } }));
                },
                "lanes are 1..5",
            );
            expect_throw_with(
                || {
                    scenario(json!({ "p1": { "hand": [{ "def": "core-025", "defId": "core-056" }] } }));
                },
                "they are aliases, so give one",
            );
            expect_throw_with(
                || {
                    scenario(json!({ "p1": { "hand": [{ "radiant": true }] } }));
                },
                "needs `def` or `defId`",
            );
        }

        #[test]
        fn takes_def_id_as_an_alias_of_def_and_radiant_in_any_zone() {
            let s = scenario(json!({
                "p1": {
                    "hand": [{ "defId": "core-025", "radiant": true }],
                    "library": [{ "defId": "core-043", "radiant": true }, "core-002"],
                    "graveyard": [{ "def": "core-005" }],
                    "field": [{ "defId": "core-056", "lane": 2 }],
                },
            }));
            assert_eq!(
                s.hand("p1").first().map(|card| card.def_id.clone()),
                some("core-025")
            );
            assert_eq!(s.hand("p1").first().map(|card| card.radiant), Some(true));
            // #21/#23 want a Radiant card sitting on top of a library.
            assert_eq!(
                s.pile("p1", "library").first().map(|card| card.radiant),
                Some(true)
            );
            assert_eq!(
                s.pile("p1", "library").get(1).map(|card| card.radiant),
                Some(false)
            );
            assert_eq!(
                s.pile("p1", "graveyard").first().map(|card| card.def_id.clone()),
                some("core-005")
            );
            assert_eq!(def_id_at(s.unit("p1", 2)), some("core-056"));
        }

        #[test]
        fn routes_a_field_entry_by_its_defs_type_and_row_overrides_it() {
            // core-006 Mana Well is a Field Spell: listing it under `field` puts it in the backrow.
            let s = scenario(json!({
                "p1": {
                    "field": [{ "defId": "core-006", "row": "backrow", "lane": 1 }, { "defId": "core-025", "lane": 1 }],
                    "backrow": [{ "defId": "core-041", "lane": 2 }],
                },
            }));
            assert_eq!(def_id_at(s.backrow("p1", 1)), some("core-006"));
            assert_eq!(def_id_at(s.backrow("p1", 2)), some("core-041"));
            assert_eq!(def_id_at(s.unit("p1", 1)), some("core-025"));

            // Same board with the row left to the def's type.
            let implied = scenario(json!({ "p1": { "field": ["core-006", "core-025"] } }));
            assert_eq!(def_id_at(implied.backrow("p1", 1)), some("core-006"));
            assert_eq!(def_id_at(implied.unit("p1", 1)), some("core-025"));
        }

        #[test]
        fn s3_2_stack_true_buries_the_lanes_card_with_an_explicit_lane_or_the_entry_before_it() {
            let mut s = scenario(json!({
                "p1": {
                    "field": [
                        "core-043", // lane 1: the leftmost free zone
                        { "def": "core-025", "stack": true }, // no lane of its own: onto the entry before it
                        { "def": "core-056", "lane": 4 },
                        { "def": "core-025", "stack": true, "lane": 4 }, // an explicit lane: onto lane 4's card
                    ],
                },
            }));

            // Two piles, not four zones: a stacked entry takes no lane of its own.
            assert_eq!(def_ids(&pile_at(&s, PlayerId::P1, 1)), ["core-025", "core-043"]);
            assert_eq!(def_ids(&pile_at(&s, PlayerId::P1, 4)), ["core-025", "core-056"]);
            assert!(s.unit("p1", 2).is_none());
            assert!(s.unit("p1", 3).is_none());
            assert!(s.unit("p1", 5).is_none());

            // `unit()` answers with the card on top, which is the one that acts (§3.2).
            assert_eq!(def_id_at(s.unit("p1", 1)), some("core-025"));
            assert_eq!(def_id_at(s.unit("p1", 4)), some("core-025"));
            // R13: the buried card is still on the field, it is just not the one acting.
            let buried = pile_at(&s, PlayerId::P1, 1)[1].clone();
            s.expect_in_zone(buried, "field");
        }

        #[test]
        fn s3_2_a_three_deep_pile_reads_top_first_so_the_list_reads_bottom_first() {
            let s = scenario(json!({
                "p1": {
                    "field": [
                        "core-043", // written first, so it is at the BOTTOM
                        { "def": "core-025", "stack": true },
                        { "def": "core-056", "stack": true }, // written last, so it is on TOP
                    ],
                },
            }));

            let pile = pile_at(&s, PlayerId::P1, 1);
            assert_eq!(def_ids(&pile), ["core-056", "core-025", "core-043"]);
            assert_eq!(def_id_at(s.unit("p1", 1)), some("core-056"));

            // Instances are created in list order whatever the pile order is, so reading the pile
            // bottom-first gives the ids in the order they were handed out.
            let bottom_first: Vec<u32> = pile
                .iter()
                .rev()
                .map(|card| card.id[1..].parse::<u32>().expect("an instance id is c<n>"))
                .collect();
            let mut ascending = bottom_first.clone();
            ascending.sort();
            assert_eq!(bottom_first, ascending);
        }

        #[test]
        fn s3_2_a_dormant_card_keeps_the_damage_and_position_the_setup_gave_it_r13() {
            let mut s = scenario(json!({
                "p1": {
                    "field": [
                        { "def": "core-043", "position": "DEF", "damage": 4 },
                        { "def": "core-025", "stack": true },
                    ],
                },
            }));

            let buried = pile_at(&s, PlayerId::P1, 1)[1].clone();
            assert_eq!(buried.def_id, "core-043");
            // Dormant is not "gone": the damage and the position are the ones it will resume with (R13).
            assert_eq!(buried.damage, 4);
            assert_eq!(buried.position, Some(Position::Def));
            assert_ne!(s.unit("p1", 1).map(|card| card.id), Some(buried.id.clone()));
            s.expect_in_zone(&buried, "field");
        }

        #[test]
        fn refuses_a_repeated_lane_without_stack_and_a_stack_over_nothing() {
            // The refusal names the fix: the only legal way to repeat a lane is a §3.2 pile.
            expect_throw_with(
                || {
                    scenario(
                        json!({ "p1": { "field": [{ "def": "core-025", "lane": 2 }, { "def": "core-056", "lane": 2 }] } }),
                    );
                },
                "add `stack: true` to entry [1]",
            );

            // `stack: true` on the first entry: there is no card under it, and no lane either.
            expect_throw_with(
                || {
                    scenario(json!({ "p1": { "field": [{ "def": "core-025", "stack": true }] } }));
                },
                "`stack: true` but lane ? holds nothing yet",
            );
            // Same with a lane that names an empty zone: the card it buries goes EARLIER in the list.
            expect_throw_with(
                || {
                    scenario(
                        json!({ "p1": { "field": [{ "def": "core-025", "lane": 1 }, { "def": "core-056", "stack": true, "lane": 3 }] } }),
                    );
                },
                "`stack: true` but lane 3 holds nothing yet",
            );
        }

        #[test]
        fn b5_e21_builds_a_backrow_pile_the_later_entry_on_top_the_one_beneath_dormant_and_still_found_r13() {
            let mut s =
                scenario(json!({ "p1": { "backrow": ["core-084", { "def": "core-006", "stack": true }] } }));
            assert_eq!(def_id_at(s.backrow("p1", 1)), some("core-006"));
            assert!(s.backrow("p1", 2).is_none());
            let beneath = s.card("core-084").clone();
            let under: Vec<String> = crate::zones::beneath_at(
                s.state(),
                ZoneSlot {
                    player: PlayerId::P1,
                    row: Row::Backrow,
                    lane: 1,
                },
            )
            .iter()
            .map(|card| card.id.clone())
            .collect();
            assert_eq!(under, vec![beneath.id.clone()]);
            s.expect_in_zone(&beneath, "field");
        }

        #[test]
        fn seeds_s10_1s_instance_counters_and_leaves_them_empty_when_the_entry_says_nothing() {
            let s = scenario(json!({
                "p1": {
                    "field": [
                        { "def": "core-043", "counters": { "plague": 2 } }, // #91's plague counters
                        { "def": "core-025", "counters": { "grade": 3 }, "lane": 2 }, // #93's grade
                        { "def": "core-056", "lane": 3 },
                    ],
                },
            }));

            let counters =
                |lane: i32| serde_json::to_value(s.unit("p1", lane).map(|card| card.counters)).ok();
            assert_eq!(counters(1), Some(json!({ "plague": 2 })));
            assert_eq!(counters(2), Some(json!({ "grade": 3 })));
            assert_eq!(counters(3), Some(json!({})));
        }

        #[test]
        fn r78_seeds_cost_mod_and_cost_override_in_all_four_off_field_zones() {
            let s = scenario(json!({
                "p1": {
                    "hand": [{ "def": "core-025", "costMod": -1 }],
                    "library": [{ "def": "core-025", "costOverride": 0 }],
                    "graveyard": [{ "def": "core-025", "costMod": 2 }],
                    "exile": [{ "def": "core-025", "costOverride": 1, "costMod": -1 }],
                },
            }));
            let at = |zone: &str| -> CardInstance { s.pile("p1", zone)[0].clone() };
            let cost = |card: &CardInstance| effective_cost(s.state(), card, Default::default());

            // R65/R66 read the cost at resolution through `effective_cost`, printed 4 for core-025.
            assert_eq!(at("hand").cost_mod, -1);
            assert_eq!(cost(&at("hand")), 3);
            assert_eq!(at("library").cost_override, Some(0));
            assert_eq!(cost(&at("library")), 0);
            assert_eq!(at("graveyard").cost_mod, 2);
            assert_eq!(cost(&at("graveyard")), 6);
            // Both layers at once: the override replaces the printed cost and the mod still applies over it.
            assert_eq!((at("exile").cost_mod, at("exile").cost_override), (-1, Some(1)));
            assert_eq!(cost(&at("exile")), 0);

            // An entry that says nothing leaves both alone: costMod 0, no override, printed cost.
            let bare = scenario(json!({ "p1": { "hand": ["core-025"] } }));
            let untouched = bare.hand("p1")[0].clone();
            assert_eq!(untouched.cost_mod, 0);
            assert_eq!(untouched.cost_override, None);
            assert_eq!(effective_cost(bare.state(), &untouched, Default::default()), 4);
        }

        #[test]
        fn r33_writes_face_up_exactly_as_given_so_false_reads_back_false_and_not_undefined() {
            let s = scenario(json!({
                "p1": {
                    "backrow": [
                        { "def": "core-041", "faceUp": false }, // a Trap, explicitly face-down
                        { "def": "core-073", "faceUp": true, "lane": 2 },
                        { "def": "core-041", "lane": 3 }, // nothing said: the flag stays unset
                    ],
                },
            }));

            assert_eq!(s.backrow("p1", 1).and_then(|card| card.face_up), Some(false));
            assert_eq!(s.backrow("p1", 2).and_then(|card| card.face_up), Some(true));
            assert_eq!(s.backrow("p1", 3).and_then(|card| card.face_up), None);
        }

        #[test]
        fn honours_stats_override_s10_4_layer_1_r41() {
            let mut s = scenario(
                json!({ "p1": { "field": [{ "defId": "core-043", "statsOverride": { "attack": 3, "health": 3 } }] } }),
            );
            s.expect_stats("core-043", json!({ "attack": 3, "maxHealth": 3, "health": 3 }));
        }

        #[test]
        fn accepts_a_fixture_declared_as_const() {
            // TS: a fixture `as const` is assignable as it stands. Here: a literal built ahead of the
            // scenario goes in unchanged.
            let fixture = json!({ "hand": ["core-025"], "field": [{ "defId": "core-056", "lane": 5 }] });
            let s = scenario(json!({ "p1": fixture }));
            assert_eq!(s.hand("p1").len(), 1);
            assert_eq!(def_id_at(s.unit("p1", 5)), some("core-056"));
        }
    }

    mod string_references {
        use super::*;

        #[test]
        fn resolves_a_catalog_id_a_s5_index_a_name_and_an_instance_id() {
            let s = scenario(json!({ "p1": { "field": [{ "def": "core-043", "lane": 2 }] } }));
            let inst = s.unit("p1", 2).expect("a unit in lane 2");
            assert_eq!(s.card("core-043").id, inst.id);
            assert_eq!(s.card("43").id, inst.id);
            assert_eq!(s.card("Big Felinor").id, inst.id);
            assert_eq!(s.card("big felinor").id, inst.id);
            assert_eq!(s.card(&inst.id).id, inst.id);
        }

        #[test]
        fn prefers_the_active_players_copy() {
            let s = scenario(
                json!({ "active": "p2", "p1": { "field": ["core-025"] }, "p2": { "field": ["core-025"] } }),
            );
            assert_eq!(s.card("core-025").controller, PlayerId::P2);
        }

        #[test]
        fn throws_a_diagnosable_error_when_nothing_matches() {
            let mut s = scenario(json!({ "p1": { "field": ["core-025"] } }));
            s.expect_refused_with(
                |s| {
                    s.card("core-056");
                    s
                },
                "nothing matching \"core-056\"",
            );
            // The error lists what was there instead.
            s.expect_refused_with(
                |s| {
                    s.card("core-056");
                    s
                },
                "core-025",
            );
            s.expect_refused_with(
                |s| {
                    s.card("not-a-card");
                    s
                },
                "nothing matching \"not-a-card\"",
            );
        }

        #[test]
        fn re_resolves_a_stale_card_instance_by_its_id_across_a_reduce() {
            let mut s = scenario(json!({ "p1": { "hand": ["core-025"], "field": [anchor()] } }));
            let stale = s.hand("p1")[0].clone();
            s.play(&stale, json!({})); // `reduce` returned a new state, so `stale` is a copy of a dead card
            s.expect_in_zone(&stale, "field")
                .expect_stats(&stale, json!({ "attack": 7 }));
            assert_eq!(s.card(&stale).zone.z(), ZoneName::Field);
        }
    }

    mod play {
        use super::*;

        #[test]
        fn spends_mana_emits_card_played_and_summoned_and_puts_the_unit_in_the_leftmost_free_zone_r64() {
            let mut s = scenario(json!({ "p1": { "hand": ["core-025"], "field": [anchor()] } }));
            s.play("core-025", json!({}));
            s.expect_mana("p1", 0)
                .expect_events(json!(["cardPlayed", "summoned"]))
                .expect_in_zone("core-025", "field");
            assert_eq!(def_id_at(s.unit("p1", 1)), some("core-025"));
            assert_eq!(s.state().players.p1.turn_log.cards_played, 1);
        }

        #[test]
        fn takes_a_1_based_lane_through_zone() {
            let mut s = scenario(json!({ "p1": { "hand": ["core-025"], "field": [anchor()] } }));
            s.play("core-025", json!({ "zone": 3 }));
            assert_eq!(def_id_at(s.unit("p1", 3)), some("core-025"));
            assert!(s.unit("p1", 1).is_none());
        }

        #[test]
        fn sends_a_spell_to_the_graveyard_s10_5_step_7() {
            let mut s = scenario(json!({ "p1": { "hand": ["core-010"], "field": [anchor()] } }));
            s.play("core-010", json!({}));
            s.expect_in_zone("core-010", "graveyard")
                .expect_events(json!(["cardPlayed", "enteredGraveyard"]));
            assert!(s.unit("p1", 1).is_none());
        }

        #[test]
        fn throws_the_engines_own_refusal() {
            let mut poor =
                scenario(json!({ "p1": { "hand": ["core-025"], "mana": 3, "field": [anchor()] } }));
            poor.expect_refused_with(|s| s.play("core-025", json!({})), "more than your mana");
            // The refusal leaves the state alone: the card is still in hand and the mana unspent.
            poor.expect_in_zone("core-025", "hand").expect_mana("p1", 3);

            // "Uncastable at 4 mana" (M4-T4): a 6-cost card with MAX_MANA available.
            let mut giga = scenario(json!({ "p1": { "hand": ["core-029"], "field": [anchor()] } }));
            giga.expect_refused_with(|s| s.play("core-029", json!({})), "costs 6, more than your mana");

            let mut full = scenario(json!({
                "p1": { "hand": ["core-025"], "field": ["core-056", "core-056", "core-056", "core-056", "core-056"] },
            }));
            full.expect_refused_with(|s| s.play("core-025", json!({})), "no free units zone");

            let mut taken = scenario(json!({ "p1": { "hand": ["core-025"], "field": [anchor()] } }));
            taken.expect_refused_with(|s| s.play("core-025", json!({ "zone": 5 })), "not open");

            let mut wrong = scenario(json!({ "p1": { "hand": ["core-010"], "field": [anchor()] } }));
            wrong.expect_refused_with(
                |s| s.play("core-010", json!({ "zone": 1 })),
                "a Spell takes no zone",
            );

            let mut absent = scenario(json!({ "p1": { "hand": ["core-025"], "field": [anchor()] } }));
            absent.expect_refused_with(
                |s| s.play("core-056", json!({})),
                "nothing matching \"core-056\" is in a hand",
            );
        }

        #[test]
        fn refuses_a_play_on_the_other_players_turn() {
            let mut s = scenario(
                json!({ "p1": { "field": [anchor()] }, "p2": { "hand": ["core-025"], "field": [anchor()] } }),
            );
            s.expect_refused_with(|s| s.play("core-025", json!({})), "not your turn");
        }
    }

    mod attack {
        use super::*;

        #[test]
        fn hits_the_enemy_hero_through_hero_player() {
            // R82: with an empty hand and its one unit exerted, p1 would have nothing but `endTurn` left
            // and the turn would auto-end under the assertion — taking the opponent's turn with it and
            // charging §2.4 fatigue to both empty libraries. The hand anchor keeps the turn open.
            let mut s = scenario(
                json!({ "p1": { "field": ["core-056"], "hand": [HAND_ANCHOR] }, "p2": { "health": 20 } }),
            );
            s.attack("core-056", "hero");
            // Jilliax is 3/2 with Lifesteal, so §4.4 step 8 heals p1 by the 3 it dealt.
            s.expect_health("p2", 17)
                .expect_health("p1", 33)
                .expect_events(json!(["attackDeclared", "damage", "healed"]));
        }

        #[test]
        fn resolves_a_unit_exchange_and_spends_the_exertion_s4_1_s4_3() {
            // R82 again: the second attack must be refused for having ALREADY ACTED, which it can only be
            // while it is still p1's turn (see the hero test above).
            let mut s = scenario(
                json!({ "p1": { "field": ["core-025"], "hand": [HAND_ANCHOR] }, "p2": { "field": ["core-056"] } }),
            );
            let target = s.unit("p2", 1).expect("p2's unit");
            s.attack("core-025", target);
            // Divine Shield eats the 7 (§4.4 step 1); the strike-back of 3 is stopped by Armor 7 (step 2).
            s.expect_events(json!(["attackDeclared", "divineShieldLost"]))
                .expect_stats("core-025", json!({ "health": 7 }));
            let target = s.unit("p2", 1).expect("p2's unit");
            s.expect_stats(&target, json!({ "health": 2 }));
            s.expect_refused_with(|s| s.attack("core-025", target), "already acted");
        }

        #[test]
        fn enforces_taunt_s4_2_step_3() {
            let mut s = scenario(json!({ "p1": { "field": ["core-025"] }, "p2": { "field": ["core-056"] } }));
            s.expect_refused_with(|s| s.attack("core-025", "hero"), "Taunt");
        }
    }

    mod start_turn_and_end_turn {
        use super::*;

        #[test]
        fn start_turn_re_starts_the_active_players_turn_turn_plus_1_mana_refresh_triggers_one_draw() {
            let mut s =
                scenario(json!({ "p1": { "field": [anchor()], "library": ["core-002"], "mana": 1 } }));
            s.start_turn();

            assert_eq!(s.state().active, PlayerId::P1);
            assert_eq!(s.state().turn, DEFAULT_TURN + 1);
            assert_eq!(s.state().phase, Phase::Main);
            assert_eq!(s.state().players.p1.turns_started, 6);
            s.expect_mana("p1", MAX_MANA);
            assert_eq!(def_ids(&s.hand("p1")), ["core-002"]);
            s.expect_events(json!(["turnStarted", "manaChanged", "drawn", "addedToHand"]));
        }

        #[test]
        fn end_turn_hands_the_turn_to_the_opponent_and_two_end_turns_come_back_around() {
            let mut s = scenario(json!({
                "p1": { "field": [anchor()], "hand": ["core-010"] },
                "p2": { "field": [anchor()], "hand": ["core-010"] },
            }));
            s.end_turn();
            assert_eq!(s.state().active, PlayerId::P2);
            assert_eq!(s.state().turn, DEFAULT_TURN + 1);
            s.expect_events(json!(["turnEnded", "turnStarted"]));

            s.end_turn();
            assert_eq!(s.state().active, PlayerId::P1);
            assert_eq!(s.state().turn, DEFAULT_TURN + 2);
        }

        #[test]
        fn exertion_resets_at_the_controllers_own_turn_start_s4_1() {
            // R82: without the hand anchor the first attack auto-ends the turn, and the refusal below comes
            // back as "it is not your turn" instead of the exertion message this test is about.
            let mut s = scenario(
                json!({ "p1": { "field": ["core-056"], "hand": [HAND_ANCHOR] }, "p2": { "health": 20 } }),
            );
            s.attack("core-056", "hero");
            s.expect_refused_with(|s| s.attack("core-056", "hero"), "already acted");
            s.start_turn();
            s.attack("core-056", "hero");
            s.expect_health("p2", 14);
        }
    }

    mod answer {
        use super::*;

        #[test]
        fn throws_when_no_prompt_is_open_naming_the_phase_and_the_turn() {
            let mut s = scenario(json!({ "p1": { "field": [anchor()] } }));
            s.expect_refused_with(|s| s.answer(json!("anything")), "no prompt is open");
            s.expect_refused_with(|s| s.answer(json!([{ "pick": "none" }])), "no prompt is open");
        }

        // A real prompt chain (`s.play(x, …).answer(json!("a")).answer(json!("b"))`) needs a card
        // script that opens a prompt, which is a card file's test. `answer` reads `state.pending` fresh
        // on every call, so a chain works as soon as a prompt is open.
    }

    mod assertion_helpers_fail_loudly {
        use super::*;

        #[test]
        fn expect_in_zone() {
            let mut s = scenario(json!({ "p1": { "hand": ["core-025"], "field": [anchor()] } }));
            s.expect_in_zone("core-025", "hand");
            s.expect_refused_with(
                |s| s.expect_in_zone("core-025", "graveyard"),
                "should be in graveyard but is in hand",
            );
        }

        #[test]
        fn expect_in_zone_gone_for_a_card_that_ceased_to_exist_r11() {
            // A unit token in a hand is legal (#75, R11); playing it and letting it leave the field is what
            // makes it vanish, which needs card scripts. Here: a live card is not "gone".
            let mut s = scenario(json!({ "p1": { "field": ["core-025", anchor()] } }));
            let unit = s.unit("p1", 1).expect("a unit in lane 1");
            s.expect_refused_with(
                |s| s.expect_in_zone(unit, "gone"),
                "should be in gone but is in field",
            );
        }

        #[test]
        fn expect_stats() {
            let mut s = scenario(json!({ "p1": { "field": ["core-025"] } }));
            s.expect_stats("core-025", json!({ "attack": 7, "health": 7, "maxHealth": 7 }));
            s.expect_refused_with(
                |s| s.expect_stats("core-025", json!({ "attack": 6 })),
                "attack should be 6 but is 7",
            );
        }

        #[test]
        fn expect_events() {
            let mut s = scenario(json!({ "p1": { "hand": ["core-025"], "field": [anchor()] } }));
            s.play("core-025", json!({}));
            s.expect_events(json!(["cardPlayed", "summoned"]));
            // A subsequence, not contiguous: manaChanged sits between them in the log.
            s.expect_events(json!(["manaChanged", "summoned"]));
            s.expect_refused_with(
                |s| s.expect_events(json!(["summoned", "cardPlayed"])),
                "not a subsequence",
            );
            s.expect_refused_with(|s| s.expect_events(json!(["gameOver"])), "not a subsequence");
        }

        #[test]
        fn expect_health_and_expect_mana() {
            let mut s = scenario(json!({ "p1": { "health": 11, "mana": 2 } }));
            s.expect_health("p1", 11).expect_mana("p1", 2);
            s.expect_refused_with(|s| s.expect_health("p1", 30), "should be at 30 but is at 11");
            s.expect_refused_with(|s| s.expect_mana("p1", 4), "should have 4 mana but has 2");
        }

        #[test]
        fn unit_backrow_hand_and_pile() {
            let s = scenario(
                json!({ "p1": { "hand": ["core-010"], "field": [{ "def": "core-025", "lane": 2 }], "backrow": ["core-041"] } }),
            );
            assert_eq!(def_id_at(s.unit("p1", 2)), some("core-025"));
            assert!(s.unit("p1", 1).is_none());
            assert!(s.unit("p2", 2).is_none());
            assert_eq!(def_id_at(s.backrow("p1", 1)), some("core-041"));
            assert_eq!(def_ids(&s.hand("p1")), ["core-010"]);
            assert!(s.hand("p2").is_empty());
            assert_eq!(def_ids(&s.pile("p1", "hand")), ["core-010"]);
            expect_throw_with(
                || {
                    s.unit("p1", 6);
                },
                "lanes are 1..5",
            );
            expect_throw_with(
                || {
                    s.backrow("p1", 0);
                },
                "lanes are 1..5",
            );
        }

        #[test]
        fn every_step_returns_the_same_mutable_scenario_and_state_and_events_follow_it() {
            let mut s = scenario(json!({ "p1": { "hand": ["core-025"], "field": [anchor()] } }));
            let before = s.state().clone();
            let at: *const Scenario = &s;
            let played: *const Scenario = s.play("core-025", json!({}));
            assert!(std::ptr::eq(played, at));
            let checked: *const Scenario = s.expect_mana("p1", 0);
            assert!(std::ptr::eq(checked, at));
            // `reduce` returned a new state, and the scenario tracks it.
            assert_ne!(s.state(), &before);
            assert!(!s.last_events().is_empty());
            assert_eq!(s.events(), s.last_events()); // one step so far
            assert!(s.hand("p1").is_empty());
            assert!(std::hint::black_box(HAND_CAP) > 0);
        }
    }

    mod determinism {
        use super::*;

        fn steps(seed: &str) -> Scenario {
            let mut s = scenario(json!({
                "seed": seed,
                "p1": { "field": [{ "def": "core-056", "lane": 5 }], "hand": ["core-025"], "library": ["core-002"] },
                "p2": { "field": ["core-056"], "hand": ["core-010"] },
            }));
            let attacker = s.unit("p1", 5).expect("p1's lane 5");
            let defender = s.unit("p2", 1).expect("p2's lane 1");
            s.play("core-025", json!({}))
                .attack(attacker, defender)
                .start_turn();
            s
        }

        fn text<T: serde::Serialize + ?Sized>(value: &T) -> String {
            serde_json::to_string(value).expect("serialises")
        }

        #[test]
        fn the_same_seed_and_the_same_steps_give_identical_event_logs_and_identical_rng_cursor() {
            let a = steps("seed-alpha");
            let b = steps("seed-alpha");
            assert_eq!(text(a.events()), text(b.events()));
            assert_eq!(a.state().rng_cursor, b.state().rng_cursor);
            assert_eq!(text(a.state()), text(b.state()));
        }

        #[test]
        fn the_rng_cursor_is_threaded_and_nothing_reachable_consumes_rng_yet() {
            // Nothing in these steps draws from the rng: setup does not shuffle, `draw` and `combat` never
            // call it, and no card script runs here to. So the cursor is still 0 and the threading
            // (`state.rng_cursor = rng.cursor()` after a direct call, `reduce`'s own write after an
            // action) is a no-op — which is exactly why a cross-seed DIFFERENCE cannot be shown here.
            let a = steps("seed-alpha");
            let b = steps("seed-beta");
            assert_eq!(a.state().rng_cursor, 0);
            assert_eq!(b.state().rng_cursor, 0);
            assert_eq!(text(a.events()), text(b.events()));
        }
    }

    mod view {
        use super::*;

        #[test]
        fn reports_the_viewer_the_turn_and_the_phase() {
            let s = scenario(json!({ "turn": 5, "active": "p2", "p1": { "field": ["core-025"] } }));
            let view = serde_json::to_value(s.view("p1")).expect("a view serialises");
            assert_eq!(view["viewer"], "p1");
            assert_eq!(view["you"]["player"], "p1");
            assert_eq!(view["opponent"]["player"], "p2");
            assert_eq!(view["turn"], 5);
            assert_eq!(view["active"], "p2");
            assert_eq!(view["phase"], "main");
            assert_eq!(s.view(()).viewer, PlayerId::P2); // defaults to the active player
        }

        #[test]
        fn never_leaks_the_other_sides_hand_s10_8() {
            let s = scenario(
                json!({ "p1": { "hand": ["core-025"] }, "p2": { "hand": ["core-043", "core-010"] } }),
            );
            let view = serde_json::to_value(s.view("p1")).expect("a view serialises");
            assert!(view["you"]["hand"].is_array());
            assert!(!view["opponent"]["hand"].is_array());
            assert!(!view["opponent"]["hand"].to_string().contains("core-043"));
            assert!(!view["opponent"].to_string().contains("core-010"));
            assert_eq!(view["opponent"]["hand"], json!({ "count": 2 }));
        }
    }
}
