//! C #40 MC Tech (SPEC §8.6 row 40). (1) Unit, Rare, 3/3 → 6/6.
//!   Base:    "Cry: If your opponent controls {threshold} or more permanents, steal a random one." — 4
//!   Radiant: "Cry: If your opponent controls {threshold} or more permanents, steal one of your choice." — 4
//!
//! Permanents your opponent controls: the top card of each of their unit piles (a card dormant under
//! a Stack is not on the field, R13) and every backrow card, face-down ones included; the count is
//! public. It is read as the Cry resolves. Base: a random pick (R60) from the match rng. Radiant: a
//! target prompt opened then (§10.6), since the condition is only known at resolution (R81); a
//! face-down option names nothing but its id to its chooser (R177).
//! §6.3 Steal: R15 places it in the same lane of the controller's row if free, else the first free zone;
//! with none it stays with its owner. The change of control is an entry (R171). R33: a stolen
//! face-down trap stays face-down, read by its new controller. R195: in hand the card glows when the
//! opponent controls enough permanents now. The threshold is the declared `threshold` (R386).

use jackioh_engine::prelude::*;
use jackioh_engine::effects::{ForEachCardArgs, choose_target, for_each_card, steal};

pub const ID: &str = "classic-040";

/// The Radiant pick's resume step.
const STEAL_STEP: &str = "steal";

/// The permanents `player`'s opponent controls: the tops of their unit piles, then their backrow.
fn enemy_permanents(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    let enemy = opponent_of(player);
    let backrow: Vec<CardInstance> = slots_of(enemy, Row::Backrow)
        .iter()
        .flat_map(|slot| card_at(state, slot).cloned())
        .collect();
    let mut permanents: Vec<CardInstance> =
        active_units_of(state, enemy).into_iter().cloned().collect();
    permanents.extend(backrow);
    permanents
}

/// "If your opponent controls {threshold} or more permanents": the one test the Cry and the glow make.
/// The Cry's and the glow's contexts are different types, so the caller hands over the three readings.
fn enough_permanents(state: &GameState, controller: PlayerId, threshold: i32) -> bool {
    enemy_permanents(state, controller).len() as i32 >= threshold
}

/// R195: in hand, whether playing it now would steal.
fn condition_met(c: ConditionContext) -> bool {
    c.zone == ConditionZone::Hand && enough_permanents(c.state, c.controller, param(&c, "threshold"))
}

/// R60: one of them at random, drawn as the Cry reaches the clause.
fn random_permanent(c: &mut EffectContext) -> Vec<String> {
    let permanents = enemy_permanents(c.state, c.controller);
    c.rng.shuffle(&permanents).into_iter().take(1).map(|card| card.id).collect()
}

fn steal_it(instance_id: &str) -> Effect {
    steal(json_as(json!({ "instanceId": instance_id })))
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            let threshold = param(ctx, "threshold");
            if enough_permanents(ctx.state, ctx.controller, threshold) {
                vec![for_each_card(ForEachCardArgs {
                    cards: Arc::new(random_permanent),
                    each: Arc::new(steal_it),
                })]
            } else {
                vec![]
            }
        })),
        condition_met: Some(condition_hook(condition_met)),
        ..Script::default()
    };

    let radiant = Script {
        cry: Some(hook(|ctx| {
            let threshold = param(ctx, "threshold");
            if enough_permanents(ctx.state, ctx.controller, threshold) {
                vec![choose_target(json_as(json!({
                    "step": STEAL_STEP,
                    "scope": { "side": "enemy", "of": ["unit", "backrow"] },
                    "prompt": "Steal one of their permanents",
                })))]
            } else {
                vec![]
            }
        })),
        resume: IndexMap::from([(STEAL_STEP, hook(|_ctx| vec![steal(json_as(json!({ "target": { "of": "chosen" } })))]))]),
        condition_met: Some(condition_hook(condition_met)),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #40 MC Tech — SPEC §8.6 row 40, BUILD M9 Classic row C 40: at resolution, if the opponent controls 4+
// permanents (unit-pile tops and backrow, face-down included; dormant cards don't count, R13), steal one at
// random (R60), placed per R15, an entry (R171); radiant 6/6: you pick, a face-down option carrying only
// its id (R177); a stolen face-down trap is read by you (R33); threshold reads through `param()` (R386).
// The `conditionMet` proofs (R195) are in `../condition-active.test.ts`, with the other cards'.
// The face-down traps used below never fire during these plays: My Pawn answers only a lethal attack,
// Bread and Butter and Intern Stimmy the end of a turn, and Unlicensed Experimentation a permanent of a
// type its controller controls (p2 controls no Unit when they hold only traps).
#[cfg(test)]
mod tests {
    use super::{script, ID};
    use jackioh_engine::testkit::*;

    const TECH: &str = "classic-040";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9.
    const TEMPO: &str = "core-011"; // (1) Unit 3/3 Rush.
    const FIENDER: &str = "core-092"; // Felinor Fiender: Stack.
    const FIELD_SPELL: &str = "core-073"; // (2) Field Spell.
    const PAWN: &str = "core-096";
    const BREAD: &str = "core-018";
    const STIMMY: &str = "core-071";
    const UNLICENSED: &str = "core-085";
    const ANCHOR: &str = "core-010";

    use crate::js;

    /// The four traps, each face-down.
    fn face_down_traps() -> Value {
        Value::Array([PAWN, BREAD, STIMMY, UNLICENSED].iter().map(|d| json!({ "def": d, "faceUp": false })).collect())
    }

    fn stolen_ids(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .map(js)
            .filter(|event| event["type"] == "controlChanged")
            .filter_map(|event| event["instanceId"].as_str().map(String::from))
            .collect()
    }

    fn first_or_empty(ids: &[String]) -> String {
        ids.first().cloned().unwrap_or_default()
    }

    mod c_40_mc_tech {
        use super::*;

        #[test]
        fn has_a_script_per_face_and_the_radiant_pick_answers_into_its_resume_step() {
            crate::register_all();
            assert_eq!(crate::card_def(ID).id, TECH);
            let scripts = script();
            assert!(scripts.base.cry.is_some());
            assert!(scripts.radiant.resume.contains_key("steal"));
        }

        mod base {
            use super::*;

            #[test]
            fn is_a_3_3() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "field": [TECH], "hand": [ANCHOR] } }));
                s.expect_stats(TECH, json!({ "attack": 3, "health": 3 }));
            }

            #[test]
            fn r60_with_4_enemy_permanents_it_steals_one_at_random_which_is_now_yours() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TECH, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, MENACE, TEMPO], "backrow": [FIELD_SPELL] },
                }));

                s.play(TECH, json!({ "zone": 5 }));

                let stolen = stolen_ids(&s);
                assert_eq!(stolen.len(), 1);
                let card = s.card(first_or_empty(&stolen));
                assert_eq!(card.controller, PlayerId::P1);
                assert_eq!(card.owner, PlayerId::P2);
            }

            #[test]
            fn r15_a_stolen_unit_lands_in_the_same_lane_of_your_row_when_it_is_free() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TECH, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, VANILLA, VANILLA, VANILLA] },
                }));

                s.play(TECH, json!({ "zone": 5 }));

                let stolen = s.card(first_or_empty(&stolen_ids(&s))).id.clone();
                let lane = [1, 2, 3, 4].into_iter().find(|&l| s.unit(PlayerId::P1, l).is_some_and(|unit| unit.id == stolen));
                assert!(lane.is_some());
                assert!(s.unit(PlayerId::P2, lane.unwrap_or(0)).is_none());
            }

            #[test]
            fn r15_with_no_free_zone_in_your_row_the_pick_stays_with_them_and_nothing_is_stolen() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TECH, ANCHOR], "field": [MENACE, MENACE, MENACE, MENACE] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, VANILLA, VANILLA, VANILLA] },
                }));

                s.play(TECH, json!({ "zone": 5 }));

                assert!(stolen_ids(&s).is_empty());
                let defs: Vec<Option<String>> =
                    [1, 2, 3, 4].into_iter().map(|l| s.unit(PlayerId::P2, l).map(|unit| unit.def_id.clone())).collect();
                assert_eq!(
                    defs,
                    vec![
                        Some(VANILLA.to_string()),
                        Some(VANILLA.to_string()),
                        Some(VANILLA.to_string()),
                        Some(VANILLA.to_string()),
                    ],
                );
            }

            #[test]
            fn c3_or_fewer_enemy_permanents_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TECH, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, MENACE], "backrow": [FIELD_SPELL] },
                }));
                let cursor = s.state().rng_cursor;

                s.play(TECH, json!({ "zone": 5 }));

                assert!(stolen_ids(&s).is_empty());
                assert_eq!(s.state().rng_cursor, cursor);
            }

            #[test]
            fn r13_a_card_dormant_under_a_stack_pile_does_not_count_a_face_down_trap_does() {
                crate::register_all();
                let mut dormant = scenario(json!({
                    "p1": { "hand": [TECH, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, { "def": FIENDER, "stack": true }, MENACE, TEMPO] },
                }));
                dormant.play(TECH, json!({ "zone": 5 }));
                assert!(stolen_ids(&dormant).is_empty());

                let mut trap = scenario(json!({
                    "p1": { "hand": [TECH, ANCHOR] },
                    "p2": {
                        "hand": [ANCHOR],
                        "field": [VANILLA, { "def": FIENDER, "stack": true }, MENACE, TEMPO],
                        "backrow": [{ "def": PAWN, "faceUp": false }],
                    },
                }));
                trap.play(TECH, json!({ "zone": 5 }));
                assert_eq!(stolen_ids(&trap).len(), 1);
            }

            #[test]
            fn r171_the_steal_is_an_entry_a_stolen_unit_without_rush_or_charge_cant_attack_this_turn() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TECH, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, VANILLA, VANILLA, VANILLA] },
                }));

                s.play(TECH, json!({ "zone": 5 }));

                let stolen = s.card(first_or_empty(&stolen_ids(&s))).id.clone();
                s.expect_refused(|s| {
                    s.attack(&stolen, "hero")
                });
            }

            #[test]
            fn r33_a_stolen_face_down_trap_stays_face_down_and_is_read_by_you_from_then_on_not_by_them() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [TECH, ANCHOR] }, "p2": { "hand": [ANCHOR], "backrow": face_down_traps() } }));

                s.play(TECH, json!({ "zone": 5 }));

                let stolen_id = s.card(first_or_empty(&stolen_ids(&s))).id.clone();
                let stolen = js(s.card(&stolen_id));
                assert_eq!(stolen["zone"]["z"], "field");
                assert_eq!(stolen["zone"]["player"], "p1");
                assert_eq!(stolen["zone"]["row"], "backrow");
                assert_ne!(stolen["faceUp"], true);
                let def_id = s.card(&stolen_id).def_id.clone();
                assert!(js(&s.view(PlayerId::P1).you.backrow).to_string().contains(&def_id));
                assert!(!js(&s.view(PlayerId::P2)).to_string().contains(&def_id));
            }

            #[test]
            fn r386_a_degrade_of_the_threshold_to_5_stops_4_permanents_an_upgrade_to_3_lets_3_be_enough() {
                crate::register_all();
                let mut harder = scenario(json!({
                    "p1": { "hand": [TECH, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, MENACE, TEMPO, VANILLA] },
                }));
                step_param(harder.card_mut(TECH), "threshold", 1);
                harder.play(TECH, json!({ "zone": 5 }));
                assert!(stolen_ids(&harder).is_empty());

                let mut easier = scenario(json!({
                    "p1": { "hand": [TECH, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, MENACE, TEMPO] },
                }));
                step_param(easier.card_mut(TECH), "threshold", -1);
                easier.play(TECH, json!({ "zone": 5 }));
                assert_eq!(stolen_ids(&easier).len(), 1);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn is_a_6_6_that_with_4_enemy_permanents_asks_which_one_to_steal_and_steals_that_one() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": TECH, "radiant": true }, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, MENACE, TEMPO], "backrow": [FIELD_SPELL] },
                }));

                s.play(TECH, json!({ "zone": 5 }));

                s.expect_stats(TECH, json!({ "attack": 6, "health": 6 }));
                let pending = js(&s.state().pending);
                assert_eq!(pending["playerId"], "p1");
                assert_eq!(pending["options"].as_array().map(Vec::len), Some(4));
                let menace = s.card(MENACE).id.clone();
                s.answer(json!([{ "pick": "instance", "instanceId": menace }]));

                assert_eq!(stolen_ids(&s), vec![menace.clone()]);
                assert_eq!(s.card(&menace).controller, PlayerId::P1);
            }

            #[test]
            fn r13_the_prompt_offers_the_top_of_a_stack_pile_and_never_the_card_beneath_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": TECH, "radiant": true }, ANCHOR] },
                    "p2": {
                        "hand": [ANCHOR],
                        "field": [VANILLA, { "def": FIENDER, "stack": true }, MENACE, TEMPO],
                        "backrow": [FIELD_SPELL],
                    },
                }));

                s.play(TECH, json!({ "zone": 5 }));

                let offered: Vec<Value> = js(&s.state().pending)["options"]
                    .as_array()
                    .map(|options| options.iter().map(|option| option["selection"].clone()).collect())
                    .unwrap_or_default();
                assert_eq!(offered.len(), 4);
                let fiender = s.card(FIENDER).id.clone();
                let vanilla = s.card(VANILLA).id.clone();
                assert!(offered.contains(&json!({ "pick": "instance", "instanceId": fiender })));
                assert!(!offered.contains(&json!({ "pick": "instance", "instanceId": vanilla })));
            }

            #[test]
            fn r15_with_your_unit_row_full_the_prompt_still_opens_and_a_unit_you_pick_stays_with_them() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": TECH, "radiant": true }, ANCHOR], "field": [MENACE, MENACE, MENACE, MENACE] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, VANILLA, VANILLA, VANILLA] },
                }));
                s.play(TECH, json!({ "zone": 5 }));
                let picked = s.unit(PlayerId::P2, 1).expect("a Vanilla in lane 1").id.clone();

                s.answer(json!([{ "pick": "instance", "instanceId": picked }]));

                assert!(stolen_ids(&s).is_empty());
                assert_eq!(s.unit(PlayerId::P2, 1).map(|unit| unit.id.clone()), Some(picked.clone()));
                assert_eq!(s.card(&picked).controller, PlayerId::P2);
            }

            #[test]
            fn c3_or_fewer_no_prompt_opens_and_nothing_is_stolen() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": TECH, "radiant": true }, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, MENACE, TEMPO] },
                }));

                s.play(TECH, json!({ "zone": 5 }));

                assert!(s.state().pending.is_none());
                assert!(stolen_ids(&s).is_empty());
            }

            #[test]
            fn r177_a_face_down_option_carries_only_its_id_your_view_of_the_prompt_never_names_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": TECH, "radiant": true }, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "backrow": face_down_traps() },
                }));

                s.play(TECH, json!({ "zone": 5 }));

                // An absent prompt serialises as `null`.
                let prompt = js(&s.view(PlayerId::P1).pending).to_string();
                assert_ne!(prompt, "null");
                for trap in [PAWN, BREAD, STIMMY, UNLICENSED] {
                    assert!(!prompt.contains(trap));
                }
            }

            #[test]
            fn r33_the_trap_you_pick_is_read_by_you_once_it_is_yours() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": TECH, "radiant": true }, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "backrow": face_down_traps() },
                }));
                s.play(TECH, json!({ "zone": 5 }));
                let pawn = s.card(PAWN).id.clone();

                s.answer(json!([{ "pick": "instance", "instanceId": pawn }]));

                assert!(js(&s.view(PlayerId::P1).you.backrow).to_string().contains(PAWN));
                assert!(!js(&s.view(PlayerId::P2)).to_string().contains(PAWN));
            }

            #[test]
            fn r113_the_paused_pick_survives_a_json_round_trip_and_resumes_through_reduce() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": TECH, "radiant": true }, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, MENACE, TEMPO], "backrow": [FIELD_SPELL] },
                }));
                s.play(TECH, json!({ "zone": 5 }));
                let revived: GameState =
                    serde_json::from_str(&serde_json::to_string(s.state()).expect("the state serialises"))
                        .expect("the state parses back");
                assert_eq!(&revived, s.state());
                let tempo = s.card(TEMPO).id.clone();

                let choice_id = js(&revived.pending)["id"].as_str().unwrap_or("").to_string();
                let action: Action = json_as(json!({
                    "type": "answer",
                    "playerId": "p1",
                    "choiceId": choice_id,
                    "selection": [{ "pick": "instance", "instanceId": tempo }],
                    "nonce": "rt-040",
                }));
                let result = reduce(&revived, &action);

                assert!(result.error.is_none());
                assert!(result.state.pending.is_none());
                assert!(result
                    .events
                    .iter()
                    .map(js)
                    .any(|event| event["type"] == "controlChanged" && event["instanceId"] == tempo));
            }

            #[test]
            fn r386_a_degrade_of_the_threshold_to_5_stops_4_permanents_asking() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": TECH, "radiant": true }, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, MENACE, TEMPO], "backrow": [FIELD_SPELL] },
                }));
                step_param(s.card_mut(TECH), "threshold", 1);

                s.play(TECH, json!({ "zone": 5 }));

                assert!(s.state().pending.is_none());
            }
        }
    }
}
