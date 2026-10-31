//! SPEC §8.11 Training Grounds 2 Personal Trainer: (1) Unit, Human, Common, 3/2 → 6/4, a card of the held
//! Training Grounds set (R1420, R2194000).
//! Base:    "Cry: Give one of your other Humans +2/+2."
//! Radiant: "Cry: Give one of your other Humans +4/+4."
//! Friendly Human target (granted tag counts, R923; Help aim, R656; §10.4 layer 4 buff).
//! Pick made in hand (R81, R90; R2196000 cannot pick itself); with no target Cry fizzles and unit enters (R90; C+ #57).

use jackioh_engine::effects::buff;
use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-002";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Help),
        ..TargetDecl::target(1, 1, json!({ "side": "ally", "of": ["unit"], "tags": ["Human"] }))
    }]
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            let amount = param(&*ctx, "buff");
            vec![buff(json_as(json!({ "target": { "of": "chosen" }, "attack": amount, "health": amount })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 4 is its declared `buff`, which `param` reads.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// TG #2 Personal Trainer — SPEC §8.11 row 2, BUILD M13 row TG 2 (TG-P3-1).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TRAINER: &str = super::ID;
    const HUMAN: &str = "traininggrounds-001"; // Spikey Steven 5/2, Human
    const BEAST: &str = "core-012"; // Duplicating Felinors 3/4, Felinor: no Human
    const FILLER: &str = "core-005";

    /// p1 holds the Trainer, a Human in lane 1 and a Felinor in lane 2; p2 a Human in lane 1.
    fn trainer(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": TRAINER, "radiant": radiant }, FILLER], "field": [HUMAN, BEAST] },
            "p2": { "hand": [FILLER], "field": [HUMAN] },
        }))
    }

    /// The instance ids `legalActions` offers the Trainer as targets.
    fn offered(s: &Scenario) -> Vec<String> {
        let trainer = s.card(TRAINER).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .flat_map(|action| match action {
                ActionBody::Play { instance_id, targets, .. } if instance_id == trainer => targets
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|pick| match pick {
                        Selection::Instance { instance_id } => Some(instance_id),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .fold(Vec::new(), |mut seen, id| {
                // One play is offered per open zone: each pick once.
                if !seen.contains(&id) {
                    seen.push(id);
                }
                seen
            })
    }

    fn buffed_count(s: &Scenario) -> usize {
        s.last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Buffed { .. }))
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn s10_4_the_chosen_friendly_human_gets_2_2_permanently() {
            let mut s = trainer(false);
            let human = s.unit_at(P1, 1);
            let targets = pick::unit_at(&s, P1, 1);
            s.play(TRAINER, json!({ "targets": targets, "zone": 3 }));
            assert_eq!(buffed_count(&s), 1);
            s.expect_stats(&human, json!({ "attack": 7, "health": 4, "maxHealth": 4 }));
            s.end_turn().end_turn();
            s.expect_stats(&human, json!({ "attack": 7, "health": 4 }));
        }

        #[test]
        fn r81_an_enemy_human_and_a_friendly_non_human_are_no_legal_picks() {
            let mut s = trainer(false);
            let enemy = s.unit_at(P2, 1);
            let beast = s.unit_at(P1, 2);
            let offered = offered(&s);
            assert_eq!(offered, vec![s.unit_at(P1, 1).id]);
            assert!(!offered.contains(&enemy.id) && !offered.contains(&beast.id));
            let at_enemy = pick::unit_at(&s, P2, 1);
            s.expect_refused(|s| s.play(TRAINER, json!({ "targets": at_enemy })));
            let at_beast = pick::unit_at(&s, P1, 2);
            s.expect_refused(|s| s.play(TRAINER, json!({ "targets": at_beast })));
        }

        #[test]
        fn r90_with_no_other_human_it_is_played_the_cry_fizzles_and_it_enters_3_2() {
            let mut s = scenario(json!({
                "p1": { "hand": [TRAINER, FILLER], "field": [BEAST] },
                "p2": { "hand": [FILLER], "field": [HUMAN] },
            }));
            s.play(TRAINER, json!({}));
            s.expect_in_zone(TRAINER, "field");
            s.expect_stats(TRAINER, json!({ "attack": 3, "health": 2 }));
            assert_eq!(buffed_count(&s), 0);
            let beast = s.unit_at(P1, 1);
            s.expect_stats(&beast, json!({ "attack": 3, "health": 4 }));
        }

        #[test]
        fn r923_a_unit_given_the_human_tag_is_a_legal_pick() {
            let mut s = trainer(false);
            let beast = s.unit_at(P1, 2);
            s.card_mut(&beast).granted_tags = Some(vec![Tag::Human]);
            assert!(offered(&s).contains(&beast.id));
            let targets = pick::unit_at(&s, P1, 2);
            s.play(TRAINER, json!({ "targets": targets }));
            s.expect_stats(&beast, json!({ "attack": 5, "health": 6 }));
        }

        #[test]
        fn r386_an_upgrade_gives_3_3_a_degrade_1_1() {
            for (step, gain) in [(1, 3), (-1, 1)] {
                let mut s = trainer(false);
                step_param(s.card_mut(TRAINER), "buff", step);
                let human = s.unit_at(P1, 1);
                let targets = pick::unit_at(&s, P1, 1);
                s.play(TRAINER, json!({ "targets": targets }));
                s.expect_stats(&human, json!({ "attack": 5 + gain, "health": 2 + gain }));
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s10_4_a_6_4_that_gives_the_chosen_friendly_human_4_4() {
            let mut s = trainer(true);
            let human = s.unit_at(P1, 1);
            let targets = pick::unit_at(&s, P1, 1);
            s.play(TRAINER, json!({ "targets": targets }));
            s.expect_stats(TRAINER, json!({ "attack": 6, "health": 4 }));
            s.expect_stats(&human, json!({ "attack": 9, "health": 6, "maxHealth": 6 }));
        }

        #[test]
        fn r2196000_the_radiant_face_cannot_pick_itself() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": TRAINER, "radiant": true }, FILLER], "field": [BEAST] },
                "p2": { "hand": [FILLER] },
            }));
            let me = s.card(TRAINER).id.clone();
            assert!(!offered(&s).contains(&me));
            let at_itself = pick::card(&s, TRAINER);
            s.expect_refused(|s| s.play(TRAINER, json!({ "targets": at_itself })));
            // Played with no pick, it is the only Human of yours and the Cry fizzles: it stays 6/4.
            s.play(TRAINER, json!({}));
            s.expect_stats(TRAINER, json!({ "attack": 6, "health": 4, "maxHealth": 4 }));
            assert_eq!(buffed_count(&s), 0);
        }
    }
}
