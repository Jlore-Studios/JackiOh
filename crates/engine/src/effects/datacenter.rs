//! The AI generated cards' verbs (SPEC §8.7 rows T-AI-4 and T-AI-6): a draw that repeats while the card
//! it brought is cheap (T-AI-4 Chain of Thought), and a sweep of Field Spells that hits the heroes once
//! per Field Spell it dooms (T-AI-6 Datacenter Fire).

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

// T-AI-4 Chain of Thought

/// `drawWhileCheap`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DrawWhileCheapArgs {
    pub max_cost: i32,
    pub repeats: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerId>,
}

/// T-AI-4: each round one §2.4 draw, "it" the card THAT draw put in hand (R596), priced as it arrives
/// (R65). A cast on draw (R58), a burn, a fatigue or limited draw and the game's end (R216) end the
/// chain; a cast that asks (R158) parks its own remainder (R113).
pub fn draw_while_cheap(args: DrawWhileCheapArgs) -> Effect {
    Effect::new("drawWhileCheap", move |ctx| {
        let player = args.player.unwrap_or(ctx.controller);
        for _round in 0..=args.repeats {
            if ctx.sink.state.result.is_some() {
                return;
            }
            // A prompt's id names it alone.
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

// T-AI-6 Datacenter Fire

/// Whose Field Spells a sweep takes, relative to the card running it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum FieldSpellSide {
    Any,
    Enemy,
}

/// The card a doomed count runs for (a Spell's effects pass a Spell-immune card by).
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

/// E35 (`restrictions.effectIsFromSpell`, off a sweep reader): whether the running list is a Spell's;
/// once its card has gone, by the script it names (`defId`) on the face it recorded.
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

/// E35 (`restrictions.unaffectedBy`, off a sweep reader).
fn reader_unaffected_by(reader: &SweepReader<'_>, card: &CardInstance) -> bool {
    card.zone.z() == ZoneName::Field && reader_is_spell(reader) && immune_to_spells(reader.state, card)
}

/// T-AI-6: the Field Spells "destroy all (enemy) Field Spells" would destroy now: each backrow zone's
/// acting card (an Ivory Tower beneath its Unit counts, R418; a dormant one does not, §3.2), except
/// Indestructible (R46) or unaffected (B5 E35). An animated one in a unit zone is a Unit (R383, R588).
/// A pure read, so `preview` (R280) and resolution agree.
pub fn field_spells_doomed(reader: SweepReader<'_>, side: FieldSpellSide) -> Vec<CardInstance> {
    let sides: Vec<PlayerId> = if side == FieldSpellSide::Enemy {
        vec![opponent_of(reader.controller)]
    } else {
        PLAYER_IDS.to_vec()
    };
    let mut out: Vec<CardInstance> = Vec::new();
    for player in sides {
        for slot in slots_of(player, Row::Backrow) {
            let Some(card) = card_at(reader.state, slot) else {
                continue;
            };
            if card_type_of(reader.state, card) != CardType::FieldSpell {
                continue;
            }
            if unit_has(reader.state, card, KeywordKind::Indestructible)
                || reader_unaffected_by(&reader, card)
            {
                continue;
            }
            out.push(card.clone());
        }
    }
    out
}

/// `destroyFieldSpellsAndHit`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DestroyFieldSpellsAndHitArgs {
    pub side: FieldSpellSide,
    pub damage_per: i32,
}

/// T-AI-6 Datacenter Fire: every Field Spell of the sides is marked destroyed (§6.3; §4.5's check
/// collects them, R59); the count doomed as the sweep lands (as C+ #12.6's is, R408) sets one §4.4
/// instance per hero of `damagePer` times it (so Spell Damage applies once), in R68's order.
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
