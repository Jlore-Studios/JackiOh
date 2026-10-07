//! How an effect names what it acts on. A card script writes a spec; the engine resolves it against
//! the context, so a card file never reaches into state (CLAUDE.md rule 5).
//!
//! Port of `packages/engine/src/effects/targets.ts`. TS handed back the live `CardInstance` objects
//! a caller then wrote through; here every function that names a card answers with a copy of it as
//! it stands now (`CardInstance`, owned), and a caller that changes the card looks it up again by
//! `id` (`state::find_instance_mut`).

use serde::{Deserialize, Serialize};

use crate::catalog::def_of;
use crate::damage::DamageTarget;
use crate::faces::card_type_of;
use crate::restrictions::unaffected_by;
use crate::script::EffectContext;
use crate::state::{CardInstance, find_instance};
use crate::stays::{exit_mark, left_field_after};
use crate::wire::{CardType, PlayerId, Row, Selection, Tag, ZoneName, opponent_of};
use crate::zones::{ZoneSlot, adjacent, card_at, carried_units_of, is_buried, row_size, slot_of, slots_of};

/// Which card or hero an effect names (TS `TargetSpec`, discriminated on `of`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
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
    Instance { instance_id: String },
    /// The n-th selection the play carried (R81), default the first.
    Chosen {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
    },
}

/// `"self" | "enemy"`: a player relative to the controller.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
            return self_on_its_stay(ctx).map(|instance| DamageTarget::Unit { instance });
        }
        TargetSpec::SelfHero => return Some(DamageTarget::Hero { player: ctx.controller }),
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
            // R174: a card chosen on the field is chosen as that stay. One an earlier effect of this same
            // list took off the field is gone for this one, wherever it is now: back already — a fused
            // card's #22 half sacrificed it and Reborn returned it, a new arrival (R83) — or in a graveyard
            // or a hand, reset (R78), where a buff or an exile aimed at the unit on the field has nothing to
            // land on (§8 Conventions). The same holds when a prompt split the list across actions (R113),
            // and for a play's declared target a trap answering the play took off the field at §10.5 step 4
            // (the play's Cry runs with the mark step 1 checked the choices at). A pick an answer made is
            // chosen as its prompt offered it (`ctx.chosenFrom`, §10.6): a Reborn body the list's own
            // sacrifice put back before it asked is the stay that was picked.
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

/// R174: a card a script names by its id — captured in a continuation's data, read off the event a
/// trigger answers, or enumerated by the list itself — on the stay it had when the run began. An
/// effect later in the list is aimed at the card the list named, and a card an earlier effect of the
/// same list took off the field is gone for it wherever it is now, a Reborn body included (R83);
/// naming it by id rather than as "the chosen one" changes nothing, and neither does a prompt that
/// split the list (R113). A card the answer to this run's own prompt picked is named on the stay
/// the prompt offered it on, by id as much as as the chosen card (`stay_mark_of`). `None` when there
/// is no such card.
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

/// R174: the card running the script, while it is on the stay it had when the run began — a card an
/// earlier effect of the same list took off the field (#22's sacrifice, radiant #52's bounce) is gone
/// for "this", even once it is back, and a card R78 has reset in a hand is not what the effect was
/// aimed at. A card that never stood on the field in the run (a Spell resolving, a hand card) has no
/// stay to lose, and a Death hook's snapshot (R89) died before its hook began.
///
/// TS answered the live `ctx.self`: this is the card as it stands now (`EffectContext::live_self`).
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

/// R174, §10.6: the field's departures a card's stay is judged from. A card the answer to this run's
/// own prompt picked (`ctx.targets` with `ctx.chosen_from`) is picked on the stay the prompt offered it
/// on — a Reborn body the list's own sacrifice put back before it asked — and the answered step is
/// aimed at that stay however it names the card: as the chosen one, or by the id it read off the
/// selection, as a delayed effect that watches it (#50's shape), a steal, a Transform or a Make
/// Radiant by id do. A card the event a queued trigger answers names is judged from when that event
/// happened (`ctx.event_stay`, R212): the played unit a trigger reads off its `cardPlayed` is not the
/// Reborn body an earlier trigger on the same play made. Any other card is judged from when the run
/// began (`ctx.exits_from`) — a card a trigger reads off the board as it resolves included.
fn stay_mark_of(ctx: &EffectContext<'_>, instance_id: &str) -> u32 {
    let picked = ctx.chosen_from.is_some()
        && ctx.targets.iter().any(|selection| {
            matches!(selection, Selection::Instance { instance_id: id } if id == instance_id)
        });
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

// ---------------------------------------------------------------------------
// Board scope: how a verb names cards nobody chose
// ---------------------------------------------------------------------------

/// `BoardScope["side"]`: sides, relative to `ctx.controller` (also `ZoneScope.side` and
/// `CardScope.side`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
            // TS checked `PLAYER_IDS.includes(active)`; a `PlayerId` always is one.
            let active = ctx.state.active;
            vec![active, opponent_of(active)]
        }
    }
}

/// Whether one card passes a scope's filters. Zone membership is the caller's business. (TS's
/// default `scope = {}` is `&BoardScope::default()`.)
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
    let def = def_of(Some(&*ctx.state), &card.def_id);
    if let Some(types) = &scope.types
        && !types.contains(&card_type_of(ctx.state, card))
    {
        return false;
    }
    if let Some(tags) = &scope.tags
        && !tags.iter().any(|tag| def.tags.contains(tag))
    {
        return false;
    }
    if let Some(not_tags) = &scope.not_tags
        && not_tags.iter().any(|tag| def.tags.contains(tag))
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
    let mut out: Vec<CardInstance> = Vec::new();
    for slot in slots_in_scope(ctx, scope) {
        if let Some(card) = card_at(ctx.state, slot)
            && matches_scope(ctx, card, scope)
        {
            out.push(card.clone());
        }
        // R446: a Unit a carrier holds is one of that side's units ("all Units" reach it), after the unit
        // lanes; a backrow scope finds the carrier beneath it and never the Unit.
        if slot.row == Row::Units && slot.lane == row_size(Row::Units) {
            for unit in carried_units_of(ctx.state, slot.player) {
                if matches_scope(ctx, unit, scope) {
                    out.push(unit.clone());
                }
            }
        }
    }
    out
}

/// §3.1 Adjacent: lanes N-1 and N+1 on the target's own side and row, never across sides and never
/// the target itself. An empty or dormant neighbour contributes nothing (§3.2, R13). The target may
/// be anywhere the spec can name it; off the field it has no neighbours.
pub fn adjacent_to(ctx: &EffectContext<'_>, spec: &TargetSpec, scope: &BoardScope) -> Vec<CardInstance> {
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
        if !matches_scope(ctx, found, scope) {
            continue;
        }
        out.push(found.clone());
    }
    out
}
