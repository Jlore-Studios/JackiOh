//! C+ #13 Mommy Barker (SPEC §8.7 row 13): (1) Unit, Human, Pancake, Legendary, 2/2 → 4/4.
//!   Base:    "Death: Add {tokens} random Pancake token(s) to your hand."
//!   Radiant: Reborn, and the same Death, which fires on both deaths (§4.5).
//! The pool is C+ #12's, the eight Pancake tokens, on their base faces (R60); a full hand burns it.

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-013";

pub fn script() -> CardScripts {
    let base = Script {
        death: Some(hook(|ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Pancake", "Token"] },
                "count": param(&*ctx, "tokens"),
            })))]
        })),
        ..Script::default()
    };
    CardScripts {
        // Reborn is catalog data; the Death is the same.
        radiant: base.clone(),
        base,
    }
}

// C+ #13 Mommy Barker — SPEC §8.7 row 13, BUILD M9 Classic+ row C+ 13: "Death adds 1 random Pancake
// token (C+ #12.1–#12.8) to your hand, hidden from the opponent (R97); exile or a bounce adds nothing; a
// full hand burns it; the count reads through `param()`; radiant 4/4 with Reborn, adding a token on
// both deaths (R8)".

/// `describe("C+ #13 Mommy Barker")`.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MOMMY: &str = "classicplus-013";
    const MENACE: &str = "core-019"; // 9/9: Mommy dies attacking it.
    const FLOOD: &str = "core-017"; // (4) Spell: bounce all Units.
    const COLLATERAL: &str = "core-034"; // (4) Spell: exile target permanent and a random card of the opponent's deck.
    const FILLER: &str = "core-005";

    /// The harness registers every card first (TS's harness did on import).
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialises")
    }

    /// TS `PANCAKE_TOKENS`: C+ #12.1 to #12.8.
    fn pancake_tokens() -> Vec<String> {
        (1..=8).map(|k| format!("classicplus-012-{k}")).collect()
    }

    /// TS `/classicplus-012-\d/`.
    fn names_a_pancake_token(text: &str) -> bool {
        (0..=9).any(|digit| text.contains(&format!("classicplus-012-{digit}")))
    }

    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        step_param(find_instance_mut(s.state_mut(), &id).expect("the card is in the game"), key, steps);
    }

    fn pancakes_in(s: &Scenario) -> Vec<String> {
        let tokens = pancake_tokens();
        s.hand(P1)
            .into_iter()
            .map(|card| card.def_id)
            .filter(|id| tokens.contains(id))
            .collect()
    }

    fn dies_attacking(radiant: bool, hand: Vec<&str>) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": hand, "field": [{ "def": MOMMY, "radiant": radiant }], "library": [FILLER] },
            "p2": { "hand": [FILLER], "field": [MENACE] },
        }));
        s.attack(MOMMY, MENACE);
        s
    }

    /// `describe("base")`.
    mod base {
        use super::*;

        /// Death adds one random Pancake token to your hand
        #[test]
        fn death_adds_one_random_pancake_token_to_your_hand() {
            let mut s = dies_attacking(false, vec![FILLER]);
            s.expect_in_zone(MOMMY, "graveyard");
            assert_eq!(pancakes_in(&s).len(), 1);
        }

        /// R97 the token is hidden from the opponent
        #[test]
        fn r97_the_token_is_hidden_from_the_opponent() {
            let s = dies_attacking(false, vec![FILLER]);
            assert!(!names_a_pancake_token(&js(&s.view(P2)).to_string()));
        }

        /// a bounce adds nothing
        #[test]
        fn a_bounce_adds_nothing() {
            let mut s = scenario(json!({
                "p1": { "hand": [FLOOD, FILLER], "field": [MOMMY], "library": [FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            s.play(FLOOD, json!({}));
            s.expect_in_zone(MOMMY, "hand");
            assert!(pancakes_in(&s).is_empty());
        }

        /// exile adds nothing: it never dies
        #[test]
        fn exile_adds_nothing_it_never_dies() {
            let mut s = scenario(json!({
                "p1": { "hand": [COLLATERAL, FILLER], "field": [MOMMY], "library": [FILLER], "mana": 8 },
                "p2": { "hand": [FILLER], "library": [FILLER] },
            }));
            let mommy = s.card(MOMMY).id.clone();
            s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": mommy }] }));
            s.expect_in_zone(MOMMY, "exile");
            assert!(pancakes_in(&s).is_empty());
        }

        /// a full hand burns it
        #[test]
        fn a_full_hand_burns_it() {
            let s = dies_attacking(false, vec![FILLER; HAND_CAP as usize]);
            assert!(pancakes_in(&s).is_empty());
            let tokens = pancake_tokens();
            assert!(
                s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Burned { def_id, .. } if tokens.contains(def_id)))
            );
        }

        /// R386 the count reads through param(): an Upgrade adds 2
        #[test]
        fn r386_the_count_reads_through_param_an_upgrade_adds_2() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [MOMMY], "library": [FILLER] },
                "p2": { "hand": [FILLER], "field": [MENACE] },
            }));
            step(&mut s, MOMMY, "tokens", 1);
            s.attack(MOMMY, MENACE);
            assert_eq!(pancakes_in(&s).len(), 2);
        }
    }

    /// `describe("radiant")`.
    mod radiant {
        use super::*;

        /// 4/4 with Reborn; a token on each of both deaths (R8)
        #[test]
        fn four_4_with_reborn_a_token_on_each_of_both_deaths_r8() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [{ "def": MOMMY, "radiant": true }], "library": [FILLER] },
                "p2": { "hand": [FILLER], "field": [MENACE] },
            }));
            s.expect_stats(MOMMY, json!({ "attack": 4, "health": 4 }));
            assert!(s.stats(MOMMY).keywords.contains(&Keyword::Reborn));
            s.attack(MOMMY, MENACE); // first death: Reborn brings it back
            assert_eq!(pancakes_in(&s).len(), 1);
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id).as_deref(), Some(MOMMY));
            s.end_turn().end_turn();
            let reborn = s.unit(P1, 1).map(|card| card.id).unwrap_or_else(|| MOMMY.to_string());
            s.attack(&reborn, MENACE); // second death
            assert_eq!(pancakes_in(&s).len(), 2);
            // §5.1: added on their base faces.
            let tokens = pancake_tokens();
            assert!(
                s.hand(P1)
                    .iter()
                    .filter(|card| tokens.contains(&card.def_id))
                    .all(|card| !card.radiant)
            );
        }
    }
}
