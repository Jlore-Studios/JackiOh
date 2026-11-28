//! FvH #63 S-Tier Speciesist (SPEC §8.14 row 63): (4) Unit, Human, Rare, 10/1 → 20/2. Built by S08 (#5258).
//!   Base:    "Cry: Destroy every non-Human card on the field. Overload ({mana})." — mana 2
//!   Radiant: "Cry: Destroy every enemy non-Human card on the field. Overload ({mana})." — mana 2
//!
//! Core #2 Speciesist's Radiant sweep over both rows: `destroy_all` of every card whose tags hold no
//! Human in the unit row and the backrow, both sides on the base face (its own non-Humans too), the
//! enemy's alone on the Radiant. A Field Spell, a Trap and a face-down card carry no Human tag, so they
//! go; hands and decks are untouched, a Dormant card is not reached (R1839000), and every mark is
//! collected at one state check (R59). The list's standalone "Overload 2" line is folded into the Cry
//! (FV2, Core #2's shape, R4353021), printed "Overload ({mana})." (R4751000): `next_turn_mana` of
//! −`mana` on its controller's next refresh, paid whenever the Cry runs and never on a summon, which
//! runs none (R1).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-063";

fn s_tier(side: &'static str) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![
                destroy_all(json_as(json!({
                    "side": side,
                    "rows": ["units", "backrow"],
                    "notTags": ["Human"],
                }))),
                next_turn_mana(json_as(json!({ "amount": -param(ctx, "mana") }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: s_tier("any"),
        radiant: s_tier("enemy"),
    }
}

// FvH #63 S-Tier Speciesist — SPEC §8.14 row 63.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const HUMAN: &str = "core-008"; // Mr. Vanilla: a Human
    const OTHER: &str = "stand-in (1) 3/3";
    const FIELD_SPELL: &str = "stand-in (1) Field Spell";
    const TRAP: &str = "stand-in (1) Trap";
    const FILLER: &str = "core-005"; // Stockpile: a card in hand

    /// The Speciesist in p1's hand; each side holds a non-Human (lane 1), a Human (lane 2) and a
    /// backrow card (lane 1): p1 a Field Spell, p2 a Trap.
    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": [OTHER, HUMAN],
                "backrow": [FIELD_SPELL],
            },
            "p2": { "hand": [FILLER], "field": [OTHER, HUMAN], "backrow": [TRAP] },
        }))
    }

    /// The ids of p1's and p2's non-Human, Human and backrow card.
    fn sides(s: &Scenario) -> [(String, String, String); 2] {
        [P1, P2].map(|player| {
            (
                s.unit_at(player, 1).id,
                s.unit_at(player, 2).id,
                must(s.backrow(player, 1), "a backrow card").id,
            )
        })
    }

    mod base {
        use super::*;

        #[test]
        fn destroys_every_non_human_card_on_both_sides_and_spares_the_humans() {
            let mut s = board(false);
            let [(mine, my_human, my_spell), (theirs, their_human, their_trap)] = sides(&s);
            s.play(ID, json!({}));
            for gone in [&mine, &my_spell, &theirs, &their_trap] {
                s.expect_in_zone(gone.as_str(), "graveyard");
            }
            for kept in [&my_human, &their_human] {
                s.expect_in_zone(kept.as_str(), "field");
            }
            s.expect_in_zone(ID, "field");
        }

        #[test]
        fn hands_and_decks_are_untouched() {
            let mut s = scenario(json!({
                "p1": { "hand": [ID, OTHER, FILLER], "library": [OTHER] },
                "p2": { "hand": [OTHER, FILLER], "library": [OTHER] },
            }));
            s.play(ID, json!({}));
            for player in [P1, P2] {
                assert!(s.hand(player).iter().any(|card| card.def_id == OTHER));
                assert_eq!(s.pile(player, "library").len(), 1);
            }
        }

        #[test]
        fn its_cry_overloads_2_on_the_next_refresh() {
            let mut s = board(false);
            s.play(ID, json!({}));
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -2);
            assert_eq!(s.state().players.p2.mana.next_turn_mod, 0);
        }

        #[test]
        fn the_overload_is_paid_with_nothing_to_destroy() {
            let mut s = scenario(json!({
                "p1": { "hand": [ID, FILLER], "field": [HUMAN] },
                "p2": { "hand": [FILLER] },
            }));
            s.play(ID, json!({}));
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -2);
        }

        #[test]
        fn the_overload_tunes() {
            let mut s = board(false);
            set_param(s.card_mut(ID), "mana", 1);
            s.play(ID, json!({}));
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn destroys_every_enemy_non_human_card_and_spares_your_own() {
            let mut s = board(true);
            let [(mine, my_human, my_spell), (theirs, their_human, their_trap)] = sides(&s);
            s.play(ID, json!({}));
            for gone in [&theirs, &their_trap] {
                s.expect_in_zone(gone.as_str(), "graveyard");
            }
            for kept in [&mine, &my_human, &my_spell, &their_human] {
                s.expect_in_zone(kept.as_str(), "field");
            }
        }

        #[test]
        fn its_cry_overloads_2_on_the_next_refresh() {
            let mut s = board(true);
            s.play(ID, json!({}));
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -2);
        }
    }
}
