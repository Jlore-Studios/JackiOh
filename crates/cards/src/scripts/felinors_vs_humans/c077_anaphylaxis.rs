//! FvH #77 Anaphylaxis (SPEC §8.14 row 77): (4) Spell, Common, of the Felinors vs Humans set.
//!   Base:    "Destroy a Unit. Costs ({discount}) less if you control a Felinor." (discount 2)
//!   Radiant: "Flashback" and the same, discount 3.
//!
//! One declared Unit of either side (R81) is `destroy`ed. A `cost` hook (R65, C+ #64 Mulch Muncher's
//! shape): the printed cost less the declared `discount` while its controller's field holds a Felinor
//! Unit (its tags now, granted ones counted, R923; a Dormant one is no Unit, R1839000), floored at 0. It
//! is a price for a play (R584's reading), so it holds in the hand and, for the Radiant face's Flashback,
//! in the graveyard it may be played from (R783012, at least (1) there, R4285000); anywhere else it costs
//! its printed (4). Flashback is the Radiant face's catalog keyword, so both faces run one script.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-077";

/// R923, R1839000: whether `player` controls a Felinor Unit on the field.
fn controls_a_felinor(state: &GameState, player: PlayerId) -> bool {
    active_units_of(state, player)
        .iter()
        .any(|unit| tags_of(state, unit).contains(&Tag::Felinor))
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
        cry: Some(hook(|_ctx| vec![destroy(json_as(json!({ "target": { "of": "chosen" } })))])),
        cost: Some(cost_hook(|CostArgs { state, instance }| {
            // The printed cost of the version the game folds under (R767000).
            let printed_cost = query_cost(def_of(Some(state), ID));
            let at = &instance.zone;
            let for_play = matches!(at, Zone::Hand { .. })
                || (matches!(at, Zone::Graveyard { .. }) && playable_from_graveyard(state, instance));
            if !for_play || !controls_a_felinor(state, at.player()) {
                return printed_cost;
            }
            let discount = param(
                &HookArgs {
                    state,
                    self_: instance,
                    radiant: instance.radiant,
                },
                "discount",
            );
            (printed_cost - discount).max(0)
        })),
        ..Script::default()
    };
    // The same script: Flashback is the Radiant face's catalog keyword and its (3) its declared `discount`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #77 Anaphylaxis — SPEC §8.14 row 77 (R65, R783012).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    /// Stockpile: a Spell each hand keeps.
    const FILLER: &str = "core-005";
    /// Felinor Token: a Felinor Unit.
    const FELINOR: &str = "core-t-felinor";
    /// A sturdy vanilla Unit with no tag.
    const WALL: &str = "stand-in (1) 1/5";

    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "seed": "anaphylaxis",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": mine, "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "field": theirs, "library": filler(4) },
        }))
    }

    /// The price the engine charges for the card where it lies.
    fn price(s: &Scenario, id: &str) -> i32 {
        effective_cost(s.state(), s.card(id), Default::default())
    }

    fn hand_price(s: &Scenario) -> i32 {
        let card = must(s.hand(P1).into_iter().find(|card| card.def_id == ID), "Anaphylaxis in hand");
        price(s, &card.id)
    }

    mod base {
        use super::*;

        #[test]
        fn destroys_the_chosen_unit_of_either_side() {
            let mut s = board(false, json!([WALL]), json!([WALL]));
            let theirs = s.unit_at(P2, 1).id;
            s.play(ID, pick::targets(pick::id(&theirs)));
            s.expect_in_zone(&theirs, "graveyard");
            assert!(s.unit(P1, 1).is_some(), "the caster's own Unit stands");
        }

        #[test]
        fn r65_costs_4_and_2_less_while_you_control_a_felinor() {
            let without = board(false, json!([WALL]), json!([]));
            assert_eq!(hand_price(&without), 4);
            let with = board(false, json!([FELINOR]), json!([]));
            assert_eq!(hand_price(&with), 2);
        }

        #[test]
        fn r65_the_opponents_felinor_does_not_count() {
            let s = board(false, json!([WALL]), json!([FELINOR]));
            assert_eq!(hand_price(&s), 4);
        }

        #[test]
        fn r65_the_discount_goes_when_the_felinor_does() {
            let mut s = scenario(json!({
                "seed": "anaphylaxis-twice",
                "p1": { "hand": [ID, ID, FILLER], "field": [FELINOR], "library": filler(4), "mana": 10 },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            let copies: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| card.def_id == ID).collect();
            assert_eq!(copies.len(), 2);
            assert_eq!(price(&s, &copies[1].id), 2);
            let felinor = s.unit_at(P1, 1).id;
            s.play(&copies[0], pick::targets(pick::id(&felinor)));
            // A token that dies ceases to exist.
            s.expect_in_zone(&felinor, "gone");
            assert_eq!(price(&s, &copies[1].id), 4, "no Felinor, no discount");
            assert_eq!(price(&s, &copies[0].id), 4, "in the graveyard with no Flashback: no play, its printed cost");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r65_costs_3_less_while_you_control_a_felinor() {
            let s = board(true, json!([FELINOR]), json!([]));
            assert_eq!(hand_price(&s), 1);
        }

        #[test]
        fn r783012_flashback_offers_it_from_the_graveyard_at_the_discounted_price() {
            let mut s = board(true, json!([FELINOR]), json!([WALL, WALL]));
            let card = must(s.hand(P1).into_iter().find(|card| card.def_id == ID), "Anaphylaxis in hand");
            let first = s.unit_at(P2, 1).id;
            s.play(&card, pick::targets(pick::id(&first)));
            s.expect_in_zone(&first, "graveyard");
            s.expect_in_zone(&card.id, "graveyard");
            assert!(s.can_play(&card.id), "Flashback offers it from the graveyard");
            assert_eq!(price(&s, &card.id), 1, "the Felinor's discount prices it there too");
            let second = s.unit_at(P2, 2).id;
            let aims = pick::instance_ids(&s.play_targets(&card.id));
            assert!(aims.contains(&second), "it may aim at a Unit again: {aims:?}");
        }

        #[test]
        fn r783012_without_a_felinor_the_graveyard_play_costs_its_printed_4() {
            let mut s = board(true, json!([WALL]), json!([WALL]));
            let card = must(s.hand(P1).into_iter().find(|card| card.def_id == ID), "Anaphylaxis in hand");
            let theirs = s.unit_at(P2, 1).id;
            s.play(&card, pick::targets(pick::id(&theirs)));
            s.expect_in_zone(&card.id, "graveyard");
            assert_eq!(price(&s, &card.id), 4);
        }
    }
}
