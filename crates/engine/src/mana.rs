//! Mana refresh, temporary mana and the cost calculation (SPEC §2.3, §6.3 Cost, R65, R396, R455).

use crate::catalog::def_of;
use crate::config::GLITCH_DEF_ID;
use crate::cost_rules::{climb_price_rules, cost_floor_of, price_rules_for};
use crate::graveyard_play::playable_from_graveyard;
use crate::script::{CostArgs, EngineSink};
use crate::state::{
    CardInstance, GameState, ModifierExpiry, ModifierKind, PlayerModifier, PlayerState, handicap_of,
};
use crate::triggers::{TriggerZone, holders_answering};
use crate::wire::{CardCost, CardType, GameEvent, GameEventType, ManaSpentFor, PLAYER_IDS, PlayerId, Zone};

/// §2.3, R181: max mana is min(turns started + the seat's mana bonus, its mana cap), plus persistent
/// modifiers, floored at 0; with no handicap, §2.3's "min(number of turns you have started, 4), plus
/// persistent modifiers". `nextTurnMod` is a one-shot rider the refresh spends, never max mana; other
/// modifiers apply on top of the capped value as for a human. R1854003: moved crystals count in the cap;
/// R4331001: stolen crystals count on top of it.
pub fn max_mana_for(side: &PlayerState) -> i32 {
    let handicap = handicap_of(side);
    let base = mana_crystal_count(side).min(handicap.mana_cap);
    (base + stolen_mana_crystals_of(side) + side.mana.perm_mod).max(0)
}

/// R1854003: a player's mana crystals before the cap (Hearthstone's count): the turns they have started
/// and their seat's mana bonus (R181), moved by the crystals effects stole from them or gave them.
pub fn mana_crystal_count(side: &PlayerState) -> i32 {
    side.turns_started + handicap_of(side).mana_bonus + side.mana_crystals.unwrap_or(0)
}

/// R4331001: the crystals a player stole, which stand on top of the cap.
pub fn stolen_mana_crystals_of(side: &PlayerState) -> i32 {
    side.stolen_mana_crystals.unwrap_or(0)
}

/// How a steal counts the thief's gain: past the cap (R4331001, the steal since issue #4331), or inside
/// it, a thief at the cap gaining none (R1854003, the steal v0.6.9c and before ran, kept, R767000).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrystalGain {
    PastTheCap,
    ToTheCap,
}

/// R1854003: `thief` steals `victim`'s mana crystals: the victim's max mana falls to their persistent
/// modifiers alone and their current mana by as much, never below 0, their own crystals coming back one
/// a turn (§2.3) and the ones they had stolen gone for good (R4331001); the thief gains as many, full,
/// past the cap or up to it as `gain` says. Returns the max mana the victim lost and the crystals the
/// thief gained; the caller reports each side whose mana changed (`mana_event`).
pub fn move_mana_crystals(
    state: &mut GameState,
    thief: PlayerId,
    victim: PlayerId,
    gain: CrystalGain,
) -> (i32, i32) {
    let side = &mut state.players[victim];
    let count = mana_crystal_count(side);
    let before = side.mana.max;
    if count > 0 {
        side.mana_crystals = Some(side.mana_crystals.unwrap_or(0) - count).filter(|moved| *moved != 0);
    }
    side.stolen_mana_crystals = None;
    side.mana.max = max_mana_for(side);
    let lost = (before - side.mana.max).max(0);
    side.mana.current = (side.mana.current - lost).max(0);

    let side = &mut state.players[thief];
    let gained = match gain {
        CrystalGain::PastTheCap => lost,
        CrystalGain::ToTheCap => lost.min((handicap_of(side).mana_cap - mana_crystal_count(side)).max(0)),
    };
    if gained > 0 {
        let held = side.mana.max;
        match gain {
            CrystalGain::PastTheCap => {
                side.stolen_mana_crystals = Some(stolen_mana_crystals_of(side) + gained);
            }
            CrystalGain::ToTheCap => {
                side.mana_crystals =
                    Some(side.mana_crystals.unwrap_or(0) + gained).filter(|moved| *moved != 0);
            }
        }
        side.mana.max = max_mana_for(side);
        side.mana.current += side.mana.max - held;
    }
    (lost, gained)
}

/// R4590002 (IA #135 Mana Leech): `thief` steals `count` of `victim`'s mana crystals, counted
/// from the capped maximum: the victim's own crystals first (up to `held`, their count inside the
/// cap), then crystals they had stolen. The victim's count drops to `held - own` inside the cap, so
/// their own crystals come back one a turn (§2.3); the thief holds what they lost past the cap
/// (`CrystalGain::PastTheCap`, R4331001). Returns the max mana the victim lost and the crystals the
/// thief gained; the caller reports each side whose mana changed (`mana_event`).
pub fn move_some_mana_crystals(
    state: &mut GameState,
    thief: PlayerId,
    victim: PlayerId,
    count: i32,
) -> (i32, i32) {
    let cap = handicap_of(&state.players[victim]).mana_cap;
    let side = &mut state.players[victim];
    let held = mana_crystal_count(side).min(cap).max(0);
    let own = count.clamp(0, held);
    let offset = side.mana_crystals.unwrap_or(0);
    side.mana_crystals = Some(offset - (mana_crystal_count(side) - held) - own).filter(|moved| *moved != 0);
    let from_stolen = (count - own).clamp(0, stolen_mana_crystals_of(side));
    if from_stolen > 0 {
        side.stolen_mana_crystals =
            Some(stolen_mana_crystals_of(side) - from_stolen).filter(|left| *left != 0);
    }
    let before = side.mana.max;
    side.mana.max = max_mana_for(side);
    let lost = (before - side.mana.max).max(0);
    side.mana.current = (side.mana.current - lost).max(0);

    let side = &mut state.players[thief];
    let gained = lost;
    if gained > 0 {
        let held = side.mana.max;
        side.stolen_mana_crystals = Some(stolen_mana_crystals_of(side) + gained);
        side.mana.max = max_mana_for(side);
        side.mana.current += side.mana.max - held;
    }
    (lost, gained)
}

/// R169: the id the next refresh's rider (`mana.nextTurnMod`, §6.3 Mana: "'next turn' mana is stored
/// as a modifier for the next refresh") travels under — one badge per player, which `modifierChanged`
/// names as it appears, changes and is spent, and which the view lists while the rider is not 0.
pub const NEXT_REFRESH_MODIFIER_ID: &str = "nextTurnMana";

/// Start of turn: refresh to max, moved by the one-shot rider (§6.3 Mana), which is then cleared. The
/// rider changes what the refresh gives, not max mana: #24 Efficiency Dividend's next-turn mana is
/// temporary mana that "can exceed 4", as #6 Mana Well's gain does, and #21 Hinder "subtracts from the
/// opponent's next refresh". Current mana never goes below 0 (§2.3).
pub fn refresh_mana(side: &mut PlayerState) {
    refresh_mana_with(side, true);
}

/// `refresh_mana` with the natural generation named: under MD-B2's aura (`natural: false`) the
/// refresh sets max mana as usual but current mana to the next-turn rider only. A lost refresh
/// still gives 0 first (`refresh_is_lost`).
pub fn refresh_mana_with(side: &mut PlayerState, natural: bool) {
    refresh_mana_with_bonus(side, natural, 0);
}

/// R4440052: `refresh_mana_with`, the refresh giving `bonus` more on top, a standing rider beside the
/// one-shot one (`mana_per_turn_of`): max mana is unmoved, and a lost refresh gives none of it.
pub fn refresh_mana_with_bonus(side: &mut PlayerState, natural: bool, bonus: i32) {
    let max = max_mana_for(side);
    side.mana.max = max;
    side.mana.current = refreshed_mana_with_bonus(side, max, natural, bonus);
    side.mana.next_turn_mod = 0;
    // R1224: ME-TURN's repayment schedule — the next instalment comes off the refresh, after any
    // rider. A lost refresh forgives the instalment: there is nothing to take it from.
    crate::credit::take_instalment(side);
}

/// R844 (Meditative #18, #19): the id "lose all mana next N turns" travels under — one badge per
/// player, which `modifierChanged` names as it appears and as it goes, and which the view lists
/// while a loss covers a future refresh (R169).
pub const LOST_REFRESH_MODIFIER_ID: &str = "lostRefresh";

/// R844: whether this player's refresh is lost: `lost_refresh_through` covers their `turns_started`.
pub fn refresh_is_lost(side: &PlayerState) -> bool {
    side.lost_refresh_through
        .is_some_and(|through| side.turns_started <= through)
}

/// R844, R941: what a refresh gives. A lost refresh gives 0 and spends the rider with it; without
/// natural generation (MD-B2) it gives the rider alone; otherwise max plus the rider. Never below 0.
/// MB23's repayment schedule (#89) comes off afterwards (`credit::take_instalment`).
pub fn refreshed_mana(side: &PlayerState, max: i32, natural: bool) -> i32 {
    refreshed_mana_with_bonus(side, max, natural, 0)
}

/// R4440052: `refreshed_mana` with `bonus` more, which counts as the rider does: with it under MD-B2's
/// aura, against it when it is negative, and not at all on a lost refresh. Never below 0.
pub fn refreshed_mana_with_bonus(side: &PlayerState, max: i32, natural: bool, bonus: i32) -> i32 {
    let rider = side.mana.next_turn_mod + bonus;
    if refresh_is_lost(side) {
        0
    } else if natural {
        (max + rider).max(0)
    } else {
        rider.max(0)
    }
}

/// R4440052: the declared number a "You have +N mana per turn" flag stands for (C #28 Second Wind's).
pub const MANA_PER_TURN_PARAM: &str = "mana";

/// R4440052: what `player`'s refresh gives on top of max mana, read as their turn begins, before it: the
/// sum of the `mana_per_turn` flags of the face-up cards acting on their side of the field, a Unit a
/// carrier holds included (R446, R1824000), each the card's declared `mana` as tunes moved it (R386) and
/// never below 0. 0 with none.
pub fn mana_per_turn_of(state: &GameState, player: PlayerId) -> i32 {
    crate::zones::acting_on(state, player, crate::zones::StandingAt::ItsLane)
        .into_iter()
        .filter(|card| !crate::preview::is_face_down(state, card))
        .filter_map(|card| {
            crate::scripts::flags_of(state, card)
                .mana_per_turn
                .map(|flag| crate::params::declared_or(state, card, MANA_PER_TURN_PARAM, flag).max(0))
        })
        .sum()
}

/// R844: how many lost refreshes this player has left past this one, never below 0.
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

/// MD-B2, R941: whether any player generates mana naturally now: false while a face-up card holding
/// `no_natural_mana` acts on either side's field, a Unit a carrier holds included (R446, R1824000).
pub fn natural_mana_on(state: &GameState) -> bool {
    for player in PLAYER_IDS {
        for card in crate::zones::acting_on(state, player, crate::zones::StandingAt::ItsLane) {
            if !crate::preview::is_face_down(state, card)
                && crate::scripts::flags_of(state, card).no_natural_mana == Some(true)
            {
                return false;
            }
        }
    }
    true
}

/// §6.3 Refresh, R364: give back up to `amount` spent mana, never past max (Hearthstone's "Refresh
/// Mana Crystals", #78 /fullsend), unlike temporary mana (`gain_mana`).
pub fn refresh_some_mana(side: &mut PlayerState, amount: i32) {
    if amount <= 0 || side.mana.current >= side.mana.max {
        return;
    }
    // Saturating: "refresh all your mana" (Classic+ #27) passes `i32::MAX`.
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

/// MD-D26: whether a `manaSpent` would be heard — some card answers it that is not a trap and sits
/// on the field or in the backrow. Those cards are public, and a trigger on a public card is
/// public, so asking costs a listener nothing hidden.
pub fn mana_spent_heard(state: &GameState) -> bool {
    holders_answering(state, GameEventType::ManaSpent)
        .iter()
        .any(|holder| !holder.is_trap && matches!(holder.zone, TriggerZone::Field | TriggerZone::Backrow))
}

/// MD-D26: report mana `player` spent, for a play's price or an activation's — but only while
/// something answers it, so a game with no listener keeps its events, and its `next_seq` hashes,
/// unchanged (D14). Nothing is emitted for an amount below 1.
pub fn push_mana_spent(sink: &mut EngineSink<'_>, player: PlayerId, amount: i32, for_: ManaSpentFor) {
    if amount <= 0 || !mana_spent_heard(sink.state) {
        return;
    }
    sink.events.push(GameEvent::ManaSpent { player, amount, for_ });
}

/// The printed cost as it stands: X uses the chosen X, an embiggen card the chosen price (R65).
/// R1848003: an X-cost card whose X is set (`fixed_x`) costs that X wherever it is. R4350000: an
/// embiggen card with no Embiggen premium (Meditative+ #23) prints its normal price for both versions.
pub fn printed_cost(state: &GameState, instance: &CardInstance) -> i32 {
    let script = crate::scripts::script_of(state, instance);
    if let Some(cost) = script.cost.as_ref() {
        return cost(CostArgs { state, instance }).max(0);
    }
    // R3560001: the graveyard price reads in the graveyard (S+ #120 Crawler).
    if let Some(price) = crate::scripts::flags_of(state, instance).graveyard_price
        && matches!(instance.zone, Zone::Graveyard { .. })
    {
        return price.max(0);
    }

    let cost: CardCost = def_of(Some(state), &instance.def_id).cost;
    match cost {
        CardCost::X => instance.x.or(instance.fixed_x).unwrap_or(0).max(0),
        CardCost::Fixed(n) => n,
        CardCost::Embiggen { base, embiggen } => {
            // R4350000: no Embiggen premium prices the Embiggened version at the normal price.
            if instance.embiggened == Some(true) && instance.no_embiggen_premium != Some(true) {
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

/// R48: a modifier for a player's *next* turn (#77: "during **your** next turn") is live only once that
/// player is active on a later turn, not on the turn it was made nor the opponent's between. Expiry
/// ends it at that turn's cleanup, a separate question.
pub fn modifier_is_live(state: &GameState, modifier: &PlayerModifier) -> bool {
    // R757: Armor Up's Armor holds from the moment it is gained until its expiry's cleanup removes it.
    // MD-C21: Tranquility's immunity holds the same way.
    if matches!(
        modifier.kind,
        ModifierKind::HeroArmor { .. }
            | ModifierKind::HeroImmune
            | ModifierKind::UnitsImmune
            | ModifierKind::UnitsAttack { .. }
    ) {
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

/// R65: start from costOverride or the printed cost, add the instance's costMod and the player's discounts,
/// then Professor Curvature if the result is 4 or more (R363), and floor at 0; an X-cost card costs exactly
/// X unless an override makes it free. Discounts and Curvature price a play (§6.3's Cost, #35, #78, R48), so
/// they reach only a card in its controller's hand (§10.5 step 1) or a graveyard a permission lets them play
/// from (E11, R454); any other is read at its own cost (#30, R24; #94, R66). `options.asPlay` prices any
/// card as a play of it now.
///
/// R455 (E15) adds its rungs through `cost_rules.rs`: after R65's discounts the flat price rules, then the
/// threshold rules on the number the flat ones left (as R363 reads Curvature), then "costs (N)" sets, then
/// the card's own floor (Forever&'s), which holds in every zone and alone reaches X, then 0. R4285000: last,
/// a play by Flashback from the graveyard costs at least FLASHBACK_MIN_PRICE (`flashback_floor`), X included.
pub fn effective_cost(state: &GameState, instance: &CardInstance, options: CostOptions) -> i32 {
    price_of(state, instance, options).cost
}

/// R454, R455: what a play of this card now costs, wherever the play takes it from.
pub fn play_cost(state: &GameState, instance: &CardInstance) -> i32 {
    effective_cost(state, instance, CostOptions { as_play: Some(true) })
}

/// R4285000: what a play of this card now costs before Flashback's floor: the price a Plague payment
/// meets (`graveyard_play::plague_price`), since such a play is its permission's and spends no Flashback.
pub fn play_cost_before_flashback_floor(state: &GameState, instance: &CardInstance) -> i32 {
    priced(state, instance, CostOptions { as_play: Some(true) }, false).cost
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

/// R1181002 (Meditative+ #22 Mountaintop Garden): the least premium the Gardens acting on either side
/// (`acting_texts_of`, each reaching both players' cards) ask for an embiggen card's dearer price, or
/// `None` while no Garden acts; each moves with its card's declared number `premium` (R386).
pub fn garden_premium(state: &GameState) -> Option<i32> {
    let mut least: Option<i32> = None;
    for side in PLAYER_IDS {
        for card in crate::damage::acting_texts_of(state, side) {
            for text in crate::scripts::texts_of(state, &card) {
                let Some(flag) = text.flags.embiggen_garden else {
                    continue;
                };
                let premium = crate::params::declared_or(state, &card, "premium", flag).max(0);
                least = Some(least.map_or(premium, |held| held.min(premium)));
            }
        }
    }
    least
}

/// R1181002: whether a Garden reaches a play of this embiggen card now, which plays its Embiggened
/// version at its normal price — an embiggen card played face-up from a hand, or from a graveyard
/// (R454), while a Garden acts. A cast (R545) and a card set face-down (R783005) are not reached.
pub fn garden_reaches(state: &GameState, card: &CardInstance) -> bool {
    crate::play_choices::has_embiggen_price(state, card) && garden_premium_for(state, card).is_some()
}

/// R1181002, R1563004: the premium a Garden asks of a play of this card now — any card played face-up
/// from a hand or from a graveyard (R454), while a Garden acts.
fn garden_premium_for(state: &GameState, card: &CardInstance) -> Option<i32> {
    if card.set_as.is_some() || !matches!(card.zone, Zone::Hand { .. } | Zone::Graveyard { .. }) {
        return None;
    }
    garden_premium(state)
}

/// R1563004: whether a Garden offers this card of one price its Radiant price: not Radiant, no embiggen
/// price and no X (R65), played as a Garden reaches a play. Its dearer choice is the play's `embiggen:
/// true`, stamped on the card as `embiggened` until §10.5 step 3 (`play_steps::garden_step`).
pub fn garden_offers_radiant(state: &GameState, card: &CardInstance) -> bool {
    !card.radiant
        && !crate::play_choices::has_embiggen_price(state, card)
        && !is_x_cost(state, card)
        && garden_premium_for(state, card).is_some()
}

/// R1181002, R1563004: the price a Garden sets on the dearer play of a card that is not Radiant: its
/// normal price plus the premium, in place of an embiggen card's Embiggened price; else `None`.
/// R4350000: a card with no Embiggen premium (Meditative+ #23) pays no Garden premium either, so its
/// dearer choice costs its normal price, and is still the Garden's (`Some`, which stamps it Radiant).
pub fn garden_dearer_price(state: &GameState, card: &CardInstance) -> Option<i32> {
    if card.embiggened != Some(true) || card.radiant {
        return None;
    }
    let premium = garden_premium_for(state, card)?;
    // R4350000: a card with no Embiggen premium (M+ #23) has none under a Garden either.
    let waived = card.no_embiggen_premium == Some(true);
    let premium = if waived { 0 } else { premium };
    match def_of(Some(state), &card.def_id).cost {
        CardCost::Embiggen { base, .. } => Some(base + premium),
        _ if garden_offers_radiant(state, card) => Some(printed_cost(state, card) + premium),
        _ => None,
    }
}

/// R65, R1181003: the override a price starts from: for the Embiggened version its own price
/// (`embiggenCostOverride`, Meditative+ #23) before `costOverride`; for every other play `costOverride`.
pub fn override_of(card: &CardInstance) -> Option<i32> {
    if card.embiggened == Some(true) {
        card.embiggen_cost_override.or(card.cost_override)
    } else {
        card.cost_override
    }
}

fn price_of(state: &GameState, instance: &CardInstance, options: CostOptions) -> Price {
    priced(state, instance, options, true)
}

/// `price_of`, with Flashback's floor (R4285000) held or not.
fn priced(state: &GameState, instance: &CardInstance, options: CostOptions, flashback_floor: bool) -> Price {
    let floored = |cost: i32| {
        if flashback_floor {
            crate::graveyard_play::flashback_floor(state, instance, cost)
        } else {
            cost
        }
    };
    let side = &state.players[instance.controller];
    let override_ = override_of(instance);
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
        // R4285000: Flashback's floor holds for an X card too.
        return Price {
            cost: floored(0.max(printed).max(floor)),
            used_rules: Vec::new(),
        };
    }

    // R1181002: under a Garden the dearer play of a card that is not Radiant starts from the normal
    // price plus the premium.
    let mut cost = override_.unwrap_or_else(|| {
        garden_dearer_price(state, instance).unwrap_or_else(|| printed_cost(state, instance))
    }) + instance.cost_mod
        + crate::hand_order::hand_aura_cost_delta(state, instance);
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

    // R65, R363: "apply Professor Curvature if the result is then 4 or more": every live Curvature
    // (#39's copy, #33's) tests that one number, never one another lowered, so play order changes
    // nothing (R48). R455: the flat price rules come first; the threshold rules read Curvature's number.
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

    // R4285000: a play by Flashback from the graveyard costs at least (1), each embiggen price apart.
    Price {
        cost: floored(0.max(climbed.price).max(floor)),
        used_rules: climbed.used,
    }
}

/// R1180004 (Meditative+ #6 Bloodbloom): the health price a play of this card would use now, as
/// `(multiple, echo)`: the newest live `HealthPrice` of the player holding it, for a Spell played
/// face-up from that hand, not one set face-down (R783005); a graveyard's play or a cast (R70) never
/// uses it. Every price, legal play and payment reads this one rule.
pub fn health_price_rule(state: &GameState, instance: &CardInstance) -> Option<(i32, Option<i32>)> {
    let Zone::Hand { player } = instance.zone else {
        return None;
    };
    if instance.set_as.is_some() || crate::faces::card_type_of(state, instance) != CardType::Spell {
        return None;
    }
    state.players[player]
        .mods
        .iter()
        .rev()
        .find_map(|modifier| match modifier.kind {
            ModifierKind::HealthPrice { multiple, echo } if modifier_is_live(state, modifier) => {
                Some((multiple.max(0), echo))
            }
            _ => None,
        })
}

/// R1180004: whether a play of this card from its hand would pay its price in health now.
pub fn health_priced(state: &GameState, instance: &CardInstance) -> bool {
    health_price_rule(state, instance).is_some()
}

/// R1180004: the mana a play of this card at `cost` takes now — nothing under a health price, else the
/// cost. What every affordability check compares with the player's spendable mana.
pub fn mana_due_now(state: &GameState, instance: &CardInstance, cost: i32) -> i32 {
    if health_priced(state, instance) { 0 } else { cost }
}

/// R3548004: whether `player`'s mana is infinite now: a card giving it acts on their field (a
/// Dormant one sleeps through it, a face-down one's text with it, as every flag read this way).
pub fn has_infinite_mana(state: &GameState, player: PlayerId) -> bool {
    crate::query::acting_with_flag(state, player, |flags| flags.infinite_mana)
}

/// R396 (Classic #10, #18, #25, #32, #39): what a card costs wherever a rule or a card script compares
/// or counts costs. An X-cost card costs the X it was played for on the field, else 0 (R65; a Recruit,
/// a summon); an embiggen card outside a play its base price (R65); any other card R65's cost as it
/// stands, its own outside a hand (R66). A floor the card carries holds (R455).
pub fn cost_now(state: &GameState, card: &CardInstance) -> i32 {
    let floor = cost_floor_of(card);
    if is_x_cost(state, card) {
        // R1848003: a set X is the card's price anywhere, the field's chosen X first.
        let x = if matches!(card.zone, Zone::Field { .. }) {
            card.x.or(card.fixed_x).unwrap_or(0)
        } else {
            card.fixed_x.unwrap_or(0)
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
