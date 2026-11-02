//! The AI's static evaluation (SPEC §9.9): hero health (concave, so the last points weigh most),
//! hero armor, board stats and keywords through §10.4's layers (a Defense-Position unit's attack at
//! a discount), cards in hand, library, the enemy's face damage next turn against our life and the
//! reverse (in a Duos duel own HP and the team's pool, R964000), and, late in the game, the turn cap.
//! #1086 prices what a side has banked in public state, #1224 what its coming refreshes will or will
//! not give, #1506 the cards dormant under its Stack piles, #1602 three facts both seats see. The
//! decision's root prices each Death the seat can read by what it leaves behind (#2424; a Starship
//! Piece's by the copy it fuses into, #2566), each turn hook by one firing (#2477, #2565), each
//! Activate by one use (#2564; a face-up backrow card's, #2603) and each attack hook by the unit's next
//! attack against its hookless twin's (#2688), and the rest read them (`Prices`). It reads only what
//! the seat may know or what a determinization sampled, and nothing here reads a handicap or a
//! difficulty (R180).

use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::IndexSet;
use jackioh_engine::config::{
    CALL_TO_CHAOS_RADIANT_EFFECTS, DUOS_OWN_HEALTH, FIRST_GRADE, HERO_HEALTH, JADE_ASCEND_AT, JADE_BEAUTY_AT,
    TURN_CAP_PLAYER_TURNS,
};
use jackioh_engine::effects::{DELAYED_DESTROY_HOOK, DELAYED_DISCARD_HAND_HOOK};
use jackioh_engine::{
    ActionBody, AltWinCondition, AttackHealth, AttackTarget, CardDef, CardInstance, CardType, CostOptions,
    EngineSink, GameEvent, GameState, Grant, HookName, Keyword, KeywordKind, ModifierKind, PLAYER_IDS, Phase,
    PlayerId, PlayerState, Position, Rng, Row, SandboxOp, SeatOutcome, SettleOptions, Slumber, SpawnZone,
    Tag, TriggerHolder, TriggerZone, UnitView, ZoneName, acting_with_flag, active_brittle_count,
    active_units_of, animated_with_stats, armor_of, attack_restriction, attacks_per_turn, backrow_is_public,
    board_totals, cannot_attack, card_at, card_keywords, card_type_of_face, credit_lapsing, death_hook_of,
    effective_cost, effective_life, face_of, find_def, find_instance, find_instance_mut, first_entry_zone,
    frozen_next_turn, has_keyword, heal_multiplier_of, hero_armor_of, is_standing_unit, is_starship_piece,
    is_temporary_card, legal_actions, lost_refreshes_left, modifier_is_live, outcome_for, owed_mana_of,
    own_cost, pierces, pile_at, query_cost, queue_hook, reset_instance, script_of, settle, slots_of,
    stats_with_buffs, subsystems, sync_fused_scripts, takes_strike_back, trigger_holders_with_hook, unit_has,
    unit_view, units_on_field_of, why_sandbox_refused, with_pool_hold,
};
use serde::{Deserialize, Serialize};

use crate::candidates::{Side, blow, cleaved_neighbours, side_of};
use crate::config::{
    AI_ACTIVATION_PROBE_NODES, AI_ATTACK_HOOK_PROBE_NODES, AI_DEATH_PROBE_NODES, AI_EVAL,
    AI_EXTRA_TURN_CARDS, AI_TURN_HOOK_PROBE_NODES, EvalWeights,
};
use crate::reply::{Known, hidden_card_ids, visible_activation};
use crate::simulate::{create_sub_counter, note_sim_panic, simulate};
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

/// E8, R413: whether a heal of `player`'s hero is replaced by Pierce damage, the read-only half of
/// `replacements::healing_replaced`: a live `HealToDamage` modifier on any other player (R169, both
/// seats see it), or a face-up `heal_to_damage` card on any other player's side (`acting_with_flag`
/// drops face-down traps per R33/R1487000, so a hidden converter stays unmodeled).
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

/// R351: the cost a backrow card the seat cannot read shows both players (`view_for`'s number): a
/// placeholder's `cost_override` from `redact`, any other card's `effective_cost`. A determinization
/// keeps the parked price on the trap it samples (R762), so every world shows what the true board does.
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

/// R2424000, R2566000: whether `card`'s death leaves its controller something to price: a live Death
/// (`death_hook_of`: a Vanilla unit has none), or, while it is a Starship Piece (`is_starship_piece`:
/// the keyword through the layers, face-up), the copy its death fuses into its owner's Starship (R1852000).
fn leaves_behind(state: &GameState, card: &CardInstance) -> bool {
    death_hook_of(state, card).is_some() || is_starship_piece(state, card)
}

/// R2424000: what `unit`'s death leaves its controller, by `prices`, while it leaves something
/// (`leaves_behind`, R2566000). 0 where `prices` has no entry and at a zero `death` weight.
fn death_price(state: &GameState, unit: &CardInstance, w: &EvalWeights, prices: &Prices) -> f64 {
    if w.death == 0.0 {
        return 0.0;
    }
    let price = prices
        .deaths
        .iter()
        .find(|entry| entry.is_for(unit))
        .map(|entry| entry.price);
    match price {
        Some(price) if leaves_behind(state, unit) => price,
        _ => 0.0,
    }
}

/// R2477000, R2565000: what one firing of a card's start-of-turn and end-of-turn hooks gives its
/// controller, as a decision's root priced it (`turn_hook_prices`), and the card it prices: its
/// definition and its face, so the same card the search moves or sets later reads the same price.
#[derive(Clone, Debug, PartialEq)]
pub struct TurnHookPrice {
    pub def_id: String,
    pub radiant: bool,
    pub price: f64,
}

impl TurnHookPrice {
    fn key(card: &CardInstance) -> Self {
        Self {
            def_id: card.def_id.clone(),
            radiant: card.radiant,
            price: 0.0,
        }
    }

    fn is_for(&self, card: &CardInstance) -> bool {
        self.def_id == card.def_id && self.radiant == card.radiant
    }
}

/// R2424000, R2477000, R2564000, R2688000: the prices a decision's root probed, passed by reference
/// from `decide` through the search to every scorer that reads them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Prices {
    pub deaths: Vec<DeathPrice>,
    pub turn_hooks: Vec<TurnHookPrice>,
    pub activations: Vec<ActivationPrice>,
    pub attack_hooks: Vec<AttackHookPrice>,
}

impl Prices {
    /// No price: what every call outside a decision reads (the probes, the baselines, the tools).
    pub const NONE: &'static Prices = &Prices {
        deaths: Vec::new(),
        turn_hooks: Vec::new(),
        activations: Vec::new(),
        attack_hooks: Vec::new(),
    };
}

/// R2477000: whether the face `card` runs has a start-of-turn or an end-of-turn hook (`script_of`: a
/// Vanilla card has none).
fn has_turn_hook(state: &GameState, card: &CardInstance) -> bool {
    let script = script_of(state, card);
    script.start_of_turn.is_some() || script.end_of_turn.is_some()
}

/// R2477000: what one firing of `card`'s turn hooks gives its controller, by `prices`, while its face
/// has one (`has_turn_hook`). 0 where `prices` has no entry and at a zero `turn_hook` weight.
fn turn_hook_price(state: &GameState, card: &CardInstance, w: &EvalWeights, prices: &Prices) -> f64 {
    if w.turn_hook == 0.0 {
        return 0.0;
    }
    let price = prices
        .turn_hooks
        .iter()
        .find(|entry| entry.is_for(card))
        .map(|entry| entry.price);
    match price {
        Some(price) if has_turn_hook(state, card) => price,
        _ => 0.0,
    }
}

/// R2564000, R2603000: what one use of a card's Activate gives its controller, as a decision's root priced
/// it (`activation_prices`), and the card it prices: its definition, its face and its controller, so the
/// same card the search moves or makes later reads the same price. The controller is in the key since
/// each copy aims at a different board.
#[derive(Clone, Debug, PartialEq)]
pub struct ActivationPrice {
    pub def_id: String,
    pub radiant: bool,
    pub controller: PlayerId,
    pub price: f64,
}

impl ActivationPrice {
    fn key(card: &CardInstance) -> Self {
        Self {
            def_id: card.def_id.clone(),
            radiant: card.radiant,
            controller: card.controller,
            price: 0.0,
        }
    }

    fn is_for(&self, card: &CardInstance) -> bool {
        self.def_id == card.def_id && self.radiant == card.radiant && self.controller == card.controller
    }
}

/// R2688000: what a unit's next attack gives its controller through its "when this attacks" and
/// "after this attacks" hooks, as a decision's root priced it (`attack_hook_prices`), keyed as
/// `ActivationPrice` is, its controller included since each copy attacks a different board.
#[derive(Clone, Debug, PartialEq)]
pub struct AttackHookPrice {
    pub def_id: String,
    pub radiant: bool,
    pub controller: PlayerId,
    pub price: f64,
}

impl AttackHookPrice {
    /// The price `card` would hold, at 0 until it is probed.
    fn key(card: &CardInstance) -> Self {
        Self {
            def_id: card.def_id.clone(),
            radiant: card.radiant,
            controller: card.controller,
            price: 0.0,
        }
    }

    fn is_for(&self, card: &CardInstance) -> bool {
        self.def_id == card.def_id && self.radiant == card.radiant && self.controller == card.controller
    }
}

/// R2564000, R2603000: what one use of `unit`'s Activate gives its controller, by `prices`, while it has
/// an ability to use (`abilities_of`: a Vanilla card has none). 0 with `Prices::NONE`, for a card its root
/// did not price, and at a zero `activation` weight (no reading).
fn activation_price(state: &GameState, unit: &CardInstance, w: &EvalWeights, prices: &Prices) -> f64 {
    if w.activation == 0.0 {
        return 0.0;
    }
    let price = prices
        .activations
        .iter()
        .find(|entry| entry.is_for(unit))
        .map(|entry| entry.price);
    match price {
        Some(price) if !subsystems::abilities_of(state, unit).is_empty() => price,
        _ => 0.0,
    }
}

/// R2688000: whether the face `card` runs has a "when this attacks" or an "after this attacks" hook
/// (`script_of`: a Vanilla or Dormant card has none).
fn has_attack_hook(state: &GameState, card: &CardInstance) -> bool {
    let script = script_of(state, card);
    script.when_attacks.is_some() || script.after_attack.is_some()
}

/// R2688000: what `unit`'s next attack gives its controller through its attack hooks, by `prices`,
/// while its face has one (`has_attack_hook`). 0 with `Prices::NONE`, for a card its root did not
/// price, and at a zero `attack_hook` weight (no reading).
fn attack_hook_price(state: &GameState, unit: &CardInstance, w: &EvalWeights, prices: &Prices) -> f64 {
    if w.attack_hook == 0.0 {
        return 0.0;
    }
    let price = prices
        .attack_hooks
        .iter()
        .find(|entry| entry.is_for(unit))
        .map(|entry| entry.price);
    match price {
        Some(price) if has_attack_hook(state, unit) => price,
        _ => 0.0,
    }
}

/// One unit's worth on the board: its stats through the layers and its keywords (AI_EVAL), Spell
/// Damage per point, `activation` of one use of its Activate (R2564000) and, while it may attack,
/// `attack_hook` of its attack hooks (R2688000), all discounted by its Brittle count; then `death` of
/// what its Death leaves behind (R2424000) and `turn_hook` of one firing of its turn hooks (R2565000).
pub fn unit_worth(state: &GameState, unit: &CardInstance, w: &EvalWeights, prices: &Prices) -> f64 {
    let view = unit_view(state, unit);
    let weights = &w.keyword;
    let mut value = w.health * f64::from(view.health) + w.armor_point * f64::from(view.armor);
    let swings = can_swing(state, unit, &view);
    if swings {
        // §4.1: a Defense-Position unit strikes back in full but cannot attack until a switch spends its
        // exertion, so only part of its attack counts; a Deft unit keeps it all (R49, `deftDefenseAttacks`).
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
    // §4.1: Defense Position's Taunt and Armor +1 are the position's, not the unit's: their worth is the
    // damage they soak, which `face_threat` and the reply already count, so here they count for only
    // `positionGrants`. `defenseOwnTauntKept` keeps a unit's own Taunt in full and discounts Armor only
    // when it is on the body.
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
    // R2564000: what one use of its Activate gives, inside the Brittle share: a crumbled card has no
    // coming uses.
    value += w.activation * activation_price(state, unit, w, prices);
    // R2688000: what its attack hooks give, inside the Brittle share too, and only while it may attack.
    if swings {
        value += w.attack_hook * attack_hook_price(state, unit, w, prices);
    }
    // §10.1: the counters on the card are worth their own, outside the Brittle share — Fed Fauci's Plague
    // Counters are mana it already banked. R2424000: so is what its Death leaves, since Brittle's end is a
    // death too. R2565000: and what one firing of its turn hooks gives, a firing that harms it a debt.
    value * brittle_share(unit, w)
        + w.plague_counter * f64::from(unit.counters.plague.unwrap_or(0))
        + w.death * death_price(state, unit, w, prices)
        + w.turn_hook * turn_hook_price(state, unit, w, prices)
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
/// none of the field's layers, position or damage.
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

/// R437: the `unit_worth` of the enemy units a delayed effect `player` owns has marked, each once (an entry
/// `redact` dropped, R185, counts nothing). A mark on a hand card (R1141), a backrow card or an own unit counts
/// nothing, nor does an unmarked effect (as often a cost as a payoff; a countdown is weighed in `reply_score`,
/// #1995) or a delayed destroy's mark on a unit Indestructible now, which it spares (Classic #20, R46).
fn marked_enemy_worth(state: &GameState, player: PlayerId, w: &EvalWeights, prices: &Prices) -> f64 {
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
            worth += unit_worth(state, unit, w, prices);
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

/// §3.2, R13: what the cards dormant under `player`'s Stack piles are worth. The seat reads its own unit
/// piles card by card, each at `dormant_share` of its `unit_worth`; every other dormant card counts
/// `dormant_card`: the enemy's unit piles, and every backrow pile, whose cards both seats see only as a
/// count (R447; `redact` makes a face-down one a placeholder, R185).
fn dormant_worth(
    state: &GameState,
    player: PlayerId,
    seat: PlayerId,
    w: &EvalWeights,
    prices: &Prices,
) -> f64 {
    let side = &state.players[player];
    let beneath = side.units.iter().flatten().flat_map(|pile| pile.iter().skip(1));
    let buried = side.backrow_piles.iter().flatten().map(Vec::len).sum::<usize>();
    let units = if player != seat {
        beneath.count() as f64 * w.dormant_card
    } else if w.dormant_share != 0.0 {
        beneath
            .map(|card| w.dormant_share * unit_worth(state, card, w, prices))
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
fn slumber_worth(
    state: &GameState,
    unit: &CardInstance,
    slumber: &Slumber,
    w: &EvalWeights,
    prices: &Prices,
) -> f64 {
    let share = match slumber.turns {
        Some(turns) => w.slumber_turn_share.powi(turns.max(0)),
        None => w.slumber_waiting_share,
    };
    if share == 0.0 {
        return 0.0;
    }
    let mut awake = unit.clone();
    awake.slumber = None;
    share * unit_worth(state, &awake, w, prices)
}

/// A hero's side of the ledger: its concave life (`effective_life`, R964000) and its armor.
fn hero_side(state: &GameState, player: PlayerId, w: &EvalWeights) -> f64 {
    hero_value(effective_life(state, player), starting_life(state, player), w)
        + w.hero_armor * f64::from(hero_armor_of(state, player))
}

/// Everything on one side but the hero: the board and the cards dormant under its piles, what it has
/// banked, what its coming refreshes will or will not give, the backrow (an enemy face-down card by the
/// cost it shows, never a Unit standing there, R2490000), the hand, the library and the fatigue draws it
/// has taken.
fn material(state: &GameState, player: PlayerId, seat: PlayerId, w: &EvalWeights, prices: &Prices) -> f64 {
    let side = &state.players[player];
    let mut value = 0.0;
    for unit in units_on_field_of(state, player) {
        value += match &unit.slumber {
            None => unit_worth(state, unit, w, prices),
            Some(slumber) => slumber_worth(state, unit, slumber, w, prices),
        };
    }
    value += dormant_worth(state, player, seat, w, prices);

    // R1224: a debt is coming turns' mana spent early, so it counts against the side that owes it.
    value -= w.owed_mana * f64::from(owed_mana_of(state, player));
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
        value += w.marked_share * marked_enemy_worth(state, player, w, prices);
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
        // R2490000: a Unit standing in the backrow is priced once, as the Unit the unit walk above
        // priced, never again as a backrow card (R1081000: it is a Unit for every rule, and a count
        // of permanents counts it once).
        if w.standing_unit_priced_once && is_standing_unit(state, card) {
            continue;
        }
        if lapsing.iter().any(|id| id == &card.id) {
            continue;
        }
        let def = find_def(Some(state), &card.def_id);
        match def {
            Some(def) if readable_by(state, card, seat) => {
                value += brittle_share(card, w)
                    * (w.backrow_base + w.backrow_per_cost * f64::from(query_cost(def)));
                // R2477000, R2565000: what one firing of its turn hooks gives its controller, as the root
                // priced it; either side's, since a card the seat can read is one it may price.
                value += w.turn_hook * turn_hook_price(state, card, w, prices);
                // R2603000: what one use of its Activate gives its controller, as the root priced it, inside
                // its Brittle share as a unit's is (R2564000); either side's.
                value += brittle_share(card, w) * w.activation * activation_price(state, card, w, prices);
                // B3.1: a Unit in waiting (`unit_worth` brings its own Brittle share), with stats (R2280000).
                if w.animated_share != 0.0 && animated_with_stats(state, card).is_some() {
                    value += w.animated_share * unit_worth(state, card, w, prices);
                }
                // §10.1, R27/R62: a grade counter's banked steps are worth their own. On the controller's
                // own turn a rise this turn's plays already arm counts as banked (`grade_rises`), so the
                // beam can prefer the lines that set it off.
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

/// The face damage `attacker`'s board could deal the other hero next turn: units with attack > 0 that may swing
/// (`can_swing`; with `w.deft_defense_attacks` a Deft unit in Defense Position too, R49), ascending by attack.
/// Taunts soak the smallest first (`damage_past_taunts`); each remaining attack goes through
/// `subsystems::projected_hero_damage` (armor, which a Pierce hit skips, R346, and the Anti-oneshot cap), summed.
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

/// What `attacks` (attacker, its attack, whether it may hit the hero once no Taunt walls it) deal
/// `defender`'s hero once its Taunts have soaked the attackers they bind (those that may not hit the hero
/// first, then the smallest). §4.2 step 3: a Taunt binds only attackers `attack_restriction` lets attack
/// it (R783002, R783003). Each bound attacker is one §4.4 hit on it (#898, R346, R63), a destroyed Reborn
/// Taunt returning; lane strike (R94) and combat-only bonuses (R1120) stay out. A hit that may not hit the
/// hero (R1849005, #1823) lands nothing past the Taunts.
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

/// §4.3 step 1 and R93, as `subsystems::lethal`'s private `falls_to_first_strike` reads it: a First Strike Taunt
/// strikes an attacker without it first, and an attacker that falls there never lands its hit. The strike back runs
/// the §4.4 steps that can stop it (Divine Shield, Armor, which a Pierce Taunt skips, R346, Indestructible, the zero
/// rule); a Poisonous Taunt destroys with any damage. A Ranged attacker takes none from a non-Ranged Taunt (R783000).
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
pub fn kill_worth(state: &GameState, unit: &CardInstance, w: &EvalWeights, prices: &Prices) -> f64 {
    let worth = unit_worth(state, unit, w, prices) - death_price(state, unit, w, prices);
    if !unit_has(state, unit, KeywordKind::Reborn) {
        return worth;
    }
    let (body, _) = reborn_return(state, unit);
    worth - unit_worth(state, &body, w, prices)
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
    let Ok(next) = simulate(state, seat, &ActionBody::Sandbox { op }, counter, Prices::NONE)? else {
        return Some(None);
    };
    Some((next.pending.is_none() && next.result.is_none()).then_some(next))
}

/// R2424000: what `unit`'s Death leaves its controller on `before`, a sandbox (R923000): the controller's
/// evaluation after the destroy, which runs its Death, less after its remove, which does not (R923003),
/// less the body a Reborn brings back. `None` when the counter is spent; `Some(None)` when the destroy
/// spares the unit, or either leaves a prompt open or ends the game (`sandbox_step`).
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
        unit_worth(before, &reborn_return(before, unit).0, &AI_EVAL, Prices::NONE)
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

/// R923001: `card`, a Unit in `seat`'s hand, spawned onto `world`'s field (a fresh card of its definition
/// and face) in the zone a summon would take (`first_entry_zone`); on a full unit row the first zone whose
/// pile is a single card is cleared first, by the sandbox's remove. `None` when the counter refuses a
/// step; `Some(None)` when no zone can be made or a tool is refused.
fn spawn_hand_unit(
    world: &GameState,
    seat: PlayerId,
    card: &CardInstance,
    counter: &dyn NodeCounter,
) -> Option<Option<(GameState, CardInstance)>> {
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
    Some(Some((placed, unit)))
}

/// R2424000: `death_probe` for a Unit in `seat`'s hand, placed on its field first (`spawn_hand_unit`).
fn spawned_death_probe(
    world: &GameState,
    seat: PlayerId,
    card: &CardInstance,
    counter: &dyn NodeCounter,
) -> Option<Option<f64>> {
    let Some((placed, unit)) = spawn_hand_unit(world, seat, card, counter)? else {
        return Some(None);
    };
    death_probe(&placed, seat, &unit, counter)
}

/// R2424000: the Death prices a decision of `seat`'s passes (`Prices`), probed on `world`, its probe world
/// (R185), each card once: every unit on either field whose Death is live, the seat's first, then each Unit
/// in the seat's hand with one (R923001). R2566000: last, each Starship Piece with none, so pieces never take
/// the nodes a Death drew. On a sandbox copy (R923000), automatic turn end off (R345, R82), no price passed
/// (R923003), at most `AI_DEATH_PROBE_NODES` nodes from `counter`.
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
    let listed: Vec<(&CardInstance, bool)> = field.chain(hand).collect();
    let mut candidates: Vec<(DeathPrice, &CardInstance, bool)> = Vec::new();
    for pieces in [false, true] {
        for &(card, in_hand) in &listed {
            let wanted = if pieces {
                is_starship_piece(world, card)
            } else {
                death_hook_of(world, card).is_some()
            };
            if wanted && !candidates.iter().any(|(key, _, _)| key.is_for(card)) {
                candidates.push((DeathPrice::key(card), card, in_hand));
            }
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
}

/// R2477000: the two hooks a turn fires for its player's cards (§6.2, R62), in the order a turn meets them.
const TURN_HOOKS: [HookName; 2] = [HookName::StartOfTurn, HookName::EndOfTurn];

/// R2477000: `before` after one firing of `holder`'s `hook`, queued as its controller's turn would queue
/// it (`queue_hook`, R68) and drained by the resolution loop (`settle`), where the decision stands:
/// under its game's pool hold (R2201001), on the state's own random stream with the cursor written back
/// (CLAUDE.md rule 4), as `reduce` steps a game.
fn fire_turn_hook(before: &GameState, holder: &TriggerHolder, hook: HookName) -> GameState {
    with_pool_hold(before, || {
        let mut next = before.clone();
        sync_fused_scripts(&mut next);
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&next.seed, next.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut next, &mut events, &mut rng);
            queue_hook(&mut sink, holder, hook);
            settle(&mut sink, SettleOptions::default());
        }
        next.rng_cursor = rng.cursor();
        next
    })
}

/// R2477000: what one firing of `holder`'s `hook` on `before` gives its controller: the controller's
/// evaluation after it less before it, and `AI_EVAL.next_turn_mana` for each crystal it gained, which the
/// evaluation does not count. One node of `counter`: `None` when the counter refuses it; `Some(None)`
/// when the firing panics (`note_sim_panic`), or leaves a prompt open or the game over.
fn hook_firing_price(
    before: &GameState,
    holder: &TriggerHolder,
    hook: HookName,
    counter: &dyn NodeCounter,
) -> Option<Option<f64>> {
    if !counter.take() {
        return None;
    }
    let after = match catch_unwind(AssertUnwindSafe(|| fire_turn_hook(before, holder, hook))) {
        Ok(after) => after,
        Err(payload) => {
            note_sim_panic(counter, payload.as_ref());
            return Some(None);
        }
    };
    if after.pending.is_some() || after.result.is_some() {
        return Some(None);
    }
    let owner = holder.controller;
    let gained = after.players[owner].mana.current - before.players[owner].mana.current;
    Some(Some(
        evaluate(&after, owner, NextSwing::Enemy, &AI_EVAL)
            - evaluate(before, owner, NextSwing::Enemy, &AI_EVAL)
            + AI_EVAL.next_turn_mana * f64::from(gained),
    ))
}

/// R2477000, R2565000: what one firing of each turn hook of `owner`'s card `instance_id`, in its backrow
/// or on its field, gives `owner` on `before`, a sandbox (R923000), the hooks' prices summed
/// (`hook_firing_price`), each fired on `before` alone. `None` when the counter is spent; `Some(None)`
/// when no hook of it is priced.
fn turn_hook_probe(
    before: &GameState,
    owner: PlayerId,
    instance_id: &str,
    counter: &dyn NodeCounter,
) -> Option<Option<f64>> {
    let mut total: Option<f64> = None;
    for hook in TURN_HOOKS {
        let Some(holder) = trigger_holders_with_hook(before, hook, Some(owner))
            .into_iter()
            .find(|holder| {
                matches!(holder.zone, TriggerZone::Backrow | TriggerZone::Field)
                    && holder.card.id == instance_id
            })
        else {
            continue;
        };
        if let Some(price) = hook_firing_price(before, &holder, hook, counter)? {
            total = Some(total.unwrap_or(0.0) + price);
        }
    }
    Some(total)
}

/// R2477000, R2565000: `turn_hook_probe` for a Field Spell or a Unit in `seat`'s hand, set or placed
/// first (R923001's spawn): a Field Spell in the backrow zone a
/// set would take (`first_entry_zone`; a full backrow prices nothing), a Unit as `spawn_hand_unit` does.
fn spawned_turn_hook_probe(
    world: &GameState,
    seat: PlayerId,
    card: &CardInstance,
    counter: &dyn NodeCounter,
) -> Option<Option<f64>> {
    match card_type_of_face(world, &card.def_id, card.radiant) {
        CardType::Unit => {
            let Some((placed, unit)) = spawn_hand_unit(world, seat, card, counter)? else {
                return Some(None);
            };
            turn_hook_probe(&placed, seat, &unit.id, counter)
        }
        CardType::FieldSpell => {
            let Some(slot) = first_entry_zone(world, seat, Row::Backrow) else {
                return Some(None);
            };
            let spawn = SandboxOp::Spawn {
                player: seat,
                def_id: card.def_id.clone(),
                radiant: card.radiant.then_some(true),
                to: SpawnZone::Field {
                    row: Row::Backrow,
                    lane: slot.lane,
                },
            };
            let Some(placed) = sandbox_step(world, seat, spawn, counter)? else {
                return Some(None);
            };
            let Some(set) = card_at(&placed, slot).cloned() else {
                return Some(None);
            };
            turn_hook_probe(&placed, seat, &set.id, counter)
        }
        _ => Some(None),
    }
}

/// R2477000, R2565000: the turn-hook prices a decision of `seat`'s passes (`Prices`), probed on `world`
/// (R185), each card once, the seat's own copy first: its field and backrow cards with a turn hook, then the
/// enemy's units and the backrow cards the seat can read (`readable_by`), each priced as its controller's,
/// then each Field Spell or Unit in the seat's hand with one (R923001). Probed as `death_prices` is, at
/// most `AI_TURN_HOOK_PROBE_NODES` nodes from `counter`.
pub fn turn_hook_prices(world: &GameState, seat: PlayerId, counter: &dyn NodeCounter) -> Vec<TurnHookPrice> {
    if AI_EVAL.turn_hook == 0.0 || world.pending.is_some() || world.result.is_some() {
        return Vec::new();
    }
    let board: Vec<(PlayerId, CardInstance)> = [seat, seat.opponent()]
        .into_iter()
        .flat_map(|owner| {
            TURN_HOOKS
                .into_iter()
                .flat_map(move |hook| trigger_holders_with_hook(world, hook, Some(owner)))
        })
        .filter(|holder| match holder.zone {
            TriggerZone::Field => true,
            TriggerZone::Backrow => readable_by(world, &holder.card, seat),
            _ => false,
        })
        .map(|holder| (holder.controller, holder.card))
        .collect();
    let hand = world.players[seat]
        .hand
        .iter()
        .filter(|card| {
            matches!(
                card_type_of_face(world, &card.def_id, card.radiant),
                CardType::Unit | CardType::FieldSpell
            ) && has_turn_hook(world, card)
        })
        .map(|card| (seat, card, true));
    let mut candidates: Vec<(TurnHookPrice, PlayerId, &CardInstance, bool)> = Vec::new();
    for (owner, card, in_hand) in board
        .iter()
        .map(|(owner, card)| (*owner, card, false))
        .chain(hand)
    {
        if !candidates.iter().any(|(key, _, _, _)| key.is_for(card)) {
            candidates.push((TurnHookPrice::key(card), owner, card, in_hand));
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
    let probes = create_sub_counter(counter, AI_TURN_HOOK_PROBE_NODES);
    let mut priced = Vec::new();
    for (key, owner, card, in_hand) in candidates {
        let probed = if in_hand {
            spawned_turn_hook_probe(&sandbox, seat, card, &probes)
        } else {
            turn_hook_probe(&sandbox, owner, &card.id, &probes)
        };
        match probed {
            None => break,
            Some(None) => {}
            Some(Some(price)) => priced.push(TurnHookPrice { price, ..key }),
        }
    }
    priced
}

/// R2564000: `before` after `player` ends its turn, one node of `counter` (`simulate`, which answers
/// the other seat's prompts on the way), when that reaches the other player's main phase with no prompt
/// open and the game on. `None` when the counter is spent; `Some(None)` when the turn end fails or
/// reaches anything else: `player`'s own prompt, an extra turn, the game's end.
fn next_turn(before: &GameState, player: PlayerId, counter: &dyn NodeCounter) -> Option<Option<GameState>> {
    let Ok(next) = simulate(before, player, &ActionBody::EndTurn, counter, Prices::NONE)? else {
        return Some(None);
    };
    let ready = next.pending.is_none()
        && next.result.is_none()
        && next.active == player.opponent()
        && next.phase == Phase::Main;
    Some(ready.then_some(next))
}

/// R2564000: what the use of `unit`'s Activate that took `turn` to `after` gives its controller: the
/// controller's evaluation after it less before it, and `AI_EVAL.next_turn_mana` per crystal it gained
/// (less per crystal spent). `None` when the use leaves a prompt open, ends the game, takes the card off
/// the field, or gives nothing.
fn use_price(turn: &GameState, after: &GameState, unit: &CardInstance) -> Option<f64> {
    if after.pending.is_some() || after.result.is_some() {
        return None;
    }
    if !find_instance(after, &unit.id).is_some_and(|card| card.zone.z() == ZoneName::Field) {
        return None;
    }
    let owner = unit.controller;
    let gained = after.players[owner].mana.current - turn.players[owner].mana.current;
    let price = evaluate(after, owner, NextSwing::Enemy, &AI_EVAL)
        - evaluate(turn, owner, NextSwing::Enemy, &AI_EVAL)
        + AI_EVAL.next_turn_mana * f64::from(gained);
    (price > 0.0).then_some(price)
}

/// R2564000, R2603000: prices into `priced` the best use of each of `controller`'s cards with an Activate
/// on `turn`, the start of its turn (`activate_sources`), tried as the opponent's reply tries them
/// (`visible_activation`: only a card acting face-up that is none of `hidden`) and priced by `use_price`,
/// each card once (`ActivationPrice::is_for`). `false` when the counter is spent.
fn price_uses(
    turn: &GameState,
    controller: PlayerId,
    hidden: &IndexSet<String>,
    counter: &dyn NodeCounter,
    tried: &mut Vec<ActivationPrice>,
    priced: &mut Vec<ActivationPrice>,
) -> bool {
    let legal = legal_actions(turn, controller);
    for unit in activate_sources(turn, controller) {
        if hidden.contains(&unit.id)
            || tried.iter().any(|key| key.is_for(unit))
            || subsystems::abilities_of(turn, unit).is_empty()
        {
            continue;
        }
        tried.push(ActivationPrice::key(unit));
        let uses: Vec<ActionBody> = legal
            .iter()
            .filter(|action| matches!(action, ActionBody::Activate { instance_id, .. } if *instance_id == unit.id))
            .cloned()
            .collect();
        if uses.is_empty() {
            continue;
        }
        match visible_activation(turn, controller.opponent(), &uses, hidden, counter, Prices::NONE) {
            Known::Cut => return false,
            Known::Nothing => {}
            Known::Played(after) => {
                if let Some(price) = use_price(turn, &after, unit) {
                    priced.push(ActivationPrice {
                        price,
                        ..ActivationPrice::key(unit)
                    });
                }
            }
        }
    }
    true
}

/// R2564000, R2603000: the cards of `player`'s whose Activates the probes try, in this order: each unit
/// acting on its field (`active_units_of`: never a Dormant card), then each card acting face-up in its
/// backrow (`is_acting_on_field`: never a face-down card), a Field Spell among them, but for a Unit
/// standing there, which is one of its units already (R1081000).
fn activate_sources(state: &GameState, player: PlayerId) -> Vec<&CardInstance> {
    let mut cards = active_units_of(state, player);
    cards.extend(
        state.players[player]
            .backrow
            .iter()
            .flatten()
            .filter(|card| !is_standing_unit(state, card) && subsystems::is_acting_on_field(state, card)),
    );
    cards
}

/// R2564000, R2603000: whether any card the probes would try for `player` (`activate_sources`) has an
/// ability to use (`abilities_of`).
fn has_activate_card(world: &GameState, player: PlayerId) -> bool {
    activate_sources(world, player)
        .into_iter()
        .any(|card| !subsystems::abilities_of(world, card).is_empty())
}

/// R2564000, R2688000: how `probe_turns` prices a player's cards at the start of its turn (`price_uses`,
/// `price_attacks`); `false` when the counter is spent.
type PriceTurn<P> =
    fn(&GameState, PlayerId, &IndexSet<String>, &dyn NodeCounter, &mut Vec<P>, &mut Vec<P>) -> bool;

/// R2564000, R2603000, R2688000: the two turns `activation_prices` and `attack_hook_prices` walk on
/// `start`: `seat` ends its turn and `price` prices the enemy's cards at the start of the enemy's; then,
/// when `own` (the seat has a card to price), the enemy ends that turn and the seat's are priced at the
/// start of its next one. A turn the pass does not reach prices nothing, and nor does any turn after it.
fn probe_turns<P>(
    start: &GameState,
    seat: PlayerId,
    own: bool,
    counter: &dyn NodeCounter,
    price: PriceTurn<P>,
) -> Vec<P> {
    let hidden = hidden_card_ids(start, seat);
    let (mut tried, mut priced) = (Vec::new(), Vec::new());
    let enemy = seat.opponent();
    let Some(Some(enemy_turn)) = next_turn(start, seat, counter) else {
        return priced;
    };
    if !price(&enemy_turn, enemy, &hidden, counter, &mut tried, &mut priced) || !own {
        return priced;
    }
    if let Some(Some(own_turn)) = next_turn(&enemy_turn, enemy, counter) {
        price(&own_turn, seat, &hidden, counter, &mut tried, &mut priced);
    }
    priced
}

/// R2564000, R2603000: the activation prices a decision of `seat`'s passes (`Prices`), probed on `world`
/// (R185): one use of the Activate of each unit acting on either field and each card acting face-up in
/// either backrow, each once. The enemy's are tried at the start of its next turn, the seat's at the start
/// of its own (`probe_turns`), so a use already spent this turn hides nothing. Automatic turn end off
/// (R345), no price passed, at most `AI_ACTIVATION_PROBE_NODES` nodes.
pub fn activation_prices(
    world: &GameState,
    seat: PlayerId,
    counter: &dyn NodeCounter,
) -> Vec<ActivationPrice> {
    if AI_EVAL.activation == 0.0
        || world.pending.is_some()
        || world.result.is_some()
        || world.active != seat
        || world.phase != Phase::Main
    {
        return Vec::new();
    }
    let own = has_activate_card(world, seat);
    if !own && !has_activate_card(world, seat.opponent()) {
        return Vec::new();
    }
    let mut start = world.clone();
    for player in PLAYER_IDS {
        start.players[player].auto_end_turn = Some(false);
    }
    let probes = create_sub_counter(counter, AI_ACTIVATION_PROBE_NODES);
    probe_turns(&start, seat, own, &probes, price_uses)
}

/// R2688000: `turn` with `unit` as its hookless twin, and the keywords it prints: the twin is Vanilla
/// (§6.3), so `script_of` runs none of its text and neither of its attack hooks fires, and it is
/// granted the keywords it prints, as tuning leaves them (`face_of`), so it fights as itself.
fn hookless_twin(turn: &GameState, unit: &CardInstance) -> (GameState, Vec<Keyword>) {
    let printed = face_of(turn, unit).keywords;
    let mut twin = turn.clone();
    if let Some(card) = find_instance_mut(&mut twin, &unit.id) {
        card.granted_keywords.extend(printed.iter().cloned());
        card.vanilla = true;
    }
    (twin, printed)
}

/// R2688000: `after`, the hookless twin's attack, with the unit's text given back before it is scored:
/// Vanilla off and the printed keywords it was granted taken off again, one of each, so its Brittle,
/// its lent keywords, its auras and its set stats read on both sides of the price as they do on the
/// unit's own. A twin the attack took off the field has nothing to give back.
fn with_text_back(mut after: GameState, unit_id: &str, printed: &[Keyword]) -> GameState {
    if let Some(card) = find_instance_mut(&mut after, unit_id) {
        card.vanilla = false;
        for keyword in printed {
            if let Some(at) = card
                .granted_keywords
                .iter()
                .position(|granted| granted == keyword)
            {
                card.granted_keywords.remove(at);
            }
        }
    }
    after
}

/// R2688000: the attack the probe plays for `unit`, of those `legal_actions` lists (`legal`): on the
/// enemy hero where the rules allow it, else the first target listed. `None` when it has no attack.
fn probe_attack(legal: &[ActionBody], unit: &CardInstance) -> Option<ActionBody> {
    let hero = format!("hero-{}", unit.controller.opponent().as_str());
    let attacks: Vec<&ActionBody> = legal
        .iter()
        .filter(|action| matches!(action, ActionBody::Attack { attacker_id, .. } if *attacker_id == unit.id))
        .collect();
    let on_hero = attacks
        .iter()
        .copied()
        .find(|action| matches!(action, ActionBody::Attack { target_id, .. } if *target_id == hero));
    on_hero.or(attacks.first().copied()).cloned()
}

/// R2688000: what `unit`'s `attack` on `turn`, the start of its controller's turn, gives its controller
/// through its attack hooks: its evaluation after it less after the same attack by its hookless twin
/// (`with_text_back`), plus `AI_EVAL.next_turn_mana` per crystal gained over the twin, which the
/// evaluation does not count. Signed: a costly hook is a debt. `None` when the counter is spent;
/// `Some(None)` when either attack is refused or leaves a prompt open or the game over.
fn attack_hook_probe(
    turn: &GameState,
    unit: &CardInstance,
    attack: &ActionBody,
    counter: &dyn NodeCounter,
) -> Option<Option<f64>> {
    let owner = unit.controller;
    let settled = |after: &GameState| after.pending.is_none() && after.result.is_none();
    let Ok(hooked) = simulate(turn, owner, attack, counter, Prices::NONE)? else {
        return Some(None);
    };
    if !settled(&hooked) {
        return Some(None);
    }
    let (twin, printed) = hookless_twin(turn, unit);
    let Ok(plain) = simulate(&twin, owner, attack, counter, Prices::NONE)? else {
        return Some(None);
    };
    if !settled(&plain) {
        return Some(None);
    }
    let plain = with_text_back(plain, &unit.id, &printed);
    let gained = hooked.players[owner].mana.current - plain.players[owner].mana.current;
    Some(Some(
        evaluate(&hooked, owner, NextSwing::Enemy, &AI_EVAL)
            - evaluate(&plain, owner, NextSwing::Enemy, &AI_EVAL)
            + AI_EVAL.next_turn_mana * f64::from(gained),
    ))
}

/// R2688000: prices into `priced` the next attack of each of `controller`'s units with an attack hook
/// on `turn`, the start of its turn, but none of `hidden`, each card once (`tried`): the attack
/// `probe_attack` picks, priced by `attack_hook_probe` and kept signed, 0 included. `false` when the
/// counter is spent.
fn price_attacks(
    turn: &GameState,
    controller: PlayerId,
    hidden: &IndexSet<String>,
    counter: &dyn NodeCounter,
    tried: &mut Vec<AttackHookPrice>,
    priced: &mut Vec<AttackHookPrice>,
) -> bool {
    let legal = legal_actions(turn, controller);
    for unit in active_units_of(turn, controller) {
        if hidden.contains(&unit.id)
            || tried.iter().any(|key| key.is_for(unit))
            || !has_attack_hook(turn, unit)
        {
            continue;
        }
        tried.push(AttackHookPrice::key(unit));
        let Some(attack) = probe_attack(&legal, unit) else {
            continue;
        };
        match attack_hook_probe(turn, unit, &attack, counter) {
            None => return false,
            Some(None) => {}
            Some(Some(price)) => priced.push(AttackHookPrice {
                price,
                ..AttackHookPrice::key(unit)
            }),
        }
    }
    true
}

/// R2688000: whether any unit acting on `player`'s field (`active_units_of`) has an attack hook.
fn has_attack_hook_unit(world: &GameState, player: PlayerId) -> bool {
    active_units_of(world, player)
        .into_iter()
        .any(|unit| has_attack_hook(world, unit))
}

/// R2688000: the attack-hook prices a decision of `seat`'s passes (`Prices`), probed on `world` (R185):
/// one attack of each unit acting on either field with an attack hook, each once, against the same attack
/// by its hookless twin, at the start of its controller's next turn (`probe_turns`), so an attack already
/// made this turn hides nothing. Automatic turn end off (R345), no price passed, at most
/// `AI_ATTACK_HOOK_PROBE_NODES` nodes.
pub fn attack_hook_prices(
    world: &GameState,
    seat: PlayerId,
    counter: &dyn NodeCounter,
) -> Vec<AttackHookPrice> {
    if AI_EVAL.attack_hook == 0.0
        || world.pending.is_some()
        || world.result.is_some()
        || world.active != seat
        || world.phase != Phase::Main
    {
        return Vec::new();
    }
    let own = has_attack_hook_unit(world, seat);
    if !own && !has_attack_hook_unit(world, seat.opponent()) {
        return Vec::new();
    }
    let mut start = world.clone();
    for player in PLAYER_IDS {
        start.players[player].auto_end_turn = Some(false);
    }
    let probes = create_sub_counter(counter, AI_ATTACK_HOOK_PROBE_NODES);
    probe_turns(&start, seat, own, &probes, price_attacks)
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

/// `evaluate_priced` with no price (`Prices::NONE`): the evaluation outside a decision.
pub fn evaluate(state: &GameState, seat: PlayerId, next: NextSwing, w: &EvalWeights) -> f64 {
    evaluate_priced(state, seat, next, w, Prices::NONE)
}

/// A score for `seat` (higher is better). Pure, cheap (O(cards)), reads only public-or-sampled state.
/// `w` is the AI's own AI_EVAL unless a caller brings its own weights (the greedy baseline's
/// GREEDY_EVAL, which tuning the AI never moves); `prices` are the decision's (R2424000, R2477000,
/// R2564000, R2688000).
pub fn evaluate_priced(
    state: &GameState,
    seat: PlayerId,
    next: NextSwing,
    w: &EvalWeights,
    prices: &Prices,
) -> f64 {
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

    let board = material(state, seat, seat, w, prices) - material(state, opp, seat, w, prices);

    hero_side(state, seat, w) - hero_side(state, opp, w) + board - threat + pressure + closing - race
}
