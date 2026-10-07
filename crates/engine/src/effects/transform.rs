//! Transform and Vanilla (SPEC §6.3): the two verbs that rewrite what a card is, so Immutable
//! refuses both (R23). A Transform puts a new instance of another definition in the same zone and
//! position, with no Cry (R1); the card it replaces ceases to exist rather than reaching a graveyard
//! (R35). Vanilla is a flag the layers read (§10.4): stats, buffs and damage are other layers and
//! must survive it, and the definition is shared by every copy of the card, so it is never edited.
//!
//! Port of `packages/engine/src/effects/transform.ts`. TS wrote through the live instances it held;
//! here every write finds the card again by id (`state::find_instance_mut`), and the old card a
//! Replace retires is handed to `zones::cease_to_exist` as the copy this file read, which is what TS
//! did too: the old object had already left its pile.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::catalog::{CatalogQueryArgs, def_of, excluding_def_id, pick_generated, query};
use crate::damage::DamageTarget;
use crate::enchantments::add_enchantment;
use crate::faces::card_type_of;
use crate::layers::unit_has;
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut, new_instance};
use crate::temporary::is_temporary_card;
use crate::wire::{
    CardDef, CardType, Enchantment, GameEvent, Keyword, KeywordKind, PLAYER_IDS, PlayerId, Row, Zone, ZoneName,
    opponent_of,
};
use crate::zones::{
    MoveToZoneOptions, LibraryPosition, OffFieldZone, cease_to_exist, is_carried, move_to_zone, pile_at,
    replace_in_zone, slot_of, zone_of,
};

use super::summon::clone_of;
use super::targets::{TargetSpec, instance_on_its_stay, resolve_target};

/// Which card to rewrite: the pick the play or a prompt carried (R81), or an instance id a trigger
/// read off the event it answers — Sheepish transforms the unit the opponent just played (#41).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TransformTarget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
}

fn instance_of(ctx: &EffectContext<'_>, args: &TransformTarget) -> Option<CardInstance> {
    // R174: a card named by id is aimed at the stay it had when the run began (`instance_on_its_stay`).
    if let Some(instance_id) = &args.instance_id {
        return instance_on_its_stay(ctx, instance_id);
    }
    let spec = args.target.clone().unwrap_or(TargetSpec::Chosen { index: None });
    match resolve_target(ctx, &spec)? {
        DamageTarget::Unit { instance } => Some(instance),
        _ => None,
    }
}

/// §5.1: the row a type lives in; a Spell is never a permanent, so it can never replace one.
fn row_for(type_: CardType) -> Option<Row> {
    match type_ {
        CardType::Unit => Some(Row::Units),
        CardType::Spell => None,
        _ => Some(Row::Backrow),
    }
}

/// §6.3 Replace: the same zone and position, with the old card's owner and controller. The new card
/// takes the old one's place (`zones::replace_in_zone`) rather than being summoned into an emptied zone:
/// a Transform result is no summon (§6.2), so the Lock §3.2 puts on a zone — "accepts no summons … the
/// current occupant is unaffected" — does not refuse it, and neither does a reservation (R64). #36
/// Magic Jammed locks the zone of a Heroic Power it could not destroy (R46), and radiant #36 locks
/// the zone of a card it could not steal (R15); R35 still replaces either. A Stack pile keeps its
/// dormant cards beneath the replacement (§3.2).
fn replace_on_field(
    ctx: &mut EffectContext<'_>,
    old: &CardInstance,
    def: &CardDef,
    radiant: bool,
) -> Option<CardInstance> {
    let at = slot_of(ctx.state, old)?;
    // R446: a Unit a carrier holds stands in a backrow zone as a Unit, and a Unit may take its place there.
    let row_here = if is_carried(ctx.state, old) { Row::Units } else { at.row };
    if row_for(def.type_) != Some(row_here) {
        return None;
    }

    let turn = ctx.state.turn;
    let mut replacement = new_instance(&mut *ctx.state, &def.id, old.owner, zone_of(&at));
    replacement.radiant = radiant;
    if old.position.is_some() {
        replacement.position = old.position;
    }
    // R659: a new body enters the field this turn, so it is summoning sick like a summoned card (§4.1);
    // only text that says otherwise lifts it (R424).
    replacement.summoned_turn = Some(turn);
    let replacement_id = replacement.id.clone();

    if !replace_in_zone(&mut *ctx.state, old, replacement) {
        return None;
    }
    // The replaced card ceases to exist: no graveyard, no exile pile, no Death trigger (§6.3, R35) —
    // and it has left the field, which R174 counts like any departure (`zones::cease_to_exist`).
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

/// `player`'s pile of an off-field zone.
fn off_field_pile(ctx: &EffectContext<'_>, player: PlayerId, at: OffFieldZone) -> Vec<CardInstance> {
    let side = &ctx.state.players[player];
    match at {
        OffFieldZone::Hand => side.hand.clone(),
        OffFieldZone::Library => side.library.clone(),
        OffFieldZone::Graveyard => side.graveyard.clone(),
        OffFieldZone::Exile => side.exile.clone(),
    }
}

/// R35's other zones: Transmogulate replaces hand, library, graveyard and exile cards too, "same
/// counts" per zone. A library and a hand keep their order, so the replacement takes the old card's
/// index there; `move_to_zone` appends in the other piles.
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
    // R11: a unit-token card cannot sit in a graveyard or exile, so a replacement that would cease
    // to exist on arrival is refused instead of thinning the zone.
    if matches!(at, OffFieldZone::Graveyard | OffFieldZone::Exile) && def.token && def.type_ == CardType::Unit {
        return None;
    }

    let owner = old.owner;
    let index = off_field_pile(ctx, owner, at).iter().position(|card| card.id == old.id)?;

    let mut retired = old.clone();
    cease_to_exist(&mut *ctx.state, &mut retired);
    // R312: a new instance with no `knownAs`, so a library replacement is a card its owner was never
    // shown, and their list counts it unknown (the `transformed` event names it to nobody, R177).
    let zone = match at {
        OffFieldZone::Hand => Zone::Hand { player: owner },
        OffFieldZone::Library => Zone::Library { player: owner },
        OffFieldZone::Graveyard => Zone::Graveyard { player: owner },
        OffFieldZone::Exile => Zone::Exile { player: owner },
    };
    let mut replacement = new_instance(&mut *ctx.state, &def.id, owner, zone);
    replacement.radiant = radiant;
    let replacement_id = replacement.id.clone();
    let made = replacement.clone();
    let _ = move_to_zone(
        &mut *ctx.state,
        replacement,
        at,
        MoveToZoneOptions {
            position: Some(LibraryPosition::At(index as i32)),
            ..MoveToZoneOptions::default()
        },
    );
    // R671: a hand keeps its order too, so a card swapped in hand stays where its owner put it.
    if at == OffFieldZone::Hand {
        let hand = &mut ctx.state.players[owner].hand;
        // TS: `hand.splice(hand.indexOf(replacement), 1); hand.splice(index, 0, replacement)`, and a
        // `splice(-1, 1)` takes the last card when the replacement is not there.
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

/// `transform`'s argument: `TransformTarget & { defId; radiant? }`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransformArgs {
    #[serde(flatten)]
    pub target: TransformTarget,
    pub def_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
}

/// §6.3 Transform: a new instance of `defId` where the old card was, no Cry (R1), and the old card
/// ceases to exist. Immutable refuses it (R23), and so does a definition that cannot live in the
/// zone the old card occupies (§5.1).
pub fn transform(args: TransformArgs) -> Effect {
    Effect::new("transform", move |ctx| {
        let Some(old) = instance_of(ctx, &args.target) else {
            return;
        };
        if !transformable(ctx, &old) {
            return;
        }
        let def = def_of(ctx.state, &args.def_id).clone();
        let _ = replace_card(ctx, &old, &def, args.radiant == Some(true));
    })
}

/// R23: Immutable blocks a Transform, which is what a Replace is on the field (§6.3). Off the field a
/// Replace is no Transform — the card is not rewritten, it ceases to exist and another takes its
/// place — so R35's "other zones: any card from the pool, same counts" replaces an Immutable card too,
/// and a library keeps its count whatever it held (§9.1).
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
        hidden_from: if hidden_from.is_empty() { None } else { Some(hidden_from) },
    });
    Some(replacement)
}

/// R177: who could not read this card where it is, read before it ceases to exist there — a library
/// card is hidden from both players (§9.1), a hand card from the other one, and a face-down trap from
/// everyone but its controller (R33). A card that ceases to exist leaves no zone of its own to be
/// judged by later, so the event records this for the view.
fn unreadable_by(ctx: &EffectContext<'_>, card: &CardInstance) -> Vec<PlayerId> {
    match card.zone {
        Zone::Library { .. } => PLAYER_IDS.to_vec(),
        Zone::Hand { player } => vec![opponent_of(player)],
        Zone::Field { row: Row::Backrow, .. } if card.face_up != Some(true) => {
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

/// §6.3 Vanilla: the card's text stops applying — its printed keywords and its scripts — while its
/// stats, buffs and damage stay. Immutable refuses it (R23), and a card that is already Vanilla is
/// unchanged. The flag lives on the instance; the layers read it (§10.4) and R78 clears it when the
/// card leaves the field. (TS default argument `{}`.)
pub fn vanilla(args: TransformTarget) -> Effect {
    Effect::new("vanilla", move |ctx| {
        let Some(card) = instance_of(ctx, &args) else {
            return;
        };
        if unit_has(ctx.state, &card, KeywordKind::Immutable) {
            return;
        }
        if card.vanilla {
            return;
        }

        if let Some(live) = find_instance_mut(ctx.state, &card.id) {
            live.vanilla = true;
        }
        // No instance is created and no definition changes, so both sides of `transformed` name the
        // same card and the same def: the event the client animates is the text going away.
        ctx.events.push(GameEvent::Transformed {
            instance_id: card.id.clone(),
            from_def_id: card.def_id.clone(),
            to_def_id: card.def_id.clone(),
            new_instance_id: card.id.clone(),
            hidden_from: None,
        });
    })
}

// ---------------------------------------------------------------------------------------------
// Patch v0.2.0: the Transform variants (docs/classic-sets.md B5 E24; §6.3 Transform, R23, R35).
// ---------------------------------------------------------------------------------------------

/// Whether `def` could take `old`'s place: on the field only a definition of the same row, as R35
/// keeps a board card's type (a Unit for a Unit); in a graveyard or exile never a unit token (R11).
fn can_replace(old: &CardInstance, def: &CardDef) -> bool {
    match old.zone {
        Zone::Field { row, .. } => row_for(def.type_) == Some(row),
        Zone::Graveyard { .. } | Zone::Exile { .. } => !(def.token && def.type_ == CardType::Unit),
        Zone::Hand { .. } | Zone::Library { .. } => true,
        _ => false,
    }
}

/// `"keep"`: `transformRandom`'s radiant literal for the old card's face.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeepFace {
    #[serde(rename = "keep")]
    Keep,
}

/// `transformRandom`'s `radiant`: `boolean | "keep"`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum TransformRadiant {
    Flag(bool),
    Keep(KeepFace),
}

/// `transformRandom`'s argument: `TransformTarget & { query?; radiant?; readyToAttack? }`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
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
}

/// E24, Classic+ #73.1 Classic Golem: "it transforms into a random Classic or Classic+ Unit". A §6.3
/// Transform into a definition drawn with `ctx.rng` from `query` (§5.1's one pool, never the running
/// card or its ingredients, B4.1), narrowed to the definitions that could take the card's place — on
/// the field, one of its row (R35) — so the draw never lands on one the zone would refuse. Immutable
/// refuses it (R23), and a refusal or an empty pool draws nothing (R129). `radiant` is the new card's
/// face: `true`, `false` (the default), or `"keep"` the old card's. `readyToAttack` is Classic Golem's
/// "the new Unit may attack again this turn" (R424): the new body on the field is not summoning sick
/// this turn (a new instance's exertion is already fresh).
pub fn transform_random(args: TransformRandomArgs) -> Effect {
    Effect::new("transformRandom", move |ctx| {
        let Some(old) = instance_of(ctx, &args.target) else {
            return;
        };
        if !transformable(ctx, &old) {
            return;
        }
        let own: Option<String> = match &ctx.self_ {
            Some(this) => Some(this.def_id.clone()),
            None => ctx.def_id.clone(),
        };
        let asked = excluding_def_id(Some(&*ctx.state), &args.query.clone().unwrap_or_default(), own.as_deref());
        let pool: Vec<CardDef> = query(&asked)
            .into_iter()
            .filter(|def| can_replace(&old, def))
            .cloned()
            .collect();
        // R673: a card transformed in a hand or a deck is generated there and may be Glitch; one on the
        // field may not, since Glitch is only ever played.
        let held = matches!(old.zone, Zone::Hand { .. } | Zone::Library { .. });
        let glitch: Option<&GameState> = if held { Some(&*ctx.sink.state) } else { None };
        let Some(def) = pick_generated(ctx.sink.rng, &pool, glitch) else {
            return;
        };
        let radiant = match args.radiant {
            Some(TransformRadiant::Keep(_)) => old.radiant,
            Some(TransformRadiant::Flag(flag)) => flag,
            None => false,
        };
        let replacement = replace_card(ctx, &old, &def, radiant);
        if let Some(replacement) = replacement
            && args.ready_to_attack == Some(true)
            && replacement.zone.z() == ZoneName::Field
            && let Some(live) = find_instance_mut(ctx.state, &replacement.id)
        {
            live.summoned_turn = None;
        }
    })
}

/// `transformBeneath`'s argument (TS default `{}`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TransformBeneathArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<TargetSpec>,
}

/// E24, Classic+ #4 Juhan Biggest Bat: "the cards beneath it become copies of this". Every card dormant
/// beneath the top of the unit pile `of` names (default the running card, which must be that top) is
/// Replaced (§6.3) by a copy of the top — R57's copy, with the top's face, buffs, granted keywords,
/// Vanilla state and X/X — that keeps the old card's owner and controller, its position and its place
/// in the pile, and stays dormant (R13): when the top leaves, the next copy resumes. An Immutable card
/// stays as it is (R23). Each replaced card ceases to exist (R35) with a `transformed` event.
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
        let Some(pile) = pile_at(ctx.state, &at).cloned() else {
            return;
        };
        if pile.first().map(|card| card.id.as_str()) != Some(top.id.as_str()) {
            return;
        }
        for old in pile.iter().skip(1) {
            if unit_has(ctx.state, old, KeywordKind::Immutable) {
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
            if !replace_in_zone(&mut *ctx.state, old, copy) {
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
                new_instance_id: copy_id,
                hidden_from: if hidden_from.is_empty() { None } else { Some(hidden_from) },
            });
        }
    })
}

/// `swapBook`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SwapBookArgs {
    pub instance_id: String,
    pub from: String,
}

/// Classic #55 Book of Wildfire, R671: "Becomes a different Book at the end of your turn" — the card
/// `instanceId` names, while it is in a hand, is Replaced (§6.3, R35) by a Book drawn with `ctx.rng`
/// from §5.1's pool: every non-token Book of every set (R380) but the one it is now and the card that
/// started the swap (`from`, Wildfire itself). The new Book keeps the old card's face (a Radiant card
/// becomes the Radiant face of the Book) and its place in the hand, and carries the swap on as the
/// `swapsBook` enchantment, so it changes again at its owner's next end of turn. It is otherwise a new
/// card, as any Transform result is (its cost changes and its other enchantments stay behind), except
/// that a Temporary card's new Book is Temporary too (R637): the swap comes before cleanup, and a
/// Temporary card must not leave the turn as a card that stays.
pub fn swap_book(args: SwapBookArgs) -> Effect {
    Effect::new("swapBook", move |ctx| {
        let Some(old) = instance_on_its_stay(ctx, &args.instance_id) else {
            return;
        };
        if old.zone.z() != ZoneName::Hand {
            return;
        }
        let books: CatalogQueryArgs = json_as(json!({ "tags": ["Book"] }));
        let pool: Vec<CardDef> = query(&books)
            .into_iter()
            .filter(|def| def.id != old.def_id && def.id != args.from)
            .cloned()
            .collect();
        let Some(def) = pick_generated(ctx.sink.rng, &pool, None) else {
            return;
        };
        let temporary = is_temporary_card(ctx.state, &old);
        let Some(replacement) = replace_card(ctx, &old, &def, old.radiant) else {
            return;
        };
        if let Some(live) = find_instance_mut(ctx.state, &replacement.id) {
            let _ = add_enchantment(
                live,
                Enchantment::SwapsBook {
                    from: args.from.clone(),
                },
            );
            if temporary {
                live.granted_keywords.push(Keyword::Temporary);
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
