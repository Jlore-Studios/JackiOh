//! #43 Big Felinor (SPEC §8.2). Unit 3/10 → 6/20, Felinor, cost 3, Rare.
//!   Base:    "Cry: destroy all non-Felinor units on both sides"
//!   Radiant: "Enemy non-Felinor units only" — §8 Conventions: a restated clause replaces the base
//!            version, so the radiant Cry is the same destroy narrowed to one side.
//!   Engine:  "Simultaneous destroy, one state check."
//!
//! R59 IS WHY THIS IS ONE EFFECT. "The state check never runs between the damage instances of a
//! single effect", and §10.3 adds that "Apply effect means a whole script's effect list", so the
//! check comes after the Cry's list either way. `destroy` only MARKS a card (`markedDestroyed`) and
//! §4.5 step 1 "collects" every mark "at once", so nothing here can die early and take a Death
//! trigger's side effects with it while the rest of the board is still being marked.
//!
//! MISSING VERB. `destroy({ target })` takes a `TargetSpec`, which is only `self | selfHero |
//! enemyHero | chosen` — it cannot name a computed instance, and nothing chose these units, so a
//! board-wide destroy is not expressible today. The call below is the verb the engine must add:
//!
//!   destroyAll({ side: "any" | "self" | "enemy", notTags?, tags?, excludeSelf? })
//!       One effect that marks every unit the filter matches, so the single state check after the
//!       whole effect collects them together (R59, §4.5). `side` is relative to `ctx.controller`;
//!       `tags`/`notTags` read the def's §5 tags (tags are printed data, not a layer); only active
//!       units are matched, so a card dormant under a Stack is neither marked nor hit (R13);
//!       Indestructible is §4.5's business, not the filter's (R46, R69).
//!
//! See the agent report for the alternative the engine team may prefer instead: adding
//! `{ of: "instance"; instanceId: string }` to `TargetSpec`, which unblocks every board-wide card at
//! once. Either way the card file stays a list of effects.
//!
//! "AND ITSELF": Big Felinor is Felinor-tagged in `catalog.json`, so `notTags: ["Felinor"]` already
//! spares it and `excludeSelf` would be a second, redundant reason. The tests below prove that
//! rather than asserting it here.
//!
//! (Rust: the verb is `effects::destroy_all(BoardScope)`, handed its TS object literal through
//! `json_as`.)

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
//
// The must-pass row (BUILD M4-T4 #43): "Non-Felinors on both sides destroyed, Felinors and itself
// survive; radiant enemy side only."
//
// R59 is what makes the two deaths one event rather than two: `destroy` only MARKS a card, §4.5
// step 1 collects every mark "at once", and the state check runs after the whole Cry, never between
// two effects of it. So a single play producing two `destroyed` events — one per side — is the
// assertion, not a sequence of separate checks.
//
// The board keeps one def per slot so a string reference is never ambiguous (harness header):
//   p1  lane 2  #20 Pointmaster   7/2 Human            non-Felinor, dies (base) / lives (radiant)
//   p1  lane 4  Felinor Token     1/1 Felinor, Token   survives both faces
//   p2  lane 1  #12 Duplicating Felinors 3/4 Felinor   survives both faces
//   p2  lane 2  #25 4-mana 7/7    7/7 Armor 7          non-Felinor, dies on both faces
// Nothing on that board has a Cry that could fire (units placed by the builder are not played), and
// Armor is irrelevant to a destroy, which is not damage (§6.3).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    /// TS's harness registered the real catalog and every script on import (`registerAll()`); the
    /// Rust testkit cannot name this crate, so the scenario builder registers first.
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    /// Big Felinor waits in p1's hand; #21 Hinder rides along so the turn does not auto-end (R82).
    /// `radiant_felinor` flags the hand card Radiant (TS set the flag on it after the build; the setup
    /// entry's own `radiant` sets the same flag on the same freshly created hand card).
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

    /// "#43 Big Felinor — base"
    mod base {
        use super::*;

        /// "R59 destroys every non-Felinor unit on both sides in one state check"
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

        /// "spares Felinor units on both sides"
        #[test]
        fn spares_felinor_units_on_both_sides() {
            let mut s = board(false, false);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_in_zone("core-012", "field");
            s.expect_in_zone("core-t-felinor", "field");
            assert_eq!(def_at(&s, PlayerId::P2, 1).as_deref(), Some("core-012"));
            assert_eq!(def_at(&s, PlayerId::P1, 4).as_deref(), Some("core-t-felinor"));
        }

        /// "spares itself, because Big Felinor is Felinor-tagged and `notTags` already excludes it"
        #[test]
        fn spares_itself_because_big_felinor_is_felinor_tagged_and_not_tags_already_excludes_it() {
            let mut s = board(false, false);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_in_zone("core-043", "field");
            s.expect_stats("core-043", json!({ "attack": 3, "health": 10, "maxHealth": 10 }));
            assert_eq!(def_at(&s, PlayerId::P1, 3).as_deref(), Some("core-043"));
        }

        /// "R46 an Indestructible non-Felinor ignores the destroy mark and switches to Attack Position"
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

        /// "§8 Conventions: an empty target set fizzles and the unit still enters the field"
        #[test]
        fn s8_conventions_an_empty_target_set_fizzles_and_the_unit_still_enters_the_field() {
            let mut s = scenario(json!({ "seed": "big-felinor", "p1": { "hand": ["core-043", "core-021"] } }));
            s.play("core-043", json!({ "zone": 1 }));

            s.expect_in_zone("core-043", "field");
            s.expect_events(json!(["cardPlayed", "summoned"]));
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Destroyed));
        }
    }

    /// "#43 Big Felinor — radiant"
    mod radiant {
        use super::*;

        /// "§8 Conventions: the restated clause replaces the base one, so only enemy non-Felinors die"
        #[test]
        fn s8_conventions_the_restated_clause_replaces_the_base_one_so_only_enemy_non_felinors_die() {
            let mut s = board(false, true);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_in_zone("core-025", "graveyard");
            // Your own non-Felinor is untouched: that is the whole difference between the faces.
            s.expect_in_zone("core-020", "field");
            assert_eq!(def_at(&s, PlayerId::P1, 2).as_deref(), Some("core-020"));
        }

        /// "still spares Felinors on both sides, and itself"
        #[test]
        fn still_spares_felinors_on_both_sides_and_itself() {
            let mut s = board(false, true);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_in_zone("core-012", "field");
            s.expect_in_zone("core-t-felinor", "field");
            s.expect_in_zone("core-043", "field");
            s.expect_stats("core-043", json!({ "attack": 6, "health": 20, "maxHealth": 20 }));
        }

        /// "R59 the one enemy death is still the single state check after the whole Cry"
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

        /// "R46 an Indestructible enemy non-Felinor survives the radiant face too"
        #[test]
        fn r46_an_indestructible_enemy_non_felinor_survives_the_radiant_face_too() {
            let mut s = board(true, true);
            s.play("core-043", json!({ "zone": 3 }));

            s.expect_in_zone("core-066", "field");
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Destroyed));
        }
    }
}
