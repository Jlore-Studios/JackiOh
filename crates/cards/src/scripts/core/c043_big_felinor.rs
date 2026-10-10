//! #43 Big Felinor (SPEC §8.2). Unit 3/10 → 6/20, Felinor, cost 3, Rare.
//!   Base:    "Cry: destroy all non-Felinor units on both sides"
//!   Radiant: "Enemy non-Felinor units only" (§8 Conventions: a restated clause replaces the base one).
//!   Engine:  "Simultaneous destroy, one state check."
//!
//! R59 IS WHY THIS IS ONE EFFECT: `destroy` only MARKS a card (`markedDestroyed`) and §4.5 step 1
//! "collects" every mark "at once"; the state check runs after the Cry's whole list (§10.3), never
//! between its effects, so nothing dies early while the rest of the board is still being marked.
//! `destroy_all` marks every active unit its `BoardScope` matches (`side` is relative to `ctx.controller`,
//! `tags`/`notTags` read the def's §5 tags), so a card dormant under a Stack is not hit (R13).
//! Indestructible is §4.5's business, not the filter's (R46, R69). Big Felinor is Felinor-tagged, so
//! `notTags: ["Felinor"]` already spares it; the tests prove that.

use jackioh_engine::effects::destroy_all;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-043";

/// The faces differ only in which sides the destroy reaches (`"any"` or `"enemy"`).
fn big_felinor(side: &'static str) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![destroy_all(json_as(json!({ "side": side, "notTags": ["Felinor"] })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: big_felinor("any"),
        radiant: big_felinor("enemy"),
    }
}

// #43 Big Felinor (SPEC §8.2, §4.5, §6.3 Destroy; R11, R46, R59, R69).
// Must-pass row (BUILD M4-T4 #43): "Non-Felinors on both sides destroyed, Felinors and itself
// survive; radiant enemy side only."
// One play producing two `destroyed` events, one per side, is R59's single state check.
// The board keeps one def per slot so a string reference is never ambiguous:
//   p1  lane 2  #20 Pointmaster   7/2 Human            non-Felinor, dies (base) / lives (radiant)
//   p1  lane 4  Felinor Token     1/1 Felinor, Token   survives both faces
//   p2  lane 1  #12 Duplicating Felinors 3/4 Felinor   survives both faces
//   p2  lane 2  #25 4-mana 7/7    7/7 Armor 7          non-Felinor, dies on both faces
// Nothing on that board has a Cry that could fire (units placed by the builder are not played), and
// Armor is irrelevant to a destroy, which is not damage (§6.3).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    /// Big Felinor waits in p1's hand; #21 Hinder rides along so the turn does not auto-end (R82).
    /// `radiant_felinor` flags the hand card Radiant.
    fn board(indestructible_enemy: bool, radiant_felinor: bool) -> Scenario {
        scenario(json!({
            "seed": "big-felinor",
            "p1": {
                "hand": [{ "def": "core-043", "radiant": radiant_felinor }, "core-021"],
                "field": [
                    { "def": "core-020", "lane": 2 },
                    { "def": "core-t-felinor", "lane": 4 },
                ],
            },
            "p2": {
                "field": [
                    { "def": "core-012", "lane": 1 },
                    // #66 The Rock is Indestructible; #25 4-mana 7/7 is not.
                    { "def": if indestructible_enemy { "core-066" } else { "core-025" }, "lane": 2 },
                ],
            },
        }))
    }

    fn def_at(s: &Scenario, player: PlayerId, lane: i32) -> Option<String> {
        s.unit(player, lane).map(|card| card.def_id)
    }

    mod base {
        use super::*;

        #[test]
        fn r59_destroys_every_non_felinor_unit_on_both_sides_in_one_state_check() {
            let mut s = board(false, false);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_in_zone("core-020", "graveyard");
            s.expect_in_zone("core-025", "graveyard");
            // Two deaths out of one Cry: neither died before the other was marked.
            s.expect_events(json!(["cardPlayed", "destroyed", "destroyed"]));
            assert!(s.unit(PlayerId::P1, 2).is_none());
            assert!(s.unit(PlayerId::P2, 2).is_none());
        }

        #[test]
        fn spares_felinor_units_on_both_sides() {
            let mut s = board(false, false);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_in_zone("core-012", "field");
            s.expect_in_zone("core-t-felinor", "field");
            assert_eq!(def_at(&s, PlayerId::P2, 1).as_deref(), Some("core-012"));
            assert_eq!(def_at(&s, PlayerId::P1, 4).as_deref(), Some("core-t-felinor"));
        }

        #[test]
        fn spares_itself_because_big_felinor_is_felinor_tagged_and_not_tags_already_excludes_it() {
            let mut s = board(false, false);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_in_zone("core-043", "field");
            s.expect_stats("core-043", json!({ "attack": 3, "health": 10, "maxHealth": 10 }));
            assert_eq!(def_at(&s, PlayerId::P1, 3).as_deref(), Some("core-043"));
        }

        #[test]
        fn r46_an_indestructible_non_felinor_ignores_the_destroy_mark_and_switches_to_attack_position() {
            // #66 The Rock is Indestructible, so §4.5 step 1 leaves it on the field.
            let mut s = board(true, false);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_in_zone("core-066", "field");
            assert_eq!(s.unit(PlayerId::P2, 2).and_then(|card| card.position), Some(Position::Atk));
            // The other non-Felinor is not Indestructible and still dies in the same check.
            s.expect_in_zone("core-020", "graveyard");
        }

        #[test]
        fn s8_conventions_an_empty_target_set_fizzles_and_the_unit_still_enters_the_field() {
            let mut s = scenario(json!({ "seed": "big-felinor", "p1": { "hand": ["core-043", "core-021"] } }));
            s.play("core-043", json!({ "zone": 1 }));

            s.expect_in_zone("core-043", "field");
            s.expect_events(json!(["cardPlayed", "summoned"]));
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Destroyed));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s8_conventions_the_restated_clause_replaces_the_base_one_so_only_enemy_non_felinors_die() {
            let mut s = board(false, true);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_in_zone("core-025", "graveyard");
            // Your own non-Felinor is untouched: that is the whole difference between the faces.
            s.expect_in_zone("core-020", "field");
            assert_eq!(def_at(&s, PlayerId::P1, 2).as_deref(), Some("core-020"));
        }

        #[test]
        fn still_spares_felinors_on_both_sides_and_itself() {
            let mut s = board(false, true);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_in_zone("core-012", "field");
            s.expect_in_zone("core-t-felinor", "field");
            s.expect_in_zone("core-043", "field");
            s.expect_stats("core-043", json!({ "attack": 6, "health": 20, "maxHealth": 20 }));
        }

        #[test]
        fn r59_the_one_enemy_death_is_still_the_single_state_check_after_the_whole_cry() {
            let mut s = board(false, true);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_events(json!(["cardPlayed", "destroyed"]));
            assert_eq!(
                s.events().iter().filter(|event| event.event_type() == GameEventType::Destroyed).count(),
                1,
            );
        }

        #[test]
        fn r46_an_indestructible_enemy_non_felinor_survives_the_radiant_face_too() {
            let mut s = board(true, true);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_in_zone("core-066", "field");
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Destroyed));
        }
    }
}
