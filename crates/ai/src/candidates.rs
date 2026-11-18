//! The moves the search considers, in the order it considers them. `legal_actions` is the only source
//! (so the AI can never offer what the reducer refuses), minus R84's skipped actions (R188: the AI
//! never concedes, offers or accepts a draw) and minus the mulligan, which `decide` answers itself.

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::afflictions::Affliction;
use jackioh_engine::effects::PLAGUE_PLACEMENT_HOOK;
use jackioh_engine::{
    ActionBody, ActionType, AttackTarget, CardInstance, CostOptions, FengShuiOutcome, GameState, KeywordKind,
    PlayerId, Position, Row, Selection, SetAs, UnitView, ZoneChoice, ZoneSlot, active_units_of, adjacent,
    attack_mod_for, attack_restriction, beneath_at, canonical, card_at, carried_at, converts_to,
    damage_pool_of, effective_cost, effective_life, find_instance, find_instance_mut, flags_of, has_keyword,
    is_locked, lane_multiplier_for, legal_actions, midlane_lanes_of, pierces, position_after_switch,
    row_size, slot_of, strike_back_multiplier_for, subsystems, takes_strike_back, unit_damage_cap, unit_view,
};
use serde_json::Value;

use crate::config::{AI_EVAL, AI_SAME_CARD_PLAYS_PER_TURN, AI_SEARCH};
use crate::evaluate::{
    Prices, afflicted_before_swing, can_swing, crumbles_before_swing, dead_in_hand, deft_in_defense,
    exiles_what_it_damages, face_threat, feng_shui_acts, feng_shui_verdict, plague_bomb_on_field,
};
use crate::lethal::ready_gap;
use crate::reply::{face_value, trade_value};

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
/// R1283), and the defender's attack times its lane strike alone (`strike_back_multiplier_for`, R4156000),
/// which no bonus ever rides.
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
        strike_back: t.attack * strike_back_multiplier_for(state, defender, attacker),
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
    /// R2179005: its own guard's cap on each hit it takes, after Armor (§4.4 step 3, `unit_damage_cap`).
    pub(crate) cap: Option<i32>,
    /// R1124, R4065001: its hits exile the Unit they damage (`exiles_what_it_damages`, by `AI_EVAL`'s flag).
    pub(crate) exiles: bool,
    /// R4064001: the counter its hits turn into instead of damage (§4.4 step 3a, `converts_to`), read
    /// with `AI_EVAL.pool_and_conversion_read`.
    pub(crate) converts: Option<Affliction>,
    /// R4064000: what its side's damage pools have left for a hit on it (§4.4 step 3b, `damage_pool_of`),
    /// read with `AI_EVAL.pool_and_conversion_read`; 0 without it.
    pub(crate) pool: i32,
}

pub(crate) fn side_of(state: &GameState, unit: &CardInstance) -> Side {
    let view = unit_view(state, unit);
    let read = AI_EVAL.pool_and_conversion_read;
    Side {
        shield: has_keyword(&view.keywords, KeywordKind::DivineShield),
        indestructible: has_keyword(&view.keywords, KeywordKind::Indestructible),
        first_strike: has_keyword(&view.keywords, KeywordKind::FirstStrike),
        poisonous: has_keyword(&view.keywords, KeywordKind::Poisonous),
        pierce: pierces(state, Some(unit), None),
        cap: unit_damage_cap(state, unit),
        exiles: exiles_what_it_damages(state, unit, &AI_EVAL),
        converts: if read { converts_to(state, unit) } else { None },
        pool: if read {
            damage_pool_of(state, unit.controller, false)
        } else {
            0
        },
        view,
    }
}

/// What one blow does (`blow`'s answer).
#[derive(Default)]
pub(crate) struct Blow {
    pub(crate) dealt: i32,
    pub(crate) kills: bool,
    pub(crate) pops_shield: bool,
    /// R4064000: what the target side's damage pools took of it.
    pub(crate) soaked: i32,
    /// R4064001: the counter it adds instead of damage, of `from.converts`'s kind.
    pub(crate) converted: i32,
}

/// §4.4 steps 1 to 4 and 7 for one blow: what `from` deals to `to` with a strike of `amount`
/// (`exchange`'s), Poisonous when `poisonous`, and whether it kills. Step 2's Armor is skipped when
/// `from` pierces (R346), and step 3 caps it at `to`'s own cap (R2179005). Then a hit that converts
/// deals nothing and is its counter (step 3a, R4064001), and `to.pool` soaks what it can of the rest
/// (step 3b, R4064000). A blow from a side that exiles what it damages removes `to` with any damage
/// left dealt, as a Poisonous one does (R1124, R4065001).
pub(crate) fn blow(from: &Side, amount: i32, poisonous: bool, to: &Side) -> Blow {
    if amount <= 0 {
        return Blow::default();
    }
    if to.shield {
        return Blow {
            pops_shield: true,
            ..Blow::default()
        };
    }
    if to.indestructible {
        return Blow::default();
    }
    let dealt = if from.pierce {
        amount
    } else {
        (amount - to.view.armor).max(0)
    };
    let dealt = to.cap.map_or(dealt, |cap| dealt.min(cap));
    // R4064001, §4.4 step 3a: a hit that converts deals nothing and spends no pool; what is left is its
    // counter.
    if from.converts.is_some() {
        return Blow {
            converted: dealt,
            ..Blow::default()
        };
    }
    // R4064000, step 3b: the target side's pools soak what they have left.
    let soaked = dealt.min(to.pool).max(0);
    let dealt = dealt - soaked;
    let kills = dealt >= to.view.health || ((poisonous || from.exiles) && dealt > 0);
    Blow {
        dealt,
        kills,
        soaked,
        ..Blow::default()
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
/// it pierces (§4.4 step 2, R346), and each side's hit is capped by the side it hits (§4.4 step 3,
/// R2179005). With `AI_EVAL.pool_and_conversion_read`, a hit that Burns or is Toxic instead kills
/// nothing (step 3a, R4064001), and each side's damage pools soak what they have left of the hit on it
/// (step 3b, R4064000). A hit from a side that exiles what it damages removes with any damage, as a
/// Poisonous one does (R1124, R4065001).
fn kills_and_survives(state: &GameState, attacker: &CardInstance, target: &CardInstance) -> bool {
    let a = unit_view(state, attacker);
    let t = unit_view(state, target);
    if has_keyword(&t.keywords, KeywordKind::DivineShield)
        || has_keyword(&t.keywords, KeywordKind::Indestructible)
    {
        return false;
    }
    let read = AI_EVAL.pool_and_conversion_read;
    if read && converts_to(state, attacker).is_some() {
        return false;
    }
    let combat = exchange(state, attacker, &a, target, &t);
    let dealt = if pierces(state, Some(attacker), None) {
        combat.hit
    } else {
        (combat.hit - t.armor).max(0)
    };
    let dealt = unit_damage_cap(state, target).map_or(dealt, |cap| dealt.min(cap));
    let dealt = if read {
        dealt - dealt.min(damage_pool_of(state, target.controller, false))
    } else {
        dealt
    };
    if dealt <= 0 {
        return false;
    }
    let kills = dealt >= t.health || combat.poisonous || exiles_what_it_damages(state, attacker, &AI_EVAL);
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
    let taken = unit_damage_cap(state, attacker).map_or(taken, |cap| taken.min(cap));
    let taken = if !read {
        taken
    } else if converts_to(state, target).is_some() {
        0
    } else {
        taken - taken.min(damage_pool_of(state, attacker.controller, false))
    };
    if taken <= 0 {
        return true;
    }
    if has_keyword(&t.keywords, KeywordKind::Poisonous) || exiles_what_it_damages(state, target, &AI_EVAL) {
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
/// target's worth, less the attacker's when it dies too), else `None`. Each attacker's best one ranks
/// with the hero attacks and the winning trades (`in_move_order`, #2825).
fn even_trade(state: &GameState, attacker_id: &str, target_id: &str, prices: &Prices) -> Option<f64> {
    let trade = trade_value(state, find_instance(state, attacker_id)?, target_id, prices);
    ((trade.kills || trade.splash_kills) && trade.value >= 0.0).then_some(trade.value)
}

/// #2825: where an attack on the enemy hero or a winning trade stands among the move order's first
/// attacks, by the opponent model's own values (`reply.rs`'s `ranked_attacks`): a hit on the hero by
/// `face_value` (`AI_EVAL.win` for each when `face_is_lethal`), a hit on a unit by `trade_value`.
fn front_value(
    state: &GameState,
    seat: PlayerId,
    action: &ActionBody,
    face_is_lethal: bool,
    prices: &Prices,
) -> f64 {
    let ActionBody::Attack {
        attacker_id,
        target_id,
    } = action
    else {
        return f64::NEG_INFINITY;
    };
    let Some(attacker) = find_instance(state, attacker_id) else {
        return f64::NEG_INFINITY;
    };
    let enemy = seat.opponent();
    if *target_id == format!("hero-{}", enemy.as_str()) {
        face_value(state, attacker, enemy, face_is_lethal)
    } else {
        trade_value(state, attacker, target_id, prices).value
    }
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

/// R3951100: how a Feng Shui acting would judge a source's play now, tier 2's first key while one acts:
/// a play it rewards first, one it punishes last, and every other source between.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Auspice {
    /// R4154403: the seat's own acting Feng Shui's Activate, a free Auspicious Rock, before every play.
    Judge,
    Rewarded,
    Plain,
    Punished,
}

/// R3951100: the `Auspice` of `action`'s source while a Feng Shui acts (`judging`): a play by the verdict
/// its card would get now (`feng_shui_verdict`), an Activate or a power, which no Feng Shui judges, plain.
/// Plain for every source with none acting, so the move order is unchanged without one.
fn auspice(state: &GameState, seat: PlayerId, action: &ActionBody, judging: bool) -> Auspice {
    if judging && judges_own_rock(state, seat, action) {
        return Auspice::Judge;
    }
    let ActionBody::Play { instance_id, .. } = action else {
        return Auspice::Plain;
    };
    if !judging {
        return Auspice::Plain;
    }
    match find_instance(state, instance_id).and_then(|card| feng_shui_verdict(state, seat, card)) {
        Some((FengShuiOutcome::Positive, _)) => Auspice::Rewarded,
        Some((FengShuiOutcome::Negative, _)) => Auspice::Punished,
        None => Auspice::Plain,
    }
}

/// R4154403: whether `action` is an Activate of a Feng Shui of `seat`'s own (a card whose running face holds
/// the `feng_shui` flag, Meditative #40), whose every ability is free and adds an Auspicious Rock (M #39.1)
/// to the hand: tried before the plays, the Rock in hand is a card whose element the next play can chain on
/// (R981), and a level's `branching` cut never drops the turn's free card.
fn judges_own_rock(state: &GameState, seat: PlayerId, action: &ActionBody) -> bool {
    let ActionBody::Activate { instance_id, .. } = action else {
        return false;
    };
    find_instance(state, instance_id)
        .is_some_and(|card| card.controller == seat && flags_of(state, card).feng_shui == Some(true))
}

/// One source instance's variants (legal_actions order, then dealt by `enemy_first`), its cost and its
/// `Auspice`.
struct Source {
    cost: i32,
    auspice: Auspice,
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
/// While a Feng Shui acts, each round takes the plays it rewards first and those it punishes last
/// (`Auspice`, R3951100), so a level's `branching` cut keeps the play that chains the elements.
/// A source's variants are interleaved across its ways of being played (#1778), and inside each way
/// aimed at the enemy first (`enemy_first`, #1062), so the seat's own targets cannot crowd them out.
fn round_robin(state: &GameState, seat: PlayerId, actions: &[ActionBody]) -> Vec<ActionBody> {
    let judging = feng_shui_acts(state);
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
                    auspice: auspice(state, seat, action, judging),
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
    ordered.sort_by(|(ia, a), (ib, b)| {
        a.auspice
            .cmp(&b.auspice)
            .then(b.cost.cmp(&a.cost))
            .then(ia.cmp(ib))
    });
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

/// The candidates `actions` in move order: hero attacks, winning trades and even trades (each attacker's
/// best, #1779) ranked together by the opponent model's values, best first (#2825); then the Trades of dead
/// Tradable cards (R4154400), plays round-robin, answers (a Plague Counter's placement under a Living Bomb
/// aimed at the enemy first, R4154402), the other Trades of Tradable hand cards (R1838000), buys and sales at
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
    // R4154400: the Trades of dead Tradable cards (`dead_in_hand`), before the plays.
    let mut dead_trades: Vec<ActionBody> = Vec::new();
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
            // R4154400: a Trade of a dead card first, ahead of the plays, so a line can play what it draws.
            ActionBody::Trade { instance_id } if trades_dead_card(state, seat, instance_id) => {
                dead_trades.push(action)
            }
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
        .partition(|(at, _)| picked.contains(at));

    // #2825: the hero attacks, winning trades and even trades ranked together by the opponent model's
    // values, best first, ties in that order; while the ready attacks reach the face together
    // (`ready_gap`), every hero attack ranks first, so the lethal solver's walk still finds it first.
    let face_is_lethal = !hero_attacks.is_empty() && ready_gap(state, seat) <= 0.0;
    let mut front: Vec<(ActionBody, f64)> = hero_attacks
        .into_iter()
        .chain(good_trades)
        .map(|action| {
            let value = front_value(state, seat, &action, face_is_lethal, prices);
            (action, value)
        })
        .collect();
    front.extend(
        even_trades
            .into_iter()
            .map(|(_, (action, even))| (action, even.unwrap_or(f64::NEG_INFINITY))),
    );
    front.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut out: Vec<ActionBody> = Vec::new();
    out.extend(front.into_iter().map(|(action, _)| action));
    out.extend(dead_trades);
    out.extend(round_robin(state, seat, &plays));
    out.extend(plague_aims_first(state, seat, answers));
    out.extend(hand_trades);
    out.extend(market_deals);
    out.extend(other_attacks.into_iter().map(|(_, (action, _))| action));
    out.extend(switches);
    out.extend(ends);
    out
}

/// R4154400: whether the hand card `instance_id` a Trade would put back is a dead Tradable card of `seat`'s
/// (`dead_in_hand`), and the evaluation reads one (`dead_tradable_discount`).
fn trades_dead_card(state: &GameState, seat: PlayerId, instance_id: &str) -> bool {
    AI_EVAL.dead_tradable_discount != 0.0
        && state.players[seat]
            .hand
            .iter()
            .find(|card| card.id == instance_id)
            .is_some_and(|card| dead_in_hand(state, seat, card))
}

/// R4154402: whether `answerer`'s open prompt places Plague Counters (`PLAGUE_PLACEMENT_HOOK`) while a Living
/// Bomb lies on the field (`plague_bomb_on_field`), where a counter is a destroy waiting (`plague_doomed`).
pub(crate) fn plague_placement_open(state: &GameState, answerer: PlayerId) -> bool {
    state
        .pending
        .as_ref()
        .is_some_and(|pending| pending.player_id == answerer && pending.resume.hook == PLAGUE_PLACEMENT_HOOK)
        && plague_bomb_on_field(state)
}

/// R4154402: `answers`, the answers of `answerer`'s open prompt, those aimed at a permanent the answerer's
/// enemy controls first, each group in its own order, while that prompt places a Plague Counter under a
/// Living Bomb (`plague_placement_open`), so a level's `branching` cut keeps the enemy's permanents; the
/// answers as they came otherwise. A face-down permanent is named by its id alone (R177), its controller
/// public.
pub(crate) fn plague_aims_first(
    state: &GameState,
    answerer: PlayerId,
    answers: Vec<ActionBody>,
) -> Vec<ActionBody> {
    if !plague_placement_open(state, answerer) {
        return answers;
    }
    let (enemy, own): (Vec<ActionBody>, Vec<ActionBody>) = answers.into_iter().partition(|answer| {
        let ActionBody::Answer { selection, .. } = answer else {
            return false;
        };
        selection.iter().any(|picked| {
            matches!(picked, Selection::Instance { instance_id }
                if find_instance(state, instance_id).is_some_and(|card| card.controller != answerer))
        })
    });
    enemy.into_iter().chain(own).collect()
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

/// #2976, R2976000: of `candidates`, the switches of a unit from Attack to Defense Position that lower the
/// face damage the opponent's board could deal `seat` next turn (`face_threat`, which counts the Taunt
/// Defense Position grants, §4.1, binding the attackers, §4.2 step 3), best first: the least damage left,
/// ties in `candidates`' order. None unless that damage reaches `seat`'s life (`effective_life`) as it
/// stands, the point where the evaluation's `lethal_threat` applies. Each switch is tried on a copy of
/// the state with the unit's position set, so no search node is spent.
pub fn wall_switches(state: &GameState, seat: PlayerId, candidates: &[ActionBody]) -> Vec<ActionBody> {
    let opponent = seat.opponent();
    let threat = face_threat(state, opponent);
    if threat < effective_life(state, seat) {
        return Vec::new();
    }
    let mut walls: Vec<(ActionBody, i32)> = Vec::new();
    for action in candidates {
        let ActionBody::SwitchPosition { instance_id } = action else {
            continue;
        };
        let Some(unit) = find_instance(state, instance_id) else {
            continue;
        };
        if unit_view(state, unit).position != Position::Atk {
            continue;
        }
        let mut walled = state.clone();
        if let Some(live) = find_instance_mut(&mut walled, instance_id) {
            live.position = Some(position_after_switch(unit));
        }
        let left = face_threat(&walled, opponent);
        if left < threat {
            walls.push((action.clone(), left));
        }
    }
    walls.sort_by_key(|(_, left)| *left);
    walls.into_iter().map(|(action, _)| action).collect()
}

/// R2021000: whether `enemy` could attack `target` on its controller's next turn: it is in Attack
/// Position, or Defense Position with Deft (R49); it may swing at all (`can_swing`: not Frozen through
/// that turn, R1174000); its own Brittle tick does not destroy it first (`crumbles_before_swing`,
/// R3793000, by `AI_EVAL`'s flag: only the search asks), nor the first tick of its own Burn and Toxic
/// (`afflicted_before_swing`, R4065000, by `AI_EVAL`'s flag too); and no restriction bars it from `target` (a
/// Dormant card, R1839000, or a Flying unit only a Flying or Ranged unit may attack, R783002).
fn could_attack_next_turn(state: &GameState, enemy: &CardInstance, target: &AttackTarget) -> bool {
    let view = unit_view(state, enemy);
    (view.position == Position::Atk || deft_in_defense(&view))
        && can_swing(state, enemy, &view)
        && !(AI_EVAL.brittle_crumbles_before_swing && crumbles_before_swing(state, enemy, &view, false))
        && !(AI_EVAL.affliction_kills_before_swing
            && afflicted_before_swing(
                state,
                enemy,
                &view,
                false,
                matches!(target, AttackTarget::Hero { .. }),
                &AI_EVAL,
            ))
        && attack_restriction(state, enemy, target).is_ok()
}

#[cfg(test)]
mod tests {
    use jackioh_engine::combat_forecast;
    use jackioh_engine::config::DECK_SIZE;
    use jackioh_engine::testkit::{Keyword, Scenario, Value, json, scenario};
    use jackioh_engine::{PerPlayer, Rng};

    use super::*;
    use crate::config::AI_REPLY;
    use crate::deck::{AiDeckOptions, build_ai_deck};
    use crate::evaluate::{kill_worth, unit_worth};
    use crate::match_::{MatchConfig, MatchHooks, MatchSetup, SeatController, play_match};
    use crate::reply::fight;

    /// Classic #73 Nurse Cleaver, with Rush, Cleave and Lifesteal: a 3/4 since issue #3136, set back to the 3/6
    /// these trades were worked out for (see `facing`).
    const CLEAVER: &str = "classic-073";
    /// #67 Zoomerbin Oomen, a 1/2.
    const OOMEN: &str = "core-067";
    /// #53 Reno, a 4/6.
    const RENO: &str = "core-053";

    /// p1's Nurse Cleaver facing p2's `field`.
    fn facing(field: Value) -> Scenario {
        jackioh_cards::register_all();
        scenario(json!({
            "p1": { "field": [{ "def": CLEAVER, "statsOverride": { "attack": 3, "health": 6 } }] },
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

    /// #8 Mr. Vanilla, a 4/4.
    const VANILLA: &str = "core-008";
    /// #30 Archivist, a 4/5.
    const ARCHIVIST: &str = "core-030";
    /// M #97.5 Bunker, a 5/10 with Armor 3, Can't attack and First Strike, 3× on the Unit across its lane.
    const BUNKER: &str = "meditative-097-5";
    /// M #52 Economic Anxiety: +3 attack to a Unit attacking a Unit that shares no tag with it.
    const ANXIETY: &str = "meditative-052";
    /// #43 Big Felinor, a 3/10 Felinor.
    const BIG_FELINOR: &str = "core-043";

    /// C+ #11 Anime Armor, a 3/3 whose own guard caps each hit on it at 1.
    const ANIME_ARMOR: &str = "classicplus-011";

    /// p1's `attacker`, given `attacker_keywords`, facing p2's `target`, given `target_keywords`: the
    /// state and the two units' ids.
    fn duel(
        attacker: &str,
        attacker_keywords: &[Keyword],
        target: &str,
        target_keywords: &[Keyword],
    ) -> (GameState, String, String) {
        jackioh_cards::register_all();
        let mut state = scenario(json!({ "p1": { "field": [attacker] }, "p2": { "field": [target] } }))
            .state()
            .clone();
        let attacker_id = active_units_of(&state, PlayerId::P1)[0].id.clone();
        let target_id = active_units_of(&state, PlayerId::P2)[0].id.clone();
        for (id, keywords) in [(&attacker_id, attacker_keywords), (&target_id, target_keywords)] {
            let unit = find_instance_mut(&mut state, id).expect("on the field");
            unit.granted_keywords.extend(keywords.iter().cloned());
        }
        (state, attacker_id, target_id)
    }

    /// Whether p1's `attacker`, given `attacker_keywords`, kills p2's `target`, given `target_keywords`,
    /// and survives, by `kills_and_survives`.
    fn kills_and_survives_with(
        attacker: &str,
        attacker_keywords: &[Keyword],
        target: &str,
        target_keywords: &[Keyword],
    ) -> bool {
        let (state, attacker_id, target_id) = duel(attacker, attacker_keywords, target, target_keywords);
        let attacker = find_instance(&state, &attacker_id).expect("the attacker");
        let target = find_instance(&state, &target_id).expect("the target");
        kills_and_survives(&state, attacker, target)
    }

    /// §4.4 steps 1 and 4: the Archivist's 4 kills a 4/4 and the 4 back leaves it standing, but a
    /// Divine Shield or Indestructible on the target, either one alone, means no kill.
    #[test]
    fn a_target_with_divine_shield_or_indestructible_is_no_kill() {
        assert!(kills_and_survives_with(ARCHIVIST, &[], VANILLA, &[]));
        for keyword in [Keyword::DivineShield, Keyword::Indestructible] {
            assert!(
                !kills_and_survives_with(ARCHIVIST, &[], VANILLA, std::slice::from_ref(&keyword)),
                "{keyword:?}"
            );
        }
    }

    /// §4.3: two 4/4s trade, unless the attacker alone strikes first; when both have First Strike they
    /// strike together and still trade.
    #[test]
    fn only_a_first_strike_the_target_lacks_survives_an_even_fight() {
        assert!(!kills_and_survives_with(VANILLA, &[], VANILLA, &[]));
        assert!(kills_and_survives_with(
            VANILLA,
            &[Keyword::FirstStrike],
            VANILLA,
            &[]
        ));
        assert!(!kills_and_survives_with(
            VANILLA,
            &[Keyword::FirstStrike],
            VANILLA,
            &[Keyword::FirstStrike]
        ));
    }

    /// §4.4 steps 1 and 4: an attacker with Divine Shield or Indestructible, either one alone, takes
    /// nothing from the strike back, so it survives the fight two 4/4s would trade in.
    #[test]
    fn an_attacker_with_divine_shield_or_indestructible_survives_an_even_fight() {
        for keyword in [Keyword::DivineShield, Keyword::Indestructible] {
            assert!(
                kills_and_survives_with(VANILLA, std::slice::from_ref(&keyword), VANILLA, &[]),
                "{keyword:?}"
            );
        }
    }

    /// §4.4 step 7: Poisonous destroys only when 1 or more was dealt, so a Poisonous hit the Bunker's
    /// Armor 3 takes whole kills nothing, and one past it kills the 5/10.
    #[test]
    fn a_poisonous_blow_kills_only_when_it_deals_damage() {
        jackioh_cards::register_all();
        let s = scenario(json!({ "p1": { "field": [VANILLA] }, "p2": { "field": [BUNKER] } }));
        let from = side_of(s.state(), s.card(VANILLA));
        let to = side_of(s.state(), s.card(BUNKER));
        assert_eq!(to.view.armor, 3);
        let absorbed = blow(&from, 3, true, &to);
        assert_eq!((absorbed.dealt, absorbed.kills), (0, false));
        let dealt = blow(&from, 4, true, &to);
        assert_eq!((dealt.dealt, dealt.kills), (1, true));
    }

    /// R1283: the attacker's strike on the Unit across its own lane is its attack times its lane
    /// multiplier, as `resolve_combat` reads it in a forced attack (the Bunker's Can't attack is skipped
    /// there, §4.2), and the strike back is the defender's attack times its own, 1.
    #[test]
    fn the_strike_across_a_bunkers_lane_is_multiplied() {
        jackioh_cards::register_all();
        let s = scenario(json!({
            "p1": { "field": [{ "def": BUNKER, "lane": 1 }] },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }] },
        }));
        let (bunker, vanilla) = (s.card(BUNKER), s.card(VANILLA));
        let state = s.state();
        let combat = exchange(
            state,
            bunker,
            &unit_view(state, bunker),
            vanilla,
            &unit_view(state, vanilla),
        );
        assert_eq!(lane_multiplier_for(state, bunker, vanilla), 3);
        assert_eq!((combat.hit, combat.strike_back, combat.splash), (15, 4, 5));
    }

    /// S+ #107 Fuse, a 2/5 Ranged Unit with Armor 3 and Pierce whose own attacks weigh 5× in its lane.
    const FUSE: &str = "stinkyscapeplus-107";

    /// R4156000: a Fuse that is attacked strikes back with its plain attack, as `resolve_combat` reads it:
    /// its sweep's weight is on its own attacks alone (R3554008), so the exchange the AI weighs an attack on
    /// it by takes 2 back, not 10.
    #[test]
    fn r4156000_an_attacked_fuse_strikes_back_with_its_plain_attack() {
        jackioh_cards::register_all();
        let s = scenario(json!({
            "p1": { "field": [{ "def": VANILLA, "lane": 1 }] },
            "p2": { "field": [{ "def": FUSE, "lane": 1 }] },
        }));
        let (vanilla, fuse) = (s.card(VANILLA), s.card(FUSE));
        let state = s.state();
        let combat = exchange(
            state,
            vanilla,
            &unit_view(state, vanilla),
            fuse,
            &unit_view(state, fuse),
        );
        assert_eq!(
            lane_multiplier_for(state, fuse, vanilla),
            5,
            "its own attack still sweeps"
        );
        assert_eq!((combat.hit, combat.strike_back), (4, 2));
    }

    /// R1120, §4.4 step 10: Economic Anxiety's +3 rides the strike and the Cleave splash alike (a
    /// Felinor attacking a Human, which shares no tag with it).
    #[test]
    fn the_combat_only_bonus_rides_the_strike_and_the_splash() {
        jackioh_cards::register_all();
        let s = scenario(json!({
            "p1": { "field": [BIG_FELINOR], "backrow": [ANXIETY] },
            "p2": { "field": [VANILLA] },
        }));
        let (felinor, vanilla) = (s.card(BIG_FELINOR), s.card(VANILLA));
        let state = s.state();
        let combat = exchange(
            state,
            felinor,
            &unit_view(state, felinor),
            vanilla,
            &unit_view(state, vanilla),
        );
        assert_eq!(attack_mod_for(state, felinor, vanilla).attack, 3);
        assert_eq!((combat.hit, combat.strike_back, combat.splash), (6, 4, 6));
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

    /// R2179005, §4.4 step 3: Anime Armor's own guard caps each hit on it at 1 after Armor, a Pierce hit's
    /// too, and a Poisonous hit of 1 still destroys it (step 7).
    #[test]
    fn r2179005_a_hit_on_anime_armor_deals_it_one() {
        jackioh_cards::register_all();
        let s = scenario(json!({ "p1": { "field": [VANILLA] }, "p2": { "field": [ANIME_ARMOR] } }));
        let mut from = side_of(s.state(), s.card(VANILLA));
        let to = side_of(s.state(), s.card(ANIME_ARMOR));
        assert_eq!((from.cap, to.cap), (None, Some(1)));
        let plain = blow(&from, 4, false, &to);
        assert_eq!((plain.dealt, plain.kills), (1, false));
        let poisonous = blow(&from, 4, true, &to);
        assert_eq!((poisonous.dealt, poisonous.kills), (1, true));
        from.pierce = true;
        assert_eq!(blow(&from, 4, false, &to).dealt, 1);
    }

    /// R2179005: a 4/4's 4 deals Anime Armor 1, so the attack is a chip of 1 that kills nothing (its 3
    /// back leaves the 4/4 standing): no tier 1 attack and no even trade.
    #[test]
    fn r2179005_a_four_four_into_anime_armor_is_no_free_kill() {
        let (state, vanilla, anime) = duel(VANILLA, &[], ANIME_ARMOR, &[]);
        let attacker = find_instance(&state, &vanilla).expect("the attacker");
        let trade = trade_value(&state, attacker, &anime, Prices::NONE);
        assert!(!trade.kills);
        assert!(
            (trade.value - AI_REPLY.chip_per_damage).abs() < 1e-9,
            "{}",
            trade.value
        );
        assert_eq!(attack_tier(&state, PlayerId::P1, &vanilla, &anime), 4);
        assert_eq!(even_trade(&state, &vanilla, &anime, Prices::NONE), None);
    }

    /// R2179005: a 4/4's strike back deals the attacking Anime Armor 1, so it survives; a Poisonous Anime
    /// Armor kills the 4/4 and walks away.
    #[test]
    fn r2179005_anime_armor_survives_the_strike_back_of_a_four_four() {
        let (state, anime_id, vanilla_id) = duel(ANIME_ARMOR, &[], VANILLA, &[]);
        let anime = find_instance(&state, &anime_id).expect("the attacker");
        let vanilla = find_instance(&state, &vanilla_id).expect("the target");
        let read = fight(&state, anime, vanilla);
        assert_eq!(
            blow(&read.t, read.combat.strike_back, read.t.poisonous, &read.a).dealt,
            1
        );
        assert!(!read.dies && !read.kills);
        assert!(kills_and_survives_with(
            ANIME_ARMOR,
            &[Keyword::Poisonous],
            VANILLA,
            &[]
        ));
    }

    /// The bodies `r2179005_the_trade_reads_agree_with_the_combat_forecast` sets against each other: a
    /// name, a card and the keywords granted it.
    fn bodies() -> Vec<(&'static str, &'static str, Vec<Keyword>)> {
        vec![
            ("Mr. Vanilla", VANILLA, vec![]),
            ("Anime Armor", ANIME_ARMOR, vec![]),
            ("Armor 2", VANILLA, vec![Keyword::Armor { n: 2 }]),
            ("Divine Shield", VANILLA, vec![Keyword::DivineShield]),
            ("Indestructible", VANILLA, vec![Keyword::Indestructible]),
            ("Poisonous", VANILLA, vec![Keyword::Poisonous]),
            ("Pierce", VANILLA, vec![Keyword::Pierce]),
            ("First Strike", VANILLA, vec![Keyword::FirstStrike]),
            ("Poisonous Anime Armor", ANIME_ARMOR, vec![Keyword::Poisonous]),
            (
                "First Strike Anime Armor",
                ANIME_ARMOR,
                vec![Keyword::FirstStrike],
            ),
            ("Pierce Anime Armor", ANIME_ARMOR, vec![Keyword::Pierce]),
            ("Armored Anime Armor", ANIME_ARMOR, vec![Keyword::Armor { n: 1 }]),
            (
                "Poisonous First Strike",
                VANILLA,
                vec![Keyword::Poisonous, Keyword::FirstStrike],
            ),
        ]
    }

    /// R2179005, R3100000: on every legal attack of one body on another, the static trader's kill and death
    /// (`trade_value`, `fight`) and the move order's tier 1 read (`kills_and_survives`) are the engine's
    /// combat forecast's, Anime Armor's cap among Armor, Divine Shield, Indestructible, Poisonous, Pierce and
    /// First Strike.
    #[test]
    fn r2179005_the_trade_reads_agree_with_the_combat_forecast() {
        let bodies = bodies();
        let mut checked = 0;
        for (a, attacker_card, attacker_keywords) in &bodies {
            for (t, target_card, target_keywords) in &bodies {
                let (state, attacker_id, target_id) =
                    duel(attacker_card, attacker_keywords, target_card, target_keywords);
                let attacker = find_instance(&state, &attacker_id).expect("the attacker");
                for entry in combat_forecast(&state, PlayerId::P1).attacks {
                    if entry.target_id.starts_with("hero-") {
                        continue;
                    }
                    assert_eq!(entry.target_id, target_id, "{a} into {t}");
                    let target = find_instance(&state, &entry.target_id).expect("the target");
                    assert_eq!(
                        trade_value(&state, attacker, &entry.target_id, Prices::NONE).kills,
                        entry.kills,
                        "{a} into {t}: trade_value's kill, {entry:?}"
                    );
                    assert_eq!(
                        fight(&state, attacker, target).dies,
                        !entry.survives,
                        "{a} into {t}: fight's death, {entry:?}"
                    );
                    assert_eq!(
                        kills_and_survives(&state, attacker, target),
                        entry.kills && entry.survives,
                        "{a} into {t}: kills_and_survives, {entry:?}"
                    );
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, bodies.len() * bodies.len());
    }

    /// Meditative #72 The Banisher, a 2/1 (Radiant 4/2 with Cleave) whose damage exiles the Unit it hits.
    const BANISHER: &str = "meditative-072";
    /// Classic+ #6 Wrong-House Attacker, whose Radiant face is a 2/2 with Rush, Lifesteal, Poisonous and Reborn.
    const WRONG_HOUSE: &str = "classicplus-006";

    /// A 5/5 Mr. Vanilla.
    fn five() -> Value {
        json!({ "def": VANILLA, "statsOverride": { "attack": 5, "health": 5 } })
    }

    /// R4065001: The Banisher's 2 back exiles the 5/5 that kills it, so the attack is no tier 1 kill, and the
    /// static trader prices it as losing the whole 5/5, its Reborn body included. A plain 2/1 is the free kill.
    #[test]
    fn r4065001_a_five_five_into_the_banisher_is_no_free_kill() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({ "p1": { "field": [five()] }, "p2": { "field": [BANISHER] } }));
        s.card_mut(VANILLA).granted_keywords.push(Keyword::Reborn);
        let (five_five, banisher) = (s.card(VANILLA).clone(), s.card(BANISHER).clone());
        let state = s.state();
        assert!(!kills_and_survives(state, &five_five, &banisher));
        assert_eq!(attack_tier(state, PlayerId::P1, &five_five.id, &banisher.id), 4);
        assert!(fight(state, &five_five, &banisher).dies);
        let trade = trade_value(state, &five_five, &banisher.id, Prices::NONE);
        assert!(trade.kills);
        let gain = kill_worth(state, &banisher, &AI_EVAL, Prices::NONE);
        let expected = gain - unit_worth(state, &five_five, &AI_EVAL, Prices::NONE);
        assert!(
            (trade.value - expected).abs() < 1e-9,
            "{} vs {expected}",
            trade.value
        );
        assert!(trade.value < gain - kill_worth(state, &five_five, &AI_EVAL, Prices::NONE));
        assert_eq!(even_trade(state, &five_five.id, &banisher.id, Prices::NONE), None);
        s.attack(&five_five, &banisher)
            .expect_in_zone(&five_five, "exile")
            .expect_in_zone(&banisher, "graveyard");

        let s = scenario(json!({
            "p1": { "field": [five()] },
            "p2": { "field": [{ "def": OOMEN, "statsOverride": { "attack": 2, "health": 1 } }] },
        }));
        assert!(kills_and_survives(s.state(), s.card(VANILLA), s.card(OOMEN)));
    }

    /// R4065001, §4.4 steps 1 to 4 and 7: The Banisher's blow exiles with any damage dealt, as a Poisonous one
    /// destroys, and nothing when a Divine Shield, Indestructible or Armor takes it; a cap of 1 lets 1 through.
    /// A Reborn 4/4 it exiles is priced whole, and the engine exiles it with no body back.
    #[test]
    fn r4065001_the_banishers_blow_exiles_only_what_it_damages() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({ "p1": { "field": [BANISHER] }, "p2": { "field": [VANILLA] } }));
        let from = side_of(s.state(), s.card(BANISHER));
        assert!(from.exiles && !side_of(s.state(), s.card(VANILLA)).exiles);
        let read = |change: fn(&mut Side)| {
            let mut to = side_of(s.state(), s.card(VANILLA));
            change(&mut to);
            let out = blow(&from, 2, false, &to);
            (out.dealt, out.kills, out.pops_shield)
        };
        assert_eq!(read(|_| {}), (2, true, false));
        assert_eq!(read(|to| to.shield = true), (0, false, true));
        assert_eq!(read(|to| to.indestructible = true), (0, false, false));
        assert_eq!(read(|to| to.view.armor = 2), (0, false, false));
        assert_eq!(read(|to| to.cap = Some(1)), (1, true, false));

        s.card_mut(VANILLA).granted_keywords.push(Keyword::Reborn);
        let (banisher, vanilla) = (s.card(BANISHER).clone(), s.card(VANILLA).clone());
        let state = s.state();
        let trade = trade_value(state, &banisher, &vanilla.id, Prices::NONE);
        let expected = unit_worth(state, &vanilla, &AI_EVAL, Prices::NONE)
            - kill_worth(state, &banisher, &AI_EVAL, Prices::NONE);
        assert!(trade.kills);
        assert!(
            (trade.value - expected).abs() < 1e-9,
            "{} vs {expected}",
            trade.value
        );
        s.attack(&banisher, &vanilla)
            .expect_in_zone(&vanilla, "exile")
            .expect_in_zone(&banisher, "graveyard");
        assert!(s.unit(PlayerId::P2, 1).is_none());
    }

    /// R4065001, R63: a Radiant Banisher's Cleave splash exiles each neighbour it damages, so its attack on the
    /// 4/4 between Reno and the Archivist kills by its splash, and the engine exiles all three.
    #[test]
    fn r4065001_a_radiant_banishers_splash_exiles_the_neighbours() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [{ "def": BANISHER, "radiant": true }] },
            "p2": { "field": [
                { "def": RENO, "lane": 1 },
                { "def": VANILLA, "lane": 2 },
                { "def": ARCHIVIST, "lane": 3 },
            ] },
        }));
        let banisher = s.card(BANISHER).clone();
        let [reno, vanilla, archivist] = [RENO, VANILLA, ARCHIVIST].map(|def| s.card(def).clone());
        let trade = trade_value(s.state(), &banisher, &vanilla.id, Prices::NONE);
        assert!(trade.kills && trade.splash_kills);
        s.attack(&banisher, &vanilla)
            .expect_in_zone(&reno, "exile")
            .expect_in_zone(&vanilla, "exile")
            .expect_in_zone(&archivist, "exile");
    }

    /// R4065000: an enemy 4/4 its Burn 5 destroys at its controller's next start could attack nothing on that
    /// turn, and the engine destroys it there; Burn 3 leaves it to swing. A Radiant Wrong-House Attacker's
    /// Reborn body comes back with Rush (R83), so it could still attack a Unit, never the hero.
    #[test]
    fn r4065000_a_unit_its_burn_destroys_first_could_attack_no_one_next_turn() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({ "p1": { "field": [OOMEN] }, "p2": { "field": [VANILLA] } }));
        let at_oomen = AttackTarget::Unit {
            instance: s.card(OOMEN).clone(),
        };
        let could = |s: &Scenario| could_attack_next_turn(s.state(), s.card(VANILLA), &at_oomen);
        assert!(could(&s));
        s.card_mut(VANILLA).counters.burn = Some(5);
        assert!(!could(&s));
        s.card_mut(VANILLA).counters.burn = Some(3);
        assert!(could(&s));
        s.card_mut(VANILLA).counters.burn = Some(5);
        let vanilla = s.card(VANILLA).clone();
        s.end_turn().expect_in_zone(&vanilla, "graveyard");

        let s = scenario(json!({
            "p1": { "field": [OOMEN] },
            "p2": { "field": [{ "def": WRONG_HOUSE, "radiant": true, "counters": { "burn": 5 } }] },
        }));
        let (attacker, oomen) = (s.card(WRONG_HOUSE), s.card(OOMEN).clone());
        let hero = AttackTarget::Hero { player: PlayerId::P1 };
        assert!(could_attack_next_turn(
            s.state(),
            attacker,
            &AttackTarget::Unit { instance: oomen }
        ));
        assert!(!could_attack_next_turn(s.state(), attacker, &hero));
    }

    /// S+ #42 Force Projector, a Field Spell: the first 5 damage its side would take each turn is nullified.
    const PROJECTOR: &str = "stinkyscapeplus-042";
    /// S+ #151 Volcanic Rounds, a 3/1: "Damage dealt Burns."
    const VOLCANIC: &str = "stinkyscapeplus-151";
    /// S+ #153 Noxious Rounds, a 1/1: "Damage dealt is Toxic."
    const NOXIOUS: &str = "stinkyscapeplus-153";
    /// A 3/3 with no text.
    const THREE_THREE: &str = "stand-in (1) 3/3";
    /// The life p1's hero starts `armed` boards at, below its starting life so a heal shows.
    const ARMED_LIFE: i32 = 20;

    /// p1's `attacker`, given `attacker_keywords`, facing p2's `target`, given `target_keywords`, as `duel`
    /// sets them, with a face-up Force Projector in p1's and p2's backrows as `pools` says and p1's hero at
    /// `ARMED_LIFE`: the scenario and the two units' ids.
    fn armed(
        attacker: Value,
        attacker_keywords: &[Keyword],
        target: Value,
        target_keywords: &[Keyword],
        pools: (bool, bool),
    ) -> (Scenario, String, String) {
        jackioh_cards::register_all();
        let mut p1 = json!({ "health": ARMED_LIFE, "field": [attacker] });
        let mut p2 = json!({ "field": [target] });
        for (side, pool) in [(&mut p1, pools.0), (&mut p2, pools.1)] {
            if pool {
                side["backrow"] = json!([{ "def": PROJECTOR, "faceUp": true }]);
            }
        }
        let mut s = scenario(json!({ "seed": "ai-pools-and-conversion", "p1": p1, "p2": p2 }));
        for player in [PlayerId::P1, PlayerId::P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        let attacker_id = active_units_of(s.state(), PlayerId::P1)[0].id.clone();
        let target_id = active_units_of(s.state(), PlayerId::P2)[0].id.clone();
        for (id, keywords) in [(&attacker_id, attacker_keywords), (&target_id, target_keywords)] {
            s.card_mut(id.as_str())
                .granted_keywords
                .extend(keywords.iter().cloned());
        }
        (s, attacker_id, target_id)
    }

    /// A Rounds card overridden to a 6/2, as its Radiant face is.
    fn rounds_six_two(rounds: &str) -> Value {
        json!({ "def": rounds, "statsOverride": { "attack": 6, "health": 2 } })
    }

    /// R4064000, §4.4 step 3b: p2's Force Projector soaks a blow of 4 on its 3/3 whole, and the 1 it would
    /// have left takes 1 of a blow of 3, which then kills nothing.
    #[test]
    fn r4064000_one_pool_soaks_two_blows_once() {
        let (s, attacker, target) = armed(json!(VANILLA), &[], json!(THREE_THREE), &[], (false, true));
        let from = side_of(s.state(), s.card(attacker.as_str()));
        let mut to = side_of(s.state(), s.card(target.as_str()));
        assert_eq!((from.pool, to.pool), (0, 5));
        let first = blow(&from, 4, false, &to);
        assert_eq!((first.dealt, first.soaked, first.kills), (0, 4, false));
        to.pool -= first.soaked;
        let second = blow(&from, 3, false, &to);
        assert_eq!((second.dealt, second.soaked, second.kills), (2, 1, false));
    }

    /// R4064001, §4.4 step 3a: a 6/2 Rounds unit's hit on a 3/3 is its Burn (or Toxic) 6, no damage: no kill
    /// in `blow`, `kills_and_survives` or `trade_value`, whose Lifesteal heals nothing, and the counter is
    /// worth something. A Divine Shield still takes the hit (step 1), and the engine plays it alike.
    #[test]
    fn r4064001_a_converted_blow_kills_nothing_and_heals_nothing() {
        for rounds in [VOLCANIC, NOXIOUS] {
            let (s, attacker, target) = armed(
                rounds_six_two(rounds),
                &[],
                json!(THREE_THREE),
                &[],
                (false, false),
            );
            let (state, a, t) = (s.state(), s.card(attacker.as_str()), s.card(target.as_str()));
            let read = blow(&side_of(state, a), 6, false, &side_of(state, t));
            assert_eq!(
                (read.dealt, read.kills, read.soaked, read.converted),
                (0, false, 0, 6),
                "{rounds}"
            );
            assert!(!kills_and_survives(state, a, t), "{rounds}");
            assert!(!trade_value(state, a, &target, Prices::NONE).kills, "{rounds}");

            let shielded = |keywords: &[Keyword]| {
                let (s, attacker, target) = armed(
                    rounds_six_two(rounds),
                    keywords,
                    json!(THREE_THREE),
                    &[],
                    (false, false),
                );
                trade_value(s.state(), s.card(attacker.as_str()), &target, Prices::NONE)
            };
            let plain = shielded(&[Keyword::DivineShield]);
            let lifesteal = shielded(&[Keyword::DivineShield, Keyword::Lifesteal]);
            assert!(!plain.kills && plain.value > 0.0, "{rounds}: {}", plain.value);
            assert!((lifesteal.value - plain.value).abs() < 1e-9, "{rounds}");

            let (s, attacker, target) = armed(
                rounds_six_two(rounds),
                &[],
                json!(THREE_THREE),
                &[Keyword::DivineShield],
                (false, false),
            );
            let popped = blow(
                &side_of(s.state(), s.card(attacker.as_str())),
                6,
                false,
                &side_of(s.state(), s.card(target.as_str())),
            );
            assert!(popped.pops_shield && popped.converted == 0, "{rounds}");

            let (mut s, attacker, target) = armed(
                rounds_six_two(rounds),
                &[Keyword::DivineShield, Keyword::Lifesteal],
                json!(THREE_THREE),
                &[],
                (false, false),
            );
            s.attack(attacker.as_str(), target.as_str());
            let struck = s.card(target.as_str());
            assert_eq!(unit_view(s.state(), struck).health, 3, "{rounds}");
            let counter = if rounds == VOLCANIC {
                jackioh_engine::burn_on(struck)
            } else {
                jackioh_engine::toxic_on(struck)
            };
            assert_eq!(counter, 6, "{rounds}");
            assert_eq!(
                s.state().players[PlayerId::P1].hero.health,
                ARMED_LIFE,
                "{rounds}"
            );
        }
    }

    /// R4064001, R4064000: against a hero at 6, a 6/2 Rounds unit's hit is no win but is worth the life
    /// its counter's ticks take; a 6/4 Mr. Vanilla's is the win, and behind p2's Force Projector it deals
    /// 1, as the engine deals it.
    #[test]
    fn r4064001_r4064000_face_value_reads_the_counter_and_the_pool() {
        const LIFE: i32 = 6;
        let at_six = |attacker: Value, pool: bool| {
            let (mut s, attacker, _) = armed(attacker, &[], json!(THREE_THREE), &[], (false, pool));
            s.state_mut().players[PlayerId::P2].hero.health = LIFE;
            (s, attacker)
        };
        for rounds in [VOLCANIC, NOXIOUS] {
            let (s, attacker) = at_six(rounds_six_two(rounds), false);
            let value = face_value(s.state(), s.card(attacker.as_str()), PlayerId::P2, false);
            assert!(value > 0.0 && value < AI_EVAL.win, "{rounds}: {value}");
        }
        let vanilla = json!({ "def": VANILLA, "statsOverride": { "attack": 6, "health": 4 } });
        let (s, attacker) = at_six(vanilla.clone(), false);
        let value = face_value(s.state(), s.card(attacker.as_str()), PlayerId::P2, false);
        assert!((value - AI_EVAL.win).abs() < 1e-9, "{value}");

        let (mut s, attacker) = at_six(vanilla, true);
        let value = face_value(s.state(), s.card(attacker.as_str()), PlayerId::P2, false);
        assert!((value - AI_REPLY.face_per_damage).abs() < 1e-9, "{value}");
        s.attack(attacker.as_str(), "hero");
        assert_eq!(s.state().players[PlayerId::P2].hero.health, LIFE - 1);
    }

    /// The bodies `r4064000_r4064001_the_trade_reads_agree_with_the_combat_forecast` sets against each
    /// other: a name, a card and the keywords granted it.
    fn pool_bodies() -> Vec<(&'static str, &'static str, Vec<Keyword>)> {
        vec![
            ("Mr. Vanilla", VANILLA, vec![]),
            ("Volcanic Rounds", VOLCANIC, vec![]),
            ("Noxious Rounds", NOXIOUS, vec![]),
            ("Divine Shield", VANILLA, vec![Keyword::DivineShield]),
            ("Poisonous", VANILLA, vec![Keyword::Poisonous]),
            ("First Strike", VANILLA, vec![Keyword::FirstStrike]),
            ("Indestructible", VANILLA, vec![Keyword::Indestructible]),
            (
                "First Strike Volcanic Rounds",
                VOLCANIC,
                vec![Keyword::FirstStrike],
            ),
            ("Poisonous Noxious Rounds", NOXIOUS, vec![Keyword::Poisonous]),
        ]
    }

    /// R4064000, R4064001: on every legal attack of one body on another, with or without a Force Projector
    /// on either side, the static trader's kill and death (`trade_value`, `fight`) and the move order's tier
    /// 1 read (`kills_and_survives`) are the engine's combat forecast's (R4032000), the damage pools and the
    /// conversions among Divine Shield, Poisonous, First Strike and Indestructible.
    #[test]
    fn r4064000_r4064001_the_trade_reads_agree_with_the_combat_forecast() {
        let bodies = pool_bodies();
        let settings = [(false, false), (true, false), (false, true), (true, true)];
        let mut checked = 0;
        for pools in settings {
            for (a, attacker_card, attacker_keywords) in &bodies {
                for (t, target_card, target_keywords) in &bodies {
                    let (s, attacker_id, target_id) = armed(
                        json!(attacker_card),
                        attacker_keywords,
                        json!(target_card),
                        target_keywords,
                        pools,
                    );
                    let state = s.state();
                    let attacker = find_instance(state, &attacker_id).expect("the attacker");
                    for entry in combat_forecast(state, PlayerId::P1).attacks {
                        if entry.target_id.starts_with("hero-") {
                            continue;
                        }
                        assert_eq!(entry.target_id, target_id, "{a} into {t}, {pools:?}");
                        let target = find_instance(state, &entry.target_id).expect("the target");
                        assert_eq!(
                            trade_value(state, attacker, &entry.target_id, Prices::NONE).kills,
                            entry.kills,
                            "{a} into {t}, {pools:?}: trade_value's kill, {entry:?}"
                        );
                        assert_eq!(
                            fight(state, attacker, target).dies,
                            !entry.survives,
                            "{a} into {t}, {pools:?}: fight's death, {entry:?}"
                        );
                        assert_eq!(
                            kills_and_survives(state, attacker, target),
                            entry.kills && entry.survives,
                            "{a} into {t}, {pools:?}: kills_and_survives, {entry:?}"
                        );
                        checked += 1;
                    }
                }
            }
        }
        assert_eq!(checked, settings.len() * bodies.len() * bodies.len());
    }

    /// R4171000: the seeded random-policy games the agreement test below plays, and its floors: the
    /// Unit-target attacks it judged, the distinct cards (`def_id`s) at either end of one, and the
    /// attacks a First Strike defender stopped whose blow would otherwise deal damage or pop a shield.
    const FORECAST_AGREEMENT_GAMES: usize = 16;
    const FORECAST_AGREEMENT_MIN_ATTACKS: usize = 1000;
    const FORECAST_AGREEMENT_MIN_CARDS: usize = 150;
    const FORECAST_AGREEMENT_MIN_STOPPED: usize = 5;

    /// R4171000, R3100000: on every state of `FORECAST_AGREEMENT_GAMES` games of SPEC §10.7's random
    /// policy, decks dealt from every set, every Unit-target entry of either seat's combat forecast is the
    /// static trader's `fight` (what the attacker deals, up to the target's health for a Trample hit,
    /// §4.4 step 5; what it takes; the kill; the survival; the pop) and the move order's
    /// `kills_and_survives`.
    #[test]
    fn r4171000_the_trade_reads_agree_with_the_combat_forecast_on_seeded_games() {
        jackioh_cards::register_all();
        let (mut judged, mut stopped) = (0, 0);
        let mut met: IndexSet<String> = IndexSet::new();
        for game in 0..FORECAST_AGREEMENT_GAMES {
            let seed = format!("forecast-agreement:{game}");
            let deck = |seat: PlayerId| {
                build_ai_deck(
                    &mut Rng::new(&format!("{seed}:deck:{}", seat.as_str()), 0),
                    DECK_SIZE,
                    &AiDeckOptions {
                        banned: Some(vec![]),
                        gated_sets: Some(vec![]),
                        ..AiDeckOptions::default()
                    },
                )
            };
            let config = MatchConfig {
                seed: seed.clone(),
                decks: (deck(PlayerId::P1), deck(PlayerId::P2)),
                handicaps: None,
                controllers: PerPlayer {
                    p1: SeatController::Random,
                    p2: SeatController::Random,
                },
                max_actions: None,
                setup: MatchSetup::default(),
            };
            let mut states: Vec<GameState> = Vec::new();
            {
                let mut hooks = MatchHooks {
                    after_action: Some(Box::new(
                        |_before: &GameState, after: &GameState, _seat: PlayerId, _action: &ActionBody| {
                            states.push(after.clone());
                        },
                    )),
                    ..MatchHooks::default()
                };
                play_match(&config, &mut hooks);
            }
            for (index, state) in states.iter().enumerate() {
                for seat in [PlayerId::P1, PlayerId::P2] {
                    for entry in combat_forecast(state, seat).attacks {
                        if entry.target_id.starts_with("hero-") {
                            continue;
                        }
                        let attacker = find_instance(state, &entry.attacker_id).expect("the attacker");
                        let target = find_instance(state, &entry.target_id).expect("the target");
                        met.insert(attacker.def_id.clone());
                        met.insert(target.def_id.clone());
                        let read = fight(state, attacker, target);
                        // §4.4 step 5: a Trample hit deals the unit no more than its health.
                        let dealt = if has_keyword(&read.a.view.keywords, KeywordKind::Trample) {
                            read.out.dealt.min(read.t.view.health.max(0))
                        } else {
                            read.out.dealt
                        };
                        let ours = (
                            dealt,
                            read.back.dealt,
                            read.kills,
                            !read.dies,
                            read.out.pops_shield,
                            kills_and_survives(state, attacker, target),
                        );
                        let theirs = (
                            entry.dealt,
                            entry.taken,
                            entry.kills,
                            entry.survives,
                            entry.pops,
                            entry.kills && entry.survives,
                        );
                        assert_eq!(
                            ours,
                            theirs,
                            "{seed}, after action {index}: {} ({}, radiant {}) into {} ({}, radiant {}); \
                             (dealt, taken, kills, survives, pops, kills_and_survives), the AI's then the \
                             forecast's; {judged} attacks judged, {} cards met so far",
                            attacker.id,
                            attacker.def_id,
                            attacker.radiant,
                            target.id,
                            target.def_id,
                            target.radiant,
                            met.len(),
                        );
                        if !read.lands {
                            let unstopped = blow(&read.a, read.combat.hit, read.combat.poisonous, &read.t);
                            if unstopped.dealt > 0 || unstopped.pops_shield {
                                stopped += 1;
                            }
                        }
                        judged += 1;
                    }
                }
            }
        }
        let counts = format!(
            "{judged} attacks judged, {} cards met, {stopped} stopped blows",
            met.len()
        );
        assert!(judged >= FORECAST_AGREEMENT_MIN_ATTACKS, "{counts}");
        assert!(met.len() >= FORECAST_AGREEMENT_MIN_CARDS, "{counts}");
        assert!(stopped >= FORECAST_AGREEMENT_MIN_STOPPED, "{counts}");
    }
}
