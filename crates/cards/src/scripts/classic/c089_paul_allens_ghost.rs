//! C #89 Paul Allen's Ghost (SPEC §8.6 row 89, BUILD M9 Classic row C 89). (2) Unit 5/6 → 10/12, Rare.
//!   Base:    "Divine Shield / To target this with anything but an attack, a player must also discard
//!            {discard|card|cards}." (2)
//!   Radiant: "Divine Shield, Reborn / (the same)."
//!   Engine:  "A replacement at 'a friendly unit is targeted' (§6.2 Replacement) that adds a cost: a
//!            declared target (a play or an activation) naming it costs 2 discards, random at pay time
//!            (R682), and the action carries none; a prompt answer naming it pays them
//!            before it goes on. With fewer than 2 other cards in hand it is not a legal target. It binds
//!            both players, its controller included. 'Target' is as R394 reads it: a declared or
//!            prompted pick, while random picks and 'all' effects target nothing. Tunes: discard 2 ↑."
//!
//! Targeting cost (B5): §10.5 step 2 pays random discards (R682, §6.3). Tuned via param() (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-089";

pub fn script() -> CardScripts {
    let base = Script {
        targeting_discards: Some(read_hook(|args| param(&args, "discard"))),
        ..Script::default()
    };

    // The same script: the Radiant face differs only in what the engine reads off the catalog (its doubled
    // stats and Reborn).
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// SPEC §8.6: targeting costs 2 random discards (R682); legalActions and prompts require enough cards;
// attacks and random picks cost nothing. Tuned via param() (R386). Echo repeat tests fresh picks (R81).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const GHOST: &str = "classic-089";
    /// (1) Spell: Deal 4 damage (a declared target).
    const WILDFIRE: &str = "classic-055";
    const ECHO: &str = "classic-057";
    /// Whenever you discard cards, draw that many.
    const RECYCLER: &str = "classic-064";
    /// (4) Spell: Destroy all permanents.
    const NETHER: &str = "core-088";
    /// (1) Unit 4/4
    const VANILLA: &str = "core-008";
    /// (3) Unit 9/9
    const MENACE: &str = "core-019";
    /// (1) Spell
    const FILLER: &str = "core-005";
    /// (0) Spell, a card to discard
    const SPARE: &str = "core-010";
    /// Activate: remove a Plague Counter from a permanent; an enemy one is exiled.
    const MUTATE: &str = "classic-078";
    /// Cry: with 4 or more mana as it was played, bounce 2 random enemy permanents.
    const MID_RUNNER: &str = "classic-022";

    fn at(card: &CardInstance) -> Selection {
        Selection::Instance {
            instance_id: card.id.clone(),
        }
    }

    /// Per-test counter (no mutable statics in a pure crate, §3): nonces stay distinct for dedupe.
    fn send(nonce: &mut u32, state: &GameState, player_id: PlayerId, body: Value) -> ReduceResult {
        *nonce += 1;
        let mut action = body;
        action["playerId"] = json!(player_id);
        action["nonce"] = json!(format!("c89-{nonce}"));
        reduce(state, &json_as::<Action>(action))
    }

    fn wildfire_plays(s: &Scenario, player: PlayerId) -> Vec<ActionBody> {
        let id = s.card(WILDFIRE).id.clone();
        legal_actions(s.state(), player)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == id))
            .collect()
    }

    fn spares(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player)
            .iter()
            .filter(|card| card.def_id == SPARE)
            .map(|card| card.id.clone())
            .collect()
    }

    fn first_target(action: &ActionBody) -> Option<Selection> {
        match action {
            ActionBody::Play { targets, .. } | ActionBody::Activate { targets, .. } => {
                targets.as_ref().and_then(|picks| picks.first().cloned())
            }
            _ => None,
        }
    }

    /// The action as it goes on the wire names no discards.
    fn carries_no_discards(action: &ActionBody) -> bool {
        serde_json::to_value(action).unwrap().get("discards").is_none()
    }

    fn discarded_ids(events: &[GameEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Discarded { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn pending_selections(s: &Scenario) -> Vec<Selection> {
        s.state()
            .pending
            .as_ref()
            .map(|pending| pending.options.iter().map(|option| option.selection.clone()).collect())
            .unwrap_or_default()
    }

    fn keyword_kinds(keywords: &[Keyword]) -> Vec<&'static str> {
        keywords.iter().map(|keyword| keyword.kind().as_str()).collect()
    }

    mod c_n89_paul_allens_ghost {
        use super::*;

        #[test]
        fn is_a_2_5_6_divine_shield_unit_10_12_divine_shield_reborn_its_discard_a_declared_number() {
            crate::register_all();
            let def = crate::card_def(ID);
            assert_eq!(json!(def.cost), json!(2));
            assert_eq!(
                [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
                [Some(5), Some(6), Some(10), Some(12)]
            );
            assert_eq!(def.base.keywords, vec![Keyword::DivineShield]);
            assert_eq!(def.radiant.keywords, vec![Keyword::DivineShield, Keyword::Reborn]);
            assert_eq!(
                json!(def.params),
                json!([{ "key": "discard", "base": 2, "radiant": 2, "better": "up", "step": 1, "min": 1 }])
            );
            let scripts = script();
            assert!(scripts.base.targeting_discards.is_some());
            // The Radiant face is the same script, so the same hook.
            assert!(std::sync::Arc::ptr_eq(
                scripts.base.targeting_discards.as_ref().unwrap(),
                scripts.radiant.targeting_discards.as_ref().unwrap()
            ));
        }

        mod base {
            use super::*;

            #[test]
            fn is_a_5_6_with_divine_shield() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [GHOST, FILLER] } }));
                s.play(GHOST, json!({}))
                    .expect_stats(GHOST, json!({ "attack": 5, "health": 6 }));
                assert_eq!(keyword_kinds(&s.stats(GHOST).keywords), vec!["Divine Shield"]);
            }

            #[test]
            fn b5_e5_r682_a_declared_target_naming_it_lists_one_play_carrying_no_discards() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [WILDFIRE, SPARE, SPARE, SPARE] },
                    "p2": { "field": [GHOST] },
                }));
                let ghost = s.card(GHOST).clone();
                let at_ghost: Vec<ActionBody> = wildfire_plays(&s, P1)
                    .into_iter()
                    .filter(|play| first_target(play) == Some(at(&ghost)))
                    .collect();
                assert_eq!(at_ghost.len(), 1);
                assert!(at_ghost.iter().all(carries_no_discards));
                let at_hero: Vec<ActionBody> = wildfire_plays(&s, P1)
                    .into_iter()
                    .filter(|play| matches!(first_target(play), Some(Selection::Hero { .. })))
                    .collect();
                assert!(at_hero.iter().all(carries_no_discards));
            }

            #[test]
            fn b5_e5_r682_the_play_pays_two_random_others_two_spares_go_never_the_card_played_and_the_hit_meets_its_divine_shield() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [WILDFIRE, SPARE, SPARE, SPARE] },
                    "p2": { "field": [GHOST] },
                }));
                let spare_ids = spares(&s, P1);
                let ghost = s.card(GHOST).clone();
                let wildfire = s.card(WILDFIRE).id.clone();
                let mut nonce = 0;
                let result = send(
                    &mut nonce,
                    s.state(),
                    P1,
                    json!({ "type": "play", "instanceId": wildfire, "targets": [at(&ghost)] }),
                );
                assert!(result.error.is_none());
                let discarded = discarded_ids(&result.events);
                assert_eq!(discarded.len(), 2);
                for id in &discarded {
                    assert!(spare_ids.contains(id));
                }
                assert!(!discarded.contains(&wildfire));
                assert_eq!(result.state.players.p1.hand.len(), 1);
                assert!(spare_ids.contains(&result.state.players.p1.hand[0].id));
                assert!(
                    result
                        .events
                        .iter()
                        .any(|event| matches!(event, GameEvent::DivineShieldLost { .. }))
                );
            }

            #[test]
            fn b5_e5_a_play_naming_it_with_too_few_other_cards_held_is_refused() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WILDFIRE, SPARE] },
                    "p2": { "field": [GHOST, VANILLA] },
                }));
                let ghost = s.card(GHOST).clone();
                // Wildfire plus one spare: only one card outside the played card, fewer than the two owed.
                s.expect_refused(|s| s.play(WILDFIRE, json!({ "targets": [at(&ghost)] })));
            }

            #[test]
            fn b5_e5_with_fewer_than_2_other_cards_it_is_no_legal_target_absent_from_legalactions() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [WILDFIRE, SPARE] },
                    "p2": { "field": [GHOST, VANILLA] },
                }));
                let ghost = s.card(GHOST).clone();
                let targets: Vec<Option<Selection>> = wildfire_plays(&s, P1).iter().map(first_target).collect();
                assert!(!targets.contains(&Some(at(&ghost))));
                assert!(targets.contains(&Some(at(s.card(VANILLA)))));
            }

            #[test]
            fn b5_e5_it_binds_both_players_its_controller_too() {
                crate::register_all();
                let s = scenario(json!({ "p1": { "hand": [WILDFIRE, SPARE], "field": [GHOST] } }));
                let targets: Vec<Option<Selection>> = wildfire_plays(&s, P1).iter().map(first_target).collect();
                assert!(!targets.contains(&Some(at(s.card(GHOST)))));
                let rich = scenario(json!({ "p1": { "hand": [WILDFIRE, SPARE, SPARE], "field": [GHOST] } }));
                let own: Vec<ActionBody> = wildfire_plays(&rich, P1)
                    .into_iter()
                    .filter(|play| matches!(first_target(play), Some(Selection::Instance { .. })))
                    .collect();
                // Offered now that two others are held — and carrying no discards, paid at random.
                let own_targets: Vec<Option<Selection>> = own.iter().map(first_target).collect();
                assert!(own_targets.contains(&Some(at(rich.card(GHOST)))));
                assert!(own.iter().all(carries_no_discards));
            }

            #[test]
            fn b5_e5_r682_a_prompt_answer_naming_it_pays_the_2_random_cards_at_once_with_no_follow_up_prompt() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": ECHO, "radiant": true }, WILDFIRE, SPARE, SPARE] },
                    "p2": { "field": [GHOST, MENACE] },
                }));
                s.play(WILDFIRE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
                let menace = s.card(MENACE).clone();
                s.play(ECHO, json!({ "targets": [at(&menace)] }));
                assert!(pending_selections(&s).contains(&at(s.card(GHOST))));
                let spare_ids = spares(&s, P1);
                let ghost = s.card(GHOST).clone();
                s.answer(json!([at(&ghost)]));
                assert!(s.state().pending.is_none());
                let discarded = discarded_ids(s.events());
                assert_eq!(discarded.len(), 2);
                for id in &discarded {
                    assert!(spare_ids.contains(id));
                }
                assert!(
                    s.events()
                        .iter()
                        .any(|event| matches!(event, GameEvent::DivineShieldLost { .. }))
                );
            }

            #[test]
            fn b5_e5_a_prompt_never_offers_it_to_a_chooser_with_fewer_than_2_other_cards() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": ECHO, "radiant": true }, WILDFIRE, SPARE] },
                    "p2": { "field": [GHOST, MENACE] },
                }));
                s.play(WILDFIRE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
                let menace = s.card(MENACE).clone();
                s.play(ECHO, json!({ "targets": [at(&menace)] }));
                let picks = pending_selections(&s);
                assert!(picks.contains(&at(s.card(MENACE))));
                assert!(!picks.contains(&at(s.card(GHOST))));
            }

            #[test]
            fn b5_e5_r682_an_activations_declared_target_naming_it_pays_2_random_cards_too_c_n78_mutate_spell() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [SPARE, SPARE, SPARE], "backrow": [MUTATE] },
                    "p2": { "hand": [FILLER], "field": [{ "def": GHOST, "counters": { "plague": 1 } }] },
                }));
                let ghost = s.card(GHOST).clone();
                let mutate = s.card(MUTATE).id.clone();
                let spare_ids = spares(&s, P1);
                let offered: Vec<ActionBody> = legal_actions(s.state(), P1)
                    .into_iter()
                    .filter(|action| matches!(action, ActionBody::Activate { instance_id, .. } if *instance_id == mutate))
                    .collect();
                // R682: one action, carrying no discards.
                assert_eq!(offered.len(), 1);
                assert!(offered.iter().all(carries_no_discards));
                let mut nonce = 0;
                let result = send(
                    &mut nonce,
                    s.state(),
                    P1,
                    json!({ "type": "activate", "instanceId": mutate, "targets": [at(&ghost)] }),
                );
                assert!(result.error.is_none());
                let discarded = discarded_ids(&result.events);
                assert_eq!(discarded.len(), 2);
                for id in &discarded {
                    assert!(spare_ids.contains(id));
                }
                assert_eq!(result.state.players.p1.hand.len(), 1);
                assert!(spare_ids.contains(&result.state.players.p1.hand[0].id));
                let exiled: Vec<String> = result.state.players.p2.exile.iter().map(|card| card.id.clone()).collect();
                assert_eq!(exiled, vec![ghost.id.clone()]);
            }

            #[test]
            fn b5_e5_with_fewer_than_2_other_cards_an_activation_cant_name_it() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [SPARE], "backrow": [MUTATE] },
                    "p2": { "hand": [FILLER], "field": [{ "def": GHOST, "counters": { "plague": 1 } }] },
                }));
                let mutate = s.card(MUTATE).id.clone();
                let ghost = s.card(GHOST).clone();
                let named: Vec<ActionBody> = legal_actions(s.state(), P1)
                    .into_iter()
                    .filter(|action| match action {
                        ActionBody::Activate {
                            instance_id, targets, ..
                        } => {
                            *instance_id == mutate
                                && targets.iter().flatten().any(|pick| {
                                    matches!(pick, Selection::Instance { instance_id } if *instance_id == ghost.id)
                                })
                        }
                        _ => false,
                    })
                    .collect();
                assert!(named.is_empty());
                let mut nonce = 0;
                assert!(
                    send(
                        &mut nonce,
                        s.state(),
                        P1,
                        json!({ "type": "activate", "instanceId": mutate, "targets": [at(&ghost)] }),
                    )
                    .error
                    .is_some()
                );
            }

            #[test]
            fn r394_a_random_pick_targets_nothing_c_n22_mid_runners_random_bounce_returns_it_with_no_discard() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [MID_RUNNER, FILLER] },
                    "p2": { "field": [GHOST], "hand": [FILLER] },
                }));
                s.play(MID_RUNNER, json!({ "zone": 1 }));
                s.expect_in_zone(GHOST, "hand");
                assert!(!s.events().iter().any(|event| matches!(event, GameEvent::Discarded { .. })));
                assert!(s.state().pending.is_none());
            }

            #[test]
            fn r394_an_attack_is_no_targeting_it_is_attacked_with_no_discard() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [MENACE] },
                    "p2": { "field": [GHOST], "hand": [FILLER] },
                }));
                s.attack(MENACE, GHOST);
                assert!(!s.last_events().iter().any(|event| matches!(event, GameEvent::Discarded { .. })));
                assert!(
                    s.last_events()
                        .iter()
                        .any(|event| matches!(event, GameEvent::DivineShieldLost { .. }))
                );
            }

            #[test]
            fn r394_an_all_effect_targets_nothing_twisting_nether_destroys_it_with_no_discard() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [NETHER, FILLER] }, "p2": { "field": [GHOST] } }));
                s.play(NETHER, json!({}));
                s.expect_in_zone(GHOST, "graveyard");
                assert!(!s.last_events().iter().any(|event| matches!(event, GameEvent::Discarded { .. })));
            }

            #[test]
            fn the_discards_are_discards_c_n64_malzahars_recycler_draws_for_them() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [WILDFIRE, SPARE, SPARE], "backrow": [RECYCLER], "library": [VANILLA, VANILLA, VANILLA] },
                    "p2": { "field": [GHOST] },
                }));
                let wildfire = s.card(WILDFIRE).id.clone();
                let ghost = s.card(GHOST).clone();
                let mut nonce = 0;
                let result = send(
                    &mut nonce,
                    s.state(),
                    P1,
                    json!({ "type": "play", "instanceId": wildfire, "targets": [at(&ghost)] }),
                );
                assert!(result.error.is_none());
                let drawn = result
                    .events
                    .iter()
                    .filter(|event| matches!(event, GameEvent::Drawn { player: PlayerId::P1, .. }))
                    .count();
                assert_eq!(drawn, 2);
                assert_eq!(result.state.players.p1.hand.len(), 2);
            }

            #[test]
            fn r386_its_discard_is_the_declared_number_an_upgrades_step_makes_it_3_and_2_others_no_longer_pay() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [WILDFIRE, SPARE, SPARE] }, "p2": { "field": [GHOST] } }));
                step_param(s.card_mut(GHOST), "discard", 1);
                let targets: Vec<Option<Selection>> = wildfire_plays(&s, P1).iter().map(first_target).collect();
                assert!(!targets.contains(&Some(at(s.card(GHOST)))));
                let mut rich = scenario(json!({
                    "p1": { "hand": [WILDFIRE, SPARE, SPARE, SPARE, SPARE] },
                    "p2": { "field": [GHOST] },
                }));
                step_param(rich.card_mut(GHOST), "discard", 1);
                // Three others now owed: offered with four others held — still carrying no discards.
                let ghost = rich.card(GHOST).clone();
                let at_ghost: Vec<ActionBody> = wildfire_plays(&rich, P1)
                    .into_iter()
                    .filter(|play| first_target(play) == Some(at(&ghost)))
                    .collect();
                assert_eq!(at_ghost.len(), 1);
                assert!(at_ghost.iter().all(carries_no_discards));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn is_a_10_12_with_divine_shield_and_reborn() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": GHOST, "radiant": true }, FILLER] } }));
                s.play(GHOST, json!({}))
                    .expect_stats(GHOST, json!({ "attack": 10, "health": 12 }));
                assert_eq!(keyword_kinds(&s.stats(GHOST).keywords), vec!["Divine Shield", "Reborn"]);
            }

            #[test]
            fn s4_5_reborn_destroyed_it_returns_at_1_health_and_targeting_it_still_costs_2_random_cards() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [NETHER, WILDFIRE, SPARE, SPARE], "mana": 9 },
                    "p2": { "field": [{ "def": GHOST, "radiant": true }] },
                }));
                s.play(NETHER, json!({}));
                s.expect_in_zone(GHOST, "field")
                    .expect_stats(GHOST, json!({ "health": 1 }));
                let ghost = s.card(GHOST).clone();
                let at_ghost: Vec<ActionBody> = wildfire_plays(&s, P1)
                    .into_iter()
                    .filter(|play| first_target(play) == Some(at(&ghost)))
                    .collect();
                assert!(!at_ghost.is_empty());
                assert!(at_ghost.iter().all(carries_no_discards));
            }

            #[test]
            fn b5_e5_the_same_cost_binds_on_the_radiant_face_too_few_other_cards_no_target() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [WILDFIRE, SPARE] },
                    "p2": { "field": [{ "def": GHOST, "radiant": true }] },
                }));
                let targets: Vec<Option<Selection>> = wildfire_plays(&s, P1).iter().map(first_target).collect();
                assert!(!targets.contains(&Some(at(s.card(GHOST)))));
            }
        }
    }
}
