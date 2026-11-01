//! The trigger registry, the trigger queue and the resolution loop of SPEC §10.3, in R68's order
//! (BUILD M3-T2):
//!
//!  1. The registry: which cards can answer now, keyed by hook and by zone (R153: a card registers only
//!     the triggers its zone allows). The field and the backrow answer `script.triggers` and the hooks;
//!     a hand `handTriggers` (#89 Corpse Eater); a graveyard `graveyardTriggers` (Classic #47, B5 E26)
//!     and §5.1's end-of-turn return (#23, #24, #31; R68); a library `deckTriggers` (Classic+ #37,
//!     R464); exile, the resolving zone and a card dormant under a Stack nothing (§3.2, R13).
//!  2. The two queues live in state, so what a prompt interrupts outlives the action's sink (§9.3):
//!     `state.dispatch` holds the events not yet offered, and a `state.trigger_queue` entry its event
//!     as plain JSON, so every trigger but Death runs off its captured event after R78's reset (R89).
//!     Queue order is R68's: the active side first; within a side unit lanes 1–5, backrow 1–5, hand,
//!     library (R464: by creation, never by hidden library position), graveyard. Traps are never
//!     queued: `traps.rs` fires them as the event is dispatched (§10.3; R100 keeps the end-of-turn
//!     window's events out). A prompt never drops what a dispatch owes (R113): an immediate one parks
//!     `OWED_TO_TRAPS` ahead of every card trigger, the end-of-turn window `traps::TRAP_WINDOW_WORK` on
//!     `state.work`, which `settle` drains first.
//!  3. The loop, `settle`: collect, run owed work, dispatch one event, run the state check, pop one
//!     trigger and apply it whole, until both queues are empty and no prompt is open. R59 holds by
//!     construction: the check never runs between the hits of one effect or inside one event batch.
//!
//! A prompt stops the loop with everything owed in state, and `answer` runs `prompts::answer_prompt`
//! then `settle` (§9.3, §10.6). `prompts.rs` matches the `"@triggerCry"` answerer key (B5 E13, R122,
//! SURFACE §6.6). An action's events are told apart by position in its event list, kept in one
//! `Frontier` every sink over that list shares (SURFACE §3).

use indexmap::{IndexMap, IndexSet};
use serde_json::{Value, json};

use crate::config::{BACKROW_ZONES, CAST_ON_DRAW_CHAIN_CAP, LIBRARY_CAP, UNIT_ZONES};
use crate::resolve::HookName;
use crate::script::{EngineSink, Script, TriggerDef};
use crate::state::{
    CardInstance, DispatchItem, GameState, QueuedTrigger, Resume, active_first, find_instance,
};
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
    /// The literal a queue entry's data carries.
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
    /// The face that is running, for the hook registry (§5.2), borrowed from the registry.
    pub script: crate::scripts::ScriptRef,
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

/// MD-B22, R946: the `hook` of a lane watcher's queue entry (Showdown, Meditative #98, Radiant face).
pub const LANE_WATCH: &str = "@laneWatch";

/// R1863000: the `hook` of a play watcher's queue entry (Great Replacement Theory, Stinkyscape #44).
pub const PLAY_WATCH: &str = "@playWatch";

/// R1882004: the `hook` of a death watcher's queue entry (Stinkyscape #150 Call to Chaos, entry 8).
pub const DEATH_WATCH: &str = "@deathWatch";

/// How many times `settle` may go round before the board is called stuck (§10.3). Each pass resolves
/// something, so the cap holds every trigger one legal action can set off, as R58 bounds it: #95 draws
/// up to LIBRARY_CAP cards, each casting up to CAST_ON_DRAW_CHAIN_CAP, each a play (R70) every permanent
/// on either side may answer (#33 Unstable Clone Machine). Derived from config (CLAUDE.md rule 9).
pub const SETTLE_PASS_CAP: usize =
    (LIBRARY_CAP * CAST_ON_DRAW_CHAIN_CAP * (UNIT_ZONES + BACKROW_ZONES)) as usize * PLAYER_IDS.len();

// 1. The registry.

/// A Trap or Field Trap by its face (B2.7) — never by `faces::card_type_of`, which answers "Unit" for an
/// animated one standing in a unit zone, where it still fires as a trap (R383, `traps::is_trap_type`).
fn is_trap_card(state: &GameState, card: &CardInstance) -> bool {
    is_trap_type(state, card)
}

/// §10.3: which list a card's zone registers (B5 E26).
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

/// §5.1 and R68: a spell whose text is "End of turn: add this back to your hand" answers its
/// `endOfTurn` hook from the graveyard on the turn it was played alone (#23, #24, #31). R155's flag
/// (§10.5 step 7) is the whole gate, as a turn log cannot tell such a Spell from a Unit with an
/// end-of-turn hook (#13) that died that turn; nothing else in a graveyard registers (R153).
fn flagged_for_return(card: &CardInstance) -> bool {
    card.return_to_hand_at_end_of_turn == Some(true)
}

/// R153: whether a holder's zone lets it register this hook at all: the field and the backrow every
/// hook, a hand none, a graveyard only the end-of-turn return above. A library, exile or resolving card
/// and one dormant under a Stack get no holder at all (§3.2, R13). Without this #58 Rush Token Farm
/// fired its `startOfTurn` from a hand and #64 Gifted Program made a play Radiant from one.
fn zone_registers_hook(_state: &GameState, holder: &TriggerHolder, hook: HookName) -> bool {
    zone_registers_hook_in(holder.zone, &holder.card, hook)
}

/// R1020, R2226000: a Trap's or Field Trap's text fused into a card standing in a unit zone as no trap
/// never fires from there. Queued as a unit's trigger it would answer every event its `when` admits
/// (R2271000) with nothing to arm or spend it (#2226: 饕餮 holding Wormhole recast a Spell forever).
fn is_fused_trap_text_in_unit_zone(
    state: &GameState,
    card: &CardInstance,
    zone: TriggerZone,
    trigger: &TriggerDef,
) -> bool {
    zone == TriggerZone::Field
        && crate::subsystems::fuse::is_trap_ingredient_trigger(
            state,
            crate::subsystems::unit_text::text_def_of(card).0,
            &trigger.id,
        )
        && !is_trap_card(state, card)
}

/// `zone_registers_hook` for a card in `zone`, before its holder is built.
fn zone_registers_hook_in(zone: TriggerZone, card: &CardInstance, hook: HookName) -> bool {
    if zone == TriggerZone::Hand || zone == TriggerZone::Library {
        return false;
    }
    if zone == TriggerZone::Graveyard {
        return hook == GRAVEYARD_HOOK && flagged_for_return(card);
    }
    true
}

fn holder_of(
    state: &GameState,
    card: &CardInstance,
    zone: TriggerZone,
    controller: PlayerId,
) -> TriggerHolder {
    let script = crate::scripts::script_of(state, card);
    // R383: an animated Field Trap in a unit zone is a trap too, fired by `traps.rs`, never queued.
    let is_trap = (zone == TriggerZone::Backrow || zone == TriggerZone::Field) && is_trap_card(state, card);
    // R671: a hand card's enchantments may grant it a hand trigger its text does not print.
    let triggers: Vec<TriggerDef> = if zone == TriggerZone::Hand {
        crate::book_swap::granted_hand_triggers(card, registered_triggers(&script, zone)).to_vec()
    } else {
        registered_triggers(&script, zone)
            .iter()
            .filter(|trigger| !is_fused_trap_text_in_unit_zone(state, card, zone, trigger))
            .cloned()
            .collect()
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
/// `next_id`); anything else sorts last.
fn creation_number(id: &str) -> i64 {
    match id.strip_prefix('c') {
        Some(digits) if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) => {
            digits.parse::<i64>().unwrap_or(i64::MAX)
        }
        _ => i64::MAX,
    }
}

/// B5 E26, R464: a library's holders, the cards whose running face declares `deckTriggers`, in creation
/// order. Never library order: that is hidden from both players (§9.1, R223).
fn library_holders(state: &GameState, player: PlayerId) -> Vec<&CardInstance> {
    let mut cards: Vec<&CardInstance> = state.players[player]
        .library
        .iter()
        .filter(|card| !crate::scripts::script_of(state, *card).deck_triggers.is_empty())
        .collect();
    // Stable (SURFACE §4.4.1).
    cards.sort_by(|a, b| {
        creation_number(&a.id)
            .cmp(&creation_number(&b.id))
            .then_with(|| a.id.cmp(&b.id))
    });
    cards
}

/// The cards `trigger_holders_of` makes holders of, with their zones, in its order.
fn holder_sites(state: &GameState, player: PlayerId) -> Vec<(&CardInstance, TriggerZone)> {
    let side = &state.players[player];
    let mut out: Vec<(&CardInstance, TriggerZone)> = Vec::new();
    for card in crate::zones::active_units_of(state, player) {
        out.push((card, TriggerZone::Field));
    }
    for slot in crate::zones::slots_of(player, Row::Backrow) {
        // R1081000: a Unit standing in the backrow is a holder above, among the active units, once.
        if let Some(card) = crate::zones::card_at(state, slot)
            && !crate::zones::is_standing_unit(state, card)
        {
            out.push((card, TriggerZone::Backrow));
        }
    }
    for card in &side.hand {
        out.push((card, TriggerZone::Hand));
    }
    for card in library_holders(state, player) {
        out.push((card, TriggerZone::Library));
    }
    for card in &side.graveyard {
        out.push((card, TriggerZone::Graveyard));
    }
    out
}

/// One side's holders in R68's within-a-side order (`holder_sites`).
pub fn trigger_holders_of(state: &GameState, player: PlayerId) -> Vec<TriggerHolder> {
    holder_sites(state, player)
        .into_iter()
        .map(|(card, zone)| holder_of(state, card, zone, player))
        .collect()
}

/// `cards_in_trigger_order(state)` filtered by `keep(card, zone)`, building only the holders it keeps.
fn holders_where(state: &GameState, keep: impl Fn(&CardInstance, TriggerZone) -> bool) -> Vec<TriggerHolder> {
    let mut out: Vec<TriggerHolder> = Vec::new();
    for player in active_first(state) {
        for (card, zone) in holder_sites(state, player) {
            if keep(card, zone) {
                out.push(holder_of(state, card, zone, player));
            }
        }
    }
    out
}

/// Whether `triggers_on_event(&holder_of(..), event_type)` has any, without building the holder.
fn registers_trigger_on(
    state: &GameState,
    card: &CardInstance,
    zone: TriggerZone,
    event_type: GameEventType,
) -> bool {
    let script = crate::scripts::script_of(state, card);
    let printed = registered_triggers(&script, zone);
    let wakes = |def: &TriggerDef| {
        def.on.contains(&event_type) && !is_fused_trap_text_in_unit_zone(state, card, zone, def)
    };
    if zone == TriggerZone::Hand {
        crate::book_swap::granted_hand_triggers(card, printed)
            .iter()
            .any(wakes)
    } else {
        printed.iter().any(wakes)
    }
}

/// Every holder with a trigger on `event_type`, traps included, in R68's order.
pub fn holders_answering(state: &GameState, event_type: GameEventType) -> Vec<TriggerHolder> {
    holders_where(state, |card, zone| {
        registers_trigger_on(state, card, zone, event_type)
    })
}

/// The field's holders (unit lanes then backrow lanes, both sides, in R68's order) of the cards `keep`
/// accepts: `cards_in_trigger_order` filtered so, without copying any other card.
pub fn field_holders_where(state: &GameState, keep: impl Fn(&CardInstance) -> bool) -> Vec<TriggerHolder> {
    let mut out: Vec<TriggerHolder> = Vec::new();
    for player in active_first(state) {
        for card in crate::zones::active_units_of(state, player) {
            if keep(card) {
                out.push(holder_of(state, card, TriggerZone::Field, player));
            }
        }
        for slot in crate::zones::slots_of(player, Row::Backrow) {
            if let Some(card) = crate::zones::card_at(state, slot)
                && !crate::zones::is_standing_unit(state, card)
                && keep(card)
            {
                out.push(holder_of(state, card, TriggerZone::Backrow, player));
            }
        }
    }
    out
}

/// Every card that could answer, in R68's order, unfiltered (a card with no script keeps its place):
/// filter with `triggers_on_event` or `trigger_holders_with_hook`.
pub fn cards_in_trigger_order(state: &GameState) -> Vec<TriggerHolder> {
    active_first(state)
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
        // R1839000: a Dormant card registers nothing until it awakens; its "When this awakens" answers
        // the `awakened` it is awake for.
        Zone::Field { .. } if crate::slumber::is_slumbering(card) => None,
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
            let on_top = crate::zones::card_at(state, ZoneRef { player, row, lane })
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
    holders_answering(state, event_type)
}

/// The registry keyed by hook: every card whose zone lets it answer that hook (R153; a graveyard's only
/// answer is the `returnToHandAtEndOfTurn` return, R68), in R68's order. `only` narrows it to one
/// controller: start-of-turn and end-of-turn hooks fire on their own controller's turn alone (§6.2).
pub fn trigger_holders_with_hook(
    state: &GameState,
    hook: HookName,
    only: Option<PlayerId>,
) -> Vec<TriggerHolder> {
    let carries = |card: &CardInstance, zone: TriggerZone| {
        zone_registers_hook_in(zone, card, hook)
            && crate::scripts::script_of(state, card)
                .hook_named(hook.as_str())
                .is_some()
    };
    match only {
        None => holders_where(state, carries),
        Some(player) => holder_sites(state, player)
            .into_iter()
            .filter(|&(card, zone)| carries(card, zone))
            .map(|(card, zone)| holder_of(state, card, zone, player))
            .collect(),
    }
}

// 2. The queue.

/// The `step` a queued entry's resume carries, which nothing reads: an entry is re-entered by trigger
/// id or hook name, never by step.
const TRIGGER_STEP: &str = "trigger";

/// `state.next_seq` numbers a queue entry, as it numbers modifiers, delayed effects and parked work, so
/// R68's creation order is one counter and ids replay deterministically (`t…`, apart from `q…` choices).
/// R177: a hidden card's entry (a hand's or a library's, B5 E26, R464) borrows the counter unmoved,
/// told apart by the queue's length, as both seats read modifier ids (R169) and a number it took would
/// tell them a hand holds, say, #89 Corpse Eater, which answers even a token's death (R11).
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
        Some(owed) => owed
            .iter()
            .filter_map(|id| id.as_str().map(str::to_string))
            .collect(),
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

/// Append one of a card's triggers to the queue, behind everything already waiting (R68). R174, R212:
/// the entry carries the stays its event happened on, and the trigger runs with them whenever it pops,
/// so an earlier trigger that killed a card its event names, and a Reborn in the check between (R59),
/// leave it nothing to land on (R83). A card it reads off the board is judged from when its run began.
pub fn queue_trigger(
    sink: &mut EngineSink<'_>,
    holder: &TriggerHolder,
    def: &TriggerDef,
    event: &GameEvent,
) -> QueuedTrigger {
    let hidden = holder.zone == TriggerZone::Hand || holder.zone == TriggerZone::Library;
    let (id, seq) = next_entry_id(sink.state, hidden, "h");
    // A `RunMarks { eventStay }`, written as its JSON.
    let stay = crate::stays::event_stay_of(sink.state, event);
    let marks =
        json!({ "eventStay": serde_json::to_value(&stay).expect("an event stay is plain JSON (§10.1)") });
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(
        "event".to_string(),
        serde_json::to_value(event).expect("an event is plain JSON (§10.1)"),
    );
    data.insert(
        "zone".to_string(),
        Value::String(holder.zone.as_str().to_string()),
    );
    data.insert("controller".to_string(), player_value(holder.controller));
    // R1864003: `traded` names no card (R1838001), so a trigger on it is handed the card the Trade put
    // into the deck, which only the queue holds and no view shows.
    if matches!(event, GameEvent::Traded { .. })
        && let Some(traded) = crate::trade::traded_card_after(sink.state)
    {
        data.insert(crate::trade::TRADED_ID_KEY.to_string(), Value::String(traded));
    }
    data.insert(crate::work::RUN_MARKS_KEY.to_string(), marks);
    let entry = QueuedTrigger {
        id,
        seq,
        instance_id: holder.card.id.clone(),
        hook: def.id.clone(),
        resume: Resume {
            // R1850000: a copier's trigger is the copied text's, re-entered under that definition.
            def_id: crate::subsystems::unit_text::text_def_of(&holder.card)
                .0
                .to_string(),
            hook: def.id.clone(),
            step: TRIGGER_STEP.to_string(),
            radiant: crate::subsystems::unit_text::text_def_of(&holder.card).1,
            instance_id: Some(holder.card.id.clone()),
            data,
        },
    };
    sink.state.trigger_queue.push(entry.clone());
    entry
}

/// Append one of a card's hooks to the queue. §6.2 and R62 resolve start-of-turn and end-of-turn hooks
/// "in queue order", so queueing them gives them a state check between each two (R59) and a pause that
/// keeps the rest in state. R177: a face-down trap's hook (Classic #65 Ace in the Hole) is a hidden
/// card's entry too, as a number it took would tell the other seat the card carries that hook.
pub fn queue_hook(sink: &mut EngineSink<'_>, holder: &TriggerHolder, hook: HookName) -> QueuedTrigger {
    let hidden =
        holder.zone == TriggerZone::Backrow && crate::preview::is_face_down(sink.state, &holder.card);
    let (id, seq) = next_entry_id(sink.state, hidden, "h");
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(
        "zone".to_string(),
        Value::String(holder.zone.as_str().to_string()),
    );
    data.insert("controller".to_string(), player_value(holder.controller));
    let entry = QueuedTrigger {
        id,
        seq,
        instance_id: holder.card.id.clone(),
        hook: hook.as_str().to_string(),
        resume: Resume {
            def_id: crate::subsystems::unit_text::text_def_of(&holder.card)
                .0
                .to_string(),
            hook: hook.as_str().to_string(),
            step: TRIGGER_STEP.to_string(),
            radiant: crate::subsystems::unit_text::text_def_of(&holder.card).1,
            instance_id: Some(holder.card.id.clone()),
            data,
        },
    };
    sink.state.trigger_queue.push(entry.clone());
    entry
}

/// Queue one hook across the board in R68's order (§6.2, R62); the caller's `settle` drains it.
///
/// R820, R821 (Meditative #9 Joint Filing): a player's own start-of-turn and end-of-turn hooks run
/// 1 + N times under a turn-hook multiplier, N read once as they are queued, each copy its own entry
/// re-read as it pops (R153). Only these two hooks: C #62's start of the opponent's turn, the delayed
/// effects and the end-of-turn trap window are never queued here as the player's own (R62).
pub fn queue_hooks_in_trigger_order(
    sink: &mut EngineSink<'_>,
    hook: HookName,
    only: Option<PlayerId>,
) -> Vec<QueuedTrigger> {
    let copies = match (hook, only) {
        (HookName::StartOfTurn | HookName::EndOfTurn, Some(player)) => {
            1 + crate::multipliers::extra_runs(sink.state, player, crate::multipliers::Multiplied::TurnHooks)
                .max(0) as usize
        }
        _ => 1,
    };
    let holders = trigger_holders_with_hook(sink.state, hook, only);
    let mut queued: Vec<QueuedTrigger> = Vec::new();
    for holder in &holders {
        for _ in 0..copies {
            queued.push(queue_hook(sink, holder, hook));
        }
    }
    queued
}

/// The `hook` of a queue entry that queues the next round of a player's end-of-turn hooks
/// (`queue_turn_hook_rounds`, R824). No card's trigger id starts with `@`.
pub const TURN_HOOKS_ROUND: &str = "@turnHooksRound";

/// Where a round entry keeps how many rounds are still to be queued after the one it queues.
const ROUNDS_LEFT_KEY: &str = "roundsLeft";

/// R824 (Meditative #12 Fear Mongerer): "trigger your End of turn effects" `rounds` times, now. Each
/// round queues `player`'s end-of-turn hooks as §2.2's step would (same holders, R68's order, R821's
/// multiplier) and nothing else of the end of the turn (R62). Behind each round, one entry in R68's
/// queue queues the next as it pops, so a round reads the board the round before left; it never waits
/// for the queue to empty, or a forced attack's playout (`effects::combat`) would wait for ever.
pub fn queue_turn_hook_rounds(sink: &mut EngineSink<'_>, player: PlayerId, rounds: i32) {
    if rounds <= 0 {
        return;
    }
    queue_hooks_in_trigger_order(sink, HookName::EndOfTurn, Some(player));
    if rounds == 1 {
        return;
    }
    let (id, seq) = next_entry_id(sink.state, false, "t");
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert("controller".to_string(), player_value(player));
    data.insert(ROUNDS_LEFT_KEY.to_string(), Value::from(rounds - 1));
    sink.state.trigger_queue.push(QueuedTrigger {
        id,
        seq,
        instance_id: String::new(),
        hook: TURN_HOOKS_ROUND.to_string(),
        resume: Resume {
            def_id: String::new(),
            hook: TURN_HOOKS_ROUND.to_string(),
            step: TRIGGER_STEP.to_string(),
            radiant: false,
            instance_id: None,
            data,
        },
    });
}

/// A round entry popping (R824): the next round of its player's end-of-turn hooks, queued now.
fn run_turn_hooks_round(sink: &mut EngineSink<'_>, entry: &QueuedTrigger) {
    let queued_for = entry.resume.data.get("controller").and_then(Value::as_str);
    let Some(player) = PLAYER_IDS
        .into_iter()
        .find(|player| Some(player.as_str()) == queued_for)
    else {
        return;
    };
    let rounds = entry
        .resume
        .data
        .get(ROUNDS_LEFT_KEY)
        .and_then(Value::as_i64)
        .map_or(0, |rounds| rounds.clamp(0, i64::from(i32::MAX)) as i32);
    queue_turn_hook_rounds(sink, player, rounds);
}

/// Queue one hook across the board and resolve it, in R68's order (§6.2, R62, R59).
pub fn run_hooks_in_trigger_order(sink: &mut SettleSink<'_>, hook: HookName, only: Option<PlayerId>) {
    queue_hooks_in_trigger_order(sink, hook, only);
    settle(sink, SettleOptions::default());
}

/// Park an event the traps have not finished answering. `owed` names the traps that still have to
/// see it, so a resume neither re-fires one that already fired nor wakes a Field Trap twice (§5.1).
/// It goes in front of every queued card trigger, because a trap is a response (§10.3).
fn owed_to_traps(
    sink: &mut EngineSink<'_>,
    event: &GameEvent,
    run: &ImmediateDispatch,
) -> Option<QueuedTrigger> {
    if run.owed.is_empty() {
        return None;
    }
    let mut controllers: serde_json::Map<String, Value> = serde_json::Map::new();
    for id in &run.owed {
        if let Some(player) = run.controllers.get(id) {
            controllers.insert(id.clone(), player_value(*player));
        }
    }
    // R177: whether this entry exists hangs on which face-down traps are still owed the event, read by
    // their controller alone (R33), so it takes no number from the counter the modifiers' ids come from.
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
/// trap's owner stops that dispatch (BUILD M3-T2), so the traps it never reached are parked in state,
/// in its order; one that met the event and declined has had its look (R99). R62's end-of-turn window
/// is `traps::run_trap_window`'s, so `turnEnded` is passed over here. `at` is the event's place in
/// this action's list, when it is on it.
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

/// Finish a trap dispatch a prompt interrupted, parking again if another trap prompts. Each trap meets
/// the event as the board now stands (`traps::standing_event`, R174): a played card an earlier trap's
/// answer killed is out of play for the next (#85 fuses nothing out of a graveyard, R61).
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
/// The action's list is in emission order, so all of it after the event came after it, whichever loop
/// takes it (a cast's step-4 window, R70; #96's AI turn, whose nested `reduce`s dispatch it before it
/// is copied here, `mark_dispatched`, R44, R168), as `combat::queue_declaration_triggers` reads too.
/// An event not on the list (`at` is `None`) is an earlier action's, owed across a pause (§9.3), so all
/// of this action's came after it. An owed item is this action's own when it collected it
/// (`Frontier.positions`) or a nested `reduce` dispatched it before it was copied.
fn events_after_dispatched(sink: &EngineSink<'_>, at: Option<usize>) -> Vec<GameEvent> {
    let list: &[GameEvent] = &sink.events[..];
    let frontier = sink.frontier.get();
    let listed = |item: &DispatchItem| -> bool {
        if frontier
            .positions
            .get(&item.seq)
            .is_some_and(|&position| position < list.len())
        {
            return true;
        }
        frontier.elsewhere.iter().any(|&position| {
            list.get(position)
                .is_some_and(|listed| listed.as_rules_read() == item.event)
        })
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
/// (§10.3 steps C and D). Queueing only reads the board, so it never pauses halfway: the frontier is
/// safe to flush into state the instant a prompt opens elsewhere.
///
/// R212: the event is offered to the board as it stood when it happened, read off the events owed
/// behind it (a state check, Death hooks and Reborn may run first, §4.5). A card that has moved zones
/// since is on a stay that did not see it (a Reborn body, R174; a card drawn since, §8 #89), and a card
/// answers for whoever controlled it then (#32 Prem Panther, even when its victim's Death steals it, #86).
///
/// The event's place in the list is found by value here; the loop knows it (`dispatch_new_events`).
pub fn dispatch_event(sink: &mut EngineSink<'_>, event: &GameEvent) -> Vec<QueuedTrigger> {
    let at = sink
        .events
        .iter()
        .position(|listed| listed == event || listed.as_rules_read() == *event);
    dispatch_event_at(sink, &event.as_rules_read(), at)
}

fn dispatch_event_at(sink: &mut EngineSink<'_>, event: &GameEvent, at: Option<usize>) -> Vec<QueuedTrigger> {
    // R63: a hit of 0 is a report, not a damage instance, and nothing — no trap, no trigger, no quest —
    // answers it. R1361: that is what a `damageAbsorbed` is to the rules (`GameEvent::as_rules_read`),
    // the report of a hit Armor took whole, an absorbed fatigue draw's included (R240, R1362).
    if let GameEvent::Damage { amount, .. } = event
        && *amount <= 0
    {
        return Vec::new();
    }
    // R2179002: a cast on draw is a cast and no play, so nothing that answers or counts plays answers
    // its `cardPlayed`: no quest, no trap and no trigger. Its other events are answered as ever.
    if matches!(
        event,
        GameEvent::CardPlayed {
            on_draw: Some(true),
            ..
        }
    ) {
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
    // R2396000: the killer that answers this kill even once gone, if it does not answer it below.
    let gone_killer = take_kill_snapshot(sink.state, event);
    let mut killer_answered = false;
    for holder in holders_answering(sink.state, wakes) {
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
        // R862: a card the event itself sent toward a graveyard is on the stay that move began, the
        // report of where it landed being that same move (`stays::stands_where_it_landed`).
        if moves.moved.contains(&holder.card.id)
            && !crate::stays::stands_where_it_landed(
                event,
                &events_after_dispatched(&*sink, at),
                &holder.card.id,
                Some(&*sink.state),
            )
        {
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
        // R2396000: the killer still on the field answers its kill itself, once.
        if gone_killer
            .as_ref()
            .is_some_and(|killer| killer.id == holder.card.id)
            && defs.iter().any(|def| def.even_if_gone)
        {
            killer_answered = true;
        }
        for def in &defs {
            queued.push(queue_trigger(sink, &holder, def, event));
        }
    }
    // R2396000: a killer gone from the field answers its kill after the board's holders, once.
    if let Some(killer) = gone_killer.filter(|_| !killer_answered) {
        queued.extend(offer_to_gone_killer(sink, &killer, event));
    }
    // MD-B22, R946: Showdown's watchers answer the event before anything notes its removals.
    queued.extend(offer_to_lane_watches(sink, event));
    // R1863000: and Great Replacement Theory's, a resolved play's first of each card they name.
    queued.extend(offer_to_play_watches(sink, event));
    // R1882004: and Call to Chaos (Stinky Edition)'s, a death of a card printing their tag.
    queued.extend(offer_to_death_watches(sink, event));
    // R212: the removal this event reports, if any, has been answered (`stays::note_reported`).
    crate::stays::note_reported(sink.state, event);
    queued
}

/// MD-B22, R946: offer one event to every live lane watcher (`ModifierKind::LaneWatch`). A watch
/// matches a permanent's `cardResolved` in one of its owner's zones in its lane, or a `summoned` there
/// that no play made (`exits_from` absent: a played card answers through its resolution, so once).
/// Each match queues one `LANE_WATCH` entry, its data holding `instanceId` and `watcher`, in R68's order.
fn offer_to_lane_watches(sink: &mut EngineSink<'_>, event: &GameEvent) -> Vec<QueuedTrigger> {
    // The owner, id and lane of the card the event puts into a lane, if it puts one there.
    let placed: Option<(PlayerId, String, i32)> = match event {
        GameEvent::CardResolved {
            permanent,
            player,
            instance_id,
            ..
        } if *permanent => {
            let Some(card) = find_instance(sink.state, instance_id) else {
                return Vec::new();
            };
            match card.zone {
                Zone::Field { player: at, lane, .. } if at == *player => {
                    Some((*player, instance_id.clone(), lane))
                }
                _ => None,
            }
        }
        GameEvent::Summoned {
            player,
            lane,
            exits_from: None,
            instance_id,
            ..
        } => Some((*player, instance_id.clone(), *lane)),
        _ => None,
    };
    let Some((player, instance_id, lane)) = placed else {
        return Vec::new();
    };
    let mut queued: Vec<QueuedTrigger> = Vec::new();
    let order: [PlayerId; 2] = if sink.state.active == PLAYER_IDS[0] {
        PLAYER_IDS
    } else {
        [PLAYER_IDS[1], PLAYER_IDS[0]]
    };
    for owner in order {
        if owner != player {
            continue;
        }
        let watches: Vec<(String, i32, Resume)> = sink.state.players[owner]
            .mods
            .iter()
            .filter_map(|modifier| match &modifier.kind {
                crate::state::ModifierKind::LaneWatch {
                    lane: watched,
                    resume,
                    ..
                } if modifier.expiry
                    == crate::state::ModifierExpiry::ThisTurn {
                        turn: sink.state.turn,
                    } =>
                {
                    Some((modifier.id.clone(), *watched, resume.clone()))
                }
                _ => None,
            })
            .collect();
        for (id, watched, resume) in watches {
            if watched != lane {
                continue;
            }
            let (entry_id, seq) = next_entry_id(sink.state, false, "w");
            let mut data = resume.data.clone();
            data.insert("instanceId".to_string(), Value::String(instance_id.clone()));
            data.insert("watcher".to_string(), Value::String(owner.as_str().to_string()));
            let entry = QueuedTrigger {
                id: entry_id,
                seq,
                instance_id: id,
                hook: LANE_WATCH.to_string(),
                resume: Resume { data, ..resume },
            };
            sink.state.trigger_queue.push(entry.clone());
            queued.push(entry);
        }
    }
    queued
}

/// R1863000: offer one event to the play watchers (`ModifierKind::PlayWatch`) of the player whose play
/// it reports. A watch matches its owner's `cardResolved` of a definition it still names and strikes it
/// off, so each card answers its first resolved play once (a countered play never resolves). Each match
/// queues one `PLAY_WATCH` entry, its data holding `defId` and `watcher`, in the order watches were made.
fn offer_to_play_watches(sink: &mut EngineSink<'_>, event: &GameEvent) -> Vec<QueuedTrigger> {
    let GameEvent::CardResolved { player, def_id, .. } = event else {
        return Vec::new();
    };
    let mut answers: Vec<(String, Resume)> = Vec::new();
    for modifier in &mut sink.state.players[*player].mods {
        if let crate::state::ModifierKind::PlayWatch { def_ids, resume, .. } = &mut modifier.kind
            && let Some(at) = def_ids.iter().position(|watched| watched == def_id)
        {
            def_ids.remove(at);
            answers.push((modifier.id.clone(), resume.clone()));
        }
    }
    let mut queued: Vec<QueuedTrigger> = Vec::new();
    for (id, resume) in answers {
        let (entry_id, seq) = next_entry_id(sink.state, false, "w");
        let mut data = resume.data.clone();
        data.insert("defId".to_string(), Value::String(def_id.clone()));
        data.insert("watcher".to_string(), Value::String(player.as_str().to_string()));
        let entry = QueuedTrigger {
            id: entry_id,
            seq,
            instance_id: id,
            hook: PLAY_WATCH.to_string(),
            resume: Resume { data, ..resume },
        };
        sink.state.trigger_queue.push(entry.clone());
        queued.push(entry);
    }
    queued
}

/// R1882004: offer one event to every death watcher (`ModifierKind::DeathWatch`). A watch matches a
/// `destroyed` of a card on either side whose definition prints its tag, answered for its controller as
/// it died (R172): one `DEATH_WATCH` entry each, its data holding `watcher` and `instanceId`, in R68's
/// order (the active player's watchers first, each player's in the order they were made).
fn offer_to_death_watches(sink: &mut EngineSink<'_>, event: &GameEvent) -> Vec<QueuedTrigger> {
    let GameEvent::Destroyed {
        instance_id,
        def_id,
        controller,
        ..
    } = event
    else {
        return Vec::new();
    };
    let Some(tags) = crate::catalog::find_def(Some(&*sink.state), def_id).map(|def| def.tags.clone()) else {
        return Vec::new();
    };
    let order: [PlayerId; 2] = if sink.state.active == PLAYER_IDS[0] {
        [PLAYER_IDS[0], PLAYER_IDS[1]]
    } else {
        [PLAYER_IDS[1], PLAYER_IDS[0]]
    };
    let mut answers: Vec<(String, Resume)> = Vec::new();
    for holder in order {
        for modifier in &sink.state.players[holder].mods {
            if let crate::state::ModifierKind::DeathWatch { tag, resume, .. } = &modifier.kind
                && tags.contains(tag)
            {
                answers.push((modifier.id.clone(), resume.clone()));
            }
        }
    }
    let mut queued: Vec<QueuedTrigger> = Vec::new();
    for (id, resume) in answers {
        let (entry_id, seq) = next_entry_id(sink.state, false, "w");
        let mut data = resume.data.clone();
        data.insert("instanceId".to_string(), Value::String(instance_id.clone()));
        data.insert(
            "watcher".to_string(),
            Value::String(controller.as_str().to_string()),
        );
        let entry = QueuedTrigger {
            id: entry_id,
            seq,
            instance_id: id,
            hook: DEATH_WATCH.to_string(),
            resume: Resume { data, ..resume },
        };
        sink.state.trigger_queue.push(entry.clone());
        queued.push(entry);
    }
    queued
}

/// R2396000: whether this card answers "Whenever this destroys a Unit" even once it is gone: one of its
/// field triggers on a `destroyed` is `even_if_gone` (Meditative #95.1 CN Golem).
pub fn answers_kills_when_gone(state: &GameState, card: &CardInstance) -> bool {
    crate::scripts::script_of(state, card)
        .triggers
        .iter()
        .any(|def| def.even_if_gone && def.on.contains(&GameEventType::Destroyed))
}

/// R2396000: the killer `state_check::collect` kept for this `destroyed`, taken off the state as it is
/// dispatched; `None` for any other event, or a kill no such killer made.
fn take_kill_snapshot(state: &mut GameState, event: &GameEvent) -> Option<CardInstance> {
    let GameEvent::Destroyed {
        instance_id,
        killer_id,
        ..
    } = event
    else {
        return None;
    };
    let snapshots = state.kill_snapshots.as_mut()?;
    let killer = snapshots.shift_remove(instance_id);
    if snapshots.is_empty() {
        state.kill_snapshots = None;
    }
    killer.filter(|killer| killer_id.as_deref() == Some(killer.id.as_str()))
}

/// R2396000: "Whenever this destroys a Unit" from a killer no longer on the field (or a Reborn body that
/// is not it, R174): its `even_if_gone` triggers on this `destroyed` queue as the snapshot it killed
/// with, which the entry carries (`prompts::SELF_KEY`) as a dead card's Death hook reads its own (R89),
/// for the player who controlled it as it killed (R212).
fn offer_to_gone_killer(
    sink: &mut EngineSink<'_>,
    killer: &CardInstance,
    event: &GameEvent,
) -> Vec<QueuedTrigger> {
    let holder = holder_of(sink.state, killer, TriggerZone::Field, killer.controller);
    let snapshot = serde_json::to_value(killer).expect("an instance is plain JSON (§10.1)");
    let mut queued: Vec<QueuedTrigger> = Vec::new();
    for def in triggers_on_event(&holder, event.event_type()) {
        if !def.even_if_gone {
            continue;
        }
        let mut entry = queue_trigger(sink, &holder, &def, event);
        entry
            .resume
            .data
            .insert(crate::prompts::SELF_KEY.to_string(), snapshot.clone());
        if let Some(pushed) = sink.state.trigger_queue.last_mut() {
            pushed.resume.data = entry.resume.data.clone();
        }
        queued.push(entry);
    }
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
    if entry.hook == TURN_HOOKS_ROUND {
        run_turn_hooks_round(sink, entry);
        return;
    }
    // MD-B22, R946: a lane watcher's answer — the stored resume runs as its owner; R1863000: a play
    // watcher's the same, and R1882004: a death watcher's, as the dead card's controller.
    if entry.hook == LANE_WATCH || entry.hook == PLAY_WATCH || entry.hook == DEATH_WATCH {
        let watcher = entry.resume.data.get("watcher").and_then(Value::as_str);
        let owner = PLAYER_IDS
            .into_iter()
            .find(|player| Some(player.as_str()) == watcher)
            .unwrap_or(PLAYER_IDS[0]);
        crate::prompts::run_resume(
            sink,
            &entry.resume,
            crate::prompts::ResumeOptions {
                controller: Some(owner),
                ..Default::default()
            },
        );
        return;
    }

    // R2396000: a gone killer's entry runs as the snapshot it killed with, which the entry carries
    // (`offer_to_gone_killer`); every other entry finds its card again.
    let (card, holder) = if let Some(snapshot) = crate::prompts::self_snapshot_of(&entry.resume.data) {
        let holder = holder_of(sink.state, &snapshot, TriggerZone::Field, snapshot.controller);
        (snapshot, holder)
    } else {
        let Some(card) = find_instance(sink.state, &entry.instance_id).cloned() else {
            return;
        };
        let Some(holder) = trigger_holder_for(sink.state, &card) else {
            return;
        };
        (card, holder)
    };

    let Some(event) = event else {
        // A hook entry: §6.2's start-of-turn and end-of-turn triggers, queued in R68's order.
        let Some(hook) = TRIGGER_HOOKS
            .iter()
            .copied()
            .find(|name| name.as_str() == entry.hook)
        else {
            return;
        };
        if holder.script.hook_named(hook.as_str()).is_none() {
            return;
        }
        // R153, re-read as a trigger is: a card now in a hand or a graveyard no longer registers it.
        if !zone_registers_hook(sink.state, &holder, hook) {
            return;
        }
        // §6.2: a start- or end-of-turn hook is its CONTROLLER's, on its controller's own turn (the queue's
        // `only`), so a card that changed sides while its entry waited fizzles rather than fire for them.
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
    // B5 E1, R448: a response to an announce whose play a Counter has already canceled, or that step 4
    // has already moved, finds no card and fizzles (`play_steps::run_announce_window` pops these itself).
    if let GameEvent::CardAnnounced { instance_id, .. } = &event
        && !crate::announce::is_announce_live(sink.state, instance_id)
    {
        return;
    }
    // R212: an event trigger answers for whoever controlled its card when the event happened.
    let queued_for = entry.resume.data.get("controller").and_then(Value::as_str);
    let controller = PLAYER_IDS
        .into_iter()
        .find(|player| Some(player.as_str()) == queued_for)
        .unwrap_or(holder.controller);
    // R174, R212: the cards the event names are judged from the stays it happened on, which the entry
    // captured as it was queued; the run itself begins now (`make_context`).
    let event_stay = crate::work::run_marks_of(&entry.resume.data).and_then(|marks| marks.event_stay);
    // Resumable, so a prompt stops the list there; `work::script_step_for` re-enters the parked tail by
    // trigger id from the captured event (R113), the answer going to the card's `resume` table (§6.3).
    // §5.2: the tail is rebuilt from the face the card wears NOW, not the queued one, which an earlier
    // trigger in the same queue can have made Radiant.
    let (text_def_id, text_radiant) = crate::subsystems::unit_text::text_def_of(&card);
    let plan = crate::work::WorkPlan::new(
        Resume {
            hook: def.id.clone(),
            def_id: text_def_id.to_string(),
            radiant: text_radiant,
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
    // R99, R2271000: a trigger's printed condition holds wherever it runs. The traps' dispatch reads a
    // trap's (`traps::fire_trap`; a trap on the field is never queued, R383), so this reads every other
    // trigger's, once, as it resolves. Declining does nothing, as a declining trap is not spent (R61).
    if def.when.as_ref().is_some_and(|when| !when(&mut ctx, &event)) {
        return;
    }
    let effects = (def.run)(&mut ctx, &event);
    crate::prompts::apply_resumable(&mut ctx, &plan, effects, None);
}

/// The trigger a queue entry names, on the card as it stands now. R77 keeps the instance a Fuse lands
/// on and R102 its text, but a fused definition namespaces each ingredient's trigger ids
/// (`<ingredientDefId>:<id>`), so an entry queued before the Fuse is found under its old definition's
/// namespace (Fed Fauci fused after a Cry hit it still owes its Plague Counter, R212).
fn queued_trigger_def(holder: &TriggerHolder, entry: &QueuedTrigger) -> Option<TriggerDef> {
    if let Some(exact) = holder
        .triggers
        .iter()
        .find(|candidate| candidate.id == entry.hook)
    {
        return Some(exact.clone());
    }
    let namespaced = format!("{}:{}", entry.resume.def_id, entry.hook);
    holder
        .triggers
        .iter()
        .find(|candidate| candidate.id == namespaced)
        .cloned()
}

/// §10.3 steps B to D without step G: the events so far reach the traps, which fire at once and to
/// completion, and the other triggers they wake are queued, but nothing queued pops and no owed work
/// drains, bar a trap's owed remainder (`OWED_TO_TRAPS`). A stage of several whole effects needs this
/// between two (R62's delayed effects, each whole, R59; the triggers wait for its loop, R68), as does a
/// cast's §10.5 step 4 inside its effect (R70). Stops at a prompt, the rest owed in state.
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

/// The sink the loop runs on. What is *owed* is the frontier in state: an action that returns paused
/// has handed every event it emitted to `state.dispatch`, so the answer action finishes the dispatch
/// (§9.3, §10.1, R122). The copy position is `Frontier.collected` (`dispatched`, `set_dispatched`).
pub type SettleSink<'a> = EngineSink<'a>;

/// Which of the action's events the frontier has taken, by position in the action's event list: what
/// `collect_events` takes (R212's `events_after_dispatched` reads the list itself) and how many
/// cast-on-draw cards the action has cast (R2067000). No game state: it dies with the action's sink.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Frontier {
    /// Every event of the action's list below this position has been taken into `state.dispatch` or is
    /// delivered some other way. Any sink over the list may take it (a cast's §10.5 step-4 window runs
    /// inside an effect, on its context, `effects::draw`, #95), and an event is taken once.
    pub collected: usize,
    /// Positions of events the frontier must not collect, as they are delivered some other way (§10.3:
    /// an event is offered once): #96 My Pawn's AI turn, whose every `reduce` settles its own events
    /// before the playout copies them here for the client (R168), and a player's `attackDeclared`,
    /// which §4.2 step 4's window delivers itself (`combat.rs`'s `withhold_from_frontier`).
    pub elsewhere: IndexSet<usize>,
    /// The position in the action's list of each event this action took into `state.dispatch`, by its
    /// dispatch item's `seq`.
    pub positions: IndexMap<u32, usize>,
    /// R2067000: the cast-on-draw cards this action has cast so far, over every chain of it
    /// (`draw::complete_draw`), held to `CAST_ON_DRAW_ACTION_CAP`.
    pub cast_on_draw_casts: i32,
}

/// Where a sink keeps its `Frontier`: its own for an action's first sink (`EngineSink::new`), its
/// parent's for every reborrow (`EngineSink::reborrow`, each context's), so every loop sees one.
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
            FrontierSlot::Shared(frontier) => frontier,
        }
    }

    pub fn get_mut(&mut self) -> &mut Frontier {
        match self {
            FrontierSlot::Own(frontier) => frontier,
            FrontierSlot::Shared(frontier) => frontier,
        }
    }
}

/// How far into the action's list the frontier has taken events.
pub fn dispatched(sink: &EngineSink<'_>) -> usize {
    sink.frontier.get().collected
}

/// The frontier counts the events below `at` as taken (`combat.rs`'s `withhold_from_frontier`, which
/// moves it past the declaration the window delivers itself).
pub fn set_dispatched(sink: &mut EngineSink<'_>, at: usize) {
    sink.frontier.get_mut().collected = at;
}

/// R1361: the report at `at` in the action's list (`damageAbsorbed`) is answered by nothing, so the
/// frontier never takes it and it takes no number from `state.next_seq` (R68's one counter), which
/// keeps a recorded game's replay at its final hash (R768).
pub fn withhold_report(sink: &mut EngineSink<'_>, at: usize) {
    sink.frontier.get_mut().elsewhere.insert(at);
}

/// R1362: the one report that keeps a place in R68's order, an absorbed fatigue draw's (the number
/// R240's `damage` of 0 took), rejoins the frontier as that hit of 0 (`GameEvent::as_rules_read`),
/// answered by nothing. `at` is its place in the action's list.
pub fn order_report(sink: &mut EngineSink<'_>, at: usize) {
    sink.frontier.get_mut().elsewhere.shift_remove(&at);
}

/// Mark events a nested `reduce` has already dispatched (see `Frontier.elsewhere`): the list's tail,
/// where both callers have just pushed them, or else the latest unmarked entry equal to each.
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
        let found = (0..len)
            .rev()
            .find(|&at| sink.events[at] == *event && !sink.frontier.get().elsewhere.contains(&at));
        if let Some(at) = found {
            sink.frontier.get_mut().elsewhere.insert(at);
        }
    }
}

/// §10.1: an emitted event joins the frontier in emission order, with an id and `seq` from
/// `state.next_seq` (R68's one counter), so a replay builds the frontier the live game had.
fn collect_events(sink: &mut SettleSink<'_>) {
    // R437: a mark whose effect has stopped waiting goes, and says so, before the frontier moves; R750:
    // a delayed destroy of a scope marks the Units it names now. R216: once the game is over neither
    // speaks, since `gameOver` is the action's last event (the view reads only live marks, `marks_on`).
    if sink.state.result.is_none() {
        crate::effects::delay::refresh_scope_marks(sink);
        crate::marks::sweep_marks(sink);
    }
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
        // R1360: the frontier holds the event as the rules read it, without the client's `absorbed`.
        let event = sink.events[at].as_rules_read();
        sink.state.dispatch.push(DispatchItem {
            id: format!("e{seq}"),
            seq,
            event,
        });
    }
}

/// Whether an event's own trap dispatch is unfinished, so the frontier may not advance (§10.3).
fn traps_still_owed(state: &GameState) -> bool {
    state
        .trigger_queue
        .iter()
        .any(|entry| entry.hook == OWED_TO_TRAPS)
}

/// Turn the frontier into trap firings and queue entries (§10.3 steps B, C, D), oldest event first,
/// taking each event off `state.dispatch` before offering it so no event is ever offered twice. It
/// stops while a prompt is open, the game is over, or an `OWED_TO_TRAPS` entry says the previous
/// event's traps have not all seen it: the traps see events in emission order.
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
/// until the queue is empty and no prompt is open. Call it after any action or answer and wherever
/// §2.2 opens a window. Safe to call again on a sink, or on a fresh sink over a state left
/// mid-dispatch: the frontier copies no event twice and `state.dispatch` holds the rest owed.
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
    /// §10.5 step 4's loop, so that a trap answering the play (Sheepish) fires before the Cry (R17).
    /// Nothing has resolved yet, so §4.5's check waits for a trap, owed work or a queued trigger: a card
    /// arriving at 0 or less health (a 6/1 Bigot under #46 Suppressive Aura) still resolves its Cry and
    /// the check after step 6 collects it. R118 loses the Cry only where a trap took the card.
    pub hold_check: Option<bool>,
}
