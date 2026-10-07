//! Cost rules (docs/classic-sets.md B5 E15, SPEC §6.3 Cost, R65, R363, R455): the rungs a card's price
//! climbs past on its way through `mana::effective_cost`, where they come from, and the plays a price
//! rule forbids outright.
//!
//! A price rule comes from one of two places, and both carry the same data (`CostRule`):
//!
//!  - a player modifier (`kind: "costRule"`, `state.rs`) — Classic #2 The Trickster's "your next Trap
//!    or Field Spell costs (2) less" (or "costs (0)", its Radiant face), until used; AI Alignment
//!    Tax's "your opponent's cards cost (1) more during their next turn", R48's timing turned outward;
//!  - an aura (`Script.cost_aura`) on a card acting on the field — Classic #6's "your Traps cost (0)",
//!    #68's "Cost (3)+ cards cost (1) more" and its Radiant "your opponent can't play Cost (3)+ cards",
//!    #77's "Spells cost (1) more". The aura is a hook, not a static flag, so a Degrade or Upgrade of
//!    the card's declared numbers (B3.4, `CardDef.params`) moves the rule it lays down.
//!
//! The ladder itself (R455) is `mana::effective_cost`'s, which reads these through `price_rules_for`:
//! flat changes first (R65's player discounts, then every rule with an `amount` and no threshold),
//! then the threshold rules, each testing the one number the flat changes left (R363 reads Professor
//! Curvature there, and #68's "(3)+" reads the same number, before its own surcharge), then "costs
//! (N)" sets, then the floor a card carries (Classic+ #14 Forever&'s "can't cost less than (2)",
//! `enchantments.rs`), then 0. A ban ("can't play") reads the finished price and refuses the play
//! (`why_play_banned`); a cast pays nothing and is never refused (R70).
//!
//! The module imports nothing from `mana.rs`, which imports it: liveness of a modifier is the caller's
//! question (`mana::modifier_is_live`), so the two never form a cycle.
//!
//! Port of `packages/engine/src/costRules.ts`. `CostRule` lives in `state.rs` (a `PlayerModifier`
//! holds one) and `CostAura`, `CostAuraWhose` and `CostAuraArgs` in `script.rs` (a `Script` field
//! names them); this module uses those and defines none of them again.

use indexmap::IndexSet;

use crate::config::GLITCH_DEF_ID;
use crate::script::{CostAura, CostAuraWhose, HookArgs};
use crate::state::{CardInstance, CostRule, GameState, ModifierExpiry, ModifierKind, PlayerModifier};
use crate::wire::{CardType, Enchantment, PLAYER_IDS, PlayerId, Row, opponent_of};

/// A `costRule` modifier (`state.rs`), named so the readers here can take one: the modifier's id, its
/// expiry and the rule it carries (TS `Extract<PlayerModifier, { kind: "costRule" }>`, whose `rule`
/// sits beside `id` and `expiry`).
#[derive(Clone, Debug, PartialEq)]
pub struct CostRuleModifier {
    pub id: String,
    pub expiry: ModifierExpiry,
    pub rule: CostRule,
}

impl CostRuleModifier {
    /// The modifier as a `costRule` one, or `None` when it is of another kind (TS's type guard).
    pub fn of(modifier: &PlayerModifier) -> Option<CostRuleModifier> {
        match &modifier.kind {
            ModifierKind::CostRule { rule } => Some(CostRuleModifier {
                id: modifier.id.clone(),
                expiry: modifier.expiry.clone(),
                rule: rule.clone(),
            }),
            _ => None,
        }
    }
}

/// `price_rules_for`'s answer: the live `costRule` modifiers and the aura rules that reach a card.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PriceRules {
    pub mods: Vec<CostRuleModifier>,
    pub auras: Vec<CostAura>,
}

/// `climb_price_rules`'s answer: the price, and the ids of the modifiers whose rule changed it.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ClimbedPrice {
    pub price: i32,
    pub used: Vec<String>,
}

// ---------------------------------------------------------------------------
// Matching
// ---------------------------------------------------------------------------

/// Whether a rule's type list reaches this card (all types when it names none).
pub fn rule_reaches(state: &GameState, rule: &CostRule, card: &CardInstance) -> bool {
    match &rule.types {
        None => true,
        Some(types) if types.is_empty() => true,
        Some(types) => types.contains(&crate::faces::card_type_of(state, card)),
    }
}

/// Whether an aura laid by a card `source` controls reaches a card `player` would play.
fn whose_reaches(whose: CostAuraWhose, source: PlayerId, player: PlayerId) -> bool {
    if whose == CostAuraWhose::All {
        return true;
    }
    if whose == CostAuraWhose::Yours {
        player == source
    } else {
        player == opponent_of(source)
    }
}

/// Every card acting on the field, both sides, in R68's walk (the active side first, units by lane,
/// then the backrow by lane): the tops of the unit piles and the backrow cards. A card dormant under a
/// Stack pile is not on the field for effects (§3.2, R13), so its aura is off, as §10.4 layer 5 reads.
fn acting_permanents(state: &GameState) -> Vec<&CardInstance> {
    let order: [PlayerId; 2] = if state.active == PlayerId::P1 {
        PLAYER_IDS
    } else {
        [PLAYER_IDS[1], PLAYER_IDS[0]]
    };
    let mut out: Vec<&CardInstance> = Vec::new();
    for player in order {
        for row in [Row::Units, Row::Backrow] {
            for slot in crate::zones::slots_of(player, row) {
                if let Some(card) = crate::zones::card_at(state, slot) {
                    out.push(card);
                }
            }
        }
    }
    out
}

/// E15: the aura rules on the field that reach a card `player` would play, in R68's walk. A Vanilla
/// card lays none (`script_of` runs no script for it, §6.3, R115).
pub fn cost_auras_for(state: &GameState, player: PlayerId, card: &CardInstance) -> Vec<CostAura> {
    let mut out: Vec<CostAura> = Vec::new();
    for source in acting_permanents(state) {
        let script = crate::scripts::script_of(state, source);
        let Some(hook) = script.cost_aura.as_ref() else {
            continue;
        };
        for aura in hook(HookArgs {
            state,
            self_: source,
            radiant: source.radiant,
        }) {
            if !whose_reaches(aura.whose, source.controller, player) {
                continue;
            }
            if !rule_reaches(state, &aura.rule, card) {
                continue;
            }
            out.push(aura);
        }
    }
    out
}

/// R455: the price rules on a card `player` would play — the live `costRule` modifiers of that player
/// that reach it (`live` is `mana::modifier_is_live`) and the aura rules of the field — the bans left
/// out, since a ban prices nothing (`why_play_banned` reads them).
pub fn price_rules_for(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    live: impl Fn(&PlayerModifier) -> bool,
) -> PriceRules {
    let mods: Vec<CostRuleModifier> = state.players[player]
        .mods
        .iter()
        .filter(|modifier| match &modifier.kind {
            ModifierKind::CostRule { rule } => live(modifier) && rule_reaches(state, rule, card),
            _ => false,
        })
        .filter_map(CostRuleModifier::of)
        .collect();
    let auras: Vec<CostAura> = cost_auras_for(state, player, card)
        .into_iter()
        .filter(|aura| aura.ban != Some(true))
        .collect();
    PriceRules { mods, auras }
}

/// R455: walk the rungs over a price that has had R65's own player discounts, and return the price and
/// the modifiers whose rule changed it (a threshold rule counts only when it met its threshold). The
/// caller applies R363's Curvature at `before_thresholds` itself, since that discount is R65's.
pub fn climb_price_rules(price: i32, rules: &PriceRules, curvature: impl Fn(i32) -> i32) -> ClimbedPrice {
    let mut used: IndexSet<String> = IndexSet::new();
    let mut all: Vec<(&CostRule, Option<&str>)> = Vec::new();
    for modifier in &rules.mods {
        all.push((&modifier.rule, Some(modifier.id.as_str())));
    }
    for aura in &rules.auras {
        all.push((&aura.rule, None));
    }
    let mut cost = price;
    // Flat rungs: every add with no threshold.
    for (rule, id) in &all {
        let Some(amount) = rule.amount else {
            continue;
        };
        if rule.min_cost.is_some() {
            continue;
        }
        cost += amount;
        if let Some(id) = id {
            used.insert((*id).to_string());
        }
    }
    // Threshold rungs, each reading the one number the flat rungs left (R363).
    let before = cost;
    cost -= curvature(before);
    for (rule, id) in &all {
        let (Some(min_cost), Some(amount)) = (rule.min_cost, rule.amount) else {
            continue;
        };
        if before < min_cost {
            continue;
        }
        cost += amount;
        if let Some(id) = id {
            used.insert((*id).to_string());
        }
    }
    // "Costs (N)": the lowest set wins over every add.
    let mut set: Option<i32> = None;
    for (rule, id) in &all {
        let Some(set_to) = rule.set_to else {
            continue;
        };
        if let Some(min_cost) = rule.min_cost
            && before < min_cost
        {
            continue;
        }
        set = Some(match set {
            None => set_to,
            Some(lowest) => lowest.min(set_to),
        });
        if let Some(id) = id {
            used.insert((*id).to_string());
        }
    }
    if let Some(set) = set {
        cost = set;
    }
    ClimbedPrice {
        price: cost,
        used: used.into_iter().collect(),
    }
}

// ---------------------------------------------------------------------------
// The floor a card carries (E39 via E15)
// ---------------------------------------------------------------------------

/// Classic+ #14 Forever&: "This can't cost less than (N)" — the `returnAfterResolve` enchantment's
/// floor, applied after every discount (E15, R455). Several floors: the highest holds. 0 without one.
pub fn cost_floor_of(card: &CardInstance) -> i32 {
    card.enchantments
        .iter()
        .flatten()
        .fold(0, |floor, entry| match entry {
            Enchantment::ReturnAfterResolve { floor: own } => floor.max(*own),
            _ => floor,
        })
}

// ---------------------------------------------------------------------------
// Bans (E15, R455)
// ---------------------------------------------------------------------------

/// E15, R455: why a play of this card at this price is forbidden, or `Ok` when it is not. Classic #68
/// Radiant's "your opponent can't play Cost (3)+ cards" reads the finished price of this very play (an
/// X card's X included), so `legal_actions` never offers it and §10.5 step 1 refuses it; a cast is not
/// asked (R70).
pub fn why_play_banned(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    price: i32,
) -> Result<(), crate::state::EngineError> {
    // R675: no rule forbids a play of Glitch.
    if card.def_id == GLITCH_DEF_ID {
        return Ok(());
    }
    for aura in cost_auras_for(state, player, card) {
        if aura.ban != Some(true) {
            continue;
        }
        let min_cost = aura.rule.min_cost.unwrap_or(0);
        if price >= min_cost {
            return Err(crate::state::EngineError::new(format!(
                "you can't play ({min_cost})+ Cost cards now"
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// What the badge says (R169, R432's "(N) Cost" noun, "costs (N)" verb)
// ---------------------------------------------------------------------------

fn types_text(types: Option<&[CardType]>, plural: bool) -> String {
    let types = match types {
        Some(types) if !types.is_empty() => types,
        _ => {
            return if plural {
                "cards".to_string()
            } else {
                "card".to_string()
            };
        }
    };
    // "Trap" says "Field Trap" too, so a list holding both is read as the one word.
    let shown: Vec<CardType> = types
        .iter()
        .copied()
        .filter(|card_type| !(*card_type == CardType::FieldTrap && types.contains(&CardType::Trap)))
        .collect();
    let words: Vec<String> = shown
        .iter()
        .map(|card_type| {
            if plural {
                format!("{card_type}s")
            } else {
                card_type.to_string()
            }
        })
        .collect();
    if words.len() == 1 {
        return words[0].clone();
    }
    let last = words.last().cloned().unwrap_or_else(|| "undefined".to_string());
    let head = &words[..words.len().saturating_sub(1)];
    format!("{} or {}", head.join(", "), last)
}

/// The sentence a rule says, for one card ("next") or for every card it reaches.
pub fn cost_rule_text(rule: &CostRule, next: bool) -> String {
    let plural = !next;
    let threshold = match rule.min_cost {
        None => String::new(),
        Some(min_cost) => format!("({min_cost})+ Cost "),
    };
    let subject = format!(
        "{}{}{}",
        if next { "Your next " } else { "Your " },
        threshold,
        types_text(rule.types.as_deref(), plural)
    );
    let verb = if plural { "cost" } else { "costs" };
    if let Some(set_to) = rule.set_to {
        return format!("{subject} {verb} ({set_to})");
    }
    let amount = rule.amount.unwrap_or(0);
    if amount < 0 {
        return format!("{subject} {verb} ({}) less", -amount);
    }
    format!("{subject} {verb} ({amount}) more")
}

/// A `costRule` modifier's badge caption: "next" while it waits to be used, else every card. (TS takes
/// `{ rule, expiry }`, the narrowed modifier; a modifier of another kind reads as an empty rule.)
pub fn cost_rule_modifier_label(modifier: &PlayerModifier) -> String {
    let rule = match &modifier.kind {
        ModifierKind::CostRule { rule } => rule.clone(),
        _ => CostRule::default(),
    };
    cost_rule_text(&rule, modifier.expiry == ModifierExpiry::Used)
}

/// An `enchantNextSpell` modifier's badge caption.
pub fn enchant_next_spell_label(enchantment: &Enchantment) -> String {
    match enchantment {
        Enchantment::ReturnAfterResolve { floor } => format!(
            "Your next Spell returns to your hand after it resolves (it can't cost less than ({floor}))"
        ),
        Enchantment::CastOnDraw => "Your next Spell gains Cast on draw".to_string(),
        Enchantment::TargetEnemies => {
            "Your next Spell aims at enemies when it harms and at your side when it helps".to_string()
        }
        Enchantment::SwapsBook { .. } => {
            "Your next Spell becomes a different Book at the end of your turn".to_string()
        }
    }
}
