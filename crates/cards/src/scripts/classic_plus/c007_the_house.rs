//! C+ #7 The House (SPEC §8.7 row 7, R406, R1704002). (3) Field Spell, Plague, Rare.
//!   Base:    "Cry: Place {tokens|Plague Counter|Plague Counters} on this.
//!             Activate: Spend a Plague Counter from this to summon a Right-house defender or a
//!             Wrong-House Attacker."
//!   Radiant: "… to summon a Radiant Right-house defender or a Radiant Wrong-House Attacker."
//!
//! The Cry is one placement of 3 on itself (§6.3 Plague Counter, R689; R471's multiplier applies). The
//! Activate is once per turn (§6.2, R384), its twin the controller's choice, a mode declared in the
//! `activate` action (R81): "defender" summons a Right-house defender (Core #3, R406), "attacker" a
//! Wrong-House Attacker (C+ #6), fresh cards, yours, no Cry (R1), R64's placement, on their Radiant faces
//! on the Radiant face. With no counter on it, or no unit zone free, it cannot be activated (R1704002).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-007";

/// R406: "Right-House Protector" is Core #3, the Wrong-House Attacker's twin.
const RIGHT_HOUSE: &str = "core-003";
const WRONG_HOUSE: &str = "classicplus-006";
/// R406: "(2 in 3)" — the odds of the Right-house defender, through v0.4.0f.
const RIGHT_HOUSE_ODDS: f64 = 2.0 / 3.0;
/// R1704002: the Activate's modes, the Right-house defender's and the Wrong-House Attacker's.
const DEFENDER: &str = "defender";
const ATTACKER: &str = "attacker";

/// R1704002: "Activate: Spend a Plague Counter from this to summon a Right-house defender or a
/// Wrong-House Attacker", the twin named by the mode the action declares.
fn house(radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: "house".to_string(),
        label: "Spend a Plague Counter: summon a Right-house defender or a Wrong-House Attacker".to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: Vec::new(),
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: vec![DEFENDER.to_string(), ATTACKER.to_string()],
        }],
        can_activate: Some(condition_hook(|c| {
            plague_on(c.self_) >= 1 && first_free_zone(c.state, c.controller, Row::Units).is_some()
        })),
        has: None,
        run: hook(move |ctx| {
            let def_id = match ctx.modes.first().map(String::as_str) {
                Some(DEFENDER) => RIGHT_HOUSE,
                Some(ATTACKER) => WRONG_HOUSE,
                _ => return vec![],
            };
            let mut args = json!({ "defId": def_id });
            if radiant {
                args["radiant"] = json!(true);
            }
            vec![consume_plague(json_as(json!({ "amount": 1 }))), summon(json_as(args))]
        }),
    }
}

fn the_house(radiant: bool) -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![place_plague(json_as(json!({ "amount": param(&*ctx, "tokens") })))]
        })),
        activations: vec![house(radiant)],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: the_house(false),
        radiant: the_house(true),
    }
}

fn summon_one() -> Hook {
    hook(|ctx| {
        if first_free_zone(ctx.state, ctx.controller, Row::Units).is_none() {
            vec![]
        } else {
            let def_id = if ctx.rng.chance(RIGHT_HOUSE_ODDS) { RIGHT_HOUSE } else { WRONG_HOUSE };
            vec![summon(json_as(json!({ "defId": def_id })))]
        }
    })
}

fn summon_both() -> Hook {
    hook(|_ctx| {
        vec![summon(json_as(json!({ "defId": RIGHT_HOUSE }))), summon(json_as(json!({ "defId": WRONG_HOUSE })))]
    })
}

/// R767000: the scripts v0.4.0f and every version before it ran, the Cry and start-of-turn summons
/// (R406); `KEPT_SCRIPTS` folds those versions' replays under them.
pub fn script_v0_4_0f() -> CardScripts {
    let one = summon_one();
    let both = summon_both();
    CardScripts {
        base: Script {
            cry: Some(one.clone()),
            start_of_turn: Some(one),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(both.clone()),
            start_of_turn: Some(both),
            ..Script::default()
        },
    }
}

// C+ #7 The House — SPEC §8.7 row 7, BUILD M9 Classic+ row C+ 7.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const HOUSE: &str = "classicplus-007";
    const RIGHT: &str = "core-003"; // Right-house defender
    const WRONG: &str = "classicplus-006"; // Wrong-House Attacker
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const HIT_JOB: &str = "core-016"; // (3) Destroy target Unit.
    const FILLER: &str = "core-010";
    const STOCKPILE: &str = "core-005";

    use crate::merged;

    fn setup(p1: Value, radiant_face: bool, seed: Option<&str>) -> Scenario {
        crate::register_all();
        let mut options = json!({
            "p1": merged(
                json!({
                    "hand": [{ "def": HOUSE, "radiant": radiant_face }, FILLER],
                    "library": [STOCKPILE, STOCKPILE, STOCKPILE],
                    "mana": 8,
                }),
                p1,
            ),
            "p2": { "hand": [FILLER], "library": [STOCKPILE, STOCKPILE, STOCKPILE] },
        });
        if let Some(seed) = seed {
            options["seed"] = json!(seed);
        }
        scenario(options)
    }

    /// The Units summoned, in order — not The House's own arrival in its backrow zone.
    fn summoned(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { def_id, .. } if def_id != HOUSE => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn counters(s: &Scenario) -> i32 {
        plague_on(s.card(HOUSE))
    }

    /// The House played into backrow zone 1, then a turn each, so its controller may activate it again.
    fn next_turn(s: &mut Scenario) {
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P1);
    }

    mod base {
        use super::*;

        #[test]
        fn costs_3_refused_with_2_mana() {
            let mut s = setup(json!({}), false, None);
            crate::expect_costs(&mut s, HOUSE, 3, json!({ "zone": 1 }));
        }

        #[test]
        fn r1704002_cry_places_3_counters() {
            let mut s = setup(json!({}), false, None);
            s.play(HOUSE, json!({ "zone": 1 }));
            assert_eq!(counters(&s), 3);
            assert_eq!(summoned(&s), Vec::<String>::new());
            // Nothing at the start of a turn.
            next_turn(&mut s);
            assert_eq!(summoned(&s), Vec::<String>::new());
            assert_eq!(counters(&s), 3);
        }

        #[test]
        fn r1704002_activate_spends_one_summons_the_named_twin_once_a_turn() {
            let mut s = setup(json!({}), false, None);
            s.play(HOUSE, json!({ "zone": 1 }));
            let cursor = s.state().rng_cursor;
            s.activate(HOUSE, json!({ "modes": ["defender"] }));
            assert_eq!(summoned(&s), vec![RIGHT]);
            assert_eq!(counters(&s), 2);
            // The controller's choice, no roll.
            assert_eq!(s.state().rng_cursor, cursor);
            let unit = s.unit(P1, 1).expect("a twin in lane 1");
            assert_eq!(
                (unit.def_id.clone(), unit.owner, unit.controller, unit.radiant),
                (RIGHT.to_string(), P1, P1, false)
            );
            s.expect_refused_with(
                |s| s.activate(HOUSE, json!({ "modes": ["attacker"] })),
                "already been used",
            );
            next_turn(&mut s);
            s.activate(HOUSE, json!({ "modes": ["attacker"] }));
            assert_eq!(summoned(&s), vec![RIGHT, WRONG]);
            assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id), Some(WRONG.to_string()));
            assert_eq!(counters(&s), 1);
            s.expect_refused(|s| s.activate(HOUSE, json!({ "modes": [] })));
        }

        #[test]
        fn r1704002_refused_with_no_counter_or_full_board() {
            let mut s = setup(json!({}), false, None);
            s.play(HOUSE, json!({ "zone": 1 }));
            for mode in ["defender", "attacker", "defender"] {
                s.activate(HOUSE, json!({ "modes": [mode] }));
                next_turn(&mut s);
            }
            assert_eq!(counters(&s), 0);
            s.expect_refused_with(
                |s| s.activate(HOUSE, json!({ "modes": ["attacker"] })),
                "can't be activated now",
            );

            let mut full = setup(json!({ "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA] }), false, None);
            full.play(HOUSE, json!({ "zone": 1 }));
            assert_eq!(counters(&full), 3);
            full.expect_refused_with(
                |s| s.activate(HOUSE, json!({ "modes": ["defender"] })),
                "can't be activated now",
            );
            assert!(
                !legal_actions(full.state(), P1)
                    .iter()
                    .any(|action| matches!(action, ActionBody::Activate { .. }))
            );
        }

        #[test]
        fn r11_the_twins_are_cards_not_tokens_a_wrong_house_attacker_it_made_goes_to_your_graveyard_when_it_dies() {
            let mut s = setup(json!({ "hand": [{ "def": HOUSE }, HIT_JOB, FILLER] }), false, None);
            s.play(HOUSE, json!({ "zone": 1 }));
            s.activate(HOUSE, json!({ "modes": ["attacker"] }));
            let made = s.unit(P1, 1).expect("a twin in lane 1");
            assert_eq!(made.def_id, WRONG);

            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": made.id }] }));

            s.expect_in_zone(&made, "graveyard");
            assert!(s.pile(P1, "graveyard").iter().any(|card| card.def_id == WRONG));
        }

        #[test]
        fn r64_lands_in_the_leftmost_open_unit_zone() {
            let mut s = setup(json!({ "field": [VANILLA, { "def": VANILLA, "lane": 3 }] }), false, None);
            s.play(HOUSE, json!({ "zone": 1 }));
            s.activate(HOUSE, json!({ "modes": ["defender"] }));
            assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id), Some(RIGHT.to_string()));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn costs_3_refused_with_2_mana() {
            let mut s = setup(json!({}), true, None);
            crate::expect_costs(&mut s, HOUSE, 3, json!({ "zone": 1 }));
        }

        #[test]
        fn r1704002_cry_places_3_counters() {
            let mut s = setup(json!({}), true, None);
            s.play(HOUSE, json!({ "zone": 1 }));
            assert_eq!(counters(&s), 3);
            assert_eq!(summoned(&s), Vec::<String>::new());
        }

        #[test]
        fn r1704002_summons_a_radiant_twin() {
            let mut s = setup(json!({}), true, None);
            s.play(HOUSE, json!({ "zone": 1 }));
            s.activate(HOUSE, json!({ "modes": ["defender"] }));
            next_turn(&mut s);
            s.activate(HOUSE, json!({ "modes": ["attacker"] }));
            assert_eq!(summoned(&s), vec![RIGHT, WRONG]);
            assert_eq!(
                [1, 2]
                    .iter()
                    .map(|&lane| s.unit(P1, lane).map(|unit| (unit.def_id, unit.radiant, unit.owner)))
                    .collect::<Vec<_>>(),
                vec![
                    Some((RIGHT.to_string(), true, P1)),
                    Some((WRONG.to_string(), true, P1))
                ]
            );
            assert_eq!(counters(&s), 1);
        }
    }

    /// R767000: the scripts kept for the replays of v0.4.0f and before: the Cry and start-of-turn summons.
    mod v0_4_0f {
        use super::*;

        fn kept(run: impl FnOnce()) {
            crate::under_kept(HOUSE, super::super::script_v0_4_0f(), run);
        }

        /// The defs The House summoned with its Cry, under a seed.
        fn cry_roll(seed: &str) -> Vec<String> {
            let mut s = setup(json!({}), false, Some(seed));
            s.play(HOUSE, json!({ "zone": 1 }));
            summoned(&s)
        }

        #[test]
        fn r406_its_cry_summons_one_of_the_twins_yours_with_no_cry_a_seeded_roll_gives_a_fixed_unit() {
            kept(|| {
                let mut s = setup(json!({}), false, None);
                s.play(HOUSE, json!({ "zone": 1 }));

                let defs = summoned(&s);
                assert_eq!(defs.len(), 1);
                assert!([RIGHT, WRONG].contains(&defs[0].as_str()));
                let unit = s.unit(P1, 1).expect("a twin in lane 1");
                assert_eq!(
                    (unit.def_id.clone(), unit.owner, unit.controller, unit.radiant),
                    (defs[0].clone(), P1, P1, false)
                );
                assert_eq!(cry_roll("jackioh-harness"), defs);
            });
        }

        #[test]
        fn r406_2_in_3_is_the_right_house_defender_s_chance_both_come_up_the_defender_about_twice_as_often() {
            kept(|| {
                let mut right = 0;
                let mut wrong = 0;
                for seed in 1..=90 {
                    let made = cry_roll(&format!("house-{seed}"));
                    match made.first().map(String::as_str) {
                        Some(RIGHT) => right += 1,
                        Some(WRONG) => wrong += 1,
                        _ => {}
                    }
                }
                assert_eq!(right + wrong, 90);
                assert!(right > 45);
                assert!(right < 75);
            });
        }

        #[test]
        fn s2_2_summons_again_at_the_start_of_your_turn_and_nothing_at_the_opponent_s() {
            kept(|| {
                let mut s = setup(json!({}), false, None);
                s.play(HOUSE, json!({ "zone": 1 }));
                assert_eq!(summoned(&s).len(), 1);

                s.end_turn();
                assert_eq!(s.state().active, P2);
                assert_eq!(summoned(&s).len(), 1);

                s.end_turn();
                assert_eq!(s.state().active, P1);
                assert_eq!(summoned(&s).len(), 2);
            });
        }

        #[test]
        fn r129_a_full_board_summons_nothing_and_rolls_nothing() {
            kept(|| {
                let mut s = setup(json!({ "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA] }), false, None);
                let cursor = s.state().rng_cursor;
                s.play(HOUSE, json!({ "zone": 1 }));
                assert_eq!(summoned(&s), Vec::<String>::new());
                assert_eq!(s.state().rng_cursor, cursor);
            });
        }

        #[test]
        fn r406_radiant_its_cry_and_each_start_of_your_turn_summon_one_of_each_on_their_base_faces() {
            kept(|| {
                let mut s = setup(json!({}), true, None);
                s.play(HOUSE, json!({ "zone": 1 }));
                assert_eq!(summoned(&s), vec![RIGHT, WRONG]);
                assert_eq!(
                    [1, 2]
                        .iter()
                        .map(|&lane| s.unit(P1, lane).map(|unit| (unit.radiant, unit.owner)))
                        .collect::<Vec<_>>(),
                    vec![Some((false, P1)), Some((false, P1))]
                );
                s.end_turn();
                s.end_turn();
                assert_eq!(summoned(&s), vec![RIGHT, WRONG, RIGHT, WRONG]);
            });
        }
    }
}
