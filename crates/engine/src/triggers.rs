//! The trigger registry, the trigger queue and the resolution loop of SPEC §10.3, in R68's order
//! (BUILD M3-T2). Three parts, read in the order the §10.3 diagram reads:
//!
//!  1. The registry — which cards can answer right now, keyed by hook and by the zone each card
//!     sits in. R153 is the whole of that second key: a card registers only the triggers its zone
//!     allows. On the field or in the backrow it answers `script.triggers` plus its `startOfTurn`,
//!     `endOfTurn`, `aura`, `setStat` and `onPlayHook` hooks; in a hand only `script.handTriggers`
//!     (#89 Corpse Eater); in a graveyard its `script.graveyardTriggers` (Classic #47, B5 E26) and the
//!     end-of-turn return of a `returnToHandAtEndOfTurn` spell of §5.1, which is how #23, #24 and #31
//!     come back at the end of the turn (R68); in a library only `script.deckTriggers` (Classic+ #37,
//!     B5 E26, R464); and in exile, in the resolving zone or dormant under a Stack, nothing at all
//!     (§3.2, R13).
//!     `aura` and `setStat` are not queued here at all — `layers.rs` reads them off the field
//!     directly — but they are field-only for the same reason the hooks are.
//!  2. The two queues in state. `state.dispatch` is §10.3's frontier: every event emitted but not
//!     yet offered to the traps and the registry, so an interrupted dispatch owes the rest in state
//!     rather than on the sink that dies with the action (§9.3). `state.trigger_queue` holds what the
//!     dispatch queued: an entry names the card and the trigger id and carries the event it answers,
//!     all plain JSON, so it survives an open prompt, a save and a replay — and R89 falls out of it,
//!     since every trigger but the Death hook runs after R78 has reset the instance, off the event it
//!     captured. Queue order is R68's: the active player's cards, then the opponent's; within a side
//!     unit lanes 1–5, then backrow 1–5, then hand, then library (R464: in the order the instances were
//!     created, never by library position, which is hidden), then graveyard. Traps are not queued at all —
//!     `traps.rs` fires them the moment the event is dispatched, which is what "a trap fires before a
//!     queued trigger" means (§10.3, BUILD M3-T2), and R100 keeps the end-of-turn window's events out
//!     of that immediate check so a `turnEnded` trap fires once per turn end. Either dispatch owes
//!     what a prompt stopped it from delivering, and neither drops it (R113): an immediate one parks
//!     an `OWED_TO_TRAPS` entry in front of every queued card trigger, because a trap is a response,
//!     while the scheduled end-of-turn window parks its remainder as `traps::TRAP_WINDOW_WORK` on
//!     `state.work`, which `settle` drains before the trigger queue moves at all. Delayed effects are
//!     never queued here: they resolve at their R62 point, which `turn.rs` owns through
//!     `modifiers::due_delayed`, in creation order.
//!  3. The loop — `settle`. Collect the new events, run whatever work is owed, dispatch one event,
//!     run the state check, then pop one trigger and apply it whole; repeat until both queues are
//!     empty and no prompt is open. R59 holds by construction: the check runs between whole effects
//!     and whole triggers, never between the hits of one, and never in the middle of one batch of
//!     emitted events.
//!
//! A trigger that opens a prompt stops the loop where it stands, and everything still owed is state:
//! `state.pending`, `state.work` (the interrupted sequence, R113), `state.dispatch` (the events not
//! yet offered) and `state.trigger_queue` (each entry with the event it captured). So the action
//! returns paused and the `answer` action finishes the job by calling `prompts::answer_prompt` and then
//! `settle` again (§9.3, §10.6). Nothing owed lives on the transient sink, which is the hole the M3
//! review's B-2 and B-3 found.
//!
//! Port of `packages/engine/src/triggers.ts` (part 3). TS's side-effect import of `./cryTrigger`
//! (B5 E13: the triggered-Cry sequence registered its prompt answerer at module scope, R122) is not
//! needed: `prompts.rs` matches the `"@triggerCry"` answerer key directly (SURFACE §6.6).
//!
//! TS told this action's events apart by object identity, with two module-level `WeakSet`s
//! (`dispatchedElsewhere`, `collected`) and `list.indexOf(event)`. Rust events are values, so the same
//! facts are positions in the action's event list, kept per call on the sink (`Frontier`, SURFACE
//! §3's "per-call state in `EngineSink`"): every sink over one action's list shares one `Frontier`
//! (`EngineSink::reborrow`), as every TS sink shared the module's sets.

use indexmap::{IndexMap, IndexSet};
use serde_json::{Value, json};

use crate::config::{BACKROW_ZONES, CAST_ON_DRAW_CHAIN_CAP, LIBRARY_CAP, UNIT_ZONES};
use crate::resolve::HookName;
use crate::script::{EngineSink, Script, TriggerDef, empty_script};
use crate::state::{CardInstance, DispatchItem, GameState, QueuedTrigger, Resume, find_instance};
use crate::stays::LaterMoves;
use crate::traps::{
    ImmediateDispatch, arrived_during_play, is_trap_type, is_trap_window_event, offer_event_to_traps,
    resume_event_to_traps, trap_controllers_of,
};
use crate::wire::{GameEvent, GameEventType, PLAYER_IDS, PlayerId, Row, Zone, ZoneRef};

/// The zones a card can hold a trigger from (§10.3). `library`: B5 E26's deck triggers (R464).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TriggerZone {
    Field,
    Backrow,
    Hand,
    Library,
    Graveyard,
}

impl TriggerZone {
    /// The literal TS writes, as a queue entry's data carries it.
    pub fn as_str(self) -> &'static str {
        match self {
            TriggerZone::Field => "field",
            TriggerZone::Backrow => "backrow",
            TriggerZone::Hand => "hand",
            TriggerZone::Library => "library",
            TriggerZone::Graveyard => "graveyard",
        }
    }
}

/// One card that can answer, with the triggers its current zone registers.
#[derive(Clone)]
pub struct TriggerHolder {
    pub card: CardInstance,
    /// The side the card sits on: a stolen trap answers for its new controller (R33).
    pub controller: PlayerId,
    pub zone: TriggerZone,
    /// A Trap or Field Trap in the backrow: a response, fired by `traps.rs`, never queued (§10.3).
    pub is_trap: bool,
    /// The event triggers this zone registers; empty for a card whose script has none.
    pub triggers: Vec<TriggerDef>,
    /// The face that is running, for the hook registry (§5.2).
    pub script: Script,
}

/// Every hook a script can register, in the registry's key order (BUILD M3-T2).
pub const TRIGGER_HOOKS: &[HookName] = &[
    HookName::Cry,
    HookName::Death,
    HookName::StartOfGame,
    HookName::StartOfTurn,
    HookName::StartOfOpponentTurn,
    HookName::EndOfTurn,
    HookName::Activate,
    HookName::OnPlayHook,
];

/// The `hook` of the one queue entry that is not a card's trigger: an event owed to the traps.
pub const OWED_TO_TRAPS: &str = "@traps";

/// How many times `settle` may go round before the board is called stuck (§10.3). Each pass resolves
/// something — a dispatch, owed work, a check or one queued trigger — so the cap has to hold every
/// trigger one legal action can set off, and R58 bounds that from the rules: "draw your whole
/// library" (#95) draws up to LIBRARY_CAP cards, each draw casts up to CAST_ON_DRAW_CHAIN_CAP cast-on-
/// draw cards, and each cast is a play (R70) every permanent on either side may answer (#33 Unstable
/// Clone Machine). A flat 1,000 threw on a Call to Chaos drawing 55 CN-Viruses beside a #33, a play
/// the rules bound at about 1,100 triggers. A board still going past this is a loop, not a game.
/// (Derived from config's numbers, as TS derived it here; CLAUDE.md rule 9.)
pub const SETTLE_PASS_CAP: usize =
    (LIBRARY_CAP * CAST_ON_DRAW_CHAIN_CAP * (UNIT_ZONES + BACKROW_ZONES)) as usize * PLAYER_IDS.len();

fn sides_of(state: &GameState) -> [PlayerId; 2] {
    if state.active == PlayerId::P1 {
        [PlayerId::P1, PlayerId::P2]
    } else {
        [PlayerId::P2, PlayerId::P1]
    }
}

// ---------------------------------------------------------------------------
// 1. The registry.
// ---------------------------------------------------------------------------

/// A Trap or Field Trap by its face (B2.7) — never by `faces::card_type_of`, which answers "Unit" for an
/// animated one standing in a unit zone, where it still fires as a trap (R383, `traps::is_trap_type`).
fn is_trap_card(state: &GameState, card: &CardInstance) -> bool {
    is_trap_type(state, card)
}

/// §10.3: which list a card's zone registers. A hand answers `handTriggers`, a library `deckTriggers`
/// and a graveyard `graveyardTriggers` (B5 E26); the field and the backrow answer `triggers`.
fn registered_triggers(script: &Script, zone: TriggerZone) -> &[TriggerDef] {
    match zone {
        TriggerZone::Hand => &script.hand_triggers,
        TriggerZone::Library => &script.deck_triggers,
        TriggerZone::Graveyard => &script.graveyard_triggers,
        TriggerZone::Field | TriggerZone::Backrow => &script.triggers,
    }
}

/// The one hook a graveyard card can still answer (R153): §5.1's end-of-turn return, nothing else.
const GRAVEYARD_HOOK: HookName = HookName::EndOfTurn;

/// §5.1 and R68: a spell whose text is "End of turn: add this back to your hand" is flagged
/// `returnToHandAtEndOfTurn` when it is played and answers its `endOfTurn` hook from the graveyard on
/// that turn alone (#23 Reoccurring Dream, #24 Efficiency Dividend, #31 KY's Math Equation).
///
/// R155 now sets that flag: §10.5 step 7 writes it as it sends to the graveyard a Spell whose
/// resolving face declares the return, and `cleanup` clears it at the end of the turn. So the flag
/// alone is the gate, and the turn-log fallback this used to carry is gone — it was strictly less
/// correct, because the log cannot tell a Spell that asked to return from a Unit with an end-of-turn
/// hook (#13 Jlockeed Shredder-10) that merely died on the turn it was played, nor from a Spell that
/// left the graveyard mid-resolution and was discarded back into it the same turn.
///
/// Everything else in a graveyard registers nothing: a spell left over from an earlier turn, and a
/// copy that arrived by being discarded or milled and was never played at all (R153).
fn flagged_for_return(card: &CardInstance) -> bool {
    card.return_to_hand_at_end_of_turn == Some(true)
}

/// R153: whether a holder's zone lets it register this hook at all. A card registers only the
/// triggers its zone allows — on the field or in the backrow its `triggers` plus every hook it
/// carries; in a hand only its `handTriggers`, so no hook; in a graveyard only the end-of-turn return
/// above. A library, exile or resolving card and a card dormant under a Stack never reach here:
/// `trigger_holders_of` and `trigger_holder_for` give them no holder at all (§3.2, R13).
///
/// Without this a Field Spell fired its `startOfTurn` from a HAND (#58 Rush Token Farm summoning a
/// Rush Token onto a board it was never on) and #64 Gifted Program made a play Radiant from a hand —
/// neither an error, both a different game.
fn zone_registers_hook(_state: &GameState, holder: &TriggerHolder, hook: HookName) -> bool {
    if holder.zone == TriggerZone::Hand || holder.zone == TriggerZone::Library {
        return false;
    }
    if holder.zone == TriggerZone::Graveyard {
        return hook == GRAVEYARD_HOOK && flagged_for_return(&holder.card);
    }
    true
}

fn holder_of(state: &GameState, card: &CardInstance, zone: TriggerZone, controller: PlayerId) -> TriggerHolder {
    let script = crate::scripts::script_of(state, card);
    // R383: an animated Field Trap in a unit zone is a trap too — `traps.rs` fires it, and it is never
    // also queued (§10.3).
    let is_trap = (zone == TriggerZone::Backrow || zone == TriggerZone::Field) && is_trap_card(state, card);
    // R671: a hand card's enchantments may grant it a hand trigger its text does not print.
    let triggers: Vec<TriggerDef> = if zone == TriggerZone::Hand {
        crate::book_swap::granted_hand_triggers(card, registered_triggers(&script, zone)).to_vec()
    } else {
        registered_triggers(&script, zone).to_vec()
    };
    TriggerHolder {
        card: card.clone(),
        controller,
        zone,
        is_trap,
        triggers,
        script,
    }
}

/// `c17` → 17: an instance's place in creation order (`state::new_instance` numbers ids from
/// `next_id`); anything else sorts last (TS `/^c(\d+)$/`, else `MAX_SAFE_INTEGER`).
fn creation_number(id: &str) -> i64 {
    match id.strip_prefix('c') {
        Some(digits) if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) => {
            digits.parse::<i64>().unwrap_or(i64::MAX)
        }
        _ => i64::MAX,
    }
}

/// B5 E26, R464: a library's holders — the cards whose running face declares `deckTriggers`, and only
/// those, since a library card registers nothing else — in the order the instances were created. Never
/// library order: that is hidden from both players (§9.1, R223), and a trigger order that followed it
/// would act on it.
fn library_holders(state: &GameState, player: PlayerId) -> Vec<TriggerHolder> {
    let mut cards: Vec<&CardInstance> = state.players[player]
        .library
        .iter()
        .filter(|card| !crate::scripts::script_of(state, *card).deck_triggers.is_empty())
        .collect();
    // Stable, as TS's `Array.prototype.sort` (SURFACE §4.4.1).
    cards.sort_by(|a, b| {
        creation_number(&a.id)
            .cmp(&creation_number(&b.id))
            .then_with(|| a.id.cmp(&b.id))
    });
    cards
        .into_iter()
        .map(|card| holder_of(state, card, TriggerZone::Library, player))
        .collect()
}

/// One side's holders in R68's within-a-side order: unit lanes, backrow lanes, hand, library (R464),
/// graveyard. A library card is a holder only while it declares deck triggers (`library_holders`).
pub fn trigger_holders_of(state: &GameState, player: PlayerId) -> Vec<TriggerHolder> {
    let side = &state.players[player];
    let mut out: Vec<TriggerHolder> = Vec::new();
    for card in crate::zones::active_units_of(state, player) {
        out.push(holder_of(state, &card, TriggerZone::Field, player));
    }
    for slot in crate::zones::slots_of(player, Row::Backrow) {
        if let Some(card) = crate::zones::card_at(state, &slot) {
            out.push(holder_of(state, &card, TriggerZone::Backrow, player));
        }
    }
    for card in &side.hand {
        out.push(holder_of(state, card, TriggerZone::Hand, player));
    }
    out.extend(library_holders(state, player));
    for card in &side.graveyard {
        out.push(holder_of(state, card, TriggerZone::Graveyard, player));
    }
    out
}

/// Every card that could answer, in R68's order: the active player's side first, then the
/// opponent's. Unfiltered — a card with no script keeps its place, so a caller can read the order
/// itself — so filter with `triggers_on_event` or `trigger_holders_with_hook`.
pub fn cards_in_trigger_order(state: &GameState) -> Vec<TriggerHolder> {
    sides_of(state)
        .into_iter()
        .flat_map(|player| trigger_holders_of(state, player))
        .collect()
}

/// The holder a card is right now, or `None` when its zone registers nothing: an exile or resolving
/// card, and a card dormant under a Stack, which does not act (§3.2, R13). A library card holds its
/// deck triggers (B5 E26).
pub fn trigger_holder_for(state: &GameState, card: &CardInstance) -> Option<TriggerHolder> {
    match card.zone {
        Zone::Hand { player } => Some(holder_of(state, card, TriggerZone::Hand, player)),
        Zone::Library { player } => Some(holder_of(state, card, TriggerZone::Library, player)),
        Zone::Graveyard { player } => Some(holder_of(state, card, TriggerZone::Graveyard, player)),
        Zone::Field { player, row, lane } => {
            // B5 E21: a card dormant under a backrow pile registers nothing, as one under a unit pile does
            // (§3.2, R13, R447); a Unit a carrier holds is a unit on the field (R446).
            if row == Row::Backrow && crate::zones::is_carried(state, card) {
                return Some(holder_of(state, card, TriggerZone::Field, player));
            }
            if row == Row::Backrow {
                return if crate::zones::is_buried(state, card) {
                    None
                } else {
                    Some(holder_of(state, card, TriggerZone::Backrow, player))
                };
            }
            let on_top = crate::zones::card_at(state, &ZoneRef { player, row, lane })
                .is_some_and(|top| top.id == card.id);
            if !on_top {
                return None;
            }
            Some(holder_of(state, card, TriggerZone::Field, player))
        }
        _ => None,
    }
}

/// The holder's triggers that wake on this event type.
pub fn triggers_on_event(holder: &TriggerHolder, event_type: GameEventType) -> Vec<TriggerDef> {
    holder
        .triggers
        .iter()
        .filter(|def| def.on.contains(&event_type))
        .cloned()
        .collect()
}

/// Every card that answers this event type, in R68's order. Traps included: they fire first.
pub fn trigger_holders_for_event(state: &GameState, event_type: GameEventType) -> Vec<TriggerHolder> {
    cards_in_trigger_order(state)
        .into_iter()
        .filter(|holder| holder.triggers.iter().any(|def| def.on.contains(&event_type)))
        .collect()
}

/// The registry keyed by hook: every card whose zone lets it carry that hook, in R68's order. `only`
/// narrows it to one controller, which is what start-of-turn and end-of-turn hooks need — they fire
/// on their own controller's turn alone (§6.2).
///
/// R153 is the filter. This walk reaches the graveyard, which is how the `returnToHandAtEndOfTurn`
/// spells are found (R68) — but carrying a hook is not the same as being allowed to answer it, so a
/// card in a hand answers none and a card in a graveyard answers only that one return. Reading the
/// hook off the script alone was the bug: it let a Field Spell in a hand act on the board.
pub fn trigger_holders_with_hook(state: &GameState, hook: HookName, only: Option<PlayerId>) -> Vec<TriggerHolder> {
    let holders = match only {
        None => cards_in_trigger_order(state),
        Some(player) => trigger_holders_of(state, player),
    };
    holders
        .into_iter()
        .filter(|holder| {
            holder.script.hook_named(hook.as_str()).is_some() && zone_registers_hook(state, holder, hook)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 2. The queue.
// ---------------------------------------------------------------------------

/// The `step` a queued entry's resume carries. An entry is re-entered by trigger id or hook name,
/// never by step, so nothing reads this; it keeps the file out of §10.6's decision on whether a step
/// is a number or a named one.
const TRIGGER_STEP: &str = "trigger";

/// `state.next_seq` numbers a queue entry, as it numbers a modifier, a delayed effect and a parked
/// work item, so R68's creation order is one counter and ids stay deterministic under replay. The
/// `t` prefix keeps them apart from `prompts.rs`'s `q…` choice ids, which count on `next_id`.
///
/// R177: except the entry of a card in a hand or a library (B5 E26, R464). The counter also numbers
/// the modifiers, whose ids both seats read (R169), so a number a hidden card took would show in them:
/// #89 Corpse Eater answers every death from its owner's hand, even one it gains nothing from (a
/// token's, R11), and the next modifier's id would then tell the opponent the hand holds one. Nothing
/// orders or finds a queue entry by its number, so a hidden card's entry borrows the counter's current
/// value without moving it, told apart by the queue's length.
fn next_entry_id(state: &mut GameState, hidden: bool, prefix: &str) -> (String, u32) {
    let seq = state.next_seq;
    if hidden {
        return (format!("{prefix}{seq}.{}", state.trigger_queue.len()), seq);
    }
    state.next_seq += 1;
    (format!("t{seq}"), seq)
}

/// The event a queue entry captured, or `None` for a hook entry, which answers no event.
pub fn event_of_queued(entry: &QueuedTrigger) -> Option<GameEvent> {
    let captured = entry.resume.data.get("event")?;
    if !captured.is_object() {
        return None;
    }
    if !captured.get("type").is_some_and(Value::is_string) {
        return None;
    }
    serde_json::from_value::<GameEvent>(captured.clone()).ok()
}

fn owed_traps_of(entry: &QueuedTrigger) -> Vec<String> {
    match entry.resume.data.get("owed").and_then(Value::as_array) {
        Some(owed) => owed.iter().filter_map(|id| id.as_str().map(str::to_string)).collect(),
        None => Vec::new(),
    }
}

/// The field's departures when the dispatch an owed entry finishes began (`stays::exit_mark`).
fn owed_mark_of(state: &GameState, entry: &QueuedTrigger) -> u32 {
    entry
        .resume
        .data
        .get("exitsFrom")
        .and_then(Value::as_u64)
        .and_then(|mark| u32::try_from(mark).ok())
        .unwrap_or_else(|| crate::stays::exit_mark(state))
}

fn player_value(player: PlayerId) -> Value {
    Value::String(player.as_str().to_string())
}

/// Append one of a card's triggers to the queue, behind everything already waiting (R68).
///
/// R174, R212: the entry carries the cards its event names and the stays the event happened on —
/// the mark a play's event was emitted at (`stays::event_mark`), or the field's departures now, as it
/// is dispatched (`stays::event_stay_of`) — and the trigger runs with them whenever it pops
/// (`run_queued_trigger`). A trigger aimed at a card its event names, by the id it reads off the event,
/// is aimed at that card's stay: an earlier trigger on the same event that killed it, and the check
/// between the two that let Reborn put a body back (R59), leave the later one nothing to land on
/// (R83). Only those cards: one the trigger reads off the board as it resolves is judged from when
/// its run began, like any run's, so a Reborn body an earlier trigger made is on the board for it.
pub fn queue_trigger(
    sink: &mut EngineSink<'_>,
    holder: &TriggerHolder,
    def: &TriggerDef,
    event: &GameEvent,
) -> QueuedTrigger {
    let hidden = holder.zone == TriggerZone::Hand || holder.zone == TriggerZone::Library;
    let (id, seq) = next_entry_id(sink.state, hidden, "h");
    // TS `const marks: RunMarks = { eventStay: eventStayOf(state, event) }`, written as its JSON.
    let stay = crate::stays::event_stay_of(sink.state, event);
    let marks = json!({ "eventStay": serde_json::to_value(&stay).expect("an event stay is plain JSON (§10.1)") });
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(
        "event".to_string(),
        serde_json::to_value(event).expect("an event is plain JSON (§10.1)"),
    );
    data.insert("zone".to_string(), Value::String(holder.zone.as_str().to_string()));
    data.insert("controller".to_string(), player_value(holder.controller));
    data.insert(crate::work::RUN_MARKS_KEY.to_string(), marks);
    let entry = QueuedTrigger {
        id,
        seq,
        instance_id: holder.card.id.clone(),
        hook: def.id.clone(),
        resume: Resume {
            def_id: holder.card.def_id.clone(),
            hook: def.id.clone(),
            step: TRIGGER_STEP.to_string(),
            radiant: holder.card.radiant,
            instance_id: Some(holder.card.id.clone()),
            data,
        },
    };
    sink.state.trigger_queue.push(entry.clone());
    entry
}

/// Append one of a card's hooks to the queue. §6.2 and R62 resolve start-of-turn and end-of-turn
/// hooks "in queue order", so queueing them gives them what an event trigger gets for free: a state
/// check between each two (R59), and a pause that keeps the rest of them in state.
///
/// R177: a face-down trap's hook (Classic #65 Ace in the Hole's end-of-turn coin) is a hidden card's
/// entry too, so it borrows the counter like a hand card's: a number it took would tell the other seat
/// the face-down card carries that hook.
pub fn queue_hook(sink: &mut EngineSink<'_>, holder: &TriggerHolder, hook: HookName) -> QueuedTrigger {
    let hidden = holder.zone == TriggerZone::Backrow && crate::preview::is_face_down(sink.state, &holder.card);
    let (id, seq) = next_entry_id(sink.state, hidden, "h");
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert("zone".to_string(), Value::String(holder.zone.as_str().to_string()));
    data.insert("controller".to_string(), player_value(holder.controller));
    let entry = QueuedTrigger {
        id,
        seq,
        instance_id: holder.card.id.clone(),
        hook: hook.as_str().to_string(),
        resume: Resume {
            def_id: holder.card.def_id.clone(),
            hook: hook.as_str().to_string(),
            step: TRIGGER_STEP.to_string(),
            radiant: holder.card.radiant,
            instance_id: Some(holder.card.id.clone()),
            data,
        },
    };
    sink.state.trigger_queue.push(entry.clone());
    entry
}

/// Queue one hook across the board in R68's order (§6.2, R62). The caller runs `settle` to drain it,
/// which is what puts a state check between each two hooks and lets one that prompts pause the rest.
pub fn queue_hooks_in_trigger_order(
    sink: &mut EngineSink<'_>,
    hook: HookName,
    only: Option<PlayerId>,
) -> Vec<QueuedTrigger> {
    let holders = trigger_holders_with_hook(sink.state, hook, only);
    holders.iter().map(|holder| queue_hook(sink, holder, hook)).collect()
}

/// Queue one hook across the board and resolve it, in R68's order (§6.2, R62, R59).
pub fn run_hooks_in_trigger_order(sink: &mut SettleSink<'_>, hook: HookName, only: Option<PlayerId>) {
    queue_hooks_in_trigger_order(sink, hook, only);
    settle(sink, SettleOptions::default());
}

/// Park an event the traps have not finished answering. `owed` names the traps that still have to
/// see it, so a resume neither re-fires one that already fired nor wakes a Field Trap twice (§5.1).
/// It goes in front of every queued card trigger, because a trap is a response (§10.3).
fn owed_to_traps(sink: &mut EngineSink<'_>, event: &GameEvent, run: &ImmediateDispatch) -> Option<QueuedTrigger> {
    if run.owed.is_empty() {
        return None;
    }
    let mut controllers: serde_json::Map<String, Value> = serde_json::Map::new();
    for id in &run.owed {
        if let Some(player) = run.controllers.get(id) {
            controllers.insert(id.clone(), player_value(*player));
        }
    }
    // R177: whether this entry exists at all hangs on which traps are still owed the event, and a
    // face-down one is read by its controller alone (R33) — a second #96 watches a declaration where
    // a Sheepish does not — so it takes no number from the counter the modifiers' ids come from.
    let (id, seq) = next_entry_id(sink.state, true, "o");
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(
        "event".to_string(),
        serde_json::to_value(event).expect("an event is plain JSON (§10.1)"),
    );
    data.insert(
        "owed".to_string(),
        Value::Array(run.owed.iter().map(|id| Value::String(id.clone())).collect()),
    );
    data.insert("exitsFrom".to_string(), Value::from(run.mark));
    data.insert("controllers".to_string(), Value::Object(controllers));
    let entry = QueuedTrigger {
        id,
        seq,
        instance_id: String::new(),
        hook: OWED_TO_TRAPS.to_string(),
        resume: Resume {
            def_id: String::new(),
            hook: OWED_TO_TRAPS.to_string(),
            step: TRIGGER_STEP.to_string(),
            radiant: false,
            instance_id: None,
            data,
        },
    };
    let queue = &mut sink.state.trigger_queue;
    match queue.iter().position(|queued| queued.hook != OWED_TO_TRAPS) {
        None => queue.push(entry.clone()),
        Some(at) => queue.insert(at, entry.clone()),
    }
    Some(entry)
}

/// §10.3 step C: the traps see the event first and fire immediately, to completion. A prompt for a
/// trap's owner stops that dispatch — "pauses the opponent's action until answered" (BUILD M3-T2) —
/// so whatever the other traps are still owed is parked in state for the answer to finish: exactly
/// the traps the dispatch never reached (`traps::offer_event_to_traps`), in its order. One that met the
/// event and declined it has had its look (R99), and is not owed it again.
/// R62's end-of-turn window is not an immediate dispatch at all: `turn.rs` fires those traps with
/// `traps::run_trap_window` at the scheduled point, so `turnEnded` is passed over here.
///
/// `at` is the event's place in this action's list, when it is on it (TS read it by identity).
fn offer_to_traps(sink: &mut EngineSink<'_>, event: &GameEvent, at: Option<usize>) -> Option<QueuedTrigger> {
    if is_trap_window_event(event) {
        return None;
    }
    // R212: the board the event happened on, read off the events that happened after it.
    let run = offer_event_to_traps(sink, event, &|seen: &EngineSink<'_>| {
        crate::stays::moves_in(&events_after_dispatched(seen, at), Some(&*seen.state))
    });
    if sink.state.result.is_some() {
        return None;
    }
    sink.state.pending.as_ref()?;
    owed_to_traps(sink, event, &run)
}

/// Finish a trap dispatch a prompt interrupted, and park again if another trap prompts. Each trap
/// meets the event as the board now stands (`traps::standing_event`, R174): a played card the answer to
/// an earlier trap's question killed is no longer in play for the next one, even though it died in a
/// later action than the dispatch began in (#85 fuses nothing out of a graveyard, R61).
fn run_owed_traps(sink: &mut EngineSink<'_>, event: &GameEvent, entry: &QueuedTrigger) {
    let owed = owed_traps_of(entry);
    let mark = owed_mark_of(sink.state, entry);
    let controllers = trap_controllers_of(entry.resume.data.get("controllers"));
    let run = resume_event_to_traps(sink, event, &owed, mark, &controllers);
    if sink.state.result.is_some() || sink.state.pending.is_none() {
        return;
    }
    owed_to_traps(sink, event, &run);
}

/// R212: the events that happened after the one being dispatched, in the order they happened.
///
/// The action's event list is in emission order, so everything after the event on it happened after
/// it, whichever loop has taken it since or whether any loop ever will: the rest of the frontier, what
/// the traps answering the event have emitted, what a cast's step-4 window inside one of them has
/// collected and dispatched on its own context (R70), and #96's AI turn, which the `reduce` of each
/// of its actions dispatches before the playout copies it onto this list (`mark_dispatched`, R44,
/// R168) — a card the AI turn drew, played or brought back is on a stay that did not see the event
/// all the same. `combat::queue_declaration_triggers` reads the window's declaration the same way.
///
/// An event not on this list is an earlier action's, still owed because a prompt paused the frontier
/// (§9.3), so every event of this action happened after it, and so did what is owed behind it.
///
/// TS found the event with `list.indexOf(event)` and the owed events that are this action's own with
/// a set of the list's objects; here `at` is the event's position (`None`: not on the list, TS's
/// `-1`), and an owed item is the list's own when this action collected it (`Frontier.positions`) or
/// it is one of the events a nested `reduce` dispatched before they were copied onto the list.
fn events_after_dispatched(sink: &EngineSink<'_>, at: Option<usize>) -> Vec<GameEvent> {
    let list: &[GameEvent] = &sink.events[..];
    let frontier = sink.frontier.get();
    let listed = |item: &DispatchItem| -> bool {
        if frontier.positions.get(&item.seq).is_some_and(|&position| position < list.len()) {
            return true;
        }
        frontier
            .elsewhere
            .iter()
            .any(|&position| list.get(position) == Some(&item.event))
    };
    let mut out: Vec<GameEvent> = sink
        .state
        .dispatch
        .iter()
        .filter(|item| !listed(item))
        .map(|item| item.event.clone())
        .collect();
    let from = at.map_or(0, |at| at + 1).min(list.len());
    out.extend(list[from..].iter().cloned());
    out
}

/// Offer one event to the traps and then queue every other trigger that wakes on it, in R68's order
/// (§10.3 steps C and D). Queueing only reads the board, so this can never pause halfway through the
/// queueing itself — which is what makes the frontier safe to flush into state the instant a prompt
/// opens somewhere else.
///
/// R212: the board an event is offered to is the board as it stood when the event happened, and the
/// events owed behind it say how the board has moved since. The loop dispatches an event after
/// whatever ran first — the state check a combat, an Echo repeat or a whole Cry is followed by (§4.5,
/// §10.3), and the Death hooks and Reborn inside it — so a card that has moved zones since the event
/// happened is on a stay that did not see it: a Reborn body does not answer the hit that killed the
/// unit or the kill its last stay made (R174), and a card drawn since answers no death from before it
/// reached the hand (§8 #89). A card whose controller has changed since answers for the player who
/// controlled it then: #32 Prem Panther's kill is its controller's as the kill happens, even when the
/// victim's Death then steals the Panther (#86).
///
/// The event's place in the action's list is found by value here; the loop itself knows it exactly
/// (`dispatch_new_events`), since TS found it by identity.
pub fn dispatch_event(sink: &mut EngineSink<'_>, event: &GameEvent) -> Vec<QueuedTrigger> {
    let at = sink.events.iter().position(|listed| listed == event);
    dispatch_event_at(sink, event, at)
}

fn dispatch_event_at(sink: &mut EngineSink<'_>, event: &GameEvent, at: Option<usize>) -> Vec<QueuedTrigger> {
    // R240, R63: a hit of 0 is a report (an absorbed fatigue draw), not a damage instance, and nothing
    // — no trap, no trigger — answers it.
    if let GameEvent::Damage { amount, .. } = event
        && *amount <= 0
    {
        return Vec::new();
    }
    // B5 E33, R404: a quest counts the events the loop reaches, in the order they happened, before
    // anything answers this one — so a quest that opened later in the stream never counts it.
    crate::subsystems::quests::observe_quest_event(sink, event, &|seen: &EngineSink<'_>| {
        events_after_dispatched(seen, at)
    });
    let mut queued: Vec<QueuedTrigger> = Vec::new();
    if let Some(owed) = offer_to_traps(sink, event, at) {
        queued.push(owed);
    }

    let wakes = event.event_type();
    let mut later: Option<LaterMoves> = None;
    let mut uncovered: Option<Vec<String>> = None;
    for holder in cards_in_trigger_order(sink.state) {
        // §10.3: the traps have already had this event; queueing them too would fire them twice.
        if holder.is_trap {
            continue;
        }
        let defs = triggers_on_event(&holder, wakes);
        if defs.is_empty() {
            continue;
        }
        let moves = later.get_or_insert_with(|| {
            crate::stays::moves_in(&events_after_dispatched(&*sink, at), Some(&*sink.state))
        });
        if moves.moved.contains(&holder.card.id) {
            continue;
        }
        // §3.2, R153: nor does the card the event's own removal uncovered in its Stack pile — dormant
        // when it happened, it resumed because of it, as a Reborn body returns because of a death.
        let uncovered = uncovered.get_or_insert_with(|| crate::stays::uncovered_by(&*sink.state, event));
        if uncovered.contains(&holder.card.id) {
            continue;
        }
        // R119: a permanent that arrived on the field while the play resolved does not answer that play.
        if arrived_during_play(event).contains(&holder.card.id) {
            continue;
        }
        let controller = moves
            .controller_before
            .get(&holder.card.id)
            .copied()
            .unwrap_or(holder.controller);
        let holder = TriggerHolder { controller, ..holder };
        for def in &defs {
            queued.push(queue_trigger(sink, &holder, def, event));
        }
    }
    // R212: the removal this event reports, if any, has been answered (`stays::note_reported`).
    crate::stays::note_reported(sink.state, event);
    queued
}

// ---------------------------------------------------------------------------
// 3. The loop.
// ---------------------------------------------------------------------------

/// Apply one queued entry whole (§10.3 step A). The card is found again now rather than trusted
/// from queue time: it may have died, been played or moved while it waited, and a trigger its
/// current zone no longer registers fizzles instead of firing from the wrong zone (R78, R89).
pub fn run_queued_trigger(sink: &mut EngineSink<'_>, entry: &QueuedTrigger) {
    let event = event_of_queued(entry);
    if entry.hook == OWED_TO_TRAPS {
        if let Some(event) = event {
            run_owed_traps(sink, &event, entry);
        }
        return;
    }

    let Some(card) = find_instance(sink.state, &entry.instance_id).cloned() else {
        return;
    };
    let Some(holder) = trigger_holder_for(sink.state, &card) else {
        return;
    };

    let Some(event) = event else {
        // A hook entry: §6.2's start-of-turn and end-of-turn triggers, queued in R68's order.
        let Some(hook) = TRIGGER_HOOKS.iter().copied().find(|name| name.as_str() == entry.hook) else {
            return;
        };
        if holder.script.hook_named(hook.as_str()).is_none() {
            return;
        }
        // R153, and the same re-reading as a trigger above: a card the queue caught on the field and
        // that is in a hand or a graveyard by the time its entry pops no longer registers this hook.
        if !zone_registers_hook(sink.state, &holder, hook) {
            return;
        }
        // §6.2: a start- or end-of-turn hook is its CONTROLLER's, on its controller's own turn — which
        // is why the queue was built for one player (`trigger_holders_with_hook`'s `only`). A card that
        // changed sides while its entry waited (a Death earlier in the same queue stole it) is now the
        // other player's, on a turn that is not theirs, so the entry fizzles rather than firing for them.
        if let Some(queued_for) = entry.resume.data.get("controller").and_then(Value::as_str)
            && holder.controller.as_str() != queued_for
        {
            return;
        }
        crate::prompts::run_hook_resumable(
            sink,
            &card,
            hook.as_str(),
            crate::prompts::HookResumableOptions {
                controller: Some(holder.controller),
                ..Default::default()
            },
        );
        return;
    };

    let Some(def) = queued_trigger_def(&holder, entry) else {
        return;
    };
    if !def.on.contains(&event.event_type()) {
        return;
    }
    // B5 E1, R448: a response to an announce whose play a Counter has already cancelled, or that step 4
    // has already moved, finds no card and fizzles (`play_steps::run_announce_window` pops these itself).
    if let GameEvent::CardAnnounced { instance_id, .. } = &event
        && !crate::announce::is_announce_live(sink.state, instance_id)
    {
        return;
    }
    // R212: an event trigger answers for the player who controlled its card when the event happened,
    // which is what its entry captured — a change of control since does not hand the answer over.
    let queued_for = entry.resume.data.get("controller").and_then(Value::as_str);
    let controller = PLAYER_IDS
        .into_iter()
        .find(|player| Some(player.as_str()) == queued_for)
        .unwrap_or(holder.controller);
    // R174, R212: the cards the event names are judged from the stays it happened on, which the entry
    // captured as it was queued; the run itself begins now (`make_context`).
    let event_stay = crate::work::run_marks_of(&entry.resume.data).and_then(|marks| marks.event_stay);
    // Resumable, so a prompt inside the list stops the list there instead of being stepped over. The
    // parked tail names the trigger's id as its hook, and `work::script_step_for` re-enters a trigger by
    // its id, rebuilding its list from the event the entry captured (R113); the answer itself goes to
    // the card's `resume` table, where every §6.3 choose effect sends it.
    //
    // §5.2, R113: the list is the face the card wears NOW (the context and `holder.triggers` read it),
    // and a paused list goes on in the list it began, so its tail is rebuilt from that same face and
    // definition — not the ones the entry recorded when it was queued, which an earlier trigger in the
    // same queue can have made Radiant since.
    let plan = crate::work::WorkPlan::new(
        Resume {
            hook: def.id.clone(),
            def_id: card.def_id.clone(),
            radiant: card.radiant,
            ..entry.resume.clone()
        },
        controller,
    );
    let mut ctx = crate::resolve::make_context(
        sink,
        Some(&card),
        crate::resolve::HookOptions {
            controller: Some(controller),
            data: Some(crate::work::card_data(&entry.resume.data)),
            ..Default::default()
        },
    );
    if event_stay.is_some() {
        ctx.event_stay = event_stay;
    }
    let effects = (def.run)(&mut ctx, &event);
    crate::prompts::apply_resumable(&mut ctx, &plan, effects, None);
}

/// The trigger a queue entry names, on the card as it stands now. R77 keeps the instance a Fuse
/// lands on — same card, same stay (`stays::moves_in` counts it as unmoved) — and R102 keeps its text,
/// but the fused definition namespaces each ingredient's trigger ids (`<ingredientDefId>:<id>`), so
/// an entry the card queued before the Fuse, under its old definition, is found under that
/// definition's namespace: Fed Fauci hit by the Cry that Unlicensed Experimentation then fused onto
/// it still owes its Plague Counter (R212).
fn queued_trigger_def(holder: &TriggerHolder, entry: &QueuedTrigger) -> Option<TriggerDef> {
    if let Some(exact) = holder.triggers.iter().find(|candidate| candidate.id == entry.hook) {
        return Some(exact.clone());
    }
    let namespaced = format!("{}:{}", entry.resume.def_id, entry.hook);
    holder
        .triggers
        .iter()
        .find(|candidate| candidate.id == namespaced)
        .cloned()
}

/// §10.3 steps B to D without step G: the events emitted so far reach the traps, which fire at once
/// and to completion, and every other trigger they wake is queued — but nothing queued is popped, and
/// no owed work is drained. A trap's remainder a prompt left owed (`OWED_TO_TRAPS`) is a response, so
/// it is finished here, in front of everything. This is what a stage made of several whole effects in
/// a row needs between two of them: R62's delayed effects are each a whole effect (R59), and a trap
/// answering the first responds before the second resolves, while the triggers they wake wait for the
/// stage's own loop (R68). A cast's §10.5 step 4 is the same kind of point inside the effect that cast
/// it (R70): Sheepish answers a cast Unit there, before its Cry, and the effect's own loop keeps the
/// rest. Stops at a prompt, leaving the rest owed in state.
pub fn dispatch_pending(sink: &mut SettleSink<'_>) {
    for _pass in 0..SETTLE_PASS_CAP {
        dispatch_new_events(sink);
        if sink.state.pending.is_some() || sink.state.result.is_some() {
            return;
        }
        // An unfinished trap dispatch is the only thing that holds the frontier back, and its entry is at
        // the head of the queue (`owed_to_traps`); anything else there waits for the stage's loop.
        match sink.state.trigger_queue.first() {
            Some(head) if head.hook == OWED_TO_TRAPS => {}
            _ => return,
        }
        let head = sink.state.trigger_queue.remove(0);
        run_queued_trigger(sink, &head);
    }
    panic!("the trap dispatch did not settle in {SETTLE_PASS_CAP} passes (§10.3)");
}

/// The sink the loop runs on (TS `EngineSink & { dispatched?: number }`). TS's `dispatched` counted
/// how many of this action's `events` had been taken into `state.dispatch`: a copy position in the
/// sink's own array, and nothing more. What is *owed* is the frontier in state, never this number — an
/// action that returns paused has already handed every event it emitted to `state.dispatch`, so the
/// answer action finishes the dispatch (§9.3, §10.1, R122). Keeping the frontier itself here was the
/// hole the M3 review's B-2 and B-3 found. In Rust the position is `Frontier.collected`, shared by
/// every sink over the action's list (`dispatched`, `set_dispatched`).
pub type SettleSink<'a> = EngineSink<'a>;

/// TS's per-action bookkeeping of which of the action's events the frontier has taken, which TS kept
/// by object identity (`SettleSink.dispatched` and the module's `collected` and `dispatchedElsewhere`
/// `WeakSet`s) and Rust keeps by position in the action's event list. It decides only what
/// `collect_events` takes — R212's reading of what happened after an event (`events_after_dispatched`)
/// reads the list itself — and it holds no game state: it lives on the sink and dies with the action.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Frontier {
    /// Every event of the action's list below this position has been taken into `state.dispatch`, or
    /// is one delivered some other way (TS: `dispatched` on the action's sink, and `collected` for a
    /// loop on any other sink over the same list — a cast's §10.5 step-4 window runs inside an effect,
    /// whose context is the only sink it has (`effects::draw`, #95), and an event is taken into the
    /// frontier once, whichever sink reaches it first).
    pub collected: usize,
    /// Positions of events the frontier must not collect because they are delivered some other way
    /// (TS `dispatchedElsewhere`). Two kinds are: #96 My Pawn's AI turn drives `reduce` once per
    /// action, and each of those settles its own events before the playout copies them onto the
    /// enclosing action's list so the client is told about them (R168), so the enclosing loop must not
    /// hand them to the traps and the trigger queue again (§10.3: an event is offered once), or every
    /// trigger of the AI turn fires twice; and a player's `attackDeclared`, which §4.2 step 4's window
    /// delivers itself (`combat.rs`'s `withhold_from_frontier`).
    pub elsewhere: IndexSet<usize>,
    /// The position in the action's list of each event this action took into `state.dispatch`, by its
    /// dispatch item's `seq`: TS's identity of a frontier item with the list's own object.
    pub positions: IndexMap<u32, usize>,
}

/// Where a sink keeps its `Frontier`: its own, for the sink an action begins with
/// (`EngineSink::new`), or its parent's, for every sink borrowed from it (`EngineSink::reborrow`, which
/// `resolve::make_context` builds each context with), so a loop on a context and the action's own loop
/// see one frontier, as TS's module-level sets were one.
#[derive(Debug)]
pub enum FrontierSlot<'a> {
    Own(Frontier),
    Shared(&'a mut Frontier),
}

impl Default for FrontierSlot<'_> {
    fn default() -> Self {
        FrontierSlot::Own(Frontier::default())
    }
}

impl FrontierSlot<'_> {
    pub fn get(&self) -> &Frontier {
        match self {
            FrontierSlot::Own(frontier) => frontier,
            FrontierSlot::Shared(frontier) => &**frontier,
        }
    }

    pub fn get_mut(&mut self) -> &mut Frontier {
        match self {
            FrontierSlot::Own(frontier) => frontier,
            FrontierSlot::Shared(frontier) => &mut **frontier,
        }
    }
}

/// TS `sink.dispatched ?? 0`: how far into the action's list the frontier has taken events.
pub fn dispatched(sink: &EngineSink<'_>) -> usize {
    sink.frontier.get().collected
}

/// TS `sink.dispatched = at`: the frontier counts the events below `at` as taken (`combat.rs`'s
/// `withhold_from_frontier`, which moves it past the declaration the window delivers itself).
pub fn set_dispatched(sink: &mut EngineSink<'_>, at: usize) {
    sink.frontier.get_mut().collected = at;
}

/// Mark events a nested `reduce` has already dispatched (see `Frontier.elsewhere`). TS marked the
/// objects; here they are the entries of the action's list that are these events — its tail, where
/// both callers have just pushed them, or else the latest unmarked entry equal to each.
pub fn mark_dispatched(sink: &mut EngineSink<'_>, events: &[GameEvent]) {
    let len = sink.events.len();
    let count = events.len();
    if count <= len && &sink.events[len - count..] == events {
        let frontier = sink.frontier.get_mut();
        for at in len - count..len {
            frontier.elsewhere.insert(at);
        }
        return;
    }
    for event in events {
        let found = (0..len).rev().find(|&at| {
            sink.events[at] == *event && !sink.frontier.get().elsewhere.contains(&at)
        });
        if let Some(at) = found {
            sink.frontier.get_mut().elsewhere.insert(at);
        }
    }
}

/// §10.1: an emitted event joins the frontier — "events still owed to the traps and the trigger
/// queue" — in emission order, with an id and `seq` from `state.next_seq` (R68's one counter), so the
/// frontier a replay builds is the frontier the live game had.
fn collect_events(sink: &mut SettleSink<'_>) {
    // R437: a mark whose effect has stopped waiting goes, and says so, before the frontier moves; R750:
    // a delayed destroy of a scope marks the Units it names now.
    crate::effects::delay::refresh_scope_marks(sink);
    crate::marks::sweep_marks(sink);
    let len = sink.events.len();
    let from = sink.frontier.get().collected;
    for at in from..len {
        sink.frontier.get_mut().collected = at + 1;
        if sink.frontier.get().elsewhere.contains(&at) {
            continue;
        }
        let seq = sink.state.next_seq;
        sink.state.next_seq += 1;
        sink.frontier.get_mut().positions.insert(seq, at);
        let event = sink.events[at].clone();
        sink.state.dispatch.push(DispatchItem {
            id: format!("e{seq}"),
            seq,
            event,
        });
    }
}

/// Whether an event's own trap dispatch is unfinished, so the frontier may not advance (§10.3).
fn traps_still_owed(state: &GameState) -> bool {
    state.trigger_queue.iter().any(|entry| entry.hook == OWED_TO_TRAPS)
}

/// Turn the frontier into trap firings and queue entries (§10.3 steps B, C, D), oldest event first,
/// taking each event off `state.dispatch` before offering it so no event is ever offered twice.
///
/// The frontier stops advancing while the event at its head cannot be finished: a prompt is open (or
/// the game is over), or the previous event's traps have not all seen it — an `OWED_TO_TRAPS` entry
/// says so, and `settle` pops that before this runs again. Both cases are the same rule: the traps
/// see the events in the order they were emitted, so a later event waits in state rather than
/// jumping the queue or, as it did while the frontier lived on the sink, being dropped on a pause.
fn dispatch_new_events(sink: &mut SettleSink<'_>) {
    // Collect first and unconditionally, so a pause below leaves nothing owed on the sink (§9.3).
    collect_events(sink);
    while !sink.state.dispatch.is_empty() {
        if sink.state.pending.is_some() || sink.state.result.is_some() {
            return;
        }
        if traps_still_owed(sink.state) {
            return;
        }
        // R58: a cast-on-draw draw's `drawn` waits, in its place, until the draw is complete — its cast
        // resolved (`draw_complete.rs`) — and the events after it go on meanwhile.
        let state: &GameState = &*sink.state;
        let Some(at) = state
            .dispatch
            .iter()
            .position(|item| !crate::draw_complete::held_back(state, &item.event))
        else {
            return;
        };
        let next = sink.state.dispatch.remove(at);
        let position = sink.frontier.get().positions.get(&next.seq).copied();
        dispatch_event_at(sink, &next.event, position);
        collect_events(sink);
    }
}

/// The resolution loop of §10.3: dispatch, drain parked work, state check, pop one trigger, repeat
/// until the queue is empty and no prompt is open. Call it after any action, after any answer, and
/// anywhere §2.2 opens a window — the end-of-turn trap window is just the `turnEnded` event reaching
/// `traps::run_trap_window`, and R62's delayed effects are `turn.rs`'s, on either side of it.
///
/// Safe to call more than once on the same sink, and safe to call on a fresh sink over a state a
/// previous action left mid-dispatch: the frontier keeps this sink from copying an event twice, and
/// `state.dispatch` holds the events still owed, so nothing is offered twice and nothing is skipped.
///
/// `options` is TS's optional second argument: `SettleOptions::default()` where TS passed none.
pub fn settle(sink: &mut SettleSink<'_>, options: SettleOptions) {
    // §4.5: the check follows something that resolved. Held, it waits until this loop has resolved a
    // piece of owed work or a queued trigger (a trap runs its own check as it fires, `traps::fire_trap`).
    let mut check_due = options.hold_check != Some(true);
    for _pass in 0..SETTLE_PASS_CAP {
        // First, so a pause below leaves nothing owed on the sink (§9.3).
        dispatch_new_events(sink);
        if sink.state.result.is_some() {
            return;
        }
        // Waiting on an answer: the queue keeps its place and the `answer` action settles again.
        if sink.state.pending.is_some() {
            return;
        }

        // A sequence a prompt interrupted finishes before the queue moves on: it is inside the trigger
        // that is already running (§10.6).
        if !sink.state.work.is_empty() {
            let parked = sink.state.work.len();
            crate::work::drain_work(sink);
            check_due = true;
            if sink.state.work.len() < parked {
                continue;
            }
        }

        if check_due {
            let emitted = sink.events.len();
            crate::state_check::state_check(sink);
            if sink.state.result.is_some() {
                return;
            }
            // Deaths, Reborn and Death triggers spoke: their events are dispatched before anything pops.
            if sink.events.len() > emitted {
                continue;
            }
        }

        if sink.state.trigger_queue.is_empty() {
            return;
        }
        let next = sink.state.trigger_queue.remove(0);
        run_queued_trigger(sink, &next);
        check_due = true;
    }
    panic!("the resolution loop did not settle in {SETTLE_PASS_CAP} passes (§10.3)");
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SettleOptions {
    /// §10.5 step 4's loop, which runs so that a trap answering the play (Sheepish) fires before the
    /// Cry (R17). Nothing has resolved yet when it starts — the play resolves at step 5 — and §4.5 runs
    /// the state check "after every resolved action, every fully resolved effect or trigger … and every
    /// combat", so this loop runs it only once something has: a trap (which checks as it fires), owed
    /// work, or a queued trigger. A card that arrives at 0 or less health — a 6/1 Bigot under #46
    /// Suppressive Aura — therefore still resolves its Cry, and the check after step 6 collects it:
    /// R118 loses the Cry only where a trap has taken the card off the field.
    pub hold_check: Option<bool>,
}
