//! M #35 RCTA (CN) (SPEC §8.8 row 35): (1) Spell, CN, Common.
//!
//! Base and Radiant: "Give a non-CN permanent the CN tag and +{stats}/+{stats}. Buff it
//! {times|time|times}."
//! Engine:
//! - **Play-time pick:** a declared target (`TargetDecl` of kind target, `of: [unit, backrow]` on
//!   either side, `notTags: [CN]`, `aim: help`, `required`, R81). A face-down card its chooser
//!   cannot read is always offered, whatever its tags (MD-B16, R924).
//! - **On resolution:** NEW `grant_tag` adds CN to a NEW `CardInstance.granted_tags`; ME-CN sets
//!   `chinese` on the card; `buff` gives +stats/+stats (layer 4, stored even on a backrow card,
//!   where it applies if the card animates); `upgrade({ target: chosen, times })`. As the designer
//!   asked, the printed text never says Chinese.
//! - A target that is CN by the time the Spell resolves fails its filter and fizzles (§10.5).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-035";

fn rcta() -> Script {
    Script {
        targets: vec![TargetDecl {
            aim: Some(TargetAim::Help),
            required: Some(true),
            ..TargetDecl::target(
                1,
                1,
                json!({ "side": "any", "of": ["unit", "backrow"], "notTags": ["CN"] }),
            )
        }],
        cry: Some(hook(|ctx| {
            let Some(target) = instance_of(&*ctx, &TargetSpec::Chosen { index: None }) else {
                return vec![];
            };
            // A target that is CN by the time the Spell resolves fails its filter (§10.5): the
            // effect fizzles on it (MD-B16, R924).
            if tags_of(&*ctx.state, &target).contains(&Tag::Cn) {
                return vec![];
            }
            let id = target.id.clone();
            let stats = param(&*ctx, "stats");
            let times = param(&*ctx, "times");
            vec![
                grant_tag(json_as(json!({ "instanceId": id, "tag": "CN" }))),
                translate(json_as(json!({ "instanceId": id }))),
                buff(json_as(json!({
                    "target": { "of": "instance", "instanceId": id },
                    "attack": stats,
                    "health": stats,
                }))),
                upgrade(json_as(json!({
                    "target": { "of": "instance", "instanceId": id },
                    "times": times,
                }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // Both faces share the one script: the declared `stats` (2, radiant 5) and `times` (1,
    // radiant 2) are what differ.
    let base = rcta();
    let radiant = rcta();
    CardScripts { base, radiant }
}

// M #35 RCTA (CN) — SPEC §8.8 row 35, BUILD M10 row M 35: "The declared target is a non-CN
// permanent on either side, a face-down card its chooser cannot read always offered (MD-B16); it
// gains the CN tag as a granted tag that every instance-level read sees (M 34's scopes, M 78's
// filter, Core 2's pick) while pools do not, kept in every zone and by a copy, dropped by a
// Transform (MD-B15), and the `chinese` flag, which no rule reads (MD-B11); then +2/+2 permanently
// and one Buff; a target that is CN by resolution fizzles; stats and times read through `param()`;
// radiant +5/+5 and two Buffs".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const RCTA: &str = "meditative-035";
    const FILLER: &str = "core-005";
    const VANILLA: &str = "core-008"; // 4/4, untagged.
    const BROTHER: &str = "classicplus-076"; // CN Unit.
    const HONEYPOT: &str = "core-060"; // Untagged Trap, set face-down.

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// p1 holds RCTA (base unless `radiant_face`) and a Vanilla stands in lane 1; both sides keep
    /// cards in hand so no turn auto-ends.
    fn casting(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": RCTA, "radiant": radiant_face }, FILLER],
                "field": [{ "def": VANILLA, "lane": 1 }],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// The pick of the victim, as the play action carries it (R81).
    fn pick(id: &str) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
    }

    fn buffed_for(events: &[GameEvent]) -> Vec<(String, i32, i32)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Buffed { instance_id, attack, health } => {
                    Some((instance_id.clone(), *attack, *health))
                }
                _ => None,
            })
            .collect()
    }

    fn upgraded_for(events: &[GameEvent], id: &str) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Upgraded { instance_id, .. } if instance_id == id))
            .count()
    }

    mod m35_rcta_cn {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r923_target_gains_cn_chinese_plus_2_2_and_one_buff() {
                let mut s = casting("rcta", false);
                let victim = s.unit(P1, 1).expect("the Vanilla");
                s.play(RCTA, pick(&victim.id));
                let changed = s.card(&victim.id).clone();
                // The granted tag, the flag, the permanent stats and the one Buff.
                assert_eq!(changed.granted_tags, Some(vec![Tag::Cn]));
                assert_eq!(changed.chinese, Some(true));
                assert_eq!((changed.buffs.attack, changed.buffs.health), (2, 2));
                assert_eq!(upgraded_for(s.events(), &victim.id), 1);
                assert_eq!(buffed_for(s.events()), vec![(victim.id.clone(), 2, 2)]);
            }

            #[test]
            fn a_cn_unit_is_refused() {
                // The only permanent is CN and no backrow card is offered: the play is refused.
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "rcta-refused",
                    "p1": {
                        "hand": [{ "def": RCTA }, FILLER],
                        "field": [{ "def": BROTHER, "lane": 1 }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.expect_refused(|s| s.play(RCTA, json!({})));
            }

            #[test]
            fn r924_an_enemy_face_down_trap_is_a_legal_pick_and_fizzles_if_cn() {
                // The enemy's face-down Honeypot is a legal pick, whatever its tags: the play
                // resolves onto it, granting CN, Chinese, stats and a Buff.
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "rcta-trap",
                    "p1": {
                        "hand": [{ "def": RCTA }, FILLER],
                        "library": filler(4),
                    },
                    "p2": {
                        "hand": [FILLER],
                        "backrow": [{ "def": HONEYPOT, "lane": 1 }],
                        "library": filler(4),
                    },
                }));
                let trap = s.backrow(P2, 1).expect("the face-down trap");
                s.play(RCTA, pick(&trap.id));
                let changed = s.card(&trap.id).clone();
                assert_eq!(changed.granted_tags, Some(vec![Tag::Cn]));
                assert_eq!(changed.chinese, Some(true));
                assert_eq!((changed.buffs.attack, changed.buffs.health), (2, 2));

                // CN by the time the Spell resolves: the effect fizzles on it — no flag, no
                // stats, no Buff.
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "rcta-fizzle",
                    "p1": {
                        "hand": [{ "def": RCTA }, FILLER],
                        "library": filler(4),
                    },
                    "p2": {
                        "hand": [FILLER],
                        "backrow": [{ "def": HONEYPOT, "lane": 1 }],
                        "library": filler(4),
                    },
                }));
                let trap = s.backrow(P2, 1).expect("the face-down trap");
                // CN before the play resolves, as a mid-resolution grant would leave it.
                find_instance_mut(s.state_mut(), &trap.id).expect("the trap").granted_tags =
                    Some(vec![Tag::Cn]);
                s.play(RCTA, pick(&trap.id));
                let unchanged = s.card(&trap.id).clone();
                assert_eq!(unchanged.chinese, None);
                assert_eq!((unchanged.buffs.attack, unchanged.buffs.health), (0, 0));
                assert_eq!(upgraded_for(s.events(), &trap.id), 0);
                assert!(buffed_for(s.events()).is_empty());
            }

            #[test]
            fn stats_and_times_read_through_param() {
                let mut s = casting("rcta-param", false);
                set_param(s.card_mut(RCTA), "stats", 4);
                set_param(s.card_mut(RCTA), "times", 3);
                let victim = s.unit(P1, 1).expect("the Vanilla");
                s.play(RCTA, pick(&victim.id));
                let changed = s.card(&victim.id).clone();
                assert_eq!((changed.buffs.attack, changed.buffs.health), (4, 4));
                assert_eq!(upgraded_for(s.events(), &victim.id), 3);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn plus_5_5_and_two_buffs() {
                let mut s = casting("rcta-radiant", true);
                let victim = s.unit(P1, 1).expect("the Vanilla");
                s.play(RCTA, pick(&victim.id));
                let changed = s.card(&victim.id).clone();
                assert_eq!(changed.granted_tags, Some(vec![Tag::Cn]));
                assert_eq!(changed.chinese, Some(true));
                assert_eq!((changed.buffs.attack, changed.buffs.health), (5, 5));
                assert_eq!(upgraded_for(s.events(), &victim.id), 2);
            }
        }
    }
}
