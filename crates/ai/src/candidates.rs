//! The moves the search considers, in the order it considers them. `legal_actions` is the only source
//! (so the AI can never offer what the reducer refuses), minus R84's skipped actions (R188: the AI
//! never concedes, offers or accepts a draw) and minus the mulligan, which `decide` answers itself.

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::{
    ActionBody, ActionType, AttackTarget, CardInstance, CostOptions, GameState, KeywordKind, PlayerId,
    Position, Row, Selection, SetAs, UnitView, ZoneChoice, ZoneSlot, active_units_of, adjacent,
    attack_mod_for, attack_restriction, beneath_at, canonical, card_at, carried_at, effective_cost,
    find_instance, has_keyword, is_locked, lane_multiplier_for, legal_actions, midlane_lanes_of, pierces,
    row_size, slot_of, subsystems, takes_strike_back, unit_view,
};
use serde_json::Value;

use crate::config::{AI_SAME_CARD_PLAYS_PER_TURN, AI_SEARCH};
use crate::evaluate::{Prices, can_swing, deft_in_defense};
use crate::reply::trade_value;

/// Canonical JSON of an action body (keys sorted, SURFACE §5.2), for equality across determinizations.
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

/// The zone a play names. An Equip play fuses where it lands, so it is no zone variant of the plain play
/// (R1086).
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

/// Per otherwise-identical play, keep one lane of each placement (`placement_key`): its first in lane
/// order, or its first in the seat's midlane, which a Cry may read (R685: C #22 Mid Runner, TG #5
/// Midlane Menace; R2203001), until `cap` lanes are kept (`None`: every placement, the search root's,
/// #1635). Everything else keeps legal_actions order.
fn collapse_zones(
    state: &GameState,
    seat: PlayerId,
    actions: &[ActionBody],
    cap: Option<usize>,
) -> Vec<ActionBody> {
    let midlane = midlane_lanes_of(state, seat);
    // The index of each placement's lane, per otherwise-identical play.
    let mut chosen: IndexMap<(ActionBody, String), usize> = IndexMap::new();
    for (index, action) in actions.iter().enumerate() {
        let Some(zone) = play_zone(action) else {
            continue;
        };
        let key = (zone_group_key(action), placement_key(state, seat, zone));
        let in_midlane =
            |index: usize| play_zone(&actions[index]).is_some_and(|zone| midlane.contains(&zone.lane));
        match chosen.get(&key) {
            Some(&kept) if in_midlane(kept) || !in_midlane(index) => {}
            _ => {
                chosen.insert(key, index);
            }
        }
    }
    let mut kept: IndexMap<ActionBody, usize> = IndexMap::new();
    actions
        .iter()
        .enumerate()
        .filter(|(index, action)| {
            let Some(zone) = play_zone(action) else {
                return true;
            };
            let group = zone_group_key(action);
            if chosen.get(&(group.clone(), placement_key(state, seat, zone))) != Some(index) {
                return false;
            }
            let placements = kept.entry(group).or_default();
            if cap.is_some_and(|cap| *placements >= cap) {
                return false;
            }
            *placements += 1;
            true
        })
        .map(|(_, action)| action.clone())
        .collect()
}

/// The two strikes of a combat between `attacker` and the Unit `defender` (`a` and `t` their views),
/// as `resolve_combat` reads them before either lands (R94): the attacker's attack plus the board's
/// combat-only bonus (`attack_mod_for`, R1120) times its lane multiplier (`lane_multiplier_for`,
/// R1283), and the defender's attack times its own, which no bonus ever rides.
pub(crate) struct Exchange {
    /// What the attacker's strike deals before §4.4.
    pub hit: i32,
    /// Whether that strike is Poisonous: the attacker's own keyword or the bonus's (R1120).
    pub poisonous: bool,
    /// What the defender's strike back deals before §4.4.
    pub strike_back: i32,
    /// The Cleave splash: the attacker's attack with the combat-only bonus, unmultiplied — the
    /// engine's `strike` passes unmultiplied `attack` as the Cleave amount.
    pub splash: i32,
}

pub(crate) fn exchange(
    state: &GameState,
    attacker: &CardInstance,
    a: &UnitView,
    defender: &CardInstance,
    t: &UnitView,
) -> Exchange {
    let bonus = attack_mod_for(state, attacker, defender);
    Exchange {
        hit: (a.attack + bonus.attack) * lane_multiplier_for(state, attacker, defender),
        poisonous: bonus.poisonous || has_keyword(&a.keywords, KeywordKind::Poisonous),
        strike_back: t.attack * lane_multiplier_for(state, defender, attacker),
        splash: a.attack + bonus.attack,
    }
}

/// One side of a §4.4 blow, read through the layers.
pub(crate) struct Side {
    pub(crate) view: UnitView,
    pub(crate) shield: bool,
    pub(crate) indestructible: bool,
    pub(crate) first_strike: bool,
    pub(crate) poisonous: bool,
    /// R346: its hits skip §4.4 step 2, the Armor.
    pub(crate) pierce: bool,
}

pub(crate) fn side_of(state: &GameState, unit: &CardInstance) -> Side {
    let view = unit_view(state, unit);
    Side {
        shield: has_keyword(&view.keywords, KeywordKind::DivineShield),
        indestructible: has_keyword(&view.keywords, KeywordKind::Indestructible),
        first_strike: has_keyword(&view.keywords, KeywordKind::FirstStrike),
        poisonous: has_keyword(&view.keywords, KeywordKind::Poisonous),
        pierce: pierces(state, Some(unit), None),
        view,
    }
}

/// What one blow does (`blow`'s answer).
pub(crate) struct Blow {
    pub(crate) dealt: i32,
    pub(crate) kills: bool,
    pub(crate) pops_shield: bool,
}

/// §4.4 steps 1, 2, 4 and 7 for one blow: what `from` deals to `to` with a strike of `amount`
/// (`exchange`'s), Poisonous when `poisonous`, and whether it kills. Step 2's Armor is skipped when
/// `from` pierces (R346).
pub(crate) fn blow(from: &Side, amount: i32, poisonous: bool, to: &Side) -> Blow {
    if amount <= 0 {
        return Blow {
            dealt: 0,
            kills: false,
            pops_shield: false,
        };
    }
    if to.shield {
        return Blow {
            dealt: 0,
            kills: false,
            pops_shield: true,
        };
    }
    if to.indestructible {
        return Blow {
            dealt: 0,
            kills: false,
            pops_shield: false,
        };
    }
    let dealt = if from.pierce {
        amount
    } else {
        (amount - to.view.armor).max(0)
    };
    let kills = dealt >= to.view.health || (poisonous && dealt > 0);
    Blow {
        dealt,
        kills,
        pops_shield: false,
    }
}

/// The units one attack on `target` also strikes (§4.4 step 10, R63): each unit adjacent to the
/// target on its own side, when the attacker has Cleave — the engine's `struck_by` minus the
/// defender. One §4.4 implementation serves the reply and the threat model alike.
pub(crate) fn cleaved_neighbours(
    state: &GameState,
    attacker: &CardInstance,
    target: &CardInstance,
) -> Vec<CardInstance> {
    if !has_keyword(&unit_view(state, attacker).keywords, KeywordKind::Cleave) {
        return Vec::new();
    }
    let Some(at) = slot_of(state, target) else {
        return Vec::new();
    };
    adjacent(at)
        .into_iter()
        .filter_map(|slot| card_at(state, slot).cloned())
        .collect()
}

/// Tier 1: an attack on a unit it kills and survives, read from the layers and the combat's own
/// reads (`exchange`). Divine Shield or Indestructible on the target means no kill; a Ranged attacker
/// survives any target that is not Ranged (R783000). Either side's hit skips the other's Armor when
/// it pierces (§4.4 step 2, R346).
fn kills_and_survives(state: &GameState, attacker: &CardInstance, target: &CardInstance) -> bool {
    let a = unit_view(state, attacker);
    let t = unit_view(state, target);
    if has_keyword(&t.keywords, KeywordKind::DivineShield)
        || has_keyword(&t.keywords, KeywordKind::Indestructible)
    {
        return false;
    }
    let combat = exchange(state, attacker, &a, target, &t);
    let dealt = if pierces(state, Some(attacker), None) {
        combat.hit
    } else {
        (combat.hit - t.armor).max(0)
    };
    if dealt <= 0 {
        return false;
    }
    let kills = dealt >= t.health || combat.poisonous;
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
    let taken = if pierces(state, Some(target), None) {
        combat.strike_back
    } else {
        (combat.strike_back - a.armor).max(0)
    };
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

/// #1779: what an attack on a unit is worth when it kills its target, or with Cleave a neighbour
/// (#1821), at an even trade or better by the opponent model's static read (`trade_value`: the
/// target's worth, less the attacker's when it dies too), else `None`. Each attacker's best one goes
/// just after the winning trades (`in_move_order`).
fn even_trade(state: &GameState, attacker_id: &str, target_id: &str, prices: &Prices) -> Option<f64> {
    let trade = trade_value(state, find_instance(state, attacker_id)?, target_id, prices);
    ((trade.kills || trade.splash_kills) && trade.value >= 0.0).then_some(trade.value)
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

/// One source instance's variants (legal_actions order, then dealt by `enemy_first`) and its cost.
struct Source {
    cost: i32,
    variants: Vec<ActionBody>,
}

/// Where a source variant's first target aims (#1062).
#[derive(Clone, Copy, PartialEq, Eq)]
enum FirstAim {
    /// At the enemy hero.
    EnemyHero,
    /// At a card the enemy controls.
    EnemyCard,
    /// Anything else: no target, one of the seat's own, a zone, a mode.
    Other,
}

/// Where `action`'s first target aims, read off the targets it carries.
fn first_aim(state: &GameState, seat: PlayerId, action: &ActionBody) -> FirstAim {
    let targets = match action {
        ActionBody::Play { targets, .. }
        | ActionBody::ActivatePower { targets, .. }
        | ActionBody::Activate { targets, .. } => targets.as_deref(),
        _ => None,
    };
    let enemy = seat.opponent();
    match targets.and_then(|targets| targets.first()) {
        Some(Selection::Hero { player }) if *player == enemy => FirstAim::EnemyHero,
        Some(Selection::Instance { instance_id })
            if find_instance(state, instance_id).is_some_and(|card| card.controller == enemy) =>
        {
            FirstAim::EnemyCard
        }
        _ => FirstAim::Other,
    }
}

/// One source's variants with those aimed at the enemy (the enemy hero's first, then the enemy's
/// cards') dealt alternately with the rest, enemy first: e0, o0, e1, o1, … (#1062). Each group keeps
/// legal_actions order, so a source with no variant aimed at the enemy keeps its order exactly.
fn enemy_first(state: &GameState, seat: PlayerId, variants: Vec<ActionBody>) -> Vec<ActionBody> {
    let mut at_hero: Vec<ActionBody> = Vec::new();
    let mut at_cards: Vec<ActionBody> = Vec::new();
    let mut rest: Vec<ActionBody> = Vec::new();
    for variant in variants {
        match first_aim(state, seat, &variant) {
            FirstAim::EnemyHero => at_hero.push(variant),
            FirstAim::EnemyCard => at_cards.push(variant),
            FirstAim::Other => rest.push(variant),
        }
    }
    at_hero.extend(at_cards);
    let mut enemy = at_hero.into_iter();
    let mut other = rest.into_iter();
    let mut out: Vec<ActionBody> = Vec::new();
    loop {
        match (enemy.next(), other.next()) {
            (None, None) => break,
            (aimed, unaimed) => {
                out.extend(aimed);
                out.extend(unaimed);
            }
        }
    }
    out
}

/// What kind of play or activation a variant represents, so that different ways of being played
/// (biggest/smallest X, Embiggen, Equip, face-down, Tribute sets) get their first candidate
/// before any way repeats a second lane or target (#1778).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct WayKey {
    x: Option<i32>,
    embiggen: Option<bool>,
    equip: bool,
    face_down: bool,
    tributes: Option<Vec<String>>,
    ability: Option<String>,
}

fn way_key(action: &ActionBody) -> WayKey {
    match action {
        ActionBody::Play {
            x,
            embiggen,
            equip,
            face_down,
            tributes,
            ..
        } => WayKey {
            x: *x,
            embiggen: *embiggen,
            equip: *equip == Some(true),
            face_down: face_down.is_some(),
            tributes: tributes.as_ref().map(|t| {
                let mut sorted = t.clone();
                sorted.sort();
                sorted
            }),
            ability: None,
        },
        ActionBody::Activate {
            ability, tributes, ..
        } => WayKey {
            x: None,
            embiggen: None,
            equip: false,
            face_down: false,
            tributes: tributes.as_ref().map(|t| {
                let mut sorted = t.clone();
                sorted.sort();
                sorted
            }),
            ability: ability.clone(),
        },
        _ => WayKey {
            x: None,
            embiggen: None,
            equip: false,
            face_down: false,
            tributes: None,
            ability: None,
        },
    }
}

/// The effective price of a specific play or activation way (#1778).
fn way_price(state: &GameState, action: &ActionBody) -> i32 {
    match action {
        ActionBody::Play {
            instance_id,
            x,
            embiggen,
            face_down,
            ..
        } => {
            let Some(card) = find_instance(state, instance_id) else {
                return 0;
            };
            let probe = CardInstance {
                x: x.or(card.x),
                embiggened: embiggen.or(card.embiggened),
                set_as: face_down
                    .map(|reveal| SetAs {
                        reveal,
                        set_turn: state.turn,
                        echo: None,
                        revealing: None,
                    })
                    .or(card.set_as),
                ..card.clone()
            };
            effective_cost(state, &probe, CostOptions { as_play: Some(true) })
        }
        _ => source_cost(state, action),
    }
}

/// The cost of sacrificed units in a Tribute set: total cost and individual sorted unit costs (#1778).
fn tribute_set_cost(state: &GameState, tributes: Option<&[String]>) -> (i32, Vec<i32>) {
    let Some(ids) = tributes else {
        return (0, Vec::new());
    };
    let mut costs: Vec<i32> = ids
        .iter()
        .map(|id| {
            find_instance(state, id)
                .map(|card| effective_cost(state, card, CostOptions::default()))
                .unwrap_or(0)
        })
        .collect();
    costs.sort();
    (costs.iter().sum(), costs)
}

struct Way {
    variants: Vec<ActionBody>,
    price: i32,
    tribute_cost: (i32, Vec<i32>),
    first_seen: usize,
}

/// Tier 2: round-robin across source instances (every source's first variant, then every second, …),
/// each round by current cost, highest first; a field card with an Activate ability is a source (B3.2).
/// A source's variants are interleaved across its ways of being played (#1778), and inside each way
/// aimed at the enemy first (`enemy_first`, #1062), so the seat's own targets cannot crowd them out.
fn round_robin(state: &GameState, seat: PlayerId, actions: &[ActionBody]) -> Vec<ActionBody> {
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
    for source in &mut sources {
        let mut way_groups: IndexMap<WayKey, (usize, Vec<ActionBody>)> = IndexMap::new();
        for (idx, action) in std::mem::take(&mut source.variants).into_iter().enumerate() {
            let key = way_key(&action);
            let entry = way_groups.entry(key).or_insert_with(|| (idx, Vec::new()));
            entry.1.push(action);
        }

        let mut ways: Vec<Way> = way_groups
            .into_iter()
            .map(|(key, (first_seen, variants))| {
                let variants = enemy_first(state, seat, variants);
                let price = way_price(state, &variants[0]);
                let tribute_cost = tribute_set_cost(state, key.tributes.as_deref());
                Way {
                    variants,
                    price,
                    tribute_cost,
                    first_seen,
                }
            })
            .collect();

        let max_price = ways.iter().map(|w| w.price).max().unwrap_or(0);
        let min_price = ways.iter().map(|w| w.price).min().unwrap_or(0);

        ways.sort_by(|a, b| {
            let rank_of = |price: i32| -> u8 {
                if price == max_price {
                    0
                } else if price == min_price {
                    1
                } else {
                    2
                }
            };
            let rank_a = rank_of(a.price);
            let rank_b = rank_of(b.price);
            rank_a
                .cmp(&rank_b)
                .then_with(|| {
                    if rank_a == 2 {
                        b.price.cmp(&a.price)
                    } else {
                        std::cmp::Ordering::Equal
                    }
                })
                .then_with(|| a.tribute_cost.cmp(&b.tribute_cost))
                .then_with(|| a.first_seen.cmp(&b.first_seen))
        });

        let way_rounds = ways.iter().map(|w| w.variants.len()).max().unwrap_or(0);
        let mut assembled = Vec::new();
        for round in 0..way_rounds {
            for way in &ways {
                if let Some(variant) = way.variants.get(round) {
                    assembled.push(variant.clone());
                }
            }
        }
        source.variants = assembled;
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

/// The candidates' membership, before their order is fixed: legal_actions minus AI_SKIPPED_ACTIONS
/// (R84) and `mulligan`, with `play` zone variants collapsed (`collapse_zones`, R2203001).
/// `candidate_actions` orders it; a caller that only asks membership is spared the ordering's reads.
pub fn candidate_set(state: &GameState, seat: PlayerId) -> Vec<ActionBody> {
    candidate_pool(state, seat, legal_actions(state, seat))
}

/// `candidate_set` on a legal_actions already in hand.
pub fn candidate_pool(state: &GameState, seat: PlayerId, legal: Vec<ActionBody>) -> Vec<ActionBody> {
    pool(state, seat, legal, Some(AI_SEARCH.zone_variants.max(1) as usize))
}

/// `candidate_set` at the search's root (#1635): every distinct placement of a play is kept, so a card
/// that cares where it lands is tried in every lane.
pub fn root_candidate_set(state: &GameState, seat: PlayerId) -> Vec<ActionBody> {
    pool(state, seat, legal_actions(state, seat), None)
}

/// A play of a card the seat has played `AI_SAME_CARD_PLAYS_PER_TURN` times this turn already
/// (`turnLog.playedIds`, rebuilt at each start of turn).
fn played_out(played: &[String], action: &ActionBody) -> bool {
    let ActionBody::Play { instance_id, .. } = action else {
        return false;
    };
    played.iter().filter(|id| *id == instance_id).count() >= AI_SAME_CARD_PLAYS_PER_TURN
}

/// `candidate_pool` with `collapse_zones` keeping at most `zone_cap` lanes of a play (`None`: all).
fn pool(
    state: &GameState,
    seat: PlayerId,
    legal: Vec<ActionBody>,
    zone_cap: Option<usize>,
) -> Vec<ActionBody> {
    let skipped = subsystems::AI_SKIPPED_ACTIONS;
    let played = &state.players[seat].turn_log.played_ids;
    let legal: Vec<ActionBody> = legal
        .into_iter()
        .filter(|action| {
            let kind = action.action_type();
            !skipped.contains(&kind) && kind != ActionType::Mulligan && !played_out(played, action)
        })
        .collect();
    let capped = cap_tribute_sets(state, seat, legal);
    collapse_zones(state, seat, &capped, zone_cap)
}

/// `candidate_set` (R84's concede/offerDraw/answerDraw skipped, `play` zone variants collapsed to at most
/// AI_SEARCH.zone_variants lanes) in move order. endTurn, when legal, is last.
pub fn candidate_actions(state: &GameState, seat: PlayerId, prices: &Prices) -> Vec<ActionBody> {
    candidate_actions_in(state, seat, legal_actions(state, seat), prices)
}

/// `candidate_actions` on a legal_actions already in hand (reply.rs asks for both the raw and the
/// ordered lists of one state).
pub fn candidate_actions_in(
    state: &GameState,
    seat: PlayerId,
    legal: Vec<ActionBody>,
    prices: &Prices,
) -> Vec<ActionBody> {
    in_move_order(state, seat, candidate_pool(state, seat, legal), prices)
}

/// `candidate_actions` at the search's root (#1635): `root_candidate_set` in move order.
pub fn root_candidate_actions(state: &GameState, seat: PlayerId, prices: &Prices) -> Vec<ActionBody> {
    in_move_order(state, seat, root_candidate_set(state, seat), prices)
}

/// The candidates `actions` in move order: hero attacks, winning trades, even trades (each attacker's
/// best, #1779), plays round-robin, answers, Trades of Tradable hand cards (R1838000), buys and sales at
/// the Marketplace (R1853004), other attacks, position switches, endTurn.
fn in_move_order(
    state: &GameState,
    seat: PlayerId,
    actions: Vec<ActionBody>,
    prices: &Prices,
) -> Vec<ActionBody> {
    let mut hero_attacks: Vec<ActionBody> = Vec::new();
    let mut good_trades: Vec<ActionBody> = Vec::new();
    let mut plays: Vec<ActionBody> = Vec::new();
    let mut answers: Vec<ActionBody> = Vec::new();
    // R1838000: a hand card into the deck for a draw. The search tries it on its determinized worlds,
    // whose libraries are resampled (R185), so the card it would draw never steers it.
    let mut hand_trades: Vec<ActionBody> = Vec::new();
    // R1853004: a buy or a sale at the Marketplace, for Credits and no mana. A purchase of the opponent's
    // is hidden from the seat (`observe::redact`), so it never steers the decision.
    let mut market_deals: Vec<ActionBody> = Vec::new();
    // Tier 4, each with `even_trade`'s value.
    let mut late_attacks: Vec<(ActionBody, Option<f64>)> = Vec::new();
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
                    let even = even_trade(state, attacker_id, target_id, prices);
                    late_attacks.push((action, even));
                }
            }
            ActionBody::Play { .. } | ActionBody::ActivatePower { .. } | ActionBody::Activate { .. } => {
                plays.push(action)
            }
            ActionBody::Answer { .. } => answers.push(action),
            ActionBody::Trade { .. } => hand_trades.push(action),
            ActionBody::MarketBuy { .. } | ActionBody::MarketSell { .. } => market_deals.push(action),
            ActionBody::EndTurn => ends.push(action),
            _ => switches.push(action),
        }
    }

    // #1779: each attacker's best even trade (ties: the first) moves up; the rest stay late.
    let mut best: IndexMap<String, (usize, f64)> = IndexMap::new();
    for (at, (action, even)) in late_attacks.iter().enumerate() {
        if let (ActionBody::Attack { attacker_id, .. }, Some(value)) = (action, *even)
            && best.get(attacker_id).is_none_or(|(_, kept)| value > *kept)
        {
            best.insert(attacker_id.clone(), (at, value));
        }
    }
    let picked: IndexSet<usize> = best.values().map(|(at, _)| *at).collect();
    let (even_trades, other_attacks): (Vec<_>, Vec<_>) = late_attacks
        .into_iter()
        .enumerate()
        .map(|(at, (action, _))| (at, action))
        .partition(|(at, _)| picked.contains(at));

    let mut out: Vec<ActionBody> = Vec::new();
    out.extend(hero_attacks);
    out.extend(good_trades);
    out.extend(even_trades.into_iter().map(|(_, action)| action));
    out.extend(round_robin(state, seat, &plays));
    out.extend(answers);
    out.extend(hand_trades);
    out.extend(market_deals);
    out.extend(other_attacks.into_iter().map(|(_, action)| action));
    out.extend(switches);
    out.extend(ends);
    out
}

/// #1759: whether `action` switches a unit in Attack Position to Defense, a stand-down, when a unit of the
/// opponent's that could attack it on its next turn (`could_attack_next_turn`, R2021000) would kill it and
/// survive the fight (`kills_and_survives`): a unit that cannot win its next fight.
pub fn stands_down(state: &GameState, action: &ActionBody) -> bool {
    let ActionBody::SwitchPosition { instance_id } = action else {
        return false;
    };
    let Some(unit) = find_instance(state, instance_id) else {
        return false;
    };
    if unit_view(state, unit).position != Position::Atk {
        return false;
    }
    let target = AttackTarget::Unit {
        instance: unit.clone(),
    };
    active_units_of(state, unit.controller.opponent())
        .into_iter()
        .any(|enemy| could_attack_next_turn(state, enemy, &target) && kills_and_survives(state, enemy, unit))
}

/// R2021000: whether `enemy` could attack `target` on its controller's next turn: it is in Attack
/// Position, or Defense Position with Deft (R49); it may swing at all (`can_swing`: not Frozen through
/// that turn, R1174000); and no restriction bars it from `target` (a Dormant card, R1839000, or a
/// Flying unit only a Flying or Ranged unit may attack, R783002).
fn could_attack_next_turn(state: &GameState, enemy: &CardInstance, target: &AttackTarget) -> bool {
    let view = unit_view(state, enemy);
    (view.position == Position::Atk || deft_in_defense(&view))
        && can_swing(state, enemy, &view)
        && attack_restriction(state, enemy, target).is_ok()
}

#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::{Scenario, Value, json, scenario};

    use super::*;

    /// Classic #73 Nurse Cleaver, a 3/6 with Rush, Cleave and Lifesteal.
    const CLEAVER: &str = "classic-073";
    /// #67 Zoomerbin Oomen, a 1/2.
    const OOMEN: &str = "core-067";
    /// #53 Reno, a 4/6.
    const RENO: &str = "core-053";

    /// p1's Nurse Cleaver facing p2's `field`.
    fn facing(field: Value) -> Scenario {
        jackioh_cards::register_all();
        scenario(json!({
            "p1": { "field": [CLEAVER] },
            "p2": { "field": field },
        }))
    }

    #[test]
    fn a_cleave_attack_whose_only_kills_are_its_splash_is_an_even_trade() {
        // The 3 leaves Reno at 3 and its 4 back leaves the Cleaver at 2, but the splash of 3 kills
        // both Oomens beside it.
        let s = facing(json!([
            { "def": OOMEN, "lane": 1 },
            { "def": RENO, "lane": 2 },
            { "def": OOMEN, "lane": 3 },
        ]));
        let (cleaver, reno) = (s.card(CLEAVER), s.card(RENO));
        let trade = trade_value(s.state(), cleaver, &reno.id, Prices::NONE);
        assert!(!trade.kills && trade.splash_kills);
        assert_eq!(
            even_trade(s.state(), &cleaver.id, &reno.id, Prices::NONE),
            Some(trade.value)
        );
    }

    #[test]
    fn a_cleave_attack_that_kills_nothing_is_no_even_trade() {
        // The same hit on Reno with no neighbour to splash: a chip worth more than 0, and no kill.
        let s = facing(json!([{ "def": RENO, "lane": 2 }]));
        let (cleaver, reno) = (s.card(CLEAVER), s.card(RENO));
        let trade = trade_value(s.state(), cleaver, &reno.id, Prices::NONE);
        assert!(!trade.kills && !trade.splash_kills && trade.value > 0.0);
        assert_eq!(even_trade(s.state(), &cleaver.id, &reno.id, Prices::NONE), None);
    }
}
