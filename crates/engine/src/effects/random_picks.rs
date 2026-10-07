//! Two random picks of existing cards that Classic #90 In Too Deep's rewards name and no verb had
//! (SPEC §8.6 row 90; R60): "a random Unit of yours gets +3/+3" (reward E) and "return 2 random cards
//! from your graveyard to your hand" (reward C). A hook may not roll dice — `ctx.rng` advances
//! `rngCursor`, which is state — so the pick happens here, as the effect applies.
//!
//! Port of `packages/engine/src/effects/randomPicks.ts`.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::draw::add_to_hand as put_in_hand;
use crate::prelude::json_as;
use crate::script::Effect;
use crate::state::CardInstance;
use crate::zones::active_units_of;

use super::buff::buff;
use super::targets::{PlayerSpec, player_of};

/// `buffRandomUnit`'s argument: `{ player? } & BuffAmount`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct BuffRandomUnitArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<i32>,
}

/// "A random Unit of yours gets +X/+Y": one Unit acting on that side (the tops of the piles, R13),
/// drawn with the match rng, gets a layer-4 buff (§10.4) through `buff`. No Unit draws nothing (R129).
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
        // `buff`'s argument as the TS literal `{ target: { of: "instance", instanceId }, ...amount }`,
        // with only the halves this call was given.
        let mut literal = Map::new();
        literal.insert("target".into(), json!({ "of": "instance", "instanceId": unit.id }));
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
#[serde(rename_all = "camelCase")]
pub struct ReturnRandomFromGraveyardArgs {
    pub count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// "Return N random cards from your graveyard to your hand": R60's random pick of N existing cards —
/// N different cards, or all of them if fewer lie there — drawn together before any moves, so a card a
/// full hand burns back into the graveyard (§2.4, R4) is not picked again. Each goes to its owner's
/// hand through §2.4's pipeline in the order drawn. An empty graveyard draws nothing (R129).
pub fn return_random_from_graveyard(args: ReturnRandomFromGraveyardArgs) -> Effect {
    Effect::new("returnRandomFromGraveyard", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let graveyard: Vec<CardInstance> = ctx.state.players[player].graveyard.clone();
        let count = args.count.max(0) as usize;
        if count == 0 || graveyard.is_empty() {
            return;
        }
        let picked: Vec<CardInstance> = ctx.rng.shuffle(&graveyard).into_iter().take(count).collect();
        for card in picked {
            put_in_hand(ctx, card);
        }
    })
}
