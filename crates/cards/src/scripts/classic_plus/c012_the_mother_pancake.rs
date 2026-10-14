//! C+ #12 The Mother Pancake (SPEC §8.7 row 12): (3) Unit, Pancake, Legendary, 8/8 → 16/16.
//!   Base:    "Taunt. End of turn: Add {tokens} random Pancake token(s) to your hand."
//!   Radiant: the same at 16/16 with 2 tokens (the catalog's `tokens` param).
//! The pool is the eight Pancake tokens, C+ #12.1–#12.8, which the text names (§5.1), on their base
//! faces, repeats allowed (R60); a full hand burns what doesn't fit (§2.4).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-012";

pub fn script() -> CardScripts {
    let base = Script {
        // The eight Pancake tokens: Pancake- and Token-tagged (Mother Pancake and Mommy Barker are no tokens).
        end_of_turn: Some(hook(|ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Pancake", "Token"] },
                "count": param(&*ctx, "tokens"),
            })))]
        })),
        ..Script::default()
    };
    CardScripts {
        // The Radiant face differs only in its stats and its `tokens` value, both catalog data.
        radiant: base.clone(),
        base,
    }
}

// C+ #12 The Mother Pancake (SPEC §8.7 row 12): at controller's end of turn adds 1 random
// card of eight Pancake tokens C+ #12.1–#12.8 (repeats allowed, R60); not at opponent's
// end; a full hand burns it (R4); opponent sees a card added under sentinel (R97); radiant
// adds 2.

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MOTHER: &str = "classicplus-012";
    const FILLER: &str = "core-005";

    use crate::scenario;

    use crate::js;

    /// C+ #12.1 to #12.8.
    fn pancake_tokens() -> Vec<String> {
        (1..=8).map(|k| format!("classicplus-012-{k}")).collect()
    }

    fn names_a_pancake_token(text: &str) -> bool {
        (0..=9).any(|digit| text.contains(&format!("classicplus-012-{digit}")))
    }

    /// The scenario after p1's end of turn, and the def ids it added.
    fn at_end_of_turn(radiant: bool, seed: Option<String>, hand: Option<Vec<&str>>) -> (Scenario, Vec<String>) {
        let hand = hand.unwrap_or_else(|| vec![FILLER]);
        let mut opts = json!({
            "p1": { "hand": hand, "field": [{ "def": MOTHER, "radiant": radiant }], "library": [FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        });
        if let Some(seed) = seed {
            opts["seed"] = json!(seed);
        }
        let mut s = scenario(opts);
        let before: IndexSet<String> = s.hand(P1).into_iter().map(|card| card.id).collect();
        s.end_turn();
        let added = s
            .hand(P1)
            .into_iter()
            .filter(|card| !before.contains(&card.id))
            .map(|card| card.def_id)
            .collect();
        (s, added)
    }

    mod base {
        use super::*;

        /// prints Taunt
        #[test]
        fn prints_taunt() {
            let s = scenario(json!({ "p1": { "field": [MOTHER] } }));
            assert!(s.stats(MOTHER).keywords.contains(&Keyword::Taunt));
        }

        /// at its controller's end of turn adds one Pancake token
        #[test]
        fn at_its_controllers_end_of_turn_adds_one_pancake_token() {
            let (_s, added) = at_end_of_turn(false, None, None);
            assert_eq!(added.len(), 1);
            assert!(pancake_tokens().contains(&added[0]));
        }

        /// R60 the pool is exactly the eight Pancake tokens, repeats allowed, never Mother Pancake or Mommy Barker
        #[test]
        fn r60_the_pool_is_exactly_the_eight_pancake_tokens_repeats_allowed_never_mother_pancake_or_mommy_barker() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for i in 0..40 {
                let (_s, added) = at_end_of_turn(true, Some(format!("mp-{i}")), None);
                assert_eq!(added.len(), 2);
                seen.extend(added);
            }
            let mut seen: Vec<String> = seen.into_iter().collect();
            seen.sort();
            let mut expected = pancake_tokens();
            expected.sort();
            assert_eq!(seen, expected);
        }

        /// not at the opponent's end of turn
        #[test]
        fn not_at_the_opponents_end_of_turn() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [FILLER], "field": [MOTHER], "library": [FILLER, FILLER] },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            }));
            s.end_turn(); // p2's end of turn
            let tokens = pancake_tokens();
            assert!(s.hand(P1).iter().all(|card| !tokens.contains(&card.def_id)));
        }

        /// R4 a full hand burns it
        #[test]
        fn r4_a_full_hand_burns_it() {
            let (s, added) = at_end_of_turn(false, None, Some(vec![FILLER; HAND_CAP as usize]));
            assert!(added.is_empty());
            let tokens = pancake_tokens();
            assert!(
                s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Burned { def_id, .. } if tokens.contains(def_id)))
            );
        }

        /// R97 the opponent sees a card added under the sentinel
        #[test]
        fn r97_the_opponent_sees_a_card_added_under_the_sentinel() {
            let (s, _added) = at_end_of_turn(false, None, None);
            let view = s.view(P2);
            let shown: Vec<&GameEvent> = view
                .events
                .iter()
                .filter(|event| event.event_type() == GameEventType::AddedToHand)
                .collect();
            assert!(!shown.is_empty());
            let tokens = pancake_tokens();
            for event in &shown {
                if let GameEvent::AddedToHand { def_id, .. } = event {
                    assert!(!tokens.contains(def_id));
                }
            }
            assert!(!names_a_pancake_token(&js(&view).to_string()));
        }

        /// R386 the count reads through param(): an Upgrade adds 2
        #[test]
        fn r386_the_count_reads_through_param_an_upgrade_adds_2() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [MOTHER], "library": [FILLER, FILLER] },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            }));
            step_param(s.card_mut(MOTHER), "tokens", 1);
            let before = s.hand(P1).len();
            s.end_turn();
            assert_eq!(s.hand(P1).len(), before + 2);
        }
    }

    mod radiant {
        use super::*;

        /// 16/16 Taunt, adding 2 Pancake tokens
        #[test]
        fn sixteen_16_taunt_adding_2_pancake_tokens() {
            let (mut s, added) = at_end_of_turn(true, None, None);
            s.expect_stats(MOTHER, json!({ "attack": 16, "health": 16 }));
            assert_eq!(added.len(), 2);
            let tokens = pancake_tokens();
            for id in &added {
                assert!(tokens.contains(id));
            }
            // §5.1: each is added on its base face, whatever the Mother's face.
            let faces: Vec<bool> = s
                .hand(P1)
                .iter()
                .filter(|card| tokens.contains(&card.def_id))
                .map(|card| card.radiant)
                .collect();
            assert_eq!(faces, vec![false, false]);
        }
    }
}
