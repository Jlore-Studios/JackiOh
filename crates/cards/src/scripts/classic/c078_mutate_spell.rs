//! C #78 Mutate Spell (SPEC §8.6 row 78). (1) Field Spell (R402: the designer wrote Spell), Rare.
//!   Base:    "Activate ♾️: Remove a Plague Counter from a permanent. If it's an enemy permanent, exile it. If
//!            it's your backrow card, draw {draw}. If it's your Unit, it attacks a random enemy
//!            {attacks|time|times}." — draw 2, 1 attack
//!   Radiant: "… If it's an enemy permanent, fuse it onto a card of yours of its type on your field, in your
//!            hand or in your deck, or exile it if you have none. …" — draw 4, 2 attacks
//!   Engine:  "Activate ♾️ (§6.2, R384) with a declared target (a permanent with a Plague Counter, either
//!            side), bounded by `ACTIVATE_UNLIMITED_CAP` and by the tokens on the field; removing the token
//!            is the ability's cost. "Attacks a random enemy" is a forced attack (R53) on a random enemy,
//!            hero or unit, drawn from the targets it may attack (§4.2); the Radiant's second attack is its
//!            own combat and happens only if the unit is still on the field (R96). The Radiant's fuse is
//!            Fuse (§6.3, R77, R102; the enemy card ceases to exist) onto the card you choose … an
//!            Immutable card of yours is not offered (R23). Tunes: draw 2 ↑; attacks 1 ↑."
//!
//! R402: the target travels in the `activate` action (R81, R384); with no tokened permanent on the field
//! the ability can't be activated. The branch is read off the target as it was activated: whose it is,
//! and its row. The forced attacks are `forcedAttackRandom`; the Radiant fuse is `fuseOntoYourCard`, whose
//! prompt offers your cards of its type (the deck's to you alone) and exiles the card when there is none.

use jackioh_engine::effects::{
    consume_plague, draw, exile, forced_attack_random, fuse_onto_your_card, instance_of,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-078";

const CHOSEN: TargetSpec = TargetSpec::Chosen { index: None };

fn outcome(ctx: &mut EffectContext<'_>, fuses: bool) -> Vec<Effect> {
    let Some(card) = instance_of(&*ctx, &CHOSEN) else {
        return vec![];
    };
    let Zone::Field { row, .. } = card.zone else {
        return vec![];
    };
    if card.controller != ctx.controller {
        return vec![if fuses {
            fuse_onto_your_card(json_as(json!({ "target": CHOSEN })))
        } else {
            exile(json_as(json!({ "target": CHOSEN })))
        }];
    }
    if row == Row::Backrow {
        return vec![draw(json_as(json!({ "count": param(&*ctx, "draw") })))];
    }
    vec![forced_attack_random(json_as(
        json!({ "attacker": CHOSEN, "times": param(&*ctx, "attacks") }),
    ))]
}

fn mutate(fuses: bool) -> Script {
    let ability = ActivationDecl {
        id: "mutate".into(),
        label: "Remove a Plague Counter".into(),
        uses: ActivationUses::Unlimited,
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
        run: hook(move |ctx| {
            let mut effects = vec![consume_plague(json_as(json!({ "target": CHOSEN })))];
            effects.extend(outcome(ctx, fuses));
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
        base: mutate(false),
        radiant: mutate(true),
    }
}

// C #78 Mutate Spell — SPEC §8.6 row 78, BUILD M9 Classic row C 78: "A Field Spell (R402); Activate ♾️
// (R384), the target carried in the `activate` action: a permanent with a Plague Counter, either side,
// face-down included (the option carries only its id, R177); remove one token, then: an enemy permanent is
// exiled; your backrow card, itself included, draws 2; your Unit makes one forced attack (R53) on a random
// enemy it may attack, hero or Unit, spending no exertion; with no tokened permanent it can't activate; at
// most `ACTIVATE_UNLIMITED_CAP` uses per turn; not a play; radiant: an enemy permanent is fused (R77, R102)
// onto a card of yours of its type, picked in a prompt over your field, hand and deck, your Immutable
// cards never offered and the deck's cards shown to you only (§10.8), never named in the opponent's view;
// the enemy card ceases to exist; with no card of its type it is exiled; your backrow card draws 4; your
// Unit attacks a random enemy twice, the second only if it survived the first; its tuned numbers (draw,
// attacks) read through `param()` (R386)".
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
    const SHEEPISH: &str = "core-041"; // (1) Trap.
    const MANA_WELL: &str = "core-006"; // (3) Field Spell.
    const THRIVE: &str = "classic-081"; // (2) Field Spell: Activate: Choose one: heal, draw 1 (Radiant 2), or mana.
    const FILLER: &str = "core-005"; // (1) Spell (§2.5).
    const X: &str = "core-020"; // library filler.

    /// The harness, after the catalog and every card script are registered (TS's harness did it on import).
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    fn def() -> CardDef {
        crate::register_all();
        crate::card_def(ID)
    }

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialisable")
    }

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
            .map(|action| js(action))
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

    fn instance_options(s: &Scenario, what: &str) -> Vec<String> {
        must(s.state().pending.as_ref(), what)
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Instance { instance_id } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn option_keys(s: &Scenario, what: &str) -> Vec<String> {
        must(s.state().pending.as_ref(), what).options.iter().map(|option| option.key.clone()).collect()
    }

    /// R402 is a Field Spell with one Activate ♾️ ability, its target a tokened permanent, and two numbers
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
        for face in [&scripts.base, &scripts.radiant] {
            assert_eq!(face.activations.len(), 1);
            assert_eq!(js(&face.activations[0].uses), json!("unlimited"));
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

            let mutate = s.card(MUTATE).clone();
            s.activate(MUTATE, json!({ "targets": at(&mutate) }));
            assert_eq!(draws_by(s.last_events(), P1), 2);
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
                .map(|event| js(event))
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

        /// R384 ♾️: again and again in one turn while tokens remain, and not a play
        #[test]
        fn r384_again_and_again_in_one_turn_while_tokens_remain_and_not_a_play() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [MUTATE] },
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
                "p1": { "hand": [FILLER], "backrow": [MUTATE] },
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

            let mutate = s.card(MUTATE).clone();
            s.activate(MUTATE, json!({ "targets": at(&mutate) }));
            assert_eq!(draws_by(s.last_events(), P1), 3);
            let vanilla = s.card(VANILLA).clone();
            s.activate(MUTATE, json!({ "targets": at(&vanilla) }));
            assert_eq!(attacks_by(s.last_events(), s.card(VANILLA)), ["hero-p2", "hero-p2"]);
            s.expect_health(P2, 22);
        }
    }

    /// radiant
    mod radiant {
        use super::*;

        /// `extra` holds the optional `hand`, `field` and `library` lists TS's `fusing` took.
        fn fusing(extra: Value) -> Scenario {
            let list = |key: &str| extra[key].as_array().cloned().unwrap_or_default();
            let mut hand = vec![json!(FILLER)];
            hand.extend(list("hand"));
            scenario(json!({
                "p1": {
                    "hand": hand,
                    "field": list("field"),
                    "backrow": [{ "def": MUTATE, "radiant": true }],
                    "library": list("library")
                },
                "p2": {
                    "hand": [FILLER],
                    "field": [{ "def": VANILLA, "counters": { "plague": 1 } }],
                    "backrow": [{ "def": SHEEPISH, "faceUp": false, "counters": { "plague": 1 } }]
                }
            }))
        }

        /// R77 an enemy Unit is fused onto a Unit of yours you pick from your field, hand and deck; it ceases to exist
        #[test]
        fn r77_an_enemy_unit_is_fused_onto_a_unit_of_yours_you_pick_from_your_field_hand_and_deck_it_ceases_to_exist()
         {
            let mut s = fusing(json!({ "field": [TIMMY], "hand": [MENACE], "library": ["core-012", X] }));
            let enemy = s.card(VANILLA).clone();
            let timmy = s.card(TIMMY).clone();

            s.activate(MUTATE, json!({ "targets": at(&enemy) }));

            let prompt = must(s.state().pending.clone(), "the fuse prompt");
            assert_eq!(prompt.player_id, P1);
            let options: BTreeSet<String> = instance_options(&s, "the fuse prompt").into_iter().collect();
            let library: Vec<CardInstance> =
                s.pile(P1, "library").into_iter().filter(|card| card.def_id != FILLER).collect();
            let mut expected: BTreeSet<String> = [timmy.id.clone(), s.card(MENACE).id.clone()].into_iter().collect();
            expected.extend(library.iter().map(|card| card.id.clone()));
            assert_eq!(options, expected);
            s.answer(json!(timmy.id));

            s.expect_in_zone(&enemy, "gone");
            let fused = must(s.unit(P1, 1), "the fused card");
            assert_eq!(fused.id, timmy.id);
            assert_ne!(fused.def_id, TIMMY);
            assert!(s.last_events().iter().any(|event| matches!(event, GameEvent::Fused { .. })));
        }

        /// R23 your Immutable cards are never offered
        #[test]
        fn r23_your_immutable_cards_are_never_offered() {
            let mut s = fusing(json!({ "field": [{ "def": MENACE, "radiant": true }, TIMMY] }));

            let vanilla = s.card(VANILLA).clone();
            s.activate(MUTATE, json!({ "targets": at(&vanilla) }));

            assert_eq!(instance_options(&s, "the fuse prompt"), vec![s.card(TIMMY).id.clone()]);
        }

        /// §10.8 the deck's cards are shown to you only: the opponent's view names none of them
        #[test]
        fn s10_8_the_deck_s_cards_are_shown_to_you_only_the_opponent_s_view_names_none_of_them() {
            let mut s = fusing(json!({ "library": ["core-012", "core-015"] }));
            let enemy = s.card(VANILLA).clone();

            s.activate(MUTATE, json!({ "targets": at(&enemy) }));

            let view = js(&s.view(P1));
            let mine = &view["pending"];
            assert!(!mine.is_null(), "missing: p1's prompt");
            if mine["forYou"] != json!(true) {
                panic!("the prompt is p1's");
            }
            let mut def_ids: Vec<Option<String>> = mine["options"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|option| option["defId"].as_str().map(String::from))
                .collect();
            def_ids.sort();
            assert_eq!(def_ids, vec![Some("core-012".to_string()), Some("core-015".to_string())]);
            assert_eq!(js(&s.view(P2))["pending"], json!({ "forYou": false, "pendingFor": "p1" }));
            let theirs = serde_json::to_string(&s.view(P2)).expect("serialisable");
            assert!(!theirs.contains("core-012"));
            assert!(!theirs.contains("core-015"));

            let deck_card = must(
                s.pile(P1, "library").into_iter().find(|card| card.def_id == "core-015"),
                "the deck's core-015",
            );
            s.answer(json!(deck_card.id));

            assert!(!serde_json::to_string(&s.view(P2)).expect("serialisable").contains("core-015"));
            s.expect_in_zone(&enemy, "gone");
        }

        /// a face-down enemy Trap is fused onto a Trap of yours in hand (Trap and Field Trap are one type)
        #[test]
        fn a_face_down_enemy_trap_is_fused_onto_a_trap_of_yours_in_hand_trap_and_field_trap_are_one_type() {
            let mut s = fusing(json!({ "hand": [PAWN] }));
            let sheepish = s.card(SHEEPISH).clone();

            s.activate(MUTATE, json!({ "targets": at(&sheepish) }));
            let pawn = s.card(PAWN).clone();
            assert_eq!(option_keys(&s, "the fuse prompt"), vec![format!("instance:{}", pawn.id)]);
            s.answer(json!(pawn.id));

            s.expect_in_zone(&sheepish, "gone");
            assert!(s.hand(P1).iter().any(|card| card.id == pawn.id && card.def_id != PAWN));
            // §10.8 the hand card it went onto is never named to the opponent.
            assert!(!serde_json::to_string(&s.view(P2)).expect("serialisable").contains(PAWN));
            assert!(!serde_json::to_string(&s.view(P2)).expect("serialisable").contains("My Pawn"));
        }

        /// with no card of its type, it is exiled with no prompt
        #[test]
        fn with_no_card_of_its_type_it_is_exiled_with_no_prompt() {
            let mut s = fusing(json!({ "hand": [MANA_WELL] }));

            let vanilla = s.card(VANILLA).clone();
            s.activate(MUTATE, json!({ "targets": at(&vanilla) }));

            assert!(s.state().pending.is_none());
            s.expect_in_zone(VANILLA, "exile");
        }

        /// §9.3 the fuse prompt survives a JSON round trip and finishes as the live one does
        #[test]
        fn s9_3_the_fuse_prompt_survives_a_json_round_trip_and_finishes_as_the_live_one_does() {
            let mut s = fusing(json!({ "field": [TIMMY], "hand": [MENACE] }));
            let vanilla = s.card(VANILLA).clone();
            s.activate(MUTATE, json!({ "targets": at(&vanilla) }));

            let revived: GameState = serde_json::from_str(&serde_json::to_string(s.state()).expect("serialisable"))
                .expect("a state reads back");
            assert_eq!(&revived, s.state());
            let choice = must(revived.pending.clone(), "the fuse prompt");
            let menace = s.card(MENACE).clone();
            let action: Action = json_as(json!({
                "type": "answer",
                "playerId": "p1",
                "choiceId": choice.id,
                "selection": at(&menace),
                "nonce": "mutate-json"
            }));
            let result = reduce(&revived, &action);
            assert!(result.error.is_none());
            s.answer(json!(menace.id));

            assert_eq!(hash_state(&result.state), hash_state(s.state()));
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

        /// R102 fused onto itself, it still draws its own 4, not the 2 the fused Power to Thrive declares
        #[test]
        fn r102_fused_onto_itself_it_still_draws_its_own_4_not_the_2_the_fused_power_to_thrive_declares() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [{ "def": MUTATE, "radiant": true, "counters": { "plague": 1 } }], "library": lib(5) },
                "p2": { "hand": [FILLER], "backrow": [{ "def": THRIVE, "counters": { "plague": 1 } }] }
            }));
            let mutate = s.card(MUTATE).id.clone();
            let thrive = s.card(THRIVE).clone();
            s.activate(MUTATE, json!({ "targets": at(&thrive) }));
            // It is the only Field Spell you have, so the prompt offers it alone.
            assert_eq!(option_keys(&s, "the fuse prompt"), vec![format!("instance:{mutate}")]);
            s.answer(json!(mutate));
            let fused = s.card(&mutate).clone();
            assert_eq!(js(&fused_id_parts(&fused.def_id)), json!([THRIVE, MUTATE]));

            s.activate(&mutate, json!({ "ability": "mutate", "targets": at(&fused) }));

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

            let mutate = s.card(MUTATE).clone();
            s.activate(MUTATE, json!({ "targets": at(&mutate) }));
            assert_eq!(draws_by(s.last_events(), P1), 3);
            let vanilla = s.card(VANILLA).clone();
            s.activate(MUTATE, json!({ "targets": at(&vanilla) }));
            assert_eq!(attacks_by(s.last_events(), s.card(VANILLA)), ["hero-p2"]);
        }
    }
}
