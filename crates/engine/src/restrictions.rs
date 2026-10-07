//! What a card may not be done to (docs/classic-sets.md B5 E35): the one place a targeting rule, a
//! damage step or an attack validator asks whether a card is out of an effect's reach, so the answer a
//! client greys a pick out by and the answer the reducer refuses it by are one call (§10.2).
//!
//! Two families live here.
//!
//!  * Immune to Spells: "a Spell can't target it and doesn't affect it". The targeting half is the
//!    play pipeline's (`spell_cannot_reach` in `play_choices`); the "doesn't affect it" half is
//!    `effects/targets.rs`, which asks `effect_is_from_spell` for every card a single target or a scope
//!    names, so every verb written in that vocabulary passes an immune unit by.
//!  * The attack restrictions: can't be attacked (Classic+ #51), attacked only from its own lane
//!    (Classic+ #19.1), can't attack or be attacked (a carried Unit, R446). `combat.rs` asks
//!    `attack_restriction` for a declared attack (§4.2 step 2), for the Taunt wall (step 3: a Taunt
//!    binds only the attackers that may attack it) and for every forced attack, which skips steps 1 to
//!    3 (R53) but not these.
//!
//! A restriction a card prints is a static flag (`Script.staticFlags`), so a Vanilla takes it with the
//! rest of the text (`scripts::script_of`).
//!
//! Port of `packages/engine/src/restrictions.ts`. Not ported (SURFACE §6.6): TS's positional bars
//! (`registerAttackBar`, `AttackBar`, `AttackBarAnswer` and the `bars` registry the attack checks also
//! consulted). Nothing ever registered one — a carried Unit's "can't attack or be attacked" is read off
//! where it stands by `combat.rs` (`zones::is_carried`) — so the registry was always empty and every
//! check here is its flag alone.

use crate::catalog::def_of;
use crate::damage::DamageTarget;
use crate::layers::unit_has;
use crate::script::{EffectContext, Script, StaticFlags, empty_script};
use crate::state::{CardInstance, EngineError, GameState};
use crate::wire::{CardType, KeywordKind, Row, Zone};
use crate::zones::{card_at, slot_of};

/// `faces::card_type_of` for a definition and a face alone (TS passed `{ defId, radiant }`, a card
/// with no zone): the running face's own type, else the definition's.
fn type_of_face(state: &GameState, def_id: &str, radiant: bool) -> CardType {
    let def = def_of(Some(state), def_id);
    let face = if radiant { &def.radiant } else { &def.base };
    face.type_.unwrap_or(def.type_)
}

/// E35: whether the card has Immune to Spells now (a keyword, so layered: printed, granted, aura).
pub fn immune_to_spells(state: &GameState, card: &CardInstance) -> bool {
    unit_has(state, card, KeywordKind::ImmuneToSpells)
}

/// E35: whether `source` is a Spell — the type "Immune to Spells" answers (not a Field Spell or Trap).
pub fn is_spell_source(state: &GameState, source: Option<&CardInstance>) -> bool {
    match source {
        Some(source) => crate::faces::card_type_of(state, source) == CardType::Spell,
        None => false,
    }
}

/// E35: "a Spell can't target it and doesn't affect it". True when `source` is a Spell and `card` is
/// immune to Spells, so the effect passes the card by as if it were not there.
pub fn spell_cannot_reach(state: &GameState, source: Option<&CardInstance>, card: &CardInstance) -> bool {
    is_spell_source(state, source) && immune_to_spells(state, card)
}

/// E35: whether the effect list running in `ctx` is a Spell's. Its card says so while it is there; a
/// continuation re-entered once the card has gone (R98's resolving Spell returned to a hand, R127's
/// card that ceased to exist) still names the script it runs (`ctx.def_id`), on the face it recorded.
pub fn effect_is_from_spell(ctx: &EffectContext<'_>) -> bool {
    // TS read the live `ctx.self`: the card as it stands now, which is what its type is judged by.
    if let Some(card) = ctx.live_self() {
        return is_spell_source(ctx.sink.state, Some(card));
    }
    match ctx.def_id.as_deref() {
        None | Some("") => false,
        Some(def_id) => type_of_face(ctx.sink.state, def_id, ctx.radiant) == CardType::Spell,
    }
}

/// E35: the "doesn't affect it" half, as `effects/targets.rs` asks it of every card an effect names —
/// a Spell's effects pass an immune unit on the field by. A card in a hand or a deck is no unit, and
/// its printed keyword protects nothing there.
pub fn unaffected_by(ctx: &EffectContext<'_>, card: &CardInstance) -> bool {
    matches!(card.zone, Zone::Field { .. }) && effect_is_from_spell(ctx) && immune_to_spells(ctx.sink.state, card)
}

// ---------------------------------------------------------------------------
// The attack restrictions
// ---------------------------------------------------------------------------

/// E35: this unit may make no attack, declared or forced ("can't attack or be attacked").
pub fn cannot_attack(state: &GameState, unit: &CardInstance) -> bool {
    crate::scripts::flags_of(state, unit).cant_attack_or_be_attacked == Some(true)
}

/// E35: no attack may be made on this unit, declared or forced. Effects still target and hit it.
pub fn cannot_be_attacked(state: &GameState, unit: &CardInstance) -> bool {
    let flags = crate::scripts::flags_of(state, unit);
    flags.cant_be_attacked == Some(true) || flags.cant_attack_or_be_attacked == Some(true)
}

/// E35: only a unit standing in this unit's lane may attack it. (TS took the unit alone; its flags are
/// read through the state here, which composes a fused card's script.)
pub fn attackable_only_from_lane(state: &GameState, unit: &CardInstance) -> bool {
    crate::scripts::flags_of(state, unit).attacked_only_from_lane == Some(true)
}

/// E35: why the unit restrictions bar `attacker` from attacking `target` — a declared attack or a
/// forced one alike (R53 waives position, sickness and Taunt, never these) — or `Ok` when nothing
/// does (SURFACE §4.4.9). A hero carries no restriction of its own.
pub fn attack_restriction(state: &GameState, attacker: &CardInstance, target: &DamageTarget) -> Result<(), EngineError> {
    if cannot_attack(state, attacker) {
        return Err(EngineError::new("that unit cannot attack"));
    }
    let defender = match target {
        DamageTarget::Hero { .. } => return Ok(()),
        DamageTarget::Unit { instance } => instance,
    };
    if cannot_be_attacked(state, defender) {
        return Err(EngineError::new("that unit cannot be attacked"));
    }
    if attackable_only_from_lane(state, defender) {
        let from = slot_of(state, attacker);
        let to = slot_of(state, defender);
        let same_lane = match (from, to) {
            (Some(from), Some(to)) => from.lane == to.lane,
            _ => false,
        };
        if !same_lane {
            return Err(EngineError::new("only a unit in its lane may attack that unit"));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Berserk
// ---------------------------------------------------------------------------

/// B5 E35: whether the unit has gone Berserk — a status, kept until it leaves the field (R78).
pub fn is_berserk(card: &CardInstance) -> bool {
    card.berserk == Some(true)
}

/// B5 E35: whether the unit may go Berserk now: on the field, not Berserk yet, and not immune to it.
pub fn can_go_berserk(state: &GameState, card: &CardInstance) -> bool {
    if !matches!(card.zone, Zone::Field { row: Row::Units, .. }) {
        return false;
    }
    if !active_on_field(state, card) {
        return false;
    }
    card.berserk != Some(true) && crate::scripts::flags_of(state, card).never_berserk != Some(true)
}

/// The card that acts in its zone: on the field and the top of its pile (§3.2, R13).
fn active_on_field(state: &GameState, card: &CardInstance) -> bool {
    match slot_of(state, card) {
        Some(at) => card_at(state, &at).is_some_and(|held| held.id == card.id),
        None => false,
    }
}
