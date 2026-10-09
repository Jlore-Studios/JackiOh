//! C #13 Boots on the Ground (SPEC §8.6 row 13). (1) Unit, Human, Common, 2/1 → 4/2.
//!   Base:    "Charge\nAfter this attacks, draw {draw}." — draw 1
//!   Radiant: "Charge\nAfter this attacks, Recruit {recruits|card|cards}." — recruits 1
//!
//! Charge is printed on both catalog faces, so §10.4 layer 1 grants it and nothing here does.
//!
//! "After this attacks" is the `afterAttack` hook: the engine runs it after the state check that closes
//! each combat this card attacked in, declared or forced (R53, #9 Moths to the Flame), on its
//! last-known snapshot as a Death hook runs (R78, R89). Defending is not attacking. A trigger on
//! `attackDeclared` could not serve: a queued entry is dropped when its card leaves the field (R174),
//! and a forced attack's event reaches no card that died in its combat (R212).
//! Radiant: §6.3 Recruit, per R64 with no Cry (R1), a Trap face-down (R33). Numbers: `param` (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-013";

pub fn script() -> CardScripts {
    // "After this attacks, draw {draw}."
    let base = Script {
        after_attack: Some(hook(|ctx| vec![draw(json_as(json!({ "count": param(&*ctx, "draw") })))])),
        ..Script::default()
    };

    // "After this attacks, Recruit {recruits|card|cards}.": one top-down scan per recruit.
    let radiant = Script {
        after_attack: Some(hook(|ctx| {
            (0..param(&*ctx, "recruits"))
                .map(|_| recruit(json_as(json!({ "player": "self" }))))
                .collect()
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// C #13 Boots on the Ground — SPEC §8.6 row 13, BUILD M9 Classic row C 13: draws 1 after each combat it
// attacked in, forced ones included, even when it died (last-known state, R78); defending draws nothing.
// Radiant 4/2: Recruits instead, a recruited trap landing face-down (R33); numbers via `param()` (R386).
// Forced attacks come from #9 Moths to the Flame (start of turn: every enemy Unit attacks this).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOOTS: &str = "classic-013";
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const MOTHS: &str = "core-009"; // 1/14: start of turn, every enemy Unit attacks this.
    const TEMPO: &str = "core-011"; // (1) Unit 3/3 Rush.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const STOCKPILE: &str = "core-005"; // (1) Spell.
    const SHEEPISH: &str = "core-041"; // (1) Trap.
    const ANCHOR: &str = "core-010";
    const A: &str = "core-020";
    const B: &str = "core-001";

    fn hand_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player).into_iter().map(|card| card.def_id).collect()
    }

    /// Lanes 1–5, `None` for an empty zone.
    fn unit_defs(s: &Scenario, player: PlayerId) -> Vec<Option<String>> {
        (1..=5).map(|lane| s.unit(player, lane).map(|card| card.def_id)).collect()
    }

    fn lanes(expected: [Option<&str>; 5]) -> Vec<Option<String>> {
        expected.iter().map(|lane| lane.map(str::to_string)).collect()
    }

    fn draws_by(s: &Scenario, who: PlayerId) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Drawn { player, .. } if *player == who))
            .count()
    }

    fn library_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.pile(player, "library").into_iter().map(|card| card.def_id).collect()
    }

    fn has_charge(s: &Scenario, card: &str) -> bool {
        s.stats(card).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Charge)
    }

    #[test]
    fn declares_its_text_as_an_afterattack_hook_on_each_face_and_no_trigger() {
        assert_eq!(crate::card_def(ID).id, BOOTS);
        let CardScripts { base, radiant } = script();
        assert!(base.after_attack.is_some());
        assert!(radiant.after_attack.is_some());
        assert!(base.triggers.is_empty());
    }

    mod base {
        use super::*;

        #[test]
        fn is_a_2_1_with_charge_played_this_turn_it_hits_the_hero_at_once_and_then_draws_1() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOTS, ANCHOR], "library": [A, B] }, "p2": { "hand": [ANCHOR] } }));

            s.play(BOOTS, json!({}));
            s.expect_stats(BOOTS, json!({ "attack": 2, "health": 1 }));
            s.attack(BOOTS, "hero");

            s.expect_health(P2, 28);
            assert_eq!(hand_defs(&s, P1), [ANCHOR, A]);
            s.expect_events(json!(["attackDeclared", "damage", "drawn"]));
        }

        #[test]
        fn sec6_1_charge_the_turn_it_is_played_it_may_attack_the_hero() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOTS, ANCHOR], "library": [A] }, "p2": { "hand": [ANCHOR] } }));

            s.play(BOOTS, json!({}));
            assert!(has_charge(&s, BOOTS));
            s.attack(BOOTS, "hero");

            s.expect_health(P2, 28);
        }

        #[test]
        fn draws_once_per_attack_not_twice() {
            let mut s = scenario(json!({ "p1": { "hand": [ANCHOR], "field": [BOOTS], "library": [A, B] }, "p2": { "hand": [ANCHOR] } }));

            s.attack(BOOTS, "hero");

            assert_eq!(draws_by(&s, P1), 1);
        }

        #[test]
        fn r78_it_draws_even_when_it_died_in_that_combat_reading_its_last_known_state() {
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [BOOTS], "library": [A, B] },
                "p2": { "hand": [ANCHOR], "field": [MENACE] },
            }));

            s.attack(BOOTS, MENACE);

            s.expect_in_zone(BOOTS, "graveyard");
            assert_eq!(hand_defs(&s, P1), [ANCHOR, A]);
            assert_eq!(draws_by(&s, P1), 1);
        }

        #[test]
        fn r53_r78_a_forced_attack_counts_moths_to_the_flame_makes_it_attack_it_dies_and_it_still_draws() {
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [BOOTS], "library": [A, B] },
                "p2": { "hand": [ANCHOR], "field": [MOTHS], "library": [VANILLA] },
            }));

            s.end_turn();

            assert!(s.events().iter().any(|event| matches!(event, GameEvent::AttackDeclared { forced: true, .. })));
            s.expect_in_zone(BOOTS, "graveyard");
            assert_eq!(hand_defs(&s, P1), [ANCHOR, A]);
        }

        #[test]
        fn defending_draws_nothing_even_when_it_dies_defending() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [ANCHOR], "field": [BOOTS], "library": [A] },
                "p2": { "hand": [ANCHOR], "field": [TEMPO] },
            }));

            s.attack(TEMPO, BOOTS);

            s.expect_in_zone(BOOTS, "graveyard");
            assert_eq!(draws_by(&s, P1), 0);
        }

        #[test]
        fn a_dead_one_in_the_graveyard_never_answers_another_unit_s_attack() {
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [BOOTS, VANILLA], "library": [A, B] },
                "p2": { "hand": [ANCHOR], "field": [MENACE] },
            }));

            s.attack(BOOTS, MENACE);
            assert_eq!(draws_by(&s, P1), 1);
            s.expect_in_zone(BOOTS, "graveyard");
            s.attack(VANILLA, MENACE);

            assert_eq!(draws_by(&s, P1), 1);
        }

        #[test]
        fn r386_an_upgrade_makes_it_draw_2() {
            let mut s = scenario(json!({ "p1": { "hand": [ANCHOR], "field": [BOOTS], "library": [A, B] }, "p2": { "hand": [ANCHOR] } }));
            step_param(s.card_mut(BOOTS), "draw", 1);

            s.attack(BOOTS, "hero");

            assert_eq!(hand_defs(&s, P1), [ANCHOR, A, B]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_a_4_2_with_charge_that_after_attacking_recruits_the_first_permanent_from_the_top_of_your_deck_instead_of_drawing() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": BOOTS, "radiant": true }, ANCHOR], "library": [STOCKPILE, TEMPO, VANILLA] },
                "p2": { "hand": [ANCHOR] },
            }));

            s.play(BOOTS, json!({}));
            s.expect_stats(BOOTS, json!({ "attack": 4, "health": 2 }));
            s.attack(BOOTS, "hero");

            s.expect_health(P2, 26);
            assert_eq!(unit_defs(&s, P1), lanes([Some(BOOTS), Some(TEMPO), None, None, None]));
            assert_eq!(library_defs(&s, P1), [STOCKPILE, VANILLA]);
            assert_eq!(draws_by(&s, P1), 0);
        }

        #[test]
        fn sec6_1_is_a_4_2_with_charge_the_turn_it_is_played_it_may_attack_the_hero() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": BOOTS, "radiant": true }, ANCHOR], "library": [STOCKPILE] },
                "p2": { "hand": [ANCHOR] },
            }));

            s.play(BOOTS, json!({}));
            s.expect_stats(BOOTS, json!({ "attack": 4, "health": 2 }));
            assert!(has_charge(&s, BOOTS));
            s.attack(BOOTS, "hero");

            s.expect_health(P2, 26);
        }

        #[test]
        fn defending_recruits_nothing() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [ANCHOR], "field": [{ "def": BOOTS, "radiant": true }], "library": [TEMPO] },
                "p2": { "hand": [ANCHOR], "field": [TEMPO] },
            }));

            match s.unit(P2, 1) {
                Some(attacker) => s.attack(&attacker, BOOTS),
                None => s.attack(TEMPO, BOOTS),
            };

            assert_eq!(library_defs(&s, P1), [TEMPO]);
            assert_eq!(unit_defs(&s, P1), lanes([None, None, None, None, None]));
        }

        #[test]
        fn with_no_permanent_in_the_deck_nothing_happens() {
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [{ "def": BOOTS, "radiant": true }], "library": [STOCKPILE] },
                "p2": { "hand": [ANCHOR] },
            }));

            s.attack(BOOTS, "hero");

            assert_eq!(unit_defs(&s, P1), lanes([Some(BOOTS), None, None, None, None]));
            assert_eq!(library_defs(&s, P1), [STOCKPILE]);
        }

        #[test]
        fn with_its_row_full_nothing_happens_and_the_card_stays_in_the_deck() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [ANCHOR],
                    "field": [{ "def": BOOTS, "radiant": true }, VANILLA, VANILLA, VANILLA, VANILLA],
                    "library": [TEMPO],
                },
                "p2": { "hand": [ANCHOR] },
            }));

            s.attack(BOOTS, "hero");

            assert_eq!(library_defs(&s, P1), [TEMPO]);
        }

        #[test]
        fn r33_a_recruited_trap_lands_face_down_in_the_backrow_and_is_never_named_in_the_opponent_s_view() {
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [{ "def": BOOTS, "radiant": true }], "library": [SHEEPISH] },
                "p2": { "hand": [ANCHOR] },
            }));

            s.attack(BOOTS, "hero");

            let trap = s.backrow(P1, 1);
            assert_eq!(trap.as_ref().map(|card| card.def_id.as_str()), Some(SHEEPISH));
            assert_ne!(trap.as_ref().and_then(|card| card.face_up), Some(true));
            assert!(!serde_json::to_string(&s.view(P2)).unwrap().contains(SHEEPISH));
            assert!(serde_json::to_string(&s.view(P1)).unwrap().contains(SHEEPISH));
        }

        #[test]
        fn r78_it_recruits_even_when_it_died_in_that_combat() {
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [{ "def": BOOTS, "radiant": true }], "library": [TEMPO] },
                "p2": { "hand": [ANCHOR], "field": [MENACE] },
            }));

            s.attack(BOOTS, MENACE);

            s.expect_in_zone(BOOTS, "graveyard");
            assert_eq!(unit_defs(&s, P1), lanes([Some(TEMPO), None, None, None, None]));
        }

        #[test]
        fn r53_a_forced_attack_it_survives_recruits_too() {
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [{ "def": BOOTS, "radiant": true }], "library": [TEMPO, VANILLA] },
                "p2": { "hand": [ANCHOR], "field": [MOTHS], "library": [VANILLA] },
            }));

            s.end_turn();

            s.expect_in_zone(BOOTS, "field");
            assert_eq!(unit_defs(&s, P1), lanes([Some(BOOTS), Some(TEMPO), None, None, None]));
        }

        #[test]
        fn r386_an_upgrade_of_recruits_makes_two_scans() {
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [{ "def": BOOTS, "radiant": true }], "library": [TEMPO, STOCKPILE, VANILLA] },
                "p2": { "hand": [ANCHOR] },
            }));
            step_param(s.card_mut(BOOTS), "recruits", 1);

            s.attack(BOOTS, "hero");

            assert_eq!(unit_defs(&s, P1), lanes([Some(BOOTS), Some(TEMPO), Some(VANILLA), None, None]));
        }
    }
}
