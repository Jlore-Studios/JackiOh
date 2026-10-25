//! Stinkyscape #148 Alteran Reach's starscape map (#1847, R1847000–R1847002). No card reaches it
//! until the Stinkyscape set ships (R1420); its fixture card proves it (`tests/rules/starscape.rs`).
//!
//! The machinery that is not card data: the map, the travel and the abilities a card declares to
//! travel it.
//!
//!   map     each player has one (R1847000), on `PlayerState.starscape`, absent until their first
//!           travel (D14): every card's travel moves the same map, and it stays when the card leaves.
//!           It is charted at that first travel from the match rng, once: `STARSCAPE_EDGE_TRAVELS`
//!           nodes, each before the last drawn uniformly from `STARSCAPE_REWARDS`, the last the edge
//!           of the universe. The nodes are stored, never drawn again, so a simulation or a replay of
//!           a later travel sees the same map. Public on both seats (`starscape_view`).
//!   travel  one node and its reward (R1847001), reported by the public `starscapeTravelled`: the
//!           node's heal, draw, mana or damage, or at the edge the card the ability names, which a full
//!           hand burns (§2.4). Nothing travels past the edge: the ability stays listed and can't be
//!           activated now.
//!   price   Activate ♾️ (`STARSCAPE_TRAVEL_MANA`); on the Radiant face the player's first travel each
//!           turn is free (R1847002), as two abilities `has` switches between (R752), so the card has
//!           exactly one of them at a time and `legal_actions`, the refusal and the view read one.

use crate::config::{STARSCAPE_EDGE_TRAVELS, STARSCAPE_REWARDS, STARSCAPE_TRAVEL_MANA};
use crate::effects::{
    AddToHandArgs, DamageEffectArgs, DamageFlagArgs, DrawArgs, GainManaArgs, HealArgs, TargetSpec, add_to_hand,
    damage, draw, gain_mana, heal,
};
use crate::rng::Rng;
use crate::script::{
    ActivationCost, ActivationDecl, ActivationUses, ConditionContext, Effect, HasHook, condition_hook, hook,
    read_hook,
};
use crate::state::{GameState, StarscapeMap};
use crate::wire::{GameEvent, PlayerId, StarscapeNodeView, StarscapeReward, StarscapeView};

/// R1847002: the paid travel's ability id, the base face's only one.
pub const STARSCAPE_TRAVEL_ABILITY: &str = "travel";
/// R1847002: the Radiant face's free first travel each turn.
pub const STARSCAPE_FREE_TRAVEL_ABILITY: &str = "free-travel";

/// R1847000: a new map, its nodes before the edge drawn uniformly from `STARSCAPE_REWARDS`, the edge
/// last.
fn chart(rng: &mut Rng) -> Vec<StarscapeReward> {
    let mut nodes: Vec<StarscapeReward> = (1..STARSCAPE_EDGE_TRAVELS)
        .filter_map(|_| rng.pick(STARSCAPE_REWARDS).map(|&(reward, _)| reward))
        .collect();
    nodes.push(StarscapeReward::Edge);
    nodes
}

/// R1847000: how much a node's reward gives (config `STARSCAPE_REWARDS`); the edge gives a card, and
/// no number.
pub fn starscape_amount(reward: StarscapeReward) -> Option<i32> {
    STARSCAPE_REWARDS
        .iter()
        .find(|(listed, _)| *listed == reward)
        .map(|&(_, amount)| amount)
}

/// R1847001: whether `player` has a node left to travel to: true before their map is charted, false
/// once they stand on the edge.
pub fn can_travel_starscape(state: &GameState, player: PlayerId) -> bool {
    state.players[player]
        .starscape
        .as_ref()
        .is_none_or(|map| map.at < map.nodes.len() as i32)
}

/// R1847002: whether `player` has travelled their map this turn.
pub fn starscape_travelled_this_turn(state: &GameState, player: PlayerId) -> bool {
    state.players[player]
        .starscape
        .as_ref()
        .is_some_and(|map| map.travelled_on == state.turn)
}

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
        ctx.sink.events.push(GameEvent::StarscapeTravelled { player, node, reward });
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

/// One travel ability: Activate ♾️, no targets or modes, while a node is left (R1847001).
fn travel_ability(
    id: &str,
    label: &str,
    cost: Option<ActivationCost>,
    has: Option<HasHook>,
    edge_def_id: &'static str,
) -> ActivationDecl {
    ActivationDecl {
        id: id.to_string(),
        label: label.to_string(),
        uses: ActivationUses::Unlimited,
        cost,
        targets: vec![],
        modes: vec![],
        can_activate: Some(condition_hook(|c: ConditionContext<'_>| {
            can_travel_starscape(c.state, c.controller)
        })),
        has,
        run: hook(move |_| vec![travel_starscape(edge_def_id)]),
    }
}

/// R1847002: the abilities a card declares to travel its controller's map, the edge giving
/// `edge_def_id`. The base face has one, a travel for `STARSCAPE_TRAVEL_MANA`. The Radiant face has
/// two that `has` switches between (R752): a free travel while the player has not travelled this
/// turn, and the paid one once they have.
pub fn starscape_abilities(radiant: bool, edge_def_id: &'static str) -> Vec<ActivationDecl> {
    let paid = Some(ActivationCost {
        mana: Some(STARSCAPE_TRAVEL_MANA),
        ..ActivationCost::default()
    });
    if !radiant {
        return vec![travel_ability(
            STARSCAPE_TRAVEL_ABILITY,
            "Travel one node",
            paid,
            None,
            edge_def_id,
        )];
    }
    vec![
        travel_ability(
            STARSCAPE_FREE_TRAVEL_ABILITY,
            "Travel one node (free)",
            None,
            Some(read_hook(|a| {
                !starscape_travelled_this_turn(a.state, a.self_.controller)
            })),
            edge_def_id,
        ),
        travel_ability(
            STARSCAPE_TRAVEL_ABILITY,
            "Travel one node",
            paid,
            Some(read_hook(|a| {
                starscape_travelled_this_turn(a.state, a.self_.controller)
            })),
            edge_def_id,
        ),
    ]
}

/// R1847000: `player`'s map as both seats see it, each node with its reward's amount; absent before
/// their first travel (D14).
pub fn starscape_view(state: &GameState, player: PlayerId) -> Option<StarscapeView> {
    let map = state.players[player].starscape.as_ref()?;
    Some(StarscapeView {
        nodes: map
            .nodes
            .iter()
            .map(|&reward| StarscapeNodeView {
                reward,
                amount: starscape_amount(reward),
            })
            .collect(),
        at: map.at,
    })
}
