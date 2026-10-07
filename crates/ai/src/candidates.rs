//! The moves the search considers, in the order it considers them. `legal_actions` is the only source
//! (so the AI can never offer what the reducer refuses), minus R84's skipped actions (R188: the AI
//! never concedes, offers or accepts a draw) and minus the mulligan, which `decide` answers itself.
//!
//! Port of `packages/ai/src/candidates.ts`.

use indexmap::IndexMap;
use jackioh_engine::{
    ActionBody, ActionType, CardInstance, CostOptions, GameState, KeywordKind, PlayerId, ZoneChoice, canonical,
    effective_cost, find_instance, has_keyword, legal_actions, subsystems, unit_view,
};
use serde_json::Value;

use crate::config::AI_SEARCH;

/// Canonical JSON of an action body (keys sorted), for equality across determinizations. TS's
/// `JSON.stringify` with a key-sorting replacer is SURFACE §5.2's `canonical`.
pub fn action_key(action: &ActionBody) -> String {
    canonical(&serde_json::to_value(action).unwrap_or(Value::Null))
}

/// The play with its lane blanked: two plays with the same key differ only in where they land.
fn zone_group_key(play: &ActionBody) -> String {
    match play {
        ActionBody::Play { zone: Some(zone), .. } => {
            let mut blanked = play.clone();
            if let ActionBody::Play { zone: blanked_zone, .. } = &mut blanked {
                *blanked_zone = Some(ZoneChoice { row: zone.row, lane: -1 });
            }
            action_key(&blanked)
        }
        _ => action_key(play),
    }
}

/// The lane of a play that names a zone (`action.type === "play" && action.zone !== undefined`).
fn play_lane(action: &ActionBody) -> Option<i32> {
    match action {
        ActionBody::Play { zone: Some(zone), .. } => Some(zone.lane),
        _ => None,
    }
}

/// AI_SEARCH.zoneVariants: per otherwise-identical play, keep the lowest and the highest lane (or only
/// the lowest when one variant is asked for). Everything else keeps legal_actions order.
fn collapse_zones(actions: &[ActionBody]) -> Vec<ActionBody> {
    let mut lanes: IndexMap<String, Vec<i32>> = IndexMap::new();
    for action in actions {
        let Some(lane) = play_lane(action) else {
            continue;
        };
        lanes.entry(zone_group_key(action)).or_default().push(lane);
    }
    actions
        .iter()
        .filter(|action| {
            let Some(lane) = play_lane(action) else {
                return true;
            };
            let list = lanes.get(&zone_group_key(action)).map(Vec::as_slice).unwrap_or(&[]);
            // TS's `Math.min(...list)`; the list always holds this play's own lane.
            let low = list.iter().copied().min().unwrap_or(lane);
            let high = list.iter().copied().max().unwrap_or(lane);
            if lane == low {
                return true;
            }
            AI_SEARCH.zone_variants >= 2 && lane == high
        })
        .cloned()
        .collect()
}

/// Tier 1: an attack on a unit it kills and survives, read from the layers. Divine Shield or
/// Indestructible on the target means no kill.
fn kills_and_survives(state: &GameState, attacker: &CardInstance, target: &CardInstance) -> bool {
    let a = unit_view(state, attacker);
    let t = unit_view(state, target);
    if has_keyword(&t.keywords, KeywordKind::DivineShield) || has_keyword(&t.keywords, KeywordKind::Indestructible) {
        return false;
    }
    let dealt = (a.attack - t.armor).max(0);
    if dealt <= 0 {
        return false;
    }
    let kills = dealt >= t.health || has_keyword(&a.keywords, KeywordKind::Poisonous);
    if !kills {
        return false;
    }
    if has_keyword(&a.keywords, KeywordKind::FirstStrike) && !has_keyword(&t.keywords, KeywordKind::FirstStrike) {
        return true;
    }
    if has_keyword(&a.keywords, KeywordKind::DivineShield) || has_keyword(&a.keywords, KeywordKind::Indestructible) {
        return true;
    }
    let taken = (t.attack - a.armor).max(0);
    if taken <= 0 {
        return true;
    }
    if has_keyword(&t.keywords, KeywordKind::Poisonous) {
        return false;
    }
    taken < a.health
}

fn attack_tier(state: &GameState, seat: PlayerId, attacker_id: &str, target_id: &str) -> i32 {
    if target_id == format!("hero-{}", seat.opponent().as_str()) {
        return 0;
    }
    let attacker = find_instance(state, attacker_id);
    let target = find_instance(state, target_id);
    if let (Some(attacker), Some(target)) = (attacker, target)
        && kills_and_survives(state, attacker, target)
    {
        return 1;
    }
    4
}

/// Tier 2's actions: a play, Heroic Power's power (R43) and any card's Activate ability (B3.2, R384).
/// The instance the source names, when `action` is one.
fn source_id(action: &ActionBody) -> Option<&str> {
    match action {
        ActionBody::Play { instance_id, .. }
        | ActionBody::ActivatePower { instance_id, .. }
        | ActionBody::Activate { instance_id, .. } => Some(instance_id),
        _ => None,
    }
}

/// What a play, a power or an ability costs right now, for tier 2's ordering (an ability: its mana price).
fn source_cost(state: &GameState, action: &ActionBody) -> i32 {
    let Some(instance_id) = source_id(action) else {
        return 0;
    };
    let Some(card) = find_instance(state, instance_id) else {
        return 0;
    };
    match action {
        ActionBody::Play { .. } => effective_cost(state, card, CostOptions::default()),
        // R752: a Heroic Power's power is one of its Activate abilities; the alias names the rolled one.
        ActionBody::ActivatePower { .. } => subsystems::power_ability_of(state, card)
            .and_then(|decl| decl.cost.clone())
            .and_then(|cost| cost.mana)
            .unwrap_or(0),
        ActionBody::Activate { ability, .. } => subsystems::abilities_of(state, card)
            .into_iter()
            .find(|decl| ability.as_deref() == Some(decl.id.as_str()))
            .and_then(|decl| decl.cost.clone())
            .and_then(|cost| cost.mana)
            .unwrap_or(0),
        _ => 0,
    }
}

/// One source instance's variants, in legal_actions order, and its cost.
struct Source {
    cost: i32,
    variants: Vec<ActionBody>,
}

/// Tier 2: round-robin across source instances — every source's first variant, then every source's
/// second, … — each round by current cost, highest first (ties keep legal_actions order). A card on
/// the field with an Activate ability is a source like a hand card (B3.2): a ♾️ or Activate N card's
/// repeated uses need nothing here, since the AI re-plans after every action.
fn round_robin(state: &GameState, actions: &[ActionBody]) -> Vec<ActionBody> {
    let mut sources: Vec<Source> = Vec::new();
    let mut by_id: IndexMap<String, usize> = IndexMap::new();
    for action in actions {
        let Some(id) = source_id(action) else {
            continue;
        };
        let at = match by_id.get(id) {
            Some(at) => *at,
            None => {
                sources.push(Source { cost: source_cost(state, action), variants: Vec::new() });
                by_id.insert(id.to_string(), sources.len() - 1);
                sources.len() - 1
            }
        };
        if let Some(source) = sources.get_mut(at) {
            source.variants.push(action.clone());
        }
    }
    let mut ordered: Vec<(usize, Source)> = sources.into_iter().enumerate().collect();
    ordered.sort_by(|(ia, a), (ib, b)| b.cost.cmp(&a.cost).then(ia.cmp(ib)));
    let rounds = ordered.iter().map(|(_, source)| source.variants.len()).max().unwrap_or(0);
    let mut out: Vec<ActionBody> = Vec::new();
    for round in 0..rounds {
        for (_, source) in &ordered {
            if let Some(variant) = source.variants.get(round) {
                out.push(variant.clone());
            }
        }
    }
    out
}

/// legal_actions(state, seat) minus AI_SKIPPED_ACTIONS (R84's concede/offerDraw/answerDraw) and minus
/// `mulligan`, with `play` zone variants collapsed (per otherwise-identical play keep the lowest and
/// the highest `zone.lane`, AI_SEARCH.zoneVariants), in move order. endTurn, when legal, is last.
pub fn candidate_actions(state: &GameState, seat: PlayerId) -> Vec<ActionBody> {
    let skipped = subsystems::AI_SKIPPED_ACTIONS;
    let legal: Vec<ActionBody> = legal_actions(state, seat)
        .into_iter()
        .filter(|action| {
            let kind = action.action_type();
            !skipped.contains(&kind) && kind != ActionType::Mulligan
        })
        .collect();
    let actions = collapse_zones(&legal);

    let mut hero_attacks: Vec<ActionBody> = Vec::new();
    let mut good_trades: Vec<ActionBody> = Vec::new();
    let mut plays: Vec<ActionBody> = Vec::new();
    let mut answers: Vec<ActionBody> = Vec::new();
    let mut other_attacks: Vec<ActionBody> = Vec::new();
    let mut switches: Vec<ActionBody> = Vec::new();
    let mut ends: Vec<ActionBody> = Vec::new();

    for action in actions {
        match &action {
            ActionBody::Attack { attacker_id, target_id } => {
                let tier = attack_tier(state, seat, attacker_id, target_id);
                if tier == 0 {
                    hero_attacks.push(action);
                } else if tier == 1 {
                    good_trades.push(action);
                } else {
                    other_attacks.push(action);
                }
            }
            ActionBody::Play { .. } | ActionBody::ActivatePower { .. } | ActionBody::Activate { .. } => {
                plays.push(action)
            }
            ActionBody::Answer { .. } => answers.push(action),
            ActionBody::EndTurn => ends.push(action),
            _ => switches.push(action),
        }
    }

    let mut out: Vec<ActionBody> = Vec::new();
    out.extend(hero_attacks);
    out.extend(good_trades);
    out.extend(round_robin(state, &plays));
    out.extend(answers);
    out.extend(other_attacks);
    out.extend(switches);
    out.extend(ends);
    out
}
