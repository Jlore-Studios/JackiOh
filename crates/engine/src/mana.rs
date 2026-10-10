//! Mana refresh, temporary mana and the cost calculation (SPEC §2.3, §6.3 Cost, R65, R396, R455).
//!
//! Port of `packages/engine/src/mana.ts`.

use crate::catalog::def_of;
use crate::config::GLITCH_DEF_ID;
use crate::cost_rules::{climb_price_rules, cost_floor_of, price_rules_for};
use crate::graveyard_play::playable_from_graveyard;
use crate::script::CostArgs;
use crate::state::{
    CardInstance, GameState, ModifierExpiry, ModifierKind, PlayerModifier, PlayerState, handicap_of,
};
use crate::wire::{CardCost, GameEvent, PlayerId, Zone};

/// §2.3, R181: max mana is min(turns started + the seat's mana bonus, its mana cap), plus persistent
/// modifiers, floored at 0. With no handicap the bonus is 0 and the cap is MAX_MANA, which is §2.3's
/// "min(number of turns you have started, 4), plus persistent modifiers". `nextTurnMod` is not one of
/// those: it is a one-shot rider on a single refresh, which the refresh spends and clears, so it never
/// reaches max mana. Hinder and every other modifier apply on top of the capped value exactly as for
/// a human.
pub fn max_mana_for(side: &PlayerState) -> i32 {
    let handicap = handicap_of(side);
    let base = (side.turns_started + handicap.mana_bonus).min(handicap.mana_cap);
    (base + side.mana.perm_mod).max(0)
}

/// R169: the id the next refresh's rider (`mana.nextTurnMod`, §6.3 Mana: "'next turn' mana is stored
/// as a modifier for the next refresh") travels under — one badge per player, which `modifierChanged`
/// names as it appears, changes and is spent, and which the view lists while the rider is not 0.
pub const NEXT_REFRESH_MODIFIER_ID: &str = "nextTurnMana";

/// Start of turn: refresh to max, moved by the one-shot rider (§6.3 Mana: "'next turn' mana is stored
/// as a modifier for the next refresh"), which is then cleared. The rider changes what the refresh
/// gives, not max mana: #24 Efficiency Dividend's next-turn mana is temporary mana on §2.3's list and
/// "adds to current mana and can exceed 4", exactly as #6 Mana Well's gain does, and #21 Hinder
/// "subtracts from the opponent's next refresh". Current mana never goes below 0 (§2.3).
pub fn refresh_mana(side: &mut PlayerState) {
    let max = max_mana_for(side);
    side.mana.max = max;
    side.mana.current = refreshed_mana(side, max);
    side.mana.next_turn_mod = 0;
    // R1224: ME-TURN's repayment schedule — the next instalment comes off the refresh, after any
    // rider. A lost refresh forgives the instalment: there is nothing to take it from.
    crate::credit::take_instalment(side);
}

/// R844 (Meditative #18, #19): the id "lose all mana next N turns" travels under — one badge per
/// player, which `modifierChanged` names as it appears and as it goes, and which the view lists
/// while a loss covers a future refresh (R169).
pub const LOST_REFRESH_MODIFIER_ID: &str = "lostRefresh";

/// R844: whether this player's refresh is lost — their `lost_refresh_through` covers the turn they
/// have started (`turns_started`).
pub fn refresh_is_lost(side: &PlayerState) -> bool {
    side.lost_refresh_through
        .is_some_and(|through| side.turns_started <= through)
}

/// R844: what a refresh gives. A lost refresh gives 0 and spends the rider with it; otherwise max
/// plus the rider, floored at 0. MB09's zeroed-refresh aura (#26) and MB23's repayment schedule
/// (#89) extend this branch.
pub fn refreshed_mana(side: &PlayerState, max: i32) -> i32 {
    if refresh_is_lost(side) {
        0
    } else {
        (max + side.mana.next_turn_mod).max(0)
    }
}

/// R844: how many lost refreshes this player still has past this one — `through − turns_started`,
/// never below 0.
pub fn lost_refreshes_left(side: &PlayerState) -> i32 {
    side.lost_refresh_through
        .map(|through| (through - side.turns_started).max(0))
        .unwrap_or(0)
}

/// R844: clear a spent loss once no lost refresh is left. Returns true when the field was set and
/// is now gone, so the turn can report the badge going with it.
pub fn spend_lost_refresh(side: &mut PlayerState) -> bool {
    if side.lost_refresh_through.is_some() && lost_refreshes_left(side) == 0 {
        side.lost_refresh_through = None;
        return true;
    }
    false
}

/// §6.3 Refresh, R364: give back up to `amount` spent mana, never past max — Hearthstone's "Refresh
/// Mana Crystals" (#78 /fullsend). Unlike temporary mana (`gain_mana`) it cannot take current above
/// max, and a player already at or above max gains nothing.
pub fn refresh_some_mana(side: &mut PlayerState, amount: i32) {
    if amount <= 0 || side.mana.current >= side.mana.max {
        return;
    }
    // Saturating: "refresh all your mana" (Classic+ #27) passes `i32::MAX`, TS's `Infinity`.
    side.mana.current = side.mana.max.min(side.mana.current.saturating_add(amount));
}

/// Temporary mana may take current above max (§2.3).
pub fn gain_mana(side: &mut PlayerState, amount: i32) {
    side.mana.current = (side.mana.current + amount).max(0);
}

pub fn spend_mana(side: &mut PlayerState, amount: i32) {
    side.mana.current = (side.mana.current - amount).max(0);
}

pub fn mana_event(player: PlayerId, side: &PlayerState) -> GameEvent {
    GameEvent::ManaChanged {
        player,
        current: side.mana.current,
        max: side.mana.max,
    }
}

/// The printed cost as it stands: X uses the chosen X, an embiggen card the chosen price (R65).
pub fn printed_cost(state: &GameState, instance: &CardInstance) -> i32 {
    let script = crate::scripts::script_of(state, instance);
    if let Some(cost) = script.cost.as_ref() {
        return cost(CostArgs { state, instance }).max(0);
    }

    let cost: CardCost = def_of(Some(state), &instance.def_id).cost;
    match cost {
        CardCost::X => instance.x.unwrap_or(0).max(0),
        CardCost::Fixed(n) => n,
        CardCost::Embiggen { base, embiggen } => {
            if instance.embiggened == Some(true) {
                embiggen
            } else {
                base
            }
        }
    }
}

pub fn is_x_cost(state: &GameState, instance: &CardInstance) -> bool {
    def_of(Some(state), &instance.def_id).cost == CardCost::X
}

/// R48: a modifier that covers a player's *next* turn does nothing on the turn it was created on,
/// and nothing on the opponent's turn in between either — #77's text is "during **your** next turn",
/// so it is live only once that player is the active one on a later turn. `expireModifiers` ends it
/// at the cleanup of that player's next turn, which is why this is a separate question from expiry.
pub fn modifier_is_live(state: &GameState, modifier: &PlayerModifier) -> bool {
    // R757: Armor Up's Armor holds from the moment it is gained until its expiry's cleanup removes it.
    if matches!(modifier.kind, ModifierKind::HeroArmor { .. }) {
        return true;
    }
    match modifier.expiry {
        // §2.2: "this turn" is the turn it names, and no later one — even when it was made after that
        // turn's cleanup had run and so outlives it until the next cleanup (`expireModifiers`).
        ModifierExpiry::ThisTurn { turn } => turn >= state.turn,
        ModifierExpiry::NextTurnOf { player, from_turn } => state.turn > from_turn && state.active == player,
        _ => true,
    }
}

/// How a price is read. `asPlay` prices the card as a play of it now wherever it lies: a card played
/// from a graveyard (B5 E11, R454) pays the player's prices exactly as a card played from a hand does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CostOptions {
    pub as_play: Option<bool>,
}

/// What `price_of` works out: the price, and the `costRule` modifiers that changed it (spent by a play).
struct Price {
    cost: i32,
    used_rules: Vec<String>,
}

/// R65: start from costOverride or the printed cost, add the instance's costMod, add the player's
/// discounts, then Professor Curvature if the result is 4 or more (R363), and floor at 0. An X-cost
/// card costs exactly X and ignores modifiers, unless an override makes it free.
///
/// The player's discounts and Curvature are prices for a play — §6.3's Cost is "what a card costs to
/// play now", #35's is "the next Spell you play", #78's "this turn your cards cost 1 less", and R48
/// reads Curvature "at play time" — so they reach a card in its controller's hand, where a play takes
/// it from (§10.5 step 1), and no other. A card in a library or a graveyard is read at its own cost,
/// its `costOverride` or printed cost with its `costMod`: #30 Archivist's "highest" (R24), #94's
/// 2-cost draw and odd-cost exile (R66), a Recruit's filter — as Hearthstone's hand discounts never
/// reach the deck or the graveyard (R65) — except a graveyard a permission lets its player play from
/// (E11, R454), where a play takes the card from. `options.asPlay` prices any card as a play of it now.
/// (TS's `options` defaulted to `{}`; a caller with none passes `CostOptions::default()`.)
///
/// R455 (E15) adds its rungs through `cost_rules.rs`: after R65's discounts, the flat price rules (the
/// `costRule` modifiers and the field's cost auras), then the threshold rules, which read the one
/// number the flat ones left, as R363 reads Curvature there; then "costs (N)" sets; then the floor the
/// card carries (Forever&'s "can't cost less than (N)"), which holds in every zone, as the card's own
/// `costOverride` does; then 0. An X-cost card still ignores every modifier (R65), and only its floor
/// reaches it.
pub fn effective_cost(state: &GameState, instance: &CardInstance, options: CostOptions) -> i32 {
    price_of(state, instance, options).cost
}

/// R454, R455: what a play of this card now costs, wherever the play takes it from.
pub fn play_cost(state: &GameState, instance: &CardInstance) -> i32 {
    effective_cost(state, instance, CostOptions { as_play: Some(true) })
}

/// R455: the `costRule` modifiers a play of this card at this price spends — the ones "until used" that
/// changed its price (Classic #2's next Trap or Field Spell). A cast spends none, since it pays nothing
/// (R70), and an X-cost card none, since no modifier reaches it (R65).
pub fn cost_rules_spent_by(state: &GameState, instance: &CardInstance) -> Vec<String> {
    let Price { used_rules, .. } = price_of(state, instance, CostOptions { as_play: Some(true) });
    let mods = &state.players[instance.controller].mods;
    used_rules
        .into_iter()
        .filter(|id| {
            mods.iter()
                .any(|modifier| &modifier.id == id && modifier.expiry == ModifierExpiry::Used)
        })
        .collect()
}

fn price_of(state: &GameState, instance: &CardInstance, options: CostOptions) -> Price {
    let side = &state.players[instance.controller];
    let override_ = instance.cost_override;
    let floor = cost_floor_of(instance);
    // R675: Glitch costs (0) wherever it is and whatever would change that.
    if instance.def_id == GLITCH_DEF_ID {
        return Price {
            cost: 0,
            used_rules: Vec::new(),
        };
    }

    // R65: X-cost cards cost exactly X and ignore modifiers, but an override makes one free. R455: a
    // floor the card carries is its own, and holds.
    if is_x_cost(state, instance) {
        let printed = if override_.is_some() {
            0
        } else {
            printed_cost(state, instance)
        };
        return Price {
            cost: 0.max(printed).max(floor),
            used_rules: Vec::new(),
        };
    }

    let mut cost = override_.unwrap_or_else(|| printed_cost(state, instance)) + instance.cost_mod;
    // R65: a player's discounts price a play, and a play takes a card from its hand — or from its
    // graveyard, while a permission lets its player play it from there (E11, R454).
    let for_play = options.as_play == Some(true)
        || matches!(instance.zone, Zone::Hand { .. })
        || (matches!(instance.zone, Zone::Graveyard { .. }) && playable_from_graveyard(state, instance));
    if !for_play {
        return Price {
            cost: 0.max(cost).max(floor),
            used_rules: Vec::new(),
        };
    }
    let type_ = crate::alt_play::priced_type_of(state, instance);

    for modifier in &side.mods {
        let ModifierKind::CostDiscount {
            amount,
            only_type,
            min_current_cost,
            ..
        } = &modifier.kind
        else {
            continue;
        };
        if !modifier_is_live(state, modifier) {
            continue;
        }
        if let Some(only) = only_type
            && *only != type_
        {
            continue;
        }
        if min_current_cost.is_some() {
            continue; // Curvature is applied below.
        }
        cost -= amount;
    }

    // R65, R363: "apply Professor Curvature if the result is then 4 or more" — the result of the steps
    // above, which every live Curvature reads. Two of them (#39's copy, #33's) each test that one
    // number, so both apply to a card the discounts leave at 4 or more, and the order they were played
    // in changes nothing: a Curvature never reads the cost another Curvature has already lowered (R48).
    // R455: the flat price rules come first, and the threshold rules read the same number Curvature does.
    let curvature = |before_curvature: i32| -> i32 {
        let mut off = 0;
        for modifier in &side.mods {
            let ModifierKind::CostDiscount {
                amount,
                min_current_cost: Some(min_current_cost),
                ..
            } = &modifier.kind
            else {
                continue;
            };
            if !modifier_is_live(state, modifier) {
                continue;
            }
            if before_curvature >= *min_current_cost {
                off += amount;
            }
        }
        off
    };
    let live = |modifier: &PlayerModifier| modifier_is_live(state, modifier);
    let rules = price_rules_for(state, instance.controller, instance, live);
    let climbed = climb_price_rules(cost, &rules, curvature);

    Price {
        cost: 0.max(climbed.price).max(floor),
        used_rules: climbed.used,
    }
}

pub fn can_afford(state: &GameState, instance: &CardInstance) -> bool {
    // R1223: affordability reads the one function — current mana plus the credit line's offer.
    effective_cost(state, instance, CostOptions::default())
        <= crate::credit::spendable_mana(state, instance.controller)
}

/// R396 (Classic #10, #18, #25, #32, #39): what a card costs wherever a rule compares or counts costs —
/// the one reader card scripts compare costs with. An X-cost card on the field costs the X it was
/// played for (the instance's `x`); anywhere else, and on the field when it arrived without a chosen X
/// (a Recruit, a summon), it costs 0 (R65). An embiggen card outside a play costs its base price (R65),
/// on the field too. Any other card is R65's cost as it stands: a hand card at its hand cost, a card in
/// a library, a graveyard or on the field at its own (R66). A floor the card carries holds (R455).
pub fn cost_now(state: &GameState, card: &CardInstance) -> i32 {
    let floor = cost_floor_of(card);
    if is_x_cost(state, card) {
        let x = if matches!(card.zone, Zone::Field { .. }) {
            card.x.unwrap_or(0)
        } else {
            0
        };
        return 0.max(x).max(floor);
    }
    if card.embiggened == Some(true) && !matches!(card.zone, Zone::Resolving { .. }) {
        let unembiggened = CardInstance {
            embiggened: Some(false),
            ..card.clone()
        };
        return effective_cost(state, &unembiggened, CostOptions::default());
    }
    effective_cost(state, card, CostOptions::default())
}
