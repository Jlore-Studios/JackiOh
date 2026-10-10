//! C #82 Sheeople (SPEC §8.6 row 82). (1) Unit, Common, 1/1 → 2/2.
//!   Base:    "Worth {worth|Tribute|Tributes}. Death: Draw {draw}." — worth 2, draw 2
//!   Radiant: the same text — worth 3, draw 3
//!   Engine:  "The Sheep Token's `tributeWorth` (§3.2, §7), 2 (Radiant 3), counted toward a Tribute X only
//!            (R101); a Tribute that takes one unit (C #21 Turtinator's) counts it once. A Tribute is a
//!            death (§6.2), so the draw happens when it is tributed. Tunes: draw 2 ↑; worth 2 ↑."
//!
//! The worth is the Sheep Token's static flag, which `playChoices.tributeValueOf` reads through the
//! declared `worth` (R386), toward a play's Tribute X only; a script's Tribute counts units. The Death
//! draw is read through `param` (R386).

use jackioh_engine::effects::draw;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-082";

fn worth() -> Param {
    match crate::card_def(ID)
        .params
        .as_ref()
        .and_then(|params| params.iter().find(|entry| entry.key == "worth"))
    {
        Some(param) => param.clone(),
        None => panic!("classic-082 declares its worth (catalog params)"),
    }
}

pub fn script() -> CardScripts {
    let worth = worth();

    let death: Hook = hook(|ctx| vec![draw(json_as(json!({ "count": param(&*ctx, "draw") })))]);

    let base = Script {
        static_flags: Some(StaticFlags {
            tribute_worth: Some(worth.base),
            ..StaticFlags::default()
        }),
        death: Some(death.clone()),
        ..Script::default()
    };

    let radiant = Script {
        static_flags: Some(StaticFlags {
            tribute_worth: Some(worth.radiant),
            ..StaticFlags::default()
        }),
        death: Some(death),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #82 Sheeople (SPEC §8.6 row 82): 1/1 worth 2 Tributes toward Tribute X only (§6.3),
// paying Tribute 2 and overpaying Tribute 1 (R101); Death: draw 2, so tributing draws;
// radiant 2/2 worth 3, draw 3; tuned numbers read through `param()` (R386).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;
    use std::sync::Arc;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SHEEOPLE: &str = "classic-082";
    const ROCK: &str = "core-066"; // (4) Unit 10/10: Tribute 1, Indestructible.
    const GOLEM: &str = "core-055"; // (3) Unit 10/5: Taunt, Tribute 3.
    const CUBE: &str = "core-022"; // (3) Unit: Cry: Tribute one of your other Units and remember it.
    const TURTINATOR: &str = "classic-021"; // (2) Unit 5/4: Activate ♾️: Tribute a Unit; deal damage equal to its attack.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy target Unit.
    const FLOOD: &str = "core-017"; // (4) Spell: Bounce all Units.
    const COLLATERAL: &str = "core-034"; // (4) Spell: Exile target permanent and a random card from your opponent's deck.
    const FILLER: &str = "core-005"; // (1) Spell (§2.5).
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).
    const X: &str = "core-020"; // library filler.

    use crate::scenario;

    fn def() -> CardDef {
        crate::register_all();
        crate::card_def(ID)
    }

    use crate::js;

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(found) => found,
            None => panic!("missing: {what}"),
        }
    }

    fn lib(n: usize) -> Vec<&'static str> {
        vec![X; n]
    }

    fn draws_by(events: &[GameEvent], player: PlayerId) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Drawn { player: p, .. } if *p == player))
            .count()
    }

    /// The Tribute sets legalActions offers for a play of `card`, each sorted, deduplicated.
    fn tribute_sets(s: &Scenario, card: &CardInstance) -> Vec<Vec<String>> {
        let sets: IndexSet<String> = legal_actions(s.state(), P1)
            .iter()
            .map(js)
            .filter(|action| action["type"] == json!("play") && action["instanceId"] == json!(card.id))
            .map(|play| {
                let mut ids: Vec<String> = play["tributes"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|id| id.as_str().map(String::from))
                    .collect();
                ids.sort();
                ids.join(",")
            })
            .collect();
        sets.into_iter()
            .map(|set| if set.is_empty() { vec![] } else { set.split(',').map(String::from).collect() })
            .collect()
    }

    /// The Rock, its Tribute X upgraded from 1 to 2.
    fn tribute2(s: &mut Scenario) -> CardInstance {
        let id = s.card(ROCK).id.clone();
        let rock = must(find_instance_mut(s.state_mut(), &id), "the Rock");
        let tuning = tuning_of(rock);
        tuning.x = Some(add_step(tuning.x.as_ref(), "Tribute", 1));
        s.card(&id).clone()
    }

    fn at(card: &CardInstance) -> Value {
        json!([{ "pick": "instance", "instanceId": card.id }])
    }

    /// declares its two numbers; the worth is the face's static flag, the draw its Death
    #[test]
    fn declares_its_two_numbers_the_worth_is_the_face_s_static_flag_the_draw_its_death() {
        assert_eq!(def().id, SHEEOPLE);
        assert_eq!(
            js(&def().params),
            json!([
                { "key": "draw", "base": 2, "radiant": 3, "better": "up", "step": 1, "min": 1 },
                { "key": "worth", "base": 2, "radiant": 3, "better": "up", "step": 1, "min": 1 }
            ])
        );
        let scripts = script();
        assert_eq!(js(&scripts.base.static_flags), json!({ "tributeWorth": 2 }));
        assert_eq!(js(&scripts.radiant.static_flags), json!({ "tributeWorth": 3 }));
        assert!(Arc::ptr_eq(
            scripts.base.death.as_ref().expect("a Death"),
            scripts.radiant.death.as_ref().expect("a Death")
        ));
    }

    mod base {
        use super::*;

        /// is a 1/1
        #[test]
        fn is_a_1_1() {
            scenario(json!({ "p1": { "field": [SHEEOPLE], "hand": [FILLER] } }))
                .expect_stats(SHEEOPLE, json!({ "attack": 1, "health": 1 }));
        }

        /// R101 worth 2: it alone pays a Tribute 2, and tributing it draws 2
        #[test]
        fn r101_worth_2_it_alone_pays_a_tribute_2_and_tributing_it_draws_2() {
            let mut s = scenario(json!({ "p1": { "hand": [ROCK, ANCHOR], "field": [SHEEOPLE], "library": lib(3) }, "p2": { "hand": [ANCHOR] } }));
            let rock = tribute2(&mut s);
            let sheeople = s.card(SHEEOPLE).clone();
            assert_eq!(tribute_sets(&s, &rock), vec![vec![sheeople.id.clone()]]);

            s.play(&rock, json!({ "tributes": [sheeople.id] }));

            s.expect_in_zone(&sheeople, "graveyard");
            s.expect_in_zone(&rock, "field");
            assert_eq!(draws_by(s.events(), P1), 2);
        }

        /// R101 it overpays a Tribute 1: alone it is a minimal set, never paired with another Unit
        #[test]
        fn r101_it_overpays_a_tribute_1_alone_it_is_a_minimal_set_never_paired_with_another_unit() {
            let mut s = scenario(json!({ "p1": { "hand": [ROCK, ANCHOR], "field": [SHEEOPLE, VANILLA], "library": lib(3) }, "p2": { "hand": [ANCHOR] } }));
            let sheeople = s.card(SHEEOPLE).clone();
            let vanilla = s.card(VANILLA).clone();

            let rock = s.card(ROCK).clone();
            let mut sets = tribute_sets(&s, &rock);
            sets.sort();
            let mut expected = vec![vec![sheeople.id.clone()], vec![vanilla.id.clone()]];
            expected.sort();
            assert_eq!(sets, expected);
            s.play(ROCK, json!({ "tributes": [sheeople.id] }));

            s.expect_in_zone(&sheeople, "graveyard");
            s.expect_in_zone(&vanilla, "field");
        }

        /// R101 it does not alone pay a Tribute 3; with one more Unit it does
        #[test]
        fn r101_it_does_not_alone_pay_a_tribute_3_with_one_more_unit_it_does() {
            let mut s = scenario(json!({ "p1": { "hand": [GOLEM, ANCHOR], "field": [SHEEOPLE, VANILLA], "library": lib(3) }, "p2": { "hand": [ANCHOR] } }));
            let sheeople = s.card(SHEEOPLE).clone();
            let vanilla = s.card(VANILLA).clone();

            let golem = s.card(GOLEM).clone();
            let mut pair = vec![sheeople.id.clone(), vanilla.id.clone()];
            pair.sort();
            assert_eq!(tribute_sets(&s, &golem), vec![pair]);
            s.expect_refused_with(|s| s.play(GOLEM, json!({ "tributes": [sheeople.id] })), "Tribute");
        }

        /// §6.3 a script's Tribute counts Units: a Carnivorous Cube's Cry takes it as one Unit, and it draws 2
        #[test]
        fn s6_3_a_script_s_tribute_counts_units_a_carnivorous_cube_s_cry_takes_it_as_one_unit_and_it_draws_2() {
            let mut s = scenario(json!({ "p1": { "hand": [CUBE, ANCHOR], "field": [SHEEOPLE], "library": lib(3) }, "p2": { "hand": [ANCHOR] } }));
            let sheeople = s.card(SHEEOPLE).clone();

            s.play(CUBE, json!({ "targets": at(&sheeople) }));

            s.expect_in_zone(&sheeople, "graveyard");
            assert_eq!(draws_by(s.events(), P1), 2);
        }

        /// §6.3 C #21 Turtinator's Tribute takes it as one Unit: 1 damage, its own attack, and it draws 2
        #[test]
        fn s6_3_c_n21_turtinator_s_tribute_takes_it_as_one_unit_1_damage_its_own_attack_and_it_draws_2() {
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [TURTINATOR, SHEEOPLE], "library": lib(3) },
                "p2": { "hand": [ANCHOR], "health": 20 }
            }));
            let sheeople = s.card(SHEEOPLE).clone();

            s.activate(TURTINATOR, json!({ "tributes": [sheeople.id], "targets": [{ "pick": "hero", "player": "p2" }] }));

            s.expect_in_zone(&sheeople, "graveyard");
            s.expect_health(P2, 19);
            assert_eq!(draws_by(s.events(), P1), 2);
        }

        /// §4.5 Death: destroyed, it draws its controller 2
        #[test]
        fn s4_5_death_destroyed_it_draws_its_controller_2() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [FILLER], "field": [SHEEOPLE], "library": lib(3) },
                "p2": { "hand": [HIT_JOB, ANCHOR] }
            }));

            let sheeople = s.card(SHEEOPLE).clone();
            s.play(HIT_JOB, json!({ "targets": at(&sheeople) }));

            assert_eq!(draws_by(s.events(), P1), 2);
            assert_eq!(draws_by(s.events(), P2), 0);
        }

        /// a bounce draws nothing
        #[test]
        fn a_bounce_draws_nothing() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [FILLER], "field": [SHEEOPLE], "library": lib(3) },
                "p2": { "hand": [FLOOD, ANCHOR] }
            }));

            s.play(FLOOD, json!({}));

            s.expect_in_zone(SHEEOPLE, "hand");
            assert_eq!(draws_by(s.events(), P1), 0);
        }

        /// an exile draws nothing
        #[test]
        fn an_exile_draws_nothing() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [FILLER], "field": [SHEEOPLE], "library": lib(3) },
                "p2": { "hand": [COLLATERAL, ANCHOR] }
            }));

            let sheeople = s.card(SHEEOPLE).clone();
            s.play(COLLATERAL, json!({ "targets": at(&sheeople) }));

            s.expect_in_zone(SHEEOPLE, "exile");
            assert_eq!(draws_by(s.events(), P1), 0);
        }

        /// §2.4 a full hand burns the draws; an empty deck makes them fatigue
        #[test]
        fn s2_4_a_full_hand_burns_the_draws_an_empty_deck_makes_them_fatigue() {
            let mut full = scenario(json!({
                "active": "p2",
                "p1": { "hand": vec![FILLER; 10], "field": [SHEEOPLE], "library": lib(3) },
                "p2": { "hand": [HIT_JOB, ANCHOR] }
            }));
            let target = full.card(SHEEOPLE).clone();
            full.play(HIT_JOB, json!({ "targets": at(&target) }));
            assert_eq!(full.events().iter().filter(|event| matches!(event, GameEvent::Burned { .. })).count(), 2);

            let mut empty = scenario(json!({
                "active": "p2",
                "p1": { "hand": [FILLER], "field": [SHEEOPLE] },
                "p2": { "hand": [HIT_JOB, ANCHOR] }
            }));
            let target = empty.card(SHEEOPLE).clone();
            empty.play(HIT_JOB, json!({ "targets": at(&target) }));
            assert_eq!(
                empty
                    .events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::Fatigue { player: PlayerId::P1, .. }))
                    .count(),
                2
            );
        }

        /// R386 an Upgrade of its draw draws 3
        #[test]
        fn r386_an_upgrade_of_its_draw_draws_3() {
            let mut s = scenario(json!({ "p1": { "hand": [ROCK, ANCHOR], "field": [SHEEOPLE], "library": lib(4) }, "p2": { "hand": [ANCHOR] } }));
            step_param(s.card_mut(SHEEOPLE), "draw", 1);

            let sheeople = s.card(SHEEOPLE).id.clone();
            s.play(ROCK, json!({ "tributes": [sheeople] }));

            assert_eq!(draws_by(s.events(), P1), 3);
        }

        /// R386 an Upgrade of its worth makes it worth 3: alone it pays a Tribute 3
        #[test]
        fn r386_an_upgrade_of_its_worth_makes_it_worth_3_alone_it_pays_a_tribute_3() {
            let mut s = scenario(json!({ "p1": { "hand": [GOLEM, ANCHOR], "field": [SHEEOPLE], "library": lib(3) }, "p2": { "hand": [ANCHOR] } }));
            let sheeople = s.card(SHEEOPLE).clone();
            step_param(s.card_mut(SHEEOPLE), "worth", 1);

            let golem = s.card(GOLEM).clone();
            assert_eq!(tribute_sets(&s, &golem), vec![vec![sheeople.id.clone()]]);
            s.play(GOLEM, json!({ "tributes": [sheeople.id] }));
            s.expect_in_zone(GOLEM, "field");
        }
    }

    mod radiant {
        use super::*;

        /// is a 2/2 worth 3: alone it pays a Tribute 3, and tributing it draws 3
        #[test]
        fn is_a_2_2_worth_3_alone_it_pays_a_tribute_3_and_tributing_it_draws_3() {
            let mut s = scenario(json!({
                "p1": { "hand": [GOLEM, ANCHOR], "field": [{ "def": SHEEOPLE, "radiant": true }], "library": lib(4) },
                "p2": { "hand": [ANCHOR] }
            }));
            let sheeople = s.card(SHEEOPLE).clone();
            s.expect_stats(&sheeople, json!({ "attack": 2, "health": 2 }));

            let golem = s.card(GOLEM).clone();
            assert_eq!(tribute_sets(&s, &golem), vec![vec![sheeople.id.clone()]]);
            s.play(GOLEM, json!({ "tributes": [sheeople.id] }));

            s.expect_in_zone(GOLEM, "field");
            assert_eq!(draws_by(s.events(), P1), 3);
        }

        /// a script's Tribute still counts it as one Unit, and a bounce draws nothing
        #[test]
        fn a_script_s_tribute_still_counts_it_as_one_unit_and_a_bounce_draws_nothing() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [CUBE, FLOOD, ANCHOR],
                    "field": [{ "def": SHEEOPLE, "radiant": true }, VANILLA],
                    "library": lib(4),
                    "mana": 10
                },
                "p2": { "hand": [ANCHOR] }
            }));

            let vanilla = s.card(VANILLA).clone();
            s.play(CUBE, json!({ "targets": at(&vanilla) }));
            s.play(FLOOD, json!({}));

            s.expect_in_zone(SHEEOPLE, "hand");
            assert_eq!(draws_by(s.events(), P1), 0);
        }

        /// R386 a Degrade of its draw draws 2
        #[test]
        fn r386_a_degrade_of_its_draw_draws_2() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [FILLER], "field": [{ "def": SHEEOPLE, "radiant": true }], "library": lib(4) },
                "p2": { "hand": [HIT_JOB, ANCHOR] }
            }));
            step_param(s.card_mut(SHEEOPLE), "draw", -1);

            let sheeople = s.card(SHEEOPLE).clone();
            s.play(HIT_JOB, json!({ "targets": at(&sheeople) }));

            assert_eq!(draws_by(s.events(), P1), 2);
        }
    }
}
