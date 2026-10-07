//! C+ #76.1 Brother Ping (SPEC §8.7 row 76.1). (2) Unit, CN, Human, Token (printed Rare), 4/4 → 8/8.
//!   Base:    "Pierce / Activate: Deal {damage} damage."
//!   Radiant: "Pierce / Activate 2: Deal {damage} damage."
//!   Engine:  "Activate (§6.2, R384): its controller, in their own main phase with no prompt open, while it
//!            is on the field; once per turn (Radiant twice), counted in `memory.activations` and reset
//!            when it leaves the field (R78); summoning sickness and exertion don't apply, and it is not a
//!            play. The target, any unit or hero, is declared with the activation (`activate { instanceId,
//!            targets }`, §10.2). The hit's source is Brother Ping, so its Pierce applies (R346).
//!            Tunes: Activate 1 ↑ (its X); damage 1 ↑."
//!
//! One ability on each face, its uses the printed Activate N (1, Radiant 2): the engine's activate
//! subsystem counts them per turn on the instance, refuses a use past them (and lists none), and reads the
//! count through B3.4's X change, so an Upgrade makes it Activate 2 and a Degrade never takes it below 1.
//! The hit is one §4.4 instance whose source is this Unit, so its Pierce skips Armor (R346); it is not a
//! Spell's hit, so Spell Damage never raises it.

use jackioh_engine::effects::damage;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-076-1";

/// §8.7: "Deal 1 damage" with no target named is targeted: any unit or hero, either side.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))]
}

/// §8.7: "Activate" is once a turn, the Radiant face's "Activate 2" twice.
const BASE_USES: i32 = 1;
const RADIANT_USES: i32 = 2;

fn ping(uses: i32) -> ActivationDecl {
    ActivationDecl {
        id: "ping".to_string(),
        label: "Deal damage".to_string(),
        uses: ActivationUses::Count(uses),
        cost: None,
        targets: targets(),
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| {
            vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": param(&*ctx, "damage") })))]
        }),
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            activations: vec![ping(BASE_USES)],
            ..Script::default()
        },
        radiant: Script {
            activations: vec![ping(RADIANT_USES)],
            ..Script::default()
        },
    }
}

// C+ #76.1 Brother Ping — SPEC §8.7 row 76.1, BUILD M9 Classic+ row C+ 76.1: "Pierce; Activate (R384):
// its controller, in their main phase with no prompt open, deals 1 damage to a target declared in the
// `activate` action, once per turn; sickness and exertion don't apply, so it activates the turn it
// arrives; the hit has Pierce (R346) and no Spell Damage; activating is no play (Combo, Quickstriker,
// Ceaseless Void ignore it); a second use that turn and any use on the opponent's turn are refused by the
// same check `legalActions` lists; its count resets on leaving the field (R78); `activated` is public; an
// Upgrade makes it Activate 2 and a Degrade never takes it below 1 (R386); Activate count and damage read
// through `param()`; radiant 8/8 and Activate 2: two uses a turn, a third refused".
//
// "Its count resets on leaving the field": Brother Ping is a unit token, so leaving the field it ceases
// to exist (R11) and no instance comes back with a count; the reset itself (R78's `memory`) is the
// activate subsystem's, proved in packages/engine/test/activate.test.ts.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const PING: &str = "classicplus-076-1";
    const LAR: &str = "classicplus-076";
    const FILLER: &str = "core-005";
    const MENACE: &str = "core-019"; // 9/9 Taunt
    const ARMORED: &str = "core-025"; // 7/7, Armor 7
    const SOLARIUS: &str = "classicplus-038"; // Spell Damage +2
    const CEASELESS_VOID: &str = "core-100"; // costs (1) less for each card played this game, R55's pattern
    const HIT_JOB: &str = "core-016";

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialises")
    }

    fn enemy_hero() -> Value {
        json!([{ "pick": "hero", "player": "p2" }])
    }

    fn at(instance_id: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": instance_id }])
    }

    /// TS's live `s.card(ref)`, written through: the card under that id in the state.
    fn card_mut<'a>(s: &'a mut Scenario, card: &str) -> &'a mut CardInstance {
        let id = s.card(card).id.clone();
        find_instance_mut(s.state_mut(), &id).expect("the card is in the state")
    }

    /// The `activate`/`activatePower` actions `legalActions` lists for `player` on that card, as JSON.
    fn listed(s: &Scenario, player: PlayerId, instance_id: &str) -> Vec<Value> {
        legal_actions(s.state(), player)
            .iter()
            .map(js)
            .filter(|action| {
                (action["type"] == "activate" || action["type"] == "activatePower")
                    && action["instanceId"] == instance_id
            })
            .collect()
    }

    fn setup(radiant: bool, p1_field: &[&str], p2_field: &[&str]) -> Scenario {
        crate::register_all();
        let mut ping = json!({ "def": PING });
        if radiant {
            ping["radiant"] = json!(true);
        }
        let mut field = vec![ping];
        field.extend(p1_field.iter().map(|id| json!(id)));
        scenario(json!({
            "p1": { "hand": [FILLER, FILLER], "field": field, "library": [FILLER, FILLER] },
            "p2": { "hand": [FILLER], "field": p2_field, "library": [FILLER, FILLER] },
        }))
    }

    mod c_n76_1_brother_ping {
        use super::*;

        #[test]
        fn is_a_2_4_4_pierce_token_printed_rare_with_one_activate_on_each_face_once_radiant_twice() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(def.id, PING);
            assert_eq!(def.printed_rarity, Some(PrintedRarity::Rare));
            assert_eq!(
                [
                    js(&def.base.attack),
                    js(&def.base.health),
                    js(&def.radiant.attack),
                    js(&def.radiant.health)
                ],
                [json!(4), json!(4), json!(8), json!(8)]
            );
            assert_eq!(js(&def.base.keywords), json!([{ "kind": "Pierce" }]));
            let scripts = super::super::script();
            let uses = |script: &Script| script.activations.iter().map(|ability| js(&ability.uses)).collect::<Vec<_>>();
            assert_eq!(uses(&scripts.base), vec![json!(1)]);
            assert_eq!(uses(&scripts.radiant), vec![json!(2)]);
            let decl = json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "hero"] } }]);
            assert_eq!(scripts.base.activations.first().map(|ability| js(&ability.targets)), Some(decl));
        }

        mod base {
            use super::*;

            #[test]
            fn r384_activating_deals_1_damage_to_the_target_the_action_declares_activated_is_public() {
                let mut s = setup(false, &[], &[MENACE]);
                let menace = s.card(MENACE).id.clone();
                s.activate(PING, json!({ "targets": at(&menace) }));
                s.expect_stats(MENACE, json!({ "health": 8 }));
                let theirs: Vec<Value> = js(&s.view(P2))["events"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|event| event["type"] == "activated")
                    .collect();
                let ping = s.card(PING).id.clone();
                assert_eq!(theirs.len(), 1);
                assert_eq!(theirs[0]["type"], json!("activated"));
                assert_eq!(theirs[0]["instanceId"], json!(ping));
                assert_eq!(theirs[0]["defId"], json!(PING));
            }

            #[test]
            fn r384_it_may_hit_either_hero_or_a_unit_of_its_own_side() {
                let mut s = setup(false, &[MENACE], &[]);
                s.activate(PING, json!({ "targets": enemy_hero() }));
                s.expect_health(P2, 29);
                s.end_turn().end_turn();
                s.activate(PING, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
                s.expect_health(P1, 29);
                s.end_turn().end_turn();
                let menace = s.card(MENACE).id.clone();
                s.activate(PING, json!({ "targets": at(&menace) }));
                s.expect_stats(MENACE, json!({ "health": 8 }));
            }

            #[test]
            fn r346_the_hit_has_pierce_armor_7_takes_none_of_it() {
                let mut s = setup(false, &[], &[ARMORED]);
                let armored = s.card(ARMORED).id.clone();
                s.activate(PING, json!({ "targets": at(&armored) }));
                s.expect_stats(ARMORED, json!({ "health": 6 }));
            }

            #[test]
            fn e6_it_is_no_spells_hit_so_spell_damage_never_raises_it() {
                let mut s = setup(false, &[SOLARIUS], &[]);
                s.activate(PING, json!({ "targets": enemy_hero() }));
                s.expect_health(P2, 29);
            }

            #[test]
            fn r384_it_activates_the_turn_it_arrives_brother_lars_death_leaves_a_ping_that_can_be_used_at_once() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [HIT_JOB, FILLER], "field": [LAR] }, "p2": { "hand": [FILLER] } }));
                let lar = s.card(LAR).id.clone();
                s.play(HIT_JOB, json!({ "targets": at(&lar) }));
                let ping = match s.unit(P1, 1) {
                    Some(ping) => ping,
                    None => panic!("no Ping"),
                };
                assert!(!listed(&s, P1, &ping.id).is_empty());
                s.activate(&ping, json!({ "targets": enemy_hero() }));
                s.expect_health(P2, 29);
            }

            #[test]
            fn r384_activating_spends_no_exertion_it_may_still_attack_that_turn() {
                let mut s = setup(false, &[], &[]);
                s.activate(PING, json!({ "targets": enemy_hero() })).attack(PING, "hero");
                s.expect_health(P2, 25);
            }

            #[test]
            fn r384_activating_is_not_a_play_no_card_played_the_play_count_and_ceaseless_voids_price_stand_still() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [CEASELESS_VOID, FILLER], "field": [PING] },
                    "p2": { "hand": [FILLER] },
                }));
                let played = s.state().players.p1.turn_log.cards_played;
                let price = js(&s.view(P1))["you"]["hand"].clone();
                s.activate(PING, json!({ "targets": enemy_hero() }));
                assert!(!s.last_events().iter().any(|event| event.event_type() == GameEventType::CardPlayed));
                assert_eq!(s.state().players.p1.turn_log.cards_played, played);
                assert_eq!(js(&s.view(P1))["you"]["hand"], price);
            }

            #[test]
            fn r384_a_second_use_that_turn_is_refused_and_not_listed_it_is_back_the_next_turn() {
                let mut s = setup(false, &[], &[]);
                let ping = s.card(PING).clone();
                s.activate(&ping, json!({ "targets": enemy_hero() }));
                assert!(listed(&s, P1, &ping.id).is_empty());
                s.expect_refused(|s| s.activate(&ping, json!({ "targets": enemy_hero() })));
                s.expect_health(P2, 29);
                s.end_turn().end_turn();
                assert!(!listed(&s, P1, &ping.id).is_empty());
                s.activate(&ping, json!({ "targets": enemy_hero() }));
                s.expect_health(P2, 28);
            }

            #[test]
            fn r78_its_count_resets_on_leaving_the_field_used_destroyed_and_back_by_a_granted_reborn_it_may_activate_again_that_turn()
             {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [HIT_JOB, FILLER], "field": [PING], "library": [FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                card_mut(&mut s, PING).granted_keywords = vec![json_as(json!({ "kind": "Reborn" }))];
                s.activate(PING, json!({ "targets": enemy_hero() }));
                let ping = s.card(PING).id.clone();
                assert!(listed(&s, P1, &ping).is_empty());
                let target = s.unit(P1, 1).expect("Ping on the field").id;
                s.play(HIT_JOB, json!({ "targets": at(&target) }));
                let back = s.unit(P1, 1);
                assert_eq!(back.as_ref().map(|card| card.def_id.clone()), Some(PING.to_string()));
                let back = back.expect("Ping back on the field");
                assert!(!listed(&s, P1, &back.id).is_empty());
                s.activate(&back, json!({ "targets": enemy_hero() }));
                s.expect_health(P2, 28);
            }

            #[test]
            fn r384_on_the_opponents_turn_it_is_neither_listed_nor_accepted() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [FILLER], "field": [PING] },
                    "p2": { "hand": [FILLER] },
                }));
                let ping = s.card(PING).clone();
                assert!(listed(&s, P1, &ping.id).is_empty());
                s.expect_refused(|s| s.activate(&ping, json!({ "targets": enemy_hero() })));
            }

            #[test]
            fn r384_legal_actions_lists_one_action_per_legal_target_the_heroes_and_the_units() {
                let s = setup(false, &[], &[MENACE]);
                let targets: Vec<String> =
                    listed(&s, P1, &s.card(PING).id).iter().map(|action| action["targets"].to_string()).collect();
                assert!(targets.contains(&enemy_hero().to_string()));
                assert!(targets.contains(&at(&s.card(MENACE).id).to_string()));
            }

            #[test]
            fn r386_an_upgrade_of_its_x_makes_it_activate_2_a_degrade_never_takes_it_below_1() {
                let mut up = setup(false, &[], &[]);
                card_mut(&mut up, PING).tuning = Some(json_as(json!({ "x": { "Activate": 1 } })));
                up.activate(PING, json!({ "targets": enemy_hero() }))
                    .activate(PING, json!({ "targets": enemy_hero() }));
                up.expect_health(P2, 28);
                up.expect_refused(|s| s.activate(PING, json!({ "targets": enemy_hero() })));

                let mut down = setup(false, &[], &[]);
                card_mut(&mut down, PING).tuning = Some(json_as(json!({ "x": { "Activate": -3 } })));
                down.activate(PING, json!({ "targets": enemy_hero() }));
                down.expect_health(P2, 29);
                down.expect_refused(|s| s.activate(PING, json!({ "targets": enemy_hero() })));
            }

            #[test]
            fn r386_an_upgrade_of_its_damage_deals_2_a_degrade_never_below_1() {
                let mut up = setup(false, &[], &[]);
                step_param(card_mut(&mut up, PING), "damage", 1);
                up.activate(PING, json!({ "targets": enemy_hero() }));
                up.expect_health(P2, 28);

                let mut down = setup(false, &[], &[]);
                step_param(card_mut(&mut down, PING), "damage", -1);
                down.activate(PING, json!({ "targets": enemy_hero() }));
                down.expect_health(P2, 29);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r384_an_8_8_with_activate_2_two_uses_a_turn_a_third_refused_and_not_listed() {
                let mut s = setup(true, &[], &[]);
                let ping = s.card(PING).clone();
                s.expect_stats(&ping, json!({ "attack": 8, "maxHealth": 8 }));
                s.activate(&ping, json!({ "targets": enemy_hero() }));
                assert!(!listed(&s, P1, &ping.id).is_empty());
                s.activate(&ping, json!({ "targets": enemy_hero() }));
                s.expect_health(P2, 28);
                assert!(listed(&s, P1, &ping.id).is_empty());
                s.expect_refused(|s| s.activate(&ping, json!({ "targets": enemy_hero() })));
            }

            #[test]
            fn r346_the_radiant_hits_pierce_too() {
                let mut s = setup(true, &[], &[ARMORED]);
                let armored = at(&s.card(ARMORED).id);
                s.activate(PING, json!({ "targets": armored.clone() }))
                    .activate(PING, json!({ "targets": armored }));
                s.expect_stats(ARMORED, json!({ "health": 5 }));
            }

            #[test]
            fn r386_an_upgrade_of_the_radiant_x_makes_it_activate_3() {
                let mut s = setup(true, &[], &[]);
                card_mut(&mut s, PING).tuning = Some(json_as(json!({ "x": { "Activate": 1 } })));
                s.activate(PING, json!({ "targets": enemy_hero() }))
                    .activate(PING, json!({ "targets": enemy_hero() }))
                    .activate(PING, json!({ "targets": enemy_hero() }));
                s.expect_health(P2, 27);
                s.expect_refused(|s| s.activate(PING, json!({ "targets": enemy_hero() })));
            }
        }
    }
}
