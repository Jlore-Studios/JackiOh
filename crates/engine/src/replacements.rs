//! Replacement windows: effects that change an event before it happens (docs/classic-sets.md B5 E5,
//! with E8's heal conversion and E9's redirects; SPEC §3.2, §4.2, §4.4, §4.5; R460–R463).
//!
//! Before patch v0.2.0 the engine had one replacement, My Pawn's attack cancel (R44), and it lives in
//! a trap WINDOW: the declaration waits while the traps answer it, and a trap there may ask a question.
//! The five v0.2.0 moments are not windows. Each sits at a fixed point inside a rule that cannot stop
//! halfway for a prompt:
//!
//!   lethalHit    §4.4, after the hero's Armor, divisor and caps and before step 5: a hit that would
//!                bring its hero to 0 or less, "this hit alone, as R44 judges it" (Classic #52);
//!   healed       inside every heal — Lifesteal, "heal up to" and "heal to full" included, set health
//!                not (E7) — on the heal's stated amount (R462; Classic+ #22);
//!   wouldDie     §4.5 step 1, once the check has collected its units and before any card moves
//!                (Classic #14's Radiant face);
//!   toGraveyard  every zone move into a graveyard, asked by `zones::move_to_zone` once the card has left
//!                the zone it was in (Classic #28, #50, #60);
//!   targeted     §4.2 step 2 for an attack, between its target and its trap window, and — the play
//!                pipeline's half — §10.5 step 1 and every target prompt (Classic #33 Joro).
//!
//! So a replacement is DECIDED SYNCHRONOUSLY, out of data the card declares (`Script.replacements`):
//! the moment, where the card must stand, a pure `when`, and a declarative `instead` the engine
//! applies — never an effect list, which could ask. What a card does beyond the replacement itself
//! (Final Gambit's "heal 10 and draw 3", Shadowstep's copies) is its `then`: a step of the card's own
//! `resume` table, owed on `state.work` as the card's continuation (R113) and resolved after the
//! replaced event, as a trap answering that event would be — so it survives a pause and a replay like
//! any continuation, and the default work handler (`prompts.rs`) runs it.
//!
//! R460 orders several replacements of one event: they apply one at a time in R68's order (the active
//! player's side first; within a side units by lane, backrow by lane, then hand; the card the event
//! is about, when it stands in none of those, after its owner's hand), each card replaces a given
//! event at most once, and once one has changed the event every later one re-checks the changed event
//! and applies only if it still does — so a second Final Gambit finds no lethal hit and stays set.
//!
//! A Trap or Field Trap that replaces an event FIRES: `trapFired`, face-up, and a Trap is spent to its
//! owner's graveyard as it would be after any firing (§5.1, R33, R61). A Field Spell, a Unit or a card
//! in a hand replaces without firing. A face-down card's text is in no one's use but its own firing,
//! so a face-down Trap is never a static source, and a replacement into a graveyard, which
//! `move_to_zone` asks with no event list to fire into, is only ever a static one.
//!
//! Port of `packages/engine/src/replacements.ts` (part 3). The declarations a card makes
//! (`ReplacementDef`, `ReplacedEvent`, `ReplacementContext`, …) are part 1's, in `script.rs`, because a
//! `Script` names them; they are re-exported here under their TS module's path. TS's module-level
//! `let converting` is `EngineSink::converting` (SURFACE §3, §6.5). TS's
//! `registerGraveyardRedirect(graveyardRedirectFor)` is not ported (SURFACE §6.6): `zones::move_to_zone`
//! calls `graveyard_redirect_for` directly.

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{DAMAGE_REDIRECT_CAP, LIBRARY_CAP};
use crate::damage::{DamageArgs, DamageTarget};
use crate::script::{EffectContext, EngineSink, InsteadLasting, InsteadTo};
use crate::state::{
    CardInstance, GameState, ModifierExpiry, ModifierKind, Resume, find_instance, find_instance_mut,
};
use crate::wire::{CardType, GameEvent, PlayerId, RedirectWhat, Row, Zone, ZoneName, opponent_of};
use crate::zones::GraveyardRedirect;

// ---------------------------------------------------------------------------
// What a card declares
// ---------------------------------------------------------------------------

/// The five points an event can be replaced at (see the header); a heal's target by id; the event a
/// replacement is offered, as plain JSON; where the card must stand; what a `when` reads; one
/// replacement a card makes; and THE declaration of "a friendly unit is targeted". All part 1's
/// (`script.rs`), under the path TS exported them from.
pub use crate::script::{
    DyingUnit, HealedRef, ReplacedEvent, ReplacementBy, ReplacementContext, ReplacementDef,
    ReplacementMoment, ReplacementWhen, ReplacementWhere, TargetedReplacement, TargetedWhat,
};

/// One unit a `wouldDie` replacement flickered, as its follow-up reads it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct FlickeredCard {
    pub instance_id: String,
    pub def_id: String,
    pub radiant: bool,
}

/// What a follow-up (`then`) is handed, in its context's data under `REPLACED_KEY`: the event as the
/// replacement met it, where a lethal hit went, and the units a `wouldDie` replacement flickered.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReplacementRecord {
    pub event: ReplacedEvent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redirected_to: Option<PlayerId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flickered: Option<Vec<FlickeredCard>>,
}

/// Where a follow-up's context data carries its `ReplacementRecord`.
pub const REPLACED_KEY: &str = "replaced";

/// The record a follow-up step was handed (Classic #14's copies read `flickered`), or `None`.
/// (TS `replacementOf(ctx: { data })`; it came through JSON, so it is read back defensively.)
pub fn replacement_of(ctx: &EffectContext<'_>) -> Option<ReplacementRecord> {
    let raw = ctx.data.get(REPLACED_KEY)?;
    if !raw.is_object() {
        return None;
    }
    raw.get("event")?;
    serde_json::from_value::<ReplacementRecord>(raw.clone()).ok()
}

// ---------------------------------------------------------------------------
// The walk (R460, R68)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CandidateZone {
    Field,
    Backrow,
    Hand,
    Elsewhere,
}

/// One card that could replace, as the walk met it. TS held the live object; this is the card as it
/// stood when the walk began, and every write goes to the live card by id as well.
#[derive(Clone, Debug)]
struct Candidate {
    card: CardInstance,
    controller: PlayerId,
    zone: CandidateZone,
}

/// The card as it stands now, found again by id (TS held the live object), or as it was handed over.
fn live(state: &GameState, card: &CardInstance) -> CardInstance {
    find_instance(state, &card.id)
        .cloned()
        .unwrap_or_else(|| card.clone())
}

fn sides_of(state: &GameState) -> [PlayerId; 2] {
    if state.active == PlayerId::P1 {
        [PlayerId::P1, PlayerId::P2]
    } else {
        [PlayerId::P2, PlayerId::P1]
    }
}

fn add_candidate(
    out: &mut Vec<Candidate>,
    seen: &mut IndexSet<String>,
    card: &CardInstance,
    controller: PlayerId,
    zone: CandidateZone,
) {
    if seen.contains(&card.id) {
        return;
    }
    seen.insert(card.id.clone());
    out.push(Candidate {
        card: card.clone(),
        controller,
        zone,
    });
}

/// Every card that could replace, in R68's order: the active player's side first; within a side the
/// units on top of their piles by lane, the backrow by lane, the hand in order — and `about`, the
/// card the event concerns, after its owner's hand when it stands in none of those (a Spell resolving,
/// a card already lifted off the field on its way to a graveyard). No graveyard card replaces.
fn candidates(state: &GameState, about: Option<&CardInstance>) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    let mut seen: IndexSet<String> = IndexSet::new();
    for player in sides_of(state) {
        for card in crate::zones::active_units_of(state, player) {
            add_candidate(&mut out, &mut seen, card, player, CandidateZone::Field);
        }
        for slot in crate::zones::slots_of(player, Row::Backrow) {
            if let Some(card) = crate::zones::card_at(state, slot) {
                add_candidate(&mut out, &mut seen, card, player, CandidateZone::Backrow);
            }
        }
        for card in &state.players[player].hand {
            add_candidate(&mut out, &mut seen, card, player, CandidateZone::Hand);
        }
        if let Some(about) = about
            && about.owner == player
        {
            add_candidate(
                &mut out,
                &mut seen,
                about,
                about.controller,
                CandidateZone::Elsewhere,
            );
        }
    }
    out
}

fn is_trap_card(state: &GameState, card: &CardInstance) -> bool {
    let card_type = crate::faces::card_type_of(state, card);
    card_type == CardType::Trap || card_type == CardType::FieldTrap
}

/// A backrow Trap whose identity nobody but its controller reads yet (R33).
fn face_down_trap(state: &GameState, cand: &Candidate) -> bool {
    cand.zone == CandidateZone::Backrow && is_trap_card(state, &cand.card) && cand.card.face_up != Some(true)
}

/// Whether the event is about this card, for a "self" replacement.
fn event_names(event: &ReplacedEvent, id: &str) -> bool {
    match event {
        ReplacedEvent::LethalHit { .. } => false,
        ReplacedEvent::Healed { target, .. } => {
            matches!(target, HealedRef::Unit { instance_id, .. } if instance_id == id)
        }
        ReplacedEvent::WouldDie { units } => units.iter().any(|unit| unit.instance_id == id),
        ReplacedEvent::ToGraveyard { instance_id, .. } | ReplacedEvent::Targeted { instance_id, .. } => {
            instance_id == id
        }
    }
}

/// Whether the card stands where its replacement says. On the field a Trap answers face-down and fires
/// (§5.1), a Field Trap face-down or up, anything else as it stands; a replacement into a graveyard is
/// static and never fires, so a Trap offers one only once it is face-up.
fn stands_where(
    state: &GameState,
    cand: &Candidate,
    where_: ReplacementWhere,
    event: &ReplacedEvent,
) -> bool {
    if where_ == ReplacementWhere::SelfCard {
        return event_names(event, &cand.card.id);
    }
    if where_ == ReplacementWhere::Hand {
        return cand.zone == CandidateZone::Hand;
    }
    if cand.zone == CandidateZone::Field {
        return true;
    }
    if cand.zone != CandidateZone::Backrow {
        return false;
    }
    if !is_trap_card(state, &cand.card) {
        return true;
    }
    if event.moment() == ReplacementMoment::ToGraveyard {
        return cand.card.face_up == Some(true);
    }
    crate::faces::card_type_of(state, &cand.card) == CardType::FieldTrap || cand.card.face_up != Some(true)
}

/// R651: whether a "targeted" replacement answers a targeting from this source — a `by: "spell"`
/// replacement (Classic #33 Joro) answers only a Spell's targeting, and an attack carries no source.
fn targeted_source_matches(def: &TargetedReplacement, event: &ReplacedEvent) -> bool {
    if def.by.is_none() {
        return true;
    }
    matches!(
        event,
        ReplacedEvent::Targeted {
            source: Some(CardType::Spell),
            ..
        }
    )
}

/// The first of the card's replacements for this moment that stands where it must and whose `when` holds.
fn answering(
    state: &GameState,
    cand: &Candidate,
    moment: ReplacementMoment,
    event: &ReplacedEvent,
) -> Option<ReplacementDef> {
    let defs: Vec<ReplacementDef> = crate::scripts::script_of(state, &cand.card)
        .replacements
        .into_iter()
        .filter(|def| def.on == moment)
        .collect();
    defs.into_iter().find(|def| {
        if !stands_where(state, cand, def.where_.unwrap_or(ReplacementWhere::Field), event) {
            return false;
        }
        if moment == ReplacementMoment::Targeted && !targeted_source_matches(def, event) {
            return false;
        }
        let Some(when) = &def.when else {
            return true;
        };
        when(ReplacementContext {
            state,
            self_: &cand.card,
            controller: cand.controller,
            radiant: cand.card.radiant,
            event,
        })
    })
}

/// A Trap or Field Trap in the backrow fires as it replaces (§5.1, R33): `trapFired`, and face-up from
/// this moment. Returns whether it fired, so `finish` knows to spend it.
fn fire(sink: &mut EngineSink<'_>, cand: &mut Candidate) -> bool {
    if cand.zone != CandidateZone::Backrow || !is_trap_card(sink.state, &cand.card) {
        return false;
    }
    let at = crate::zones::slot_of(sink.state, &cand.card);
    sink.events.push(GameEvent::TrapFired {
        instance_id: cand.card.id.clone(),
        def_id: cand.card.def_id.clone(),
        controller: cand.controller,
        row: at.map_or(Row::Backrow, |slot| slot.row),
        lane: at.map_or(0, |slot| slot.lane),
    });
    cand.card.face_up = Some(true);
    if let Some(card) = find_instance_mut(sink.state, &cand.card.id) {
        card.face_up = Some(true);
    }
    true
}

/// After the replacement: a fired Trap is spent to its owner's graveyard (a Field Trap stays face-up,
/// §5.1), and the card's follow-up is owed (R113) with what it needs to know.
fn finish(
    sink: &mut EngineSink<'_>,
    cand: &Candidate,
    def: &ReplacementDef,
    record: &ReplacementRecord,
    fired: bool,
) {
    let mut card = live(sink.state, &cand.card);
    if fired
        && crate::faces::card_type_of(sink.state, &card) == CardType::Trap
        && card.zone.z() == ZoneName::Field
    {
        let result = crate::zones::move_to_zone(
            sink.state,
            &mut card,
            crate::zones::OffFieldZone::Graveyard,
            Default::default(),
        );
        card = live(sink.state, &card);
        crate::zones::report_graveyard_landing(sink, &card, result);
    }
    let Some(then) = &def.then else {
        return;
    };
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(
        REPLACED_KEY.to_string(),
        serde_json::to_value(record).expect("a replacement record is plain JSON (§10.1)"),
    );
    let resume = Resume {
        def_id: card.def_id.clone(),
        hook: "resume".to_string(),
        step: then.clone(),
        radiant: card.radiant,
        instance_id: Some(card.id.clone()),
        data,
    };
    // R462: owed as the card's own continuation the moment it fires (R113's cursor), so it resolves once
    // the effect the replaced event happened in has finished or paused, before the resolution loop pops
    // any queued trigger — where a trap answering that event would resolve.
    crate::work::push_work(sink, resume, Some(cand.controller));
}

// ---------------------------------------------------------------------------
// §4.4: would take lethal damage (E5, E9)
// ---------------------------------------------------------------------------

/// §4.4 between the hero's caps and step 5: a hit of `amount` would bring `player`'s hero to 0 or
/// less. Returns the hero the hit now goes to — the damage pipeline deals it there as a new instance
/// from the same source, through that hero's Armor, divisor and caps — or `None` when nothing replaced
/// it. Only the hero's own controller's cards answer: "you would take lethal damage".
/// (TS `lethalHitWindow(sink, { player, amount, sourceId })`; the hit's three fields are arguments.)
pub fn lethal_hit_window(
    sink: &mut EngineSink<'_>,
    player: PlayerId,
    amount: i32,
    source_id: Option<String>,
) -> Option<PlayerId> {
    let event = ReplacedEvent::LethalHit {
        player,
        amount,
        source_id,
    };
    for mut cand in candidates(sink.state, None) {
        if cand.controller != player {
            continue;
        }
        let Some(def) = answering(sink.state, &cand, ReplacementMoment::LethalHit, &event) else {
            continue;
        };
        let to = opponent_of(cand.controller);
        let fired = fire(sink, &mut cand);
        sink.events.push(GameEvent::Redirected {
            what: RedirectWhat::Damage,
            from_id: format!("hero-{player}"),
            to_id: format!("hero-{to}"),
            by_instance_id: Some(cand.card.id.clone()),
        });
        let record = ReplacementRecord {
            event: event.clone(),
            redirected_to: Some(to),
            flickered: None,
        };
        finish(sink, &cand, &def, &record, fired);
        // R460: the hit is not on this hero any more, so nothing later in the walk finds it lethal here;
        // the new instance meets the other hero's replacements in a walk of its own.
        return Some(to);
    }
    None
}

// ---------------------------------------------------------------------------
// Heals: would be healed (E5) and heal into damage (E8)
// ---------------------------------------------------------------------------

// Heals being converted right now, so two Lifesteal converters cannot convert each other for ever:
// TS's module-level `let converting`, now `EngineSink::converting` (SURFACE §6.5).

fn healed_ref_of(target: &DamageTarget) -> HealedRef {
    match target {
        DamageTarget::Hero { player } => HealedRef::Hero { player: *player },
        DamageTarget::Unit { instance } => HealedRef::Unit {
            instance_id: instance.id.clone(),
            controller: instance.controller,
        },
    }
}

/// E8: whether the card converts heals by its text, as it stands now (face-up, when a Trap).
fn converts_by_text(state: &GameState, cand: &Candidate) -> bool {
    if crate::scripts::flags_of(state, &cand.card).heal_to_damage != Some(true) {
        return false;
    }
    if cand.zone != CandidateZone::Field && cand.zone != CandidateZone::Backrow {
        return false;
    }
    !face_down_trap(state, cand)
}

/// E8: the heal of `amount` becomes that much Pierce damage from the converting card.
fn convert(sink: &mut EngineSink<'_>, converter: Option<CardInstance>, target: &DamageTarget, amount: i32) {
    sink.converting += 1;
    crate::damage::deal_damage(
        sink,
        DamageArgs {
            source: converter,
            target: target.clone(),
            amount,
            flags: Some(crate::damage::DamageFlags {
                ignore_armor: Some(true),
                ..Default::default()
            }),
        },
    );
    sink.converting -= 1;
}

/// E5's "would be healed" and E8's conversion, asked by every heal before it lands (`damage.rs`):
/// true when the heal was replaced — it heals nothing, and its stated amount has been dealt instead.
/// A conversion already in force goes first (the active player's modifiers, then the other's), then
/// the R68 walk over the cards on the healed target's enemies' side: a card converting by its text,
/// or one replacing now — a Trap that fires, and whose `lasting` converts the rest of the turn too.
pub fn healing_replaced(sink: &mut EngineSink<'_>, target: &DamageTarget, amount: i32) -> bool {
    if amount <= 0 || sink.converting >= DAMAGE_REDIRECT_CAP {
        return false;
    }
    if let DamageTarget::Unit { instance } = target
        && !crate::zones::acts_on_field(sink.state, instance)
    {
        return false;
    }
    let healed = match target {
        DamageTarget::Hero { player } => *player,
        DamageTarget::Unit { instance } => instance.controller,
    };

    for player in sides_of(sink.state) {
        if player == healed {
            continue;
        }
        let state: &GameState = &*sink.state;
        let found = state.players[player]
            .mods
            .iter()
            .find(|each| {
                matches!(each.kind, ModifierKind::HealToDamage { .. })
                    && crate::mana::modifier_is_live(state, each)
            })
            .cloned();
        let Some(modifier) = found else {
            continue;
        };
        let ModifierKind::HealToDamage { converter_id } = &modifier.kind else {
            continue;
        };
        let converter = find_instance(sink.state, converter_id).cloned();
        convert(sink, converter, target, amount);
        return true;
    }

    let event = ReplacedEvent::Healed {
        target: healed_ref_of(target),
        amount,
    };
    let about = match target {
        DamageTarget::Unit { instance } => Some(instance.clone()),
        DamageTarget::Hero { .. } => None,
    };
    for mut cand in candidates(sink.state, about.as_ref()) {
        if cand.controller == healed {
            continue;
        }
        if converts_by_text(sink.state, &cand) {
            let converter = live(sink.state, &cand.card);
            convert(sink, Some(converter), target, amount);
            return true;
        }
        let Some(def) = answering(sink.state, &cand, ReplacementMoment::Healed, &event) else {
            continue;
        };
        let fired = fire(sink, &mut cand);
        if def.instead.lasting == Some(InsteadLasting::ThisTurn) {
            let turn = sink.state.turn;
            crate::modifiers::add_modifier(
                sink,
                cand.controller,
                ModifierExpiry::ThisTurn { turn },
                ModifierKind::HealToDamage {
                    converter_id: cand.card.id.clone(),
                },
            );
        }
        let converter = live(sink.state, &cand.card);
        convert(sink, Some(converter), target, amount);
        let record = ReplacementRecord {
            event: event.clone(),
            redirected_to: None,
            flickered: None,
        };
        finish(sink, &cand, &def, &record, fired);
        return true;
    }
    false
}

// ---------------------------------------------------------------------------
// §4.5 step 1: would die (E5), and the flicker it sends a unit through (E22)
// ---------------------------------------------------------------------------

fn in_unit_zone(card: &CardInstance) -> bool {
    matches!(card.zone, Zone::Field { row: Row::Units, .. })
}

/// §4.5 step 1, before any card moves: the units the check collected would die, and a `wouldDie`
/// replacement takes its controller's among them — one firing for all of them — and flickers them
/// instead (Classic #14's Radiant face). Returns the cards that still die. R462: the check's own
/// collection only; a Sacrifice is an effect's (§6.3) and is not offered.
pub fn would_die_window(sink: &mut EngineSink<'_>, dying: &[CardInstance]) -> Vec<CardInstance> {
    let mut left: Vec<CardInstance> = dying.to_vec();
    if !left.iter().any(in_unit_zone) {
        return left;
    }
    for mut cand in candidates(sink.state, None) {
        let theirs: Vec<CardInstance> = left
            .iter()
            .filter(|card| in_unit_zone(card) && card.controller == cand.controller)
            .cloned()
            .collect();
        if theirs.is_empty() {
            continue;
        }
        let event = ReplacedEvent::WouldDie {
            units: left
                .iter()
                .filter(|card| in_unit_zone(card))
                .map(|card| DyingUnit {
                    instance_id: card.id.clone(),
                    controller: card.controller,
                })
                .collect(),
        };
        let Some(def) = answering(sink.state, &cand, ReplacementMoment::WouldDie, &event) else {
            continue;
        };
        let fired = fire(sink, &mut cand);
        let mut flickered: Vec<FlickeredCard> = Vec::new();
        for unit in &theirs {
            let record = FlickeredCard {
                instance_id: unit.id.clone(),
                def_id: unit.def_id.clone(),
                radiant: unit.radiant,
            };
            // B5 E22, R444: the field workstream's flicker, the same one the `flicker` verb does.
            if crate::effects::flicker::flicker_card(sink, unit) {
                flickered.push(record);
            }
        }
        // R460: the event has changed — these units no longer die — and the rest of the walk re-checks it.
        left.retain(|card| !theirs.iter().any(|theirs| theirs.id == card.id));
        let record = ReplacementRecord {
            event,
            redirected_to: None,
            flickered: Some(flickered),
        };
        finish(sink, &cand, &def, &record, fired);
    }
    left
}

// ---------------------------------------------------------------------------
// Every move into a graveyard (E5)
// ---------------------------------------------------------------------------

/// `zones.GraveyardRedirect`: TS's `{ to: "exile" } | { to: "library"; position: "bottom" }`.
fn redirect_to(exile: bool) -> GraveyardRedirect {
    if exile {
        GraveyardRedirect::Exile
    } else {
        GraveyardRedirect::LibraryBottom
    }
}

/// `zones::move_to_zone`'s check (TS registered it with `registerGraveyardRedirect`; Rust calls it
/// directly): where a card that would go to its owner's graveyard goes instead, or `None`. Asked once
/// the card has left the zone it was in, so a Voidwalker dying takes its own aura with it (Classic
/// #50). A unit token never reaches a graveyard (R11), so nothing here changes its death (R462).
pub fn graveyard_redirect_for(state: &GameState, card: &CardInstance) -> Option<GraveyardRedirect> {
    if crate::zones::is_unit_token(state, card) {
        return None;
    }
    let event = ReplacedEvent::ToGraveyard {
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        owner: card.owner,
        from: card.zone.z().as_str().to_string(),
    };
    for cand in candidates(state, Some(card)) {
        let Some(def) = answering(state, &cand, ReplacementMoment::ToGraveyard, &event) else {
            continue;
        };
        // R80: a full library turns the card away, so that replacement cannot apply and the card goes on
        // toward its graveyard, where a later one may still meet it.
        if def.instead.to == Some(InsteadTo::BottomOfLibrary)
            && state.players[card.owner].library.len() as i32 >= LIBRARY_CAP
        {
            continue;
        }
        // R460: the first that applies sends the card elsewhere, and a card no longer on its way to a
        // graveyard is nothing any later one replaces.
        return Some(redirect_to(def.instead.to == Some(InsteadTo::Exile)));
    }
    None
}

// ---------------------------------------------------------------------------
// "A friendly unit is targeted" (E5, E9)
// ---------------------------------------------------------------------------

/// B5 E5, E9: `by` has chosen `target`, one of the other player's units on the field — as the target
/// of a declared attack (§4.2 step 2, before the trap window; `combat::declare_attack`) or as a play's,
/// a cast's, an activation's or a prompt answer's pick (§10.5 step 1; the play pipeline's half). A
/// card of the targeted unit's controller that replaces it by interposing (Classic #33 Joro, from the
/// hand) is summoned into its controller's leftmost open unit zone — no Cry, summoning sick — and
/// the attack or the pick moves to it: `summoned`, then `redirected`. Returns the new target, or `None`.
///
/// One targeting is answered once: the first card that interposes ends it ("one Joro answers one
/// targeting", Classic #33), and a card with no open zone to enter does nothing. A random pick or an "all"
/// effect chooses nobody and never comes here; nor does a forced attack, which the effect declares,
/// not the player (R121).
/// (TS `answerTargeting(sink, { target, by, what })`; the three fields are arguments.)
pub fn answer_targeting(
    sink: &mut EngineSink<'_>,
    target: &CardInstance,
    by: PlayerId,
    what: TargetedWhat,
) -> Option<CardInstance> {
    let defender = target.controller;
    if by == defender || !crate::zones::acts_on_field(sink.state, target) {
        return None;
    }
    let event = ReplacedEvent::Targeted {
        instance_id: target.id.clone(),
        controller: defender,
        by,
        what,
        source: None,
    };
    for cand in candidates(sink.state, Some(target)) {
        if cand.controller != defender {
            continue;
        }
        let Some(def) = answering(sink.state, &cand, ReplacementMoment::Targeted, &event) else {
            continue;
        };
        let mut card = cand.card.clone();
        // §3.2, R64, R688: the leftmost empty, unreserved unit zone (an unlocked one first), which
        // `place_on_field` accepts: Joro is summoned, not played.
        let Some(slot) = crate::zones::first_entry_zone(sink.state, defender, Row::Units) else {
            continue;
        };
        crate::zones::remove_from_any_zone(sink.state, &mut card);
        crate::zones::place_on_field(sink.state, &mut card, slot, Default::default());
        let turn = sink.state.turn;
        if let Some(placed) = find_instance_mut(sink.state, &card.id) {
            placed.summoned_turn = Some(turn);
        }
        sink.events.push(GameEvent::Summoned {
            player: defender,
            instance_id: card.id.clone(),
            def_id: card.def_id.clone(),
            row: slot.row,
            lane: slot.lane,
            former_id: None,
            arrived_during: None,
            exits_from: None,
        });
        sink.events.push(GameEvent::Redirected {
            what: match what {
                TargetedWhat::Attack => RedirectWhat::Attack,
                TargetedWhat::Target => RedirectWhat::Target,
            },
            from_id: target.id.clone(),
            to_id: card.id.clone(),
            by_instance_id: Some(card.id.clone()),
        });
        let record = ReplacementRecord {
            event: event.clone(),
            redirected_to: None,
            flickered: None,
        };
        finish(sink, &cand, &def, &record, false);
        return Some(live(sink.state, &card));
    }
    None
}
