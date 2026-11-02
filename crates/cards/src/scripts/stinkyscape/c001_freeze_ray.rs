//! S #1 Freeze Ray (SPEC §8.10 row S 1; §3.1, §6.1, R81, R1174000, R1867000): (0) Spell, Common.
//!   Base:    "Freeze a Unit and the Units adjacent to it."
//!   Radiant: "Freeze all enemy Units."
//!
//! Engine: a declared target, any Unit on either side (R81); `freeze` on it and, through
//! `for_each_card` over `adjacent_to` (Core #16 Hit Job's neighbour walk, Meditative #24
//! Polymorph's), on each Unit beside it in its row on its side (§3.1). Each Freeze is R1174000's;
//! a neighbour Immune to Spells is passed by, an Immutable one Frozen anyway (R1867000). The
//! Radiant face declares no target and Freezes every enemy Unit (`freeze_all`, M+ #24 Bob's).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-001";

/// R81: any Unit on either side, picked with the play.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

/// The Units beside the chosen one, in its row on its side (§3.1).
fn neighbours(ctx: &mut EffectContext<'_>) -> Vec<String> {
    adjacent_to(ctx, &json_as(json!({ "of": "chosen" })), &BoardScope::default())
        .into_iter()
        .map(|card| card.id)
        .collect()
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            targets: targets(),
            cry: Some(hook(|_ctx| {
                vec![
                    freeze(json_as(json!({ "target": { "of": "chosen" } }))),
                    for_each_card(ForEachCardArgs {
                        cards: Arc::new(neighbours),
                        each: Arc::new(|instance_id: &str| {
                            freeze(json_as(json!({
                                "target": { "of": "instance", "instanceId": instance_id },
                            })))
                        }),
                    }),
                ]
            })),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| vec![freeze_all(json_as(json!({ "side": "enemy" })))])),
            ..Script::default()
        },
    }
}

// S #1 Freeze Ray — SPEC §8.10 row S 1, BUILD M12 row S 1 (R1174000, R1867000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const RAY: &str = "stinkyscape-001";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005";
    const IMMUNE: &str = "classicplus-019-1"; // Radiant: Immune to Spells.
    const ROCK: &str = "core-066"; // (10) 10/10 Indestructible; Radiant also Immutable.

    fn frozen(s: &Scenario, card: &CardInstance) -> bool {
        s.card(card).frozen.is_some()
    }

    mod base {
        use super::*;

        #[test]
        fn r1867000_freezes_the_target_and_the_units_beside_it_in_its_row_on_its_side() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [RAY], "field": [VANILLA] },
                "p2": { "field": [VANILLA, VANILLA, VANILLA, VANILLA] },
            }));
            let left = s.unit_at(P2, 1);
            let target = s.unit_at(P2, 2);
            let right = s.unit_at(P2, 3);
            let far = s.unit_at(P2, 4);
            let mine = s.unit_at(P1, 1);

            s.play(RAY, json!({ "targets": pick::instance(&target) }));

            assert!(frozen(&s, &target));
            assert!(frozen(&s, &left));
            assert!(frozen(&s, &right));
            // Lane 4 is two lanes away: §3.1's adjacency is N-1 and N+1 and nothing wider.
            assert!(!frozen(&s, &far));
            assert!(!frozen(&s, &mine));
        }

        #[test]
        fn freezes_a_friendly_target_and_its_neighbours_as_readily() {
            crate::register_all();
            let mut s = scenario(json!({
                // A filler in hand: with nothing meaningful left the harness would end the turn,
                // and an own-side freeze thaws at its caster's cleanup (R1174000).
                "p1": { "hand": [RAY, FILLER], "field": [VANILLA, VANILLA] },
                "p2": { "hand": [FILLER], "field": [VANILLA] },
            }));
            let mine = s.unit_at(P1, 1);
            let neighbour = s.unit_at(P1, 2);
            let theirs = s.unit_at(P2, 1);

            s.play(RAY, json!({ "targets": pick::instance(&mine) }));

            assert!(frozen(&s, &mine));
            assert!(frozen(&s, &neighbour));
            assert!(!frozen(&s, &theirs));
        }

        #[test]
        fn freezes_a_lone_unit_alone() {
            crate::register_all();
            let mut s = scenario(json!({
                // A filler in hand: with nothing meaningful left the harness would end the turn,
                // and the freeze would thaw before the assert reads it (R1174000).
                "p1": { "hand": [RAY, FILLER] },
                "p2": { "hand": [FILLER], "field": [{ "def": VANILLA, "lane": 3 }] },
            }));
            let target = s.unit_at(P2, 3);

            s.play(RAY, json!({ "targets": pick::instance(&target) }));

            assert!(frozen(&s, &target));
            s.expect_in_zone(&target, "field");
        }

        #[test]
        fn r1867000_passes_an_immune_to_spells_neighbour_by_and_freezes_an_immutable_one() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [RAY, FILLER] },
                "p2": {
                    "hand": [FILLER],
                    "field": [
                        { "def": IMMUNE, "radiant": true, "lane": 1 },
                        { "def": VANILLA, "lane": 2 },
                        { "def": ROCK, "radiant": true, "lane": 3 },
                    ],
                },
            }));
            let immune = s.unit_at(P2, 1);
            let target = s.unit_at(P2, 2);
            let immutable = s.unit_at(P2, 3);

            s.play(RAY, json!({ "targets": pick::instance(&target) }));

            assert!(frozen(&s, &target));
            assert!(!frozen(&s, &immune));
            assert!(frozen(&s, &immutable));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn freezes_every_enemy_unit_and_no_friendly_one() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": RAY, "radiant": true }], "field": [VANILLA, VANILLA] },
                "p2": { "field": [VANILLA, VANILLA, VANILLA] },
            }));

            s.play(RAY, json!({}));

            for lane in 1..=3 {
                assert!(frozen(&s, &s.unit_at(P2, lane)), "lane {lane}");
            }
            for lane in 1..=2 {
                assert!(!frozen(&s, &s.unit_at(P1, lane)), "lane {lane}");
            }
        }

        #[test]
        fn declares_no_target() {
            crate::register_all();
            let s = scenario(json!({
                "p1": { "hand": [{ "def": RAY, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            let plays: Vec<Value> = legal_actions(s.state(), P1)
                .iter()
                .map(crate::js)
                .filter(|action| {
                    action["type"] == "play" && action["instanceId"] == s.card(RAY).id.as_str()
                })
                .collect();
            assert_eq!(plays.len(), 1);
            assert!(plays[0].get("targets").is_none_or(Value::is_null));
        }
    }
}
