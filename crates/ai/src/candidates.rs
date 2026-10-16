//! The moves the search considers, in the order it considers them. `legal_actions` is the only source
//! (so the AI can never offer what the reducer refuses), minus R84's skipped actions (R188: the AI
//! never concedes, offers or accepts a draw) and minus the mulligan, which `decide` answers itself.
//!
//! Port of `packages/ai/src/candidates.ts`.

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::{
    ActionBody, ActionType, CardInstance, CostOptions, GameState, KeywordKind, PlayerId, Row, ZoneChoice,
    ZoneSlot, beneath_at, canonical, card_at, carried_at, effective_cost, find_instance, has_keyword,
    is_locked, legal_actions, row_size, subsystems, takes_strike_back, unit_view,
};
use serde_json::Value;

use crate::config::AI_SEARCH;

/// Canonical JSON of an action body (keys sorted), for equality across determinizations. TS's
/// `JSON.stringify` with a key-sorting replacer is SURFACE §5.2's `canonical`.
pub fn action_key(action: &ActionBody) -> String {
    canonical(&serde_json::to_value(action).unwrap_or(Value::Null))
}

/// The play with its lane blanked: two plays with the same key differ only in where they land.
fn zone_group_key(play: &ActionBody) -> ActionBody {
    match play {
        ActionBody::Play { zone: Some(zone), .. } => {
            let mut blanked = play.clone();
            if let ActionBody::Play {
                zone: blanked_zone, ..
            } = &mut blanked
            {
                *blanked_zone = Some(ZoneChoice {
                    row: zone.row,
                    lane: -1,
                });
            }
            blanked
        }
        _ => play.clone(),
    }
}

/// The zone a play names (`action.type === "play" && action.zone !== undefined`). An Equip play fuses
/// where it lands, so it is no zone variant of the plain play (R1086).
fn play_zone(action: &ActionBody) -> Option<&ZoneChoice> {
    match action {
        ActionBody::Play {
            zone: Some(zone),
            equip,
            ..
        } if *equip != Some(true) => Some(zone),
        _ => None,
    }
}

/// One zone of a placement: `|` past the board's edge, else the id of the card acting there (`-` for
/// none), the Unit a carrier holds there (R446), how many cards lie beneath (R13, R447) and `L` when it
/// is Locked. `redact` keeps a hidden card's id and `determinize` keeps it in every world, so all of it
/// is what the seat can read (R185).
fn zone_part(state: &GameState, slot: ZoneSlot) -> String {
    if slot.lane < 1 || slot.lane > row_size(slot.row) {
        return "|".to_string();
    }
    format!(
        "{}{}:{}{}",
        card_at(state, slot).map_or("-", |card| card.id.as_str()),
        carried_at(state, slot).map_or(String::new(), |unit| format!("+{}", unit.id)),
        beneath_at(state, slot).len(),
        if is_locked(state, slot) { "L" } else { "" },
    )
}

/// What a play's lane means to it (#1014): the seat's zones beside it in its row, left then right, or
/// the board's edge, then the lane's four zones (§3.1): the play's own, the seat's other one, and the
/// enemy's unit and backrow zones opposite. Two lanes with the same key are the same placement.
fn placement_key(state: &GameState, seat: PlayerId, zone: &ZoneChoice) -> String {
    let other_row = if zone.row == Row::Units {
        Row::Backrow
    } else {
        Row::Units
    };
    let enemy = seat.opponent();
    [
        (seat, zone.row, zone.lane - 1),
        (seat, zone.row, zone.lane + 1),
        (seat, zone.row, zone.lane),
        (seat, other_row, zone.lane),
        (enemy, Row::Units, zone.lane),
        (enemy, Row::Backrow, zone.lane),
    ]
    .into_iter()
    .map(|(player, row, lane)| zone_part(state, ZoneSlot { player, row, lane }))
    .collect::<Vec<_>>()
    .join("/")
}

/// AI_SEARCH.zone_variants: per otherwise-identical play, keep the first lane of each placement
/// (`placement_key`), in legal_actions order (lane order), until zone_variants lanes are kept; a lane
/// whose placement an earlier one has, or past the cap, goes. Everything else keeps legal_actions order.
fn collapse_zones(state: &GameState, seat: PlayerId, actions: &[ActionBody]) -> Vec<ActionBody> {
    let cap = AI_SEARCH.zone_variants.max(1) as usize;
    let mut kept: IndexMap<ActionBody, IndexSet<String>> = IndexMap::new();
    actions
        .iter()
        .filter(|action| {
            let Some(zone) = play_zone(action) else {
                return true;
            };
            let placements = kept.entry(zone_group_key(action)).or_default();
            placements.len() < cap && placements.insert(placement_key(state, seat, zone))
        })
        .cloned()
        .collect()
}

/// Tier 1: an attack on a unit it kills and survives, read from the layers. Divine Shield or
/// Indestructible on the target means no kill; a Ranged attacker survives any target that is not
/// Ranged (R783000).
fn kills_and_survives(state: &GameState, attacker: &CardInstance, target: &CardInstance) -> bool {
    let a = unit_view(state, attacker);
    let t = unit_view(state, target);
    if has_keyword(&t.keywords, KeywordKind::DivineShield)
        || has_keyword(&t.keywords, KeywordKind::Indestructible)
    {
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
    // R783000: the engine's own read — no strike back reaches it, a First Strike one included.
    if !takes_strike_back(state, attacker, target) {
        return true;
    }
    if has_keyword(&a.keywords, KeywordKind::FirstStrike)
        && !has_keyword(&t.keywords, KeywordKind::FirstStrike)
    {
        return true;
    }
    if has_keyword(&a.keywords, KeywordKind::DivineShield)
        || has_keyword(&a.keywords, KeywordKind::Indestructible)
    {
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
            .and_then(|decl| decl.cost)
            .and_then(|cost| cost.mana)
            .unwrap_or(0),
        ActionBody::Activate { ability, .. } => subsystems::abilities_of(state, card)
            .into_iter()
            .find(|decl| ability.as_deref() == Some(decl.id.as_str()))
            .and_then(|decl| decl.cost)
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
                sources.push(Source {
                    cost: source_cost(state, action),
                    variants: Vec::new(),
                });
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
    let rounds = ordered
        .iter()
        .map(|(_, source)| source.variants.len())
        .max()
        .unwrap_or(0);
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

/// R90, R1282: When a card has more tribute sets than AI_SEARCH.tribute_sets, keep the ones
/// that spend the fewest of seat's own cards. Ties go to the set seen first.
fn cap_tribute_sets(state: &GameState, seat: PlayerId, actions: Vec<ActionBody>) -> Vec<ActionBody> {
    let mut sets_by_instance: IndexMap<String, Vec<Vec<String>>> = IndexMap::new();
    for action in &actions {
        if let ActionBody::Play {
            instance_id,
            tributes: Some(tributes),
            ..
        } = action
        {
            let mut sorted = tributes.clone();
            sorted.sort();
            let sets = sets_by_instance.entry(instance_id.clone()).or_default();
            if !sets.contains(&sorted) {
                sets.push(sorted);
            }
        }
    }

    let mut allowed_sets_by_instance: IndexMap<String, indexmap::IndexSet<Vec<String>>> = IndexMap::new();
    let limit = AI_SEARCH.tribute_sets as usize;

    for (instance_id, sets) in sets_by_instance {
        if sets.len() <= limit {
            continue;
        }
        let mut scored: Vec<(usize, Vec<String>, usize)> = sets
            .into_iter()
            .enumerate()
            .map(|(idx, set)| {
                let own_spent = set
                    .iter()
                    .filter(|id| find_instance(state, id).is_some_and(|c| c.controller == seat))
                    .count();
                (idx, set, own_spent)
            })
            .collect();

        scored.sort_by(|(ia, _, oa), (ib, _, ob)| oa.cmp(ob).then(ia.cmp(ib)));

        let kept: indexmap::IndexSet<Vec<String>> =
            scored.into_iter().take(limit).map(|(_, set, _)| set).collect();

        allowed_sets_by_instance.insert(instance_id, kept);
    }

    if allowed_sets_by_instance.is_empty() {
        return actions;
    }

    actions
        .into_iter()
        .filter(|action| {
            if let ActionBody::Play {
                instance_id,
                tributes: Some(tributes),
                ..
            } = action
                && let Some(allowed) = allowed_sets_by_instance.get(instance_id)
            {
                let mut sorted = tributes.clone();
                sorted.sort();
                return allowed.contains(&sorted);
            }
            true
        })
        .collect()
}

/// The candidates' membership, before their order is fixed: legal_actions(state, seat) minus
/// AI_SKIPPED_ACTIONS (R84's concede/offerDraw/answerDraw) and minus `mulligan`, with `play` zone
/// variants collapsed (per otherwise-identical play the lowest lane of each placement, at most
/// AI_SEARCH.zone_variants, `collapse_zones`). `candidate_actions` orders this list; a caller that only
/// needs to know whether an action is among the candidates is spared the ordering's reads of the layers.
pub fn candidate_set(state: &GameState, seat: PlayerId) -> Vec<ActionBody> {
    candidate_pool(state, seat, legal_actions(state, seat))
}

/// `candidate_set` on a legal_actions already in hand.
pub fn candidate_pool(state: &GameState, seat: PlayerId, legal: Vec<ActionBody>) -> Vec<ActionBody> {
    let skipped = subsystems::AI_SKIPPED_ACTIONS;
    let legal: Vec<ActionBody> = legal
        .into_iter()
        .filter(|action| {
            let kind = action.action_type();
            !skipped.contains(&kind) && kind != ActionType::Mulligan
        })
        .collect();
    let capped = cap_tribute_sets(state, seat, legal);
    collapse_zones(state, seat, &capped)
}

/// legal_actions(state, seat) minus AI_SKIPPED_ACTIONS (R84's concede/offerDraw/answerDraw) and minus
/// `mulligan`, with `play` zone variants collapsed (per otherwise-identical play the lowest lane of each
/// placement, at most AI_SEARCH.zone_variants, `collapse_zones`), in move order. endTurn, when legal, is
/// last.
pub fn candidate_actions(state: &GameState, seat: PlayerId) -> Vec<ActionBody> {
    candidate_actions_in(state, seat, legal_actions(state, seat))
}

/// `candidate_actions` on a legal_actions already in hand (reply.rs asks for both the raw and the
/// ordered lists of one state).
pub fn candidate_actions_in(state: &GameState, seat: PlayerId, legal: Vec<ActionBody>) -> Vec<ActionBody> {
    let actions = candidate_pool(state, seat, legal);

    let mut hero_attacks: Vec<ActionBody> = Vec::new();
    let mut good_trades: Vec<ActionBody> = Vec::new();
    let mut plays: Vec<ActionBody> = Vec::new();
    let mut answers: Vec<ActionBody> = Vec::new();
    let mut other_attacks: Vec<ActionBody> = Vec::new();
    let mut switches: Vec<ActionBody> = Vec::new();
    let mut ends: Vec<ActionBody> = Vec::new();

    for action in actions {
        match &action {
            ActionBody::Attack {
                attacker_id,
                target_id,
            } => {
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
