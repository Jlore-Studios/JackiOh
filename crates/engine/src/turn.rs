//! The turn loop of SPEC §2.2 in R62's order, cleanup, the turn cap and the ways a game ends
//! (§2.5, R79, R389). The sequence is fixed here; every part of it belongs to the module that owns it —
//! the start-of-turn and end-of-turn hooks to `triggers::queue_hooks_in_trigger_order` (R68's order,
//! drained by `triggers::settle`), the end-of-turn trap window to `traps::run_trap_window` (R62, R100),
//! the delayed effects to `modifiers::due_delayed` in creation order, and mana to `mana.rs`. Patch
//! v0.2.0 adds two start-of-turn stages between the refresh and the delayed effects — the Brittle tick
//! (`brittle::brittle_tick`, B3.3) and the "Animated on your turn" cards stepping into their unit zones
//! (`animated::animate_at_turn_start`, B3.1) — the rest-of-game start-of-turn effects among the delayed
//! ones (B5 E28, R458), and those cards going back home as cleanup's last step, after every
//! end-of-turn step (`animated::return_at_cleanup`). The start of a turn is therefore: refresh → Brittle
//! tick → animate → delayed effects → start-of-turn triggers → draw (R62).
//!
//! Every one of those parts can pause, because a trigger, a trap or a delayed effect may ask its
//! controller something (§9.3, §10.6). So BOTH turn boundaries are resumable sequences like any
//! other: at the moment one pauses it parks what is left of R62's order on `state.work` through
//! `owe` and returns, and `run_owed_start_of_turn` / `run_owed_end_of_turn` — which `work.rs`'s
//! dispatcher calls for `START_OF_TURN_WORK` and `END_OF_TURN_WORK` (TS registered them with
//! `work.registerWorkHandler` at module scope; SURFACE §6.6) — pick it up when the answer drains the
//! queue (R113, R117, R122). Walking on instead is what let a delayed effect resolve inside an
//! unfinished trap window and `cleanup` close the turn log before the window's last traps had fired,
//! and, at the other boundary, what had the turn's draw land inside an open start-of-turn prompt.
//!
//! The other half of §10.3 at a boundary is dispatch: a stage that emits events settles before the
//! next one runs, so the draw's events — for a Cast on draw, a whole play (R58) — reach the traps
//! and the trigger queue instead of sitting undispatched on the sink.
//!
//! Port of `packages/engine/src/turn.ts`.

use indexmap::{IndexMap, IndexSet};
use serde_json::Value;

use crate::config::{DRAW_OFFER_BLOCK_TURNS, DRAWS_PER_TURN, TURN_CAP_PLAYER_TURNS};
use crate::game_over::end_game;
use crate::resolve::HookName;
use crate::script::EngineSink;
use crate::state::{
    DelayedEffect, Exertion, GameState, ModifierKind, Resume, TurnLog, WorkItem, find_instance_mut,
    handicap_of,
};
use crate::wire::{GameEvent, GameOverReason, PLAYER_IDS, Phase, PlayerId, Winner, opponent_of};

/// R846 (Meditative #19.1): the id owed extra turns travel under — one badge per player, which
/// `modifierChanged` names as turns are owed and as the last one is taken, and which the view
/// lists while any is owed (R169).
pub const EXTRA_TURN_MODIFIER_ID: &str = "extraTurns";

/// `prompts.runResume(sink, resume, { controller })`.
fn run_resume_for(sink: &mut EngineSink, resume: &Resume, controller: PlayerId) {
    let _ = crate::prompts::run_resume(
        sink,
        resume,
        crate::prompts::ResumeOptions {
            controller: Some(controller),
            ..Default::default()
        },
    );
}

/// TS's `Number.POSITIVE_INFINITY` as a creation mark: every entry is due. A seq is a `u32`, so the
/// largest one stands in for it (no game makes 4 billion of anything).
const DUE_ALL: u32 = u32::MAX;

/// §2.2's two delayed-effect points, in R68's creation order (`modifiers::due_delayed`). An entry is
/// dropped before it runs, so it can never fire twice.
///
/// R126: a delayed continuation is re-entered exactly the way a prompt answer is. `resume.hook`
/// names a key of the card's `Script`, which may be a hook (`delayed`) or the step table (`resume`,
/// where `resume.step` picks the entry), and **one** reader resolves both shapes — which is why this
/// goes through `prompts::run_resume` and not `resolve::run_hook`. `runHook` resolves only the first
/// shape: it does `script[name]` and *calls* it, so `hook: "resume"` fetched the step-table object
/// and threw "not a function", and no card could keep its continuation where every other pause in
/// the engine keeps one.
///
/// R127: an entry whose instance is gone still resolves, named by its stored def id. `run_resume`
/// finds the instance itself and re-enters with `ctx.self_ == None`, the step reading what it needs
/// out of `resume.data` (§10.6, R76: #50 K-Pop Fanatic's steal fires after K-Pop Fanatic has died,
/// #39's exile after the card has exiled itself). The old `instanceId === undefined` skip dropped
/// that shape with no error at all, which is the silent loss of a sequence R113 forbids.
///
/// A step that opens a prompt stops the run where it stands (§9.3): its own tail is parked by
/// `prompts::apply_resumable`, the entries still due stay in `state.delayed`, and the caller parks the
/// rest of the turn boundary — which is what brings this function back for them.
fn run_delayed(sink: &mut EngineSink, phase: Phase, player: PlayerId, due_before: u32) {
    // R62, R68: the delayed effects due at this point are the ones that exist as it begins. One made
    // while the stage is resolving — by the answer to an earlier one's question, say — is due at the
    // next such point, as it is when nothing asks: `dueBefore` is the creation mark the stage began
    // at, which a pause carries to the step that picks the stage up (R113).
    for entry in due_entries(&*sink.state, phase, player, due_before) {
        // One entry at a time in R68's order: a prompt, or a game that has just ended, stops the run.
        if crate::work::paused(sink) {
            return;
        }
        if !run_due_entry(sink, player, &entry) {
            continue;
        }
        // R59: the check runs after the whole delayed effect, never between its parts. One that asked
        // is not whole yet — the answer finishes it — so the check is owed to the step that picks the
        // boundary up after it (`check_before_delayed`), before the next delayed effect runs (R174).
        if crate::work::paused(sink) {
            return;
        }
        if !check_after_delayed(sink) {
            return;
        }
    }
}

/// One entry of R62's delayed stage: a delayed effect (`state.delayed`), or at the start of a turn a
/// rest-of-game effect of that player's (B5 E28, R458), which is a delayed effect that comes due at
/// every start of their turn. Both are ordered by creation `seq` (R68).
struct DueEntry {
    seq: u32,
    what: Due,
}

enum Due {
    Delayed(Box<DelayedEffect>),
    /// The `startOfTurnEffect` modifier's id, found again on the player as it runs.
    Recurring {
        id: String,
    },
}

fn due_entries(state: &GameState, phase: Phase, player: PlayerId, due_before: u32) -> Vec<DueEntry> {
    // `modifiers.dueDelayed(state, phase, player)`: the entries due at this point of this player's
    // turn, `notBefore` passed, in creation order. (A private copy, fullsend builder rule 5.)
    let mut delayed: Vec<&DelayedEffect> = state
        .delayed
        .iter()
        .filter(|effect| {
            effect.at.phase == phase
                && effect.at.player == player
                && effect
                    .not_before
                    .is_none_or(|not_before| state.turn >= not_before)
        })
        .collect();
    delayed.sort_by_key(|a| a.seq);
    let mut entries: Vec<DueEntry> = delayed
        .into_iter()
        .filter(|due| due.seq < due_before)
        .map(|due| DueEntry {
            seq: due.seq,
            what: Due::Delayed(Box::new(due.clone())),
        })
        .collect();
    if phase == Phase::Start {
        // `modifiers.dueStartOfTurnEffects(state, player)`: the player's rest-of-game start-of-turn
        // effects that have not run this turn, in creation order. (A private copy, as above.)
        let mut recurring: Vec<(u32, String)> = state.players[player]
            .mods
            .iter()
            .filter_map(|held| match &held.kind {
                ModifierKind::StartOfTurnEffect { seq, ran_turn, .. } if *ran_turn != Some(state.turn) => {
                    Some((*seq, held.id.clone()))
                }
                _ => None,
            })
            .collect();
        recurring.sort_by_key(|a| a.0);
        entries.extend(
            recurring
                .into_iter()
                .filter(|(seq, _)| *seq < due_before)
                .map(|(seq, id)| DueEntry {
                    seq,
                    what: Due::Recurring { id },
                }),
        );
    }
    // A stable sort, as `Array.prototype.sort` (SURFACE §4.4.1).
    entries.sort_by_key(|a| a.seq);
    entries
}

/// Run one due entry, or pass it by (false) when it is no longer due. A delayed effect is dropped
/// before it runs, so it can never fire twice; a rest-of-game effect marks the turn it ran on
/// (`ranTurn`) for the same reason, since it stays for the next turn.
fn run_due_entry(sink: &mut EngineSink, player: PlayerId, entry: &DueEntry) -> bool {
    match &entry.what {
        Due::Delayed(effect) => {
            // R174: an earlier entry's resolution can end a later one — the state check after #50's first
            // steal kills a second steal's target, and `zones::forget_watchers` drops the entry aimed at it
            // even if Reborn brings the card straight back. The list was read before either ran, so an
            // entry is run only while `state.delayed` still holds it.
            // R1140: and it runs as it stands now, a hand watch with the cards it still watches.
            let Some(live) = sink.state.delayed.iter().find(|due| due.id == effect.id).cloned() else {
                return false;
            };
            // `modifiers.dropDelayed(state, effect.id)`.
            sink.state.delayed.retain(|due| due.id != effect.id);
            // B5 E27: the engine's own delayed kinds (a destroy, a hand discarded) run as verbs; every other
            // entry re-enters its card's step (R126).
            if !crate::effects::delay::run_engine_delayed(sink, &live) {
                run_resume_for(sink, &crate::effects::delay::due_resume(&live), live.owner);
            }
            true
        }
        Due::Recurring { id } => {
            let turn = sink.state.turn;
            let Some(held) = sink.state.players[player]
                .mods
                .iter_mut()
                .find(|held| held.id == *id)
            else {
                return false;
            };
            let ModifierKind::StartOfTurnEffect { resume, ran_turn, .. } = &mut held.kind else {
                return false;
            };
            if *ran_turn == Some(turn) {
                return false;
            }
            *ran_turn = Some(turn);
            let resume = resume.clone();
            // R127, R458: the player's effect now, re-entered by its def id with no instance.
            run_resume_for(sink, &resume, player);
            true
        }
    }
}

/// §10.3 after one whole delayed effect: its events go to the traps, which fire at once as responses
/// (a trap answering #50's first steal resolves before the second steal does), and then the state
/// check (R59). The other triggers they wake are queued and wait for the stage's own loop, in R68's
/// order behind every delayed effect due (`start_of_turn_settle`, `end_of_turn_delayed_settle`). False
/// when a trap's question or a Death hook's paused the stage, or the game ended.
fn check_after_delayed(sink: &mut EngineSink) -> bool {
    // The check's own events — the deaths it collects, their Death hooks, a Reborn body — are the
    // delayed effect's consequences too, so the traps answer them before the next delayed effect
    // resolves (§10.3: `settle` dispatches a check's events before anything else), and what those
    // traps did is checked in turn, until nothing more is said (§4.5, R59).
    loop {
        crate::triggers::dispatch_pending(sink);
        if crate::work::paused(sink) {
            return false;
        }
        let emitted = sink.events.len();
        crate::state_check::state_check(sink);
        if crate::work::paused(sink) {
            return false;
        }
        if sink.events.len() == emitted {
            return true;
        }
    }
}

/// The owed `delayed` step of either boundary: a delayed effect (or a trap answering one, or the trap
/// window before them) paused, and the answer has finished it by the time this runs, so the traps
/// still owed its events and then its state check come first (§10.3, R59) — a unit it killed has
/// died before the next delayed effect meets the board (R174: #50's steal fizzles on it). False when
/// a trap asked again, or that check ended the game or paused on a Death hook's prompt.
fn check_before_delayed(sink: &mut EngineSink) -> bool {
    check_after_delayed(sink)
}

/// Exertion and the once-per-turn flags reset at the controller's own turn start (§4.1).
fn reset_exertion(sink: &mut EngineSink, player: PlayerId) {
    let side = &mut sink.state.players[player];
    for card in side.units.iter_mut().flatten().flatten() {
        card.exertion = Exertion {
            attacked: false,
            switched: false,
            attacks: None,
        };
    }
    // R446: and the Units this player's carriers hold, which may switch position like any unit
    // (`zones.carriedUnitsOf`).
    for card in side.carried.iter_mut().flatten().flatten() {
        card.exertion = Exertion {
            attacked: false,
            switched: false,
            attacks: None,
        };
    }
}

// ---------------------------------------------------------------------------
// The start of a turn, and the remainder it owes when something pauses (§2.2, R62, R113, R117)
// ---------------------------------------------------------------------------

/// R113: the `resume.hook` of the work item the START of a turn parks, the twin of
/// `END_OF_TURN_WORK` below. It is an engine sequence and not a card's, so the name is one no
/// `Script` can hold, and `work.rs` hands it to `run_owed_start_of_turn`.
pub const START_OF_TURN_WORK: &str = "@startOfTurn";

/// Which part of R62's opening is still owed: `brittle` has ticked the Brittle counts and owes the
/// loop the events that woke (§10.3), then everything after; `animate` has moved the "Animated on your
/// turn" cards and owes the same, then the delayed effects and everything after; `delayed` still has
/// start-of-turn delayed effects to finish and owes everything after them; `settle` has had them and
/// owes the loop their events wake (§10.3), then the triggers and the draw; `triggers` has had its
/// effects and owes the rest of the trigger queue and then the draw; `main` has drawn and owes only the
/// phase the turn opens in.
const START_BRITTLE_STEP: &str = "brittle";
const START_ANIMATE_STEP: &str = "animate";
const START_DELAYED_STEP: &str = "delayed";
const START_SETTLE_STEP: &str = "settle";
const START_TRIGGERS_STEP: &str = "triggers";
const START_MAIN_STEP: &str = "main";

/// Park the rest of the start of a turn (R113), exactly as `owe_end_of_turn` parks the rest of an end.
fn owe_start_of_turn(sink: &mut EngineSink, player: PlayerId, step: &str, due_before: Option<u32>) {
    crate::work::owe(
        sink,
        boundary_resume(START_OF_TURN_WORK, player, step, due_before),
    );
}

/// A parked boundary's record: whose turn, and for a delayed stage the mark it began at.
fn boundary_resume(hook: &str, player: PlayerId, step: &str, due_before: Option<u32>) -> Resume {
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert("player".to_string(), Value::String(player.as_str().to_string()));
    if let Some(due_before) = due_before {
        // TS stores the number itself; its `Infinity` is JSON's `null` (`JSON.stringify`, and so the
        // hash and every clone), which `due_before_of` reads back as every entry due.
        let mark = if due_before == DUE_ALL {
            Value::Null
        } else {
            Value::from(due_before)
        };
        data.insert("dueBefore".to_string(), mark);
    }
    Resume {
        def_id: String::new(),
        hook: hook.to_string(),
        step: step.to_string(),
        radiant: false,
        instance_id: None,
        data,
    }
}

/// The mark a parked delayed stage began at; a record without one owes every entry due (R68).
fn due_before_of(data: &IndexMap<String, Value>) -> u32 {
    // `typeof mark === "number" ? mark : Infinity`. A seq is a whole number, so `seq < mark` is
    // `seq < ceil(mark)` for any number the record could hold.
    match data.get("dueBefore").and_then(Value::as_f64) {
        Some(mark) => mark.ceil().clamp(0.0, f64::from(DUE_ALL)) as u32,
        None => DUE_ALL,
    }
}

/// Start a turn (§2.2, R62): refresh, then the Brittle tick, then the "Animated on your turn" cards,
/// then the start-of-turn delayed effects, then the start-of-turn triggers, then the draw.
///
/// Every one of those stages can pause, exactly as the end of a turn can, so the start of one is a
/// resumable sequence in the same shape (R113, R117, R122): each stage stops at a pause, parks what
/// is still owed on `state.work` through `owe_start_of_turn` and returns, and `run_owed_start_of_turn`
/// picks it up when the answer drains the queue. Walking on instead is what had a start-of-turn
/// trigger's prompt open with the turn's draw landing *inside* it, where R62 puts the draw after the
/// triggers — the same bug the end of turn had.
pub fn start_turn(sink: &mut EngineSink, player: PlayerId) {
    begin_turn(sink, player, false);
}

/// R845 (Meditative #19.1): one turn's start, extra or not. An extra turn is a whole §2.2 turn —
/// the count grows, the refresh runs (lost or not), and every stage after it follows.
fn begin_turn(sink: &mut EngineSink, player: PlayerId, extra: bool) {
    sink.state.active = player;
    sink.state.turn += 1;
    sink.state.phase = Phase::Start;
    // R419, R62: the field as this turn begins, before anything else happens (C+ #35 Rollback's history).
    crate::subsystems::board_history::record_board_snapshot(sink.state);
    {
        let side = &mut sink.state.players[player];
        side.turns_started += 1;
        side.turn_log = TurnLog {
            played_ids: Vec::new(),
            cards_played: 0,
            ..TurnLog::default()
        };
    }
    // "This turn" is this turn for both players (§6.2 Combo, #38's "cards you played earlier this
    // turn"): a card the other player casts during it (a cast on draw, R40, R70) counts from zero,
    // not on top of what they played on their own turn before. `unspentAtEnd` is the close of their
    // last turn (§2.2 cleanup) and stays.
    {
        let other = &mut sink.state.players[opponent_of(player)];
        other.turn_log = TurnLog {
            played_ids: Vec::new(),
            cards_played: 0,
            unspent_at_end: other.turn_log.unspent_at_end,
            ..TurnLog::default()
        };
    }
    // R152's backstop: the lockout ends at the cleanup of the turn it was set for, so by now it is
    // already false for the player whose turn My Pawn took. This clears one set on the other player.
    sink.state.players[player].ai_turn = false;
    reset_exertion(sink, player);

    let turn = sink.state.turn;
    sink.events.push(GameEvent::TurnStarted {
        player,
        turn,
        extra: extra.then_some(true),
    });
    let side = &mut sink.state.players[player];
    let rider = side.mana.next_turn_mod;
    crate::mana::refresh_mana(side);
    let lost_spent = crate::mana::spend_lost_refresh(side);
    sink.events.push(crate::mana::mana_event(player, side));
    // R169: the refresh spends the rider (§6.3 Mana), and its badge goes with it.
    if rider != 0 {
        sink.events.push(GameEvent::ModifierChanged {
            player,
            modifier_id: crate::mana::NEXT_REFRESH_MODIFIER_ID.to_string(),
            added: false,
        });
    }
    // R844: a spent loss's badge goes with it.
    if lost_spent {
        sink.events.push(GameEvent::ModifierChanged {
            player,
            modifier_id: crate::mana::LOST_REFRESH_MODIFIER_ID.to_string(),
            added: false,
        });
    }

    // R62: the delayed effects due at this start are the ones that exist as the turn begins, so the
    // mark is taken here and carried through the two stages before them.
    let due_before = sink.state.next_seq;
    start_of_turn_brittle(sink, player, due_before);
}

/// R62, B3.3 (R385): the first stage after the refresh — every Brittle count of `player`'s that has had
/// a full turn cycle ticks, and a count that reaches 0 crumbles its card. The tick is `brittle.rs`'s;
/// this is its place in the turn and what follows it (`after_start_stage`).
fn start_of_turn_brittle(sink: &mut EngineSink, player: PlayerId, due_before: u32) {
    // The testkit's stand-in for this stage, when a test set one (`testkit::seams`, TS's `vi.mock`).
    #[cfg(feature = "testkit")]
    let double = crate::testkit::seams::brittle_tick_double();
    #[cfg(not(feature = "testkit"))]
    let double: Option<fn(&mut EngineSink, PlayerId)> = None;
    match double {
        Some(double) => double(sink, player),
        None => crate::brittle::brittle_tick(sink, player),
    }
    after_start_stage(sink, player, START_BRITTLE_STEP, due_before);
}

/// R62, B3.1 (R383): `player`'s "Animated on your turn" cards step into their unit zones, after the
/// Brittle tick and before the delayed effects. The move is `animated.rs`'s.
fn start_of_turn_animate(sink: &mut EngineSink, player: PlayerId, due_before: u32) {
    // The testkit's stand-in for this stage, when a test set one (`testkit::seams`, TS's `vi.mock`).
    #[cfg(feature = "testkit")]
    let double = crate::testkit::seams::animate_at_turn_start_double();
    #[cfg(not(feature = "testkit"))]
    let double: Option<fn(&mut EngineSink, PlayerId)> = None;
    match double {
        Some(double) => double(sink, player),
        None => crate::animated::animate_at_turn_start(sink, player),
    }
    after_start_stage(sink, player, START_ANIMATE_STEP, due_before);
}

/// §10.3 after a start-of-turn stage, as after the delayed effects (`start_of_turn_settle`): the events
/// the stage emitted reach the traps and the trigger queue — a crumbled unit's Death, a trap answering
/// an arrival — and what they wake resolves, with §4.5's check, before the next stage. A stage that
/// asked something mid-way (a Death hook's prompt) has parked its own remainder; either way what is
/// owed is this settle and every stage after it, parked behind that remainder (R113, R117).
fn after_start_stage(sink: &mut EngineSink, player: PlayerId, step: &str, due_before: u32) {
    if sink.state.result.is_some() {
        return;
    }
    if sink.state.pending.is_none() {
        crate::triggers::settle(sink, crate::triggers::SettleOptions::default());
    }
    if sink.state.result.is_some() {
        return;
    }
    if sink.state.pending.is_some() {
        owe_start_of_turn(sink, player, step, Some(due_before));
        return;
    }
    if step == START_BRITTLE_STEP {
        start_of_turn_animate(sink, player, due_before);
    } else {
        start_of_turn_delayed(sink, player, due_before);
    }
}

/// R62's first stage: the start-of-turn delayed effects, in creation order.
fn start_of_turn_delayed(sink: &mut EngineSink, player: PlayerId, due_before: u32) {
    run_delayed(sink, Phase::Start, player, due_before);
    if sink.state.result.is_some() {
        return;
    }
    if sink.state.pending.is_some() {
        // The entries still due are in `state.delayed`, so the same step picks them up (R68's order).
        owe_start_of_turn(sink, player, START_DELAYED_STEP, Some(due_before));
        return;
    }

    start_of_turn_settle(sink, player);
}

/// §10.3 between R62's first two stages: the events the delayed effects emitted reach the traps and
/// the trigger queue, and what they wake resolves, before the start-of-turn triggers are queued —
/// §6.2's "Delayed effects first (K-Pop Fanatic's steal), then the trigger queue in R68 order". Left to
/// the triggers' own loop, a trigger answering #50's steal was queued behind every start-of-turn hook,
/// a backrow one included.
fn start_of_turn_settle(sink: &mut EngineSink, player: PlayerId) {
    crate::triggers::settle(sink, crate::triggers::SettleOptions::default());
    if sink.state.result.is_some() {
        return;
    }
    if sink.state.pending.is_some() {
        owe_start_of_turn(sink, player, START_SETTLE_STEP, None);
        return;
    }

    start_of_turn_triggers(sink, player);
}

/// R62's second stage: the start-of-turn triggers, and R68's four zones rather than the two on the
/// field — a hand or graveyard trigger holder carries a start-of-turn hook too, and queueing lets one
/// that prompts keep the rest in state.
fn start_of_turn_triggers(sink: &mut EngineSink, player: PlayerId) {
    let _ = crate::triggers::queue_hooks_in_trigger_order(sink, HookName::StartOfTurn, Some(player));
    // R68: then the opponent's cards, which answer the start of a turn that is not theirs (Classic #62).
    let _ = crate::triggers::queue_hooks_in_trigger_order(
        sink,
        HookName::StartOfOpponentTurn,
        Some(opponent_of(player)),
    );
    crate::triggers::settle(sink, crate::triggers::SettleOptions::default());
    if sink.state.result.is_some() {
        return;
    }
    if sink.state.pending.is_some() {
        // The queue is not empty yet, so what is owed is the queue's remainder and then the draw.
        owe_start_of_turn(sink, player, START_TRIGGERS_STEP, None);
        return;
    }

    start_of_turn_draw(sink, player);
}

/// R62's last stage: the turn's draw, and then the main phase.
///
/// The `settle` after the draw is §10.3 and not tidiness: the draw's own events have to be collected
/// into `state.dispatch` and offered to the traps and the trigger queue like any others. §2.4's Cast
/// on draw makes that a whole play at the start of a turn — `drawn`, `cardPlayed`, `cardResolved`,
/// `addedToHand` (R58) — and without this they were emitted and never dispatched, so a trap that
/// answers one of them (#60 Bear Honeypot's `cardResolved`) never saw it. It only ever showed on a
/// direct `start_turn` call, because `reduce` settles at the end of every action.
///
/// R183: the draw is DRAWS_PER_TURN plus the seat's handicap `extraDrawsPerTurn`, made as one
/// "draw N". `draw` already makes N separate draws, each with its own cast-on-draw chain (R58), its
/// own fatigue step (R3) and R158's pause handling: a prompt opened inside the first parks the rest
/// on `state.work` ahead of the `main` step parked below, so the answer makes the owed draw and only
/// then opens the main phase (R113).
fn start_of_turn_draw(sink: &mut EngineSink, player: PlayerId) {
    let count = DRAWS_PER_TURN + handicap_of(&sink.state.players[player]).extra_draws_per_turn;
    let _ = crate::draw::draw(sink, player, count);
    // R59: after the draw as a whole. A cast-on-draw card whose cast is asking something is not
    // whole yet: the chain it owes runs the check once the answer has finished the cast (§2.4, R158).
    if !crate::work::paused(sink) {
        crate::state_check::state_check(sink);
    }
    crate::triggers::settle(sink, crate::triggers::SettleOptions::default());
    if sink.state.result.is_some() {
        return;
    }
    if sink.state.pending.is_some() {
        // A trap or a cast-on-draw card asked something: the turn still owes its own opening.
        owe_start_of_turn(sink, player, START_MAIN_STEP, None);
        return;
    }

    sink.state.phase = Phase::Main;
}

/// `work.rs`'s handler for a parked start of turn (`START_OF_TURN_WORK`): the same turn, continued
/// where it stopped (R113).
///
/// The `triggers` stage settles the queue itself, because the hooks are already queued and `work.rs`
/// is drained *ahead* of the trigger queue (`triggers::settle`), so the remainder has to finish the
/// queue rather than jump it. The `delayed` stage re-enters `run_delayed`, whose still-due entries are
/// in `state.delayed`, and the `main` step is the phase change a pause inside the draw held up.
pub fn run_owed_start_of_turn(sink: &mut EngineSink, item: &WorkItem) {
    let Some(player) = turn_player_of(&item.resume.data) else {
        return;
    };
    let step = item.resume.step.as_str();

    if step == START_BRITTLE_STEP || step == START_ANIMATE_STEP {
        after_start_stage(sink, player, step, due_before_of(&item.resume.data));
        return;
    }

    if step == START_DELAYED_STEP {
        let due_before = due_before_of(&item.resume.data);
        if !check_before_delayed(sink) {
            if sink.state.result.is_none() {
                owe_start_of_turn(sink, player, START_DELAYED_STEP, Some(due_before));
            }
            return;
        }
        start_of_turn_delayed(sink, player, due_before);
        return;
    }

    if step == START_SETTLE_STEP {
        start_of_turn_settle(sink, player);
        return;
    }

    if step == START_TRIGGERS_STEP {
        crate::triggers::settle(sink, crate::triggers::SettleOptions::default());
        if sink.state.result.is_some() {
            return;
        }
        if sink.state.pending.is_some() {
            owe_start_of_turn(sink, player, START_TRIGGERS_STEP, None);
            return;
        }
        start_of_turn_draw(sink, player);
        return;
    }

    if sink.state.result.is_none() {
        sink.state.phase = Phase::Main;
    }
}

/// Cleanup (§2.2): "this turn" modifiers expire, the turn log is closed and the AI lockout ends.
///
/// R152: §8 #96 says the opponent's client is locked out "until end of turn", so the flag is cleared
/// HERE, at the end of the turn the effect took, and not at that player's next turn start — clearing
/// it later leaves them locked out of a turn that is no longer the one My Pawn took. `start_turn`
/// still clears it as a backstop, for a flag somehow set on the player who is not the active one.
///
/// R155: §5.1's `returnToHandAtEndOfTurn` is cleared here for the same reason. §10.5 step 7 sets it
/// as the Spell lands in the graveyard and R68's end-of-turn queue — which has already run by the
/// time cleanup does — is what acts on it, so this is the end of the one turn the flag was ever
/// about. Leaving it set would make the card return from the graveyard on every later turn it
/// happened to be in one, including after it was merely discarded or milled (R153).
pub fn clear_return_flags(state: &mut GameState) {
    // The cards played this turn are exactly the ones step 7 could have flagged: it writes the flag on
    // the card it just landed, and step 4 logged that same card on its player's turn log. Both logs:
    // a Spell cast on the other player's turn (a cast on draw, R70) is flagged too, and §6.2 makes an
    // "End of turn" its controller's own turn end, which this is not — so its return is over with this
    // turn as well (R155). `start_turn` empties both logs, so these are still this turn's lists.
    for player in PLAYER_IDS {
        let ids: IndexSet<String> = state.players[player]
            .turn_log
            .played_ids
            .iter()
            .cloned()
            .collect();
        for id in ids {
            if let Some(card) = find_instance_mut(state, &id)
                && card.return_to_hand_at_end_of_turn == Some(true)
            {
                card.return_to_hand_at_end_of_turn = None;
                // R429, R766: the price noted for the return ends with it.
                crate::resolve::forget_return_price(card);
            }
        }
    }
}

fn cleanup(sink: &mut EngineSink, player: PlayerId) {
    crate::modifiers::expire_modifiers(sink, player);
    let side = &mut sink.state.players[player];
    side.turn_log.unspent_at_end = Some(side.mana.current);
    let handed_over = side.ai_turn;
    if handed_over {
        crate::traps::end_handed_over_turn(sink);
    }
    sink.state.players[player].ai_turn = false;
    clear_return_flags(sink.state);
    // R637: a Temporary card still in `player`'s hand is discarded now, after every end-of-turn step.
    crate::temporary::discard_temporary_cards(sink, player);
    // §2.2, B3.1 rule 4 (R383): cleanup's last step — `player`'s animated "on your turn" cards go back
    // to their backrow zones, after every end-of-turn step, so their own end-of-turn text ran while they
    // were Units. The move is `animated.rs`'s; its events are answered by the loop after cleanup
    // (`end_of_turn_cleanup_settle`), which parks the rest of the turn on a prompt like any other stage.
    // The testkit's stand-in for this step, when a test set one (`testkit::seams`, TS's `vi.mock`).
    #[cfg(feature = "testkit")]
    let double = crate::testkit::seams::return_at_cleanup_double();
    #[cfg(not(feature = "testkit"))]
    let double: Option<fn(&mut EngineSink, PlayerId)> = None;
    match double {
        Some(double) => double(sink, player),
        None => crate::animated::return_at_cleanup(sink, player),
    }
}

// ---------------------------------------------------------------------------
// The end of a turn, and the remainder it owes when something pauses (§2.2, R62, R113, R117)
// ---------------------------------------------------------------------------

/// R113: the `resume.hook` of the one work item this module parks at the end of a turn — the rest
/// of an end of turn. It is an engine sequence and not a card's, so the name is one no `Script` can
/// hold, and `work.rs` hands it to `run_owed_end_of_turn`: `work::run_work_item` raises on a hook
/// nothing knows, and an end of turn that cannot be resumed is exactly the lost sequence that rule
/// exists to prevent.
pub const END_OF_TURN_WORK: &str = "@endOfTurn";

/// Which part of R62's order is still owed, named so a reader of `state.work` can see it:
/// `triggers` still has the end-of-turn trigger queue to finish before the `turnEnded` event is
/// even emitted; `window` has had its trap window and owes the loop the window's events wake
/// (§10.3), then everything after; `delayed` owes the delayed effects still due; `cleanup` has had
/// them and owes the loop their events wake, then cleanup and everything after; `next` has had
/// cleanup (the animated cards' return its last step, B3.1) and owes the loop its events wake, then
/// the turn cap and the next turn.
const END_TRIGGERS_STEP: &str = "triggers";
const END_WINDOW_STEP: &str = "window";
const END_DELAYED_STEP: &str = "delayed";
const END_CLEANUP_STEP: &str = "cleanup";
const END_NEXT_STEP: &str = "next";

/// Whose turn a parked boundary belongs to — the one ending, or the one starting — read back
/// defensively, because the item came through JSON (§10.1). Both handlers park `{ player }`.
fn turn_player_of(data: &IndexMap<String, Value>) -> Option<PlayerId> {
    let player = data.get("player").and_then(Value::as_str);
    PLAYER_IDS.into_iter().find(|id| Some(id.as_str()) == player)
}

/// Park the rest of the end of turn (R113). `work.rs` owns `state.work`, so this only ever owes:
/// the item lands at `state.workCursor`, which the pausing scope has just advanced past its own
/// item, so it sits *behind* the remainder of whatever paused — the trap window's owed traps first,
/// then this — and R62's order survives the pause. Nothing is held but plain JSON.
///
/// R117: every caller calls this at the moment it actually pauses and never in advance. While
/// `end_turn` is on the stack the steps after the pause point are `end_turn`'s alone, so a `settle`
/// running inside one of them — a trap's or a trigger's own effects can start one — can neither
/// take nor re-run the steps it is standing in. Pre-parking a sequence's continuation is what made
/// a played card's Cry fire twice (§10.5's driver, R1).
fn owe_end_of_turn(sink: &mut EngineSink, player: PlayerId, step: &str, due_before: Option<u32>) {
    crate::work::owe(sink, boundary_resume(END_OF_TURN_WORK, player, step, due_before));
}

/// End the active player's turn and start the next, unless the cap ends the game (§2.5, R2).
///
/// R62's order is the whole point: end-of-turn triggers, then the end-of-turn trap window (the
/// ending player's traps first, then the opponent's), then the end-of-turn delayed effects, then
/// cleanup, then the turn-cap check.
///
/// Every step of that order can pause, because a trigger, a trap or a delayed effect may ask its
/// controller something (§9.3, §10.6). So each stage below stops at a pause and owes what is left
/// to `state.work` instead of walking on: a delayed effect that resolved *inside* an unfinished trap
/// window, and a `cleanup` that closed the turn log before the window's last traps had fired, were
/// both R62's order broken on the pause path.
pub fn end_turn(sink: &mut EngineSink) {
    let player = sink.state.active;
    sink.state.phase = Phase::End;

    // R68: the hand and graveyard holders too (the `returnToHandAtEndOfTurn` spells of §5.1).
    let _ = crate::triggers::queue_hooks_in_trigger_order(sink, HookName::EndOfTurn, Some(player));
    crate::triggers::settle(sink, crate::triggers::SettleOptions::default());
    if sink.state.result.is_some() {
        return;
    }
    if sink.state.pending.is_some() {
        // The trigger queue is not empty yet, so what is owed is the whole rest of R62's order.
        owe_end_of_turn(sink, player, END_TRIGGERS_STEP, None);
        return;
    }

    end_of_turn_after_triggers(sink, player);
}

/// R62 from the `turnEnded` event on: the trap window, then everything after it.
///
/// `turnEnded` is emitted *before* the window rather than after it, because the window's traps read
/// it: #18 Bread and Butter answers `event.unspentMana`, which is the mana the player still holds
/// before `cleanup` closes the turn log. R100 keeps the event out of the immediate trap check —
/// `traps::TRAP_WINDOW_EVENTS` withholds `turnEnded` from `fire_traps_for`, so the window below is the
/// only place a `turnEnded` trap fires, exactly once per turn end. Ordinary (non-trap) triggers on
/// `turnEnded` still queue the usual way when `triggers::settle` reaches the event.
fn end_of_turn_after_triggers(sink: &mut EngineSink, player: PlayerId) {
    let ended = GameEvent::TurnEnded {
        player,
        turn: sink.state.turn,
        unspent_mana: crate::query::unspent_mana_of(&*sink.state, player),
    };
    sink.events.push(ended.clone());

    // §2.2's end-of-turn trap window, at R62's scheduled point: after the triggers, before the
    // delayed effects. A prompt a trap opens pauses the rest of the window in state (§9.3), and
    // `traps::run_trap_window` parks the traps it never reached; the steps after the window are this
    // module's, so they are parked here, behind them.
    let _ = crate::traps::run_trap_window(sink, &ended);
    if sink.state.result.is_some() {
        return;
    }
    if sink.state.pending.is_some() {
        owe_end_of_turn(sink, player, END_WINDOW_STEP, None);
        return;
    }

    end_of_turn_window_settle(sink, player);
}

/// §10.3 after the window: a trap's firing is an effect like any other, so the events it emitted
/// reach the traps and the trigger queue, and what they wake resolves, before R62 moves on to the
/// delayed effects — at this turn's end, not in the next turn's opening loop, behind that turn's own
/// start-of-turn hooks. A trigger answering Bread and Butter's token that finishes the opponent wins
/// the game at the end of this turn (§4.5 step 2).
fn end_of_turn_window_settle(sink: &mut EngineSink, player: PlayerId) {
    crate::triggers::settle(sink, crate::triggers::SettleOptions::default());
    if sink.state.result.is_some() {
        return;
    }
    if sink.state.pending.is_some() {
        owe_end_of_turn(sink, player, END_WINDOW_STEP, None);
        return;
    }

    let due_before = sink.state.next_seq;
    end_of_turn_after_window(sink, player, due_before);
}

/// R62's tail: the end-of-turn delayed effects, then (`end_of_turn_delayed_settle`) the rest.
fn end_of_turn_after_window(sink: &mut EngineSink, player: PlayerId, due_before: u32) {
    run_delayed(sink, Phase::End, player, due_before);
    if sink.state.result.is_some() {
        return;
    }
    if sink.state.pending.is_some() {
        // The entries still due are in `state.delayed`, so the same step picks them up (R68's order).
        owe_end_of_turn(sink, player, END_DELAYED_STEP, Some(due_before));
        return;
    }

    end_of_turn_delayed_settle(sink, player);
}

/// §10.3 after the delayed effects, as after the window; then cleanup, the turn cap, the next turn.
fn end_of_turn_delayed_settle(sink: &mut EngineSink, player: PlayerId) {
    crate::triggers::settle(sink, crate::triggers::SettleOptions::default());
    if sink.state.result.is_some() {
        return;
    }
    if sink.state.pending.is_some() {
        owe_end_of_turn(sink, player, END_CLEANUP_STEP, None);
        return;
    }

    cleanup(sink, player);
    end_of_turn_cleanup_settle(sink, player);
}

/// §10.3 after cleanup, as after every other stage of the turn: cleanup's own events — My Pawn
/// reaching its owner's graveyard at the end of the turn it took (R152), the "this turn" modifiers
/// expiring — reach the traps and the trigger queue, and what they wake resolves, before the turn-cap
/// check and the next turn (R62's order: cleanup, then the cap, then the opponent's turn). Left to the
/// next turn's first loop, a trigger answering them resolved after that turn had begun, and at the
/// cap the game was drawn before it could.
/// R846 (Meditative #19.1): spend one owed extra turn of the player whose turn just ended.
/// Returns true when one was owed, so the cleanup starts their turn again instead of the
/// opponent's. The count is `None` at 0, so a game without one hashes as before.
fn take_owed_extra_turn(sink: &mut EngineSink, player: PlayerId) -> bool {
    let owed = sink.state.players[player].extra_turns.unwrap_or(0);
    if owed <= 0 {
        return false;
    }
    let left = owed - 1;
    sink.state.players[player].extra_turns = if left == 0 { None } else { Some(left) };
    if left == 0 {
        sink.events.push(GameEvent::ModifierChanged {
            player,
            modifier_id: EXTRA_TURN_MODIFIER_ID.to_string(),
            added: false,
        });
    }
    true
}

fn end_of_turn_cleanup_settle(sink: &mut EngineSink, player: PlayerId) {
    crate::triggers::settle(sink, crate::triggers::SettleOptions::default());
    if sink.state.result.is_some() {
        return;
    }
    if sink.state.pending.is_some() {
        owe_end_of_turn(sink, player, END_NEXT_STEP, None);
        return;
    }

    // R155: a return Spell cast while cleanup's events were answered — a cast on draw a trigger's draw
    // made — was flagged by §10.5 step 7 after cleanup had cleared the flags. This turn's end-of-turn
    // triggers are over, so its return is over too, and it is cleared now, while this is still the
    // turn whose logs name it: `start_turn` empties them, and a flag no cleanup saw came back at the
    // end of its caster's next turn, one it was not played on.
    clear_return_flags(sink.state);

    if sink.state.turn >= TURN_CAP_PLAYER_TURNS {
        end_game(sink, Winner::Draw, GameOverReason::TurnCap);
        return;
    }

    // R846: an owed extra turn starts the same player's turn again, counting toward the cap above.
    if take_owed_extra_turn(sink, player) {
        begin_turn(sink, player, true);
        return;
    }

    start_turn(sink, opponent_of(player));
}

/// `work.rs`'s handler for a parked end of turn (`END_OF_TURN_WORK`): the same turn, continued where
/// it stopped (R113).
///
/// The `triggers` stage finishes the trigger queue first, because R62 puts the end-of-turn triggers
/// before the window and `work.rs` is drained *ahead* of the queue (`triggers::settle`) — so the
/// remainder has to settle the queue itself rather than jump it. The item is off the queue by the
/// time this runs (`work::take_work`), so that `settle` can neither take nor re-run this item.
pub fn run_owed_end_of_turn(sink: &mut EngineSink, item: &WorkItem) {
    let Some(player) = turn_player_of(&item.resume.data) else {
        return;
    };
    let step = item.resume.step.as_str();

    if step == END_TRIGGERS_STEP {
        crate::triggers::settle(sink, crate::triggers::SettleOptions::default());
        if sink.state.result.is_some() {
            return;
        }
        if sink.state.pending.is_some() {
            owe_end_of_turn(sink, player, END_TRIGGERS_STEP, None);
            return;
        }
        end_of_turn_after_triggers(sink, player);
        return;
    }

    if step == END_WINDOW_STEP {
        end_of_turn_window_settle(sink, player);
        return;
    }

    if step == END_CLEANUP_STEP {
        end_of_turn_delayed_settle(sink, player);
        return;
    }

    if step == END_NEXT_STEP {
        end_of_turn_cleanup_settle(sink, player);
        return;
    }

    // `delayed`: a delayed effect asked, and the answer has finished it.
    let due_before = due_before_of(&item.resume.data);
    if !check_before_delayed(sink) {
        if sink.state.result.is_none() {
            owe_end_of_turn(sink, player, END_DELAYED_STEP, Some(due_before));
        }
        return;
    }
    end_of_turn_after_window(sink, player, due_before);
}

pub fn concede(sink: &mut EngineSink, player: PlayerId) {
    end_game(sink, opponent_of(player), GameOverReason::Concede);
}

/// R36: only the active player offers, once per turn, and a declined offer blocks 3 of their turns.
pub fn can_offer_draw(state: &GameState, player: PlayerId) -> bool {
    if state.active != player || state.phase != Phase::Main {
        return false;
    }
    let offer = &state.players[player].draw_offer;
    if offer.offered_turn == Some(state.turn) {
        return false;
    }
    let blocked_until = offer.blocked_until.unwrap_or(0);
    state.players[player].turns_started >= blocked_until
}

pub fn offer_draw(sink: &mut EngineSink, player: PlayerId) {
    sink.state.players[player].draw_offer.offered_turn = Some(sink.state.turn);
    sink.events.push(GameEvent::DrawOffered { player });
}

/// R36: whether `player` has an offer to answer — the active opponent offered this turn and it has
/// not been answered yet. `legal_actions` and the reducer both ask this, so a declined offer is gone
/// from both at once.
///
/// R269: an offer lives for the rest of the turn it was made on. Nothing clears it when that turn
/// ends: the turn number moves on, so the offer simply stops standing — it lapses, and a lapsed
/// offer is not a declined one, so it blocks nothing (R36's block is a decline's).
pub fn has_standing_draw_offer(state: &GameState, player: PlayerId) -> bool {
    let offering = opponent_of(player);
    state.active == offering && state.players[offering].draw_offer.offered_turn == Some(state.turn)
}

/// R269: the player whose draw offer stands right now, or `None`. `view_for` shows it to both seats —
/// the offer was a public action (`drawOffered`) — so a client can tell the offerer it is waiting
/// and the other seat that it has an offer to answer, and a reconnect shows the same.
pub fn standing_draw_offer(state: &GameState) -> Option<PlayerId> {
    // R216: nothing stands once the game is over, an offer included (a concede, the turn cap).
    if state.result.is_some() {
        return None;
    }
    if has_standing_draw_offer(state, opponent_of(state.active)) {
        Some(state.active)
    } else {
        None
    }
}

pub fn answer_draw(sink: &mut EngineSink, player: PlayerId, accept: bool) {
    sink.events.push(GameEvent::DrawAnswered { player, accept });
    let offering = opponent_of(player);
    // §2.5, R36: an offer is answered once; afterwards there is nothing standing to accept. The
    // once-per-turn limit does not need the record: a declined offer blocks its player's next
    // DRAW_OFFER_BLOCK_TURNS turns, this one included, and an accepted one ends the game.
    sink.state.players[offering].draw_offer.offered_turn = None;
    if accept {
        end_game(sink, Winner::Draw, GameOverReason::DrawAccepted);
        return;
    }
    // R36: the next DRAW_OFFER_BLOCK_TURNS turns of theirs are blocked, counting from the next one.
    let side = &mut sink.state.players[offering];
    side.draw_offer.blocked_until = Some(side.turns_started + DRAW_OFFER_BLOCK_TURNS + 1);
}
