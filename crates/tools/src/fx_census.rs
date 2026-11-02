//! `cargo jackioh fx census [--seeds N] [--format md|json] [--out <path>]`: what each card's
//! plays and hooks do on the board, from recorded games, with a suggested motif.
//!
//! It plays `--seeds` random-policy base games over every shipped set (handicap and duos off),
//! then forced games carrying each rarely seen card, so every deckable non-token card is played
//! `FX_CENSUS_MIN_SEEN` times or is listed as unseen with a reason. For each card and hook it
//! reports, from the event windows, the event counts, the targets, the sides and the lanes, the
//! random splits, the graveyard returns, the own-unit costs and the `via` keywords, plus a motif
//! suggested through the motif table's "Reads" column (`docs/polish/9-card-animations.md`).
//!
//! A play's window runs from its `cardPlayed` to its `cardResolved` or `countered`. A hook's
//! window runs from its `hookResolving` marker to the next marker or `cardResolved`; a Trap's
//! firing opens a `fire` window, since Traps carry no marker. A prompt keeps a window open:
//! windows close only on their closing event or on `settle`, which runs once the state holds no
//! paused work. The tool reads the raw stream offline and prints only aggregates, so nothing
//! pure changes (CLAUDE.md rule 4); every number it plays by is a named constant (rule 9).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::mem;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;

use anyhow::{Context, Result};
use jackioh_cards::{CATALOG, register_all};
use jackioh_engine::{
    Action, ActionBody, CreateGameArgs, GameState, PlayerId, Rng, UNIT_ZONES, Zone, begin_game, create_game,
    deckable, find_instance, reduce, set_ships, subsystems::choose_action,
};
use rayon::prelude::*;
use serde::Serialize;
use serde_json::Value;

use crate::agent::panic_message;
use crate::fuzz::{SHIPPED_FUZZ_POOL, actor_of, decks_for_seed_from, state_paused};
use crate::golden::force_into;

/// Base random-policy games the census plays before forcing (`--seeds` overrides it).
pub(crate) const FX_CENSUS_SEEDS: u32 = 200;
/// Plays below which a deckable non-token card gets forced games, else an `unseen` reason.
pub(crate) const FX_CENSUS_MIN_SEEN: u32 = 3;
/// Forced games per card at most, stopping early once it reaches `FX_CENSUS_MIN_SEEN` plays.
pub(crate) const FX_CENSUS_MAX_FORCED_GAMES: u32 = 20;
/// Actions one census game may take before the loop stops recording it.
const FX_CENSUS_STEP_CAP: usize = 3000;
/// Stream head every census game seed, policy seed and nonce hangs off, so `--seeds` replays.
const FX_CENSUS_STREAM: &str = "jackioh-fx-census";
/// Head of a hero's target id (`hero-p1`), which sits in no zone.
const HERO_ID_PREFIX: &str = "hero-";
/// A played card whose windows show nothing suggests this.
const VANILLA: &str = "none: vanilla";
/// A card with no window at all suggests this.
const UNSEEN: &str = "none: unseen";
/// Event types no window counts: the play's own frame and the mana bookkeeping around it.
const IGNORED_EVENTS: &[&str] = &[
    "cardPlayed",
    "cardResolved",
    "cardAnnounced",
    "hookResolving",
    "trapFired",
    "activated",
    "manaSpent",
    "enteredGraveyard",
];
/// Hook order for rendering and for picking a card's motif; unknown hooks sort last.
const HOOK_ORDER: &[&str] = &[
    "cry",
    "play",
    "fire",
    "activate",
    "death",
    "whenever",
    "startOfTurn",
    "endOfTurn",
];

/// `fx census [--seeds N] [--format md|json] [--out <path>]`.
#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    command: FxCommand,
}

#[derive(clap::Subcommand)]
enum FxCommand {
    /// What each card's plays and hooks do on the board, from seeded games, with a motif.
    Census(CensusArgs),
}

#[derive(clap::Args)]
struct CensusArgs {
    /// How many seeded base games to play before the forced games.
    #[arg(long, default_value_t = FX_CENSUS_SEEDS)]
    seeds: u32,
    /// The report's shape: a table for the set passes, or JSON a test can read.
    #[arg(long, value_enum, default_value_t = Format::Md)]
    format: Format,
    /// The file to write; without it, the report goes to stdout.
    #[arg(long, value_name = "PATH")]
    out: Option<PathBuf>,
}

/// The report's shape (`--format`).
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum Format {
    /// One row per card, grouped by set.
    Md,
    /// One JSON object a test can read.
    Json,
}

// Places and windows

/// Where an event's card stood: its board seat and lane, and whether it came from a graveyard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Place {
    board: Option<(PlayerId, Option<i32>)>,
    from_graveyard: bool,
}

/// `id`'s place across one step: a hero by its id, else its field zone in `before` or `after`,
/// with `from_graveyard` when `before` held it in a graveyard.
fn place_in(before: &GameState, after: &GameState, id: &str) -> Place {
    if let Some(seat) = id.strip_prefix(HERO_ID_PREFIX)
        && let Ok(player) = seat.parse::<PlayerId>()
    {
        return Place {
            board: Some((player, None)),
            from_graveyard: false,
        };
    }
    let zone_of = |state: &GameState| find_instance(state, id).map(|found| found.zone.clone());
    let zone = zone_of(before).or_else(|| zone_of(after));
    Place {
        board: match zone {
            Some(Zone::Field { player, lane, .. }) => Some((player, Some(lane))),
            _ => None,
        },
        from_graveyard: matches!(zone_of(before), Some(Zone::Graveyard { .. })),
    }
}

/// One open window's facts, before they fold into a tally.
#[derive(Default)]
struct WindowFacts {
    events: BTreeMap<String, u32>,
    targets: BTreeMap<String, (PlayerId, Option<i32>)>,
    hits: BTreeMap<(String, String), u32>,
    graveyard_return: bool,
    own_cost: u32,
    via: BTreeMap<String, u32>,
}

/// The string field `key` of `event`, or empty when it is missing or no string.
fn text_of(event: &Value, key: &str) -> String {
    event
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// The seat `event` names in `key`, or p1 when it names none (the stream always names one).
fn seat_of(event: &Value, key: &str) -> PlayerId {
    event
        .get(key)
        .and_then(Value::as_str)
        .and_then(|seat| seat.parse::<PlayerId>().ok())
        .unwrap_or(PlayerId::P1)
}

impl WindowFacts {
    /// Fold one event of `source_id`'s window in: nothing about the card itself, only what it
    /// did to the board. `place` reads the step's states, so targets keep their seat and lane.
    fn add(&mut self, event: &Value, source_id: &str, place: &dyn Fn(&str) -> Place) {
        let kind = text_of(event, "type");
        if IGNORED_EVENTS.contains(&kind.as_str()) {
            return;
        }
        let mut ids: Vec<&str> = Vec::new();
        for key in ["targetId", "instanceId"] {
            if let Some(id) = event.get(key).and_then(Value::as_str) {
                ids.push(id);
            }
        }
        if !ids.is_empty() && ids.iter().all(|id| *id == source_id) {
            return;
        }
        *self.events.entry(kind.clone()).or_default() += 1;
        if let Some(keyword) = event.get("via").and_then(Value::as_str) {
            *self.via.entry(keyword.to_string()).or_default() += 1;
        }
        for id in ids {
            if id == source_id {
                continue;
            }
            let found = place(id);
            if let Some(board) = found.board {
                self.targets.entry(id.to_string()).or_insert(board);
            }
            if kind == "summoned" && found.from_graveyard {
                self.graveyard_return = true;
            }
        }
        if kind == "damage" && event.get("combat").and_then(Value::as_bool) != Some(true) {
            let source = event.get("sourceId").and_then(Value::as_str);
            let target = event.get("targetId").and_then(Value::as_str);
            if let (Some(source), Some(target)) = (source, target) {
                *self
                    .hits
                    .entry((source.to_string(), target.to_string()))
                    .or_default() += 1;
            }
        }
    }
}

/// One open window: the definition and hook it folds into, the card resolving it, and its facts.
struct Open {
    def_id: String,
    hook: String,
    source_id: String,
    controller: PlayerId,
    facts: WindowFacts,
}

/// Tallies by definition id, then hook.
pub(crate) type Tallies = BTreeMap<String, BTreeMap<String, HookTally>>;

/// The open plays and hook of one game, folding windows into tallies as they close.
#[derive(Default)]
pub(crate) struct Tracker {
    plays: Vec<Open>,
    hook: Option<Open>,
    tallies: Tallies,
}

impl Tracker {
    /// Fold one step's events in, carrying the action's `tributes` as the windows' cost.
    fn observe(&mut self, events: &[Value], tributes: u32, place: &dyn Fn(&str) -> Place) {
        let mut cost = tributes;
        for event in events {
            let kind = text_of(event, "type");
            match kind.as_str() {
                "cardPlayed" => self.plays.push(Open {
                    def_id: text_of(event, "defId"),
                    hook: "play".to_string(),
                    source_id: text_of(event, "instanceId"),
                    controller: seat_of(event, "player"),
                    facts: WindowFacts {
                        own_cost: mem::take(&mut cost),
                        ..WindowFacts::default()
                    },
                }),
                "hookResolving" => {
                    self.close_hook();
                    if !text_of(event, "defId").is_empty() {
                        let hook = text_of(event, "hook");
                        let mut facts = WindowFacts::default();
                        if hook == "activate" {
                            facts.own_cost = mem::take(&mut cost);
                        }
                        self.hook = Some(Open {
                            def_id: text_of(event, "defId"),
                            hook,
                            source_id: text_of(event, "sourceId"),
                            controller: seat_of(event, "controller"),
                            facts,
                        });
                    }
                }
                // A Trap's firing carries no marker, so it opens its own window.
                "trapFired" => {
                    self.close_hook();
                    self.hook = Some(Open {
                        def_id: text_of(event, "defId"),
                        hook: "fire".to_string(),
                        source_id: text_of(event, "instanceId"),
                        controller: seat_of(event, "controller"),
                        facts: WindowFacts::default(),
                    });
                }
                "cardResolved" | "countered" => {
                    if kind == "countered"
                        && let Some(hook) = self.hook.as_mut()
                    {
                        let source = hook.source_id.clone();
                        hook.facts.add(event, &source, place);
                    }
                    self.close_hook();
                    let instance = text_of(event, "instanceId");
                    if let Some(at) = self.plays.iter().rposition(|open| open.source_id == instance) {
                        while self.plays.len() > at {
                            self.close_play();
                        }
                    }
                }
                _ => {
                    if let Some(hook) = self.hook.as_mut() {
                        let source = hook.source_id.clone();
                        hook.facts.add(event, &source, place);
                    }
                    let hook_source = self.hook.as_ref().map(|hook| hook.source_id.clone());
                    if let Some(play) = self.plays.last_mut()
                        && (hook_source.is_none() || hook_source.as_deref() == Some(play.source_id.as_str()))
                    {
                        let source = play.source_id.clone();
                        play.facts.add(event, &source, place);
                    }
                }
            }
        }
    }

    /// Close the open hook, if any, folding its window into the tallies.
    fn close_hook(&mut self) {
        if let Some(open) = self.hook.take() {
            self.tallies
                .entry(open.def_id)
                .or_default()
                .entry(open.hook)
                .or_default()
                .add_window(&open.facts, open.controller);
        }
    }

    /// Close the innermost open play, folding its window into the tallies.
    fn close_play(&mut self) {
        if let Some(open) = self.plays.pop() {
            self.tallies
                .entry(open.def_id)
                .or_default()
                .entry(open.hook)
                .or_default()
                .add_window(&open.facts, open.controller);
        }
    }

    /// Close the hook and every play: the game settled, so no window stays open.
    fn settle(&mut self) {
        self.close_hook();
        while !self.plays.is_empty() {
            self.close_play();
        }
    }
}

// Tallies

/// One card and hook's windows folded together, as the report prints them.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HookTally {
    windows: u32,
    events: BTreeMap<String, u32>,
    targets: BTreeMap<u32, u32>,
    sides: BTreeMap<String, u32>,
    lanes: BTreeMap<String, u32>,
    families: BTreeMap<String, u32>,
    random_split: u32,
    graveyard_return: u32,
    own_cost: u32,
    via: BTreeMap<String, u32>,
    motif: String,
}

/// Add `count` to `hist`'s `key`.
fn bump(hist: &mut BTreeMap<String, u32>, key: &str, count: u32) {
    *hist.entry(key.to_string()).or_default() += count;
}

impl HookTally {
    /// Fold one closed window in, counting the window once per shape it holds.
    fn add_window(&mut self, facts: &WindowFacts, controller: PlayerId) {
        self.windows += 1;
        for (kind, count) in &facts.events {
            bump(&mut self.events, kind, *count);
        }
        for (keyword, count) in &facts.via {
            bump(&mut self.via, keyword, *count);
        }
        *self
            .targets
            .entry(u32::try_from(facts.targets.len()).unwrap_or(u32::MAX))
            .or_default() += 1;
        bump(&mut self.sides, sides_of(&facts.targets, controller), 1);
        bump(&mut self.lanes, lanes_of(&facts.targets), 1);
        if facts.hits.values().any(|hits| *hits >= 2) {
            self.random_split += 1;
        }
        if facts.graveyard_return {
            self.graveyard_return += 1;
        }
        if facts.own_cost > 0 {
            self.own_cost += 1;
        }
        let mut held: BTreeSet<String> = facts
            .events
            .keys()
            .filter_map(|kind| family_of(kind).map(str::to_string))
            .collect();
        if facts.graveyard_return {
            held.insert("return".to_string());
        }
        if facts.own_cost > 0 {
            held.insert("cost".to_string());
        }
        for family in held {
            bump(&mut self.families, &family, 1);
        }
    }

    /// Fold another tally of the same card and hook in; the motif stays the row's own.
    fn merge(&mut self, other: HookTally) {
        self.windows += other.windows;
        for (kind, count) in other.events {
            bump(&mut self.events, &kind, count);
        }
        for (count, windows) in other.targets {
            *self.targets.entry(count).or_default() += windows;
        }
        for (side, windows) in other.sides {
            bump(&mut self.sides, &side, windows);
        }
        for (lanes, windows) in other.lanes {
            bump(&mut self.lanes, &lanes, windows);
        }
        for (family, windows) in other.families {
            bump(&mut self.families, &family, windows);
        }
        self.random_split += other.random_split;
        self.graveyard_return += other.graveyard_return;
        self.own_cost += other.own_cost;
        for (keyword, count) in other.via {
            bump(&mut self.via, &keyword, count);
        }
    }
}

/// Fold `from` into `into`, card by card and hook by hook.
fn merge_tallies(into: &mut Tallies, from: Tallies) {
    for (def, hooks) in from {
        for (hook, tally) in hooks {
            into.entry(def.clone())
                .or_default()
                .entry(hook)
                .or_default()
                .merge(tally);
        }
    }
}

/// Which seats `targets` touches from `controller`'s side: both, own, enemy or none.
fn sides_of(targets: &BTreeMap<String, (PlayerId, Option<i32>)>, controller: PlayerId) -> &'static str {
    let mut own = false;
    let mut enemy = false;
    for (seat, _) in targets.values() {
        if *seat == controller {
            own = true;
        } else {
            enemy = true;
        }
    }
    match (own, enemy) {
        (true, true) => "both",
        (true, false) => "own",
        (false, true) => "enemy",
        (false, false) => "none",
    }
}

/// Which lanes `targets` touches: none, one, all, adjacent or spread.
fn lanes_of(targets: &BTreeMap<String, (PlayerId, Option<i32>)>) -> &'static str {
    let lanes: BTreeSet<i32> = targets.values().filter_map(|(_, lane)| *lane).collect();
    match lanes.len() {
        0 => "none",
        1 => "one",
        _ => {
            let first = lanes.iter().next().unwrap_or(&0);
            let last = lanes.iter().next_back().unwrap_or(&0);
            if i32::try_from(lanes.len()).unwrap_or(i32::MAX) >= UNIT_ZONES {
                "all"
            } else if last - first + 1 == i32::try_from(lanes.len()).unwrap_or(-1) {
                "adjacent"
            } else {
                "spread"
            }
        }
    }
}

// The motif table: docs/polish/9-card-animations.md, "The motifs" (its "Reads" column)

/// The event types each motif family reads.
const FAMILY_EVENTS: &[(&str, &str)] = &[
    ("damage", "damage"),
    ("destroyed", "damage"),
    ("exiled", "banish"),
    ("bounced", "banish"),
    ("countered", "counter"),
];

/// The family an event type reads as, if any.
fn family_of(kind: &str) -> Option<&'static str> {
    FAMILY_EVENTS
        .iter()
        .find(|(event, _)| *event == kind)
        .map(|(_, family)| *family)
}

/// The motif each family suggests, in tie-break order: the earlier family wins a tied tally.
/// `rise` (a unit back from a graveyard) and `offering` (own units paid as a cost) are the set
/// passes' names, not yet in the doc's table.
const FAMILY_MOTIFS: &[(&str, &str)] = &[
    ("cost", "offering"),
    ("return", "rise"),
    ("counter", "negate"),
    ("banish", "vanish"),
    ("damage", "bolt"),
];

/// The key with the highest count; the first key wins a tie.
fn commonest<K: Ord + Clone>(hist: &BTreeMap<K, u32>) -> Option<K> {
    let mut best: Option<(K, u32)> = None;
    for (key, count) in hist {
        if best.as_ref().is_none_or(|(_, held)| *count > *held) {
            best = Some((key.clone(), *count));
        }
    }
    best.map(|(key, _)| key)
}

/// The damage family's motif: how the hits landed picks the shape.
fn damage_motif(tally: &HookTally) -> &'static str {
    let targets = commonest(&tally.targets).unwrap_or(0);
    if tally.random_split > 0 && 2 * tally.random_split >= tally.windows {
        "barrage"
    } else if commonest(&tally.lanes).as_deref() == Some("one") && targets >= 2 {
        "lance"
    } else if !tally.events.contains_key("damage") {
        if targets >= 2 { "wipe" } else { "bolt" }
    } else if commonest(&tally.sides).as_deref() == Some("both") && targets >= 2 {
        "nova"
    } else if targets >= 2 {
        "volley"
    } else {
        "bolt"
    }
}

/// The tally's suggested motif: its most common family, or why no family reads it.
pub(crate) fn suggest(tally: &HookTally) -> String {
    let mut best: Option<(&str, u32)> = None;
    for (family, _) in FAMILY_MOTIFS {
        let windows = tally.families.get(*family).copied().unwrap_or(0);
        if windows > 0 && best.is_none_or(|(_, held)| windows > held) {
            best = Some((family, windows));
        }
    }
    match best {
        Some(("damage", _)) => damage_motif(tally).to_string(),
        Some((family, _)) => FAMILY_MOTIFS
            .iter()
            .find(|(held, _)| *held == family)
            .map(|(_, motif)| (*motif).to_string())
            .unwrap_or_else(|| VANILLA.to_string()),
        None => match commonest(&tally.events) {
            Some(kind) => format!("none: no motif reads {kind}"),
            None => VANILLA.to_string(),
        },
    }
}

/// The card's motif: its first hook in `HOOK_ORDER` with a real suggestion.
fn card_motif(hooks: &BTreeMap<String, HookTally>) -> String {
    if hooks.is_empty() {
        return UNSEEN.to_string();
    }
    let mut ordered: Vec<&String> = hooks.keys().collect();
    ordered.sort_by_key(|hook| {
        (
            HOOK_ORDER
                .iter()
                .position(|held| held == hook)
                .unwrap_or(HOOK_ORDER.len()),
            (*hook).clone(),
        )
    });
    if let Some(hook) = ordered
        .iter()
        .map(|hook| hooks[*hook].motif.clone())
        .find(|motif| !motif.starts_with("none:"))
    {
        return hook;
    }
    if hooks.values().all(|tally| tally.motif == VANILLA) {
        VANILLA.to_string()
    } else {
        hooks[ordered[0]].motif.clone()
    }
}

// Games

/// One census game: its stream name and its two decks.
struct GameSpec {
    name: String,
    decks: (Vec<String>, Vec<String>),
}

/// Base game `k`: seed `k`'s deal from every shipped set.
fn base_spec(k: u32) -> GameSpec {
    GameSpec {
        name: format!("{FX_CENSUS_STREAM}-{k}"),
        decks: decks_for_seed_from(k, &SHIPPED_FUZZ_POOL),
    }
}

/// Forced game `a` of card `id`: seed `a`'s deal with the card in both decks.
fn forced_spec(id: &str, a: u32) -> GameSpec {
    let (mut p1, mut p2) = decks_for_seed_from(a, &SHIPPED_FUZZ_POOL);
    let forced = [id.to_string()];
    p1 = force_into(&p1, &forced);
    p2 = force_into(&p2, &forced);
    GameSpec {
        name: format!("{FX_CENSUS_STREAM}-{id}-{a}"),
        decks: (p1, p2),
    }
}

/// The tributes an action pays: its Tribute cost's units, else none.
fn tributes_of(body: &ActionBody) -> u32 {
    let count = match body {
        ActionBody::Play { tributes, .. } | ActionBody::Activate { tributes, .. } => {
            tributes.as_ref().map_or(0, Vec::len)
        }
        _ => 0,
    };
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// The step's events as values, so the tracker reads them like the tests' fixtures.
fn values_of(events: &[jackioh_engine::GameEvent]) -> Result<Vec<Value>> {
    let array = serde_json::to_value(events)?;
    array
        .as_array()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("the step's events are no array"))
}

/// Play `spec`'s game with the random policy, folding every step's windows into tallies.
/// A refusal, a stall or a panic is an `Err` naming the game, never a crash.
fn play_game(spec: &GameSpec) -> Result<Tallies, String> {
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| play_game_inner(spec)));
    match outcome {
        Ok(inner) => inner,
        Err(payload) => Err(format!("{}: panic: {}", spec.name, panic_message(&*payload))),
    }
}

fn play_game_inner(spec: &GameSpec) -> Result<Tallies, String> {
    register_all();
    let failed = |detail: &str| Err(format!("{}: {detail}", spec.name));
    let args = CreateGameArgs {
        seed: spec.name.clone(),
        decks: spec.decks.clone(),
        ..CreateGameArgs::default()
    };
    let initial = create_game(&args);
    let begun = begin_game(&initial);
    if let Some(error) = begun.error {
        return failed(&format!("beginGame refused: {error}"));
    }
    let mut tracker = Tracker::default();
    let setup = values_of(&begun.events).map_err(|error| format!("{}: {error:#}", spec.name))?;
    tracker.observe(&setup, 0, &|id| place_in(&initial, &begun.state, id));
    let mut state = begun.state;
    if !state_paused(&state) {
        tracker.settle();
    }
    let mut policy = Rng::new(&format!("{}-policy", spec.name), 0);
    let mut order = Rng::new(&format!("{}-order", spec.name), 0);
    let mut step = 0usize;
    while state.result.is_none() && step < FX_CENSUS_STEP_CAP {
        let player = actor_of(&state, &mut order);
        let Some(chosen) = choose_action(&state, player, &mut policy) else {
            return failed(&format!(
                "no legal action for {player} while the game is live (turn {})",
                state.turn
            ));
        };
        let tributes = tributes_of(&chosen);
        let action = Action::new(chosen, player, format!("fx-census-{step}"));
        let result = reduce(&state, &action);
        if let Some(error) = result.error {
            return failed(&format!(
                "reduce refused \"{}\" at step {step}: {error}",
                action.action_type()
            ));
        }
        let events = values_of(&result.events).map_err(|error| format!("{}: {error:#}", spec.name))?;
        tracker.observe(&events, tributes, &|id| place_in(&state, &result.state, id));
        state = result.state;
        if !state_paused(&state) {
            tracker.settle();
        }
        step += 1;
    }
    tracker.settle();
    Ok(tracker.tallies)
}

// The census

/// One catalog card the report may hold.
#[derive(Clone, Debug)]
struct CensusCard {
    id: String,
    name: String,
    set: String,
    token: bool,
    deckable: bool,
}

/// Every shipped card in catalog order, with what the rows need of each.
fn catalog_cards() -> Vec<CensusCard> {
    CATALOG
        .iter()
        .filter(|(_, def)| set_ships(def.set))
        .map(|(id, def)| CensusCard {
            id: id.clone(),
            name: def.name.clone(),
            set: def.set.as_str().to_string(),
            token: def.is_token(),
            deckable: deckable(def),
        })
        .collect()
}

/// How many games the census plays.
#[derive(Clone, Copy, Debug)]
struct CensusOptions {
    seeds: u32,
    min_seen: u32,
    max_forced: u32,
}

/// One game that refused, stalled or panicked; reported, never fatal.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct FailedGame {
    game: String,
    error: String,
}

/// One card's report row.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct CardRow {
    id: String,
    name: String,
    set: String,
    token: bool,
    plays: u32,
    forced_games: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    unseen: Option<String>,
    motif: String,
    hooks: BTreeMap<String, HookTally>,
}

/// The whole census, as `--format json` writes it.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Report {
    seeds: u32,
    min_seen: u32,
    forced_games: u32,
    failed: Vec<FailedGame>,
    cards: Vec<CardRow>,
}

/// `id`'s `play` windows in `tallies`.
fn plays(tallies: &Tallies, id: &str) -> u32 {
    tallies
        .get(id)
        .and_then(|hooks| hooks.get("play"))
        .map_or(0, |tally| tally.windows)
}

/// Run the census over `cards`: the base games, then forced games for the rarely seen cards,
/// merged in order so the same seeds give the same report on any thread count.
fn census(options: CensusOptions, cards: &[CensusCard]) -> Report {
    let base: Vec<Result<Tallies, String>> = (1..=options.seeds)
        .into_par_iter()
        .map(|k| play_game(&base_spec(k)))
        .collect();
    let mut tallies = Tallies::new();
    let mut failed: Vec<FailedGame> = Vec::new();
    for (k, outcome) in (1..).zip(base) {
        match outcome {
            Ok(got) => merge_tallies(&mut tallies, got),
            Err(error) => failed.push(FailedGame {
                game: base_spec(k).name,
                error,
            }),
        }
    }
    let short: Vec<&CensusCard> = cards
        .iter()
        .filter(|card| !card.token && card.deckable && plays(&tallies, &card.id) < options.min_seen)
        .collect();
    let forced: Vec<(String, Tallies, u32, Vec<FailedGame>)> = short
        .par_iter()
        .map(|card| {
            let mut owned = Tallies::new();
            let mut errors: Vec<FailedGame> = Vec::new();
            let mut games = 0u32;
            for a in 1..=options.max_forced {
                let spec = forced_spec(&card.id, a);
                match play_game(&spec) {
                    Ok(got) => merge_tallies(&mut owned, got),
                    Err(error) => errors.push(FailedGame {
                        game: spec.name,
                        error,
                    }),
                }
                games += 1;
                if plays(&tallies, &card.id) + plays(&owned, &card.id) >= options.min_seen {
                    break;
                }
            }
            (card.id.clone(), owned, games, errors)
        })
        .collect();
    let mut forced_games = 0u32;
    let mut forced_count: BTreeMap<String, u32> = BTreeMap::new();
    for (id, owned, games, errors) in forced {
        merge_tallies(&mut tallies, owned);
        failed.extend(errors);
        forced_games += games;
        forced_count.insert(id, games);
    }
    Report {
        seeds: options.seeds,
        min_seen: options.min_seen,
        forced_games,
        failed,
        cards: rows(cards, &tallies, &forced_count, options.min_seen),
    }
}

/// The report's rows: every non-token card, and each token seen, with motifs suggested.
fn rows(
    cards: &[CensusCard],
    tallies: &Tallies,
    forced: &BTreeMap<String, u32>,
    min_seen: u32,
) -> Vec<CardRow> {
    let mut out: Vec<CardRow> = Vec::new();
    for card in cards {
        let seen = tallies.get(&card.id);
        if card.token && seen.is_none() {
            continue;
        }
        let mut hooks: BTreeMap<String, HookTally> = BTreeMap::new();
        if let Some(found) = seen {
            for (hook, tally) in found {
                let mut scored = tally.clone();
                scored.motif = suggest(&scored);
                hooks.insert(hook.clone(), scored);
            }
        }
        let played = hooks.get("play").map_or(0, |tally| tally.windows);
        let unseen = if played >= min_seen {
            None
        } else if !card.deckable {
            Some(format!(
                "played {played} of {min_seen} times: no deck may hold it (made by a rule)"
            ))
        } else {
            let games = forced.get(&card.id).copied().unwrap_or(0);
            Some(format!(
                "played {played} of {min_seen} times in {games} forced games"
            ))
        };
        out.push(CardRow {
            id: card.id.clone(),
            name: card.name.clone(),
            set: card.set.clone(),
            token: card.token,
            plays: played,
            forced_games: forced.get(&card.id).copied().unwrap_or(0),
            unseen,
            motif: card_motif(&hooks),
            hooks,
        });
    }
    out
}

// Rendering

/// The report as JSON a test can read.
fn render_json(report: &Report) -> Result<String> {
    Ok(serde_json::to_string_pretty(report)? + "\n")
}

/// Escape a table cell's pipes.
fn cell(text: &str) -> String {
    text.replace('|', "\\|")
}

/// `name count` pairs, or the empty mark.
fn pairs(hist: &BTreeMap<String, u32>) -> String {
    if hist.is_empty() {
        return "—".to_string();
    }
    hist.iter()
        .map(|(name, count)| format!("{name} {count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// One hook's one-line shape: its events, targets, sides, lanes, split, return, cost and motif.
fn hook_cell(hook: &str, tally: &HookTally) -> String {
    let targets = commonest(&tally.targets).unwrap_or(0);
    let sides = commonest(&tally.sides).unwrap_or("none".to_string());
    let lanes = commonest(&tally.lanes).unwrap_or("none".to_string());
    format!(
        "{hook} ×{}: {}; targets {targets}; sides {sides}; lanes {lanes}; split {}; return {}; cost {}; via {}; {}",
        tally.windows,
        pairs(&tally.events),
        tally.random_split,
        tally.graveyard_return,
        tally.own_cost,
        pairs(&tally.via),
        tally.motif,
    )
}

/// The report as a table with one row per card, grouped by set.
fn render_md(report: &Report) -> String {
    let mut out = "# fx census\n".to_string();
    out.push_str(&format!(
        "\n{} seeded games, {} forced games, {} failed; each card played at least {} times or marked unseen.\n",
        report.seeds,
        report.forced_games,
        report.failed.len(),
        report.min_seen,
    ));
    let mut set = String::new();
    for card in &report.cards {
        if card.set != set {
            set.clone_from(&card.set);
            out.push_str(&format!(
                "\n## {set}\n\n| Card | Name | Plays | Unseen | Hooks | Suggested |\n| --- | --- | --- | --- | --- | --- |\n"
            ));
        }
        let mut hooks: Vec<&String> = card.hooks.keys().collect();
        hooks.sort_by_key(|hook| {
            (
                HOOK_ORDER
                    .iter()
                    .position(|held| held == hook)
                    .unwrap_or(HOOK_ORDER.len()),
                (*hook).clone(),
            )
        });
        let cells = hooks
            .iter()
            .map(|hook| hook_cell(hook, &card.hooks[*hook]))
            .collect::<Vec<_>>()
            .join("<br>");
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} |\n",
            card.id,
            cell(&card.name),
            card.plays,
            card.unseen.as_deref().map_or("—".to_string(), cell),
            if cells.is_empty() {
                "—".to_string()
            } else {
                cells
            },
            card.motif,
        ));
    }
    if !report.failed.is_empty() {
        out.push_str("\n## Failed games\n");
        for game in &report.failed {
            out.push_str(&format!("- `{}`: {}\n", game.game, cell(&game.error)));
        }
    }
    out
}

/// Play the census and print it, to stdout or `--out`; failed games are reported, never fatal.
pub fn run(args: Args) -> Result<()> {
    register_all();
    let FxCommand::Census(asked) = args.command;
    let report = census(
        CensusOptions {
            seeds: asked.seeds,
            min_seen: FX_CENSUS_MIN_SEEN,
            max_forced: FX_CENSUS_MAX_FORCED_GAMES,
        },
        &catalog_cards(),
    );
    let text = match asked.format {
        Format::Md => render_md(&report),
        Format::Json => render_json(&report)?,
    };
    match asked.out {
        Some(path) => {
            fs::write(&path, &text).with_context(|| format!("writing {}", path.display()))?;
            println!("fx census: {} cards to {}", report.cards.len(), path.display());
        }
        None => print!("{text}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::GameEvent;
    use serde_json::json;

    /// Fixture events through the stream's own types, so a typo fails the parse.
    fn stream(events: Value) -> Vec<Value> {
        let typed: Vec<GameEvent> = serde_json::from_value(events).expect("the fixture's events are valid");
        typed
            .into_iter()
            .map(|event| serde_json::to_value(event).expect("an event"))
            .collect()
    }

    /// A unit of `side` in `lane`.
    fn unit(side: PlayerId, lane: i32) -> Place {
        Place {
            board: Some((side, Some(lane))),
            from_graveyard: false,
        }
    }

    /// The hero of `side`, which sits in no lane.
    fn hero(side: PlayerId) -> Place {
        Place {
            board: Some((side, None)),
            from_graveyard: false,
        }
    }

    /// A card still in its graveyard, as a return's summon finds it.
    fn buried() -> Place {
        Place {
            board: None,
            from_graveyard: true,
        }
    }

    /// `observe` over a map lookup, then `settle`: one settled step's tallies.
    fn tally(events: Vec<Value>, tributes: u32, places: &BTreeMap<&str, Place>) -> Tallies {
        let mut tracker = Tracker::default();
        tracker.observe(&events, tributes, &|id| {
            places.get(id).copied().unwrap_or_default()
        });
        tracker.settle();
        tracker.tallies
    }

    fn played(player: &str, instance: &str, def: &str) -> Value {
        json!({"type": "cardPlayed", "player": player, "instanceId": instance, "defId": def, "costPaid": 2})
    }

    fn resolved(player: &str, instance: &str, def: &str) -> Value {
        json!({"type": "cardResolved", "player": player, "instanceId": instance, "defId": def, "permanent": false, "costPaid": 2})
    }

    fn damage(source: &str, target: &str) -> Value {
        json!({"type": "damage", "sourceId": source, "targetId": target, "amount": 3, "combat": false})
    }

    fn destroyed(instance: &str, owner: &str, controller: &str) -> Value {
        json!({"type": "destroyed", "instanceId": instance, "defId": "core-001", "owner": owner, "controller": controller, "attack": 2, "maxHealth": 2})
    }

    fn marker(source: &str, def: &str, controller: &str, hook: &str) -> Value {
        json!({"type": "hookResolving", "sourceId": source, "defId": def, "controller": controller, "hook": hook})
    }

    #[test]
    fn a_single_hit_suggests_bolt() {
        let events = stream(json!([
            played("p1", "s1", "core-035"),
            damage("s1", "e1"),
            resolved("p1", "s1", "core-035")
        ]));
        let tallies = tally(events, 0, &BTreeMap::from([("e1", unit(PlayerId::P2, 2))]));
        let play = &tallies["core-035"]["play"];
        assert_eq!(play.targets, BTreeMap::from([(1, 1)]));
        assert_eq!(play.sides, BTreeMap::from([("enemy".to_string(), 1)]));
        assert_eq!(play.lanes, BTreeMap::from([("one".to_string(), 1)]));
        assert_eq!(suggest(play), "bolt");
    }

    #[test]
    fn hits_on_several_enemies_suggest_volley() {
        let events = stream(json!([
            played("p1", "s1", "traininggrounds-010"),
            damage("s1", "e1"),
            damage("s1", "e2"),
            resolved("p1", "s1", "traininggrounds-010"),
        ]));
        let tallies = tally(
            events,
            0,
            &BTreeMap::from([("e1", unit(PlayerId::P2, 1)), ("e2", unit(PlayerId::P2, 3))]),
        );
        let play = &tallies["traininggrounds-010"]["play"];
        assert_eq!(play.lanes, BTreeMap::from([("spread".to_string(), 1)]));
        assert_eq!(suggest(play), "volley");
    }

    #[test]
    fn a_random_split_suggests_barrage() {
        let events = stream(json!([
            played("p1", "s1", "stinkyscape-097"),
            damage("s1", "e1"),
            damage("s1", "e2"),
            damage("s1", "e1"),
            resolved("p1", "s1", "stinkyscape-097"),
        ]));
        let tallies = tally(
            events,
            0,
            &BTreeMap::from([("e1", unit(PlayerId::P2, 1)), ("e2", unit(PlayerId::P2, 2))]),
        );
        let play = &tallies["stinkyscape-097"]["play"];
        assert_eq!(play.random_split, 1);
        assert_eq!(suggest(play), "barrage");
    }

    #[test]
    fn a_hit_on_one_lane_suggests_lance() {
        let events = stream(json!([
            played("p1", "u1", "classic-071"),
            json!({"type": "summoned", "player": "p1", "instanceId": "u1", "defId": "classic-071", "row": "units", "lane": 2}),
            marker("u1", "classic-071", "p1", "cry"),
            destroyed("v1", "p1", "p1"),
            destroyed("v2", "p2", "p2"),
            resolved("p1", "u1", "classic-071"),
        ]));
        let tallies = tally(
            events,
            0,
            &BTreeMap::from([("v1", unit(PlayerId::P1, 2)), ("v2", unit(PlayerId::P2, 2))]),
        );
        let cry = &tallies["classic-071"]["cry"];
        assert_eq!(cry.events, BTreeMap::from([("destroyed".to_string(), 2)]));
        assert_eq!(cry.sides, BTreeMap::from([("both".to_string(), 1)]));
        assert_eq!(suggest(cry), "lance");
        let mut scored = cry.clone();
        scored.motif = suggest(&scored);
        let mut hooks = BTreeMap::new();
        hooks.insert("cry".to_string(), scored);
        assert_eq!(card_motif(&hooks), "lance");
    }

    #[test]
    fn hits_on_both_sides_suggest_nova() {
        let events = stream(json!([
            played("p1", "s1", "classicplus-021"),
            damage("s1", "hero-p1"),
            damage("s1", "hero-p2"),
            resolved("p1", "s1", "classicplus-021"),
        ]));
        let tallies = tally(
            events,
            0,
            &BTreeMap::from([("hero-p1", hero(PlayerId::P1)), ("hero-p2", hero(PlayerId::P2))]),
        );
        let play = &tallies["classicplus-021"]["play"];
        assert_eq!(suggest(play), "nova");
    }

    #[test]
    fn destroying_without_damage_suggests_wipe() {
        let events = stream(json!([
            played("p1", "s1", "core-088"),
            destroyed("v1", "p2", "p2"),
            destroyed("v2", "p2", "p2"),
            resolved("p1", "s1", "core-088"),
        ]));
        let tallies = tally(
            events,
            0,
            &BTreeMap::from([("v1", unit(PlayerId::P2, 1)), ("v2", unit(PlayerId::P2, 4))]),
        );
        assert_eq!(suggest(&tallies["core-088"]["play"]), "wipe");
    }

    #[test]
    fn an_exile_suggests_vanish() {
        let events = stream(json!([
            played("p1", "s1", "meditative-078"),
            json!({"type": "exiled", "instanceId": "v1", "defId": "core-001", "owner": "p2"}),
            resolved("p1", "s1", "meditative-078"),
        ]));
        let tallies = tally(events, 0, &BTreeMap::from([("v1", unit(PlayerId::P2, 3))]));
        assert_eq!(suggest(&tallies["meditative-078"]["play"]), "vanish");
    }

    #[test]
    fn a_trap_that_counters_suggests_negate() {
        let events = stream(json!([
            played("p1", "s9", "core-001"),
            json!({"type": "trapFired", "instanceId": "t1", "defId": "classic-002", "controller": "p2", "row": "backrow", "lane": 1}),
            json!({"type": "countered", "player": "p1", "instanceId": "s9", "defId": "core-001", "to": "graveyard"}),
            resolved("p1", "s9", "core-001"),
        ]));
        let tallies = tally(events, 0, &BTreeMap::new());
        assert_eq!(suggest(&tallies["classic-002"]["fire"]), "negate");
        assert_eq!(suggest(&tallies["core-001"]["play"]), VANILLA);
    }

    #[test]
    fn a_summon_from_the_graveyard_suggests_rise() {
        let events = stream(json!([
            played("p1", "u1", "core-010"),
            marker("u1", "core-010", "p1", "death"),
            json!({"type": "summoned", "player": "p1", "instanceId": "m1", "defId": "core-010", "row": "units", "lane": 2}),
            resolved("p1", "u1", "core-010"),
        ]));
        let tallies = tally(events, 0, &BTreeMap::from([("m1", buried())]));
        let death = &tallies["core-010"]["death"];
        assert_eq!(death.graveyard_return, 1);
        assert_eq!(suggest(death), "rise");
    }

    #[test]
    fn tributes_suggest_offering() {
        let events = stream(json!([
            played("p1", "s1", "core-034"),
            resolved("p1", "s1", "core-034")
        ]));
        let tallies = tally(events, 1, &BTreeMap::new());
        let play = &tallies["core-034"]["play"];
        assert_eq!(play.own_cost, 1);
        assert_eq!(suggest(play), "offering");
    }

    #[test]
    fn a_play_that_shows_nothing_is_vanilla() {
        let events = stream(json!([
            played("p1", "s1", "core-002"),
            resolved("p1", "s1", "core-002")
        ]));
        let tallies = tally(events, 0, &BTreeMap::new());
        assert_eq!(suggest(&tallies["core-002"]["play"]), VANILLA);
    }

    #[test]
    fn events_no_motif_reads_name_the_commonest() {
        let events = stream(json!([
            played("p1", "s1", "core-003"),
            json!({"type": "drawn", "player": "p1", "instanceId": "d1", "defId": "core-004"}),
            json!({"type": "drawn", "player": "p1", "instanceId": "d2", "defId": "core-005"}),
            resolved("p1", "s1", "core-003"),
        ]));
        let tallies = tally(events, 0, &BTreeMap::new());
        assert_eq!(
            suggest(&tallies["core-003"]["play"]),
            "none: no motif reads drawn"
        );
    }

    #[test]
    fn a_different_cards_trigger_is_not_the_plays() {
        let events = stream(json!([
            played("p1", "s1", "core-020"),
            marker("t9", "classic-030", "p2", "whenever"),
            damage("t9", "e1"),
            resolved("p1", "s1", "core-020"),
        ]));
        let tallies = tally(events, 0, &BTreeMap::from([("e1", unit(PlayerId::P1, 1))]));
        assert!(tallies["core-020"]["play"].events.is_empty());
        assert_eq!(
            tallies["classic-030"]["whenever"].events,
            BTreeMap::from([("damage".to_string(), 1)])
        );
    }

    #[test]
    fn a_marker_closes_the_hook_before_it() {
        let events = stream(json!([
            marker("b1", "core-040", "p1", "cry"),
            damage("b1", "e1"),
            marker("c1", "core-041", "p1", "whenever"),
            damage("c1", "e1"),
        ]));
        let tallies = tally(events, 0, &BTreeMap::from([("e1", unit(PlayerId::P2, 1))]));
        assert_eq!(
            tallies["core-040"]["cry"].events,
            BTreeMap::from([("damage".to_string(), 1)])
        );
        assert_eq!(
            tallies["core-041"]["whenever"].events,
            BTreeMap::from([("damage".to_string(), 1)])
        );
    }

    #[test]
    fn a_prompt_keeps_the_window_open() {
        let places = BTreeMap::from([("e1", unit(PlayerId::P2, 1))]);
        let lookup = |id: &str| places.get(id).copied().unwrap_or_default();
        let mut tracker = Tracker::default();
        tracker.observe(&stream(json!([played("p1", "s1", "core-050")])), 0, &lookup);
        tracker.observe(
            &stream(json!([damage("s1", "e1"), resolved("p1", "s1", "core-050")])),
            0,
            &lookup,
        );
        tracker.settle();
        let play = &tracker.tallies["core-050"]["play"];
        assert_eq!(play.windows, 1);
        assert_eq!(play.events, BTreeMap::from([("damage".to_string(), 1)]));
    }

    #[test]
    fn via_keywords_are_counted() {
        let events = stream(json!([
            played("p1", "s1", "core-060"),
            json!({"type": "healed", "targetId": "e1", "amount": 2, "via": "Lifesteal"}),
            resolved("p1", "s1", "core-060"),
        ]));
        let tallies = tally(events, 0, &BTreeMap::from([("e1", unit(PlayerId::P1, 1))]));
        assert_eq!(
            tallies["core-060"]["play"].via,
            BTreeMap::from([("Lifesteal".to_string(), 1)])
        );
    }

    fn fixture_card(id: &str, name: &str, set: &str, token: bool, deckable: bool) -> CensusCard {
        CensusCard {
            id: id.to_string(),
            name: name.to_string(),
            set: set.to_string(),
            token,
            deckable,
        }
    }

    #[test]
    fn every_card_is_seen_or_listed_unseen_with_its_reason() {
        let cards = vec![
            fixture_card("a-001", "Seen", "Core", false, true),
            fixture_card("b-001", "Short", "Core", false, true),
            fixture_card("c-001", "Ruled", "Core", false, false),
            fixture_card("t-001", "Token Seen", "Core", true, false),
            fixture_card("t-002", "Token Hidden", "Core", true, false),
        ];
        let mut tallies = Tallies::new();
        tallies.entry("a-001".to_string()).or_default().insert(
            "play".to_string(),
            HookTally {
                windows: 3,
                ..HookTally::default()
            },
        );
        tallies.entry("b-001".to_string()).or_default().insert(
            "play".to_string(),
            HookTally {
                windows: 1,
                ..HookTally::default()
            },
        );
        tallies.entry("t-001".to_string()).or_default().insert(
            "death".to_string(),
            HookTally {
                windows: 2,
                ..HookTally::default()
            },
        );
        let forced = BTreeMap::from([("b-001".to_string(), 20)]);
        let got = rows(&cards, &tallies, &forced, 3);
        let ids: Vec<&str> = got.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids, ["a-001", "b-001", "c-001", "t-001"]);
        assert_eq!(got[0].unseen, None);
        assert_eq!(
            got[1].unseen.as_deref(),
            Some("played 1 of 3 times in 20 forced games")
        );
        assert_eq!(
            got[2].unseen.as_deref(),
            Some("played 0 of 3 times: no deck may hold it (made by a rule)")
        );
        assert_eq!(got[2].motif, UNSEEN);
    }

    fn two_set_report() -> Report {
        let cards = vec![
            fixture_card("core-035", "Lunar Eclipse", "Core", false, true),
            fixture_card("classic-071", "Lane Eater", "Classic", false, true),
        ];
        let mut tallies = Tallies::new();
        tallies.entry("core-035".to_string()).or_default().insert(
            "play".to_string(),
            HookTally {
                windows: 1,
                events: BTreeMap::from([("damage".to_string(), 2)]),
                targets: BTreeMap::from([(2, 1)]),
                sides: BTreeMap::from([("enemy".to_string(), 1)]),
                lanes: BTreeMap::from([("spread".to_string(), 1)]),
                families: BTreeMap::from([("damage".to_string(), 1)]),
                ..HookTally::default()
            },
        );
        Report {
            seeds: 2,
            min_seen: 3,
            forced_games: 0,
            failed: Vec::new(),
            cards: rows(&cards, &tallies, &BTreeMap::new(), 3),
        }
    }

    #[test]
    fn the_markdown_groups_rows_by_set_and_the_json_names_every_field() {
        let report = two_set_report();
        let md = render_md(&report);
        let core = md.find("## Core").expect("the Core heading");
        let classic = md.find("## Classic").expect("the Classic heading");
        assert!(core < classic, "{md}");
        assert!(
            md.contains("| `core-035` | Lunar Eclipse | 1 | played 1 of 3 times in 0 forced games | play ×1: damage 2; targets 2; sides enemy; lanes spread; split 0; return 0; cost 0; via —; volley | volley |"),
            "{md}"
        );
        let json = render_json(&report).expect("the JSON");
        let parsed: Value = serde_json::from_str(&json).expect("the JSON parses");
        let hook = &parsed["cards"][0]["hooks"]["play"];
        for field in [
            "windows",
            "events",
            "targets",
            "sides",
            "lanes",
            "randomSplit",
            "graveyardReturn",
            "ownCost",
            "via",
            "motif",
        ] {
            assert!(hook.get(field).is_some(), "the hook holds {field}: {json}");
        }
    }

    #[test]
    fn the_census_plays_forced_games_until_each_card_is_seen() {
        register_all();
        let cards: Vec<CensusCard> = catalog_cards()
            .into_iter()
            .filter(|card| card.id == "core-035" || card.id == "classic-071")
            .collect();
        assert_eq!(cards.len(), 2);
        let report = census(
            CensusOptions {
                seeds: 1,
                min_seen: 1,
                max_forced: 4,
            },
            &cards,
        );
        assert_eq!(report.cards.len(), 2);
        for row in &report.cards {
            assert!(
                row.plays >= 1 || row.unseen.is_some(),
                "{} is neither played nor listed unseen",
                row.id
            );
        }
    }

    #[test]
    fn the_same_seeds_give_the_same_output_on_one_or_four_threads() {
        register_all();
        let cards: Vec<CensusCard> = catalog_cards()
            .into_iter()
            .filter(|card| card.id == "core-035" || card.id == "classic-071")
            .collect();
        let options = CensusOptions {
            seeds: 1,
            min_seen: 1,
            max_forced: 2,
        };
        let one = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
        let four = rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap();
        let a = one.install(|| census(options, &cards));
        let b = four.install(|| census(options, &cards));
        assert_eq!(render_json(&a).unwrap(), render_json(&b).unwrap());
        assert_eq!(render_md(&a), render_md(&b));
    }
}
