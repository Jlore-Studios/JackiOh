//! What a card may not be done to (docs/classic-sets.md B5 E35): the one place a targeting rule, a damage
//! step or an attack validator asks whether a card is out of an effect's reach, so the answer a client
//! greys a pick out by and the answer the reducer refuses it by are one call (§10.2).
//!
//!  * Immune to Spells: targeting is `play_choices`'s; "doesn't affect it" is `effects/targets.rs`'s, which
//!    asks `effect_is_from_spell` for every card a target or a scope names.
//!  * Immutable's targeting half (R781001, R450): every declaration and `target` prompt asks
//!    `immutable_bars`, as does a Field Trap naming its one card (Classic #5 Tesla, Classic+ #74); random
//!    picks, scopes and attacks still reach it. Its text half (R23) is read at each verb.
//!  * The attack restrictions (Classic+ #51, #19.1, a carried Unit's R446, Flying's #783, the Gauntlet's
//!    R1849005): `combat.rs` asks `attack_restriction` for a declared attack (§4.2 step 2), the Taunt wall
//!    (step 3: a Taunt binds only who may attack it) and every forced attack, which skips steps 1 to 3
//!    (R53) but not these.
//!
//! Beside them, Stinkyscape's (#1849) Immune to Spell Damage (R1849000) and "Units cannot be in Defense
//! Position" (R1849006). A printed restriction is a static flag, so a Vanilla takes it with the text. A
//! carried Unit's bar is read where it stands (`zones::is_carried`), not registered (SURFACE §6.6).

use crate::config::GAUNTLET_LANES;
use crate::damage::DamageTarget;
use crate::faces::card_type_of_face;
use crate::layers::{AuraTable, unit_has};
use crate::script::{EffectContext, StaticFlags};
use crate::state::{CardInstance, EngineError, GameState, Position};
use crate::wire::{CardType, KeywordKind, PLAYER_IDS, Row, Zone, has_keyword};
use crate::zones::{card_at, slot_of};

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

/// E35: "a Spell can't target it and doesn't affect it", for an immune `card` and a Spell `source`.
pub fn spell_cannot_reach(state: &GameState, source: Option<&CardInstance>, card: &CardInstance) -> bool {
    is_spell_source(state, source) && immune_to_spells(state, card)
}

/// E35: whether the effect list running in `ctx` is a Spell's. Its card says so while it is there; a
/// continuation re-entered once the card has gone (R98's resolving Spell returned to a hand, R127's
/// card that ceased to exist) still names the script it runs (`ctx.def_id`), on the face it recorded.
pub fn effect_is_from_spell(ctx: &EffectContext<'_>) -> bool {
    // The card as it stands now is what its type is judged by.
    if let Some(card) = ctx.live_self() {
        return is_spell_source(ctx.sink.state, Some(card));
    }
    match ctx.def_id.as_deref() {
        None | Some("") => false,
        Some(def_id) => card_type_of_face(ctx.sink.state, def_id, ctx.radiant) == CardType::Spell,
    }
}

/// E35: the "doesn't affect it" half, as `effects/targets.rs` asks it of every card an effect names:
/// a Spell's effects pass an immune unit on the field by. R1839000: every effect, whatever its
/// source, passes a Dormant card by; only `effects::awaken` names it.
pub fn unaffected_by(ctx: &EffectContext<'_>, card: &CardInstance) -> bool {
    if crate::slumber::is_slumbering(card) {
        return true;
    }
    matches!(card.zone, Zone::Field { .. })
        && effect_is_from_spell(ctx)
        && immune_to_spells(ctx.sink.state, card)
}

/// R781001: the type Immutable's targeting rule reads off a source: a Unit in a unit zone (an Animated
/// card included), else its running face's printed type, so a revealed face-down Spell (ME-ALTPLAY,
/// R1044) is a Spell and a Field Trap firing from the backrow a Field Trap, even if it animates later.
pub fn targeting_type_of(state: &GameState, source: &CardInstance) -> CardType {
    if let Zone::Field { row: Row::Units, .. } = source.zone {
        return CardType::Unit;
    }
    card_type_of_face(state, &source.def_id, source.radiant)
}

/// R781001: whether the card is Immutable in play: on the field, face-up (a face-down card's text is
/// dormant, and reading it would tell the opponent what it is), with the keyword now.
pub fn immutable_in_play(state: &GameState, card: &CardInstance) -> bool {
    matches!(card.zone, Zone::Field { .. })
        && !crate::preview::is_face_down(state, card)
        && unit_has(state, card, KeywordKind::Immutable)
}

/// R781001: "While in play, Spells, Field Spells and Field Traps can't target it": true when `source`
/// is one of those (`targeting_type_of`) and `card` is Immutable in play, its controller's own cards
/// included. A Unit, a Trap that is not a Field Trap, and no source at all may target it.
pub fn immutable_bars(state: &GameState, source: Option<&CardInstance>, card: &CardInstance) -> bool {
    let Some(source) = source else {
        return false;
    };
    matches!(
        targeting_type_of(state, source),
        CardType::Spell | CardType::FieldSpell | CardType::FieldTrap
    ) && immutable_in_play(state, card)
}

/// MD-B1, R940: whether the card holds the tribal-hate immunity now (a static flag).
/// It protects the card only while the card acts on the field.
pub fn immune_to_tribal_hate(state: &GameState, card: &CardInstance) -> bool {
    matches!(card.zone, Zone::Field { .. })
        && crate::scripts::flags_of(state, card).immune_to_tribal_hate == Some(true)
}

/// MD-B1, R940: whether a harmful effect naming a tribal tag (`TRIBAL_TAGS`) in its card
/// filter's `tags` or `not_tags` can neither pick nor reach `card`, when it holds the immunity.
/// Scopes without a tribal tag and helpful effects (the caller passes the aim) still reach it.
pub fn tribal_hate_cannot_reach(
    state: &GameState,
    card: &CardInstance,
    tags: Option<&[crate::wire::Tag]>,
    not_tags: Option<&[crate::wire::Tag]>,
) -> bool {
    if !immune_to_tribal_hate(state, card) {
        return false;
    }
    let names_tribal = |list: Option<&[crate::wire::Tag]>| {
        list.is_some_and(|list| list.iter().any(|tag| crate::config::TRIBAL_TAGS.contains(tag)))
    };
    names_tribal(tags) || names_tribal(not_tags)
}

/// Whether any card acting on either side (§3.2; a face-down card's text asleep, R1487000, a Dormant
/// one's off, R1839000) carries a static flag `has` finds: an aura that reaches both players.
fn any_acting_flag(state: &GameState, has: impl Fn(&StaticFlags) -> bool) -> bool {
    PLAYER_IDS.into_iter().any(|side| {
        crate::zones::acting_on(state, side, crate::zones::StandingAt::AmongUnits)
            .into_iter()
            .filter(|card| !crate::preview::lasting_text_off(state, card))
            .any(|card| has(&crate::scripts::flags_of(state, card)))
    })
}

/// R1849000: "Immune to Spell Damage" — a Spell's hit on this Unit on the field deals nothing and is no
/// damage instance (§4.4 step 0). Only damage: a Spell still targets it and does the rest to it.
pub fn spell_damage_cannot_reach(
    state: &GameState,
    source: Option<&CardInstance>,
    unit: &CardInstance,
) -> bool {
    matches!(unit.zone, Zone::Field { .. })
        && is_spell_source(state, source)
        && crate::scripts::flags_of(state, unit).immune_to_spell_damage == Some(true)
}

/// R1849005: whether a Gauntlet acts on the field now (Stinkyscape #157).
pub fn gauntlet_acts(state: &GameState) -> bool {
    any_acting_flag(state, |flags| flags.gauntlet == Some(true))
}

/// R1849005: whether the unit stands in the Gauntlet — a unit zone of one of `GAUNTLET_LANES`, on either
/// side. A Unit a carrier holds or one standing in the backrow is outside it.
pub fn in_gauntlet(state: &GameState, unit: &CardInstance) -> bool {
    slot_of(state, unit).is_some_and(|at| at.row == Row::Units && GAUNTLET_LANES.contains(&at.lane))
}

/// R1849006: whether an aura that Units cannot be in Defense Position acts on the field now (Stinkyscape
/// #23 Warp Disruptor). It reaches both sides.
pub fn defense_barred(state: &GameState) -> bool {
    any_acting_flag(state, |flags| flags.units_never_defense == Some(true))
}

// The attack restrictions

/// R1839000: why a Dormant card may neither attack, be attacked, switch its position nor be picked —
/// the one reason `legal_actions`, the reducer and the view all give.
pub const SLUMBER_REFUSAL: &str = "that card is Dormant";

/// E35: this unit may make no attack, declared or forced ("can't attack or be attacked").
pub fn cannot_attack(state: &GameState, unit: &CardInstance) -> bool {
    crate::scripts::flags_of(state, unit).cant_attack_or_be_attacked == Some(true)
}

/// E35: no attack may be made on this unit, declared or forced. Effects still target and hit it.
pub fn cannot_be_attacked(state: &GameState, unit: &CardInstance) -> bool {
    let flags = crate::scripts::flags_of(state, unit);
    flags.cant_be_attacked == Some(true) || flags.cant_attack_or_be_attacked == Some(true)
}

/// E35: only a unit standing in this unit's lane may attack it.
pub fn attackable_only_from_lane(state: &GameState, unit: &CardInstance) -> bool {
    crate::scripts::flags_of(state, unit).attacked_only_from_lane == Some(true)
}

/// E35: why the unit restrictions bar `attacker` from `target`, declared or forced (R53), or `Ok`
/// when nothing does (SURFACE §4.4.9). A hero carries no restriction of its own.
pub fn attack_restriction(
    state: &GameState,
    attacker: &CardInstance,
    target: &DamageTarget,
) -> Result<(), EngineError> {
    attack_restriction_in(&AuraTable::per_read(state), attacker, target)
}

/// `attack_restriction` read through a sweep's `AuraTable` (#892).
pub fn attack_restriction_in(
    auras: &AuraTable<'_>,
    attacker: &CardInstance,
    target: &DamageTarget,
) -> Result<(), EngineError> {
    let state = auras.state();
    // R1839000: a Dormant card neither attacks nor is attacked, declared or forced.
    if crate::slumber::is_slumbering(attacker) {
        return Err(EngineError::new(SLUMBER_REFUSAL));
    }
    if cannot_attack(state, attacker) {
        return Err(EngineError::new("that unit cannot attack"));
    }
    // R1849005: inside the Gauntlet a Unit attacks only a Unit inside it, never a hero, and from
    // outside no Unit attacks one inside. The cheap half first: the board walk only when they differ.
    let inside = in_gauntlet(state, attacker);
    let target_inside = matches!(target, DamageTarget::Unit { instance } if in_gauntlet(state, instance));
    if inside != target_inside && gauntlet_acts(state) {
        return Err(EngineError::new("the Gauntlet's Units fight only each other"));
    }
    let defender = match target {
        DamageTarget::Hero { .. } => return Ok(()),
        DamageTarget::Unit { instance } => instance,
    };
    if crate::slumber::is_slumbering(defender) {
        return Err(EngineError::new(SLUMBER_REFUSAL));
    }
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
    if flies_out_of_reach(auras, attacker, defender) {
        return Err(EngineError::new(
            "only a Flying or Ranged unit may attack that unit",
        ));
    }
    Ok(())
}

/// R783002, R783003: a Flying unit in Attack Position may be attacked only by a unit with Flying or
/// Ranged; in Defense Position it has landed, and anything may attack it.
fn flies_out_of_reach(auras: &AuraTable<'_>, attacker: &CardInstance, defender: &CardInstance) -> bool {
    let view = auras.unit_view(defender);
    if !has_keyword(&view.keywords, KeywordKind::Flying) || view.position == Position::Def {
        return false;
    }
    !auras.unit_has(attacker, KeywordKind::Flying) && !auras.unit_has(attacker, KeywordKind::Ranged)
}

// Berserk

/// B5 E35: whether the unit has gone Berserk — a status, kept until it leaves the field (R78).
pub fn is_berserk(card: &CardInstance) -> bool {
    card.berserk == Some(true)
}

/// B5 E35: whether the unit may go Berserk now: on the field, not Berserk yet, and not immune to it.
pub fn can_go_berserk(state: &GameState, card: &CardInstance) -> bool {
    // R1081000: a Unit standing in the backrow goes Berserk as one in a unit zone does.
    if !crate::zones::in_units_row_or_standing(state, card) {
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
        Some(at) => card_at(state, at).is_some_and(|held| held.id == card.id),
        None => false,
    }
}
