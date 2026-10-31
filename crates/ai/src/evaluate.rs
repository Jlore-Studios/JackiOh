//! The AI's static evaluation (SPEC §9.9): hero health, armor, board stats and keywords through
//! §10.4's layers, hand, library, enemy face damage next turn, and the turn cap. Evaluates public state
//! banked items (#1086, #1143), refreshes and riders (#1224), Stack piles (#1506), face-down cost
//! visibility (#1602), modifiers and death prices (#2424). Reads only what the seat may know.

use std::cell::Cell;

use jackioh_engine::config::{
    CALL_TO_CHAOS_RADIANT_EFFECTS, DUOS_OWN_HEALTH, FIRST_GRADE, HERO_HEALTH, JADE_ASCEND_AT, JADE_BEAUTY_AT,
    TURN_CAP_PLAYER_TURNS,
};
use jackioh_engine::effects::{DELAYED_DESTROY_HOOK, DELAYED_DISCARD_HAND_HOOK};
use jackioh_engine::{
    ActionBody, AltWinCondition, AttackHealth, AttackTarget, CardDef, CardInstance, CardType, CostOptions,
    GameState, Grant, Keyword, KeywordKind, ModifierKind, PLAYER_IDS, PlayerId, PlayerState, Position, Row,
    SandboxOp, SeatOutcome, Slumber, SpawnZone, Tag, UnitView, ZoneName, acting_with_flag,
    active_brittle_count, active_units_of, animated_with_stats, armor_of, attack_restriction,
    attacks_per_turn, backrow_is_public, board_totals, cannot_attack, card_at, card_keywords,
    card_type_of_face, credit_lapsing, death_hook_of, effective_cost, effective_life, find_def,
    find_instance, first_entry_zone, frozen_next_turn, has_keyword, heal_multiplier_of, hero_armor_of,
    is_temporary_card, lost_refreshes_left, modifier_is_live, outcome_for, owed_mana_of, own_cost, pierces,
    pile_at, query_cost, reset_instance, slots_of, stats_with_buffs, subsystems, takes_strike_back, unit_has,
    unit_view, units_on_field_of, why_sandbox_refused,
};
use serde::{Deserialize, Serialize};

use crate::candidates::{Side, blow, cleaved_neighbours, side_of};
use crate::config::{AI_DEATH_PROBE_NODES, AI_EVAL, AI_EXTRA_TURN_CARDS, EvalWeights};
use crate::simulate::{create_sub_counter, simulate};
use crate::types::NodeCounter;

/// The life `player` started the game with, where the hero value and `closing` are anchored (R964000):
/// HERO_HEALTH in a 1v1 game; in a Duos duel the seat's own HP and its team's pool at its cap (§13.4;
/// every team's own HP is DUOS_OWN_HEALTH, §13.7). Both teams' life is public (§13.6).
pub fn starting_life(state: &GameState, player: PlayerId) -> i32 {
    match state.players[player].duos {
        Some(life) => DUOS_OWN_HEALTH + life.pool_cap,
        None => HERO_HEALTH,
    }
}

/// A hero's life (`effective_life`, R964000), concave: heroHealth × sqrt(h × start), `start` its
/// `starting_life`, so a full hero is worth its life: 30 in a 1v1 game.
fn hero_value(health: i32, start: i32, w: &EvalWeights) -> f64 {
    if health <= 0 {
        return -w.win;
    }
    w.hero_health * (f64::from(health) * f64::from(start)).sqrt()
}

/// §4.4 step 8: what healing `player`'s hero by `amount` is worth on `hero_value`'s curve, the more the
/// lower its life, its healing multiplier included (R1180003). A hero's heal has no cap (§6.3).
pub(crate) fn heal_worth(state: &GameState, player: PlayerId, amount: i32, w: &EvalWeights) -> f64 {
    let life = effective_life(state, player);
    if amount <= 0 || life <= 0 {
        return 0.0;
    }
    let start = starting_life(state, player);
    let healed = amount.saturating_mul(heal_multiplier_of(state, player));
    hero_value(life.saturating_add(healed), start, w) - hero_value(life, start, w)
}

/// E8, R413: whether a hero heal is replaced by Pierce damage: a live `HealToDamage`
/// modifier (R169) or face-up converter (R33/R1487000; hidden converters unmodeled).
pub(crate) fn heal_converted(state: &GameState, player: PlayerId) -> bool {
    PLAYER_IDS
        .into_iter()
        .filter(|other| *other != player)
        .any(|other| {
            state.players[other].mods.iter().any(|modifier| {
                matches!(modifier.kind, ModifierKind::HealToDamage { .. })
                    && modifier_is_live(state, modifier)
            }) || acting_with_flag(state, other, |flags| flags.heal_to_damage)
        })
}

/// §10.8, R33, R686: whether `seat` may read this backrow card, by the rule `view_for` reads
/// (`backrow_is_public`). A placeholder never is, and is asked first: the rule reads the definition.
fn readable_by(state: &GameState, card: &CardInstance, seat: PlayerId) -> bool {
    find_def(Some(state), &card.def_id).is_some() && backrow_is_public(state, card, seat)
}

/// R351: public backrow cost (`cost_override` or `effective_cost`), preserved across determinizations (R762).
fn shown_cost(state: &GameState, card: &CardInstance) -> i32 {
    if find_def(Some(state), &card.def_id).is_none() {
        return card.cost_override.unwrap_or(0);
    }
    effective_cost(state, card, CostOptions::default())
}

/// B3.3: the share of itself a card keeps under its Brittle count (`active_brittle_count`), 1 with none.
fn brittle_share(card: &CardInstance, w: &EvalWeights) -> f64 {
    match active_brittle_count(card) {
        None => 1.0,
        Some(count) => 1.0 - w.brittle_discount / f64::from(count.max(1)),
    }
}

/// Whether the unit may attack at all: no "Can't attack" keyword, no E35 status bars it, and it is
/// not Frozen through its controller's next turn (R1174000: one that thaws at this turn's cleanup
/// swings next turn).
pub(crate) fn can_swing(state: &GameState, unit: &CardInstance, view: &UnitView) -> bool {
    !has_keyword(&view.keywords, KeywordKind::CantAttack)
        && !cannot_attack(state, unit)
        && !frozen_next_turn(state, unit)
}

/// R49: a unit with Deft, read through the layers, in Defense Position. Its switch to Attack spends
/// only the switch's exertion, so it can switch and still attack in the same turn.
pub(crate) fn deft_in_defense(view: &UnitView) -> bool {
    view.position == Position::Def && has_keyword(&view.keywords, KeywordKind::Deft)
}

/// R2424000: what a unit's Death leaves its controller, as a decision's root priced it (`death_prices`),
/// and the card it prices: its definition, its face, its granted Deaths and its stat override (Trigor's
/// Death reads its base stats), so the same card the search moves or makes later reads the same price.
#[derive(Clone, Debug, PartialEq)]
pub struct DeathPrice {
    pub def_id: String,
    pub radiant: bool,
    pub grants: Vec<Grant>,
    pub stats_override: Option<AttackHealth>,
    pub price: f64,
}

impl DeathPrice {
    /// The price `card` would hold, at 0 until it is probed.
    fn key(card: &CardInstance) -> Self {
        Self {
            def_id: card.def_id.clone(),
            radiant: card.radiant,
            grants: card.grants.clone().unwrap_or_default(),
            stats_override: card.stats_override,
            price: 0.0,
        }
    }

    fn is_for(&self, card: &CardInstance) -> bool {
        self.def_id == card.def_id
            && self.radiant == card.radiant
            && self.stats_override == card.stats_override
            && self.grants.as_slice() == card.grants.as_deref().unwrap_or_default()
    }
}

// CLAUDE.md rule 4's scoped exception (R2424000): the Death prices of the decision in flight on this
// thread, held by `DeathPricing` for that one decision and put back after it, so every `unit_worth`
// the search makes reads them without a parameter through every scorer.
thread_local! {
    static DEATH_PRICES: Cell<Vec<DeathPrice>> = const { Cell::new(Vec::new()) };
}

/// R2424000: `prices` held on this thread until the guard drops, then the prices held before put back,
/// a panic included.
pub(crate) struct DeathPricing(Vec<DeathPrice>);

impl DeathPricing {
    pub(crate) fn hold(prices: Vec<DeathPrice>) -> Self {
        Self(DEATH_PRICES.with(|cell| cell.replace(prices)))
    }
}

impl Drop for DeathPricing {
    fn drop(&mut self) {
        let before = std::mem::take(&mut self.0);
        DEATH_PRICES.with(|cell| cell.set(before));
    }
}

/// R2424000: `run` with `prices` held on this thread for that call alone, as a decision holds the ones
/// its root priced. The prices held before are put back after it, a panic included.
pub fn with_death_prices<T>(prices: Vec<DeathPrice>, run: impl FnOnce() -> T) -> T {
    let _held = DeathPricing::hold(prices);
    run()
}

/// R2424000: what `unit`'s Death leaves its controller, by the prices held (`with_death_prices`), while
/// its Death is live (`death_hook_of`: a Vanilla unit has none). 0 outside a decision, for a card its
/// root did not price, and at a zero `death` weight (no reading).
fn death_price(state: &GameState, unit: &CardInstance, w: &EvalWeights) -> f64 {
    if w.death == 0.0 {
        return 0.0;
    }
    let price = DEATH_PRICES.with(|cell| {
        let prices = cell.take();
        let price = prices
            .iter()
            .find(|entry| entry.is_for(unit))
            .map(|entry| entry.price);
        cell.set(prices);
        price
    });
    match price {
        Some(price) if death_hook_of(state, unit).is_some() => price,
        _ => 0.0,
    }
}

/// One unit's worth on the board: its stats through the layers and its keywords (AI_EVAL), Spell
/// Damage per point, all discounted by its Brittle count, and `death` of what its Death leaves behind
/// (R2424000).
pub fn unit_worth(state: &GameState, unit: &CardInstance, w: &EvalWeights) -> f64 {
    let view = unit_view(state, unit);
    let weights = &w.keyword;
    let mut value = w.health * f64::from(view.health) + w.armor_point * f64::from(view.armor);
    if can_swing(state, unit, &view) {
        // §4.1: a Defense-Position unit cannot attack until it spends a turn's exertion switching back,
        // though it still strikes back in full, so only part of its attack counts. With
        // `deftDefenseAttacks` a Deft unit keeps it all: it can switch back and attack in one turn (R49).
        let deft = w.deft_defense_attacks && deft_in_defense(&view);
        let share = if view.position == Position::Def && !deft {
            w.defense_attack_share
        } else {
            1.0
        };
        value += w.attack * f64::from(view.attack) * share;
    }
    for keyword in &view.keywords {
        if let Some(weight) = weights.of(keyword.kind()) {
            value += weight;
        }
        if let Keyword::SpellDamage { n } = keyword {
            value += w.spell_damage * f64::from(*n);
        }
    }
    // §4.1: Defense Position grants (Taunt/Armor) soak damage counted by `face_threat`,
    // so here they count for `position_grants`.
    if view.position == Position::Def {
        let taunts = has_keyword(&view.keywords, KeywordKind::Taunt);
        if !w.defense_own_taunt_kept {
            if taunts {
                value -= (1.0 - w.position_grants) * (weights.taunt + w.armor_point);
            }
        } else {
            if taunts && !has_keyword(&card_keywords(state, unit), KeywordKind::Taunt) {
                value -= (1.0 - w.position_grants) * weights.taunt;
            }
            if view.armor > 0 {
                value -= (1.0 - w.position_grants) * w.armor_point;
            }
        }
    }
    // §10.1: the counters on the card are worth their own, outside the Brittle share — Fed Fauci's
    // Plague Counters are mana it already banked, not part of the body that crumbles. R2424000: so is
    // what its Death leaves, since Brittle's end is a death too.
    value * brittle_share(unit, w)
        + w.plague_counter * f64::from(unit.counters.plague.unwrap_or(0))
        + w.death * death_price(state, unit, w)
}

/// B3.4, R65: the crystals the card's own `costMod` moves its cost by (`own_cost`, floored at 0 as a
/// price is): +1 for a Degrade's dearer card, −1 for an Upgrade's cheaper one, 0 for an X-cost card.
fn cost_shift(state: &GameState, card: &CardInstance) -> i32 {
    if card.cost_mod == 0 {
        return 0;
    }
    match own_cost(state, card) {
        None => 0,
        Some(own) => own.max(0) - (own - card.cost_mod).max(0),
    }
}

/// #1143: a Unit's worth off its own stats and keywords by the weights `unit_worth` reads a view with,
/// none of the field's layers, position or damage: attack (none with "Can't attack"), health, Armor,
/// each keyword and Spell Damage per point.
fn face_worth(attack: i32, health: i32, keywords: &[Keyword], w: &EvalWeights) -> f64 {
    let mut value = w.health * f64::from(health) + w.armor_point * f64::from(armor_of(keywords));
    if !has_keyword(keywords, KeywordKind::CantAttack) {
        value += w.attack * f64::from(attack);
    }
    for keyword in keywords {
        if let Some(weight) = w.keyword.of(keyword.kind()) {
            value += weight;
        }
        if let Keyword::SpellDamage { n } = keyword {
            value += w.spell_damage * f64::from(*n);
        }
    }
    value
}

/// #1143: what the face an own hand card will enter with is worth over its printed base face
/// (`face_worth`): the running face (§5.2) with the buffs and tuning it carries onto the field (§10.4
/// layers 1 and 4, `stats_with_buffs`) and its own keywords, granted ones included (`card_keywords`).
/// 0 unless both its faces are Units (B2.7: a face may print its own type).
fn face_uplift(state: &GameState, card: &CardInstance, def: &CardDef, w: &EvalWeights) -> f64 {
    let unit = |radiant| card_type_of_face(state, &card.def_id, radiant) == CardType::Unit;
    if !unit(false) || !unit(true) {
        return 0.0;
    }
    let stats = stats_with_buffs(state, card);
    let entering = face_worth(stats.attack, stats.max_health, &card_keywords(state, card), w);
    let base = &def.base;
    let printed = face_worth(
        base.attack.unwrap_or(0),
        base.health.unwrap_or(0),
        &base.keywords,
        w,
    );
    entering - printed
}

/// An own hand card, a Temporary one (R637) at `temporary_discount` off, a Radiant one with
/// `radiant_in_hand` for its Radiant text, and one with two Unit faces with `radiant_face_share` of
/// what the face it will enter with adds to its printed one (`face_uplift`, #1143).
pub(crate) fn hand_card_value(state: &GameState, card: &CardInstance, w: &EvalWeights) -> f64 {
    doomed_hand_card_value(state, card, false, w)
}

/// `hand_card_value`, #1722: while a delayed discard of the hand waits (`doomed`, `discards_hand`), the
/// card keeps the smaller of Temporary's share and `doomed_hand_discount`'s.
fn doomed_hand_card_value(state: &GameState, card: &CardInstance, doomed: bool, w: &EvalWeights) -> f64 {
    let def = find_def(Some(state), &card.def_id);
    let cost = match def {
        None => w.opponent_hand_cost,
        Some(def) => f64::from(query_cost(def)),
    };
    let radiant = match def {
        _ if !card.radiant => 0.0,
        // R423: a Radiant Call to Chaos rolls CALL_TO_CHAOS_RADIANT_EFFECTS where its base face rolls one.
        Some(def) if def.tags.contains(&Tag::CallToChaos) => w.radiant_in_hand.max(
            w.radiant_chaos_effect
                * f64::from(CALL_TO_CHAOS_RADIANT_EFFECTS - 1)
                * w.hand_per_cost
                * cost.min(w.hand_cost_cap),
        ),
        _ => w.radiant_in_hand,
    };
    let shift = if def.is_none() {
        0.0
    } else {
        w.hand_cost_delta * f64::from(cost_shift(state, card))
    };
    // #1143: no reading at a zero share (the frozen baseline's).
    let uplift = match def {
        Some(def) if w.radiant_face_share != 0.0 => w.radiant_face_share * face_uplift(state, card, def, w),
        _ => 0.0,
    };
    // R637: `is_temporary_card` reads the definition, so a card without one is never asked.
    let temporary = if def.is_some() && is_temporary_card(state, card) {
        1.0 - w.temporary_discount
    } else {
        1.0
    };
    // #1722: a hand its owner's delayed discard will empty loses what a Temporary card does, once.
    let kept = if doomed {
        temporary.min(1.0 - w.doomed_hand_discount)
    } else {
        temporary
    };
    // #1886: a card that owes an extra turn once cast (R846).
    let turn = if AI_EXTRA_TURN_CARDS.contains(&card.def_id.as_str()) {
        w.extra_turn_in_hand * w.extra_turn
    } else {
        0.0
    };
    (w.hand_card + w.hand_per_cost * cost.min(w.hand_cost_cap) + radiant + uplift + turn - shift)
        * brittle_share(card, w)
        * kept
}

/// R962: the steps a Jade Counter at `jade` has climbed toward the next threshold it has not crossed.
/// It only rises (R961): toward JADE_BEAUTY_AT, then toward JADE_ASCEND_AT, then none.
fn jade_steps(jade: i32) -> i32 {
    if jade < JADE_BEAUTY_AT {
        jade
    } else if jade < JADE_ASCEND_AT {
        jade - JADE_BEAUTY_AT
    } else {
        0
    }
}

/// R844: the crystals `side` will not get from the refreshes it still has to lose past this turn's
/// (one lost this turn is already in its current mana). Each is priced at `mana.max`, not the coming
/// turn's maximum, which reads the handicap (`max_mana_for`; R180); the first also loses the rider
/// it spends, floored at 0 as a refresh is.
fn lost_crystals(side: &PlayerState) -> i32 {
    let left = lost_refreshes_left(side);
    if left == 0 {
        return 0;
    }
    (side.mana.max + side.mana.next_turn_mod).max(0) + (left - 1) * side.mana.max
}

/// §2.3: the crystals `side`'s next-turn rider moves its coming refresh by, no lower than a refresh
/// of 0, and none while that refresh is lost, which spends it (R844; `lost_crystals` counts it).
fn rider_crystals(side: &PlayerState) -> i32 {
    if lost_refreshes_left(side) > 0 {
        return 0;
    }
    side.mana.next_turn_mod.max(-side.mana.max)
}

/// R844, §2.3: what `side`'s coming refreshes will or will not give, both public badges (R169,
/// R185): its rider for or against it, the refreshes it still has to lose against it.
fn coming_refreshes(side: &PlayerState, w: &EvalWeights) -> f64 {
    w.next_turn_mana * f64::from(rider_crystals(side)) - w.lost_refresh * f64::from(lost_crystals(side))
}

/// R844, §2.3: `coming_refreshes` for `seat` less its enemy's, the share of `evaluate` the coming
/// refreshes make up and spend as they run; `reply_score` takes it from the line's end.
pub fn coming_mana(state: &GameState, seat: PlayerId, w: &EvalWeights) -> f64 {
    coming_refreshes(&state.players[seat], w) - coming_refreshes(&state.players[seat.opponent()], w)
}

/// R437: `unit_worth` of marked enemy units owned by `player`'s delayed effects (R185). Excludes
/// hand marks (R1141), count-downs (#1995), and Indestructible targets spared by destroy (Classic #20, R46).
fn marked_enemy_worth(state: &GameState, player: PlayerId, w: &EvalWeights) -> f64 {
    let Some(marks) = &state.marks else {
        return 0.0;
    };
    let enemy = active_units_of(state, player.opponent());
    let mut counted: Vec<&str> = Vec::new();
    let mut worth = 0.0;
    for mark in marks {
        let Some(entry) = state
            .delayed
            .iter()
            .find(|entry| entry.id == mark.delayed_id && entry.owner == player)
        else {
            continue;
        };
        if counted.contains(&mark.instance_id.as_str()) {
            continue;
        }
        if let Some(unit) = enemy.iter().find(|unit| unit.id == mark.instance_id) {
            if entry.resume.hook == DELAYED_DESTROY_HOOK && unit_has(state, unit, KeywordKind::Indestructible)
            {
                continue;
            }
            counted.push(mark.instance_id.as_str());
            worth += unit_worth(state, unit, w);
        }
    }
    worth
}

/// B5 E27 (Classic #37 Last Hurrah): whether a delayed discard of `player`'s whole hand waits. The
/// engine runs `DELAYED_DISCARD_HAND_HOOK` as its owner and discards that owner's hand. Every delayed
/// entry `redact` keeps is public (R185).
fn discards_hand(state: &GameState, player: PlayerId) -> bool {
    state
        .delayed
        .iter()
        .any(|entry| entry.owner == player && entry.resume.hook == DELAYED_DISCARD_HAND_HOOK)
}

/// R458: how many rest-of-game start-of-turn effects `side` holds.
fn start_of_turn_effects(side: &PlayerState) -> i32 {
    side.mods
        .iter()
        .filter(|held| matches!(held.kind, ModifierKind::StartOfTurnEffect { .. }))
        .count() as i32
}

/// R848, R849: how far `player` has come toward one held alternative win, in [0, 1]: the measure
/// `condition_met` reads (the hero's health, the graveyard's count, the lesser of the acting Units'
/// two totals, `board_totals`) over the threshold. A threshold of 0 or less is met.
fn alt_win_progress(state: &GameState, player: PlayerId, condition: AltWinCondition, threshold: i32) -> f64 {
    if threshold <= 0 {
        return 1.0;
    }
    let now = match condition {
        AltWinCondition::Health => state.players[player].hero.health,
        AltWinCondition::Graveyard => state.players[player].graveyard.len() as i32,
        AltWinCondition::Board => {
            let (attack, health) = board_totals(state, player);
            attack.min(health)
        }
    };
    (f64::from(now) / f64::from(threshold)).clamp(0.0, 1.0)
}

/// R848, R169: `alt_win` times the square of the progress of `player`'s closest held alternative win.
/// Any one met wins, so the others add nothing. Both seats see each condition and its progress (its
/// badge), so it counts for its holder and against the holder's enemy alike.
fn alt_win_worth(state: &GameState, player: PlayerId, w: &EvalWeights) -> f64 {
    let closest = state.players[player]
        .mods
        .iter()
        .filter_map(|held| match &held.kind {
            ModifierKind::AltWin { condition, threshold } => {
                Some(alt_win_progress(state, player, *condition, *threshold))
            }
            _ => None,
        })
        .fold(0.0, f64::max);
    w.alt_win * closest * closest
}

/// §3.2, R13: worth of dormant cards under Stack piles. Own unit piles read card by card;
/// other piles count by `dormant_card` (R447, R185).
fn dormant_worth(state: &GameState, player: PlayerId, seat: PlayerId, w: &EvalWeights) -> f64 {
    let side = &state.players[player];
    let beneath = side.units.iter().flatten().flat_map(|pile| pile.iter().skip(1));
    let buried = side.backrow_piles.iter().flatten().map(Vec::len).sum::<usize>();
    let units = if player != seat {
        beneath.count() as f64 * w.dormant_card
    } else if w.dormant_share != 0.0 {
        beneath
            .map(|card| w.dormant_share * unit_worth(state, card, w))
            .sum::<f64>()
    } else {
        0.0
    };
    units + buried as f64 * w.dormant_card
}

/// R2380000: what a Dormant card on the field (§6.1, R1839000) is worth: the `unit_worth` of the same
/// card awake, `slumber_turn_share` for each turn left on its countdown, or `slumber_waiting_share`
/// when it has none and waits for an effect (no call at a zero share). Its status and countdown are
/// on both seats' views (R185), so the seat prices its enemy's alike.
fn slumber_worth(state: &GameState, unit: &CardInstance, slumber: &Slumber, w: &EvalWeights) -> f64 {
    let share = match slumber.turns {
        Some(turns) => w.slumber_turn_share.powi(turns.max(0)),
        None => w.slumber_waiting_share,
    };
    if share == 0.0 {
        return 0.0;
    }
    let mut awake = unit.clone();
    awake.slumber = None;
    share * unit_worth(state, &awake, w)
}

/// A hero's side of the ledger: its concave life (`effective_life`, R964000) and its armor.
fn hero_side(state: &GameState, player: PlayerId, w: &EvalWeights) -> f64 {
    hero_value(effective_life(state, player), starting_life(state, player), w)
        + w.hero_armor * f64::from(hero_armor_of(state, player))
}

/// Non-hero board, dormant piles, banked modifiers, refreshes, backrow, hand, library, and fatigue.
fn material(state: &GameState, player: PlayerId, seat: PlayerId, w: &EvalWeights) -> f64 {
    let side = &state.players[player];
    let mut value = 0.0;
    // R2380000: a Dormant card (R1839000) counts by the unit it wakes as, discounted by its wait.
    for unit in units_on_field_of(state, player) {
        value += match &unit.slumber {
            None => unit_worth(state, unit, w),
            Some(slumber) => slumber_worth(state, unit, slumber, w),
        };
    }
    // §3.2, R13: the cards dormant under its Stack piles, each resuming when the cards above it leave.
    value += dormant_worth(state, player, seat, w);

    // R1224: a debt is coming turns' mana spent early, so it counts against the side that owes it.
    value -= w.owed_mana * f64::from(owed_mana_of(state, player));
    // R844, §2.3: what the side's coming refreshes will or will not give.
    value += coming_refreshes(side, w);
    // #1086: what the side has banked in state both seats see (R185): Jade steps (R962), extra
    // turns (R846) and the enemy units it has marked (R437; no `unit_worth` call at a zero share).
    value += w.jade_step * f64::from(jade_steps(side.jade.unwrap_or(0)));
    value += w.extra_turn * f64::from(side.extra_turns.unwrap_or(0));
    // R1855003, #1886: the misses the side's Scanning Probes have banked. Own side only: a miss is
    // silent, so the enemy's count is no fact the seat has.
    if player == seat {
        value += w.anomaly_miss * f64::from(side.anomaly_misses.unwrap_or(0));
        // #2389: the Credits the side holds, at what they buy at the Marketplace (R1853002), so a
        // Resource's quote (R2250001) and a buy's price count. Own side only, as the anomaly misses.
        value += w.market_credit * f64::from(side.market_credits.unwrap_or(0));
    }
    if w.marked_share != 0.0 {
        value += w.marked_share * marked_enemy_worth(state, player, w);
    }
    // #1722: what the side holds in its player modifiers, both seats' (R169): its rest-of-game
    // start-of-turn effects (R458) and its closest alternative win (R848).
    value += w.start_of_turn_effect * f64::from(start_of_turn_effects(side));
    value += alt_win_worth(state, player, w);
    // R1225: on its controller's turn, an unused base-face Jlarna is tributed at end of turn, so it
    // is worth nothing.
    let lapsing: Vec<String> = if state.active == player {
        credit_lapsing(state, player)
    } else {
        Vec::new()
    };

    for card in side.backrow.iter().flatten() {
        if lapsing.iter().any(|id| id == &card.id) {
            continue;
        }
        let def = find_def(Some(state), &card.def_id);
        match def {
            Some(def) if readable_by(state, card, seat) => {
                value += brittle_share(card, w)
                    * (w.backrow_base + w.backrow_per_cost * f64::from(query_cost(def)));
                // B3.1: a Unit in waiting (`unit_worth` brings its own Brittle share), with stats (R2280000).
                if w.animated_share != 0.0 && animated_with_stats(state, card).is_some() {
                    value += w.animated_share * unit_worth(state, card, w);
                }
                // §10.1, R27/R62: grade counter banked steps are worth extra, including steps
                // already armed this turn (`grade_rises`).
                if card.counters.grade.is_some() {
                    let grade = subsystems::grade_of(card);
                    if !subsystems::is_terminal_grade(grade) {
                        let armed = card.controller == state.active && subsystems::grade_rises(state, card);
                        value += w.grade_counter * f64::from(grade - FIRST_GRADE + i32::from(armed));
                    }
                }
            }
            // R351: the cost it shows is public; nothing else of it is (no call at a zero weight).
            _ => {
                value += w.enemy_face_down;
                if w.enemy_face_down_per_cost != 0.0 {
                    value += w.enemy_face_down_per_cost * f64::from(shown_cost(state, card));
                }
            }
        }
    }

    // #1722: a delayed discard of the side's hand waits (Classic #37 Last Hurrah).
    let doomed = discards_hand(state, player);
    if player == seat {
        for card in &side.hand {
            value += doomed_hand_card_value(state, card, doomed, w);
        }
    } else {
        let unseen = side.hand.len() as f64 * (w.hand_card + w.hand_per_cost * w.opponent_hand_cost);
        value += if doomed {
            (1.0 - w.doomed_hand_discount) * unseen
        } else {
            unseen
        };
    }

    value += w.library_card * (side.library.len() as f64).min(w.library_comfort);
    // §2.4: every fatigue draw taken adds a point to each one still to come.
    value -= w.fatigue_draw * f64::from(side.fatigue_count);
    value
}

/// The face damage `attacker`'s board could deal the other hero next turn, as AI_EVAL reads it
/// (`face_threat_with`).
pub fn face_threat(state: &GameState, attacker: PlayerId) -> i32 {
    face_threat_with(state, attacker, &AI_EVAL)
}

/// The face damage `attacker`'s board could deal next turn: swinging units (including Deft in Def, R49;
/// excluding Frozen, R1174000) soaked by Taunts (§4.4) with remaining damage hitting face (Pierce skips armor, R346).
pub fn face_threat_with(state: &GameState, attacker: PlayerId, w: &EvalWeights) -> i32 {
    let mut attacks: Vec<(&CardInstance, i32, bool)> = Vec::new();
    for unit in active_units_of(state, attacker) {
        let view = unit_view(state, unit);
        let ready = view.position == Position::Atk || (w.deft_defense_attacks && deft_in_defense(&view));
        if view.attack <= 0 || !ready {
            continue;
        }
        if !can_swing(state, unit, &view) {
            continue;
        }
        // R636: the unit's next turn starts with its exertion reset, so a Windfury unit swings twice.
        // Nor is it summoning sick then, so every swing may hit the hero.
        for _ in 0..attacks_per_turn(state, unit) {
            attacks.push((unit, view.attack, true));
        }
    }
    damage_past_taunts(state, attacker.opponent(), &attacks)
}

/// §4.5 step 4: a Reborn unit comes back at 1 health.
const REBORN_HEALTH: i32 = 1;

/// One Taunt's standing while `damage_past_taunts` walks it: the unit, its view, and the shield
/// and health its attackers (and their splashes) have left it with.
struct TauntStand {
    unit: CardInstance,
    view: UnitView,
    shield: bool,
    health: i32,
}

impl TauntStand {
    fn of(unit: CardInstance, view: UnitView) -> Self {
        let shield = has_keyword(&view.keywords, KeywordKind::DivineShield);
        let health = view.health;
        Self {
            unit,
            view,
            shield,
            health,
        }
    }
}

/// What `attacks` deal `defender`'s hero once Taunt units bind attackers (§4.2, R783002, R783003).
/// Each bound attacker resolves combat hits (#898, R94, R1120, R1849005, #1823); remaining attacks
/// reach the hero.
pub fn damage_past_taunts(
    state: &GameState,
    defender: PlayerId,
    attacks: &[(&CardInstance, i32, bool)],
) -> i32 {
    let mut free: Vec<(&CardInstance, i32, bool)> = attacks.to_vec();
    free.sort_by_key(|(_, attack, may_hit_hero)| (*may_hit_hero, *attack));
    let mut stands: Vec<TauntStand> = active_units_of(state, defender)
        .into_iter()
        .map(|unit| TauntStand::of(unit.clone(), unit_view(state, unit)))
        .filter(|stand| has_keyword(&stand.view.keywords, KeywordKind::Taunt))
        .collect();
    stands.sort_by_key(|stand| stand.view.health + stand.view.armor);

    let mut total = 0;
    let mut i = 0;
    while i < stands.len() {
        let target = AttackTarget::Unit {
            instance: stands[i].unit.clone(),
        };
        let (bound, mut unbound): (Vec<_>, Vec<_>) = free
            .into_iter()
            .partition(|(attacker, _, _)| attack_restriction(state, attacker, &target).is_ok());
        let mut bound = bound.into_iter();
        let mut poisoned = false;
        while stands[i].health > 0 && !poisoned {
            let Some((attacker, attack, _)) = bound.next() else {
                break;
            };
            // §4.3 step 1: an attacker the Taunt's First Strike destroys lands nothing.
            if falls_to_first_strike(state, attacker, &stands[i].unit, &stands[i].view) {
                continue;
            }
            let keywords = unit_view(state, attacker).keywords;
            let pierce = pierces(state, Some(attacker), None);
            // The splash belongs to the attack rather than to the hit (R63), so it lands even when
            // the hit on the Taunt itself is stopped below. Only live neighbouring Taunts matter:
            // the target itself and the stands already walked bind nothing more.
            if has_keyword(&keywords, KeywordKind::Cleave) {
                // Combat-only bonuses stay out of the threat model, so the splash is Poisonous
                // from the attacker's own keyword alone.
                let poisonous = has_keyword(&keywords, KeywordKind::Poisonous);
                let from = side_of(state, attacker);
                for neighbour in cleaved_neighbours(state, attacker, &stands[i].unit) {
                    let Some(j) = stands.iter().position(|stand| stand.unit.id == neighbour.id) else {
                        continue;
                    };
                    if j <= i || !has_keyword(&stands[j].view.keywords, KeywordKind::Taunt) {
                        continue;
                    }
                    // The stand's tracked shield and health, not the state's: an earlier splash
                    // may already have popped or hurt it.
                    let mut damaged = stands[j].view.clone();
                    damaged.health = stands[j].health;
                    let to = Side {
                        view: damaged,
                        shield: stands[j].shield,
                        indestructible: has_keyword(&stands[j].view.keywords, KeywordKind::Indestructible),
                        first_strike: false,
                        poisonous: false,
                        pierce: false,
                    };
                    let splash = blow(&from, attack, poisonous, &to);
                    if splash.pops_shield {
                        stands[j].shield = false;
                    } else {
                        // Step 9: what Trample lands beyond the neighbour's health is a hit on
                        // the hero (R63), as for the main hit below.
                        if has_keyword(&keywords, KeywordKind::Trample) && splash.dealt > stands[j].health {
                            total += subsystems::projected_hero_damage(
                                state,
                                defender,
                                splash.dealt - stands[j].health,
                                pierce,
                            );
                        }
                        if splash.kills {
                            // §4.5 step 4: a destroyed Reborn neighbour comes back and binds what
                            // is left, unless its body has no Taunt. Only a later stand is ever
                            // removed, so `i` stays valid.
                            if has_keyword(&stands[j].view.keywords, KeywordKind::Reborn) {
                                match reborn_body(state, &stands[j].unit) {
                                    Some((body, view)) => stands[j] = TauntStand::of(body, view),
                                    None => {
                                        stands.remove(j);
                                    }
                                }
                            } else {
                                stands.remove(j);
                            }
                        } else {
                            stands[j].health -= splash.dealt;
                        }
                    }
                }
            }
            // §4.4 step 1: the shield takes the whole hit.
            if stands[i].shield {
                stands[i].shield = false;
                continue;
            }
            // Step 2: the Taunt's Armor comes off each hit, unless it pierces (R346); a hit it takes
            // whole is no damage instance at all (R1361).
            let dealt = if pierce {
                attack
            } else {
                (attack - stands[i].view.armor).max(0)
            };
            if dealt <= 0 {
                continue;
            }
            // Step 9: what Trample lands beyond the Taunt's health is a hit on the hero (R63).
            if has_keyword(&keywords, KeywordKind::Trample) && dealt > stands[i].health {
                total += subsystems::projected_hero_damage(state, defender, dealt - stands[i].health, pierce);
            }
            // Step 7: Poisonous destroys with any damage dealt.
            poisoned = has_keyword(&keywords, KeywordKind::Poisonous);
            stands[i].health -= dealt;
        }
        unbound.extend(bound);
        unbound.sort_by_key(|(_, attack, may_hit_hero)| (*may_hit_hero, *attack));
        free = unbound;
        // §4.5 step 4: a destroyed Reborn Taunt comes back and binds what is left.
        if (stands[i].health <= 0 || poisoned)
            && has_keyword(&stands[i].view.keywords, KeywordKind::Reborn)
            && let Some((body, view)) = reborn_body(state, &stands[i].unit)
        {
            stands[i] = TauntStand::of(body, view);
            continue;
        }
        i += 1;
    }

    for (attacker, hit, may_hit_hero) in &free {
        if !may_hit_hero {
            continue;
        }
        let pierce = pierces(state, Some(attacker), None);
        total += subsystems::projected_hero_damage(state, defender, *hit, pierce);
    }
    total
}

/// §4.3 step 1, R93: a First Strike Taunt destroys an attacker before it hits (runs §4.4 steps, Pierce R346).
/// Ranged attackers take no strike back from non-Ranged Taunts (R783000).
fn falls_to_first_strike(
    state: &GameState,
    attacker: &CardInstance,
    taunt: &CardInstance,
    view: &UnitView,
) -> bool {
    if view.attack <= 0
        || !has_keyword(&view.keywords, KeywordKind::FirstStrike)
        || !takes_strike_back(state, attacker, taunt)
    {
        return false;
    }
    let mine = unit_view(state, attacker);
    if has_keyword(&mine.keywords, KeywordKind::FirstStrike)
        || has_keyword(&mine.keywords, KeywordKind::DivineShield)
        || has_keyword(&mine.keywords, KeywordKind::Indestructible)
    {
        return false;
    }
    let dealt = if pierces(state, Some(taunt), None) {
        view.attack
    } else {
        (view.attack - mine.armor).max(0)
    };
    dealt > 0 && (has_keyword(&view.keywords, KeywordKind::Poisonous) || dealt >= mine.health)
}

/// §4.5 step 4: the body a destroyed Reborn unit comes back as, a reset instance (R78) without Reborn
/// at `REBORN_HEALTH`, and its view. A printed Divine Shield comes back with it; a granted keyword, a
/// buff, or Defense Position's Taunt, is gone.
pub(crate) fn reborn_return(state: &GameState, unit: &CardInstance) -> (CardInstance, UnitView) {
    let mut body = unit.clone();
    reset_instance(&mut body);
    body.reborn_spent = Some(true);
    body.damage = (unit_view(state, &body).max_health - REBORN_HEALTH).max(0);
    let view = unit_view(state, &body);
    (body, view)
}

/// §4.5 step 4: the body a destroyed Reborn Taunt comes back as (`reborn_return`), or nothing when
/// the body has no Taunt.
fn reborn_body(state: &GameState, taunt: &CardInstance) -> Option<(CardInstance, UnitView)> {
    let (body, view) = reborn_return(state, taunt);
    has_keyword(&view.keywords, KeywordKind::Taunt).then_some((body, view))
}

/// What destroying `unit` takes off the board (#1796): its `unit_worth`, less the worth of the body it
/// comes back as while its Reborn is unspent (§4.5 step 4, `reborn_return`), and less what its Death
/// leaves its controller (`death_price`), whole, as Reborn's body is (R2424000). A spent Reborn is gone
/// from its keywords (§10.4), so such a unit is worth its whole `unit_worth`.
pub fn kill_worth(state: &GameState, unit: &CardInstance, w: &EvalWeights) -> f64 {
    let worth = unit_worth(state, unit, w) - death_price(state, unit, w);
    if !unit_has(state, unit, KeywordKind::Reborn) {
        return worth;
    }
    let (body, _) = reborn_return(state, unit);
    worth - unit_worth(state, &body, w)
}

/// R923003: the sandbox's tool `op` as `seat` uses it on the probe world `state`, one node of `counter`
/// (`simulate`). `None` when the counter refuses it; `Some(None)` when the tool is refused (checked
/// first, so no refusal counts as a failed simulation), the step fails, or it leaves a prompt open or
/// the game over, which the probe neither answers nor prices.
fn sandbox_step(
    state: &GameState,
    seat: PlayerId,
    op: SandboxOp,
    counter: &dyn NodeCounter,
) -> Option<Option<GameState>> {
    if why_sandbox_refused(state, &op).is_err() {
        return Some(None);
    }
    let Ok(next) = simulate(state, seat, &ActionBody::Sandbox { op }, counter)? else {
        return Some(None);
    };
    Some((next.pending.is_none() && next.result.is_none()).then_some(next))
}

/// R2424000: sandbox (R923000) valuation of `unit`'s Death: destroy eval minus remove eval (R923003),
/// excluding Reborn. None when counter spent, Some(None) if unpriceable.
fn death_probe(
    before: &GameState,
    seat: PlayerId,
    unit: &CardInstance,
    counter: &dyn NodeCounter,
) -> Option<Option<f64>> {
    let destroy = SandboxOp::Destroy {
        instance_id: unit.id.clone(),
    };
    let Some(destroyed) = sandbox_step(before, seat, destroy, counter)? else {
        return Some(None);
    };
    // A unit still on the field with its Reborn as it was is one the destroy spared (Indestructible).
    let spared = find_instance(&destroyed, &unit.id)
        .is_some_and(|card| card.zone.z() == ZoneName::Field && card.reborn_spent == unit.reborn_spent);
    if spared {
        return Some(None);
    }
    let remove = SandboxOp::Remove {
        instance_id: unit.id.clone(),
    };
    let Some(removed) = sandbox_step(before, seat, remove, counter)? else {
        return Some(None);
    };
    let reborn = if unit_has(before, unit, KeywordKind::Reborn) {
        unit_worth(before, &reborn_return(before, unit).0, &AI_EVAL)
    } else {
        0.0
    };
    let owner = unit.controller;
    Some(Some(
        evaluate(&destroyed, owner, NextSwing::Enemy, &AI_EVAL)
            - evaluate(&removed, owner, NextSwing::Enemy, &AI_EVAL)
            - reborn,
    ))
}

/// R2424000: `death_probe` for a Unit in `seat`'s hand, placed on its field first (R923001's spawn,
/// a fresh card of its definition and face) in the zone a summon would take (`first_entry_zone`). On a
/// full unit row the first zone whose pile is a single card is cleared first, by the sandbox's remove.
fn spawned_death_probe(
    world: &GameState,
    seat: PlayerId,
    card: &CardInstance,
    counter: &dyn NodeCounter,
) -> Option<Option<f64>> {
    let cleared;
    let (room, slot) = match first_entry_zone(world, seat, Row::Units) {
        Some(slot) => (world, slot),
        None => {
            let single = slots_of(seat, Row::Units)
                .into_iter()
                .find_map(|slot| match pile_at(world, slot) {
                    Some(pile) if pile.len() == 1 => Some((slot, pile[0].id.clone())),
                    _ => None,
                });
            let Some((slot, top)) = single else {
                return Some(None);
            };
            let Some(state) = sandbox_step(world, seat, SandboxOp::Remove { instance_id: top }, counter)?
            else {
                return Some(None);
            };
            cleared = state;
            (&cleared, slot)
        }
    };
    let spawn = SandboxOp::Spawn {
        player: seat,
        def_id: card.def_id.clone(),
        radiant: card.radiant.then_some(true),
        to: SpawnZone::Field {
            row: Row::Units,
            lane: slot.lane,
        },
    };
    let Some(placed) = sandbox_step(room, seat, spawn, counter)? else {
        return Some(None);
    };
    let Some(unit) = card_at(&placed, slot).cloned() else {
        return Some(None);
    };
    death_probe(&placed, seat, &unit, counter)
}

/// R2424000: Death prices for `seat` probed on `world` (R185) for field units and hand units (R923001).
/// Probed on sandbox copies (R923000, R923003) with auto turn-end disabled (R345, R82), capped at `AI_DEATH_PROBE_NODES`.
pub fn death_prices(world: &GameState, seat: PlayerId, counter: &dyn NodeCounter) -> Vec<DeathPrice> {
    if AI_EVAL.death == 0.0 || world.pending.is_some() || world.result.is_some() {
        return Vec::new();
    }
    let field = active_units_of(world, seat)
        .into_iter()
        .chain(active_units_of(world, seat.opponent()))
        .map(|unit| (unit, false));
    let hand = world.players[seat]
        .hand
        .iter()
        .filter(|card| card_type_of_face(world, &card.def_id, card.radiant) == CardType::Unit)
        .map(|card| (card, true));
    let mut candidates: Vec<(DeathPrice, &CardInstance, bool)> = Vec::new();
    for (card, in_hand) in field.chain(hand) {
        if death_hook_of(world, card).is_some() && !candidates.iter().any(|(key, _, _)| key.is_for(card)) {
            candidates.push((DeathPrice::key(card), card, in_hand));
        }
    }
    if candidates.is_empty() {
        return Vec::new();
    }
    let mut sandbox = world.clone();
    sandbox.sandbox = Some(true);
    for player in PLAYER_IDS {
        sandbox.players[player].auto_end_turn = Some(false);
    }
    let probes = create_sub_counter(counter, AI_DEATH_PROBE_NODES);
    with_death_prices(Vec::new(), || {
        let mut priced = Vec::new();
        for (key, card, in_hand) in candidates {
            let probed = if in_hand {
                spawned_death_probe(&sandbox, seat, card, &probes)
            } else {
                death_probe(&sandbox, seat, card, &probes)
            };
            match probed {
                None => break,
                Some(None) => {}
                Some(Some(price)) => priced.push(DeathPrice { price, ..key }),
            }
        }
        priced
    })
}

/// Who swings next, for the threat and pressure terms: "enemy" (the default) reads a state as the end
/// of `seat`'s turn, where the enemy's board attacks before `seat` can answer; "seat" reads it as the
/// start of `seat`'s turn (the opponent's reply has been played out), where `seat` attacks first and
/// the enemy's board is a threat `seat` still has a whole turn to answer.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[serde(rename_all = "camelCase")]
pub enum NextSwing {
    #[default]
    Enemy,
    Seat,
}

/// A score for `seat` (higher is better). Pure, cheap (O(cards)), reads only public-or-sampled state.
/// `w` is the AI's own AI_EVAL unless a caller brings its own weights (the greedy baseline's
/// GREEDY_EVAL, which tuning the AI never moves).
pub fn evaluate(state: &GameState, seat: PlayerId, next: NextSwing, w: &EvalWeights) -> f64 {
    let opp = seat.opponent();
    if let Some(result) = &state.result {
        let turn = f64::from(state.turn);
        // R1843004: the game as it ended for this seat, so the opponent's flight is a draw here.
        return match outcome_for(result, seat) {
            SeatOutcome::Win => w.win - turn,
            SeatOutcome::Loss => -w.win + turn,
            SeatOutcome::Draw => w.drawn,
        };
    }

    let enemy_damage = face_threat_with(state, opp, w);
    let our_damage = face_threat_with(state, seat, w);
    // §13.4, R964000: a hero's life is `effective_life`, own HP and the team's pool in a Duos duel,
    // the hero's health in a 1v1 game.
    let seat_life = effective_life(state, seat);
    let opp_life = effective_life(state, opp);
    let seat_first = next == NextSwing::Seat;
    let threat_share = if seat_first { w.answerable_threat } else { 1.0 };
    let threat = threat_share
        * (if enemy_damage >= seat_life {
            w.lethal_threat
        } else {
            w.threat_per_damage * f64::from(enemy_damage)
        });
    let pressure = if our_damage >= opp_life {
        if seat_first {
            w.lethal_on_board
        } else {
            w.lethal_pressure
        }
    } else {
        w.pressure_per_damage * f64::from(our_damage)
    };

    // §2.5: the game is a draw at the turn cap, which scores nothing, so late on every point of damage
    // on the enemy hero is worth more than the concave hero value alone says.
    let span = (f64::from(TURN_CAP_PLAYER_TURNS) - w.closing_from).max(1.0);
    let progress = ((f64::from(state.turn) - w.closing_from).max(0.0) / span).min(1.0);
    let closing = w.closing_weight * progress * f64::from(starting_life(state, opp) - opp_life);

    let race = w.enemy_health * f64::from(opp_life);

    let board = material(state, seat, seat, w) - material(state, opp, seat, w);

    hero_side(state, seat, w) - hero_side(state, opp, w) + board - threat + pressure + closing - race
}
