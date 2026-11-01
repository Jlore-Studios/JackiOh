//! Transform and Vanilla (SPEC §6.3): the two verbs that rewrite what a card is, so Immutable
//! refuses both (R23). A Transform puts a new instance of another definition in the same zone and
//! position, with no Cry (R1); the card it replaces ceases to exist rather than reaching a graveyard
//! (R35). Vanilla is a flag the layers read (§10.4): stats, buffs and damage are other layers and
//! must survive it, and the definition is shared by every copy of the card, so it is never edited.
//! A retired card goes to `zones::cease_to_exist` as the copy read here: it has already left its pile.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::catalog::{CatalogQueryArgs, def_of, excluding_def_id, glitch_or_not, pick_generated, query};
use crate::damage::DamageTarget;
use crate::enchantments::add_enchantment;
use crate::faces::card_type_of;
use crate::layers::{unit_has, unit_view};
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut, new_instance};
use crate::temporary::is_temporary_card;
use crate::wire::{
    AttackHealth, CardDef, CardType, Enchantment, GameEvent, Keyword, KeywordKind, PLAYER_IDS, PlayerId, Row,
    Tuning, Zone, ZoneName, opponent_of,
};
use crate::zones::{
    LibraryPosition, MoveToZoneOptions, OffFieldZone, cease_to_exist, is_carried, move_to_zone, pile_at,
    replace_in_zone, slot_of, zone_of,
};

use super::card_scope::{CardScope, CardScopeOptions, cards_in_card_scope};
use super::summon::{clone_of, row_of};
use super::targets::{TargetSpec, instance_by_id_or_spec, instance_on_its_stay, resolve_target};

/// Which card to rewrite: the pick the play or a prompt carried (R81), or an instance id a trigger
/// read off the event it answers — Sheepish transforms the unit the opponent just played (#41).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TransformTarget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
}

/// §6.3 Replace: the same zone and position, owner and controller. The new card takes the old one's
/// place (`zones::replace_in_zone`): a Transform result is no summon (§6.2), so neither a §3.2 Lock nor
/// a reservation (R64) refuses it, #36's locks (R46, R15) included (R35). A Stack pile keeps its
/// dormant cards beneath the replacement (§3.2).
fn replace_on_field(
    ctx: &mut EffectContext<'_>,
    old: &CardInstance,
    def: &CardDef,
    radiant: bool,
) -> Option<CardInstance> {
    let at = slot_of(ctx.state, old)?;
    // R446, R1081000: a Unit a carrier holds, or one standing in a backrow zone, is replaced as a Unit.
    let row_here = if is_carried(ctx.state, old) || crate::zones::stands_in_backrow(ctx.state, old) {
        Row::Units
    } else {
        at.row
    };
    if row_of(def.type_) != Some(row_here) {
        return None;
    }

    let turn = ctx.state.turn;
    let mut replacement = new_instance(&mut *ctx.state, &def.id, old.owner, zone_of(at));
    replacement.radiant = radiant;
    if old.position.is_some() {
        replacement.position = old.position;
    }
    // R659: a new body is summoning sick like a summoned card (§4.1) unless text lifts it (R424).
    replacement.summoned_turn = Some(turn);
    crate::generators::record_generator(ctx, &replacement);
    let replacement_id = replacement.id.clone();

    if !replace_in_zone(&mut *ctx.state, old, &mut replacement) {
        return None;
    }
    // §6.3, R35: no graveyard, no exile, no Death trigger; R174 still counts it as leaving the field.
    let mut retired = old.clone();
    cease_to_exist(&mut *ctx.state, &mut retired);
    // §3.2: a Field Spell is public where a Trap stays face-down until it fires (R33).
    if def.type_ == CardType::FieldSpell
        && let Some(live) = find_instance_mut(ctx.state, &replacement_id)
    {
        live.face_up = Some(true);
    }
    find_instance(ctx.state, &replacement_id).cloned()
}

fn off_field_pile(ctx: &EffectContext<'_>, player: PlayerId, at: OffFieldZone) -> Vec<CardInstance> {
    let side = &ctx.state.players[player];
    match at {
        OffFieldZone::Hand => side.hand.clone(),
        OffFieldZone::Library => side.library.clone(),
        OffFieldZone::Graveyard => side.graveyard.clone(),
        OffFieldZone::Exile => side.exile.clone(),
    }
}

/// R35's other zones ("same counts" per zone). A library and a hand keep their order, so the
/// replacement takes the old card's index there; `move_to_zone` appends in the other piles.
fn replace_off_field(
    ctx: &mut EffectContext<'_>,
    old: &CardInstance,
    def: &CardDef,
    radiant: bool,
) -> Option<CardInstance> {
    // A card mid-resolution, or one that has ceased to exist (R11), is in no pile to replace it in.
    let at = match old.zone.z() {
        ZoneName::Hand => OffFieldZone::Hand,
        ZoneName::Library => OffFieldZone::Library,
        ZoneName::Graveyard => OffFieldZone::Graveyard,
        ZoneName::Exile => OffFieldZone::Exile,
        ZoneName::Field | ZoneName::Resolving | ZoneName::Gone => return None,
    };
    // R11: a unit token would cease to exist in a graveyard or exile, so it is refused, not thinning it.
    if matches!(at, OffFieldZone::Graveyard | OffFieldZone::Exile) && def.token && def.type_ == CardType::Unit
    {
        return None;
    }

    let owner = old.owner;
    let index = off_field_pile(ctx, owner, at)
        .iter()
        .position(|card| card.id == old.id)?;

    let mut retired = old.clone();
    cease_to_exist(&mut *ctx.state, &mut retired);
    // R312: no `knownAs`, so its owner's list counts a library replacement unknown (R177).
    let zone = match at {
        OffFieldZone::Hand => Zone::Hand { player: owner },
        OffFieldZone::Library => Zone::Library { player: owner },
        OffFieldZone::Graveyard => Zone::Graveyard { player: owner },
        OffFieldZone::Exile => Zone::Exile { player: owner },
    };
    let mut replacement = new_instance(&mut *ctx.state, &def.id, owner, zone);
    replacement.radiant = radiant;
    crate::generators::record_generator(ctx, &replacement);
    let replacement_id = replacement.id.clone();
    let made = replacement.clone();
    let _ = move_to_zone(
        &mut *ctx.state,
        &mut replacement,
        at,
        MoveToZoneOptions {
            position: Some(LibraryPosition::At(index as i32)),
            ..MoveToZoneOptions::default()
        },
    );
    // R671: a hand keeps its order too, so a card swapped in hand stays where its owner put it.
    if at == OffFieldZone::Hand {
        let hand = &mut ctx.state.players[owner].hand;
        // When the replacement is not in the hand, the last card gives way to it.
        let moved = match hand.iter().position(|card| card.id == replacement_id) {
            Some(now) => Some(hand.remove(now)),
            None => {
                hand.pop();
                None
            }
        };
        let card = moved.unwrap_or_else(|| made.clone());
        let to = index.min(hand.len());
        hand.insert(to, card);
    }
    Some(find_instance(ctx.state, &replacement_id).cloned().unwrap_or(made))
}

/// `transform`'s argument: `TransformTarget & { defId; radiant?; costOverride?; chinese? }`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TransformArgs {
    #[serde(flatten)]
    pub target: TransformTarget,
    pub def_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    /// MD-B10, R921: the replacement's price where it is a price (a hand or a deck, R766).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_override: Option<i32>,
    /// ME-CN, R921: the replacement is shown in Chinese (Meditative #37).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chinese: Option<bool>,
}

/// MD-B10, R921: `costOverride` only on a card in a hand or a library (a price, which R766 takes off
/// a card in a graveyard or exile), and `chinese` wherever the replacement is.
fn apply_transform_riders(
    ctx: &mut EffectContext<'_>,
    id: &str,
    cost_override: Option<i32>,
    chinese: Option<bool>,
) {
    let Some(live) = find_instance_mut(ctx.state, id) else {
        return;
    };
    if let Some(cost_override) = cost_override
        && matches!(live.zone, Zone::Hand { .. } | Zone::Library { .. })
    {
        live.cost_override = Some(cost_override);
    }
    if chinese == Some(true) {
        live.chinese = Some(true);
    }
}

/// §6.3 Transform into `defId` (R1, R23), refused where the old card's zone cannot hold it (§5.1).
pub fn transform(args: TransformArgs) -> Effect {
    Effect::new("transform", move |ctx| {
        let Some(old) = instance_by_id_or_spec(
            ctx,
            args.target.instance_id.as_deref(),
            args.target.target.as_ref(),
        ) else {
            return;
        };
        if !transformable(ctx, &old) {
            return;
        }
        let def = def_of(Some(&*ctx.state), &args.def_id).clone();
        if let Some(replacement) = replace_card(ctx, &old, &def, args.radiant == Some(true)) {
            let id = replacement.id.clone();
            apply_transform_riders(ctx, &id, args.cost_override, args.chinese);
        }
    })
}

/// R23: Immutable blocks a Transform, which a Replace is on the field (§6.3). Off it the card is not
/// rewritten but replaced, so R35 replaces an Immutable card too and a library keeps its count (§9.1).
fn transformable(ctx: &EffectContext<'_>, old: &CardInstance) -> bool {
    !(old.zone.z() == ZoneName::Field && unit_has(ctx.state, old, KeywordKind::Immutable))
}

/// Replace `old` with a new card of `def` where it is, and report it; `None` when the zone refuses it.
fn replace_card(
    ctx: &mut EffectContext<'_>,
    old: &CardInstance,
    def: &CardDef,
    radiant: bool,
) -> Option<CardInstance> {
    let from_def_id = old.def_id.clone();
    let hidden_from = unreadable_by(ctx, old);
    let replacement = if old.zone.z() == ZoneName::Field {
        replace_on_field(ctx, old, def, radiant)
    } else {
        replace_off_field(ctx, old, def, radiant)
    }?;

    ctx.events.push(GameEvent::Transformed {
        instance_id: old.id.clone(),
        from_def_id,
        to_def_id: replacement.def_id.clone(),
        new_instance_id: replacement.id.clone(),
        hidden_from: if hidden_from.is_empty() {
            None
        } else {
            Some(hidden_from)
        },
        creator: None,
    });
    crate::generators::credit_creator(ctx, &replacement.id);
    Some(replacement)
}

/// R177: who could not read this card where it is (a library card: both, §9.1; a hand card: the
/// opponent; a face-down trap: all but its controller, R33), recorded on the event because a card
/// that ceases to exist leaves no zone to be judged by later.
fn unreadable_by(ctx: &EffectContext<'_>, card: &CardInstance) -> Vec<PlayerId> {
    match card.zone {
        Zone::Library { .. } => PLAYER_IDS.to_vec(),
        Zone::Hand { player } => vec![opponent_of(player)],
        Zone::Field {
            row: Row::Backrow, ..
        } if card.face_up != Some(true) => {
            let type_ = card_type_of(ctx.state, card);
            if type_ == CardType::Trap || type_ == CardType::FieldTrap {
                vec![opponent_of(card.controller)]
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    }
}

/// R781003: a Vanilla returns a card to the stats printed on the face it wears (§6.3): its buffs and
/// its `tuning` stats go; the rest of its tuning, a summon's X/X (`statsOverride`, not a buff) and the
/// Radiant flag stay.
pub(crate) fn to_printed_stats(card: &mut CardInstance) {
    card.buffs = AttackHealth { attack: 0, health: 0 };
    if let Some(tuning) = card.tuning.as_mut() {
        tuning.attack = None;
        tuning.health = None;
        if *tuning == Tuning::default() {
            card.tuning = None;
        }
    }
}

/// §6.3 Vanilla: its printed keywords and scripts stop applying and it returns to its printed stats
/// (R781003), keeping its damage, but a Unit the lost stats would kill stays alive at 1 health.
/// Immutable refuses it (R23); the layers read the flag (§10.4) and R78 clears it off the field.
pub fn vanilla(args: TransformTarget) -> Effect {
    Effect::new("vanilla", move |ctx| {
        let Some(card) = instance_by_id_or_spec(ctx, args.instance_id.as_deref(), args.target.as_ref())
        else {
            return;
        };
        if unit_has(ctx.state, &card, KeywordKind::Immutable) {
            return;
        }
        if card.vanilla {
            return;
        }

        // R1081000: a Unit standing in the backrow is kept alive as one in the unit row is.
        let in_units_row = crate::zones::in_units_row_or_standing(ctx.state, &card);
        let alive_before = in_units_row && unit_view(ctx.state, &card).health > 0;
        if let Some(live) = find_instance_mut(ctx.state, &card.id) {
            live.vanilla = true;
            to_printed_stats(live);
        }
        if alive_before && let Some(live) = find_instance(ctx.state, &card.id).cloned() {
            let view = unit_view(ctx.state, &live);
            if view.health <= 0
                && let Some(stored) = find_instance_mut(ctx.state, &card.id)
            {
                stored.damage = (view.max_health - 1).max(0);
            }
        }
        // Both sides of `transformed` name the same card: the client animates the text going away.
        ctx.events.push(GameEvent::Transformed {
            instance_id: card.id.clone(),
            from_def_id: card.def_id.clone(),
            to_def_id: card.def_id.clone(),
            new_instance_id: card.id.clone(),
            hidden_from: None,
            creator: None,
        });
        // R1174000: a Vanilla removes Frozen, as Hearthstone's Silence removes Freeze.
        crate::freeze::unfreeze(&mut ctx.sink, &card.id);
    })
}

// The Transform variants (docs/classic-sets.md B5 E24; §6.3 Transform, R23, R35).

/// Whether `def` could take `old`'s place: on the field one of the same row (R35; a Unit standing in a
/// backrow zone counts as a Unit, R1081000); in a graveyard or exile never a unit token (R11).
fn can_replace(state: &GameState, old: &CardInstance, def: &CardDef) -> bool {
    match old.zone {
        Zone::Field { .. } if crate::zones::stands_in_backrow(state, old) => {
            row_of(def.type_) == Some(Row::Units)
        }
        Zone::Field { row, .. } => row_of(def.type_) == Some(row),
        Zone::Graveyard { .. } | Zone::Exile { .. } => !(def.token && def.type_ == CardType::Unit),
        Zone::Hand { .. } | Zone::Library { .. } => true,
        _ => false,
    }
}

/// `"keep"`: `transformRandom`'s radiant literal for the old card's face.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum KeepFace {
    #[serde(rename = "keep")]
    Keep,
}

/// `transformRandom`'s `radiant`: `boolean | "keep"`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(untagged)]
pub enum TransformRadiant {
    Flag(bool),
    Keep(KeepFace),
}

/// `transformRandom`'s argument: `TransformTarget & { query?; radiant?; readyToAttack?;
/// scope?; same?; costOverride?; chinese?; arrives? }`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TransformRandomArgs {
    #[serde(flatten)]
    pub target: TransformTarget,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<CatalogQueryArgs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<TransformRadiant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ready_to_attack: Option<bool>,
    /// MD-B10, R921: replace the scope's cards instead of the target's (Meditative #32's hand).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<CardScope>,
    /// MD-B10, R921: one definition drawn for every card replaced (Meditative #32's "the same CN card").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub same: Option<bool>,
    /// MD-B10, R921: the replacements' price where it is a price (a hand or a deck, R766).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_override: Option<i32>,
    /// ME-CN, R921: the replacements are shown in Chinese (Meditative #37).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chinese: Option<bool>,
    /// MD-B18, R925: a replacement in a hand runs its own arrival hooks (Meditative #37).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrives: Option<bool>,
}

/// E24, Classic+ #73.1 Classic Golem (the Unit it attacks, R2285002): a §6.3 Transform into a definition
/// drawn with `ctx.rng` from `query` (§5.1's one pool, never the running card or its ingredients, B4.1),
/// narrowed to those that could take the card's place (R35), so the draw never lands on a refusal.
/// Immutable refuses it (R23); a refusal or an empty pool draws nothing (R129). `readyToAttack` lifts the
/// new body's summoning sickness (R424).
///
/// MD-B10, R921 (Meditative #32): `scope` replaces each card of the scope instead, and `same` draws one
/// definition all of them could take, each replaced in its place (R31, R671; an Immutable hand card too,
/// R35). Each replacement in a hand or a deck rolls for Glitch on its own (R673).
pub fn transform_random(args: TransformRandomArgs) -> Effect {
    Effect::new("transformRandom", move |ctx| {
        let olds: Vec<CardInstance> = match &args.scope {
            // MD-B1, R940: a harmful walk — an immune card in a tribal scope is passed by.
            Some(scope) => cards_in_card_scope(
                ctx,
                scope,
                Some(&CardScopeOptions {
                    aim: Some(crate::wire::TargetAim::Harm),
                    ..Default::default()
                }),
            )
            .into_iter()
            .map(|entry| entry.card)
            .filter(|card| transformable(ctx, card))
            .collect(),
            None => match instance_by_id_or_spec(
                ctx,
                args.target.instance_id.as_deref(),
                args.target.target.as_ref(),
            ) {
                Some(old) if transformable(ctx, &old) => vec![old],
                _ => Vec::new(),
            },
        };
        // R129: with nothing to replace, nothing is drawn.
        if olds.is_empty() {
            return;
        }
        let own: Option<String> = match &ctx.self_ {
            Some(this) => Some(this.def_id.clone()),
            None => ctx.def_id.clone(),
        };
        let asked = excluding_def_id(
            Some(&*ctx.state),
            &args.query.clone().unwrap_or_default(),
            own.as_deref(),
        );
        // With `same`, one definition every old card could take; without it, each draws its own below.
        let shared: Option<CardDef> = if args.same == Some(true) {
            let pool: Vec<&CardDef> = query(&asked)
                .into_iter()
                .filter(|def| olds.iter().all(|old| can_replace(&*ctx.state, old, def)))
                .collect();
            pick_generated(ctx.sink.rng, &pool, None).cloned()
        } else {
            None
        };
        if args.same == Some(true) && shared.is_none() {
            return;
        }
        for old in &olds {
            // R673: a card transformed in a hand or a deck is generated there and may be Glitch; one on
            // the field may not, since Glitch is only ever played.
            let held = matches!(old.zone, Zone::Hand { .. } | Zone::Library { .. });
            let def: Option<CardDef> = match &shared {
                Some(shared) if held => Some(glitch_or_not(ctx.sink.rng, shared, &*ctx.sink.state).clone()),
                Some(shared) => Some(shared.clone()),
                None => {
                    let pool: Vec<&CardDef> = query(&asked)
                        .into_iter()
                        .filter(|def| can_replace(&*ctx.state, old, def))
                        .collect();
                    let glitch: Option<&GameState> = if held { Some(&*ctx.sink.state) } else { None };
                    pick_generated(ctx.sink.rng, &pool, glitch).cloned()
                }
            };
            let Some(def) = def else {
                continue;
            };
            let radiant = match args.radiant {
                Some(TransformRadiant::Keep(_)) => old.radiant,
                Some(TransformRadiant::Flag(flag)) => flag,
                None => false,
            };
            let replacement = replace_card(ctx, old, &def, radiant);
            let Some(replacement) = replacement else {
                continue;
            };
            if args.ready_to_attack == Some(true)
                && replacement.zone.z() == ZoneName::Field
                && let Some(live) = find_instance_mut(ctx.state, &replacement.id)
            {
                live.summoned_turn = None;
            }
            let id = replacement.id.clone();
            apply_transform_riders(ctx, &id, args.cost_override, args.chinese);
            // MD-B18, R925: opt in, and only for a replacement in a hand (#37's new card arrives).
            if args.arrives == Some(true)
                && let Some(live) = find_instance(ctx.state, &id).cloned()
                && matches!(live.zone, Zone::Hand { .. })
            {
                crate::draw::run_arrival_hooks(ctx, &live);
            }
        }
    })
}

/// `transformBeneath`'s argument (TS default `{}`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TransformBeneathArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<TargetSpec>,
}

/// E24, Classic+ #4 Juhan Biggest Bat: "the cards beneath it become copies of this". Each card dormant
/// beneath the top of the pile `of` names (default the running card, which must be that top) is Replaced
/// (§6.3) by R57's copy of the top, keeping its owner, controller, position and place in the pile, and
/// stays dormant (R13). An Immutable card stays as it is (R23); each replaced one ceases to exist (R35).
pub fn transform_beneath(args: TransformBeneathArgs) -> Effect {
    Effect::new("transformBeneath", move |ctx| {
        let spec = args.of.clone().unwrap_or(TargetSpec::SelfCard);
        let Some(DamageTarget::Unit { instance: top }) = resolve_target(ctx, &spec) else {
            return;
        };
        let Some(at) = slot_of(ctx.state, &top) else {
            return;
        };
        if at.row != Row::Units {
            return;
        }
        let Some(pile) = pile_at(ctx.state, at).cloned() else {
            return;
        };
        if pile.first().map(|card| card.id.as_str()) != Some(top.id.as_str()) {
            return;
        }
        for old in pile.iter().skip(1) {
            // R1839003: a Dormant card beneath the pile is reached by nothing, this Transform included.
            if unit_has(ctx.state, old, KeywordKind::Immutable) || crate::slumber::is_slumbering(old) {
                continue;
            }
            let mut copy = clone_of(ctx, &top, old.owner, &json_as(json!({ "of": { "of": "self" } })));
            if old.position.is_some() {
                copy.position = old.position;
            }
            // A new body in the pile, like any Transform result (§4.1).
            copy.summoned_turn = Some(ctx.state.turn);
            let hidden_from = unreadable_by(ctx, old);
            let copy_id = copy.id.clone();
            let copy_def_id = copy.def_id.clone();
            if !replace_in_zone(&mut *ctx.state, old, &mut copy) {
                continue;
            }
            if let Some(live) = find_instance_mut(ctx.state, &copy_id) {
                live.controller = old.controller;
            }
            let mut retired = old.clone();
            cease_to_exist(&mut *ctx.state, &mut retired);
            ctx.events.push(GameEvent::Transformed {
                instance_id: old.id.clone(),
                from_def_id: old.def_id.clone(),
                to_def_id: copy_def_id,
                new_instance_id: copy_id.clone(),
                hidden_from: if hidden_from.is_empty() {
                    None
                } else {
                    Some(hidden_from)
                },
                creator: None,
            });
            crate::generators::credit_creator(ctx, &copy_id);
        }
    })
}

/// `swapBook`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SwapBookArgs {
    pub instance_id: String,
    pub from: String,
}

/// Classic #55 Book of Wildfire, R671: "Becomes a different Book at the end of your turn". The hand card
/// `instanceId` names is Replaced (§6.3, R35) by a Book from §5.1's pool, any non-token Book (R380) but
/// its own and `from`'s, keeping its face and hand place and carrying `swapsBook` on. A Temporary card's
/// Book stays Temporary with its count (R637, R2179001): the swap comes before cleanup.
pub fn swap_book(args: SwapBookArgs) -> Effect {
    Effect::new("swapBook", move |ctx| {
        let Some(old) = instance_on_its_stay(ctx, &args.instance_id) else {
            return;
        };
        if old.zone.z() != ZoneName::Hand {
            return;
        }
        let books: CatalogQueryArgs = json_as(json!({ "tags": ["Book"] }));
        let pool: Vec<&CardDef> = query(&books)
            .into_iter()
            .filter(|def| def.id != old.def_id && def.id != args.from)
            .collect();
        let Some(def) = pick_generated(ctx.sink.rng, &pool, None) else {
            return;
        };
        let temporary = is_temporary_card(ctx.state, &old)
            .then(|| crate::temporary::temporary_number(&crate::layers::card_keywords(ctx.state, &old)))
            .flatten();
        let Some(replacement) = replace_card(ctx, &old, def, old.radiant) else {
            return;
        };
        if let Some(live) = find_instance_mut(ctx.state, &replacement.id) {
            let _ = add_enchantment(
                live,
                &Enchantment::SwapsBook {
                    from: args.from.clone(),
                },
            );
            if let Some(count) = temporary {
                live.granted_keywords.push(Keyword::Temporary {
                    n: (count > 1).then_some(count),
                });
            }
        }
    })
}

/// R671: the card that started a card's Book swap, or `None` when it carries none.
pub fn book_swap_source_of(card: &CardInstance) -> Option<String> {
    card.enchantments.iter().flatten().find_map(|entry| match entry {
        Enchantment::SwapsBook { from } => Some(from.clone()),
        _ => None,
    })
}
