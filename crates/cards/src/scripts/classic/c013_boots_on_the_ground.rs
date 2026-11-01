//! C #13 Boots on the Ground (SPEC §8.6 row 13). (1) Unit, Human, Common, 2/1 → 4/2.
//!   Base:    "Charge\nWhen this attacks, draw {draw}." — draw 1
//!   Radiant: "Charge\nWhen this attacks, Recruit {recruits|card|cards}." — recruits 1
//!
//! Charge is printed on both catalog faces, so §10.4 layer 1 grants it and nothing here does.
//!
//! "When this attacks" is the `whenAttacks` hook (R2285000, R2396000): the engine runs it before any
//! damage of each attack this card makes, a declared one's once its trap window has closed and a forced
//! one's before its combat (R53, #9 Moths to the Flame), so it draws or Recruits though it dies in that
//! combat. An attack called off in its trap window runs nothing (R44), and defending is not attacking.
//!
//! Radiant: §6.3 Recruit, the first permanent from the top of your deck, summoned into its row per R64
//! with no Cry (R1), a Trap face-down (R33); nothing when there is none or its row is full. The
//! numbers are the declared `draw` and `recruits` (R386), read through `param`.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-013";

/// "Draw {draw}."
fn draws() -> Hook {
    hook(|ctx| vec![draw(json_as(json!({ "count": param(&*ctx, "draw") })))])
}

/// "Recruit {recruits|card|cards}.": one top-down scan per recruit.
fn recruits() -> Hook {
    hook(|ctx| {
        (0..param(&*ctx, "recruits"))
            .map(|_| recruit(json_as(json!({ "player": "self" }))))
            .collect()
    })
}

pub fn script() -> CardScripts {
    // "When this attacks, draw {draw}."
    let base = Script {
        when_attacks: Some(draws()),
        ..Script::default()
    };
    // "When this attacks, Recruit {recruits|card|cards}."
    let radiant = Script {
        when_attacks: Some(recruits()),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

/// R767000: the scripts v0.5.11 and before ran: "After this attacks", only for a survivor (R2285001).
pub fn script_v0_5_11() -> CardScripts {
    let base = Script {
        after_attack: Some(draws()),
        ..Script::default()
    };
    let radiant = Script {
        after_attack: Some(recruits()),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// C #13 Boots on the Ground — SPEC §8.6 row 13, BUILD M9 Classic row C 13: draws 1 when it attacks,
// before the damage, forced attacks included, and so though it dies in that combat (R2285000,
// R2396000); an attack called off in its trap window (R44) or defending draws nothing. Radiant 4/2:
// Recruits instead, a recruited trap landing face-down (R33); numbers via `param()` (R386). Forced
// attacks come from #9 Moths to the Flame (start of turn: every enemy Unit attacks this).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::def_ids;
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
    const PAWN: &str = "core-096"; // Trap: cancels an attack lethal to its controller's hero (R44).
    const A: &str = "core-020";
    const B: &str = "core-001";

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

    fn has_charge(s: &Scenario, card: &str) -> bool {
        s.stats(card).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Charge)
    }

    /// R2285000, R2396000: whether the first event `hit` matches comes before the first damage, in the
    /// last action's events.
    fn before_the_damage(s: &Scenario, hit: impl Fn(&GameEvent) -> bool) -> bool {
        let events = s.events();
        let first = events.iter().position(hit);
        let damage = events.iter().position(|event| matches!(event, GameEvent::Damage { .. }));
        matches!((first, damage), (Some(first), Some(damage)) if first < damage)
    }

    /// R44: p2 at 2 health with My Pawn face-down, so an attack on its hero is lethal and called off.
    fn pawned(boots: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [VANILLA], "field": [boots], "library": [TEMPO, A] },
            "p2": {
                "hand": [ANCHOR],
                "library": [VANILLA, VANILLA],
                "health": 2,
                "backrow": [{ "def": PAWN, "faceUp": false }],
            },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn is_a_2_1_with_charge_played_this_turn_it_draws_1_as_it_attacks_and_then_hits_the_hero() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOTS, ANCHOR], "library": [A, B] }, "p2": { "hand": [ANCHOR] } }));

            s.play(BOOTS, json!({}));
            s.expect_stats(BOOTS, json!({ "attack": 2, "health": 1 }));
            s.attack(BOOTS, "hero");

            s.expect_health(P2, 28);
            assert_eq!(def_ids(&s.hand(P1)), [ANCHOR, A]);
            s.expect_events(json!(["attackDeclared", "drawn", "damage"]));
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
        fn r2396000_draws_before_the_damage() {
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [BOOTS], "library": [A, B] },
                "p2": { "hand": [ANCHOR], "field": [MENACE] },
            }));

            s.attack(BOOTS, MENACE);

            assert!(before_the_damage(&s, |event| matches!(event, GameEvent::Drawn { player: P1, .. })));
        }

        #[test]
        fn r2396000_draws_though_it_dies_in_that_combat() {
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [BOOTS], "library": [A, B] },
                "p2": { "hand": [ANCHOR], "field": [MENACE] },
            }));

            s.attack(BOOTS, MENACE);

            s.expect_in_zone(BOOTS, "graveyard");
            assert_eq!(def_ids(&s.hand(P1)), [ANCHOR, A]);
            assert_eq!(draws_by(&s, P1), 1);
        }

        #[test]
        fn r2396000_r53_a_forced_attack_into_moths_draws_though_it_dies() {
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [BOOTS], "library": [A, B] },
                "p2": { "hand": [ANCHOR], "field": [MOTHS], "library": [VANILLA] },
            }));

            s.end_turn();

            assert!(s.events().iter().any(|event| matches!(event, GameEvent::AttackDeclared { forced: true, .. })));
            s.expect_in_zone(BOOTS, "graveyard");
            assert_eq!(def_ids(&s.hand(P1)), [ANCHOR, A]);
            assert!(before_the_damage(&s, |event| matches!(event, GameEvent::Drawn { player: P1, .. })));
        }

        #[test]
        fn r2396000_r44_an_attack_my_pawn_calls_off_runs_neither_face() {
            for radiant in [false, true] {
                let mut s = pawned(json!({ "def": BOOTS, "radiant": radiant }));

                s.attack(BOOTS, "hero");

                s.expect_health(P2, 2);
                assert_eq!(draws_by(&s, P1), 0, "radiant: {radiant}");
                assert_eq!(def_ids(&s.pile(P1, "library")), [TEMPO, A], "radiant: {radiant}");
            }
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

            // It draws as it attacks, though it dies there (R2396000), and from the graveyard it answers
            // nothing.
            s.attack(BOOTS, MENACE);
            assert_eq!(draws_by(&s, P1), 1);
            s.expect_in_zone(BOOTS, "graveyard");
            s.attack(VANILLA, MENACE);

            assert_eq!(draws_by(&s, P1), 1);
            assert_eq!(def_ids(&s.hand(P1)), [ANCHOR, A]);
        }

        #[test]
        fn r386_an_upgrade_makes_it_draw_2() {
            let mut s = scenario(json!({ "p1": { "hand": [ANCHOR], "field": [BOOTS], "library": [A, B] }, "p2": { "hand": [ANCHOR] } }));
            step_param(s.card_mut(BOOTS), "draw", 1);

            s.attack(BOOTS, "hero");

            assert_eq!(def_ids(&s.hand(P1)), [ANCHOR, A, B]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_a_4_2_with_charge_that_when_it_attacks_recruits_the_first_permanent_from_the_top_of_your_deck_instead_of_drawing() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": BOOTS, "radiant": true }, ANCHOR], "library": [STOCKPILE, TEMPO, VANILLA] },
                "p2": { "hand": [ANCHOR] },
            }));

            s.play(BOOTS, json!({}));
            s.expect_stats(BOOTS, json!({ "attack": 4, "health": 2 }));
            s.attack(BOOTS, "hero");

            s.expect_health(P2, 26);
            assert_eq!(unit_defs(&s, P1), lanes([Some(BOOTS), Some(TEMPO), None, None, None]));
            assert_eq!(def_ids(&s.pile(P1, "library")), [STOCKPILE, VANILLA]);
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

            assert_eq!(def_ids(&s.pile(P1, "library")), [TEMPO]);
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
            assert_eq!(def_ids(&s.pile(P1, "library")), [STOCKPILE]);
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

            assert_eq!(def_ids(&s.pile(P1, "library")), [TEMPO]);
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
        fn r2396000_recruits_though_it_dies() {
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [{ "def": BOOTS, "radiant": true }], "library": [TEMPO] },
                "p2": { "hand": [ANCHOR], "field": [MENACE] },
            }));

            s.attack(BOOTS, MENACE);

            s.expect_in_zone(BOOTS, "graveyard");
            assert_eq!(unit_defs(&s, P1), lanes([None, Some(TEMPO), None, None, None]));
        }

        #[test]
        fn r2396000_r33_a_recruited_trap_lands_face_down_before_the_damage() {
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [{ "def": BOOTS, "radiant": true }], "library": [SHEEPISH] },
                "p2": { "hand": [ANCHOR], "field": [MENACE] },
            }));

            s.attack(BOOTS, MENACE);

            s.expect_in_zone(BOOTS, "graveyard");
            let trap = s.backrow(P1, 1).expect("the recruited trap");
            assert_eq!(trap.def_id, SHEEPISH);
            assert_ne!(trap.face_up, Some(true));
            // It landed before any damage of the attack: the first event that names it comes first.
            let named = format!("\"{}\"", trap.id);
            assert!(before_the_damage(&s, |event| {
                serde_json::to_string(event).is_ok_and(|text| text.contains(&named))
            }));
        }

        #[test]
        fn r53_a_forced_attack_recruits_too() {
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

    // R767000: a game on v0.5.11 folds under the script it ran, "After this attacks" for a survivor.
    mod v0_5_11 {
        use super::*;

        #[test]
        fn r767000_a_game_on_v0_5_11_that_dies_draws_nothing() {
            crate::under_kept(BOOTS, super::super::script_v0_5_11(), || {
                let mut s = scenario(json!({
                    "p1": { "hand": [ANCHOR], "field": [BOOTS], "library": [A, B] },
                    "p2": { "hand": [ANCHOR], "field": [MENACE] },
                }));

                s.attack(BOOTS, MENACE);

                s.expect_in_zone(BOOTS, "graveyard");
                assert_eq!(draws_by(&s, P1), 0);
            });
        }
    }
}
