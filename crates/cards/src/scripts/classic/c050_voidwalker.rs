//! C #50 Voidwalker (SPEC §8.6 row 50; §6.2 Replacement; R11, R135, R398). Unit 6/3 → 12/6, cost 2, Rare.
//!   Base:    "Cry: Exile every card in both graveyards.\nAura: Cards that would go to a graveyard are
//!            exiled instead."
//!   Radiant: "Cry: Exile every card in your opponent's graveyard.\nAura: Cards your opponent owns that
//!            would go to a graveyard are exiled instead."
//!   Engine:  the aura is a replacement at "would go to a graveyard" while Voidwalker is on the field,
//!            whatever sends the card there; the Radiant face judges by owner. Its own card reaches the
//!            graveyard when it dies: its aura left the field with it (R398). Unit tokens still cease to
//!            exist (R11).

use jackioh_engine::effects::exile_matching;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-050";

/// `{ id: "voidwalker", on: "toGraveyard", instead: { to: "exile" } }`, with the face's `when`.
fn voidwalker(when: Option<ReplacementWhen>) -> ReplacementDef {
    ReplacementDef {
        id: "voidwalker".to_string(),
        on: ReplacementMoment::ToGraveyard,
        where_: None,
        when,
        instead: ReplacementInstead {
            to: Some(InsteadTo::Exile),
            ..ReplacementInstead::default()
        },
        then: None,
        by: None,
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| {
            vec![
                exile_matching(json_as(json!({ "zones": ["graveyard"], "player": "self" }))),
                exile_matching(json_as(json!({ "zones": ["graveyard"], "player": "enemy" }))),
            ]
        })),
        replacements: vec![voidwalker(None)],
        ..Script::default()
    };

    let radiant = Script {
        cry: Some(hook(|_ctx| vec![exile_matching(json_as(json!({ "zones": ["graveyard"], "player": "enemy" })))])),
        replacements: vec![voidwalker(Some(replacement_when(|ctx| {
            matches!(ctx.event, ReplacedEvent::ToGraveyard { owner, .. } if *owner != ctx.controller)
        })))],
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #50 Voidwalker (SPEC §8.6 row 50, BUILD M9 Classic row C 50): the aura takes a death, a discard, a
// resolved Spell, a fired trap and a burn; unit tokens still cease to exist (R11); its own card reaches
// the graveyard (R398). Radiant judges by owner: a stolen Unit of theirs dying on your side is exiled.
//
// A scenario places its graveyards after its field, through the engine's own moves, so a Voidwalker
// already on the field would exile them as they were laid; the Cry's cases play it from hand instead.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VOID: &str = "classic-050";
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy target Unit.
    const ZAO_GAO: &str = "core-080"; // (2) Spell: discard 2 random cards; summon 2 Rush Tokens with 2 random keywords.
    const STOCKPILE: &str = "core-005"; // (1) Spell: draw 2, heal your hero 2.
    const SHEEPISH: &str = "core-041"; // (1) Trap: when your opponent plays a Unit, after its Cry, transform it into a Sheep.
    const MIND_CONTROL: &str = "core-049"; // (4) Spell: steal target enemy permanent.
    const FIENDER: &str = "core-092"; // (2) Unit, Stack.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const GARY: &str = "core-004"; // (1) Unit 1/1
    const LUNAR: &str = "core-035"; // (1) Spell: deal 3 damage to a target.
    const DEFENDER: &str = "core-003"; // (1) Unit 1/1, Reborn; Radiant: Death: summon a base Right-house defender.
    const RUSH_TOKEN: &str = "core-t-rush";
    const FILLER: &str = "core-005";

    use crate::js;

    /// `zone` is "graveyard" or "exile".
    fn def_ids(s: &Scenario, player: PlayerId, zone: &str) -> Vec<String> {
        s.pile(player, zone).into_iter().map(|card| card.def_id).collect()
    }

    fn target(s: &Scenario, player: PlayerId, lane: i32) -> Value {
        let Some(unit) = s.unit(player, lane) else {
            panic!("no unit in {} lane {lane}", player.as_str());
        };
        json!([{ "pick": "instance", "instanceId": unit.id }])
    }

    fn exiled_events(s: &Scenario) -> Vec<Value> {
        s.events().iter().map(js).filter(|event| event["type"] == "exiled").collect()
    }

    fn contains_all(have: &[String], want: &[&str]) -> bool {
        want.iter().all(|id| have.iter().any(|card| card == id))
    }

    mod c50_voidwalker {
        use super::*;

        #[test]
        fn declares_a_cry_and_one_replacement_into_exile_on_each_face_and_no_numbers() {
            crate::register_all();
            let def = registered_catalog()[ID].clone();
            assert_eq!(def.id, VOID);
            assert!(def.params.is_none());
            let scripts = script();
            assert_eq!(scripts.base.replacements.len(), 1);
            let only = &scripts.base.replacements[0];
            assert_eq!(only.id, "voidwalker");
            assert_eq!(only.on, ReplacementMoment::ToGraveyard);
            assert_eq!(js(&only.instead), json!({ "to": "exile" }));
            assert!(only.where_.is_none() && only.when.is_none() && only.then.is_none() && only.by.is_none());
            let radiant: Vec<Value> = scripts
                .radiant
                .replacements
                .iter()
                .map(|each| json!([js(&each.on), js(&each.instead)]))
                .collect();
            assert_eq!(json!(radiant), json!([["toGraveyard", { "to": "exile" }]]));
        }

        #[test]
        fn events_every_exile_it_causes_is_an_exiled_event_naming_the_card() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [HIT_JOB, FILLER], "field": [VOID] }, "p2": { "hand": [FILLER], "field": [VANILLA] } }));
            let vanilla = s.card(VANILLA).clone();
            let targets = target(&s, P2, 1);
            s.play(HIT_JOB, json!({ "targets": targets }));
            let exiled: Vec<Value> = exiled_events(&s).into_iter().map(|event| event["instanceId"].clone()).collect();
            assert!(exiled.contains(&json!(vanilla.id)));
        }

        mod base {
            use super::*;

            #[test]
            fn cry_exiles_every_card_in_both_graveyards_each_to_its_owners_exile() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [VOID, FILLER], "graveyard": [STOCKPILE, VANILLA] },
                    "p2": { "hand": [FILLER], "graveyard": [HIT_JOB, GARY] },
                }));
                s.play(VOID, json!({}));
                assert!(def_ids(&s, P1, "graveyard").is_empty());
                assert!(def_ids(&s, P2, "graveyard").is_empty());
                assert_eq!(def_ids(&s, P1, "exile"), vec![STOCKPILE, VANILLA]);
                assert_eq!(def_ids(&s, P2, "exile"), vec![HIT_JOB, GARY]);
                assert_eq!(exiled_events(&s).len(), 4);
                s.expect_in_zone(VOID, "field");
            }

            #[test]
            fn cry_with_both_graveyards_empty_it_exiles_nothing_and_voidwalker_still_enters() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [VOID, FILLER] }, "p2": { "hand": [FILLER] } }));
                s.play(VOID, json!({}));
                assert!(exiled_events(&s).is_empty());
                s.expect_in_zone(VOID, "field").expect_stats(VOID, json!({ "attack": 6, "health": 3 }));
            }

            #[test]
            fn aura_a_unit_that_dies_is_exiled_and_so_is_the_spell_that_killed_it_once_it_resolves() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [HIT_JOB, FILLER], "field": [VOID] }, "p2": { "hand": [FILLER], "field": [VANILLA] } }));
                let vanilla = s.card(VANILLA).clone();
                let hit_job = s.card(HIT_JOB).clone();
                let targets = target(&s, P2, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
                s.expect_in_zone(&vanilla, "exile").expect_in_zone(&hit_job, "exile");
                assert_eq!(def_ids(&s, P2, "exile"), vec![VANILLA]);
                assert_eq!(def_ids(&s, P1, "exile"), vec![HIT_JOB]);
                assert!(def_ids(&s, P1, "graveyard").is_empty());
                assert!(def_ids(&s, P2, "graveyard").is_empty());
            }

            #[test]
            fn aura_your_opponents_cards_too_their_spell_is_exiled_as_it_resolves() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [VOID] },
                    "p2": { "hand": [STOCKPILE, FILLER], "library": [FILLER, FILLER] },
                    "active": "p2",
                }));
                let stockpile = s.card(STOCKPILE).clone();
                s.play(&stockpile, json!({}));
                s.expect_in_zone(&stockpile, "exile");
            }

            #[test]
            fn aura_discarded_cards_are_exiled() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [ZAO_GAO, LUNAR, GARY], "field": [VOID] }, "p2": { "hand": [FILLER] } }));
                let (lunar, gary) = (s.card(LUNAR).clone(), s.card(GARY).clone());
                s.play(ZAO_GAO, json!({}));
                s.expect_in_zone(&lunar, "exile").expect_in_zone(&gary, "exile").expect_in_zone(ZAO_GAO, "exile");
                assert!(def_ids(&s, P1, "graveyard").is_empty());
            }

            #[test]
            fn aura_a_card_burned_at_the_hand_cap_is_exiled_r317() {
                crate::register_all();
                let mut hand = vec![STOCKPILE];
                hand.extend((0..9).map(|_| FILLER));
                let mut s = scenario(json!({
                    "p1": { "hand": hand, "field": [VOID], "library": [LUNAR, GARY] },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(STOCKPILE, json!({}));
                assert!(s.events().iter().any(|event| matches!(event, GameEvent::Burned { .. })));
                s.expect_in_zone(GARY, "exile");
                assert_eq!(s.hand(P1).len(), 10);
                assert!(def_ids(&s, P1, "graveyard").is_empty());
            }

            #[test]
            fn aura_a_trap_spent_after_it_fires_is_exiled() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [VANILLA, FILLER], "field": [VOID] },
                    "p2": { "hand": [FILLER], "backrow": [{ "def": SHEEPISH, "faceUp": false }] },
                }));
                let sheepish = s.card(SHEEPISH).clone();
                s.play(VANILLA, json!({ "zone": 2 }));
                s.expect_events(json!(["trapFired"]));
                s.expect_in_zone(&sheepish, "exile");
                assert!(def_ids(&s, P2, "graveyard").is_empty());
            }

            #[test]
            fn r11_a_unit_token_still_ceases_to_exist_it_reaches_neither_a_graveyard_nor_an_exile() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [HIT_JOB, FILLER], "field": [VOID] }, "p2": { "hand": [FILLER], "field": [RUSH_TOKEN] } }));
                let token = s.card(RUSH_TOKEN).clone();
                let targets = target(&s, P2, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
                s.expect_in_zone(&token, "gone");
                assert!(def_ids(&s, P2, "exile").is_empty());
            }

            #[test]
            fn a_reborn_unit_the_aura_exiles_has_not_died_no_reborn_body_comes_back_and_its_death_hook_does_not_fire() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [HIT_JOB, FILLER], "field": [VOID] },
                    "p2": { "hand": [FILLER], "field": [{ "def": DEFENDER, "radiant": true }] },
                }));
                let defender = s.card(DEFENDER).clone();
                let targets = target(&s, P2, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
                s.expect_in_zone(&defender, "exile");
                assert!([1, 2, 3, 4, 5].iter().all(|lane| s.unit(P2, *lane).is_none()));
                assert!(
                    !s.events()
                        .iter()
                        .any(|event| matches!(event, GameEvent::Destroyed { .. } | GameEvent::Summoned { .. }))
                );
            }

            #[test]
            fn r398_its_own_card_reaches_its_owners_graveyard_when_it_dies_its_aura_left_the_field_with_it() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [FILLER], "field": [VOID] }, "p2": { "hand": [HIT_JOB, FILLER] }, "active": "p2" }));
                let voidwalker = s.card(VOID).clone();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": voidwalker.id }] }));
                s.expect_in_zone(&voidwalker, "graveyard");
                assert_eq!(def_ids(&s, P1, "graveyard"), vec![VOID]);
                assert!(def_ids(&s, P1, "exile").is_empty());
            }

            #[test]
            fn r398_and_once_it_has_gone_cards_reach_the_graveyard_again() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [VOID] },
                    "p2": { "hand": [HIT_JOB, STOCKPILE, FILLER], "library": [FILLER, FILLER] },
                    "active": "p2",
                }));
                let targets = target(&s, P1, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
                s.play(STOCKPILE, json!({}));
                assert_eq!(def_ids(&s, P2, "graveyard"), vec![HIT_JOB, STOCKPILE]);
            }

            #[test]
            fn r398_a_second_voidwalker_still_on_the_field_exiles_the_first_ones_card_as_it_dies() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [FILLER], "field": [VOID, VOID] }, "p2": { "hand": [HIT_JOB, FILLER] }, "active": "p2" }));
                let Some(first) = s.unit(P1, 1) else {
                    panic!("the first Voidwalker should be on the board");
                };
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": first.id }] }));
                s.expect_in_zone(&first, "exile");
                s.expect_in_zone(HIT_JOB, "exile");
            }

            #[test]
            fn s3_2_a_voidwalker_dormant_under_a_stack_pile_is_not_on_the_field_cards_reach_the_graveyard() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [HIT_JOB, FILLER], "field": [VOID, { "def": FIENDER, "stack": true }] },
                    "p2": { "hand": [FILLER], "field": [VANILLA] },
                }));
                let targets = target(&s, P2, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
                assert_eq!(def_ids(&s, P2, "graveyard"), vec![VANILLA]);
                assert_eq!(def_ids(&s, P1, "graveyard"), vec![HIT_JOB]);
            }

            #[test]
            fn in_a_hand_it_replaces_nothing_the_aura_is_the_card_on_the_fields() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [VOID, HIT_JOB, FILLER] }, "p2": { "hand": [FILLER], "field": [VANILLA] } }));
                let targets = target(&s, P2, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
                assert_eq!(def_ids(&s, P2, "graveyard"), vec![VANILLA]);
                assert_eq!(def_ids(&s, P1, "graveyard"), vec![HIT_JOB]);
            }

            #[test]
            fn s10_8_an_exiled_card_is_public_both_players_read_it() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [ZAO_GAO, LUNAR, GARY], "field": [VOID] }, "p2": { "hand": [FILLER] } }));
                s.play(ZAO_GAO, json!({}));
                let seen: Vec<String> = js(&s.view(P2))["opponent"]["exile"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|card| card["defId"].as_str().map(str::to_string))
                    .collect();
                assert!(contains_all(&seen, &[LUNAR, GARY, ZAO_GAO]));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn cry_exiles_only_your_opponents_graveyard_yours_stays() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": VOID, "radiant": true }, FILLER], "graveyard": [STOCKPILE, VANILLA] },
                    "p2": { "hand": [FILLER], "graveyard": [HIT_JOB, GARY] },
                }));
                s.play(VOID, json!({}));
                assert_eq!(def_ids(&s, P1, "graveyard"), vec![STOCKPILE, VANILLA]);
                assert!(def_ids(&s, P2, "graveyard").is_empty());
                assert_eq!(def_ids(&s, P2, "exile"), vec![HIT_JOB, GARY]);
                s.expect_stats(VOID, json!({ "attack": 12, "health": 6 }));
            }

            #[test]
            fn aura_only_cards_your_opponent_owns_are_exiled_their_dead_unit_is_your_resolved_spell_is_not() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [HIT_JOB, FILLER], "field": [{ "def": VOID, "radiant": true }] },
                    "p2": { "hand": [FILLER], "field": [VANILLA] },
                }));
                let targets = target(&s, P2, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
                assert_eq!(def_ids(&s, P2, "exile"), vec![VANILLA]);
                assert_eq!(def_ids(&s, P1, "graveyard"), vec![HIT_JOB]);
            }

            #[test]
            fn aura_judged_by_owner_a_stolen_unit_of_theirs_dying_on_your_side_is_exiled() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [MIND_CONTROL, HIT_JOB, FILLER], "field": [{ "def": VOID, "radiant": true }], "mana": 7 },
                    "p2": { "hand": [FILLER], "field": [VANILLA] },
                }));
                let vanilla = s.card(VANILLA).clone();
                s.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla.id }] }));
                assert_eq!(s.card(&vanilla).controller, P1);
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla.id }] }));
                s.expect_in_zone(&vanilla, "exile");
                assert_eq!(def_ids(&s, P2, "exile"), vec![VANILLA]);
                assert_eq!(def_ids(&s, P1, "graveyard"), vec![MIND_CONTROL, HIT_JOB]);
            }

            #[test]
            fn aura_your_own_cards_reach_your_graveyard_your_unit_that_dies_your_discards() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": VOID, "radiant": true }, GARY] },
                    "p2": { "hand": [HIT_JOB, FILLER] },
                    "active": "p2",
                }));
                let targets = target(&s, P1, 2);
                s.play(HIT_JOB, json!({ "targets": targets }));
                assert_eq!(def_ids(&s, P1, "graveyard"), vec![GARY]);
                assert_eq!(def_ids(&s, P2, "exile"), vec![HIT_JOB]);
            }

            #[test]
            fn aura_your_opponents_discards_are_exiled() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": VOID, "radiant": true }] },
                    "p2": { "hand": [ZAO_GAO, LUNAR, GARY] },
                    "active": "p2",
                }));
                s.play(ZAO_GAO, json!({}));
                assert!(contains_all(&def_ids(&s, P2, "exile"), &[LUNAR, GARY, ZAO_GAO]));
                assert!(def_ids(&s, P2, "graveyard").is_empty());
            }

            #[test]
            fn r398_its_own_card_reaches_your_graveyard_when_it_dies() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": VOID, "radiant": true }] },
                    "p2": { "hand": [HIT_JOB, FILLER] },
                    "active": "p2",
                }));
                let targets = target(&s, P1, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
                assert_eq!(def_ids(&s, P1, "graveyard"), vec![VOID]);
            }

            #[test]
            fn r11_an_enemy_unit_token_still_ceases_to_exist() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [HIT_JOB, FILLER], "field": [{ "def": VOID, "radiant": true }] },
                    "p2": { "hand": [FILLER], "field": [RUSH_TOKEN] },
                }));
                let token = s.card(RUSH_TOKEN).clone();
                let targets = target(&s, P2, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
                s.expect_in_zone(&token, "gone");
                assert!(def_ids(&s, P2, "exile").is_empty());
            }
        }
    }
}
