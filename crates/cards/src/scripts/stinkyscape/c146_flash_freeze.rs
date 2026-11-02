//! S #146 Flash Freeze (SPEC §8.10 row S 146; §6.1, §6.2, R58, R1174000, R1867009): (0) Spell, Rare.
//!   Base:    "Cast on draw: Freeze all Units."
//!   Radiant: "Cast on draw: Freeze all enemy Units."
//!
//! Engine: Cast on draw is the static flag §6.2's draw reads (R58, R70; C+ #36.1 Bone Storm's shape):
//! drawn, it is cast at no cost and counted as played, and its player draws again; set aside through
//! the setup (R635). Played from the hand at (0) it does the same. The Cry is `freeze_all` over
//! every Unit of both sides (R1174000) — the Radiant face over the enemy's alone — a hero passed by
//! and a Unit Immune to Spells too (R81). A Unit Frozen on its controller's turn loses that turn's
//! attack (R1867009).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-146";

fn flash_freeze(radiant: bool) -> Script {
    let mut scope = json!({});
    if radiant {
        scope["side"] = json!("enemy");
    }
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |_ctx| vec![freeze_all(json_as(scope.clone()))])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: flash_freeze(false),
        radiant: flash_freeze(true),
    }
}

// S #146 Flash Freeze — SPEC §8.10 row S 146, BUILD M12 row S 146 (#36, R1174000, R1867009, R58,
// R70).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{def_ids, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FREEZE: &str = "stinkyscape-146";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const TIMMY: &str = "core-011"; // (1) Unit 3/3, keeps the turn alive.
    const FILLER: &str = "core-005";

    fn storm(radiant: bool) -> Value {
        json!({ "def": FREEZE, "radiant": radiant })
    }

    fn frozen(s: &Scenario, card: &CardInstance) -> bool {
        s.card(card).frozen.is_some()
    }

    fn draw_freeze(library: Vec<Value>) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [TIMMY], "library": library },
            "p2": { "hand": [FILLER], "field": [VANILLA], "library": [FILLER, FILLER, FILLER] },
        }));
        s.end_turn(); // p2's turn.
        s.end_turn(); // p1's turn: the draw.
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r1867009_drawn_it_freezes_every_unit_of_both_sides_and_you_draw_again() {
            let mut s = draw_freeze(vec![storm(false), json!(FILLER), json!(FILLER)]);

            let mine = s.unit_at(P1, 1);
            let theirs = s.unit_at(P2, 1);
            assert!(frozen(&s, &mine));
            assert!(frozen(&s, &theirs));
            s.expect_in_zone(FREEZE, "graveyard");
            assert_eq!(def_ids(&s.hand(P1)), vec![FILLER, FILLER]);
        }

        #[test]
        fn played_from_the_hand_it_does_the_same() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FREEZE, FILLER], "field": [TIMMY] },
                "p2": { "hand": [FILLER], "field": [VANILLA] },
            }));
            let mine = s.unit_at(P1, 1);
            let theirs = s.unit_at(P2, 1);

            s.play(FREEZE, json!({}));

            assert!(frozen(&s, &mine));
            assert!(frozen(&s, &theirs));
            s.expect_in_zone(FREEZE, "graveyard");
        }

        #[test]
        fn r1174000_yours_thaws_at_this_turns_end_the_enemys_at_the_end_of_its_controllers_next_turn() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FREEZE, FILLER], "field": [TIMMY], "library": [FILLER, FILLER, FILLER] },
                "p2": { "hand": [FILLER], "field": [VANILLA], "library": [FILLER, FILLER, FILLER] },
            }));
            let mine = s.unit_at(P1, 1);
            let theirs = s.unit_at(P2, 1);
            s.play(FREEZE, json!({}));
            assert!(frozen(&s, &mine) && frozen(&s, &theirs));

            s.end_turn(); // p1's turn ends: their own Unit, frozen on their turn, thaws.
            assert_eq!(s.state().active, P2);
            assert!(
                !frozen(&s, &mine),
                "frozen on its controller's turn, it thaws at that turn's end"
            );
            assert!(
                frozen(&s, &theirs),
                "the enemy's stays Frozen through its controller's next turn"
            );

            s.end_turn(); // p2's turn ends: theirs thaws.
            assert!(
                !frozen(&s, &theirs),
                "it thaws at the end of its controller's next turn"
            );
        }

        #[test]
        fn r1867009_a_unit_frozen_on_its_controllers_turn_loses_that_turns_attack() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FREEZE, FILLER], "field": [TIMMY, VANILLA] },
                "p2": { "hand": [FILLER], "field": [VANILLA] },
            }));
            let mine = s.unit_at(P1, 1);
            s.play(FREEZE, json!({}));
            assert!(frozen(&s, &mine));

            // Frozen: no attack to declare (R1174000).
            let attacks: Vec<Value> = legal_actions(s.state(), P1)
                .iter()
                .map(crate::js)
                .filter(|action| {
                    action["type"] == "attack" && action["instanceId"] == mine.id.as_str()
                })
                .collect();
            assert!(attacks.is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1867009_drawn_it_freezes_only_the_enemys_units() {
            let mut s = draw_freeze(vec![storm(true), json!(FILLER), json!(FILLER)]);

            assert!(!frozen(&s, &s.unit_at(P1, 1)));
            assert!(frozen(&s, &s.unit_at(P2, 1)));
            s.expect_in_zone(FREEZE, "graveyard");
            assert_eq!(def_ids(&s.hand(P1)), vec![FILLER, FILLER]);
        }

        #[test]
        fn played_from_the_hand_it_freezes_only_the_enemys_units() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": FREEZE, "radiant": true }, FILLER], "field": [TIMMY] },
                "p2": { "hand": [FILLER], "field": [VANILLA] },
            }));

            s.play(FREEZE, json!({}));

            assert!(!frozen(&s, &s.unit_at(P1, 1)));
            assert!(frozen(&s, &s.unit_at(P2, 1)));
        }
    }
}
