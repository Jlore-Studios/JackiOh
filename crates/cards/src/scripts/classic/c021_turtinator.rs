//! C #21 Turtinator (SPEC §8.6 row 21). Unit 5/4 → 10/8, cost 2, Common.
//!   Both faces: "Activate ♾️: Tribute a Unit. Deal damage equal to {multiplier}× its Attack to any
//!   target." — the multiplier is 1 on the base face and 2 on the Radiant face (a declared number).
//!
//! Activate ♾️ (R384): any number of uses a turn, bounded by `ACTIVATE_UNLIMITED_CAP` and, in
//! practice, by the units there are to Tribute. The cost is "sacrifice one of your units", a pick
//! carried in the `activate` action (`tributes`) — never Turtinator itself (R683,
//! `cost.tributeExcludesSelf`), paid as the ability is activated (`cost.tribute`); the Tribute is a
//! Sacrifice, so it is a death (Death fires, §6.3) and it bypasses Indestructible. One unit is one
//! Tribute here: a Sheep Token's "worth 2" counts only toward a play's Tribute X (§6.3), and an
//! activation needs no zone (R391 is about plays).
//!
//! The hit's amount is the tributed unit's Attack as it stood when it was paid — its last-known
//! state (R78), which the activation records (`subsystems.activationPaid`), since the unit is in a
//! graveyard, reset, by the time the effect runs — times the multiplier. The hit comes from Turtinator
//! (the source is the card as it last stood, §4.4). An amount of 0 is no hit at all (R63), so nothing
//! is dealt and nothing is reported.

use jackioh_engine::effects::damage;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-021";

fn eat() -> ActivationDecl {
    ActivationDecl {
        id: "eat".to_string(),
        label: "Tribute a Unit. Deal damage equal to its Attack times the multiplier to any target".to_string(),
        uses: ActivationUses::Unlimited,
        cost: Some(ActivationCost {
            tribute: Some(1),
            tribute_excludes_self: Some(true),
            ..ActivationCost::default()
        }),
        targets: vec![TargetDecl::target(1, 1, json!({ "of": ["unit", "hero"] }))],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| {
            let attack = subsystems::activation_paid(&*ctx)
                .tributed
                .first()
                .map_or(0, |tributed| tributed.attack);
            let amount = attack * param(&*ctx, "multiplier");
            if amount > 0 {
                vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))]
            } else {
                vec![]
            }
        }),
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        activations: vec![eat()],
        ..Script::default()
    };
    // The same script: the Radiant face's "twice its Attack" is its declared multiplier (2), which
    // `param` reads off the running face.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C #21 Turtinator — SPEC §8.6 row 21, BUILD M9 Classic row C 21: "Activate ♾️ (R384): its cost,
// Tributing one of your Units (a pick carried in the action, Turtinator itself allowed), is paid as it
// activates, so with no Unit it can't activate; then one hit from Turtinator on a declared target equal
// to that Unit's attack as it stood (last-known, R78), the hit still coming when it tributed itself;
// 0 attack → no hit (R63); a Sheep or C #82 is one Unit here (a script's Tribute, §6.3); the Tribute is
// a death (Death fires); it needs no zone (R391); at most `ACTIVATE_UNLIMITED_CAP` uses per turn; not a
// play; radiant 10/8: twice that attack; its tuned number (multiplier) reads through `param()` (R386)".
//
// Turtinator is always one of its controller's Units while it can be activated, so "no Unit" never
// arises on the field; what the cost refuses is an activation that names no Tribute, or a Tribute that
// is not one of its controller's own Units.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TURTLE: &str = "classic-021";
    const FILLER: &str = "core-005"; // (1) Spell, a card to keep a hand from auto-ending (§2.5).
    const VANILLA: &str = "core-008"; // 4/4.
    const TIMMY: &str = "core-011"; // 3/3.
    const POINTMASTER: &str = "core-020"; // 7/1.
    const D_FENDER: &str = "core-001"; // 0/7.
    const SAINTESS: &str = "core-081"; // 2/2, "Death: Make your other Units Radiant."
    const WEAPONS: &str = "core-014"; // Field Spell, "Aura: Your Units have +4 attack, Rush and First Strike."
    const SHEEP: &str = "core-t-sheep"; // 1/1, worth 2 Tributes.
    const ROCK: &str = "core-066"; // 10/10 Indestructible.

    fn enemy_hero() -> Value {
        json!([{ "pick": "hero", "player": "p2" }])
    }

    /// TS's `{ …defaults, ...side }`: the side's own keys over the defaults.
    fn spread(mut defaults: Value, side: Value) -> Value {
        if let (Some(into), Some(from)) = (defaults.as_object_mut(), side.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        defaults
    }

    fn setup(p1: Value, p2: Value) -> Scenario {
        scenario(json!({
            "p1": spread(json!({ "hand": [FILLER], "library": [FILLER] }), p1),
            "p2": spread(json!({ "hand": [FILLER], "library": [FILLER] }), p2),
        }))
    }

    /// TS `eat(s, tribute, targets = ENEMY_HERO, who = TURTLE)`, the defaults written out.
    fn eat<'a>(s: &'a mut Scenario, tribute: &str, targets: Value, who: &str) -> &'a mut Scenario {
        s.activate(who, json!({ "tributes": [tribute], "targets": targets }))
    }

    fn damage_events(s: &Scenario) -> Vec<GameEvent> {
        s.last_events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Damage)
            .cloned()
            .collect()
    }

    fn activations_of(s: &Scenario, player: PlayerId, id: &str) -> Vec<ActionBody> {
        legal_actions(s.state(), player)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Activate { instance_id, .. } if instance_id == id))
            .collect()
    }

    /// TS `stepParam(s.card(ref), key, steps)`: on the live instance, found again by id.
    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        step_param(find_instance_mut(s.state_mut(), &id).expect("the card is in the state"), key, steps);
    }

    mod c_n21_turtinator {
        use super::*;

        #[test]
        fn r384_declares_one_activate_whose_cost_is_one_tribute_with_a_declared_unit_or_hero_target() {
            crate::register_all();
            let scripts = script();
            assert_eq!(crate::card_def(ID).id, TURTLE);
            // TS `expect(radiant).toBe(base)`: the Radiant face is the base script itself, so its
            // ability's hook is the very same closure.
            assert!(Arc::ptr_eq(&scripts.radiant.activations[0].run, &scripts.base.activations[0].run));
            let ability = &scripts.base.activations[0];
            assert_eq!(ability.uses, ActivationUses::Unlimited);
            assert_eq!(serde_json::to_value(ability.cost).unwrap(), json!({ "tribute": 1, "tributeExcludesSelf": true }));
            assert_eq!(serde_json::to_value(&ability.targets[0].filter).unwrap(), json!({ "of": ["unit", "hero"] }));
        }

        mod base {
            use super::*;

            #[test]
            fn r384_tributes_one_of_your_units_as_it_activates_then_deals_that_unit_s_attack_to_the_declared_target() {
                crate::register_all();
                let mut s = setup(json!({ "field": [TURTLE, POINTMASTER] }), json!({}));
                let point = s.card(POINTMASTER).clone();

                eat(&mut s, POINTMASTER, enemy_hero(), TURTLE);

                s.expect_in_zone(&point, "graveyard");
                s.expect_health(P2, 23);
                s.expect_events(json!(["activated", "destroyed", "damage"]));
            }

            #[test]
            fn the_hit_s_source_is_turtinator() {
                crate::register_all();
                let mut s = setup(json!({ "field": [TURTLE, POINTMASTER] }), json!({}));
                let turtle = s.card(TURTLE).clone();

                eat(&mut s, POINTMASTER, enemy_hero(), TURTLE);

                let hit = s.last_events().iter().find(|event| event.event_type() == GameEventType::Damage).cloned();
                match hit {
                    Some(GameEvent::Damage { source_id, amount, .. }) => {
                        assert_eq!(source_id, Some(turtle.id.clone()));
                        assert_eq!(amount, 7);
                    }
                    other => panic!("no damage event: {other:?}"),
                }
            }

            #[test]
            fn r78_the_amount_is_the_tributed_unit_s_attack_as_it_stood_an_aura_s_bonus_included() {
                crate::register_all();
                let mut s = setup(json!({ "field": [TURTLE, TIMMY], "backrow": [WEAPONS] }), json!({}));
                s.expect_stats(TIMMY, json!({ "attack": 7 }));

                eat(&mut s, TIMMY, enemy_hero(), TURTLE);

                // Timmy is 3 in the graveyard; the aura's +4 counted, because it was paid as it stood.
                s.expect_health(P2, 23);
            }

            #[test]
            fn r683_turtinator_cannot_tribute_itself_alone_on_its_side_no_activation_is_listed() {
                crate::register_all();
                let mut s = setup(json!({ "field": [TURTLE] }), json!({}));
                let turtle = s.card(TURTLE).clone();

                assert_eq!(activations_of(&s, P1, &turtle.id).len(), 0);
                s.expect_refused(|s| eat(s, TURTLE, enemy_hero(), TURTLE));

                s.expect_in_zone(&turtle, "field");
                s.expect_health(P2, 30);
            }

            #[test]
            fn r384_the_tribute_is_its_cost_an_activation_that_names_none_is_refused_and_changes_nothing() {
                crate::register_all();
                let mut s = setup(json!({ "field": [TURTLE, POINTMASTER] }), json!({}));

                s.expect_refused(|s| s.activate(TURTLE, json!({ "targets": enemy_hero(), "tributes": [] })));

                s.expect_in_zone(POINTMASTER, "field");
                s.expect_health(P2, 30);
            }

            #[test]
            fn r384_an_enemy_unit_is_no_tribute_for_it() {
                crate::register_all();
                let mut s = setup(json!({ "field": [TURTLE] }), json!({ "field": [POINTMASTER] }));

                s.expect_refused(|s| eat(s, POINTMASTER, enemy_hero(), TURTLE));

                s.expect_in_zone(POINTMASTER, "field");
                s.expect_health(P2, 30);
            }

            #[test]
            fn r384_one_unit_is_one_tribute_two_named_are_refused() {
                crate::register_all();
                let mut s = setup(json!({ "field": [TURTLE, TIMMY, POINTMASTER] }), json!({}));

                s.expect_refused(|s| {
                    s.activate(TURTLE, json!({ "tributes": [TIMMY, POINTMASTER], "targets": enemy_hero() }))
                });
            }

            #[test]
            fn s6_3_a_sheep_token_is_one_unit_here_not_two_tributes_it_pays_the_cost_and_deals_its_1() {
                crate::register_all();
                let mut s = setup(json!({ "field": [TURTLE, SHEEP] }), json!({}));

                eat(&mut s, SHEEP, enemy_hero(), TURTLE);

                s.expect_health(P2, 29);
                assert!(s.unit(P1, 2).is_none());
            }

            #[test]
            fn s6_3_a_tribute_bypasses_indestructible_the_rock_is_tributed_and_deals_its_10() {
                crate::register_all();
                let mut s = setup(json!({ "field": [TURTLE, ROCK] }), json!({}));
                let rock = s.card(ROCK).clone();

                eat(&mut s, ROCK, enemy_hero(), TURTLE);

                s.expect_in_zone(&rock, "graveyard");
                s.expect_health(P2, 20);
            }

            #[test]
            fn r13_a_unit_dormant_under_a_stack_pile_is_no_tribute_for_it() {
                crate::register_all();
                let mut s = setup(
                    json!({ "field": [TURTLE, { "def": TIMMY, "lane": 2 }, { "def": POINTMASTER, "stack": true }] }),
                    json!({}),
                );
                let dormant = s.card(TIMMY).clone();

                s.expect_refused(|s| s.activate(TURTLE, json!({ "tributes": [dormant.id], "targets": enemy_hero() })));

                s.expect_in_zone(&dormant, "field");
                s.expect_health(P2, 30);
            }

            #[test]
            fn r63_a_unit_with_0_attack_pays_the_cost_but_deals_no_hit_at_all() {
                crate::register_all();
                let mut s = setup(json!({ "field": [TURTLE, D_FENDER] }), json!({}));

                eat(&mut s, D_FENDER, enemy_hero(), TURTLE);

                s.expect_in_zone(D_FENDER, "graveyard");
                s.expect_health(P2, 30);
                assert_eq!(damage_events(&s).len(), 0);
            }

            #[test]
            fn s6_3_the_tribute_is_a_death_the_tributed_unit_s_death_fires() {
                crate::register_all();
                let mut s = setup(json!({ "field": [TURTLE, SAINTESS, VANILLA] }), json!({}));
                let vanilla = s.card(VANILLA).clone();

                eat(&mut s, SAINTESS, enemy_hero(), TURTLE);

                s.expect_events(json!(["destroyed", "radiantSet"]));
                assert!(s.card(&vanilla).radiant);
            }

            #[test]
            fn targets_any_unit_either_side_or_a_hero() {
                crate::register_all();
                let mut s = setup(json!({ "field": [TURTLE, POINTMASTER, VANILLA] }), json!({ "field": ["core-019"] }));
                let menace = s.card("core-019").clone();

                eat(&mut s, POINTMASTER, json!([{ "pick": "instance", "instanceId": menace.id }]), TURTLE);
                s.expect_stats(&menace, json!({ "health": 2 }));

                eat(&mut s, VANILLA, json!([{ "pick": "hero", "player": "p1" }]), TURTLE);
                s.expect_health(P1, 26);
            }

            #[test]
            fn r391_needs_no_zone_it_activates_with_its_controller_s_unit_row_full() {
                crate::register_all();
                let mut s = setup(json!({ "field": [TURTLE, TIMMY, VANILLA, POINTMASTER, D_FENDER] }), json!({}));

                eat(&mut s, TIMMY, enemy_hero(), TURTLE);

                s.expect_health(P2, 27);
            }

            #[test]
            fn r384_activate_many_uses_in_one_turn_each_paying_its_own_tribute() {
                crate::register_all();
                let mut s = setup(json!({ "field": [TURTLE, TIMMY, VANILLA, POINTMASTER] }), json!({}));

                eat(&mut s, TIMMY, enemy_hero(), TURTLE);
                eat(&mut s, VANILLA, enemy_hero(), TURTLE);
                eat(&mut s, POINTMASTER, enemy_hero(), TURTLE);

                s.expect_health(P2, 30 - 3 - 4 - 7);
                // R683: with only itself left to Tribute, no further activation is listed.
                let turtle = s.card(TURTLE).id.clone();
                assert_eq!(activations_of(&s, P1, &turtle).len(), 0);
            }

            #[test]
            fn r384_stops_at_activate_unlimited_cap_uses_in_a_turn() {
                crate::register_all();
                let mut s = setup(json!({ "field": [TURTLE, TIMMY] }), json!({}));
                let turtle = s.card(TURTLE).id.clone();
                let turn = s.state().turn;
                find_instance_mut(s.state_mut(), &turtle).expect("Turtinator is in the state").memory.insert(
                    subsystems::ACTIVATIONS_MEMORY_KEY.to_string(),
                    json!({ "turn": turn, "count": ACTIVATE_UNLIMITED_CAP }),
                );

                assert_eq!(activations_of(&s, P1, &turtle).len(), 0);
                s.expect_refused(|s| eat(s, TIMMY, enemy_hero(), TURTLE));
                s.expect_in_zone(TIMMY, "field");
            }

            #[test]
            fn r384_activating_is_not_a_play_no_cardplayed_and_no_play_counted() {
                crate::register_all();
                let mut s = setup(json!({ "field": [TURTLE, TIMMY] }), json!({}));
                let played = s.state().players[P1].turn_log.cards_played;

                eat(&mut s, TIMMY, enemy_hero(), TURTLE);

                assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::CardPlayed));
                assert_eq!(s.state().players[P1].turn_log.cards_played, played);
            }

            #[test]
            fn r386_an_upgrade_of_its_multiplier_doubles_the_hit_a_degrade_never_takes_it_below_1() {
                crate::register_all();
                let mut up = setup(json!({ "field": [TURTLE, POINTMASTER] }), json!({}));
                step(&mut up, TURTLE, "multiplier", 1);
                eat(&mut up, POINTMASTER, enemy_hero(), TURTLE);
                up.expect_health(P2, 16);

                let mut down = setup(json!({ "field": [TURTLE, POINTMASTER] }), json!({}));
                step(&mut down, TURTLE, "multiplier", -1);
                eat(&mut down, POINTMASTER, enemy_hero(), TURTLE);
                down.expect_health(P2, 23);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r275_the_radiant_face_is_a_10_8() {
                crate::register_all();
                let mut s = setup(json!({ "field": [{ "def": TURTLE, "radiant": true }] }), json!({}));
                s.expect_stats(TURTLE, json!({ "attack": 10, "health": 8, "maxHealth": 8 }));
            }

            #[test]
            fn deals_twice_the_tributed_unit_s_attack() {
                crate::register_all();
                let mut s = setup(json!({ "field": [{ "def": TURTLE, "radiant": true }, POINTMASTER] }), json!({}));

                eat(&mut s, POINTMASTER, enemy_hero(), TURTLE);

                s.expect_health(P2, 16);
            }

            #[test]
            fn r683_the_radiant_face_cannot_tribute_itself_either() {
                crate::register_all();
                let mut s = setup(json!({ "field": [{ "def": TURTLE, "radiant": true }] }), json!({ "health": 30 }));
                let turtle = s.card(TURTLE).clone();

                assert_eq!(activations_of(&s, P1, &turtle.id).len(), 0);
                s.expect_refused(|s| eat(s, TURTLE, enemy_hero(), TURTLE));

                s.expect_in_zone(&turtle, "field");
                s.expect_health(P2, 30);
            }

            #[test]
            fn r386_an_upgrade_takes_the_radiant_multiplier_to_3() {
                crate::register_all();
                let mut s = setup(json!({ "field": [{ "def": TURTLE, "radiant": true }, TIMMY] }), json!({}));
                step(&mut s, TURTLE, "multiplier", 1);

                eat(&mut s, TIMMY, enemy_hero(), TURTLE);

                s.expect_health(P2, 21);
            }
        }
    }
}
