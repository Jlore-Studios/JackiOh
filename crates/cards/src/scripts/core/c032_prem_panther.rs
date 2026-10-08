//! #32 Prem Panther (SPEC §8.2 row 32): 5/4 "Rush / After this attacks, draw 2 for each Unit that
//! attack destroyed", radiant 10/8 "Rush, Cleave / the same" (balance patch 1: the draw happens even
//! when the Panther died in the combat; R426).
//!
//! Rush and Cleave are printed on the catalog faces, so §10.4 layer 1 grants them. The text is the
//! engine's `afterAttack` hook, which runs once the state check that closes each combat the Panther
//! attacked in has run, declared or forced (R53, R59), and hands it the combat's facts
//! (`afterAttackOf`): the Units whose lethal hit it dealt in that combat (its strike and its Cleave's;
//! a defender's strike back is the defender's, and a defending Panther runs no hook). Survival is no
//! longer asked (R212 still pins the stay: a Panther back through Reborn draws for the kill of its
//! last stay, and a defending one draws nothing). The hook acts for the player who controlled it in
//! that combat, though a Death in the check (#86 Mrow's) took it since.

use jackioh_engine::combat::after_attack_of;
use jackioh_engine::effects::draw;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-032";

/// TS `const afterAttack: Hook`. §8 row 32: "draw 2 for each Unit that attack destroyed", the 2 the
/// declared number `draw` (R386).
fn after_attack() -> Hook {
    hook(|ctx| {
        let Some(facts) = after_attack_of(ctx) else {
            return vec![];
        };
        if facts.destroyed_ids.is_empty() {
            return vec![];
        }
        vec![draw(json_as(json!({ "count": param(&*ctx, "draw") * facts.destroyed_ids.len() as i32 })))]
    })
}

pub fn script() -> CardScripts {
    let after_attack = after_attack();
    CardScripts {
        base: Script {
            after_attack: Some(after_attack.clone()),
            ..Script::default()
        },
        // "Rush, Cleave / the same": the keyword list is the radiant face's, the text is unchanged.
        radiant: Script {
            after_attack: Some(after_attack),
            ..Script::default()
        },
    }
}

// #32 Prem Panther — SPEC §8.2 row 32, BUILD M4-T4 must-pass row 32, patch v0.2.0 (R426) as
// amended by balance patch 1 (issue #88): "Rush / After this attacks, draw 2 for each Unit that
// attack destroyed"; radiant "Rush, Cleave / the same".
//
// R426 rewrites the old reading (R42's "whenever this destroys a unit", on either side of a combat):
// it draws only after an attack it made — declared or forced (R53) — 2 for each Unit that attack
// destroyed (the Unit it attacked, plus the Cleave kills of the Radiant face). It never draws for a
// Unit it kills defending. Survival is no longer asked: the hook is owed on the snapshot the Panther
// fought with, so it draws even when it dies in the combat, even one back through Reborn. R42 still
// says who killed a Unit, per Unit.
//
// The sparring partners: #15 Me and Mr Token is a 1/1 with no keywords (its Cry does not fire from a
// `field` setup), #13 Jlockeed Shredder-10 is an 8/10 with no keywords, so it kills a 5/4 Panther on
// the swing back and survives, #11 Tempo Timmy is a 3/3 Rush, First Strike that attacks a Panther
// and dies to its strike back, and #9 Moths to the Flame (1/14) makes every enemy Unit attack it at
// its controller's start of turn.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    /// The harness's import-time `registerAll()`: the engine's testkit cannot name the cards crate.
    fn scn(opts: Value) -> Scenario {
        crate::register_all();
        scenario(opts)
    }

    fn must(card: Option<CardInstance>, what: &str) -> CardInstance {
        card.unwrap_or_else(|| panic!("the scenario has no {what}"))
    }

    /// TS `s.events.find((event) => event.type === "destroyed" && event.instanceId === id)`, as JSON
    /// so `toMatchObject` reads its keys.
    fn destroyed_event(s: &Scenario, instance_id: &str) -> Value {
        s.events()
            .iter()
            .map(|event| serde_json::to_value(event).expect("an event is JSON"))
            .find(|event| event["type"] == "destroyed" && event["instanceId"] == instance_id)
            .unwrap_or_else(|| panic!("no destroyed event for {instance_id}"))
    }

    fn drew(events: &[GameEvent]) -> bool {
        events.iter().any(|event| matches!(event, GameEvent::Drawn { .. }))
    }

    mod base {
        use super::*;

        #[test]
        fn sec6_1_rush_a_panther_played_this_turn_may_attack_a_unit_but_not_the_hero() {
            let mut s = scn(json!({
                "seed": "panther-rush",
                "p1": { "hand": ["32"], "field": ["15"], "library": ["15", "15", "15"] },
                "p2": { "field": ["15", "15"] },
            }));
            s.play("32", json!({}));

            s.expect_refused_with(|s| s.attack("32", "hero"), "Rush cannot hit the hero");
            let target = must(s.unit("p2", 1), "p2 lane 1");
            s.attack("32", &target);
            let panther = must(s.unit("p1", 2), "the Panther");
            s.expect_in_zone(&panther, "field");
        }

        #[test]
        fn r426_draws_2_after_it_attacks_for_the_unit_that_attack_destroyed() {
            let mut s = scn(json!({
                "seed": "panther-kill",
                "p1": { "field": ["32", "15"], "library": ["15", "15", "15"] },
                "p2": { "field": ["15"] },
            }));
            let prey = must(s.unit("p2", 1), "p2 lane 1");
            let panther = must(s.unit("p1", 1), "the Panther");
            assert_eq!(s.hand("p1").len(), 0);

            s.attack(&panther, &prey);

            s.expect_in_zone(&prey, "graveyard");
            s.expect_in_zone(&panther, "field");
            assert_eq!(s.hand("p1").len(), 2);
            assert_eq!(s.pile("p1", "library").len(), 1);
            s.expect_events(json!(["attackDeclared", "damage", "destroyed", "drawn", "drawn"]));
            // The death names the Panther as its killer (R42).
            let death = destroyed_event(&s, &prey.id);
            assert_eq!(death["killerId"], json!(panther.id));
        }

        #[test]
        fn r426_draws_2_when_it_dies_in_that_combat_for_the_unit_it_destroyed_in_the_trade() {
            let mut s = scn(json!({
                "seed": "panther-trade",
                "p1": { "field": ["32", "15"], "library": ["15", "15", "15"] },
                "p2": { "field": ["32"] },
            }));
            let panther = must(s.unit("p1", 1), "p1's Panther");
            let other = must(s.unit("p2", 1), "p2's Panther");

            // 5 into a 5/4 and 5 back into a 5/4: both die. The hook is owed on the snapshot the Panther
            // fought with, so it still draws 2 for the Unit its attack destroyed (balance patch 1).
            s.attack(&panther, &other);

            s.expect_in_zone(&panther, "graveyard");
            s.expect_in_zone(&other, "graveyard");
            assert_eq!(s.hand("p1").len(), 2);
            assert_eq!(s.pile("p1", "library").len(), 1);
            assert!(drew(s.last_events()));
            // The death names the Panther as its killer (R42).
            let death = destroyed_event(&s, &other.id);
            assert_eq!(death["killerId"], json!(panther.id));
        }

        #[test]
        fn r426_draws_nothing_when_it_dies_without_killing() {
            let mut s = scn(json!({
                "seed": "panther-trade",
                "p1": { "field": ["32"], "library": ["15", "15", "15"] },
                "p2": { "field": ["13"] },
            }));
            let wall = must(s.unit("p2", 1), "p2 lane 1");
            let panther = must(s.unit("p1", 1), "p1 lane 1");

            s.attack(&panther, &wall);

            // 5 into an 8/10 is not lethal; 8 back into a 5/4 is.
            s.expect_in_zone(&panther, "graveyard");
            s.expect_in_zone(&wall, "field");
            assert_eq!(s.hand("p1").len(), 0);
            assert_eq!(s.pile("p1", "library").len(), 3);
        }

        #[test]
        fn r426_never_draws_while_defending_a_unit_it_kills_striking_back_is_the_attackers_attack() {
            let mut s = scn(json!({
                "seed": "panther-defends",
                "active": "p2",
                "turn": 10,
                "p1": { "field": ["32"], "library": ["15", "15", "15"] },
                "p2": { "field": ["11"], "hand": ["15"], "library": ["15", "15"] },
            }));
            let panther = must(s.unit("p1", 1), "p1's Panther");
            let timmy = must(s.unit("p2", 1), "p2's Tempo Timmy");

            // First Strike 3 leaves the 5/4 Panther at 1; its 5 back kills the 3/3.
            s.attack(&timmy, &panther);

            s.expect_in_zone(&timmy, "graveyard");
            s.expect_in_zone(&panther, "field");
            let death = destroyed_event(&s, &timmy.id);
            // R42: the Panther killed it — but in the other player's attack, which runs no hook of its (R426).
            assert_eq!(death["killerId"], json!(panther.id));
            assert_eq!(s.hand("p1").len(), 0);
            assert_eq!(s.pile("p1", "library").len(), 3);
        }

        #[test]
        fn r426_r53_a_forced_attack_is_an_attack_forced_into_9_moths_to_the_flame_it_kills_it_survives_and_draws_2()
        {
            // p2's start of turn: Moths makes every enemy Unit attack it. Its 14 health, 10 of it already
            // gone, falls to the Panther's 5; its 1 back leaves the Panther standing.
            let mut s = scn(json!({
                "seed": "panther-forced",
                "p1": { "field": ["32"], "hand": ["15"], "library": ["15", "15", "15"] },
                "p2": { "field": [{ "def": "9", "damage": 10 }], "hand": ["15"], "library": ["15", "15", "15"] },
            }));
            let panther = must(s.unit("p1", 1), "p1's Panther");
            let moths = must(s.unit("p2", 1), "p2's Moths");

            s.end_turn(); // p2's turn begins: the forced attack, on p2's turn.

            assert_eq!(s.state().active, PlayerId::P2);
            s.expect_in_zone(&moths, "graveyard");
            s.expect_in_zone(&panther, "field");
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::AttackDeclared { attacker_id, forced: true, .. } if *attacker_id == panther.id
            )));
            // p1 drew 2 on p2's turn, for the Unit its forced attack destroyed.
            assert_eq!(s.hand("p1").len(), 3);
            assert_eq!(s.pile("p1", "library").len(), 1);
        }

        #[test]
        fn r426_r212_a_panther_stolen_by_the_death_of_the_unit_it_destroyed_still_draws_for_the_player_who_attacked_with_it_86()
        {
            // #86 "Miss" Mrow: "Death: Take control of the Unit that destroyed this." The Panther survives the
            // combat on the stay it attacked from (a change of control is not leaving the field, R171), so
            // the draws are the ones that attack earned — for p1, who controlled it in that combat (R212).
            let mut s = scn(json!({
                "seed": "panther-mrow",
                "p1": { "field": ["32"], "hand": ["5"], "library": ["15", "15", "15"] },
                "p2": { "field": ["86"], "hand": ["15"], "library": ["15", "15"] },
            }));
            let panther = must(s.unit("p1", 1), "p1's Panther");
            let mrow = must(s.unit("p2", 1), "p2's Mrow");

            s.attack(&panther, &mrow);

            s.expect_in_zone(&mrow, "graveyard");
            assert_eq!(s.card(&panther).controller, PlayerId::P2);
            assert_eq!(s.hand("p1").len(), 3);
            assert_eq!(s.hand("p2").len(), 1);
        }

        #[test]
        fn r426_r83_a_panther_that_dies_in_the_combat_and_comes_back_through_reborn_still_draws_2_the_hook_is_owed_on_the_snapshot_it_fought_with_not_answered_by_the_new_stay()
        {
            let mut s = scn(json!({
                "seed": "panther-reborn",
                "p1": { "field": ["32"], "hand": ["5"], "library": ["15", "15", "15"] },
                "p2": { "field": ["32"], "hand": ["15"] },
            }));
            let panther = must(s.unit("p1", 1), "p1's Panther");
            find_instance_mut(s.state_mut(), &panther.id)
                .expect("the Panther is in the state")
                .granted_keywords
                .push(json_as(json!({ "kind": "Reborn" })));

            // 5 into p2's 5/4 Panther and 5 back: both die, and p1's comes back at 1 health. The Reborn body
            // is a new arrival (R83, R174) that destroyed nothing — but the draw is the dead stay's hook,
            // owed on the snapshot (R426), so p1 still draws 2 for the kill of its last stay.
            let other = must(s.unit("p2", 1), "p2's Panther");
            s.attack(&panther, &other);

            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::Destroyed { instance_id, .. } if *instance_id == panther.id
            )));
            s.expect_in_zone(&panther, "field");
            assert!(drew(s.last_events()));
            assert_eq!(s.hand("p1").len(), 3);
            assert_eq!(s.pile("p1", "library").len(), 1);
        }

        #[test]
        fn r426_attacking_the_hero_destroys_no_unit_and_draws_nothing() {
            let mut s = scn(json!({
                "seed": "panther-face",
                "p1": { "field": ["32"], "hand": ["5"], "library": ["15", "15", "15"] },
                "p2": { "hand": ["15"], "library": ["15"] },
            }));
            s.attack("32", "hero");
            s.expect_health("p2", 25);
            assert!(!drew(s.last_events()));
            assert_eq!(s.hand("p1").len(), 1);
        }

        #[test]
        fn r42_r426_is_per_unit_and_a_base_panther_has_no_cleave_so_only_the_defender_dies() {
            let mut s = scn(json!({
                "seed": "panther-no-cleave",
                "p1": { "field": ["32"], "library": ["15", "15", "15", "15", "15", "15", "15"] },
                "p2": { "field": ["15", "15", "15"] },
            }));
            let middle = must(s.unit("p2", 2), "p2 lane 2");

            s.attack("32", &middle);

            s.expect_in_zone(&middle, "graveyard");
            let left = must(s.unit("p2", 1), "p2 lane 1");
            s.expect_in_zone(&left, "field");
            let right = must(s.unit("p2", 3), "p2 lane 3");
            s.expect_in_zone(&right, "field");
            assert_eq!(s.hand("p1").len(), 2);
        }

        #[test]
        fn r426_an_attack_by_another_unit_draws_nothing() {
            let mut s = scn(json!({
                "seed": "panther-someone-else",
                "p1": { "field": ["32", "13"], "library": ["15", "15", "15"] },
                "p2": { "field": ["15"] },
            }));
            let prey = must(s.unit("p2", 1), "p2 lane 1");

            // The Shredder swings, not the Panther.
            let shredder = must(s.unit("p1", 2), "p1 lane 2");
            s.attack(&shredder, &prey);

            s.expect_in_zone(&prey, "graveyard");
            assert_eq!(s.hand("p1").len(), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r426_radiant_cleave_draws_2_per_unit_that_attack_destroyed_three_kills_draw_6() {
            let mut s = scn(json!({
                "seed": "panther-cleave",
                "p1": {
                    "field": [{ "def": "32", "radiant": true }],
                    "library": ["15", "15", "15", "15", "15", "15", "15", "15"],
                },
                "p2": { "field": ["15", "15", "15"] },
            }));
            let left = must(s.unit("p2", 1), "p2 lane 1");
            let middle = must(s.unit("p2", 2), "p2 lane 2");
            let right = must(s.unit("p2", 3), "p2 lane 3");

            s.attack("32", &middle);

            // §4.4 step 10: Cleave hits the same-side neighbours as separate instances of 10.
            s.expect_in_zone(&left, "graveyard");
            s.expect_in_zone(&middle, "graveyard");
            s.expect_in_zone(&right, "graveyard");
            assert_eq!(s.hand("p1").len(), 6);
            assert_eq!(s.pile("p1", "library").len(), 2);
        }

        #[test]
        fn r426_radiant_cleave_kills_draw_6_even_when_the_panther_dies_in_that_combat() {
            let mut s = scn(json!({
                "seed": "panther-cleave-dies",
                "p1": {
                    "field": [{ "def": "32", "radiant": true }],
                    "hand": ["5"],
                    "library": ["15", "15", "15", "15", "15", "15", "15", "15"],
                },
                "p2": { "field": ["15", "13", "15"], "hand": ["15"] },
            }));
            let panther = must(s.unit("p1", 1), "the radiant Panther");

            // 10 kills the 8/10 and cleaves both 1/1s; 8 back kills the 10/8. Three kills the attack
            // destroyed, so p1 draws 6 though the Panther died (balance patch 1).
            let shredder = must(s.unit("p2", 2), "p2's Shredder");
            s.attack(&panther, &shredder);

            s.expect_in_zone(&panther, "graveyard");
            for lane in 1..=3 {
                assert!(s.unit("p2", lane).is_none(), "p2 lane {lane} should be empty");
            }
            assert!(drew(s.last_events()));
            assert_eq!(s.hand("p1").len(), 7);
            assert_eq!(s.pile("p1", "library").len(), 2);
        }

        #[test]
        fn sec3_1_cleave_never_crosses_sides_so_the_panthers_own_neighbours_are_not_kills() {
            let mut s = scn(json!({
                "seed": "panther-cleave-sides",
                "p1": {
                    "field": [
                        { "def": "32", "radiant": true, "lane": 2 },
                        { "def": "15", "lane": 1 },
                        { "def": "15", "lane": 3 },
                    ],
                    "library": ["15", "15", "15", "15", "15"],
                },
                "p2": { "field": [{ "def": "15", "lane": 2 }] },
            }));
            let ally1 = must(s.unit("p1", 1), "p1 lane 1");
            let ally3 = must(s.unit("p1", 3), "p1 lane 3");

            let panther = must(s.unit("p1", 2), "the Panther");
            let prey = must(s.unit("p2", 2), "p2 lane 2");
            s.attack(&panther, &prey);

            s.expect_in_zone(&ally1, "field");
            s.expect_in_zone(&ally3, "field");
            assert_eq!(s.hand("p1").len(), 2);
        }

        #[test]
        fn radiant_is_10_8_and_its_text_is_unchanged_same_so_one_kill_still_draws_2() {
            let mut s = scn(json!({
                "seed": "panther-radiant-same",
                "p1": { "field": [{ "def": "32", "radiant": true }], "library": ["15", "15", "15"] },
                "p2": { "field": ["43"] },
            }));
            let panther = must(s.unit("p1", 1), "p1 lane 1");
            s.expect_stats(&panther, json!({ "attack": 10, "maxHealth": 8 }));

            // 10 into #43's 3/10 is lethal; 3 back into a 10/8 is not.
            let prey = must(s.unit("p2", 1), "p2 lane 1");
            s.attack(&panther, &prey);

            s.expect_in_zone(&panther, "field");
            assert_eq!(s.hand("p1").len(), 2);
        }
    }

    #[test]
    fn r386_an_upgrade_draws_3_for_the_kill_and_a_degrade_1() {
        for (upgrade, draws) in [(true, 3), (false, 1)] {
            let mut s = scn(json!({
                "seed": "panther-kill",
                "p1": { "field": ["32", "15"], "library": ["15", "15", "15", "15"] },
                "p2": { "field": ["15"] },
            }));
            let prey = must(s.unit("p2", 1), "p2 lane 1");
            let panther = must(s.unit("p1", 1), "the Panther");
            let moved = if upgrade {
                crate::upgrade_number(&mut s, &panther.id, "draw")
            } else {
                crate::degrade_number(&mut s, &panther.id, "draw")
            };
            assert_eq!(moved, draws);

            s.attack(&panther, &prey);

            assert_eq!(s.hand("p1").len(), draws as usize);
        }
    }
}
