//! M #36 CN Peptides (SPEC §8.8 row 36): (1) Spell, CN, Common.
//!
//! Base:    "Choose a Unit. 50% chance: Buff it {times|time|times}. Otherwise, destroy it."
//! Radiant: "Lucky 1\nChoose a Unit. 50% chance: Buff it {times|time|times}. Otherwise, destroy it."
//! Engine:
//! - **Play-time pick:** a declared Unit on either side (R81). The pick is `required` (R703), so
//!   with no Unit the play is refused; a cast still fizzles (R70).
//! - **The roll:** the Cry resolves the target first; with no target it draws nothing (R129). Then
//!   it flips one `ctx.rng.coin()`. With the card's Lucky X (§6.1 `rng.lucky`; the Spell's printed
//!   keyword as Nerf and Buff have moved it, plus any Lucky given to it, R1438, read with `lucky_on`
//!   as `effects/fruit.rs`'s `lucky_of` reads a Spell's), it flips X more coins. The comparator keeps
//!   the outcome better for the caster: the Buffs on a Unit the caster controls, the destroy on an
//!   enemy one (MD-B17, R902).
//! - **Heads:** `upgrade({ target: chosen, times })`. **Tails:** `destroy`; an Indestructible Unit
//!   survives (R46).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-036";

/// The card's Lucky X, printed plus given (R1438): the face's as Nerf and Buff have moved it, and any
/// Lucky given to it (`effects/fruit.rs`'s `lucky_of` shape).
fn lucky_of(ctx: &EffectContext<'_>) -> i32 {
    ctx.self_.as_ref().map_or(0, |own| lucky_on(ctx.sink.state, own))
}

fn peptides() -> Script {
    Script {
        targets: vec![TargetDecl {
            // R703: no Unit to target means no play. The default `aim: harm` stays: the destroy
            // is the harmful branch, and a cast that targets enemies should aim it at them.
            required: Some(true),
            ..TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))
        }],
        cry: Some(hook(|ctx| {
            let Some(target) = instance_of(&*ctx, &TargetSpec::Chosen { index: None }) else {
                return vec![];
            };
            let (id, friendly) = (target.id.clone(), target.controller == ctx.controller);
            let lucky = lucky_of(ctx);
            let heads = ctx.rng.lucky(
                lucky,
                |rng| rng.coin(),
                move |a, b| {
                    if friendly { a || b } else { a && b }
                },
            );
            if heads {
                vec![upgrade(json_as(json!({
                    "target": { "of": "instance", "instanceId": id },
                    "times": param(&*ctx, "times"),
                })))]
            } else {
                vec![destroy(json_as(json!({
                    "target": { "of": "instance", "instanceId": id },
                })))]
            }
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // Both faces share the one script: the printed Lucky 1 and the declared `times` (7,
    // radiant 10) are what differ.
    let base = peptides();
    let radiant = peptides();
    CardScripts { base, radiant }
}

// M #36 CN Peptides — SPEC §8.8 row 36, BUILD M10 row M 36: "A declared Unit on either side; one
// coin: heads 7 separate Buffs (R386), tails destroyed, an Indestructible one surviving (R46); fair
// on the base face; times reads through `param()`; radiant Lucky 1: two coins, keeping the Buffs on
// your own Unit and the destroy on an enemy one (MD-B17), the controller read as it resolves, so
// 75% each way, and 10 Buffs".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PEPTIDES: &str = "meditative-036";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4, the target.
    const WALL: &str = "classic-041"; // 3/3 Indestructible, which tails cannot kill.
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// p1 holds Peptides (base unless `radiant_face`); the victim stands in `lane` of `side`'s
    /// field. Both sides keep cards in hand so no turn auto-ends.
    fn aiming(seed: &str, radiant_face: bool, side: PlayerId, lane: i32, victim: &str) -> Scenario {
        crate::register_all();
        let (p1_field, p2_field) = if side == P1 {
            (json!([{ "def": victim, "lane": lane }]), json!([]))
        } else {
            (json!([]), json!([{ "def": victim, "lane": lane }]))
        };
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": PEPTIDES, "radiant": radiant_face }, FILLER],
                "field": p1_field,
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "field": p2_field, "library": filler(4) },
        }))
    }

    /// The pick of the victim, as the play action carries it (R81).
    fn pick(id: &str) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
    }

    /// The coins the match rng would flip for a scenario built but not yet played: one, or two on
    /// the Radiant face.
    fn coins(s: &Scenario, radiant_face: bool) -> Vec<bool> {
        let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
        let count = if radiant_face { 2 } else { 1 };
        (0..count).map(|_| rng.coin()).collect()
    }

    fn upgraded(events: &[GameEvent]) -> Vec<Value> {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Upgraded { .. }))
            .map(crate::js)
            .collect()
    }

    fn destroyed(events: &[GameEvent]) -> Vec<Value> {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Destroyed { .. }))
            .map(crate::js)
            .collect()
    }

    mod m36_cn_peptides {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r902_base_one_coin_heads_buffs_7_times_tails_destroys() {
                let mut saw_heads = false;
                let mut saw_tails = false;
                for n in 0..32 {
                    let seed = format!("peptides-{n}");
                    let mut s = aiming(&seed, false, P1, 1, VANILLA);
                    let victim = s.unit(P1, 1).expect("the victim");
                    let heads = coins(&s, false)[0];
                    s.play(PEPTIDES, pick(&victim.id));
                    if heads {
                        saw_heads = true;
                        // Seven separate Buffs (R386), and the victim stands.
                        assert_eq!(upgraded(s.events()).len(), 7, "{seed}");
                        assert!(destroyed(s.events()).is_empty(), "{seed}");
                        assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(victim.id));
                    } else {
                        saw_tails = true;
                        assert_eq!(destroyed(s.events()).len(), 1, "{seed}");
                        s.expect_in_zone(&victim, "graveyard");
                    }
                }
                assert!(saw_heads && saw_tails, "both outcomes occur over 32 seeds");
            }

            #[test]
            fn r46_an_indestructible_unit_survives_tails() {
                let mut s = None;
                for n in 0..32 {
                    let seed = format!("peptides-wall-{n}");
                    let candidate = aiming(&seed, false, P1, 1, WALL);
                    if !coins(&candidate, false)[0] {
                        s = Some(candidate);
                        break;
                    }
                }
                let mut s = s.expect("a tails seed over 32 tries");
                let wall = s.unit(P1, 1).expect("the wall");
                s.play(PEPTIDES, pick(&wall.id));
                // Tails destroys, and the Indestructible wall ignores the mark (R46): no Buffs,
                // no destruction, and the wall stands as it was.
                assert!(upgraded(s.events()).is_empty());
                assert!(destroyed(s.events()).is_empty());
                let wall_id = wall.id.clone();
                assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(wall_id));
                s.expect_stats(&wall, json!({ "attack": 3, "health": 3 }));
            }

            #[test]
            fn cannot_be_played_with_no_unit() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "peptides-no-unit",
                    "p1": { "hand": [PEPTIDES, FILLER], "library": filler(4) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.expect_refused(|s| s.play(PEPTIDES, json!({})));
            }

            #[test]
            fn times_reads_through_param() {
                // A heads seed, then the number is tuned on that scenario's card.
                let mut heads = None;
                for n in 0..32 {
                    let candidate = aiming(&format!("peptides-param-{n}"), false, P1, 1, VANILLA);
                    if coins(&candidate, false)[0] {
                        heads = Some(candidate);
                        break;
                    }
                }
                let mut s = heads.expect("a heads seed over 32 tries");
                set_param(s.card_mut(PEPTIDES), "times", 3);
                let victim = s.unit(P1, 1).expect("the victim");
                s.play(PEPTIDES, pick(&victim.id));
                assert_eq!(upgraded(s.events()).len(), 3);
            }
        }

        /// R1438: given Lucky 1, the base face flips twice and keeps the result better for the caster:
        /// on your own Unit, the 7 Buffs unless both coins land tails.
        #[test]
        fn r1438_given_lucky_1_the_base_face_rolls_twice_and_keeps_the_better() {
            let mut pairs = std::collections::BTreeSet::new();
            for n in 0..32 {
                let seed = format!("peptides-given-lucky-{n}");
                let mut s = aiming(&seed, false, P1, 1, VANILLA);
                crate::give_lucky(&mut s, PEPTIDES, 1);
                let victim = s.unit(P1, 1).expect("the victim");
                let pair = coins(&s, true);
                pairs.insert((pair[0], pair[1]));
                s.play(PEPTIDES, pick(&victim.id));
                if pair[0] || pair[1] {
                    assert_eq!(upgraded(s.events()).len(), 7, "{seed}");
                    assert!(destroyed(s.events()).is_empty(), "{seed}");
                } else {
                    assert_eq!(destroyed(s.events()).len(), 1, "{seed}");
                }
            }
            assert_eq!(pairs.len(), 4, "all four coin pairs occur over 32 seeds");
        }

        mod radiant {
            use super::*;

            #[test]
            fn r902_lucky_keeps_the_buffs_on_your_own_unit() {
                // Heads unless both coins land tails: 75% for the caster's own Unit.
                let mut pairs = std::collections::BTreeSet::new();
                for n in 0..32 {
                    let seed = format!("peptides-lucky-own-{n}");
                    let mut s = aiming(&seed, true, P1, 1, VANILLA);
                    let victim = s.unit(P1, 1).expect("the victim");
                    let pair = coins(&s, true);
                    pairs.insert((pair[0], pair[1]));
                    let heads = pair[0] || pair[1];
                    s.play(PEPTIDES, pick(&victim.id));
                    if heads {
                        // The Buffs come 10 at a time.
                        assert_eq!(upgraded(s.events()).len(), 10, "{seed}");
                        assert!(destroyed(s.events()).is_empty(), "{seed}");
                    } else {
                        assert_eq!(destroyed(s.events()).len(), 1, "{seed}");
                    }
                }
                assert_eq!(pairs.len(), 4, "all four coin pairs occur over 32 seeds");
            }

            #[test]
            fn r902_lucky_keeps_the_destroy_on_an_enemy_unit() {
                // Heads only when both coins land heads: the destroy is kept 75% of the time.
                let mut pairs = std::collections::BTreeSet::new();
                for n in 0..32 {
                    let seed = format!("peptides-lucky-foe-{n}");
                    let mut s = aiming(&seed, true, P2, 1, VANILLA);
                    let victim = s.unit(P2, 1).expect("the victim");
                    let pair = coins(&s, true);
                    pairs.insert((pair[0], pair[1]));
                    let heads = pair[0] && pair[1];
                    s.play(PEPTIDES, pick(&victim.id));
                    if heads {
                        assert_eq!(upgraded(s.events()).len(), 10, "{seed}");
                        assert!(destroyed(s.events()).is_empty(), "{seed}");
                    } else {
                        assert_eq!(destroyed(s.events()).len(), 1, "{seed}");
                        s.expect_in_zone(&victim, "graveyard");
                    }
                }
                assert_eq!(pairs.len(), 4, "all four coin pairs occur over 32 seeds");
            }
        }
    }
}
