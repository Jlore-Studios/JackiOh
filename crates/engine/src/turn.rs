//! The turn loop of SPEC §2.2 in R62's order, cleanup, the turn cap and the ways a game ends (§2.5, R79,
//! R389). Each part belongs to its module: the turn hooks to `triggers::queue_hooks_in_trigger_order` (R68's
//! order), the end-of-turn trap window to `traps::run_trap_window` (R100), the delayed effects to
//! `modifiers::due_delayed`, mana to `mana.rs`. A turn starts: refresh → Brittle tick (B3.3), Dormant
//! countdowns (R1839001), quests' waits (R1854004) → animate (B3.1) → delayed effects, rest-of-game ones
//! among them (B5 E28, R458) → start-of-turn triggers → draw. Animated cards go home last in cleanup.
//!
//! Any part can pause on a question (§9.3, §10.6), so both boundaries are resumable: a pause parks the
//! rest of R62's order on `state.work` through `owe`, and `run_owed_start_of_turn` / `run_owed_end_of_turn`
//! (`work.rs`'s handlers, SURFACE §6.6) pick it up when the answer drains the queue (R113, R117, R122).
//! Each stage that emits events settles before the next (§10.3), so the draw's events (a Cast on draw is
//! a whole play, R58) reach the traps and the trigger queue.

use indexmap::{IndexMap, IndexSet};
use serde_json::Value;

use crate::config::{DRAW_OFFER_BLOCK_TURNS, DRAW_OFFERS_PER_TURN, DRAWS_PER_TURN, TURN_CAP_PLAYER_TURNS};
use crate::game_over::end_game;
use crate::resolve::HookName;
use crate::script::EngineSink;
use crate::state::{
    DelayedEffect, Exertion, GameState, ModifierKind, Resume, TurnLog, WorkItem, find_instance_mut,
    handicap_of,
};
use crate::wire::{GameEvent, GameOverReason, PLAYER_IDS, Phase, PlayerId, Winner, opponent_of};

/// R846 (Meditative #19.1): the id owed extra turns travel under, one badge per player: `modifierChanged`
/// names it as turns are owed and the last is taken, and the view lists it while any is owed (R169).
pub const EXTRA_TURN_MODIFIER_ID: &str = "extraTurns";

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

/// The creation mark at which every entry is due: no game's seq reaches `u32::MAX`.
const DUE_ALL: u32 = u32::MAX;

/// §2.2's two delayed-effect points, in R68's creation order. An entry is dropped before it runs, so it can
/// never fire twice. R126: a delayed continuation is re-entered as a prompt answer is, through
/// `prompts::run_resume` and not `resolve::run_hook`: `resume.hook` may name a hook (`delayed`) or the step
/// table (`resume`), and only `run_resume` resolves both. R127: an entry whose instance is gone still
/// resolves by its def id with `ctx.self_ == None` (§10.6, R76: #50 K-Pop Fanatic's steal after it died,
/// #39's exile after it exiled itself); skipping it would be the silent loss R113 forbids. A step that opens
/// a prompt stops the run (§9.3): its tail is parked by `prompts::apply_resumable`, the entries still due
/// stay in `state.delayed`, and the caller parks the rest of the boundary.
fn run_delayed(sink: &mut EngineSink, phase: Phase, player: PlayerId, due_before: u32) {
    // R62, R68: due here are the entries that exist as the stage begins; one made while it resolves is
    // due at the next such point. `due_before` is that creation mark, carried across a pause (R113).
    for entry in due_entries(&*sink.state, phase, player, due_before) {
        if crate::work::paused(sink) {
            return;
        }
        if !run_due_entry(sink, player, &entry) {
            continue;
        }
        // R59: the check runs after the whole delayed effect. One that asked is not whole yet, so its
        // check is owed to the step that picks the boundary up (`check_before_delayed`, R174).
        if crate::work::paused(sink) {
            return;
        }
        if !check_after_delayed(sink) {
            return;
        }
    }
}

/// One entry of R62's delayed stage: a delayed effect (`state.delayed`), or a rest-of-game effect of the
/// player's (B5 E28, R458), due at every start of their turn. Both are ordered by creation `seq` (R68).
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
    // A private copy of `modifiers::due_delayed`: the entries due at this point of this player's turn.
    let mut delayed: Vec<&DelayedEffect> = state
        .delayed
        .iter()
        .filter(|effect| {
            effect.at.phase == phase
                && effect.at.player == player
                && effect
                    .not_before
                    .is_none_or(|not_before| state.turn >= not_before)
                && crate::modifiers::own_turn_reached(state, effect)
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
    // The player's rest-of-game effects not yet run this turn, due at its start, or at its end for an
    // `atEnd` one (R1861002).
    let mut recurring: Vec<(u32, String)> = state.players[player]
        .mods
        .iter()
        .filter_map(|held| match &held.kind {
            ModifierKind::StartOfTurnEffect {
                seq,
                ran_turn,
                at_end,
                ..
            } if *ran_turn != Some(state.turn) && (*at_end == Some(true)) == (phase == Phase::End) => {
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
    // A stable sort (SURFACE §4.4.1).
    entries.sort_by_key(|a| a.seq);
    entries
}

/// Run one due entry, or pass it by (false) when it is no longer due. A rest-of-game effect stays for
/// the next turn, so it marks the turn it ran on (`ranTurn`) where a delayed effect is dropped.
fn run_due_entry(sink: &mut EngineSink, player: PlayerId, entry: &DueEntry) -> bool {
    match &entry.what {
        Due::Delayed(effect) => {
            // R174: an earlier entry can end a later one (the check after #50's first steal kills the
            // second's target, and `zones::forget_watchers` drops its entry even if Reborn brings it back),
            // so an entry runs only while `state.delayed` holds it. R1140: and as it stands now.
            let Some(live) = sink.state.delayed.iter().find(|due| due.id == effect.id).cloned() else {
                return false;
            };
            sink.state.delayed.retain(|due| due.id != effect.id);
            // R1181000, R169: a countdown's badge goes as its entry resolves.
            if crate::modifiers::has_countdown(&live) {
                sink.events.push(GameEvent::ModifierChanged {
                    player: live.owner,
                    modifier_id: live.id.clone(),
                    added: false,
                });
            }
            // B5 E27: the engine's own delayed kinds (a destroy, a discard) run as verbs; every other entry
            // re-enters its card's step (R126).
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

/// §10.3 after one whole delayed effect: its events go to the traps, which fire at once (a trap answering
/// #50's first steal resolves before the second), then the state check (R59), repeated while the check's
/// own events wake traps (§4.5). Other triggers wait for the stage's own loop, in R68's order behind every
/// delayed effect due. False when a trap's or Death hook's question paused the stage, or the game ended.
fn check_after_delayed(sink: &mut EngineSink) -> bool {
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

/// The owed `delayed` step of either boundary: the answer has finished what paused, so its owed traps and
/// state check come first (§10.3, R59), and a unit it killed has died before the next delayed effect meets
/// the board (R174). False as for `check_after_delayed`.
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
    // R446: and the Units this player's carriers hold, which switch position like any unit.
    for card in side.carried.iter_mut().flatten().flatten() {
        card.exertion = Exertion {
            attacked: false,
            switched: false,
            attacks: None,
        };
    }
    // R1081000: and the Units standing in its backrow zones, which attack and switch as any unit does.
    let standing: Vec<String> = crate::zones::standing_units_of(sink.state, player)
        .into_iter()
        .map(|card| card.id.clone())
        .collect();
    for card in sink.state.players[player].backrow.iter_mut().flatten() {
        if standing.contains(&card.id) {
            card.exertion = Exertion {
                attacked: false,
                switched: false,
                attacks: None,
            };
        }
    }
}

// The start of a turn, and the remainder it owes when something pauses (§2.2, R62, R113, R117)

/// R113: the `resume.hook` a paused start of turn parks under, the twin of `END_OF_TURN_WORK`. No
/// `Script` can hold the name, and `work.rs` hands it to `run_owed_start_of_turn`.
pub const START_OF_TURN_WORK: &str = "@startOfTurn";

/// Which part of R62's opening is still owed: `brittle` and `animate` have run their stage and owe its
/// settle (§10.3), then every stage after; `delayed` owes the delayed effects still due; `settle` owes
/// their settle, then the triggers and the draw; `triggers` owes the rest of the trigger queue, then the
/// draw; `main` has drawn and owes only the phase the turn opens in.
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
        // Every entry due is stored as `null`, as the hashed state holds it; `due_before_of` reads it back.
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
    // A seq is a whole number, so `seq < mark` is `seq < ceil(mark)` for any number the record holds.
    match data.get("dueBefore").and_then(Value::as_f64) {
        Some(mark) => mark.ceil().clamp(0.0, f64::from(DUE_ALL)) as u32,
        None => DUE_ALL,
    }
}

/// Start a turn (§2.2, R62): refresh, the Brittle tick, the "Animated on your turn" cards, the
/// start-of-turn delayed effects, the start-of-turn triggers, the draw. A stage that pauses parks the
/// rest through `owe_start_of_turn` for `run_owed_start_of_turn` (R113, R117, R122), so the draw never
/// lands inside an open start-of-turn prompt.
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
    // turn"): a card the other player casts during it (a cast on draw, R40, R70) counts from zero.
    // `unspentAtEnd` is the close of their last turn (§2.2 cleanup) and stays.
    {
        let other = &mut sink.state.players[opponent_of(player)];
        other.turn_log = TurnLog {
            played_ids: Vec::new(),
            cards_played: 0,
            unspent_at_end: other.turn_log.unspent_at_end,
            ..TurnLog::default()
        };
    }
    // R152's backstop: cleanup ends the lockout of the turn it was set for; this clears one set on the
    // other player.
    sink.state.players[player].ai_turn = false;
    reset_exertion(sink, player);

    let turn = sink.state.turn;
    sink.events.push(GameEvent::TurnStarted {
        player,
        turn,
        extra: extra.then_some(true),
    });
    // R1844000: an autopilot aura takes the turn as it begins, so the start's prompts are its too.
    crate::subsystems::autopilot::engage_aura(sink, player);
    // R2250002, R1853002: an open Marketplace rolls the player their offers and quotes its prices.
    crate::marketplace::marketplace_turn_start(sink, player);
    // MD-B2, R941: the aura is read before the refresh, off the field as it stands now.
    let natural = crate::mana::natural_mana_on(sink.state);
    let side = &mut sink.state.players[player];
    let rider = side.mana.next_turn_mod;
    crate::mana::refresh_mana_with(side, natural);
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

/// R62, B3.3 (R385): the first stage after the refresh, `brittle.rs`'s tick: each of `player`'s Brittle
/// counts that has had a full turn cycle ticks, and one that reaches 0 crumbles its card.
fn start_of_turn_brittle(sink: &mut EngineSink, player: PlayerId, due_before: u32) {
    // The testkit's stand-in for this stage, when a test set one (`testkit::seams`).
    #[cfg(feature = "testkit")]
    let double = crate::testkit::seams::brittle_tick_double();
    #[cfg(not(feature = "testkit"))]
    let double: Option<fn(&mut EngineSink, PlayerId)> = None;
    match double {
        Some(double) => double(sink, player),
        None => crate::brittle::brittle_tick(sink, player),
    }
    // R1839001: then `player`'s Dormant cards count down and one whose countdown ends awakens, in this
    // stage so its events settle with the tick's.
    crate::slumber::awaken_at_turn_start(sink, player);
    // R1854004: then `player`'s quests waiting to hear back count the turn; one whose wait ends opens
    // its stage.
    crate::subsystems::quests::hear_back_at_turn_start(sink, player);
    after_start_stage(sink, player, START_BRITTLE_STEP, due_before);
}

/// R62, B3.1 (R383): `player`'s "Animated on your turn" cards step into their unit zones, after the
/// Brittle tick and before the delayed effects. The move is `animated.rs`'s.
fn start_of_turn_animate(sink: &mut EngineSink, player: PlayerId, due_before: u32) {
    // The testkit's stand-in for this stage, when a test set one (`testkit::seams`).
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

/// §10.3 after a start-of-turn stage: its events (a crumbled unit's Death, an arrival) reach the traps and
/// the trigger queue and resolve, with §4.5's check, before the next stage. A stage that asked mid-way has
/// parked its own remainder; this settle and every stage after are owed behind it (R113, R117).
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

/// §10.3 between R62's delayed effects and triggers: what the delayed effects' events wake resolves before
/// the start-of-turn triggers are queued (§6.2's "Delayed effects first, then the trigger queue in R68
/// order"), or a trigger answering #50's steal would queue behind every start-of-turn hook.
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

/// R62's start-of-turn triggers, from R68's four zones (a hand or graveyard holder has the hook too);
/// queueing lets one that prompts keep the rest in state.
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

/// R62's last stage: the turn's draw, then the main phase. The `settle` after it is §10.3: §2.4's Cast on
/// draw is a whole play (R58) whose events must reach the traps and the trigger queue (#60 Bear Honeypot
/// answers `cardResolved`); `reduce` settles only at the end of an action. R183: the draw is one "draw N"
/// (DRAWS_PER_TURN plus the seat's handicap `extraDrawsPerTurn`), each draw with its cast-on-draw chain
/// (R58), fatigue (R3) and R158's pause handling, so a prompt in the first parks the rest ahead of the `main`
/// step parked below (R113).
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

/// `work.rs`'s handler for a parked start of turn (`START_OF_TURN_WORK`), continued where it stopped
/// (R113). The `triggers` step settles the queue itself: `work.rs` is drained ahead of the trigger queue,
/// so the remainder must finish the queue rather than jump it. `main` is the phase change a pause inside
/// the draw held up.
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

/// Cleanup (§2.2): "this turn" modifiers expire, the turn log is closed and the AI lockout ends. R152: §8 #96
/// locks the opponent's client out "until end of turn", so the flag is cleared at the end of the turn My Pawn
/// took, not at that player's next turn start; `start_turn` clears it as a backstop. R155: §5.1's
/// `returnToHandAtEndOfTurn` is cleared here too: §10.5 step 7 sets it as the Spell lands and R68's
/// end-of-turn queue has already acted on it, so left set the card would return from the graveyard on every
/// later turn, even after a discard or mill (R153).
pub fn clear_return_flags(state: &mut GameState) {
    // Step 7 flags the card step 4 logged on its player's turn log. Both logs: a Spell cast on the other
    // player's turn is over with this turn too (§6.2, R155). A card cast on draw is logged as no play
    // (R2179002), so flagged graveyard cards are cleared too: the flag lives only there, and every one
    // left is this turn's.
    for player in PLAYER_IDS {
        let mut ids: IndexSet<String> = state.players[player]
            .turn_log
            .played_ids
            .iter()
            .cloned()
            .collect();
        ids.extend(
            state.players[player]
                .graveyard
                .iter()
                .filter(|card| card.return_to_hand_at_end_of_turn == Some(true))
                .map(|card| card.id.clone()),
        );
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
    // R1174000: beside the "this turn" modifiers, `player`'s Frozen Units whose thaw is due thaw.
    crate::freeze::thaw_at_cleanup(sink, player);
    let side = &mut sink.state.players[player];
    side.turn_log.unspent_at_end = Some(side.mana.current);
    let handed_over = side.ai_turn;
    if handed_over {
        crate::traps::end_handed_over_turn(sink);
    }
    sink.state.players[player].ai_turn = false;
    // R1183002: the autopilot played this turn and ends with it.
    sink.state.players[player].autopilot = None;
    // R1844001: so does the Pareto hint the `pareto_hint` verb showed for it.
    sink.state.players[player].pareto_hint = None;
    clear_return_flags(sink.state);
    // R637: a Temporary card still in `player`'s hand is discarded now, after every end-of-turn step.
    crate::temporary::discard_temporary_cards(sink, player);
    // §2.2, B3.1 rule 4 (R383): cleanup's last step, `player`'s animated "on your turn" cards go back to
    // their backrow zones, so their own end-of-turn text ran while they were Units; the loop after
    // cleanup (`end_of_turn_cleanup_settle`) answers their events. The testkit's stand-in for this step,
    // when a test set one (`testkit::seams`).
    #[cfg(feature = "testkit")]
    let double = crate::testkit::seams::return_at_cleanup_double();
    #[cfg(not(feature = "testkit"))]
    let double: Option<fn(&mut EngineSink, PlayerId)> = None;
    match double {
        Some(double) => double(sink, player),
        None => crate::animated::return_at_cleanup(sink, player),
    }
}

// The end of a turn, and the remainder it owes when something pauses (§2.2, R62, R113, R117)

/// R113: the `resume.hook` a paused end of turn parks under. No `Script` can hold the name, and `work.rs`
/// hands it to `run_owed_end_of_turn` (`work::run_work_item` raises on a hook nothing knows).
pub const END_OF_TURN_WORK: &str = "@endOfTurn";

/// Which part of R62's order is still owed: `triggers` has the trigger queue to finish before `turnEnded`
/// is emitted; `window` has had the trap window and owes its settle (§10.3), then the rest; `delayed`
/// owes the delayed effects still due; `cleanup` owes their settle, then cleanup and the rest; `next`
/// has had cleanup (the animated cards' return last, B3.1) and owes its settle, the cap and next turn.
const END_TRIGGERS_STEP: &str = "triggers";
const END_WINDOW_STEP: &str = "window";
const END_DELAYED_STEP: &str = "delayed";
const END_CLEANUP_STEP: &str = "cleanup";
const END_NEXT_STEP: &str = "next";

/// Whose turn a parked boundary (`{ player }`) belongs to, read back defensively: it came through JSON
/// (§10.1).
fn turn_player_of(data: &IndexMap<String, Value>) -> Option<PlayerId> {
    let player = data.get("player").and_then(Value::as_str);
    PLAYER_IDS.into_iter().find(|id| Some(id.as_str()) == player)
}

/// Park the rest of the end of turn (R113). The item lands at `state.workCursor`, past the pausing scope's
/// own item, so it sits behind that remainder (the window's owed traps first) and R62's order survives the
/// pause. R117: callers owe at the moment they pause, never in advance, so a `settle` inside one of
/// `end_turn`'s steps can neither take nor re-run them (pre-parking made a played card's Cry fire twice,
/// §10.5's driver, R1).
fn owe_end_of_turn(sink: &mut EngineSink, player: PlayerId, step: &str, due_before: Option<u32>) {
    crate::work::owe(sink, boundary_resume(END_OF_TURN_WORK, player, step, due_before));
}

/// End the active player's turn and start the next, unless the cap ends the game (§2.5, R2). R62's
/// order: end-of-turn triggers, the trap window (the ending player's traps first), the delayed effects,
/// cleanup, the turn cap. Each stage stops at a pause and owes the rest to `state.work`, so no delayed
/// effect resolves inside an unfinished window and cleanup never closes the log before its last traps.
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

/// R62 from the `turnEnded` event on: the trap window, then everything after it. `turnEnded` goes out
/// before the window because its traps read it (#18 Bread and Butter's `unspentMana`, before cleanup
/// closes the log). R100: `traps::TRAP_WINDOW_EVENTS` withholds it from `fire_traps_for`, so the window
/// is the one place a `turnEnded` trap fires; other triggers on it queue as usual.
fn end_of_turn_after_triggers(sink: &mut EngineSink, player: PlayerId) {
    let ended = GameEvent::TurnEnded {
        player,
        turn: sink.state.turn,
        unspent_mana: crate::query::unspent_mana_of(&*sink.state, player),
    };
    sink.events.push(ended.clone());

    // §2.2's trap window: a trap's prompt pauses the rest of it (§9.3) and `traps::run_trap_window`
    // parks the traps it never reached; the steps after the window are parked here, behind them.
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

/// §10.3 after the window: what its traps' events wake resolves at this turn's end, before R62's delayed
/// effects, not behind the next turn's start-of-turn hooks; a trigger answering Bread and Butter's token
/// that finishes the opponent wins the game now (§4.5 step 2).
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

/// R846 (Meditative #19.1): spend one owed extra turn of the player whose turn just ended; true when one
/// was owed, so their turn starts again. The count is `None` at 0, keeping it out of a game's hash
/// until one is owed.
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

/// §10.3 after cleanup: its events (My Pawn reaching its owner's graveyard, R152; "this turn" modifiers
/// expiring) reach the traps and the trigger queue and resolve before the turn cap and the next turn
/// (R62), not once that turn has begun or the cap has drawn the game.
fn end_of_turn_cleanup_settle(sink: &mut EngineSink, player: PlayerId) {
    crate::triggers::settle(sink, crate::triggers::SettleOptions::default());
    if sink.state.result.is_some() {
        return;
    }
    if sink.state.pending.is_some() {
        owe_end_of_turn(sink, player, END_NEXT_STEP, None);
        return;
    }

    // R155: a return Spell cast while cleanup's events were answered (a cast on draw a trigger made) was
    // flagged after cleanup cleared the flags; its return is over too, so it is cleared now, while this
    // turn's logs still name it, or it would come back at the end of its caster's next turn.
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

/// `work.rs`'s handler for a parked end of turn (`END_OF_TURN_WORK`), continued where it stopped (R113).
/// The `triggers` step settles the queue itself, as at the start (R62 puts the triggers before the
/// window). The item is off the queue by now (`work::take_work`), so that `settle` cannot re-run it.
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

// R36: `DrawOfferState` keeps the turn of a player's last offer and no count, so `can_offer_draw`
// plays one offer a turn; more needs a count on the state.
const _: () = assert!(
    DRAW_OFFERS_PER_TURN == 1,
    "R36: turn.rs's can_offer_draw plays one draw offer a turn"
);

/// R36: only the active player offers, once per turn, and a declined offer blocks 3 of their turns.
pub fn can_offer_draw(state: &GameState, player: PlayerId) -> bool {
    if state.active != player || state.phase != Phase::Main {
        return false;
    }
    let offer = &state.players[player].draw_offer;
    let offered_this_turn = offer.offered_turn == Some(state.turn);
    if i32::from(offered_this_turn) >= DRAW_OFFERS_PER_TURN {
        return false;
    }
    let blocked_until = offer.blocked_until.unwrap_or(0);
    state.players[player].turns_started >= blocked_until
}

pub fn offer_draw(sink: &mut EngineSink, player: PlayerId) {
    sink.state.players[player].draw_offer.offered_turn = Some(sink.state.turn);
    sink.events.push(GameEvent::DrawOffered { player });
}

/// R36: whether `player` has an offer to answer: the active opponent offered this turn, unanswered.
/// `legal_actions` and the reducer both ask this, so a declined offer is gone from both at once. R269: an
/// offer lapses as its turn ends (nothing clears it; the turn number moves on), and a lapsed offer is not a
/// declined one, so it blocks nothing.
pub fn has_standing_draw_offer(state: &GameState, player: PlayerId) -> bool {
    let offering = opponent_of(player);
    state.active == offering && state.players[offering].draw_offer.offered_turn == Some(state.turn)
}

/// R269: the player whose draw offer stands now, or `None`. `view_for` shows it to both seats (the offer
/// was public, `drawOffered`), so each client, reconnected or not, can tell who waits and who answers.
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
    // §2.5, R36: an offer is answered once. The once-per-turn limit needs no record: a decline blocks
    // the next DRAW_OFFER_BLOCK_TURNS turns, this one included, and an accept ends the game.
    sink.state.players[offering].draw_offer.offered_turn = None;
    if accept {
        end_game(sink, Winner::Draw, GameOverReason::DrawAccepted);
        return;
    }
    // R36: the next DRAW_OFFER_BLOCK_TURNS turns of theirs are blocked, counting from the next one.
    let side = &mut sink.state.players[offering];
    side.draw_offer.blocked_until = Some(side.turns_started + DRAW_OFFER_BLOCK_TURNS + 1);
}
