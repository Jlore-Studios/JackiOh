//! C #54 Rewind (SPEC §8.6 row 54; §6.3 Trigger a Cry; R70, R81, R90, R386). Spell, cost 1, Common.
//!   Base:    "Trigger a Cry of one of your Units on the field or in your graveyard." (once, not tunable, R749)
//!   Radiant: "Trigger the Cry of any Unit on the field or in a graveyard, {repeats|time|times}." (2)
//!   Engine:  the declared target (R81) is a Unit that has a Cry — the top of a pile on the field, or a
//!            Unit card in a graveyard; its Cry runs with that unit as `self`, under your control, its
//!            choices yours as prompts (R70); out of a graveyard "this" finds nothing. Radiant: either
//!            side, two separate runs, each with its own choices.
//!
//! `triggerCry` is the engine's whole sequence; this card only declares the pick (a `check` on
//! `hasTriggerableCry`) and runs it `param(ctx, "repeats")` times on the chosen Unit.

use jackioh_engine::effects::{has_triggerable_cry, trigger_cry};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-054";

fn rewind(side: FilterSide) -> Script {
    Script {
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": side, "of": ["unit", "graveyard"], "check": "hasCry" }),
        )],
        target_checks: IndexMap::from([(
            "hasCry",
            target_check(|args| args.candidate.is_some_and(|candidate| has_triggerable_cry(args.state, candidate))),
        )]),
        cry: Some(hook(|ctx| (0..param(ctx, "repeats")).map(|_| trigger_cry(Default::default())).collect())),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = rewind(FilterSide::Ally);

    let radiant = rewind(FilterSide::Any);

    CardScripts { base, radiant }
}

// C #54 Rewind — SPEC §8.6 row 54, BUILD M9 Classic row C 54: "A declared target: one of your Units on
// the field (the top of a pile) or a Unit card in your graveyard that has a Cry; its Cry runs with that
// Unit as `self` under your control, its choices yours as prompts (R70's caster picks); a Cry that acts
// on "this" finds nothing when the Unit is in a graveyard; running a Cry is not a play of that Unit; no
// Unit with a Cry → it may still be played and fizzles, counting as played (§8's conventions, R90);
// radiant: any Unit with a Cry on the field or in either graveyard, its Cry run twice, each run with
// its own choices; its tuned number (repeats) reads through `param()` (R386)". The base face prints no
// repeats, so it is tuned on the Radiant face only (R749).
//
// The Cries are Core cards with their own tests: Gary the Gambler (flips coins to buff itself), Me and
// Mr Token (summons a Rush Token), Duplicating Felinors (summons a copy of itself), Bigot (destroys a
// target enemy non-Human Unit) and Twisted Sorcerer (deals 4 damage to a target). Mr. Vanilla has no
// Cry; Felinor Fiender's Stack buries a card beneath it.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::effects::{TuneDirection, applicable_changes};
    use jackioh_engine::testkit::*;
    use std::collections::BTreeSet;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const REWIND: &str = "classic-054";
    const GARY: &str = "core-004"; // 1/1, Cry: flip 5 coins, +1 attack per heads, +1 health per tails.
    const MR_TOKEN: &str = "core-015"; // 1/1, Cry: summon a Rush Token.
    const DUPLICATING: &str = "core-012"; // 3/4, Cry: summon a copy of this.
    const BIGOT: &str = "core-002"; // 6/1 Human, Cry: destroy target enemy non-Human Unit.
    const SORCERER: &str = "core-068"; // 5/5, Cry: deal 4 damage to a target (8 under 10 health).
    const VANILLA: &str = "core-008"; // 4/4, no text.
    const MENACE: &str = "core-019"; // 9/9 Taunt.
    const FIENDER: &str = "core-092"; // Stack.
    const RUSH_TOKEN: &str = "core-t-rush";
    const LUNAR: &str = "core-035";
    const FILLER: &str = "core-005";

    /// An engine value as the JSON the TS test reads (SURFACE §5.1: the same keys and values).
    fn js<T: serde::Serialize + ?Sized>(value: &T) -> Value {
        serde_json::to_value(value).expect("an engine value serialises")
    }

    /// TS `JSON.parse(JSON.stringify(state))`: written and read back field by field, in field order.
    fn round_trip(state: &GameState) -> GameState {
        let text = serde_json::to_string(state).expect("the state serialises");
        serde_json::from_str(&text).expect("the state parses back")
    }

    /// TS `stepParam(s.card(ref), key, steps)`: the step recorded on the live card.
    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        let live = find_instance_mut(s.state_mut(), &id).expect("the card is in the game");
        step_param(live, key, steps);
    }

    /// TS `applicableChanges(s.state, s.card(ref), direction)`, each row as its JSON name.
    fn changes(s: &Scenario, card: &str, direction: TuneDirection) -> Vec<Value> {
        applicable_changes(s.state(), s.card(card), direction).iter().map(js).collect()
    }

    fn pick(id: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": id }])
    }

    /// The instance ids `legalActions` offers as Rewind's target.
    fn offered(s: &Scenario, player: PlayerId) -> Vec<String> {
        let rewind = s
            .hand(player)
            .into_iter()
            .find(|card| card.def_id == REWIND)
            .expect("Rewind should be in hand");
        legal_actions(s.state(), player)
            .iter()
            .map(js)
            .filter(|action| action["type"] == "play" && action["instanceId"] == rewind.id.as_str())
            .flat_map(|action| {
                action["targets"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|selection| selection["pick"] == "instance")
                    .filter_map(|selection| selection["instanceId"].as_str().map(str::to_string))
                    .collect::<Vec<String>>()
            })
            .collect()
    }

    fn tokens_of(s: &Scenario, player: PlayerId) -> usize {
        (1..=5)
            .filter(|lane| s.unit(player, *lane).is_some_and(|unit| unit.def_id == RUSH_TOKEN))
            .count()
    }

    mod c_54_rewind {
        use super::*;

        #[test]
        fn declares_one_target_a_unit_with_a_cry_on_the_field_or_in_a_graveyard_your_side_on_the_base_face_either_on_the_radiant()
         {
            crate::register_all();
            let def = js(&crate::card_def(ID));
            assert_eq!(def["id"], REWIND);
            assert_eq!(
                def["params"],
                json!([{ "key": "repeats", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1, "tunedOn": "radiant" }]),
            );
            let scripts = script();
            assert_eq!(
                js(&scripts.base.targets),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "ally", "of": ["unit", "graveyard"], "check": "hasCry" } }]),
            );
            assert_eq!(
                js(&scripts.radiant.targets),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "graveyard"], "check": "hasCry" } }]),
            );
        }

        mod base {
            use super::*;

            #[test]
            fn on_the_field_the_cry_runs_with_that_unit_as_itself_gary_flips_its_coins_and_buffs_itself() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [REWIND, FILLER], "field": [GARY] }, "p2": { "hand": [FILLER] } }));
                let gary = s.card(GARY).clone();
                s.play(REWIND, json!({ "targets": pick(&gary.id) }));
                let stats = s.stats(&gary);
                // Five coins, one point each, split between attack and health.
                assert_eq!(stats.attack + stats.max_health, 1 + 1 + 5);
                s.expect_events(json!(["cardPlayed", "buffed"]));
            }

            #[test]
            fn in_your_graveyard_the_cry_runs_under_your_control_me_and_mr_token_summons_a_rush_token_for_you() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [REWIND, FILLER], "graveyard": [MR_TOKEN] }, "p2": { "hand": [FILLER] } }));
                let mr_token = s.card(MR_TOKEN).id.clone();
                s.play(REWIND, json!({ "targets": pick(&mr_token) }));
                assert_eq!(tokens_of(&s, P1), 1);
                s.expect_in_zone(MR_TOKEN, "graveyard");
            }

            #[test]
            fn a_cry_that_acts_on_this_finds_nothing_in_a_graveyard_gary_buffs_nothing_duplicating_felinors_copies_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [REWIND, REWIND, FILLER], "graveyard": [GARY, DUPLICATING] },
                    "p2": { "hand": [FILLER] },
                }));
                let hand = s.hand(P1);
                let (Some(first), Some(second)) = (hand.first().cloned(), hand.get(1).cloned()) else {
                    panic!("two Rewinds should be in hand");
                };
                let gary = s.card(GARY).id.clone();
                s.play(&first, json!({ "targets": pick(&gary) }));
                let duplicating = s.card(DUPLICATING).id.clone();
                s.play(&second, json!({ "targets": pick(&duplicating) }));
                assert!(!s.events().iter().map(js).any(|event| event["type"] == "buffed" || event["type"] == "summoned"));
                assert_eq!(units(&s, P1), vec![None, None, None, None, None]);
            }

            #[test]
            fn r70_its_choices_are_yours_as_prompts_bigot_asks_you_which_enemy_non_human_unit_to_destroy() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [REWIND, FILLER], "field": [BIGOT] },
                    "p2": { "hand": [FILLER], "field": [VANILLA, DUPLICATING, MENACE] },
                }));
                let bigot = s.card(BIGOT).id.clone();
                s.play(REWIND, json!({ "targets": pick(&bigot) }));
                let pending = js(&s.state().pending);
                assert_eq!(pending["playerId"], "p1");
                let selections: Vec<Value> = pending["options"]
                    .as_array()
                    .map(|options| options.iter().map(|option| option["selection"].clone()).collect())
                    .unwrap_or_default();
                assert_eq!(
                    selections,
                    vec![
                        json!({ "pick": "instance", "instanceId": s.card(DUPLICATING).id }),
                        json!({ "pick": "instance", "instanceId": s.card(MENACE).id }),
                    ],
                );
                assert_eq!(js(&s.view(P2).pending), json!({ "forYou": false, "pendingFor": "p1" }));
                let menace = s.card(MENACE).id.clone();
                s.answer(json!(menace));
                s.expect_in_zone(MENACE, "graveyard").expect_in_zone(DUPLICATING, "field");
            }

            #[test]
            fn running_a_cry_is_not_a_play_of_that_unit_only_rewind_counts_as_played() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [REWIND, FILLER], "field": [MR_TOKEN] }, "p2": { "hand": [FILLER] } }));
                let mr_token = s.card(MR_TOKEN).id.clone();
                s.play(REWIND, json!({ "targets": pick(&mr_token) }));
                assert_eq!(cards_played_this_turn(s.state(), P1), 1);
                let played: Vec<Value> = s
                    .events()
                    .iter()
                    .map(js)
                    .filter(|event| event["type"] == "cardPlayed")
                    .map(|event| event["defId"].clone())
                    .collect();
                assert_eq!(played, vec![json!(REWIND)]);
                assert_eq!(tokens_of(&s, P1), 1);
            }

            #[test]
            fn r90_with_no_unit_that_has_a_cry_it_is_still_played_fizzles_and_counts_as_played() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [REWIND, FILLER], "field": [VANILLA], "graveyard": [LUNAR] },
                    "p2": { "hand": [FILLER], "field": [GARY] },
                }));
                let rewind = s.card(REWIND).clone();
                assert!(
                    legal_actions(s.state(), P1)
                        .iter()
                        .map(js)
                        .any(|action| action["type"] == "play" && action["instanceId"] == rewind.id.as_str())
                );
                assert_eq!(offered(&s, P1), Vec::<String>::new());
                s.play(&rewind, json!({}));
                s.expect_in_zone(&rewind, "graveyard").expect_events(json!(["cardPlayed", "cardResolved"]));
                assert_eq!(cards_played_this_turn(s.state(), P1), 1);
                assert!(!s.events().iter().map(js).any(|event| event["type"] == "buffed"));
            }

            #[test]
            fn r90_only_your_units_are_offered_an_enemy_unit_and_the_enemy_graveyard_are_refused_and_legalactions_agrees() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [REWIND, FILLER], "field": [GARY], "graveyard": [MR_TOKEN] },
                    "p2": { "hand": [FILLER], "field": [SORCERER], "graveyard": [BIGOT] },
                }));
                assert_eq!(
                    offered(&s, P1).into_iter().collect::<BTreeSet<String>>(),
                    BTreeSet::from([s.card(GARY).id.clone(), s.card(MR_TOKEN).id.clone()]),
                );
                let sorcerer = s.card(SORCERER).id.clone();
                let bigot = s.card(BIGOT).id.clone();
                s.expect_refused(|s| s.play(REWIND, json!({ "targets": pick(&sorcerer) })));
                s.expect_refused(|s| s.play(REWIND, json!({ "targets": pick(&bigot) })));
                s.expect_in_zone(REWIND, "hand");
            }

            #[test]
            fn a_unit_with_no_cry_a_spell_in_the_graveyard_and_a_card_dormant_under_a_stack_pile_are_never_offered_r13() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [REWIND, FILLER], "field": [VANILLA, GARY, { "def": FIENDER, "stack": true }], "graveyard": [LUNAR] },
                    "p2": { "hand": [FILLER] },
                }));
                assert_eq!(offered(&s, P1), Vec::<String>::new());
                let gary = s.card(GARY).id.clone();
                s.expect_refused(|s| s.play(REWIND, json!({ "targets": pick(&gary) })));
            }

            #[test]
            fn a_paused_triggered_cry_survives_a_round_trip_the_frozen_state_answers_to_the_same_game() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [REWIND, FILLER], "field": [BIGOT] }, "p2": { "hand": [FILLER], "field": [MENACE] } }));
                let bigot = s.card(BIGOT).id.clone();
                s.play(REWIND, json!({ "targets": pick(&bigot) }));
                let pending = s.state().pending.clone();
                assert!(pending.is_some());
                let thawed = round_trip(s.state());
                assert_eq!(&thawed, s.state());
                let answer: Action = json_as(json!({
                    "type": "answer",
                    "choiceId": pending.as_ref().map(|choice| choice.id.clone()).unwrap_or_default(),
                    "selection": pick(&s.card(MENACE).id),
                    "playerId": "p1",
                    "nonce": "rewind-roundtrip",
                }));
                let live = reduce(s.state(), &answer);
                let frozen = reduce(&thawed, &answer);
                assert!(live.error.is_none());
                assert_eq!(frozen.state, live.state);
                assert_eq!(frozen.events, live.events);
                assert_eq!(
                    live.state.players.p2.graveyard.iter().map(|card| card.def_id.as_str()).collect::<Vec<&str>>(),
                    vec![MENACE],
                );
            }

            #[test]
            fn r749_the_base_faces_repeats_is_not_tunable_an_upgrades_menu_offers_no_number_and_a_recorded_step_still_triggers_once() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [REWIND, FILLER], "graveyard": [MR_TOKEN] }, "p2": { "hand": [FILLER] } }));
                assert!(!changes(&s, REWIND, TuneDirection::Upgrade).contains(&json!("number")));
                assert!(!changes(&s, REWIND, TuneDirection::Degrade).contains(&json!("number")));
                step(&mut s, REWIND, "repeats", 1);
                let mr_token = s.card(MR_TOKEN).id.clone();
                s.play(REWIND, json!({ "targets": pick(&mr_token) }));
                assert_eq!(tokens_of(&s, P1), 1);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn reaches_an_enemy_unit_on_the_field_its_cry_runs_under_your_control_the_rush_tokens_are_yours_twice() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": REWIND, "radiant": true }, FILLER] },
                    "p2": { "hand": [FILLER], "field": [MR_TOKEN] },
                }));
                let mr_token = s.card(MR_TOKEN).id.clone();
                s.play(REWIND, json!({ "targets": pick(&mr_token) }));
                assert_eq!(tokens_of(&s, P1), 2);
                assert_eq!(tokens_of(&s, P2), 0);
            }

            #[test]
            fn reaches_either_graveyard() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": REWIND, "radiant": true }, FILLER], "graveyard": [GARY] },
                    "p2": { "hand": [FILLER], "graveyard": [MR_TOKEN] },
                }));
                assert_eq!(
                    offered(&s, P1).into_iter().collect::<BTreeSet<String>>(),
                    BTreeSet::from([s.card(GARY).id.clone(), s.card(MR_TOKEN).id.clone()]),
                );
                let mr_token = s.card(MR_TOKEN).id.clone();
                s.play(REWIND, json!({ "targets": pick(&mr_token) }));
                assert_eq!(tokens_of(&s, P1), 2);
            }

            #[test]
            fn runs_the_cry_twice_each_run_with_its_own_choices_asked_of_you_one_after_the_other() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": REWIND, "radiant": true }, FILLER], "field": [SORCERER] },
                    "p2": { "hand": [FILLER], "field": [MENACE] },
                }));
                let sorcerer = s.card(SORCERER).id.clone();
                s.play(REWIND, json!({ "targets": pick(&sorcerer) }));
                assert_eq!(js(&s.state().pending)["playerId"], "p1");
                let menace = s.card(MENACE).id.clone();
                s.answer(json!(menace));
                assert_eq!(js(&s.state().pending)["playerId"], "p1");
                s.answer(json!("hero:p2"));
                assert!(s.state().pending.is_none());
                s.expect_stats(MENACE, json!({ "health": 5 })).expect_health(P2, 26);
            }

            #[test]
            fn a_pause_between_the_two_runs_survives_a_round_trip() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": REWIND, "radiant": true }, FILLER], "field": [SORCERER] },
                    "p2": { "hand": [FILLER], "field": [MENACE] },
                }));
                let sorcerer = s.card(SORCERER).id.clone();
                s.play(REWIND, json!({ "targets": pick(&sorcerer) }));
                let menace = s.card(MENACE).id.clone();
                s.answer(json!(menace));
                let pending = s.state().pending.clone();
                assert!(pending.is_some());
                let thawed = round_trip(s.state());
                let answer: Action = json_as(json!({
                    "type": "answer",
                    "choiceId": pending.as_ref().map(|choice| choice.id.clone()).unwrap_or_default(),
                    "selection": [{ "pick": "hero", "player": "p2" }],
                    "playerId": "p1",
                    "nonce": "rewind-radiant-roundtrip",
                }));
                let live = reduce(s.state(), &answer);
                let frozen = reduce(&thawed, &answer);
                assert!(live.error.is_none());
                assert_eq!(frozen.state, live.state);
                assert_eq!(live.state.players.p2.hero.health, 26);
            }

            #[test]
            fn a_unit_with_no_cry_is_never_offered() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [{ "def": REWIND, "radiant": true }, FILLER], "field": [VANILLA] },
                    "p2": { "hand": [FILLER], "field": [VANILLA] },
                }));
                assert_eq!(offered(&s, P1), Vec::<String>::new());
            }

            #[test]
            fn r386_a_degrade_triggers_the_cry_once() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": REWIND, "radiant": true }, FILLER], "graveyard": [MR_TOKEN] },
                    "p2": { "hand": [FILLER] },
                }));
                assert!(changes(&s, REWIND, TuneDirection::Degrade).contains(&json!("number")));
                step(&mut s, REWIND, "repeats", -1);
                let mr_token = s.card(MR_TOKEN).id.clone();
                s.play(REWIND, json!({ "targets": pick(&mr_token) }));
                assert_eq!(tokens_of(&s, P1), 1);
            }
        }
    }

    fn units(s: &Scenario, player: PlayerId) -> Vec<Option<String>> {
        (1..=5).map(|lane| s.unit(player, lane).map(|unit| unit.def_id)).collect()
    }
}
