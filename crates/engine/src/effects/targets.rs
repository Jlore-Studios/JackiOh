//! How an effect names what it acts on. A card script writes a spec; the engine resolves it against
//! the context, so a card file never reaches into state (CLAUDE.md rule 5). A function that names a
//! card answers with an owned copy of it as it stands now; a caller that changes the card looks it up
//! again by `id` (`state::find_instance_mut`).

use serde::{Deserialize, Serialize};

use crate::damage::DamageTarget;
use crate::faces::card_type_of;
use crate::restrictions::unaffected_by;
use crate::script::EffectContext;
use crate::state::{CardInstance, find_instance};
use crate::stays::{exit_mark, left_field_after};
use crate::wire::{CardType, PlayerId, Row, Selection, Tag, TargetAim, ZoneName, opponent_of};
use crate::zones::{
    ZoneSlot, adjacent, card_at, carried_units_of, is_buried, is_standing_unit, row_size, slot_of, slots_of,
    standing_units_of,
};

/// Which card or hero an effect names (TS `TargetSpec`, discriminated on `of`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "of", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum TargetSpec {
    /// The unit running the script.
    #[serde(rename = "self")]
    SelfCard,
    SelfHero,
    EnemyHero,
    /// A card a script already has the id of: an instance captured in a `Resume`'s data (#50 Kpop
    /// Fanatic), or one an enclosing scope enumerated. `transform`, `vanilla`, `setRadiant` and
    /// `steal` each grew a private `instanceId` argument for want of this; it belongs here, so every
    /// verb that takes a `TargetSpec` can name such a card without one of its own.
    Instance {
        instance_id: String,
    },
    /// The n-th selection the play carried (R81), default the first.
    Chosen {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
    },
}

/// `"self" | "enemy"`: a player relative to the controller.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum PlayerSpec {
    #[serde(rename = "self")]
    SelfSide,
    #[serde(rename = "enemy")]
    Enemy,
}

pub fn player_of(ctx: &EffectContext<'_>, spec: PlayerSpec) -> PlayerId {
    match spec {
        PlayerSpec::SelfSide => ctx.controller,
        PlayerSpec::Enemy => opponent_of(ctx.controller),
    }
}

pub fn resolve_target(ctx: &EffectContext<'_>, spec: &TargetSpec) -> Option<DamageTarget> {
    let index = match spec {
        TargetSpec::SelfCard => {
            // R1839000: "this" is passed by as well while it is Dormant (a Cry it runs as it is played).
            return self_on_its_stay(ctx)
                .filter(|instance| !crate::slumber::is_slumbering(instance))
                .map(|instance| DamageTarget::Unit { instance });
        }
        TargetSpec::SelfHero => {
            return Some(DamageTarget::Hero {
                player: ctx.controller,
            });
        }
        TargetSpec::EnemyHero => {
            return Some(DamageTarget::Hero {
                player: opponent_of(ctx.controller),
            });
        }
        TargetSpec::Instance { instance_id } => {
            return instance_on_its_stay(ctx, instance_id).map(|instance| DamageTarget::Unit { instance });
        }
        TargetSpec::Chosen { index } => index.unwrap_or(0),
    };

    let selection = ctx.targets.get(index)?;
    match selection {
        Selection::Hero { player } => Some(DamageTarget::Hero { player: *player }),
        Selection::Instance { instance_id } => {
            let instance = find_instance(ctx.state, instance_id)?;
            // R174: a card chosen on the field is chosen as that stay. One an earlier effect of this list
            // took off the field is gone for this one wherever it is now: back as a new arrival (R83), or
            // reset in a graveyard or a hand (R78) with nothing to land on (§8 Conventions). So too across
            // a prompt's split (R113), and for a play's target a trap took off at §10.5 step 4. A pick an
            // answer made is chosen as its prompt offered it (`ctx.chosenFrom`, §10.6).
            if left_field_after(ctx.state, stay_mark_of(ctx, &instance.id), &instance.id) {
                return None;
            }
            // §3.2, R13, R174: nor is a card on the field for an effect while it lies dormant under a Stack
            // pile — one the play's own Stack card buried at §10.5 step 4 (a crafted Felinor Fiender played
            // onto the unit its Cry chose): #61's copy and #22's meal fizzle, as #68's damage does.
            if is_buried(ctx.state, instance) {
                return None;
            }
            // B5 E35: "a Spell can't target it and doesn't affect it" — the pick the play pipeline refuses to
            // offer, and a pick that became immune since, is passed by.
            if unaffected_by(ctx, instance) {
                return None;
            }
            Some(DamageTarget::Unit {
                instance: instance.clone(),
            })
        }
        _ => None,
    }
}

/// R174: a card a script names by its id, on the stay it had when the run began: one an earlier effect
/// of the same list took off the field is gone for it wherever it is now, a Reborn body included (R83),
/// as when chosen, even across a prompt's split (R113). A card this run's own prompt picked is named on
/// the stay the prompt offered it on (`stay_mark_of`). `None` when there is no such card.
pub fn instance_on_its_stay(ctx: &EffectContext<'_>, instance_id: &str) -> Option<CardInstance> {
    let instance = find_instance(ctx.state, instance_id)?;
    // §3.2, R13: a card dormant under a Stack pile is not on the field for effects.
    if is_buried(ctx.state, instance) {
        return None;
    }
    // B5 E35: a Spell's effect passes a unit immune to Spells by, named by id as much as chosen.
    if unaffected_by(ctx, instance) {
        return None;
    }
    if left_field_after(ctx.state, stay_mark_of(ctx, &instance.id), &instance.id) {
        None
    } else {
        Some(instance.clone())
    }
}

/// R174: the card running the script as it stands now (`EffectContext::live_self`), while it is on the
/// stay it had when the run began: one an earlier effect of the list took off the field (#22's
/// sacrifice, radiant #52's bounce) is gone for "this", even once back, as is one R78 reset in a hand.
/// A card that never stood on the field in the run has no stay to lose, and a Death hook's snapshot
/// (R89) died before its hook began.
pub fn self_on_its_stay(ctx: &EffectContext<'_>) -> Option<CardInstance> {
    let this = ctx.live_self()?;
    if left_field_since(ctx, &this.id) {
        None
    } else {
        Some(this.clone())
    }
}

/// R174: whether a card is on the field on the same stay it had when the running script began — on
/// the field now, and not taken off it since. A card an earlier effect of the list bounced,
/// sacrificed or exiled has no stay left, even once it is back. A card this run's answered prompt
/// picked is on the stay the prompt offered it on (`stay_mark_of`).
pub fn stands_since_script_began(ctx: &EffectContext<'_>, instance_id: &str) -> bool {
    let Some(card) = find_instance(ctx.state, instance_id) else {
        return false;
    };
    if card.zone.z() != ZoneName::Field {
        return false;
    }
    !left_field_after(ctx.state, stay_mark_of(ctx, instance_id), instance_id)
}

/// R174: whether a card has left the field since the running script began (`ctx.exits_from`).
fn left_field_since(ctx: &EffectContext<'_>, instance_id: &str) -> bool {
    let mark = ctx.exits_from.unwrap_or_else(|| exit_mark(ctx.state));
    left_field_after(ctx.state, mark, instance_id)
}

/// R174, §10.6: the field's departures a card's stay is judged from. A card this run's answered prompt
/// picked (`ctx.chosen_from`): the stay the prompt offered it on, however the step names it (#50's
/// shape). A card the event of a queued trigger names: when that event happened (`ctx.event_stay`,
/// R212), not a Reborn body an earlier trigger made. Any other: when the run began (`ctx.exits_from`).
fn stay_mark_of(ctx: &EffectContext<'_>, instance_id: &str) -> u32 {
    let picked = ctx.chosen_from.is_some()
        && ctx.targets.iter().any(
            |selection| matches!(selection, Selection::Instance { instance_id: id } if id == instance_id),
        );
    if picked && let Some(chosen_from) = ctx.chosen_from {
        return chosen_from;
    }
    if let Some(stay) = &ctx.event_stay
        && stay.ids.iter().any(|id| id == instance_id)
    {
        return stay.from;
    }
    ctx.exits_from.unwrap_or_else(|| exit_mark(ctx.state))
}

/// The instance a `TargetSpec` names, or `None` when it named a hero or nothing.
pub fn instance_of(ctx: &EffectContext<'_>, spec: &TargetSpec) -> Option<CardInstance> {
    match resolve_target(ctx, spec)? {
        DamageTarget::Unit { instance } => Some(instance),
        _ => None,
    }
}

/// The card an effect that names one card acts on (`setRadiant`, `steal`, `transform`,
/// `grantAbility`, …). R174: a card named by id is aimed at the stay it had when the run began
/// (`instance_on_its_stay`); otherwise the unit `target` names, the first chosen selection by default.
pub fn instance_by_id_or_spec(
    ctx: &EffectContext<'_>,
    instance_id: Option<&str>,
    target: Option<&TargetSpec>,
) -> Option<CardInstance> {
    if let Some(instance_id) = instance_id {
        return instance_on_its_stay(ctx, instance_id);
    }
    instance_of(ctx, target.unwrap_or(&TargetSpec::Chosen { index: None }))
}

// Board scope: how a verb names cards nobody chose

/// `BoardScope["side"]`: sides, relative to `ctx.controller` (also `ZoneScope.side` and
/// `CardScope.side`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum ScopeSide {
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "self")]
    SelfSide,
    #[serde(rename = "enemy")]
    Enemy,
}

/// Which cards on the field a board-wide verb acts on (§3.1, §3.2). `TargetSpec` cannot carry this:
/// `resolve_target` answers with one `Option<DamageTarget>`, and every verb written in it — `damage`,
/// `destroy`, `buff`, `transform` — is single-target by construction. So the scope is its own
/// vocabulary, defined once here, and each board-wide verb is a thin walk over `cards_in_scope`.
/// `buffAllUnits`, `stealAll` and `switchAllPositions` are the same shape from before it existed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct BoardScope {
    /// Sides, relative to `ctx.controller`. Default "any".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<ScopeSide>,
    /// Rows. Default `["units"]`; a verb that reaches permanents passes both (§6.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<Row>>,
    /// Card types to keep, by the def; absent keeps every type in the named rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub types: Option<Vec<CardType>>,
    /// Tags to keep (#61's "Human unit").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,
    /// Tags to reject (#2's "non-Human", #43's "non-Felinor").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_tags: Option<Vec<Tag>>,
    /// Leave the card running the script standing (#100 "all *other* permanents").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude_self: Option<bool>,
}

/// R68's walk order: the active player's side first, then the opponent's. Exported because heroes
/// are not cards, so a sweep that also hits heroes (#13's `heroes: true`) needs the side order
/// without going through `cards_in_scope` — which answers nothing at all on a cleared board, while
/// that sweep must still reach the hero.
pub fn sides_of(ctx: &EffectContext<'_>, side: Option<ScopeSide>) -> Vec<PlayerId> {
    match side {
        Some(ScopeSide::SelfSide) => vec![ctx.controller],
        Some(ScopeSide::Enemy) => vec![opponent_of(ctx.controller)],
        _ => {
            let active = ctx.state.active;
            vec![active, opponent_of(active)]
        }
    }
}

/// MD-B1, R940: whether one card passes a scope's filters under an aim. A harmful walk whose
/// scope names a tribal tag passes an immune card by; a helpful (or aim-less) walk reaches it.
pub fn matches_scope_aimed(
    ctx: &EffectContext<'_>,
    card: &CardInstance,
    scope: &BoardScope,
    aim: TargetAim,
) -> bool {
    matches_scope(ctx, card, scope)
        && !(aim == TargetAim::Harm
            && crate::restrictions::tribal_hate_cannot_reach(
                ctx.state,
                card,
                scope.tags.as_deref(),
                scope.not_tags.as_deref(),
            ))
}

/// Whether one card passes a scope's filters. Zone membership is the caller's business.
pub fn matches_scope(ctx: &EffectContext<'_>, card: &CardInstance, scope: &BoardScope) -> bool {
    if scope.exclude_self == Some(true)
        && let Some(this) = &ctx.self_
        && card.id == this.id
    {
        return false;
    }
    // B5 E35: every effect of a Spell passes a unit immune to Spells by — "all Units" included.
    if unaffected_by(ctx, card) {
        return false;
    }
    if let Some(types) = &scope.types
        && !types.contains(&card_type_of(ctx.state, card))
    {
        return false;
    }
    // MD-B15, R923: a granted tag is part of the card for a scope too.
    let tags = crate::query::tags_of(ctx.state, card);
    if let Some(wanted) = &scope.tags
        && !wanted.iter().any(|tag| tags.contains(tag))
    {
        return false;
    }
    if let Some(not_tags) = &scope.not_tags
        && not_tags.iter().any(|tag| tags.contains(tag))
    {
        return false;
    }
    true
}

/// The slots a scope covers, in R68 order: side by side, then row by row, lane 1 upward.
fn slots_in_scope(ctx: &EffectContext<'_>, scope: &BoardScope) -> Vec<ZoneSlot> {
    let rows: Vec<Row> = scope.rows.clone().unwrap_or_else(|| vec![Row::Units]);
    sides_of(ctx, scope.side)
        .into_iter()
        .flat_map(|player| {
            rows.iter()
                .flat_map(move |row| slots_of(player, *row))
                .collect::<Vec<ZoneSlot>>()
        })
        .collect()
}

/// Every card on the field a scope matches, in R68 order (the active player's side first, then the
/// opponent's; within a side the named rows lane 1 upward). Only the top card of a Stack pile is on
/// the field, so a dormant card underneath is never matched (§3.2, R13).
pub fn cards_in_scope(ctx: &EffectContext<'_>, scope: &BoardScope) -> Vec<CardInstance> {
    cards_in_scope_with(ctx, scope, TargetAim::Help)
}

/// MD-B1, R940: `cards_in_scope` under an aim. Harmful verbs pass `TargetAim::Harm` so an immune
/// card is passed by; helpful and neutral walks keep the default.
pub fn cards_in_scope_aimed(
    ctx: &EffectContext<'_>,
    scope: &BoardScope,
    aim: TargetAim,
) -> Vec<CardInstance> {
    cards_in_scope_with(ctx, scope, aim)
}

fn cards_in_scope_with(ctx: &EffectContext<'_>, scope: &BoardScope, aim: TargetAim) -> Vec<CardInstance> {
    let mut out: Vec<CardInstance> = Vec::new();
    for slot in slots_in_scope(ctx, scope) {
        // R1081000: a Unit standing in a backrow zone is no backrow card, as an animated one is none
        // (§6.1 Animated): a backrow scope passes it by, and a unit scope meets it below.
        if let Some(card) = card_at(ctx.state, slot)
            && !(slot.row == Row::Backrow && is_standing_unit(ctx.state, card))
            && matches_scope_aimed(ctx, card, scope, aim)
        {
            out.push(card.clone());
        }
        // R446: a Unit a carrier holds is one of that side's units ("all Units" reach it), after the unit
        // lanes; a backrow scope finds the carrier beneath it and never the Unit.
        if slot.row == Row::Units && slot.lane == row_size(Row::Units) {
            for unit in carried_units_of(ctx.state, slot.player) {
                if matches_scope_aimed(ctx, unit, scope, aim) {
                    out.push(unit.clone());
                }
            }
            // R1081000: and so is a Unit standing in a backrow zone, after those.
            for unit in standing_units_of(ctx.state, slot.player) {
                if matches_scope_aimed(ctx, unit, scope, aim) {
                    out.push(unit.clone());
                }
            }
        }
    }
    out
}

/// MD-B1, R940: `adjacent_to` under an aim. Harmful verbs pass `TargetAim::Harm`.
pub fn adjacent_to_aimed(
    ctx: &EffectContext<'_>,
    spec: &TargetSpec,
    scope: &BoardScope,
    aim: TargetAim,
) -> Vec<CardInstance> {
    adjacent_to_with(ctx, spec, scope, aim)
}

/// §3.1 Adjacent: lanes N-1 and N+1 on the target's own side and row, never across sides and never
/// the target itself. An empty or dormant neighbour contributes nothing (§3.2, R13). The target may
/// be anywhere the spec can name it; off the field it has no neighbours.
pub fn adjacent_to(ctx: &EffectContext<'_>, spec: &TargetSpec, scope: &BoardScope) -> Vec<CardInstance> {
    adjacent_to_with(ctx, spec, scope, TargetAim::Help)
}

fn adjacent_to_with(
    ctx: &EffectContext<'_>,
    spec: &TargetSpec,
    scope: &BoardScope,
    aim: TargetAim,
) -> Vec<CardInstance> {
    let Some(card) = instance_of(ctx, spec) else {
        return Vec::new();
    };
    if card.zone.z() != ZoneName::Field {
        return Vec::new();
    }
    let Some(slot) = slot_of(ctx.state, &card) else {
        return Vec::new();
    };

    let mut out: Vec<CardInstance> = Vec::new();
    for neighbour in adjacent(slot) {
        let Some(found) = card_at(ctx.state, neighbour) else {
            continue;
        };
        if found.id == card.id {
            continue;
        }
        // R1081000: beside a Unit standing in the backrow, only a Unit standing there is adjacent.
        if !crate::zones::is_adjacent_card(ctx.state, &card, found) {
            continue;
        }
        if !matches_scope_aimed(ctx, found, scope, aim) {
            continue;
        }
        out.push(found.clone());
    }
    out
}
