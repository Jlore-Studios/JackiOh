//! C #78 Mutate Spell (SPEC §8.6 row 78). (1) Field Spell (R402: the designer wrote Spell), Rare.
//!   Base:    "Activate: Remove a Plague Counter from a permanent. If it's an enemy permanent, exile it. If
//!            it's your backrow card, draw {draw}. If it's your Unit, it attacks a random enemy
//!            {attacks|time|times}." — draw 2, 1 attack
//!   Radiant: "Activate ♾️: …" the same text — draw 4, 2 attacks
//!   Engine:  "Activate (once a turn) and Activate ♾️ (§6.2, R81, R384; R402 target in action): remove a
//!            Plague Counter. Forced attack (R53, §4.2); the Radiant's second attack happens only if the
//!            unit is still on the field (R96). Tunes: draw 2 ↑; attacks 1 ↑."
//!   Patch v0.3.10 (#831): the base face activated without limit before, and the Radiant face fused an
//!   enemy permanent onto a card of yours; the Radiant face is now the old base face with doubled
//!   numbers.

use jackioh_engine::effects::{consume_plague, draw, exile, forced_attack_random, instance_of};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-078";

const CHOSEN: TargetSpec = TargetSpec::Chosen { index: None };

/// What the chosen permanent's side and row make of the removed Plague Counter, read as the activation
/// runs: an enemy permanent is exiled; your backrow card draws; your Unit attacks a random enemy.
fn outcome(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(card) = instance_of(&*ctx, &CHOSEN) else {
        return vec![];
    };
    let Zone::Field { row, .. } = card.zone else {
        return vec![];
    };
    if card.controller != ctx.controller {
        return vec![exile(json_as(json!({ "target": CHOSEN })))];
    }
    if row == Row::Backrow {
        return vec![draw(json_as(json!({ "count": param(&*ctx, "draw") })))];
    }
    vec![forced_attack_random(json_as(
        json!({ "attacker": CHOSEN, "times": param(&*ctx, "attacks") }),
    ))]
}

/// Patch v0.3.10 (#831): the base face activates once a turn, and the Radiant face, which fused an enemy
/// permanent before, runs the base face's text with Activate ♾️.
fn mutate(uses: ActivationUses) -> Script {
    let ability = ActivationDecl {
        id: "mutate".into(),
        label: "Remove a Plague Counter".into(),
        uses,
        cost: None,
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "any", "of": ["unit", "backrow"], "plague": true }),
        )],
        modes: vec![],
        can_activate: Some(condition_hook(|c| {
            permanents_on_field(c.state, c.controller).iter().any(|card| plague_on(card) > 0)
        })),
        has: None,
        run: hook(|ctx| {
            let mut effects = vec![consume_plague(json_as(json!({ "target": CHOSEN })))];
            effects.extend(outcome(ctx));
            effects
        }),
    };
    Script {
        activations: vec![ability],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: mutate(ActivationUses::Count(1)),
        radiant: mutate(ActivationUses::Unlimited),
    }
}

// SPEC §8.6 row 78: Field Spell (R402); Activate (Radiant ♾️, R384) removes Plague token:
// enemy exiled; backrow draws 2 (Radiant 4); Unit forced-attacks random enemy (R53; R177, R386).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;
    use std::collections::BTreeSet;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MUTATE: &str = "classic-078";
    const CRAWLER: &str = "classic-053"; // (1) Unit: whenever Plague Counters are placed on this, draw 1.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt; Radiant adds Immutable.
    const TIMMY: &str = "core-011"; // (1) Unit 3/3 Rush, First Strike.
    const PAWN: &str = "core-096"; // (1) Trap: answers only an attack that would be lethal.
    const MANA_WELL: &str = "core-006"; // (3) Field Spell.
    const FILLER: &str = "core-005"; // (1) Spell (§2.5).
    const X: &str = "core-020"; // library filler.

    use crate::scenario;

    fn def() -> CardDef {
        crate::register_all();
        crate::card_def(ID)
    }

    use crate::js;

    fn lib(n: usize) -> Vec<&'static str> {
        vec![X; n]
    }

    fn at(card: &CardInstance) -> Value {
        json!([{ "pick": "instance", "instanceId": card.id }])
    }

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(found) => found,
            None => panic!("missing: {what}"),
        }
    }

    fn draws_by(events: &[GameEvent], player: PlayerId) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Drawn { player: p, .. } if *p == player))
            .count()
    }

    fn attacks_by(events: &[GameEvent], attacker: &CardInstance) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::AttackDeclared { attacker_id, target_id, .. } if *attacker_id == attacker.id => {
                    Some(target_id.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// The targets legalActions offers for Mutate Spell's activation, each once.
    fn offered(s: &Scenario) -> Vec<String> {
        let mutate = s.card(MUTATE).id.clone();
        let ids: Vec<String> = legal_actions(s.state(), P1)
            .iter()
            .map(js)
            .filter(|action| action["type"] == json!("activate") && action["instanceId"] == json!(mutate))
            .flat_map(|action| {
                action["targets"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|target| target["pick"] == json!("instance"))
                    .filter_map(|target| target["instanceId"].as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .collect();
        ids.into_iter().collect::<IndexSet<String>>().into_iter().collect()
    }

    /// R402 is a Field Spell with one Activate ability (♾️ on the Radiant face), its target a tokened
    /// permanent, and two numbers
    #[test]
    fn r402_is_a_field_spell_with_one_activate_ability_its_target_a_tokened_permanent_and_two_numbers() {
        assert_eq!(def().id, MUTATE);
        assert_eq!(js(&def().type_), json!("Field Spell"));
        assert_eq!(
            js(&def().params),
            json!([
                { "key": "draw", "base": 2, "radiant": 4, "better": "up", "step": 1, "min": 1 },
                { "key": "attacks", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 }
            ])
        );
        let scripts = script();
        assert_eq!(js(&scripts.base.activations[0].uses), json!(1));
        assert_eq!(js(&scripts.radiant.activations[0].uses), json!("unlimited"));
        for face in [&scripts.base, &scripts.radiant] {
            assert_eq!(face.activations.len(), 1);
            assert_eq!(
                js(&face.activations[0].targets),
                json!([
                    { "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "backrow"], "plague": true } }
                ])
            );
        }
    }

    /// base
    mod base {
        use super::*;

        /// R402 an enemy permanent: its token is removed, then it is exiled
        #[test]
        fn r402_an_enemy_permanent_its_token_is_removed_then_it_is_exiled() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [MUTATE] },
                "p2": { "hand": [FILLER], "field": [{ "def": VANILLA, "counters": { "plague": 2 } }] }
            }));
            let vanilla = s.card(VANILLA).clone();

            s.activate(MUTATE, json!({ "targets": at(&vanilla) }));

            s.expect_in_zone(&vanilla, "exile");
            let kinds: Vec<&str> = s
                .last_events()
                .iter()
                .filter(|event| {
                    matches!(
                        event,
                        GameEvent::Activated { .. } | GameEvent::CounterChanged { .. } | GameEvent::Exiled { .. }
                    )
                })
                .map(|event| event.event_type().as_str())
                .collect();
            assert_eq!(kinds, ["activated", "counterChanged", "exiled"]);
            s.expect_in_zone(MUTATE, "field");
        }

        /// R177 a face-down enemy trap with a token is offered by its id alone, and is exiled
        #[test]
        fn r177_a_face_down_enemy_trap_with_a_token_is_offered_by_its_id_alone_and_is_exiled() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [MUTATE] },
                "p2": { "hand": [FILLER], "backrow": [{ "def": PAWN, "faceUp": false, "counters": { "plague": 1 } }] }
            }));
            let pawn = s.card(PAWN).clone();

            assert_eq!(offered(&s), vec![pawn.id.clone()]);
            assert!(!serde_json::to_string(&s.view(P1)).expect("serialisable").contains(PAWN));
            s.activate(MUTATE, json!({ "targets": at(&pawn) }));

            s.expect_in_zone(&pawn, "exile");
        }

        /// R402 your backrow card draws 2: another Field Spell, or Mutate Spell itself
        #[test]
        fn r402_your_backrow_card_draws_2_another_field_spell_or_mutate_spell_itself() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [FILLER],
                    "backrow": [
                        { "def": MUTATE, "counters": { "plague": 1 } },
                        { "def": MANA_WELL, "lane": 2, "counters": { "plague": 1 } }
                    ],
                    "library": lib(5)
                },
                "p2": { "hand": [FILLER] }
            }));

            let well = s.card(MANA_WELL).clone();
            s.activate(MUTATE, json!({ "targets": at(&well) }));
            assert_eq!(draws_by(s.last_events(), P1), 2);
            assert_eq!(s.card(MANA_WELL).counters.plague, None);
            s.expect_in_zone(MANA_WELL, "field");

            let mut t = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [{ "def": MUTATE, "counters": { "plague": 1 } }], "library": lib(5) },
                "p2": { "hand": [FILLER] }
            }));
            let mutate = t.card(MUTATE).clone();
            t.activate(MUTATE, json!({ "targets": at(&mutate) }));
            assert_eq!(draws_by(t.last_events(), P1), 2);
        }

        /// R53 your Unit makes one forced attack on a random enemy, spending no exertion
        #[test]
        fn r53_your_unit_makes_one_forced_attack_on_a_random_enemy_spending_no_exertion() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [{ "def": VANILLA, "counters": { "plague": 1 } }], "backrow": [MUTATE] },
                "p2": { "hand": [FILLER], "health": 30 }
            }));
            let vanilla = s.card(VANILLA).clone();

            s.activate(MUTATE, json!({ "targets": at(&vanilla) }));

            assert_eq!(attacks_by(s.last_events(), &vanilla), ["hero-p2"]);
            s.expect_health(P2, 26);
            // No exertion spent: it may still attack this turn.
            s.attack(&vanilla, "hero");
            s.expect_health(P2, 22);
        }

        /// R53 the random enemy is a hero or a Unit it may attack, Taunt and position ignored
        #[test]
        fn r53_the_random_enemy_is_a_hero_or_a_unit_it_may_attack_taunt_and_position_ignored() {
            let mut seen: BTreeSet<&str> = BTreeSet::new();
            let mut seed = 0;
            while seed < 12 && seen.len() < 2 {
                let mut s = scenario(json!({
                    "seed": format!("mutate-random-{seed}"),
                    "p1": { "hand": [FILLER], "field": [{ "def": MENACE, "counters": { "plague": 1 } }], "backrow": [MUTATE] },
                    "p2": { "hand": [FILLER], "field": [{ "def": VANILLA, "position": "DEF" }], "health": 30 }
                }));
                let menace = s.card(MENACE).clone();
                s.activate(MUTATE, json!({ "targets": at(&menace) }));
                let targets = attacks_by(s.last_events(), &menace);
                assert_eq!(targets.len(), 1);
                seen.insert(if targets[0] == "hero-p2" { "hero" } else { "unit" });
                seed += 1;
            }
            assert_eq!(seen, BTreeSet::from(["hero", "unit"]));
        }

        /// a removal is no placement: a Plague Crawler targeted draws nothing
        #[test]
        fn a_removal_is_no_placement_a_plague_crawler_targeted_draws_nothing() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [FILLER],
                    "field": [{ "def": CRAWLER, "counters": { "plague": 1 } }],
                    "backrow": [MUTATE],
                    "library": lib(3)
                },
                "p2": { "hand": [FILLER] }
            }));

            let crawler = s.card(CRAWLER).clone();
            s.activate(MUTATE, json!({ "targets": at(&crawler) }));

            assert_eq!(draws_by(s.last_events(), P1), 0);
            let changes: Vec<Value> = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::CounterChanged { .. }))
                .map(js)
                .collect();
            assert_eq!(
                json!(changes),
                json!([{ "type": "counterChanged", "instanceId": s.card(CRAWLER).id, "counter": "plague", "value": 0 }])
            );
        }

        /// R384 with no tokened permanent it can't activate, and a clean target is refused
        #[test]
        fn r384_with_no_tokened_permanent_it_can_t_activate_and_a_clean_target_is_refused() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [VANILLA], "backrow": [MUTATE] },
                "p2": { "hand": [FILLER], "field": [MENACE] }
            }));

            assert!(!legal_actions(s.state(), P1).iter().any(|action| js(action)["type"] == json!("activate")));
            let menace = s.card(MENACE).clone();
            s.expect_refused(|s| s.activate(MUTATE, json!({ "targets": at(&menace) })));
            let view = js(&s.view(P1))["you"]["backrow"][0].clone();
            assert!(serde_json::to_string(&view).expect("serialisable").contains("\"usable\":false"));

            let mut t = scenario(json!({
                "p1": { "hand": [FILLER], "field": [{ "def": VANILLA, "counters": { "plague": 1 } }], "backrow": [MUTATE] },
                "p2": { "hand": [FILLER], "field": [MENACE] }
            }));
            assert_eq!(offered(&t), vec![t.card(VANILLA).id.clone()]);
            let menace = t.card(MENACE).clone();
            t.expect_refused(|t| t.activate(MUTATE, json!({ "targets": at(&menace) })));
        }

        /// §3.2 R13 a card dormant under a Stack pile is not offered, even with tokens
        #[test]
        fn s3_2_r13_a_card_dormant_under_a_stack_pile_is_not_offered_even_with_tokens() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [MUTATE] },
                "p2": {
                    "hand": [FILLER],
                    "field": [{ "def": VANILLA, "counters": { "plague": 2 } }, { "def": "core-092", "stack": true }]
                }
            }));

            assert!(offered(&s).is_empty());
            let vanilla = s.card(VANILLA).clone();
            s.expect_refused(|s| s.activate(MUTATE, json!({ "targets": at(&vanilla) })));
        }

        /// R384 once a turn: a second use that turn is refused, and it is not a play
        #[test]
        fn r384_once_a_turn_a_second_use_is_refused_and_it_is_not_a_play() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER, FILLER], "backrow": [MUTATE], "library": lib(2) },
                "p2": {
                    "hand": [FILLER],
                    "field": [
                        { "def": VANILLA, "counters": { "plague": 1 } },
                        { "def": MENACE, "lane": 2, "counters": { "plague": 1 } }
                    ],
                    "library": lib(2)
                }
            }));
            let played = s.state().players.p1.turn_log.cards_played;
            let vanilla = s.card(VANILLA).clone();
            let menace = s.card(MENACE).clone();

            s.activate(MUTATE, json!({ "targets": at(&vanilla) }));
            s.expect_in_zone(&vanilla, "exile");
            assert!(!legal_actions(s.state(), P1).iter().any(|action| js(action)["type"] == json!("activate")));
            s.expect_refused(|s| s.activate(MUTATE, json!({ "targets": at(&menace) })));
            assert!(!s.events().iter().any(|event| matches!(event, GameEvent::CardPlayed { .. })));
            assert_eq!(s.state().players.p1.turn_log.cards_played, played);

            s.end_turn();
            s.end_turn();
            s.activate(MUTATE, json!({ "targets": at(&menace) }));
            s.expect_in_zone(&menace, "exile");
        }

        /// R384 only its controller, in their own main phase
        #[test]
        fn r384_only_its_controller_in_their_own_main_phase() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [FILLER], "backrow": [MUTATE] },
                "p2": { "hand": [FILLER], "field": [{ "def": VANILLA, "counters": { "plague": 1 } }] }
            }));

            let vanilla = s.card(VANILLA).clone();
            s.expect_refused_with(|s| s.activate(MUTATE, json!({ "targets": at(&vanilla) })), "not your turn");
        }

        /// §2.4 a full hand burns the draws
        #[test]
        fn s2_4_a_full_hand_burns_the_draws() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": vec![FILLER; 10],
                    "backrow": [{ "def": MUTATE, "counters": { "plague": 1 } }],
                    "library": lib(3)
                },
                "p2": { "hand": [FILLER] }
            }));

            let mutate = s.card(MUTATE).clone();
            s.activate(MUTATE, json!({ "targets": at(&mutate) }));

            assert_eq!(
                s.last_events().iter().filter(|event| matches!(event, GameEvent::Burned { .. })).count(),
                2
            );
        }

        /// R386 an Upgrade draws 3 and attacks twice
        #[test]
        fn r386_an_upgrade_draws_3_and_attacks_twice() {
            let upgraded = || {
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [FILLER],
                        "field": [{ "def": VANILLA, "counters": { "plague": 1 } }],
                        "backrow": [{ "def": MUTATE, "counters": { "plague": 1 } }],
                        "library": lib(4)
                    },
                    "p2": { "hand": [FILLER], "health": 30 }
                }));
                step_param(s.card_mut(MUTATE), "draw", 1);
                step_param(s.card_mut(MUTATE), "attacks", 1);
                s
            };

            let mut s = upgraded();
            let mutate = s.card(MUTATE).clone();
            s.activate(MUTATE, json!({ "targets": at(&mutate) }));
            assert_eq!(draws_by(s.last_events(), P1), 3);

            let mut t = upgraded();
            let vanilla = t.card(VANILLA).clone();
            t.activate(MUTATE, json!({ "targets": at(&vanilla) }));
            assert_eq!(attacks_by(t.last_events(), t.card(VANILLA)), ["hero-p2", "hero-p2"]);
            t.expect_health(P2, 22);
        }
    }

    /// radiant
    mod radiant {
        use super::*;

        /// R402 an enemy permanent is exiled, as on the base face (it was fused before patch v0.3.10)
        #[test]
        fn r402_an_enemy_permanent_is_exiled_with_no_prompt() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER, TIMMY], "field": [MENACE], "backrow": [{ "def": MUTATE, "radiant": true }] },
                "p2": { "hand": [FILLER], "field": [{ "def": VANILLA, "counters": { "plague": 1 } }] }
            }));
            let vanilla = s.card(VANILLA).clone();

            s.activate(MUTATE, json!({ "targets": at(&vanilla) }));

            assert!(s.state().pending.is_none());
            s.expect_in_zone(&vanilla, "exile");
            assert!(!s.last_events().iter().any(|event| matches!(event, GameEvent::Fused { .. })));
        }

        /// R384 ♾️: again and again in one turn while tokens remain, and not a play
        #[test]
        fn r384_r402_again_and_again_in_one_turn_while_tokens_remain_and_not_a_play() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [{ "def": MUTATE, "radiant": true }] },
                "p2": {
                    "hand": [FILLER],
                    "field": [
                        { "def": VANILLA, "counters": { "plague": 1 } },
                        { "def": MENACE, "lane": 2, "counters": { "plague": 1 } },
                        { "def": TIMMY, "lane": 3, "counters": { "plague": 1 } }
                    ]
                }
            }));
            let played = s.state().players.p1.turn_log.cards_played;

            let cards = [s.card(VANILLA).clone(), s.card(MENACE).clone(), s.card(TIMMY).clone()];
            for card in &cards {
                s.activate(MUTATE, json!({ "targets": at(card) }));
            }

            assert_eq!(s.pile(P2, "exile").len(), 3);
            assert_eq!(
                s.events().iter().filter(|event| matches!(event, GameEvent::Activated { .. })).count(),
                3
            );
            assert!(!s.events().iter().any(|event| matches!(event, GameEvent::CardPlayed { .. })));
            assert_eq!(s.state().players.p1.turn_log.cards_played, played);
        }

        /// R384 at most ACTIVATE_UNLIMITED_CAP uses per turn
        #[test]
        fn r384_at_most_activate_unlimited_cap_uses_per_turn() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [{ "def": MUTATE, "radiant": true }] },
                "p2": { "hand": [FILLER], "field": [{ "def": VANILLA, "counters": { "plague": 1 } }] }
            }));
            let mutate = s.card(MUTATE).id.clone();
            let turn = s.state().turn;
            must(find_instance_mut(s.state_mut(), &mutate), "Mutate Spell").memory.insert(
                subsystems::ACTIVATIONS_MEMORY_KEY.to_string(),
                json!({ "turn": turn, "count": ACTIVATE_UNLIMITED_CAP }),
            );

            assert!(!legal_actions(s.state(), P1).iter().any(|action| js(action)["type"] == json!("activate")));
            let vanilla = s.card(VANILLA).clone();
            s.expect_refused_with(|s| s.activate(MUTATE, json!({ "targets": at(&vanilla) })), "100 times");
        }

        /// your backrow card draws 4
        #[test]
        fn your_backrow_card_draws_4() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [{ "def": MUTATE, "radiant": true, "counters": { "plague": 1 } }], "library": lib(5) },
                "p2": { "hand": [FILLER] }
            }));

            let mutate = s.card(MUTATE).clone();
            s.activate(MUTATE, json!({ "targets": at(&mutate) }));

            assert_eq!(draws_by(s.last_events(), P1), 4);
        }

        /// R53 your Unit attacks a random enemy twice when it survives the first
        #[test]
        fn r53_your_unit_attacks_a_random_enemy_twice_when_it_survives_the_first() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [{ "def": VANILLA, "counters": { "plague": 1 } }], "backrow": [{ "def": MUTATE, "radiant": true }] },
                "p2": { "hand": [FILLER], "health": 30 }
            }));
            let vanilla = s.card(VANILLA).clone();

            s.activate(MUTATE, json!({ "targets": at(&vanilla) }));

            assert_eq!(attacks_by(s.last_events(), &vanilla), ["hero-p2", "hero-p2"]);
            s.expect_health(P2, 22);
        }

        /// R96 the second attack happens only if it survived the first
        #[test]
        fn r96_the_second_attack_happens_only_if_it_survived_the_first() {
            let mut died_first = 0;
            for seed in 0..16 {
                let mut s = scenario(json!({
                    "seed": format!("mutate-twice-{seed}"),
                    "p1": { "hand": [FILLER], "field": [{ "def": TIMMY, "counters": { "plague": 1 } }], "backrow": [{ "def": MUTATE, "radiant": true }] },
                    "p2": { "hand": [FILLER], "field": [MENACE], "health": 30 }
                }));
                let timmy = s.card(TIMMY).clone();
                s.activate(MUTATE, json!({ "targets": at(&timmy) }));
                let targets = attacks_by(s.last_events(), &timmy);
                if targets.first().map(String::as_str) != Some("hero-p2") {
                    died_first += 1;
                    assert_eq!(targets.len(), 1, "seed {seed}");
                    s.expect_in_zone(&timmy, "graveyard");
                } else {
                    assert!(!targets.is_empty(), "seed {seed}");
                    assert!(targets.len() <= 2, "seed {seed}");
                }
            }
            assert!(died_first > 0);
        }

        /// R386 a Degrade draws 3 and attacks once
        #[test]
        fn r386_a_degrade_draws_3_and_attacks_once() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [FILLER],
                    "field": [{ "def": VANILLA, "counters": { "plague": 1 } }],
                    "backrow": [{ "def": MUTATE, "radiant": true, "counters": { "plague": 1 } }],
                    "library": lib(4)
                },
                "p2": { "hand": [FILLER], "health": 30 }
            }));
            step_param(s.card_mut(MUTATE), "draw", -1);
            step_param(s.card_mut(MUTATE), "attacks", -1);

            // Activate ♾️: both in one turn.
            let mutate = s.card(MUTATE).clone();
            s.activate(MUTATE, json!({ "targets": at(&mutate) }));
            assert_eq!(draws_by(s.last_events(), P1), 3);
            let vanilla = s.card(VANILLA).clone();
            s.activate(MUTATE, json!({ "targets": at(&vanilla) }));
            assert_eq!(attacks_by(s.last_events(), s.card(VANILLA)), ["hero-p2"]);
        }
    }
}
