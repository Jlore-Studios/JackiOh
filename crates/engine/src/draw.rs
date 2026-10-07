//! Drawing, fatigue, the hand cap, cast-on-draw chains and the library cap
//! (SPEC §2.4, R3, R4, R58, R80), the arrival hook of R151, and — since a cast-on-draw card is a
//! whole play and a play can ask — both draw loops resuming out of `state.work` across a prompt
//! (§9.3, §10.6, R113, R117, R122). Patch v0.2.0 adds the draws a player makes each turn (B5 E4), the
//! draw limits a card on the field sets (E3), the `castOnDraw` enchantment (E39) and a Unit cast on draw
//! with nowhere to stand (R457, R459).
//!
//! Port of `packages/engine/src/draw.ts`. TS registered the two loops' work handlers at module scope
//! (`registerWorkHandler`); here `work.rs`'s dispatcher matches `DRAW_CHAIN_WORK` to
//! `run_owed_draw_chain` and `DRAW_COUNT_WORK` to `run_owed_draw_count` (SURFACE §6.6), which are `pub`
//! for it.
//!
//! TS handed the live card object around and wrote through it. Here a card that is moved is passed as
//! `&mut CardInstance` (its zone follows the move, as TS's object did) or by value when the caller has
//! already taken it out of its pile (`complete_draw`), and a later write to the card where it now lies
//! goes through `state::find_instance_mut`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{CAST_ON_DRAW_CHAIN_CAP, FATIGUE_DAMAGE, HAND_CAP, LIBRARY_CAP, SETUP_TURN};
use crate::script::{DrawLimit, DrawLimitPlayer, EngineSink, HookArgs};
use crate::state::{CardInstance, DrawCount, GameState, Resume, WorkItem, find_instance_mut, new_instance};
use crate::wire::{
    CardType, Enchantment, GameEvent, LibraryOverflowOutcome, PLAYER_IDS, PlayerId, Row, SetName, Zone,
    opponent_of,
};

/// R151: a card whose script has a start-of-game hook runs it when it ARRIVES in a hand or a
/// library, and not only at §2.1 step 4. R43 words the rule as an invariant rather than as a moment —
/// "at start of game every Heroic Power in either player's hand or library rolls its power … and one
/// that ends up in a hand or library with no `memory.power` (a bounced or reset instance, R78) rolls
/// as it arrives" — so a copy that reaches a hand later (#72 Reminisce out of a graveyard, a bounce,
/// a draw, a card an effect created) has to roll too, or it carries no power and costs 0 for ever.
///
/// GENERAL, NOT #98 BY ID: the test is the script's `start_of_game`, the same thing
/// `setup::finish_setup` reads, so any card that ever grows a start-of-game clause is covered without
/// the engine naming a catalog id. Re-running it is safe because the hook is the one that owns its own
/// idempotence: `hero_power::ensure_power` keeps a power a card already rolled ("which is why it is
/// idempotent rather than a plain roll"), so an arrival costs no rng draw once the card has its answer.
///
/// WHY HERE AND NOT IN `zones::move_to_zone`, the single point every zone change goes through: the roll
/// needs the match rng, and `move_to_zone` takes a `GameState`, which holds only the seed and the stored
/// cursor. A fresh `Rng::new(&state.seed, state.rng_cursor)` mid-action would repeat draws the action's
/// own rng has already taken, and `reduce` ends every action with the cursor written back from the
/// sink's rng, so the cursor it advanced would be discarded — two determinism bugs for one convenience.
/// These two functions are the sink-holding funnels §2.4 already routes every hand and library arrival
/// through (see the header of `effects/add_to_hand.rs`: "Both routes end in `../draw`'s `addToHand`").
fn run_arrival_hooks(sink: &mut EngineSink, instance: &CardInstance) {
    // §9.3, R113: the clause is an effect list like any other, so a question in it pauses the rest of
    // it on `state.work` (`prompts::run_start_of_game`), and a draw loop around it owes its remainder.
    crate::prompts::run_start_of_game(sink, instance, instance.owner);
}

/// #75: a backrow card that turns an empty-library draw into a Rush Token card.
fn infinite_reserves_source(sink: &EngineSink, player: PlayerId) -> Option<CardInstance> {
    for slot in crate::zones::slots_of(player, Row::Backrow) {
        if let Some(card) = crate::zones::card_at(sink.state, slot)
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

/// Where `add_to_hand` put a card (TS `"hand" | "burned"`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum AddToHandOutcome {
    Hand,
    Burned,
}

/// §2.4, R4: a card entering a full hand is burned to the graveyard; unit tokens vanish (R11).
pub fn add_to_hand(sink: &mut EngineSink, instance: &mut CardInstance) -> AddToHandOutcome {
    let full = sink.state.players[instance.owner].hand.len() as i32 >= HAND_CAP;
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
    });
    // R151: it has arrived somewhere it can be looked at, so a start-of-game clause runs now.
    run_arrival_hooks(sink, instance);
    AddToHandOutcome::Hand
}

/// Where `shuffle_into_library` put a card (TS `"library" | "dropped"`).
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

/// R80: a library holds at most LIBRARY_CAP cards, so copies stop being created at the cap, and an
/// existing card goes to its owner's graveyard instead (a unit-token card ceases to exist, R11).
///
/// R316: whichever it is, the refusal is reported by `libraryOverflow`, so the board can show the
/// full library turning the card away. Nothing else reported a copy that was never made, and an
/// existing unit-token card that ceased to exist here moved with no event at all (§10.3). `copy_of`
/// names the card a new copy copies, when it copies one, so a view can keep a face-down trap's copy
/// as secret as the trap.
pub fn shuffle_into_library(
    sink: &mut EngineSink,
    instance: &mut CardInstance,
    existing: bool,
    copy_of: Option<&str>,
) -> ShuffleInOutcome {
    let library_len = sink.state.players[instance.owner].library.len() as i32;
    if library_len >= LIBRARY_CAP {
        if !existing {
            sink.events.push(library_overflow(
                instance,
                LibraryOverflowOutcome::NotCreated,
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
    let position = sink.rng.int(library_len + 1);
    crate::zones::move_to_zone(
        sink.state,
        instance,
        crate::zones::OffFieldZone::Library,
        crate::zones::MoveToZoneOptions {
            position: Some(crate::zones::LibraryPosition::At(position)),
            ..Default::default()
        },
    );
    // R311: a shuffle-in is open to the library's owner. Every Core one goes in by a card its owner
    // watched resolve — a CN-Virus's copies, an Unstable Clone Machine's (of a card the owner played,
    // a face-down Trap included), the opponent's CN-Viral Injection, whose text names what it
    // shuffles — so the owner knows what went in, though never where (the slot below stays hidden
    // from both, R97). (TS wrote through the card now lying in the library; so does this.)
    if let Some(live) = find_instance_mut(sink.state, &instance.id) {
        crate::own_library::show_to_owner(live);
    }
    crate::own_library::show_to_owner(instance);
    sink.events.push(GameEvent::ShuffledIn {
        player: instance.owner,
        instance_id: instance.id.clone(),
        def_id: instance.def_id.clone(),
        position,
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

// ---------------------------------------------------------------------------
// ---- v0.2.0: activate and turn (B5 E3 draw limits, E4 draw counts, E39 cast on draw) ----
// ---------------------------------------------------------------------------

/// B5 E4, R457: the draws `player` has made this turn, whoever's turn it is — every draw that happened,
/// a fatigue draw included, and none a limit stopped. `PlayerState.draws` is kept per turn, so a count
/// from an earlier turn reads as 0: it resets where the turn log does (§2.2). Setup is no player's turn
/// (§2.1, `SETUP_TURN`), so the opening deal and the mulligan's draws count toward nothing.
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

/// The cards whose static text is in force: the top of each unit pile and each backrow card, both
/// sides — the cards an aura is read from (§10.4 layer 5). A Vanilla card carries no text (§6.3).
fn text_on_field(state: &GameState, player: PlayerId) -> Vec<&CardInstance> {
    let mut out: Vec<&CardInstance> = Vec::new();
    for unit in crate::zones::active_units_of(state, player) {
        out.push(unit);
    }
    for slot in crate::zones::slots_of(player, Row::Backrow) {
        if let Some(card) = crate::zones::card_at(state, slot) {
            out.push(card);
        }
    }
    out
}

fn limit_binds(limit: &DrawLimit, controller: PlayerId, player: PlayerId) -> bool {
    if limit.player == DrawLimitPlayer::Both {
        return true;
    }
    if limit.player == DrawLimitPlayer::SelfSide {
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

/// B5 E3, R457: whether `player`'s next draw may not happen — they have made as many draws this turn
/// as their limit allows. A stopped draw does not happen at all: no card moves, no fatigue, nothing is
/// cast, it is not counted, and `drawLimited` (public, it names no card) is all that is said. Every
/// draw asks this before it takes its card: `draw_one`, a named draw (`effects::draw::draw_from_library`)
/// and a draw out of the opponent's deck (B5 E16), which then hands the card to `complete_draw`.
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

/// Whether the card carries the `castOnDraw` enchantment (TS `enchantments.hasEnchantment`; a private
/// copy).
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
/// zone (R64, R70); with none open it is not cast and goes to the hand, as R58's cap sends a card
/// that is not cast. Any other card is cast wherever it lands (a Spell resolves; a cast Field Spell or
/// Trap with no zone is the cast pipeline's to settle, §10.5, R138).
fn room_to_cast(state: &GameState, player: PlayerId, card: &CardInstance) -> bool {
    crate::faces::card_type_of(state, card) != CardType::Unit
        || crate::zones::first_free_zone(state, player, Row::Units).is_some()
}

// ---------------------------------------------------------------------------
// Drawing across a prompt (§9.3, §10.6, R113, R117, R122)
// ---------------------------------------------------------------------------

/// §2.4 has two loops, and a prompt can open in the middle of either one.
///
///   * the cast-on-draw CHAIN inside `complete_draw`: the cast is a whole play (R70), and a play can
///     ask — #7 Jewelosco Scarab's Discover drawn off the top, or anything Call to Chaos reaches.
///   * the "draw N" loop inside `draw`: N separate draws, each with its own chain (§2.4, R58), so
///     draw 2 with #95 Call to Chaos is two chains and the first of them can pause.
///
/// Both used to walk straight on over the open prompt, drawing cards into a game state the player had
/// not finished deciding — the same class of bug as any sequence that keeps its place in a local
/// variable (§9.3: "mid-action choices are state, not callbacks"). So each loop gates on
/// `work::paused` and parks what it still owes on `state.work`, which the answer's drain picks up
/// (R113, R122). Two kinds rather than one, because they are two different remainders and each
/// handler is then exactly its own loop:
///
///   * `DRAW_CHAIN_WORK` owes "one more draw, continuing this chain at `chain`" — one item carrying
///     the counter, so R58's cap still bounds the chain a pause split in half. A resumed chain counts
///     on from where it stopped and can never restart at zero, which is what would let a chain evade
///     the cap by pausing.
///   * `DRAW_COUNT_WORK` owes "`count` more whole draws", each starting a fresh chain at 0.
///
/// R117: both are parked at the moment of the pause and never in advance. While the loop is on the
/// stack the draws it has not made are the loop's alone, so a resolution loop running *inside* one of
/// them — a cast's Cry can start one — can neither take nor re-run the draws it is standing in.
/// Pre-parking the remainder is what made a played card's Cry fire twice earlier in this project.
///
/// R113's order falls out of `work::push_work` placing at `state.work_cursor`: a chain that pauses parks
/// its own remainder first and the enclosing "draw N" loop parks after it, so the interrupted chain
/// finishes before the next whole draw begins. Nothing but plain JSON is held, so a paused draw
/// survives a JSON round trip and replays exactly (§9.3, §10.1).
///
/// The game ending parks nothing: like `traps::owe_window` and `resolve::cast_card`, there is nothing
/// left to resume into once `state.result` is set, and R3's fatigue is the usual way a draw ends one.
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

/// TS `ChainLink | number`: the link a repeat passes on, or a count given on its own (a caller
/// starting a chain part-way, as the engine's tests do).
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
    // A count given on its own (a caller starting a chain part-way, as the engine's tests do) is a
    // chain of its own at that count, unless a running one takes it over.
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

/// The open prompt's id, standing for TS's object identity in `stopped`'s `pending !== before`: a
/// prompt id (`q<n>`) is never handed out twice, so a different id is a different prompt.
fn pending_id(state: &GameState) -> Option<String> {
    state.pending.as_ref().map(|pending| pending.id.clone())
}

/// Whether a draw loop must stop where it stands and park the rest. `work::paused` is the test — a
/// prompt is open, or the game is over — with the one qualification R117 already implies: the pause a
/// sequence owes its remainder for is the one *it* caused.
///
/// That qualification is load-bearing here and nowhere else in the engine, because §2.1's mulligan is
/// the one caller that draws underneath an open prompt: `setup::answer_mulligan` draws the replacements
/// and only then clears `state.pending`, since R9 wants the replacements drawn before the returned
/// cards are shuffled back. A prompt that was already open when this draw began is not this draw's
/// pause, and stopping on it would owe R9's replacement draws to an action that never makes them.
/// The before/after comparison is the same one `resolve::apply_hook_resumable` makes across the effects
/// of a single hook, for the same reason.
///
/// The game ending is unconditional: `state.result` stops every sequence, and `traps::owe_window` and
/// `resolve::cast_card` both park nothing past it, because there is nothing left to resume into.
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

/// §2.4's draw from the moment the card has left the library: the game draw counter, the `drawn`
/// event, R58's cast-on-draw chain and R4's hand cap.
///
/// `draw_one` below takes the top card and `effects/draw.rs`'s `draw_from_library` takes a NAMED one
/// (#30 Archivist's highest and lowest, #94 Genn's Greed's every 2-cost card), and both end here, so
/// "a draw" means exactly one thing however the card was chosen (§6.3 Draw). The caller splices the
/// card out of the library first (and hands it over): the pile must already be short by one when a
/// cast-on-draw card resolves, or its own script would read a library that still holds it.
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
    // B5 E33: the draw that took the last card of the drawer's own library (Classic #90's quest 9). The
    // card still says where it lay (the caller spliced it out and has not moved it), so a draw out of
    // the other player's library (E16) empties nothing of the drawer's.
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
    if casts_on_draw(sink.state, &card)
        && at.chain < CAST_ON_DRAW_CHAIN_CAP
        && room_to_cast(sink.state, player, &card)
    {
        // R58, R217: counted before the cast resolves, so a draw the cast makes continues from here.
        sink.state.cast_chain = Some(at.chain + 1);
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

        // §9.3 and R122: the cast is a whole play and a play can ask, so the repeat of the draw belongs
        // to the action that answers, not to this one. Drawing on here would put cards in the hand — and
        // cast more of them — while the player is still being asked about this one. What is owed is one
        // more draw at the chain's count, which is precisely the count this draw would have passed on, so
        // R58's cap bounds the resumed chain exactly as it bounds an uninterrupted one (R113, R117).
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

/// R748: a cast-on-draw card setup dealt to a hand uncast, cast at the start of the game as its draw
/// would have cast it: out of the hand into the resolving zone and through the cast, held as a draw
/// until the cast resolves, so the card knows it is cast on draw (R58, `cast_on_draw_now`). Its draw was
/// made in setup, so nothing repeats and no chain begins (R58, R217). A card that no longer casts on
/// draw, or a Unit with no open unit zone (R459), stays in the hand, and this returns false. The state
/// check follows the cast (R59), unless the cast asked, when the answer's drain owes it.
///
/// The card is the one in its hand with this card's id, which is taken out of the hand and cast.
pub fn cast_dealt_card(sink: &mut EngineSink, card: &CardInstance) -> bool {
    let Zone::Hand { player } = card.zone else {
        return false;
    };
    if !casts_on_draw(sink.state, card) || !room_to_cast(sink.state, player, card) {
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

/// §2.4's repeat, after a cast-on-draw cast has resolved whole — its Echo repeats and its landing
/// included, since a cast is §10.5's pipeline (R70). §4.5 and R59 run the state check "after one
/// cast-on-draw cast", so a hero the cast brought to 0 ends the game here and the draw does not
/// repeat into the card beneath, and a unit it killed has died before the next card is cast. A Death
/// hook that asks stops the chain like any other pause, owing the draw it would have made.
///
/// The repeat reads the chain's count off the state, not the count this draw passed on: the cast may
/// have drawn and cast more of the same chain itself (R217). The draw that began the chain closes it
/// once its repeat is done.
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
        if infinite_reserves_source(sink, player).is_some() {
            let token_def_id =
                crate::catalog::def_by_index(SetName::Core, "T-rush").map(|def| def.id.clone());
            if let Some(token_def_id) = token_def_id {
                let mut token = new_instance(sink.state, &token_def_id, player, Zone::Hand { player });
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
        // No card is drawn, so no `drawn` event: the damage instance is what happened (§2.4, R3).
        // R315: `fatigue` announces it first, so the board shows the empty library before the hit
        // lands; it is a report and answers nothing, like R240's zero-damage one below.
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
        let dealt = crate::damage::deal_damage(
            sink,
            crate::damage::DamageArgs {
                source: None,
                target: crate::damage::DamageTarget::Hero { player },
                amount,
                flags: None,
            },
        );
        // R240: a fatigue draw whose hit the hero's Armor takes whole (§4.4 step 2; step 3's cap only
        // clamps) still happened — the public count moved and the next one deals more (§10.3) — so it is
        // reported by a hit of 0 from no source, which is a report and no damage instance: nothing
        // answers it (R63, `triggers::dispatch_event`).
        if dealt <= 0 && sink.state.result.is_none() {
            sink.events.push(GameEvent::Damage {
                source_id: None,
                target_id: format!("hero-{player}"),
                amount: 0,
                combat: false,
            });
        }
        return DrawOutcome::Fatigue;
    }

    let card = sink.state.players[player].library.remove(0);
    complete_draw(sink, player, card, link)
}

/// §2.4: "Draw N is N separate draws, each with its own chain" (R58). A draw that pauses stops the
/// rest of them, which are owed to `state.work` and made by the action that answers (R113, R122) —
/// so the outcomes this returns are the draws that really happened in this action, and a caller that
/// counts them reads a short list rather than a list of draws that have not happened yet.
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
        // R117: parked here, at the pause, and never in advance. The chain that stopped has already
        // parked its own remainder, so this lands behind it and the interrupted chain finishes first.
        if sink.state.result.is_none() {
            owe_draws(sink, player, count - i - 1);
        }
        return out;
    }
    out
}

/// `work.rs`'s handler for a chain a cast-on-draw prompt split (`DRAW_CHAIN_WORK`): the same chain, at
/// the same count. The cast that paused has finished by now — its own items were parked ahead of this
/// one (R113) — so the state check it is owed runs before the draw repeats, as it does in an unbroken
/// chain.
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

// Registered at module scope in TS, in the module that owns the sequence, and never from a test: a
// handler is code, so a suite that wired it up on production's behalf would be green over a
// production that had no wiring at all (R113). In Rust `work.rs`'s dispatcher names the two handlers
// above directly (SURFACE §6.6).
