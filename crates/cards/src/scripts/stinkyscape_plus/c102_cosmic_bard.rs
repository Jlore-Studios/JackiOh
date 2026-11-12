//! SPEC §8.12 S+ #102 Cosmic Bard (§2.4, §6.2 Combo, §6.3 Draw and Cost; R65, R129, R801, R1420,
//! R3568001): (1) Unit, Rare, 3/2 → 6/4, a card of the Stinkyscape+ set. Built by S20 (#3568).
//!   Base:    "Combo: Draw {summons} Units."  (summons 2)
//!   Radiant: "Combo: Draw {summons} Units and set both their costs to the cheapest drawn."
//!
//! A Combo is met by one card played earlier this turn (§6.2): `played_earlier`, this play's own place in
//! the turn's log, as Core #10 Rapid Replenish reads it. The hand's yellow glow (R195) is not declared: it
//! needs a `conditionMet` hook, which `tests/cross/condition_active.rs` pins to a closed list of cards.
//! The Cry draws the topmost Units of its controller's library, top first (R801: a kind of card drawn from
//! the deck takes the topmost matches, no random number), each its own named §2.4 draw, so the hand cap, a
//! cast on draw and the draw limit apply to each (R135). The library may hold fewer Units than `summons`:
//! it draws what there is, and none draws nothing and causes no fatigue (R129). The Radiant face then sets
//! every Unit it drew that is in the hand to the cheapest price the drawn Units had in the deck: the
//! effective cost as the library reads it (R65, R66: an X-cost Unit's is 0), so a Unit the hand cap burned
//! or a cast on draw took still counts toward "the cheapest drawn" and gets nothing (R3568001).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-102";

/// §6.2: a Combo is met by one card played earlier this turn.
const COMBO: i32 = 1;

/// R801: the topmost `summons` Units of the controller's library, top first.
fn topmost_units(ctx: &EffectContext<'_>) -> Vec<CardInstance> {
    let count = param(ctx, "summons").max(0) as usize;
    zone_cards(ctx.state, ctx.controller, OffFieldZone::Library)
        .into_iter()
        .filter(|card| card_type_of(ctx.state, card) == CardType::Unit)
        .take(count)
        .collect()
}

fn bard(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            if played_earlier(ctx.state, ctx.controller, ctx.live_self()) < COMBO {
                return vec![];
            }
            let drawn = topmost_units(&*ctx);
            let mut effects: Vec<Effect> = drawn
                .iter()
                .map(|card| draw_from_library(json_as(json!({ "instanceId": card.id }))))
                .collect();
            let prices = drawn
                .iter()
                .map(|card| effective_cost(ctx.state, card, Default::default()));
            if radiant && let Some(cheapest) = prices.min() {
                // R65: the set price replaces the printed one and the card's own cost change still adds
                // to it, so the override takes that change off to leave each card at the cheapest price.
                effects.extend(drawn.iter().map(|card| {
                    set_cost_override(json_as(json!({
                        "target": { "of": "instance", "instanceId": card.id },
                        "cost": (cheapest - card.cost_mod).max(0),
                        "inHandOnly": true,
                    })))
                }));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: bard(false),
        radiant: bard(true),
    }
}

// S+ #102 Cosmic Bard — SPEC §8.12 row 102 (R65, R129, R801, R3568001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{def_ids, scenario};

    const P1: PlayerId = PlayerId::P1;

    const CHEAP: &str = "stand-in (0) Spell"; // a play that does nothing: the Combo's earlier card
    const FILLER: &str = "core-005"; // Stockpile: a card in hand, so no turn ends by itself
    const SPELL: &str = "stand-in (2) Spell"; // a library card that is no Unit
    const DEAR: &str = "stand-in (3) 3/3";
    const CHEAPEST: &str = "stand-in (1) 1/1";
    const MIDDLING: &str = "stand-in (2) 2/2";
    const X_UNIT: &str = "classicplus-069"; // Buff Billy: an X-cost Unit

    /// p1 holds the Bard (of `radiant`'s face) behind a free play, p1's library is `library`.
    fn board(radiant: bool, library: Value) -> Scenario {
        scenario(json!({
            "seed": "cosmic-bard",
            "p1": {
                "hand": [CHEAP, { "def": ID, "radiant": radiant }, FILLER],
                "library": library,
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    fn price(s: &Scenario, def_id: &str) -> i32 {
        let card = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == def_id)
            .expect("drawn into the hand");
        effective_cost(s.state(), &card, Default::default())
    }

    #[test]
    fn combo_draws_two_units() {
        crate::register_all();
        let mut s = board(false, json!([SPELL, DEAR, CHEAPEST, MIDDLING, FILLER]));
        s.play(CHEAP, json!({})).play(ID, json!({}));
        // The two topmost Units, top first; the Spell above them stays (R801).
        assert_eq!(
            def_ids(&s.hand(P1)),
            vec![FILLER.to_string(), DEAR.to_string(), CHEAPEST.to_string()]
        );
        let library: Vec<String> = def_ids(&s.pile(P1, "library"));
        assert_eq!(
            library,
            vec![SPELL.to_string(), MIDDLING.to_string(), FILLER.to_string()]
        );
        // The base face leaves every price as it was.
        assert_eq!(price(&s, DEAR), 3);
        assert_eq!(price(&s, CHEAPEST), 1);
    }

    #[test]
    fn without_combo_draws_nothing() {
        crate::register_all();
        let mut s = board(false, json!([DEAR, CHEAPEST, MIDDLING]));
        // Played first: no card earlier this turn.
        s.play(ID, json!({}));
        assert_eq!(def_ids(&s.hand(P1)), vec![CHEAP.to_string(), FILLER.to_string()]);
        assert_eq!(s.pile(P1, "library").len(), 3);
    }

    #[test]
    fn radiant_without_combo_draws_and_sets_nothing() {
        crate::register_all();
        let mut s = board(true, json!([DEAR, CHEAPEST, MIDDLING]));
        s.play(ID, json!({}));
        assert_eq!(def_ids(&s.hand(P1)), vec![CHEAP.to_string(), FILLER.to_string()]);
        assert_eq!(s.pile(P1, "library").len(), 3);
    }

    #[test]
    fn r3568001_radiant_sets_both_costs_to_the_cheapest() {
        crate::register_all();
        let mut s = board(true, json!([SPELL, DEAR, CHEAPEST, MIDDLING]));
        s.play(CHEAP, json!({})).play(ID, json!({}));
        assert_eq!(
            def_ids(&s.hand(P1)),
            vec![FILLER.to_string(), DEAR.to_string(), CHEAPEST.to_string()]
        );
        // The (3) Unit now costs what the (1) Unit does, both (1).
        assert_eq!(price(&s, DEAR), 1);
        assert_eq!(price(&s, CHEAPEST), 1);
        // The Unit it did not draw is untouched.
        let left = s
            .pile(P1, "library")
            .into_iter()
            .find(|card| card.def_id == MIDDLING)
            .expect("left in the deck");
        assert_eq!(effective_cost(s.state(), &left, Default::default()), 2);
        assert_eq!(left.cost_override, None);
    }

    #[test]
    fn r3568001_a_cost_change_a_card_carries_is_taken_off_so_both_cost_the_cheapest() {
        crate::register_all();
        let mut s = board(true, json!([DEAR, CHEAPEST, MIDDLING]));
        // The (3) Unit already costs (1) more in the deck, (4): the cheapest drawn is still the (1).
        s.card_mut(DEAR).cost_mod = 1;
        s.play(CHEAP, json!({})).play(ID, json!({}));
        assert_eq!(price(&s, DEAR), 1);
        assert_eq!(price(&s, CHEAPEST), 1);
    }

    #[test]
    fn r3568001_an_x_cost_unit_counts_as_0() {
        crate::register_all();
        let mut s = board(true, json!([X_UNIT, MIDDLING, DEAR]));
        s.play(CHEAP, json!({})).play(ID, json!({}));
        // R65: outside play an X-cost card's is 0, so both are free.
        assert_eq!(price(&s, X_UNIT), 0);
        assert_eq!(price(&s, MIDDLING), 0);
    }

    #[test]
    fn one_unit_in_deck_draws_one() {
        crate::register_all();
        let mut s = board(true, json!([SPELL, DEAR, SPELL]));
        s.play(CHEAP, json!({})).play(ID, json!({}));
        assert_eq!(def_ids(&s.hand(P1)), vec![FILLER.to_string(), DEAR.to_string()]);
        // The cheapest of one is itself.
        assert_eq!(price(&s, DEAR), 3);
    }

    #[test]
    fn no_unit_in_deck_draws_nothing_and_no_fatigue() {
        crate::register_all();
        let mut s = board(true, json!([SPELL, SPELL]));
        let health = s.state().players[P1].hero.health;
        s.play(CHEAP, json!({})).play(ID, json!({}));
        assert_eq!(def_ids(&s.hand(P1)), vec![FILLER.to_string()]);
        assert_eq!(s.state().players[P1].hero.health, health);
    }
}
