//! Travel the starscape (SPEC §6.3's Starscape map row, R1847000, R1847001), Stinkyscape #148 Alteran
//! Reach's map; its chart, view and travel abilities are `subsystems/starscape.rs`'s.

use crate::effects::{
    AddToHandArgs, DamageEffectArgs, DamageFlagArgs, DrawArgs, GainManaArgs, HealArgs, TargetSpec,
    add_to_hand, damage, draw, gain_mana, heal,
};
use crate::script::Effect;
use crate::state::StarscapeMap;
use crate::subsystems::starscape::{can_travel_starscape, chart, starscape_amount};
use crate::wire::{GameEvent, StarscapeReward};

/// R1847000, R1847001: the running card's controller travels one node of their map, charting it first
/// when it is absent, and takes the node's reward: at the edge, a card of `edge_def_id` to their hand.
pub fn travel_starscape(edge_def_id: &'static str) -> Effect {
    Effect::new("travelStarscape", move |ctx| {
        let player = ctx.controller;
        if !can_travel_starscape(ctx.state, player) {
            return;
        }
        if ctx.state.players[player].starscape.is_none() {
            let nodes = chart(&mut *ctx.sink.rng);
            ctx.sink.state.players[player].starscape = Some(StarscapeMap {
                nodes,
                at: 0,
                travelled_on: 0,
            });
        }
        let turn = ctx.state.turn;
        let Some(map) = ctx.sink.state.players[player].starscape.as_mut() else {
            return;
        };
        map.at += 1;
        map.travelled_on = turn;
        let node = map.at;
        let Some(&reward) = map.nodes.get((node - 1) as usize) else {
            return;
        };
        ctx.sink
            .events
            .push(GameEvent::StarscapeTravelled { player, node, reward });
        let amount = starscape_amount(reward).unwrap_or(0);
        let effect = match reward {
            StarscapeReward::Heal => heal(HealArgs::Amount {
                target: TargetSpec::SelfHero,
                amount,
            }),
            StarscapeReward::Draw => draw(DrawArgs {
                count: amount,
                player: None,
            }),
            StarscapeReward::Mana => gain_mana(GainManaArgs { amount, player: None }),
            StarscapeReward::Damage => damage(DamageEffectArgs {
                to: TargetSpec::EnemyHero,
                amount,
                flags: DamageFlagArgs::default(),
            }),
            // §2.4: a full hand burns it.
            StarscapeReward::Edge => add_to_hand(AddToHandArgs {
                def_id: Some(edge_def_id.to_string()),
                ..AddToHandArgs::default()
            }),
        };
        (effect.apply)(ctx);
    })
}
