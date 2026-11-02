//! S #2 Absolute Zero (SPEC §8.10 row S 2; §6.1, §10.4, R78, R81, R1848000): (2) Spell, Rare.
//!   Base:    "Freeze a Unit forever."
//!   Radiant: "Freeze a Unit forever. You may give it Indestructible."
//!
//! Engine: a declared target, any Unit on either side (R81); `freeze` with `forever` on it
//! (R1848000): Frozen with its thaw stamped `FROZEN_FOREVER`, which no cleanup reaches, so only
//! leaving the field (R78) or a Vanilla ends it, a change of control keeps it, a later ordinary
//! Freeze changes nothing and a forced attack still moves it. The Radiant face's "You may" is a
//! choice of two modes made with the play (R81), not a prompt after it: given, `grant_keyword`

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-002";

/// The Radiant face's two modes: give the Unit Indestructible, or only Freeze it.
const GIVE: &str = "give Indestructible";
const FREEZE_ONLY: &str = "freeze only";

/// R81: any Unit on either side, picked with the play.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

/// R1848000: the chosen Unit Frozen with a thaw no cleanup reaches.
fn freeze_forever() -> Effect {
    freeze(json_as(json!({ "target": { "of": "chosen" }, "forever": true })))
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            targets: targets(),
            cry: Some(hook(|_ctx| vec![freeze_forever()])),
            ..Script::default()
        },
        radiant: Script {
            targets: targets(),
            modes: vec![ModeDecl {
                kind: PromptKind::Mode,
                options: vec![GIVE.to_string(), FREEZE_ONLY.to_string()],
            }],
            cry: Some(hook(|ctx| {
                let mut effects = vec![freeze_forever()];
                if chosen_options(ctx).first().map(String::as_str) == Some(GIVE) {
                    effects.push(grant_keyword(json_as(json!({
                        "target": { "of": "chosen" },
                        "keyword": { "kind": "Indestructible" },
                    }))));
                }
                effects
            })),
            ..Script::default()
        },
    }
}

// S #2 Absolute Zero — SPEC §8.10 row S 2, BUILD M12 row S 2 (R1848000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const ZERO: &str = "stinkyscape-002";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005";
    const WILD_SYSTEM: &str = "stinkyscape-120"; // (3) Spell: the melee (R1848005).
    const FLOOD: &str = "core-017"; // (4) Spell: Bounce all Units.
    const SILENCE: &str = "classicplus-009"; // (0) Spell: Vanilla a Unit.
    const STEAL: &str = "core-049"; // (4) Spell: Steal an enemy Unit.
    const BOB: &str = "meditativeplus-024"; // (3) Unit 5/5, Cry mode "freeze": Freeze all enemy Units.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const METEOR: &str = "stinkyscape-003"; // (3) Spell: 15 damage to a Unit, trampling.

    const GIVE: &str = "give Indestructible";
    const FREEZE_ONLY: &str = "freeze only";

    fn forever(s: &Scenario, card: &CardInstance) -> bool {
        s.card(card).frozen == Some(FROZEN_FOREVER)
    }

    fn acts(s: &Scenario, card: &CardInstance) -> bool {
        let unit = s.card(card).clone();
        legal_actions(s.state(), unit.controller)
            .iter()
            .any(|action| match action {
                ActionBody::Attack { attacker_id, .. } => *attacker_id == unit.id,
                ActionBody::SwitchPosition { instance_id } => *instance_id == unit.id,
                _ => false,
            })
    }

    mod base {
        use super::*;

        #[test]
        fn r1848000_the_chosen_unit_never_thaws() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [ZERO, FILLER], "library": filler(8) },
                "p2": { "hand": [FILLER], "field": [VANILLA], "library": filler(8) },
            }));
            let target = s.unit_at(P2, 1);

            s.play(ZERO, json!({ "targets": pick::instance(&target) }));
            assert!(forever(&s, &target));

            for _ in 0..6 {
                s.end_turn();
                assert!(forever(&s, &target), "turn {}", s.state().turn);
                if s.state().active == P2 {
                    assert!(!acts(&s, &target), "turn {}", s.state().turn);
                }
            }
        }

        #[test]
        fn r1848005_a_forced_attack_still_moves_it() {
            crate::register_all();
            let melee_after_zero = |seed: &str| {
                let mut s = scenario(json!({
                    "seed": seed,
                    "p1": { "hand": [ZERO, WILD_SYSTEM, FILLER], "field": [VANILLA], "mana": 10 },
                    "p2": { "hand": [FILLER], "field": [VANILLA] },
                }));
                let frozen = s.unit_at(P2, 1);
                s.play(ZERO, json!({ "targets": pick::instance(&frozen) }));
                s.play(WILD_SYSTEM, json!({}));
                (s, frozen)
            };
            let attacked = |s: &Scenario, card: &CardInstance| {
                s.last_events().iter().any(|event| {
                    matches!(
                        event,
                        GameEvent::AttackDeclared { attacker_id, forced: true, .. } if *attacker_id == card.id
                    )
                })
            };
            let seed = seed_where("absolute-zero-melee", |seed| {
                let (s, frozen) = melee_after_zero(seed);
                attacked(&s, &frozen)
            });

            let (mut s, frozen) = melee_after_zero(&seed);

            assert!(attacked(&s, &frozen), "the Unit Frozen forever makes the forced attack");
            s.expect_in_zone(&frozen, "graveyard");
        }

        #[test]
        fn r1848000_a_bounce_or_a_vanilla_ends_it() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [ZERO, ZERO, SILENCE, FLOOD, FILLER], "mana": 10 },
                "p2": { "hand": [FILLER], "field": [VANILLA, VANILLA] },
            }));
            let silenced = s.unit_at(P2, 1);
            let bounced = s.unit_at(P2, 2);
            let first = s.card(ZERO).id.clone();
            s.play(&first, json!({ "targets": pick::instance(&silenced) }));
            s.play(ZERO, json!({ "targets": pick::instance(&bounced) }));
            assert!(forever(&s, &silenced) && forever(&s, &bounced));

            s.play(SILENCE, json!({ "targets": pick::instance(&silenced) }));
            assert_eq!(s.card(&silenced).frozen, None, "a Vanilla ends it");
            s.expect_in_zone(&silenced, "field");

            s.play(FLOOD, json!({}));
            s.expect_in_zone(&bounced, "hand");
            assert_eq!(s.card(&bounced).frozen, None, "leaving the field ends it (R78)");
        }

        #[test]
        fn r1848000_a_change_of_control_keeps_it() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [ZERO, STEAL, FILLER], "mana": 10, "library": filler(4) },
                "p2": { "hand": [FILLER], "field": [VANILLA], "library": filler(4) },
            }));
            let target = s.unit_at(P2, 1);
            s.play(ZERO, json!({ "targets": pick::instance(&target) }));

            s.play(STEAL, json!({ "targets": pick::instance(&target) }));

            assert_eq!(s.card(&target).controller, P1);
            assert!(forever(&s, &target));
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert!(forever(&s, &target));
            assert!(!acts(&s, &target));
        }

        #[test]
        fn r1848000_an_ordinary_freeze_after_changes_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [ZERO, BOB, FILLER], "mana": 10, "library": filler(4) },
                "p2": { "hand": [FILLER], "field": [VANILLA, VANILLA], "library": filler(4) },
            }));
            let target = s.unit_at(P2, 1);
            let other = s.unit_at(P2, 2);
            s.play(ZERO, json!({ "targets": pick::instance(&target) }));

            s.play(BOB, json!({ "modes": ["freeze"] }));

            assert!(forever(&s, &target), "the later thaw is kept");
            let ordinary = s.card(&other).frozen;
            assert!(ordinary.is_some_and(|thaw| thaw != FROZEN_FOREVER));
            for _ in 0..4 {
                s.end_turn();
            }
            assert_eq!(s.card(&other).frozen, None);
            assert!(forever(&s, &target));
        }
    }

    mod radiant {
        use super::*;

        fn radiant_zero_at_a_vanilla(mode: &str) -> (Scenario, CardInstance) {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": ZERO, "radiant": true }, HIT_JOB, METEOR, FILLER],
                    "mana": 10,
                },
                "p2": { "hand": [FILLER], "field": [VANILLA] },
            }));
            let target = s.unit_at(P2, 1);
            s.play(ZERO, json!({ "targets": pick::instance(&target), "modes": [mode] }));
            (s, target)
        }

        #[test]
        fn the_give_mode_makes_it_indestructible() {
            let (mut s, target) = radiant_zero_at_a_vanilla(GIVE);
            assert!(forever(&s, &target));
            assert!(
                s.stats(&target)
                    .keywords
                    .iter()
                    .any(|keyword| keyword.kind() == KeywordKind::Indestructible)
            );

            s.play(HIT_JOB, json!({ "targets": pick::instance(&target) }));
            s.play(METEOR, json!({ "targets": pick::instance(&target) }));

            s.expect_in_zone(&target, "field");
            assert_eq!(s.card(&target).damage, 0, "a destroy and damage pass it by");
            assert!(forever(&s, &target));
        }

        #[test]
        fn freeze_only_leaves_it_as_the_base_face_does() {
            let (mut s, target) = radiant_zero_at_a_vanilla(FREEZE_ONLY);
            assert!(forever(&s, &target));

            s.play(HIT_JOB, json!({ "targets": pick::instance(&target) }));

            s.expect_in_zone(&target, "graveyard");
        }
    }
}
