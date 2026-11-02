//! Drawing, fatigue, the hand cap, cast-on-draw chains and the library cap
//! (SPEC §2.4, R3, R4, R58, R80), the arrival hook of R151, and — since a cast-on-draw card is a
//! whole play and a play can ask — both draw loops resuming out of `state.work` across a prompt
//! (§9.3, §10.6, R113, R117, R122). B5 adds the draws a player makes each turn (E4), the
//! draw limits a card on the field sets (E3), the `castOnDraw` enchantment (E39) and a Unit cast on draw
//! with nowhere to stand (R457, R459).
//!
//! A card that is moved is passed as `&mut CardInstance` (its zone follows the move) or by value once the
//! caller has taken it out of its pile (`complete_draw`); a later write goes through `find_instance_mut`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{CAST_ON_DRAW_ACTION_CAP, CAST_ON_DRAW_CHAIN_CAP, FATIGUE_DAMAGE, SETUP_TURN};
use crate::script::{DrawLimit, DrawLimitPlayer, EngineSink, HookArgs};
use crate::state::{CardInstance, DrawCount, GameState, Resume, WorkItem, find_instance_mut, new_instance};
use crate::wire::{
    CardType, Enchantment, GameEvent, LibraryOverflowOutcome, PLAYER_IDS, PlayerId, Row, SetName, Zone,
    opponent_of,
};

/// R151: a start-of-game hook also runs when its card ARRIVES in a hand or a library, not only at §2.1
/// step 4, as R43 states the roll as an invariant: a copy arriving later (#72 Reminisce, a bounce, a
/// reset instance, R78) must roll too, or it costs 0 for ever. It reads the script's `start_of_game`,
/// not #98's id; a rerun is safe, as `hero_power::ensure_power` keeps a rolled power. Not in
/// `zones::move_to_zone`: the roll needs the sink's rng, which a fresh `Rng` mid-action would repeat.
pub(crate) fn run_arrival_hooks(sink: &mut EngineSink, instance: &CardInstance) {
    // §9.3, R113: the clause is an effect list like any other, so a question in it pauses the rest of
    // it on `state.work` (`prompts::run_start_of_game`), and a draw loop around it owes its remainder.
    let whole = crate::prompts::run_start_of_game(sink, instance, instance.owner);
    // MD-B18, R925: a hand arrival also runs "When this enters your hand", after R151's clause.
    if whole
        && let Some(live) = crate::state::find_instance(sink.state, &instance.id).cloned()
        && matches!(live.zone, Zone::Hand { .. })
    {
        crate::prompts::run_enters_hand(sink, &live, live.owner);
    }
}

/// #75: a backrow card that turns an empty-library draw into a Rush Token card.
fn infinite_reserves_source(sink: &EngineSink, player: PlayerId) -> Option<CardInstance> {
    for slot in crate::zones::slots_of(player, Row::Backrow) {
        // R1487000: a face-down card's text sleeps until it is revealed or face-up.
        if let Some(card) = crate::zones::card_at(sink.state, slot)
            && !crate::preview::lasting_text_off(sink.state, card)
            && crate::scripts::script_of(sink.state, card)
                .flags()
                .infinite_reserves
                == Some(true)
        {
            return Some(card.clone());
        }
    }
    None
}

/// Where `add_to_hand` put a card.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum AddToHandOutcome {
    Hand,
    Burned,
}

/// §2.4, R4: a card entering a full hand is burned to the graveyard; unit tokens vanish (R11). Full is
/// `query::room_in`'s, the owner's hand cap: `HAND_CAP` or the hand size an effect set (R1143). Every
/// card that reaches a hand comes through here, so a caller reads where it went from the outcome
/// rather than testing the hand again.
pub fn add_to_hand(sink: &mut EngineSink, instance: &mut CardInstance) -> AddToHandOutcome {
    let full = crate::query::room_in(sink.state, instance.owner, crate::zones::OffFieldZone::Hand) == 0;
    if full {
        let landed = crate::zones::move_to_zone(
            sink.state,
            instance,
            crate::zones::OffFieldZone::Graveyard,
            Default::default(),
        );
        sink.events.push(GameEvent::Burned {
            instance_id: instance.id.clone(),
            def_id: instance.def_id.clone(),
            owner: instance.owner,
        });
        // B5 E5: where it landed — its graveyard, or wherever a replacement sent it; a unit token none (R11).
        crate::zones::report_graveyard_landing(sink, instance, landed);
        return AddToHandOutcome::Burned;
    }
    crate::zones::move_to_zone(
        sink.state,
        instance,
        crate::zones::OffFieldZone::Hand,
        Default::default(),
    );
    sink.events.push(GameEvent::AddedToHand {
        player: instance.owner,
        instance_id: instance.id.clone(),
        def_id: instance.def_id.clone(),
        creator: None,
    });
    // R151: it has arrived somewhere it can be looked at, so a start-of-game clause runs now.
    run_arrival_hooks(sink, instance);
    AddToHandOutcome::Hand
}

/// Where `shuffle_into_library` put a card.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum ShuffleInOutcome {
    Library,
    Dropped,
}

/// R316: the `libraryOverflow` a full library reports for `instance`.
fn library_overflow(
    instance: &CardInstance,
    outcome: LibraryOverflowOutcome,
    copy_of: Option<&str>,
) -> GameEvent {
    GameEvent::LibraryOverflow {
        player: instance.owner,
        instance_id: instance.id.clone(),
        def_id: instance.def_id.clone(),
        outcome,
        radiant: if instance.radiant { Some(true) } else { None },
        // R316: a copy that was never made is judged by the card it copies, which may be face-down.
        copy_of: if outcome == LibraryOverflowOutcome::NotCreated {
            copy_of.map(str::to_string)
        } else {
            None
        },
    }
}

/// R80: a library holds at most LIBRARY_CAP cards (`query::room_in`), so copies stop being created at
/// the cap, and an existing card goes to its owner's graveyard instead (a unit-token card ceases to
/// exist, R11). One already in its owner's graveyard stays there, with no second move and no second
/// `enteredGraveyard`, since it never left (C #30 Recycle's, `effects/shuffle_card.rs`).
///
/// R316: whichever it is, `libraryOverflow` reports the refusal (§10.3), so the board can show the full
/// library turning the card away. `copy_of` names the card a new copy copies, when it copies one, so a
/// view can keep a face-down trap's copy as secret as the trap.
pub fn shuffle_into_library(
    sink: &mut EngineSink,
    instance: &mut CardInstance,
    existing: bool,
    copy_of: Option<&str>,
) -> ShuffleInOutcome {
    into_library(sink, instance, existing, copy_of, Placement::Random)
}

/// R2657003: a Traded card shuffled in anywhere but the top, so the draw that follows never brings it
/// back while the deck holds another card; into an empty deck it is the top and only card. One rng draw
/// over the positions below the top, otherwise as `shuffle_into_library`.
pub fn shuffle_into_library_below_top(
    sink: &mut EngineSink,
    instance: &mut CardInstance,
) -> ShuffleInOutcome {
    into_library(sink, instance, true, None, Placement::BelowTop)
}

/// Where `into_library` puts a card: a random slot (a shuffle-in), a named end (ME-DECK-BOTTOM), or a
/// random slot under the top card (R2657003's Trade).
#[derive(Clone, Copy)]
enum Placement {
    Random,
    End(crate::ownership::LibraryEnd),
    BelowTop,
}

/// A fresh card put at a named end of its owner's library (Meditative #49's bottom, ME-DECK-BOTTOM):
/// `LibraryEnd::Top` is position 0, `LibraryEnd::Bottom` the last one, with no rng draw. The caps
/// and reports are `shuffle_into_library`'s.
pub fn put_into_library(
    sink: &mut EngineSink,
    instance: &mut CardInstance,
    existing: bool,
    copy_of: Option<&str>,
    end: crate::ownership::LibraryEnd,
) -> ShuffleInOutcome {
    into_library(sink, instance, existing, copy_of, Placement::End(end))
}

fn into_library(
    sink: &mut EngineSink,
    instance: &mut CardInstance,
    existing: bool,
    copy_of: Option<&str>,
    end: Placement,
) -> ShuffleInOutcome {
    let library_len = sink.state.players[instance.owner].library.len() as i32;
    if crate::query::room_in(sink.state, instance.owner, crate::zones::OffFieldZone::Library) == 0 {
        if !existing {
            sink.events.push(library_overflow(
                instance,
                LibraryOverflowOutcome::NotCreated,
                copy_of,
            ));
            return ShuffleInOutcome::Dropped;
        }
        let owner = instance.owner;
        let in_own_graveyard = crate::state::find_instance(sink.state, &instance.id)
            .is_some_and(|live| matches!(live.zone, Zone::Graveyard { player } if player == owner));
        if in_own_graveyard {
            sink.events.push(library_overflow(
                instance,
                LibraryOverflowOutcome::Graveyard,
                copy_of,
            ));
            return ShuffleInOutcome::Dropped;
        }
        if crate::zones::is_unit_token(sink.state, instance) {
            crate::zones::move_to_zone(
                sink.state,
                instance,
                crate::zones::OffFieldZone::Exile,
                Default::default(),
            );
            sink.events.push(library_overflow(
                instance,
                LibraryOverflowOutcome::Ceased,
                copy_of,
            ));
            return ShuffleInOutcome::Dropped;
        }
        let landed = crate::zones::move_to_zone(
            sink.state,
            instance,
            crate::zones::OffFieldZone::Graveyard,
            Default::default(),
        );
        sink.events.push(library_overflow(
            instance,
            LibraryOverflowOutcome::Graveyard,
            copy_of,
        ));
        crate::zones::report_graveyard_landing(sink, instance, landed);
        return ShuffleInOutcome::Dropped;
    }
    // ME-DECK-BOTTOM: a named end is placed, not drawn — the bottom is the last card, with no
    // rng draw (R80's cap still refuses a full library, above).
    let position = match end {
        Placement::Random => sink.rng.int(library_len + 1),
        Placement::End(crate::ownership::LibraryEnd::Top) => 0,
        Placement::End(crate::ownership::LibraryEnd::Bottom) => library_len,
        // R2657003: a random slot under the top card, or the top of an empty deck.
        Placement::BelowTop if library_len == 0 => 0,
        Placement::BelowTop => 1 + sink.rng.int(library_len),
    };
    crate::zones::move_to_zone(
        sink.state,
        instance,
        crate::zones::OffFieldZone::Library,
        crate::zones::MoveToZoneOptions {
            position: Some(crate::zones::LibraryPosition::At(position)),
            ..Default::default()
        },
    );
    // R311: a shuffle-in is open to the library's owner: every Core one goes in by a card its owner
    // watched resolve, so they know what went in, though never where (the slot stays hidden, R97).
    if let Some(live) = find_instance_mut(sink.state, &instance.id) {
        crate::own_library::show_to_owner(live);
    }
    crate::own_library::show_to_owner(instance);
    sink.events.push(GameEvent::ShuffledIn {
        player: instance.owner,
        instance_id: instance.id.clone(),
        def_id: instance.def_id.clone(),
        position,
        creator: None,
    });
    // R151: R43 names a library as well as a hand, so a card shuffled back rolls the same way.
    run_arrival_hooks(sink, instance);
    ShuffleInOutcome::Library
}

/// `Limited`: a draw limit stopped it (B5 E3, R457) — nothing moved, no fatigue, nothing cast.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum DrawOutcome {
    Drawn,
    Cast,
    Burned,
    Fatigue,
    Token,
    Limited,
}

// Draw limits, draw counts and cast on draw (B5 E3, E4, E39)

/// B5 E4, R457: the draws `player` has made this turn, whoever's turn it is, a fatigue draw included
/// and none a limit stopped; a count from an earlier turn reads as 0 (it resets with the turn log,
/// §2.2). Setup is no player's turn (§2.1, `SETUP_TURN`), so its draws count toward nothing.
pub fn draws_this_turn(state: &GameState, player: PlayerId) -> i32 {
    match state.players[player].draws {
        Some(draws) if draws.turn == state.turn => draws.count,
        _ => 0,
    }
}

/// Count one draw that happened, and return its number this turn (1 for the first) for the `drawn`
/// event. Nothing during setup: R225 deals a Quickdraw card as an opening draw with an event of its
/// own, and a number on the other draws would tell the opponent which one it was (§9.1, R97).
fn count_draw(state: &mut GameState, player: PlayerId) -> Option<i32> {
    if state.turn == SETUP_TURN {
        return None;
    }
    let count = draws_this_turn(state, player) + 1;
    state.players[player].draws = Some(DrawCount {
        turn: state.turn,
        count,
    });
    Some(count)
}

/// The cards whose static text is in force on `player`'s side, each once (`zones::acting_on`), a Unit
/// standing in the backrow among the Units (R1081000, R1824000): what an aura is read from (§10.4
/// layer 5). A Vanilla card carries no text (§6.3), and a face-down card's sleeps (R1487000).
fn text_on_field(state: &GameState, player: PlayerId) -> Vec<&CardInstance> {
    crate::zones::acting_on(state, player, crate::zones::StandingAt::AmongUnits)
        .into_iter()
        // R1487000: a face-down card's text sleeps until it is revealed or face-up.
        .filter(|card| !crate::preview::lasting_text_off(state, card))
        .collect()
}

fn limit_binds(limit: &DrawLimit, controller: PlayerId, player: PlayerId) -> bool {
    player_binds(limit.player, controller, player)
}

/// Whether `who`, read relative to a card's controller, names `player`.
fn player_binds(who: DrawLimitPlayer, controller: PlayerId, player: PlayerId) -> bool {
    if who == DrawLimitPlayer::Both {
        return true;
    }
    if who == DrawLimitPlayer::SelfSide {
        player == controller
    } else {
        player == opponent_of(controller)
    }
}

/// B5 E3, R457: the most draws `player` may make each turn, or `None` when nothing limits them. Every
/// card on the field may set limits (`Script.draw_limit`, Classic #4 Palantir, #49 Anti-Greed Machine),
/// and with several the lowest holds.
pub fn draw_limit_of(state: &GameState, player: PlayerId) -> Option<i32> {
    let mut lowest: Option<i32> = None;
    for side in PLAYER_IDS {
        for card in text_on_field(state, side) {
            let script = crate::scripts::script_of(state, card);
            let Some(hook) = script.draw_limit.as_ref() else {
                continue;
            };
            for limit in hook(HookArgs {
                state,
                self_: card,
                radiant: card.radiant,
            }) {
                if !limit_binds(&limit, card.controller, player) {
                    continue;
                }
                let count = limit.count.max(0);
                lowest = Some(match lowest {
                    None => count,
                    Some(low) => low.min(count),
                });
            }
        }
    }
    lowest
}

/// R800: whether an effect's discard from `player`'s hand is stopped now: while it is not their turn,
/// a card acting on the field guards them (`Script.discard_guard`, Meditative #1), several as one. A
/// discard paid as a price (an Activate's cost, R384; a targeting cost, R450) never asks.
pub fn discard_guarded(state: &GameState, player: PlayerId) -> bool {
    if state.active == player {
        return false;
    }
    PLAYER_IDS.into_iter().any(|side| {
        text_on_field(state, side).into_iter().any(|card| {
            let script = crate::scripts::script_of(state, card);
            script.discard_guard.as_ref().is_some_and(|hook| {
                hook(HookArgs {
                    state,
                    self_: card,
                    radiant: card.radiant,
                })
                .into_iter()
                .any(|who| player_binds(who, card.controller, player))
            })
        })
    })
}

/// B5 E3, R457: whether `player`'s next draw may not happen: they have made as many draws this turn as
/// their limit allows. A stopped draw does not happen at all (no card moves, no fatigue, no cast, no
/// count); `drawLimited` (public, it names no card) is all that is said. Every draw asks this first:
/// `draw_one`, a named one (`effects::draw::draw_from_library`), one out of the opponent's deck (E16).
pub fn draw_blocked(sink: &mut EngineSink, player: PlayerId) -> bool {
    let Some(limit) = draw_limit_of(sink.state, player) else {
        return false;
    };
    if draws_this_turn(sink.state, player) < limit {
        return false;
    }
    sink.events.push(GameEvent::DrawLimited { player });
    true
}

/// Whether the card carries the `castOnDraw` enchantment.
fn enchanted_cast_on_draw(card: &CardInstance) -> bool {
    card.enchantments
        .iter()
        .flatten()
        .any(|entry| matches!(entry, Enchantment::CastOnDraw))
}

/// §2.4, R58, B5 E39: whether the drawn card casts itself — printed Cast on draw, or the `castOnDraw`
/// enchantment riding it (Classic+ #40 Appropriations), or, for a card that has the last Spell's text
/// (B5 E14, Classic #57 Echo, R547), that Spell's Cast on draw.
pub fn casts_on_draw(state: &GameState, card: &CardInstance) -> bool {
    crate::scripts::script_of(state, card).flags().cast_on_draw == Some(true)
        || enchanted_cast_on_draw(card)
        || crate::subsystems::copied_text::copied_casts_on_draw(state, card)
}

/// R459 (Classic+ #26): a Unit cast on draw is played for free into its caster's leftmost open unit
/// zone (R64, R70); with none open it goes to the hand uncast, as past R58's cap. Any other card is
/// cast wherever it lands (a cast Field Spell or Trap with no zone is the cast pipeline's, §10.5, R138).
fn room_to_cast(state: &GameState, player: PlayerId, card: &CardInstance) -> bool {
    crate::faces::card_type_of(state, card) != CardType::Unit
        || crate::zones::first_free_zone(state, player, Row::Units).is_some()
}

// Drawing across a prompt (§9.3, §10.6, R113, R117, R122)

/// §2.4's two loops can each pause on a prompt: the cast-on-draw chain in `complete_draw` (a cast is a
/// whole play, R70, and a play can ask) and the "draw N" loop in `draw` (each draw its own chain, R58;
/// #95 Call to Chaos). Choices are state (§9.3), so each loop gates on `work::paused` and parks what it
/// owes for the answer's drain (R113, R122): `DRAW_CHAIN_WORK` one more draw at `chain`, so R58's cap
/// bounds a split chain; `DRAW_COUNT_WORK` `count` whole draws. R117: parked at the pause, never ahead,
/// so a loop inside one (a cast's Cry) can neither take nor re-run them. `work::push_work` parks at
/// `state.work_cursor`, so the chain finishes before the next draw (R113); a paused draw replays
/// exactly (§10.1). An ended game (R3) parks nothing.
pub const DRAW_CHAIN_WORK: &str = "@drawChain";

const DRAW_CHAIN_STEP: &str = "chain";

/// R58: one more draw continuing a chain, and the count it must continue from. `owns` is set when
/// the chain is the one a draw of its own began, rather than one a draw inside some cast of it
/// continues (R217), so the item that finishes it is the one that closes `state.cast_chain`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct OwedDrawChain {
    pub player: PlayerId,
    pub chain: i32,
    pub owns: bool,
}

/// Where a draw stands in a cast-on-draw chain: the casts the chain has made so far, and whether
/// this draw's chain is its own to close (R217). A draw that continues a chain after a cast passes
/// its link on; a fresh draw — "draw N", a Combo draw, #30's named draw — reads it off the state.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ChainLink {
    pub chain: i32,
    pub owns: bool,
}

/// The link a repeat passes on, or a count given on its own (a caller starting a chain part-way, as
/// the engine's tests do).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChainLinkOrCount {
    Link(ChainLink),
    Count(i32),
}

impl From<ChainLink> for ChainLinkOrCount {
    fn from(link: ChainLink) -> ChainLinkOrCount {
        ChainLinkOrCount::Link(link)
    }
}

impl From<i32> for ChainLinkOrCount {
    fn from(count: i32) -> ChainLinkOrCount {
        ChainLinkOrCount::Count(count)
    }
}

/// R217: a fresh draw made while a chain is running — by one of the chain's casts, whose Combo draw
/// or own "draw 1" is part of the cast — continues that chain's count instead of starting its own, so
/// R58's cap bounds everything one draw sets off. Otherwise it begins a chain of its own at 0.
fn link_for(state: &GameState, link: Option<ChainLinkOrCount>) -> ChainLink {
    if let Some(ChainLinkOrCount::Link(link)) = link {
        return ChainLink {
            chain: state.cast_chain.unwrap_or(link.chain),
            owns: link.owns,
        };
    }
    // A count given on its own is a chain of its own at that count, unless a running one takes it over.
    let given = match link {
        Some(ChainLinkOrCount::Count(count)) => count,
        _ => 0,
    };
    ChainLink {
        chain: state.cast_chain.unwrap_or(given),
        owns: state.cast_chain.is_none(),
    }
}

/// The draw that began a chain has finished it, or the game ended under it: nothing is running.
fn close_chain(state: &mut GameState, link: &ChainLink) {
    if link.owns {
        state.cast_chain = None;
    }
}

pub const DRAW_COUNT_WORK: &str = "@drawCount";

const DRAW_COUNT_STEP: &str = "draws";

/// §2.4: whole draws a "draw N" still owes, each of which starts its own chain.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct OwedDrawCount {
    pub player: PlayerId,
    pub count: i32,
}

/// The open prompt's id: a prompt id (`q<n>`) is never handed out twice, so a different id is a
/// different prompt.
fn pending_id(state: &GameState) -> Option<String> {
    state.pending.as_ref().map(|pending| pending.id.clone())
}

/// Whether a draw loop must stop and park the rest: `work::paused`, but by R117 only for a pause *it*
/// caused (as `resolve::apply_hook_resumable` compares): §2.1's mulligan draws under an open prompt
/// (`setup::answer_mulligan` clears `state.pending` after R9's replacement draws), and stopping on it
/// would owe those draws to an action that never makes them. The game ending stops every sequence.
fn stopped(sink: &EngineSink, before: &Option<String>) -> bool {
    if !crate::work::paused(sink) {
        return false;
    }
    if sink.state.result.is_some() {
        return true;
    }
    pending_id(sink.state) != *before
}

/// It came back through JSON (§10.1), so nothing about the payload is assumed.
fn player_of(data: &IndexMap<String, Value>) -> Option<PlayerId> {
    match data.get("player").and_then(Value::as_str) {
        Some("p1") => Some(PlayerId::P1),
        Some("p2") => Some(PlayerId::P2),
        _ => None,
    }
}

fn count_of(data: &IndexMap<String, Value>, key: &str) -> i32 {
    match data.get(key).and_then(Value::as_f64) {
        Some(value) if value.is_finite() => value.trunc().max(0.0) as i32,
        _ => 0,
    }
}

/// What a `DRAW_CHAIN_WORK` item owes, or `None` when it is not one: the reader for its payload.
pub fn owed_draw_chain_of(resume: &Resume) -> Option<OwedDrawChain> {
    if resume.hook != DRAW_CHAIN_WORK {
        return None;
    }
    let player = player_of(&resume.data)?;
    // An item parked before R217 carries no `owns`, and its chain was always its own draw's.
    let owns = resume.data.get("owns") != Some(&Value::Bool(false));
    Some(OwedDrawChain {
        player,
        chain: count_of(&resume.data, "chain"),
        owns,
    })
}

/// What a `DRAW_COUNT_WORK` item owes, or `None` when it is not one.
pub fn owed_draw_count_of(resume: &Resume) -> Option<OwedDrawCount> {
    if resume.hook != DRAW_COUNT_WORK {
        return None;
    }
    let player = player_of(&resume.data)?;
    Some(OwedDrawCount {
        player,
        count: count_of(&resume.data, "count"),
    })
}

/// Park the rest of a chain (R113). `work.rs` owns `state.work`, so this only ever calls `owe`: the
/// item lands at `state.work_cursor`, which puts it behind anything the pausing cast parked inside it
/// — its own Cry tail, §10.5 steps 6 and 7 — and in front of everything else still owed.
fn owe_chain(sink: &mut EngineSink, player: PlayerId, link: &ChainLink) {
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert("player".to_string(), Value::String(player.as_str().to_string()));
    data.insert(
        "chain".to_string(),
        Value::from(sink.state.cast_chain.unwrap_or(link.chain)),
    );
    data.insert("owns".to_string(), Value::Bool(link.owns));
    let resume = Resume {
        def_id: String::new(),
        hook: DRAW_CHAIN_WORK.to_string(),
        step: DRAW_CHAIN_STEP.to_string(),
        radiant: false,
        instance_id: None,
        data,
    };
    crate::work::owe(sink, resume);
}

/// A chain stopped where it stands: owed to the answer, or closed for a game that is over.
fn stop_chain(sink: &mut EngineSink, player: PlayerId, link: &ChainLink) {
    if sink.state.result.is_none() {
        owe_chain(sink, player, link);
    } else {
        close_chain(sink.state, link);
    }
}

/// Park the whole draws a "draw N" has not made yet (R113), behind the chain that interrupted it.
fn owe_draws(sink: &mut EngineSink, player: PlayerId, count: i32) {
    if count <= 0 {
        return;
    }
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert("player".to_string(), Value::String(player.as_str().to_string()));
    data.insert("count".to_string(), Value::from(count));
    let resume = Resume {
        def_id: String::new(),
        hook: DRAW_COUNT_WORK.to_string(),
        step: DRAW_COUNT_STEP.to_string(),
        radiant: false,
        instance_id: None,
        data,
    };
    crate::work::owe(sink, resume);
}

/// §2.4's draw once the card has left the library: the draw counter, `drawn`, R58's cast-on-draw chain
/// and R4's hand cap. `draw_one` and `draw_from_library` (a NAMED card: #30 Archivist, #94 Genn's Greed)
/// both end here (§6.3 Draw). The caller splices the card out first, or a cast-on-draw card's own
/// script would read a library that still holds it.
pub fn complete_draw(
    sink: &mut EngineSink,
    player: PlayerId,
    mut card: CardInstance,
    link: Option<ChainLinkOrCount>,
) -> DrawOutcome {
    sink.state.counters.drawn += 1;
    // B5 E4, R457: the draw's number this turn rides the event, so a trap answering "the 2nd card they
    // draw in a turn" (Classic #9) reads it however much later the loop hands it the event.
    let counted = count_draw(sink.state, player);
    // B5 E33: the draw that took the last card of the drawer's own library (Classic #90's quest 9); the
    // card still says where it lay, so a draw out of the other player's library (E16) empties nothing.
    let emptied = matches!(card.zone, Zone::Library { player: owner } if owner == player)
        && sink.state.players[player].library.is_empty();
    sink.events.push(GameEvent::Drawn {
        player,
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        turn_draw: counted,
        emptied: if emptied { Some(true) } else { None },
    });

    let at = link_for(sink.state, link);
    // R2533000: a seat that plays only the tutorial's cards casts no other card on draw; it is drawn as
    // any card is.
    if casts_on_draw(sink.state, &card)
        && !crate::state::barred_from_play(sink.state, player, &card)
        && at.chain < CAST_ON_DRAW_CHAIN_CAP
        && sink.frontier.get().cast_on_draw_casts < CAST_ON_DRAW_ACTION_CAP
        && room_to_cast(sink.state, player, &card)
    {
        // R58, R217: counted before the cast resolves, so a draw the cast makes continues from here.
        sink.state.cast_chain = Some(at.chain + 1);
        // R2067000: and counted against the action's cap, whichever chain it is in.
        sink.frontier.get_mut().cast_on_draw_casts += 1;
        card.zone = Zone::Resolving { player };
        // R58: the draw is complete once this cast has resolved, and its `drawn` is answered then.
        crate::draw_complete::hold_draw(sink.state, &card.id);
        let before = pending_id(sink.state);
        let mut data: IndexMap<String, Value> = IndexMap::new();
        data.insert(
            crate::draw_complete::CAST_ON_DRAW_KEY.to_string(),
            Value::String(card.id.clone()),
        );
        crate::resolve::cast_card(
            sink,
            &card,
            crate::resolve::CastOptions {
                data: Some(data),
                ..Default::default()
            },
        );

        // §9.3 and R122: the cast is a whole play and a play can ask, so the draw's repeat belongs to the
        // action that answers; drawing on would fill the hand while the player is still being asked. One
        // more draw is owed at the chain's count, so R58's cap bounds it as an unbroken chain (R113, R117).
        if stopped(sink, &before) {
            stop_chain(sink, player, &at);
            return DrawOutcome::Cast;
        }

        continue_chain(sink, player, at.owns, &before);
        return DrawOutcome::Cast;
    }

    if add_to_hand(sink, &mut card) == AddToHandOutcome::Burned {
        DrawOutcome::Burned
    } else {
        DrawOutcome::Drawn
    }
}

/// R748: a cast-on-draw card setup dealt to a hand uncast is cast at the start of the game as its draw
/// would have, held as a draw until it resolves (R58, `cast_on_draw_now`); nothing repeats and no chain
/// begins (R217). One that no longer casts on draw, or a Unit with no open zone (R459), stays: false.
/// The state check follows the cast (R59), unless the cast asked, when the answer's drain owes it.
pub fn cast_dealt_card(sink: &mut EngineSink, card: &CardInstance) -> bool {
    let Zone::Hand { player } = card.zone else {
        return false;
    };
    if !casts_on_draw(sink.state, card)
        || !room_to_cast(sink.state, player, card)
        // R2533000: nor one dealt to it.
        || crate::state::barred_from_play(sink.state, player, card)
    {
        return false;
    }
    let hand = &mut sink.state.players[player].hand;
    let Some(at) = hand.iter().position(|held| held.id == card.id) else {
        return false;
    };
    let mut taken = hand.remove(at);
    taken.zone = Zone::Resolving { player };
    crate::draw_complete::hold_draw(sink.state, &taken.id);
    let before = pending_id(sink.state);
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(
        crate::draw_complete::CAST_ON_DRAW_KEY.to_string(),
        Value::String(taken.id.clone()),
    );
    crate::resolve::cast_card(
        sink,
        &taken,
        crate::resolve::CastOptions {
            data: Some(data),
            ..Default::default()
        },
    );
    if !stopped(sink, &before) {
        crate::state_check::state_check(sink);
    }
    true
}

/// §2.4's repeat, after a cast-on-draw cast has resolved whole (§10.5, R70). §4.5 and R59 run the state
/// check "after one cast-on-draw cast", so a hero it brought to 0 ends the game before the draw repeats,
/// and a Death hook that asks stops the chain like any pause. The chain's count is read off the state,
/// since the cast may have drawn and cast more of it itself (R217).
fn continue_chain(sink: &mut EngineSink, player: PlayerId, owns: bool, before: &Option<String>) {
    let link = ChainLink {
        chain: sink.state.cast_chain.unwrap_or(0),
        owns,
    };
    crate::state_check::state_check(sink);
    if stopped(sink, before) {
        stop_chain(sink, player, &link);
        return;
    }
    let chain = sink.state.cast_chain.unwrap_or(link.chain);
    draw_one(
        sink,
        player,
        Some(ChainLinkOrCount::Link(ChainLink { chain, owns })),
    );
    // A pause further down the chain parked its own repeat with `owns`, and that item closes it.
    if !stopped(sink, before) || sink.state.result.is_some() {
        close_chain(sink.state, &link);
    }
}

/// One draw (§2.4). A cast-on-draw card resolves at once and the draw repeats, up to
/// CAST_ON_DRAW_CHAIN_CAP casts (R58); the next such card goes to hand uncast and ends the chain.
/// `link` is the chain a repeat continues; a fresh draw passes none (R217).
pub fn draw_one(sink: &mut EngineSink, player: PlayerId, link: Option<ChainLinkOrCount>) -> DrawOutcome {
    // B5 E3, R457: a draw past the player's limit this turn does not happen at all.
    if draw_blocked(sink, player) {
        return DrawOutcome::Limited;
    }

    if sink.state.players[player].library.is_empty() {
        if let Some(source) = infinite_reserves_source(sink, player) {
            let token_def_id = crate::catalog::def_id_by_index(SetName::Core, "T-rush");
            if let Some(token_def_id) = token_def_id {
                let mut token = new_instance(sink.state, &token_def_id, player, Zone::Hand { player });
                crate::generators::record_generator_of(sink.state, &source, &token);
                sink.state.counters.drawn += 1;
                let counted = count_draw(sink.state, player);
                sink.events.push(GameEvent::Drawn {
                    player,
                    instance_id: token.id.clone(),
                    def_id: token.def_id.clone(),
                    turn_draw: counted,
                    emptied: None,
                });
                add_to_hand(sink, &mut token);
                return DrawOutcome::Token;
            }
        }
        // No `drawn`: the damage instance is what happened (§2.4, R3). R315: `fatigue` announces it
        // first, so the board shows the empty library before the hit; a report answering nothing (R1362).
        sink.state.players[player].fatigue_count += 1;
        let fatigue_count = sink.state.players[player].fatigue_count;
        // R457: a draw from an empty library is a draw that happened, and counts toward a limit.
        count_draw(sink.state, player);
        let amount = FATIGUE_DAMAGE(fatigue_count);
        sink.events.push(GameEvent::Fatigue {
            player,
            count: fatigue_count,
            amount,
        });
        // R240, R1362: a fatigue draw whose hit the hero's Armor takes whole (§4.4 step 2) still happened
        // (the count moved, §10.3); the pipeline reports it by `damageAbsorbed` from no source (R1361),
        // no damage instance (R63), in the place in R68's order R240's `damage` of 0 had.
        let before = sink.events.len();
        let dealt = crate::damage::deal_damage(
            sink,
            crate::damage::DamageArgs {
                source: None,
                target: crate::damage::DamageTarget::Hero { player },
                amount,
                flags: None,
            },
        );
        // The pipeline's report is the last thing the hit emitted (`damage::land_hit`).
        let at = sink.events.len().saturating_sub(1);
        if dealt <= 0
            && sink.state.result.is_none()
            && at >= before
            && matches!(sink.events.get(at), Some(GameEvent::DamageAbsorbed { .. }))
        {
            crate::triggers::order_report(sink, at);
        }
        return DrawOutcome::Fatigue;
    }

    let card = sink.state.players[player].library.remove(0);
    complete_draw(sink, player, card, link)
}

/// §2.4: "Draw N is N separate draws, each with its own chain" (R58). A draw that pauses stops the
/// rest, owed to `state.work` for the action that answers (R113, R122), so this returns only the draws
/// that really happened in this action.
pub fn draw(sink: &mut EngineSink, player: PlayerId, count: i32) -> Vec<DrawOutcome> {
    let before = pending_id(sink.state);
    let mut out: Vec<DrawOutcome> = Vec::new();
    let mut i = 0;
    while i < count {
        // R216: a draw whose cast, or the check after it, ended the game ends the draws with it.
        if sink.state.result.is_some() {
            return out;
        }
        out.push(draw_one(sink, player, None));
        if !stopped(sink, &before) {
            i += 1;
            continue;
        }
        // R117: parked here, at the pause, behind the chain's own remainder, so that chain finishes first.
        if sink.state.result.is_none() {
            owe_draws(sink, player, count - i - 1);
        }
        return out;
    }
    out
}

/// `work.rs`'s handler for a chain a cast-on-draw prompt split (`DRAW_CHAIN_WORK`): the same chain at
/// the same count. The cast that paused has finished (its items were parked ahead, R113), so its owed
/// state check runs before the draw repeats, as in an unbroken chain.
pub fn run_owed_draw_chain(sink: &mut EngineSink, item: &WorkItem) {
    let Some(owed) = owed_draw_chain_of(&item.resume) else {
        return;
    };
    // The count is the state's while the chain runs; an item parked before R217 kept its own.
    if sink.state.cast_chain.is_none() && owed.owns {
        sink.state.cast_chain = Some(owed.chain);
    }
    let before = pending_id(sink.state);
    continue_chain(sink, owed.player, owed.owns, &before);
}

/// `work.rs`'s handler for the whole draws a "draw N" still owed when one of them paused
/// (`DRAW_COUNT_WORK`).
pub fn run_owed_draw_count(sink: &mut EngineSink, item: &WorkItem) {
    let Some(owed) = owed_draw_count_of(&item.resume) else {
        return;
    };
    draw(sink, owed.player, owed.count);
}

// `work.rs`'s dispatcher names the two handlers above directly (SURFACE §6.6), never a test: a suite
// that wired them up on production's behalf would be green over a production with no wiring (R113).
