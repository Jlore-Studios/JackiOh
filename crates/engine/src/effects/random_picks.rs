//! Random picks of existing cards (Classic #90 In Too Deep's rewards, SPEC §8.6 row 90; R60). A hook
//! may not roll dice, as `ctx.rng` advances `rngCursor`, which is state, so the pick happens here.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::draw::add_to_hand as put_in_hand;
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext};
use crate::state::CardInstance;
use crate::zones::active_units_of;

use super::buff::buff;
use super::targets::{BoardScope, PlayerSpec, ScopeSide, cards_in_scope, player_of};
use crate::wire::Row;

/// The pool every "random enemy permanent" draws from (#4763): `ctx.controller`'s opponent's
/// permanents through `cards_in_scope`, in R68 order: each unit pile's top (a card under it is not on
/// the field, R13), carried and standing Units (R446, R1081000), then each backrow card, face-down
/// ones included. A Dormant card is passed by (R4322000), as is a Unit immune to a Spell's effects
/// when a Spell draws (R4424000), and a card across a slash from the source (R3554003). It only reads:
/// the caller draws from it inside its own effect, distinct cards (R60), and an empty pool draws
/// nothing (R129).
pub fn enemy_permanent_pool(ctx: &EffectContext<'_>) -> Vec<CardInstance> {
    let scope = BoardScope {
        side: Some(ScopeSide::Enemy),
        rows: Some(vec![Row::Units, Row::Backrow]),
        ..BoardScope::default()
    };
    cards_in_scope(ctx, &scope)
}

/// `buffRandomUnit`'s argument: `{ player? } & BuffAmount`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct BuffRandomUnitArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<i32>,
}

/// "A random Unit of yours gets +X/+Y": one Unit acting on that side (R13) gets a layer-4 buff
/// (§10.4); no Unit draws nothing (R129).
pub fn buff_random_unit(args: BuffRandomUnitArgs) -> Effect {
    Effect::new("buffRandomUnit", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let units: Vec<CardInstance> = active_units_of(ctx.state, player).into_iter().cloned().collect();
        if units.is_empty() {
            return;
        }
        let Some(unit) = ctx.rng.pick(&units).cloned() else {
            return;
        };
        // `buff`'s argument, with only the halves this call was given.
        let mut literal = Map::new();
        literal.insert(
            "target".into(),
            json!({ "of": "instance", "instanceId": unit.id }),
        );
        if let Some(attack) = args.attack {
            literal.insert("attack".into(), json!(attack));
        }
        if let Some(health) = args.health {
            literal.insert("health".into(), json!(health));
        }
        (buff(json_as(Value::Object(literal))).apply)(ctx);
    })
}

/// `returnRandomFromGraveyard`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ReturnRandomFromGraveyardArgs {
    pub count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// "Return N random cards from your graveyard to your hand": R60's N different cards, drawn together
/// before any moves, so a card a full hand burns back (§2.4, R4) is not picked again.
pub fn return_random_from_graveyard(args: ReturnRandomFromGraveyardArgs) -> Effect {
    Effect::new("returnRandomFromGraveyard", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let graveyard: Vec<CardInstance> = ctx.state.players[player].graveyard.clone();
        let count = args.count.max(0) as usize;
        if count == 0 || graveyard.is_empty() {
            return;
        }
        let picked: Vec<CardInstance> = ctx.rng.shuffle(&graveyard).into_iter().take(count).collect();
        for mut card in picked {
            put_in_hand(ctx, &mut card);
        }
    })
}
