//! Stinkyscape #148 Alteran Reach's starscape map (#1847, R1847000–R1847002). No card reaches it
//! until the Stinkyscape set ships (R1420); its fixture card proves it (`tests/rules/starscape.rs`).
//!
//! Each player has one public map (R1847000) on `PlayerState.starscape`, absent until their first
//! travel (D14), shared by every card's travel and kept when the card leaves. It is charted once from
//! the match rng and stored, so a simulation or a replay of a later travel sees the same map. A travel
//! (R1847001) takes one node's reward, at the edge the card the ability names (a full hand burns it,
//! §2.4); nothing travels past the edge. On the Radiant face the first travel each turn is free
//! (R1847002), as two abilities `has` switches between (R752), so the card has one at a time.

use crate::config::{STARSCAPE_EDGE_TRAVELS, STARSCAPE_REWARDS, STARSCAPE_TRAVEL_MANA};
use crate::effects::travel_starscape;
use crate::rng::Rng;
use crate::script::{
    ActivationCost, ActivationDecl, ActivationUses, ConditionContext, HasHook, condition_hook, hook,
    read_hook,
};
use crate::state::GameState;
use crate::wire::{PlayerId, StarscapeNodeView, StarscapeReward, StarscapeView};

/// R1847002: the paid travel's ability id, the base face's only one.
pub const STARSCAPE_TRAVEL_ABILITY: &str = "travel";
/// R1847002: the Radiant face's free first travel each turn.
pub const STARSCAPE_FREE_TRAVEL_ABILITY: &str = "free-travel";

/// R1847000: a new map, its nodes drawn uniformly from `STARSCAPE_REWARDS`, the edge last.
pub(crate) fn chart(rng: &mut Rng) -> Vec<StarscapeReward> {
    let mut nodes: Vec<StarscapeReward> = (1..STARSCAPE_EDGE_TRAVELS)
        .filter_map(|_| rng.pick(STARSCAPE_REWARDS).map(|&(reward, _)| reward))
        .collect();
    nodes.push(StarscapeReward::Edge);
    nodes
}

/// R1847000: how much a node's reward gives; the edge gives a card, and no number.
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

/// R1847002: whether `player` has traveled their map this turn.
pub fn starscape_traveled_this_turn(state: &GameState, player: PlayerId) -> bool {
    state.players[player]
        .starscape
        .as_ref()
        .is_some_and(|map| map.travelled_on == state.turn)
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
/// `edge_def_id`: the paid travel, and on the Radiant face a free one before this turn's first travel.
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
                !starscape_traveled_this_turn(a.state, a.self_.controller)
            })),
            edge_def_id,
        ),
        travel_ability(
            STARSCAPE_TRAVEL_ABILITY,
            "Travel one node",
            paid,
            Some(read_hook(|a| {
                starscape_traveled_this_turn(a.state, a.self_.controller)
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
