//! The AI generated cards' verbs (SPEC §8.7 rows T-AI-4 and T-AI-6, the cards-plus-d workstream): a draw
//! that repeats while the card it brought is cheap (T-AI-4 Chain of Thought), and a sweep of Field Spells
//! that hits the heroes once per Field Spell it dooms (T-AI-6 Datacenter Fire).
//!
//! Each is card-specific — no Core card and no generic B5 system asks for either — so they live here,
//! beside the effects library they are written in.
//!
//! Port of `packages/engine/src/effects/datacenter.ts`.

use serde::{Deserialize, Serialize};

use crate::catalog::def_of;
use crate::draw::draw_one;
use crate::effects::damage::{DamageEffectArgs, DamageFlagArgs, damage};
use crate::effects::destroy::destroy_all;
use crate::effects::fruit::card_this_draw_put_in_hand;
use crate::effects::targets::{BoardScope, ScopeSide, TargetSpec, sides_of};
use crate::faces::card_type_of;
use crate::layers::unit_has;
use crate::mana::effective_cost;
use crate::restrictions::{immune_to_spells, is_spell_source};
use crate::script::{ConditionContext, Effect, EffectContext};
use crate::state::{CardInstance, GameState};
use crate::wire::{CardType, KeywordKind, PLAYER_IDS, PlayerId, Row, ZoneName, opponent_of};
use crate::zones::{card_at, slots_of};

// ---------------------------------------------------------------------------------------------
// T-AI-4 Chain of Thought
// ---------------------------------------------------------------------------------------------

/// `drawWhileCheap`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DrawWhileCheapArgs {
    pub max_cost: i32,
    pub repeats: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerId>,
}

/// T-AI-4: "Draw 1. If it costs (`maxCost`) or less, repeat this, up to `repeats` more times." Each
/// round is one §2.4 draw; "it" is the card THAT draw put in the hand (`cardThisDrawPutInHand`, R596),
/// priced as it arrives (R65's current cost: an X-cost card reads 0). A card cast on draw never gets
/// there (R58) — not even the card the cast's own repeat then brings (R596) — a burned one isn't
/// there, and a fatigue or limited draw brings none, so each ends the chain; so does the end of the game
/// (R216). A draw can pause only on a cast-on-draw card that asks (R158), whose draw has then ended the
/// chain already: its cast parks its own remainder (R113), and nothing of this chain is left to owe.
pub fn draw_while_cheap(args: DrawWhileCheapArgs) -> Effect {
    Effect::new("drawWhileCheap", move |ctx| {
        let player = args.player.unwrap_or(ctx.controller);
        for _round in 0..=args.repeats {
            if ctx.sink.state.result.is_some() {
                return;
            }
            // TS compared the prompt objects; a prompt's id names it alone.
            let before = ctx.sink.state.pending.as_ref().map(|pending| pending.id.clone());
            let from = ctx.sink.events.len();
            let outcome = draw_one(ctx, player, None);
            let card = card_this_draw_put_in_hand(ctx, player, from, outcome);
            let now = ctx.sink.state.pending.as_ref().map(|pending| pending.id.clone());
            let Some(card) = card else {
                return;
            };
            if now != before {
                return;
            }
            if effective_cost(ctx.sink.state, &card, Default::default()) > args.max_cost {
                return;
            }
        }
    })
}

// ---------------------------------------------------------------------------------------------
// T-AI-6 Datacenter Fire
// ---------------------------------------------------------------------------------------------

/// Whose Field Spells a sweep takes, relative to the card running it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum FieldSpellSide {
    Any,
    Enemy,
}

/// The reader a doomed count needs: the card running it (a Spell's effects pass a Spell-immune card by).
/// (TS `Pick<EffectContext, "state" | "self" | "defId" | "radiant" | "controller">`, which a running
/// effect and a card's `preview` both hand over.)
#[derive(Clone, Copy)]
pub struct SweepReader<'a> {
    pub state: &'a GameState,
    pub self_: Option<&'a CardInstance>,
    pub def_id: Option<&'a str>,
    pub radiant: bool,
    pub controller: PlayerId,
}

impl<'a> SweepReader<'a> {
    /// The reader a running effect is.
    pub fn of_context(ctx: &'a EffectContext<'_>) -> SweepReader<'a> {
        SweepReader {
            state: &*ctx.sink.state,
            self_: ctx.self_.as_ref(),
            def_id: ctx.def_id.as_deref(),
            radiant: ctx.radiant,
            controller: ctx.controller,
        }
    }

    /// The reader a card's `preview` or `conditionMet` is (R280, R195).
    pub fn of_condition(ctx: ConditionContext<'a>) -> SweepReader<'a> {
        SweepReader {
            state: ctx.state,
            self_: Some(ctx.self_),
            def_id: None,
            radiant: ctx.radiant,
            controller: ctx.controller,
        }
    }
}

/// E35 (`restrictions.effectIsFromSpell`, read off a sweep reader): whether the list running is a
/// Spell's. Its card says so while it is there; a continuation re-entered once the card has gone still
/// names the script it runs (`defId`), on the face it recorded.
fn reader_is_spell(reader: &SweepReader<'_>) -> bool {
    if let Some(own) = reader.self_ {
        return is_spell_source(reader.state, Some(own));
    }
    let Some(def_id) = reader.def_id.filter(|id| !id.is_empty()) else {
        return false;
    };
    let def = def_of(Some(reader.state), def_id);
    let face = if reader.radiant { &def.radiant } else { &def.base };
    face.type_.unwrap_or(def.type_) == CardType::Spell
}

/// E35 (`restrictions.unaffectedBy`, read off a sweep reader): a Spell's effects pass an immune card on
/// the field by.
fn reader_unaffected_by(reader: &SweepReader<'_>, card: &CardInstance) -> bool {
    card.zone.z() == ZoneName::Field && reader_is_spell(reader) && immune_to_spells(reader.state, card)
}

/// T-AI-6: the Field Spells "destroy all (enemy) Field Spells" would destroy now — every Field Spell in a
/// backrow zone of those sides (the acting card of each zone, so an Ivory Tower beneath the Unit it
/// holds counts, R418, and a card dormant under a backrow pile does not, §3.2), except an Indestructible
/// one (R46) and one unaffected by the running Spell (B5 E35). Traps and Field Traps are no Field
/// Spells, and an animated one standing in a unit zone is a Unit there (R383), so not one (R588). A pure read, so a card's
/// `preview` (R280) and its resolution count the same cards.
pub fn field_spells_doomed(reader: SweepReader<'_>, side: FieldSpellSide) -> Vec<CardInstance> {
    let sides: Vec<PlayerId> = if side == FieldSpellSide::Enemy {
        vec![opponent_of(reader.controller)]
    } else {
        PLAYER_IDS.to_vec()
    };
    let mut out: Vec<CardInstance> = Vec::new();
    for player in sides {
        for slot in slots_of(player, Row::Backrow) {
            let Some(card) = card_at(reader.state, &slot) else {
                continue;
            };
            if card_type_of(reader.state, &card) != CardType::FieldSpell {
                continue;
            }
            if unit_has(reader.state, &card, KeywordKind::Indestructible) || reader_unaffected_by(&reader, &card) {
                continue;
            }
            out.push(card.clone());
        }
    }
    out
}

/// `destroyFieldSpellsAndHit`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DestroyFieldSpellsAndHitArgs {
    pub side: FieldSpellSide,
    pub damage_per: i32,
}

/// T-AI-6 Datacenter Fire: "Destroy all Field Spells. Deal `damagePer` damage to each hero for each one
/// destroyed" (Radiant: the enemy's Field Spells, and the enemy hero). Every Field Spell of those sides is
/// marked destroyed (§6.3 Destroy over a backrow scope, so §4.5's check collects them together, R59, and
/// fires their Death), and the ones the mark will take (`fieldSpellsDoomed`, read as the sweep lands, as
/// C+ #12.6's "each one destroyed" is, R408) set the hit: one §4.4 instance per hero of `damagePer` times
/// that count, from the running Spell (so Spell Damage raises it once and a per-hit cap caps it once), in
/// R68's order. None doomed, no damage.
pub fn destroy_field_spells_and_hit(args: DestroyFieldSpellsAndHitArgs) -> Effect {
    Effect::new("destroyFieldSpellsAndHit", move |ctx| {
        let doomed = field_spells_doomed(SweepReader::of_context(ctx), args.side).len() as i32;
        let scope = BoardScope {
            side: Some(match args.side {
                FieldSpellSide::Any => ScopeSide::Any,
                FieldSpellSide::Enemy => ScopeSide::Enemy,
            }),
            rows: Some(vec![Row::Backrow]),
            types: Some(vec![CardType::FieldSpell]),
            ..BoardScope::default()
        };
        (destroy_all(scope).apply)(ctx);
        if doomed == 0 {
            return;
        }
        let players: Vec<PlayerId> = if args.side == FieldSpellSide::Enemy {
            vec![opponent_of(ctx.controller)]
        } else {
            sides_of(ctx, Some(ScopeSide::Any))
        };
        for player in players {
            if ctx.sink.state.result.is_some() {
                return;
            }
            let to = if player == ctx.controller {
                TargetSpec::SelfHero
            } else {
                TargetSpec::EnemyHero
            };
            let hit = damage(DamageEffectArgs {
                to,
                amount: doomed * args.damage_per,
                flags: DamageFlagArgs::default(),
            });
            (hit.apply)(ctx);
        }
    })
}
