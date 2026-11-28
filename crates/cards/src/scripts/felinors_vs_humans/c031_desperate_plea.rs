//! SPEC §8.14 FvH #31 Desperate Plea ((9) Spell, Legendary), a card of the Felinors vs Humans set
//! (R5235000).
//!   Base:    "Float reduces cost.\nCosts (0) while your hero has exactly 1 health.\nHeal your hero {heal}.
//!            Fill your board with random Legendary Units." (30)
//!   Radiant: the same with {heal} 60 and random Radiant Legendary Units.
//!
//! "Float reduces cost" is the engine's hand trigger (`float_reduces_cost`, R1563000), a `costMod` of minus
//! the Float. The (0) is a `cost` hook (C+ #64 Mulch Muncher's shape, R65): 0 while its owner's hero has
//! exactly 1 health, Armor not counted, the printed (9) of the version the game folds under otherwise,
//! in every zone; the Float's `costMod` comes on top, floored at 0. Then, in the text's order, a `heal`
//! of its hero by the declared `heal` and `fill_board_random` from the non-token Legendary Units of every
//! set that ships (R380), one into each empty unit zone (R64), Radiant on the Radiant face; no zone,
//! nothing (R129).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-031";

/// "Costs (0)": its price at the threshold, a printed price (R1748003).
const FREE: i32 = 0;
/// "exactly 1 health": the threshold, a printed number (R1748003).
const LAST_HEALTH: i32 = 1;

/// One face; `radiant` is the face the Legendary Units arrive on.
fn face(radiant: bool) -> Script {
    Script {
        cost: Some(cost_hook(|CostArgs { state, instance }| {
            if hero_of(state, instance.owner).health == LAST_HEALTH {
                FREE
            } else {
                // The printed cost of the version the game folds under (R767000).
                query_cost(def_of(Some(state), ID))
            }
        })),
        hand_triggers: vec![float_reduces_cost("felinorsvshumans-031-float-hand")],
        cry: Some(hook(move |ctx| {
            vec![
                heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": param(&*ctx, "heal") }))),
                fill_board_random(json_as(json!({
                    "query": { "type": "Unit", "rarity": "Legendary" },
                    "radiant": radiant,
                }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// FvH #31 Desperate Plea — SPEC §8.14 row 31 (R1563000, R65, R64, R129, R380).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::weapon_board;

    const FILLER: &str = "core-005"; // Stockpile: a Spell, each side's spare card (§2.5).
    const VANILLA: &str = "stand-in (1) 4/4";
    /// Its printed price.
    const PRINTED: i32 = 9;

    fn price(s: &Scenario, card: &CardInstance) -> i32 {
        effective_cost(s.state(), s.card(card), CostOptions::default())
    }

    /// p1 holds the Plea (`radiant` its face) at `health`, with one more in the deck and a Unit in lane 2.
    fn plea(seed: &str, radiant: bool, health: i32, armor: i32) -> Scenario {
        weapon_board(
            seed,
            json!({
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": [{ "def": ID, "radiant": radiant }, FILLER, FILLER],
                "field": [{ "def": VANILLA, "lane": 2 }],
                "health": health,
                "armor": armor,
                "mana": 0,
            }),
            json!({ "hand": [FILLER] }),
        )
    }

    fn held(s: &Scenario) -> CardInstance {
        must(s.hand(P1).into_iter().find(|card| card.def_id == ID), "the Plea in hand")
    }

    fn in_deck(s: &Scenario) -> CardInstance {
        must(s.pile(P1, "library").into_iter().find(|card| card.def_id == ID), "the Plea in the deck")
    }

    /// Every Unit the fill put beside lane 2's VANILLA is a Legendary non-token Unit on `radiant`, and
    /// no empty unit zone is left.
    fn expect_filled(s: &Scenario, radiant: bool) {
        let filled: Vec<CardInstance> = units_on_field_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id != VANILLA)
            .cloned()
            .collect();
        assert!((1..=UNIT_ZONES).all(|lane| s.unit(P1, lane).is_some()), "every unit zone is filled");
        for unit in nonempty(filled, "the Units the fill summoned") {
            let def = crate::card_def(&unit.def_id);
            assert_eq!(def.rarity, Rarity::Legendary, "{}", def.id);
            assert_eq!(def.type_, CardType::Unit, "{}", def.id);
            assert!(!def.token, "{}", def.id);
            assert_eq!(unit.radiant, radiant, "{}", def.id);
        }
        assert_eq!(s.unit_at(P1, 2).def_id, VANILLA);
    }

    #[test]
    fn r1563000_a_float_lowers_its_cost_in_hand_never_in_the_deck() {
        crate::stinkyscape_family::drone_float_two_less(ID, PRINTED);
    }

    #[test]
    fn r1563000_the_opponents_float_lowers_nothing() {
        crate::stinkyscape_family::drone_float_opponent(ID, PRINTED);
    }

    #[test]
    fn r65_it_costs_0_while_your_hero_has_exactly_1_health_in_every_zone() {
        let s = plea("fvh031-one", false, 1, 0);
        assert_eq!(price(&s, &held(&s)), 0);
        assert_eq!(printed_cost(s.state(), &in_deck(&s)), 0);
        for health in [2, 30] {
            let s = plea("fvh031-more", false, health, 0);
            assert_eq!(price(&s, &held(&s)), PRINTED, "at {health}");
            assert_eq!(printed_cost(s.state(), &in_deck(&s)), PRINTED, "at {health}");
        }
    }

    #[test]
    fn r65_armor_does_not_count_toward_the_1_health() {
        let s = plea("fvh031-armor", false, 1, 5);
        assert_eq!(price(&s, &held(&s)), 0);
    }

    mod base {
        use super::*;

        #[test]
        fn r64_at_1_health_it_is_played_for_0_heals_30_and_fills_the_board_with_legendary_units() {
            let mut s = plea("fvh031-play", false, 1, 0);
            s.play(ID, json!({}));
            s.expect_health(P1, 1 + 30);
            expect_filled(&s, false);
        }

        #[test]
        fn r1748000_the_heal_moves_with_a_buff() {
            let mut s = plea("fvh031-buff", false, 1, 0);
            let card = held(&s);
            let heal = crate::upgrade_number(&mut s, card.id.as_str(), "heal");
            s.play(ID, json!({}));
            s.expect_health(P1, 1 + heal);
        }

        #[test]
        fn r129_with_no_empty_unit_zone_it_only_heals() {
            let mut s = weapon_board(
                "fvh031-full",
                json!({ "hand": [ID, FILLER], "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA], "health": 1, "mana": 0 }),
                json!({ "hand": [FILLER] }),
            );
            s.play(ID, json!({}));
            s.expect_health(P1, 31);
            for lane in 1..=UNIT_ZONES {
                assert_eq!(s.unit_at(P1, lane).def_id, VANILLA);
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r64_it_heals_60_and_fills_the_board_with_radiant_legendary_units() {
            let mut s = plea("fvh031-radiant", true, 1, 0);
            s.play(ID, json!({}));
            s.expect_health(P1, 1 + 60);
            expect_filled(&s, true);
        }
    }
}
