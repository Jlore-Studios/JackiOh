//! S #99 No-No Words (SPEC §8.10 row S 99; §3.1, §10.4, R59, R81, R348, R703, R1848003, R1868000):
//! (X) Spell, Common.
//!   Base:    "Destroy a Unit with less than {per}X Attack." — per 4.
//!   Radiant: "Destroy a Unit with less than {per}X Attack, and the Units adjacent to it."
//!
//! Engine: an X Spell (R348), its player choosing X as they play it, or its set X on S #102's copies
//! (R1848003). A declared target, any Unit whose Attack, read through the layers (§10.4), is strictly
//! below `per` × X (ST-S14-4): the check is asked of the card with the play's X stamped (R1868000), so
//! `legalActions` pairs each X with the Units it reaches and the reducer refuses any other pick. The
//! pick is needed (R703), as S #12.1 and #12.2's are: an X no Unit is below is no play. It is not

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-099";

/// The target check: a Unit below `per` × X Attack.
const BELOW: &str = "belowPerX";

/// R81, R703: any Unit on either side, picked with the play and needed by it.
fn targets() -> Vec<TargetDecl> {
    let mut decl = TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"], "check": BELOW }));
    decl.required = Some(true);
    vec![decl]
}

/// R1868000: strictly below `per` × the play's X, Attack through the layers.
fn below(args: TargetCheckArgs<'_>) -> bool {
    let limit = param(&args, "per") * x_of(args.self_);
    args.candidate
        .is_some_and(|unit| unit_view(args.state, unit).attack < limit)
}

fn no_no_words(radiant: bool) -> Script {
    Script {
        targets: targets(),
        target_checks: IndexMap::from([(BELOW, target_check(below))]),
        cry: Some(hook(move |_ctx| {
            let mut effects = vec![destroy(json_as(json!({ "target": { "of": "chosen" } })))];
            if radiant {
                effects.push(destroy_adjacent_to(json_as(json!({ "target": { "of": "chosen" } }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: no_no_words(false),
        radiant: no_no_words(true),
    }
}

// S #99 No-No Words — SPEC §8.10 row S 99, BUILD M12 row S 99 (R1868000, R348).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const NONO: &str = "stinkyscape-099";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const GREASER: &str = "meditative-100"; // (2) Unit 7/7.
    const GENN: &str = "classic-086"; // (4) Unit 14/14.
    const FILLER: &str = "core-005";
    const SCALING_LAW: &str = "classicplus-t-ai-02";
    const MANA: i32 = 10;

    fn plays(s: &Scenario) -> Vec<(Option<i32>, Vec<Selection>)> {
        let held = s.card(NONO).id.clone();
        legal_actions(s.state(), P1)
            .iter()
            .filter_map(|action| match action {
                ActionBody::Play {
                    instance_id,
                    x,
                    targets,
                    ..
                } if *instance_id == held => Some((*x, targets.clone().unwrap_or_default())),
                _ => None,
            })
            .collect()
    }

    fn nono_board(radiant: bool, theirs: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": { "hand": [{ "def": NONO, "radiant": radiant }, FILLER] },
            "p2": { "hand": [FILLER], "field": theirs },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r1868000_x1_destroys_3_attack_and_refuses_4() {
            let mut s = nono_board(false, json!([VANILLA, VANILLA]));
            let three = s.unit_at(P2, 1);
            let four = s.unit_at(P2, 2);
            s.card_mut(&three).buffs.attack = -1;

            s.expect_refused(|s| s.play(NONO, json!({ "x": 1, "targets": pick::instance(&four) })));
            s.expect_in_zone(NONO, "hand");
            s.play(NONO, json!({ "x": 1, "targets": pick::instance(&three) }));

            s.expect_in_zone(&three, "graveyard");
            s.expect_in_zone(&four, "field");
        }

        #[test]
        fn r1868000_x2_reaches_7() {
            let mut s = nono_board(false, json!([GREASER]));
            let seven = s.unit_at(P2, 1);
            assert!(plays(&s).iter().all(|(x, _)| *x != Some(1)));

            s.play(NONO, json!({ "x": 2, "targets": pick::instance(&seven) }));

            s.expect_in_zone(&seven, "graveyard");
        }

        #[test]
        fn r1868000_x0_is_no_play() {
            let mut s = nono_board(false, json!([{ "def": VANILLA, "statsOverride": { "attack": 0, "health": 4 } }]));
            let zero = s.unit_at(P2, 1);
            // Nothing is below 4 × 0, and R348 offers no X of 0.
            assert!(plays(&s).iter().all(|(x, _)| x.is_some_and(|x| x >= 1)));

            s.expect_refused(|s| s.play(NONO, json!({ "x": 0, "targets": pick::instance(&zero) })));
            s.expect_in_zone(&zero, "field");
        }

        #[test]
        fn r703_with_no_unit_on_the_field_it_is_no_play() {
            let mut s = scenario(json!({
                "p1": { "hand": [NONO, FILLER], "mana": MANA },
                "p2": { "hand": [FILLER] },
            }));
            assert!(plays(&s).is_empty(), "{:?}", plays(&s));
            s.expect_refused_with(|s| s.play(NONO, json!({ "x": 1 })), "cannot be played without");
            s.expect_in_zone(NONO, "hand");
        }

        #[test]
        fn r703_an_x_no_unit_is_below_is_not_offered_and_is_refused() {
            let mut s = scenario(json!({
                "p1": { "hand": [NONO, FILLER], "mana": MANA },
                "p2": { "hand": [FILLER], "field": [GENN] },
            }));
            let genn = s.unit_at(P2, 1);
            let offered = plays(&s);
            assert!(
                offered.iter().all(|(x, _)| x.is_some_and(|x| x >= 4)),
                "{offered:?}"
            );
            assert!(offered.iter().any(|(x, _)| *x == Some(4)), "{offered:?}");
            s.expect_refused_with(|s| s.play(NONO, json!({ "x": 3 })), "cannot be played without");
            s.expect_refused(|s| s.play(NONO, json!({ "x": 3, "targets": pick::instance(&genn) })));
            s.expect_in_zone(NONO, "hand");
            s.play(NONO, json!({ "x": 4, "targets": pick::instance(&genn) }));
            s.expect_in_zone(&genn, "graveyard");
        }

        #[test]
        fn r1868000_a_target_raised_past_it_after_the_pick_still_dies() {
            let mut s = scenario(json!({
                "p1": { "hand": [NONO, FILLER], "field": [SCALING_LAW], "mana": MANA },
                "p2": { "hand": [FILLER] },
            }));
            let law = s.unit_at(P1, 1);
            s.card_mut(&law).buffs.attack = 1;
            s.expect_stats(&law, json!({ "attack": 3 }));
            s.card_mut(NONO)
                .granted_tags
                .get_or_insert_with(Vec::new)
                .push(Tag::Ai);

            s.play(NONO, json!({ "x": 1, "targets": pick::instance(&law) }));

            // §10.5 step 4 counted the AI card's play, raising the pick to 4 before the Spell resolved.
            assert_eq!(played_this_game_with_tag(s.state(), P1, Tag::Ai), 1);
            assert!(s.unit(P1, 1).is_none(), "the raised Unit still died");
            assert!(s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::Destroyed { instance_id, .. } if *instance_id == law.id
            )));
        }

        #[test]
        fn per_reads_through_param() {
            let mut s = nono_board(false, json!([VANILLA]));
            assert_eq!(upgrade_number(&mut s, NONO, "per"), 5);
            let four = s.unit_at(P2, 1);

            s.play(NONO, json!({ "x": 1, "targets": pick::instance(&four) }));

            s.expect_in_zone(&four, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1868000_the_neighbours_die_whatever_their_attack() {
            let mut s = nono_board(true, json!([GENN, VANILLA, GENN]));
            let left = s.unit_at(P2, 1);
            let target = s.unit_at(P2, 2);
            let right = s.unit_at(P2, 3);
            s.card_mut(&target).buffs.attack = -1;

            s.play(NONO, json!({ "x": 1, "targets": pick::instance(&target) }));

            s.expect_in_zone(&target, "graveyard");
            s.expect_in_zone(&left, "graveyard");
            s.expect_in_zone(&right, "graveyard");
        }
    }
}
