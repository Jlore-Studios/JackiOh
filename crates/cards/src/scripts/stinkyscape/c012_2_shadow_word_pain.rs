//! S #12.2 Shadow Word Pain (SPEC §8.10 row S 12.2; §10.4, R703, R1867005): (0) Spell,
//! Other-Origins, Token (printed Epic).
//!   Base:    "Destroy a Unit with {attack} or less Attack." — 3.
//!   Radiant: "Destroy a Unit with {attack} or less Attack, and the Units adjacent to it." — 3.
//!
//! Engine: the mirror of S #12.1 Shadow Word Death with a ceiling: a declared target, any Unit
//! whose Attack, read through the layers (§10.4), is the declared number or less (R81) — the same
//! `target_checks` shape with `<=`, the pick needed (R703). `destroy` on the chosen Unit. Radiant:
//! `destroy_adjacent_to` as well, the Units beside it whatever their Attack (R59).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-012-2";

/// The target Checks rule: a Unit with the declared Attack or less.
const THRESHOLD: &str = "attackAtMost";

/// R81: any Unit on either side, picked with the play — needed (R703), so with no Unit to choose
/// the play is refused and `legalActions` never offers it.
fn targets() -> Vec<TargetDecl> {
    let mut decl =
        TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"], "check": THRESHOLD }));
    decl.required = Some(true);
    vec![decl]
}

/// The declared Attack or less, through the layers.
fn meets_threshold(args: TargetCheckArgs<'_>) -> bool {
    let Some(candidate) = args.candidate else {
        return false;
    };
    let limit = HookArgs { state: args.state, self_: args.self_, radiant: args.radiant };
    unit_view(args.state, candidate).attack <= param(&limit, "attack")
}

fn shadow_word_pain(radiant: bool) -> Script {
    Script {
        targets: targets(),
        target_checks: IndexMap::from([(THRESHOLD, target_check(meets_threshold))]),
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
        base: shadow_word_pain(false),
        radiant: shadow_word_pain(true),
    }
}

// S #12.2 Shadow Word Pain — SPEC §8.10 row S 12.2, BUILD M12 row S 12.2 (R1867005).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P2: PlayerId = PlayerId::P2;

    const PAIN: &str = "stinkyscape-012-2";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9.
    const FILLER: &str = "core-005";

    fn stats_override(attack: i32, health: i32) -> Value {
        json!({ "attack": attack, "health": health })
    }

    /// This thread's script for `def_id`, for one test: after each card its controller's opponent
    /// plays, it gains `attack` Attack (a negative one lowers it), before that card resolves (§10.5).
    fn moves_on_enemy_plays(def_id: &str, attack: i32) {
        let moves = TriggerDef::new("swd-moves", &[GameEventType::CardPlayed], move |ctx, event| match event {
            GameEvent::CardPlayed { player, .. } if *player != ctx.controller => vec![effects::buff(json_as(
                json!({ "target": { "of": "self" }, "attack": attack }),
            ))],
            _ => vec![],
        });
        let script = Script {
            triggers: vec![moves],
            ..Script::default()
        };
        let mut scripts = registered_scripts();
        scripts.insert(
            def_id.to_string(),
            CardScripts {
                base: script.clone(),
                radiant: script,
            },
        );
        register_scripts(scripts);
    }

    mod base {
        use super::*;

        #[test]
        fn r1867005_destroys_a_unit_with_3_attack_or_less_a_0_attack_one_included() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [PAIN, PAIN, FILLER] },
                "p2": {
                    "hand": [FILLER],
                    "field": [
                        { "def": VANILLA, "lane": 1, "statsOverride": stats_override(0, 4) },
                        { "def": VANILLA, "lane": 2, "statsOverride": stats_override(3, 4) },
                    ],
                },
            }));
            let zero = s.unit_at(P2, 1);
            let three = s.unit_at(P2, 2);
            let first = s.card(PAIN).id.clone();

            s.play(&first, json!({ "targets": pick::instance(&zero) }));
            s.expect_in_zone(&zero, "graveyard");

            let second = s.card(PAIN).id.clone();
            s.play(&second, json!({ "targets": pick::instance(&three) }));
            s.expect_in_zone(&three, "graveyard");
        }

        /// R1867005, R1868000: the limit is checked with the play and not again as the Spell resolves,
        /// so a target raised from 3 Attack to 5 between the two still dies.
        #[test]
        fn r1867005_a_target_raised_past_the_limit_after_the_pick_still_dies() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [PAIN, FILLER], "mana": 9 },
                "p2": {
                    "hand": [FILLER],
                    "field": [{ "def": VANILLA, "lane": 1, "statsOverride": stats_override(3, 5) }],
                },
            }));
            moves_on_enemy_plays(VANILLA, 2);
            let target = s.unit_at(P2, 1);

            s.play(PAIN, json!({ "targets": pick::instance(&target) }));

            assert!(s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::Buffed { instance_id, attack: 2, .. } if *instance_id == target.id
            )));
            s.expect_in_zone(&target, "graveyard");
        }

        #[test]
        fn r1867005_a_4_attack_unit_cannot_be_chosen_the_play_is_refused() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [PAIN, FILLER] },
                "p2": { "hand": [FILLER], "field": [VANILLA] },
            }));

            s.expect_refused(|s| s.play(PAIN, json!({ "targets": pick::instance(&s.unit_at(P2, 1)) })));
            s.expect_in_zone(PAIN, "hand");
        }

        #[test]
        fn r386_an_upgrade_of_the_ceiling_to_4_admits_a_4_attack_unit() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [PAIN, FILLER] },
                "p2": { "hand": [FILLER], "field": [VANILLA] },
            }));
            step_param(s.card_mut(PAIN), "attack", 1);
            let target = s.unit_at(P2, 1);

            s.play(PAIN, json!({ "targets": pick::instance(&target) }));

            s.expect_in_zone(&target, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1867005_the_neighbours_die_too_a_9_attack_one_included() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": PAIN, "radiant": true }, FILLER] },
                "p2": {
                    "hand": [FILLER],
                    "field": [
                        { "def": MENACE, "lane": 1 },
                        { "def": VANILLA, "lane": 2, "statsOverride": stats_override(3, 4) },
                        { "def": VANILLA, "lane": 3, "statsOverride": stats_override(0, 2) },
                    ],
                },
            }));
            let left = s.unit_at(P2, 1);
            let target = s.unit_at(P2, 2);
            let right = s.unit_at(P2, 3);

            s.play(PAIN, json!({ "targets": pick::instance(&target) }));

            s.expect_in_zone(&target, "graveyard");
            s.expect_in_zone(&left, "graveyard");
            s.expect_in_zone(&right, "graveyard");
        }
    }
}
