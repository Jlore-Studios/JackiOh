//! C #44 Back from the GY (SPEC §8.6 row 44, §6.3 Summon, §10.6; R1, R64, R65, R178, R396). Spell,
//! cost 4, Legendary.
//!   Base:    "Summon Units from your graveyard that cost ({budget}) or less in total. Exile this."
//!   Radiant: "Summon every Unit from your graveyard. Exile this."
//!   Engine:  "A `pick` prompt (§10.6) from your graveyard, budgeted by cost, costs per R65 out of play
//!            (an X Unit counts 0, R396); then each is summoned (no Cry, R1) into your leftmost open
//!            zones (R64); a full board leaves the rest. Radiant: every Unit, oldest first, until the
//!            board is full. "Exile this" is the Spell's landing (§5.1, R178). Tunes: budget 5 ↑."
//!
//! THE BASE PICK is the engine's budgeted `choosePick` (B5 E18): every Unit in your graveyard is an
//! option carrying its cost as R65 reads it there, out of play (its own cost, `costMod` and
//! `costOverride` kept, R78; an X Unit 0), and the answer's costs may total no more than the card's
//! budget (`param(ctx, "budget")`), any number of Units under it, none included. No Unit there asks
//! nothing. Each pick is then summoned (§6.3 Summon: no Cry, R1) into your leftmost empty, unlocked,
//! unreserved unit zone (R64); once the row is full a summon finds no zone and that card stays in the
//! graveyard.
//!
//! THE RADIANT FACE asks nothing: every Unit in your graveyard, oldest first (the graveyard is
//! chronological, §3), read once as the clause begins (`forEachCard`, R113), each summoned the same
//! way until the row is full.
//!
//! "EXILE THIS" names where §10.5 step 7 sends the Spell (R178): it stays itself while it resolves and
//! lands in exile rather than the graveyard, so it never summons or offers itself.

use jackioh_engine::effects::{ForEachCardArgs, choose_pick, exile, for_each_card, summon};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-044";

/// Your graveyard's Units, oldest first (§3: a graveyard is chronological).
fn graveyard_units(ctx: &EffectContext<'_>) -> Vec<CardInstance> {
    let state: &GameState = &ctx.state;
    let mut units = Vec::new();
    for card in zone_cards(state, ctx.controller, OffFieldZone::Graveyard).iter() {
        if def_of(Some(state), &card.def_id).type_ == CardType::Unit {
            units.push(CardInstance::clone(card));
        }
    }
    units
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            // Any number of Units, the budget the only bound.
            let max = zone_count(&ctx.state, ctx.controller, OffFieldZone::Graveyard);
            let budget = param(&*ctx, "budget");
            vec![
                choose_pick(json_as(json!({
                    "step": "picked",
                    "from": [{ "zone": "graveyard" }],
                    "filter": { "type": "Unit" },
                    "max": max,
                    "budget": budget,
                    "prompt": "Summon Units from your graveyard within the budget",
                }))),
                exile(json_as(json!({ "target": { "of": "self" } }))),
            ]
        })),
        resume: IndexMap::from([(
            "picked",
            hook(|ctx| {
                (0..ctx.targets.len())
                    .map(|index| summon(json_as(json!({ "instance": { "of": "chosen", "index": index } }))))
                    .collect()
            }),
        )]),
        ..Script::default()
    };

    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(|ctx: &EffectContext<'_>| -> Vec<String> {
                        graveyard_units(ctx).into_iter().map(|card| card.id).collect()
                    }),
                    each: Arc::new(|instance_id: &str| -> Effect {
                        summon(json_as(json!({ "instance": { "of": "instance", "instanceId": instance_id } })))
                    }),
                }),
                exile(json_as(json!({ "target": { "of": "self" } }))),
            ]
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #44 Back from the GY — SPEC §8.6 row 44, BUILD M9 Classic row C 44: "A budgeted pick from your
// graveyard: Units whose costs (R65 out of play: an X Unit 0) total (5) or less, a pick over the
// budget refused; each is summoned without a Cry into your leftmost open zones, a full board leaving
// the rest in the graveyard; no Unit there → nothing; then this Spell is exiled, not sent to the
// graveyard (§5.1); radiant: every Unit there, oldest first, until the board is full; its tuned number
// (budget) reads through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::script;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BACK: &str = "classic-044";
    const FILLER: &str = "core-005"; // (1) Spell, a spare card (§2.5); also a non-Unit in the graveyard.
    const MR_TOKEN: &str = "core-015"; // (1) Unit 1/1: "Cry: Summon a Rush Token."
    const TIMMY: &str = "core-011"; // (1) Unit 3/3 Rush, First Strike
    const POINTMASTER: &str = "core-020"; // (2) Unit 7/1
    const MENACE: &str = "core-019"; // (3) Unit 9/9
    const SEVEN: &str = "core-025"; // (4) Unit 7/7
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const BILLY: &str = "classicplus-069"; // (X) Unit, "This is a 3X/3X."
    const RUSH_TOKEN: &str = "core-t-rush";

    /// An engine value as the JSON the TS test reads (SURFACE §5.1: the same keys and values).
    fn js<T: serde::Serialize + ?Sized>(value: &T) -> Value {
        serde_json::to_value(value).expect("an engine value serialises")
    }

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(value) => value,
            None => panic!("the scenario has no {what}"),
        }
    }

    fn open(s: &Scenario) -> &PendingChoice {
        must(s.state().pending.as_ref(), "open prompt")
    }

    /// TS `stepParam(s.card(ref), key, steps)`: the step written on the card as it stands in the state.
    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        step_param(must(find_instance_mut(s.state_mut(), &id), "card to step"), key, steps);
    }

    /// `graveyard` entries are bare ids or `{ def, costMod? }` objects; `field` is the TS default `[]`
    /// when empty.
    fn back(radiant_face: bool, graveyard: Value, field: &[&str]) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": BACK, "radiant": radiant_face }, FILLER], "graveyard": graveyard, "field": field },
            "p2": { "hand": [FILLER] },
        }))
    }

    fn units_on_board(s: &Scenario) -> Value {
        json!([1, 2, 3, 4, 5].map(|lane| s.unit(P1, lane).map(|card| card.def_id)))
    }

    fn graveyard_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.pile(player, "graveyard").into_iter().map(|card| card.def_id).collect()
    }

    /// The def id of the card an option offers, or "?" for an option that is not an instance.
    fn offered_def(s: &Scenario, option: &PromptOption) -> String {
        match &option.selection {
            Selection::Instance { instance_id } => s.card(instance_id).def_id.clone(),
            _ => "?".to_string(),
        }
    }

    mod c44_back_from_the_gy {
        use super::*;

        #[test]
        fn declares_its_one_number_budget_r386() {
            crate::register_all();
            let def = js(&registered_catalog()[BACK]);
            assert_eq!(def["params"], json!([{ "key": "budget", "base": 5, "radiant": 5, "better": "up", "step": 1, "min": 1 }]));
            let scripts = script();
            assert!(scripts.base.targets.is_empty());
            assert!(scripts.radiant.targets.is_empty());
        }

        mod base {
            use super::*;

            #[test]
            fn e18_opens_a_budgeted_pick_of_your_graveyards_units_only_each_carrying_its_cost_budget_5() {
                crate::register_all();
                let mut s = back(false, json!([MR_TOKEN, FILLER, POINTMASTER, MENACE, SEVEN]), &[]);
                s.play(BACK, json!({}));
                let pending = open(&s);
                assert_eq!(pending.player_id, P1);
                assert_eq!(js(&pending.kind), json!("pick"));
                assert_eq!(pending.budget, Some(5));
                assert_eq!(pending.min, 0);
                let offered: Vec<Value> = pending
                    .options
                    .iter()
                    .map(|option| match &option.selection {
                        Selection::Instance { .. } => json!([offered_def(&s, option), option.cost]),
                        _ => json!(["?"]),
                    })
                    .collect();
                assert_eq!(json!(offered), json!([[MR_TOKEN, 1], [POINTMASTER, 2], [MENACE, 3], [SEVEN, 4]]));
            }

            #[test]
            fn summons_the_units_you_pick_within_the_budget_into_your_leftmost_open_zones() {
                crate::register_all();
                let mut s = back(false, json!([POINTMASTER, MENACE, SEVEN]), &[VANILLA]);
                s.play(BACK, json!({}));
                let picks = json!([s.card(POINTMASTER).id, s.card(MENACE).id]);
                s.answer(picks);
                assert_eq!(units_on_board(&s), json!([VANILLA, POINTMASTER, MENACE, null, null]));
                assert_eq!(graveyard_defs(&s, P1), vec![SEVEN]);
            }

            #[test]
            fn r1_a_summon_fires_no_cry_and_the_units_arrive_summoning_sick() {
                crate::register_all();
                let mut s = back(false, json!([MR_TOKEN]), &[]);
                s.play(BACK, json!({}));
                let picks = json!([s.card(MR_TOKEN).id]);
                s.answer(picks);
                assert_eq!(units_on_board(&s), json!([MR_TOKEN, null, null, null, null]));
                assert!(!s.events().iter().map(js).any(|event| event["type"] == "summoned" && event["defId"] == RUSH_TOKEN));
                assert_eq!(s.card(MR_TOKEN).summoned_turn, Some(s.state().turn));
                s.expect_refused(|s| s.attack(MR_TOKEN, "hero"));
            }

            #[test]
            fn a_pick_whose_costs_total_more_than_the_budget_is_refused() {
                crate::register_all();
                let mut s = back(false, json!([POINTMASTER, SEVEN]), &[]);
                s.play(BACK, json!({}));
                let picks = json!([s.card(POINTMASTER).id, s.card(SEVEN).id]);
                s.expect_refused(move |s| s.answer(picks));
                assert_eq!(js(&open(&s).kind), json!("pick"));
            }

            #[test]
            fn r65_a_graveyard_cards_own_cost_counts_its_cost_mod_included() {
                crate::register_all();
                // The 7/7 (4) with costMod −3 costs (1) there, so it and a (4) fit in 5.
                let mut s = back(false, json!([{ "def": SEVEN, "costMod": -3 }, MENACE, VANILLA]), &[]);
                s.play(BACK, json!({}));
                let cost: Vec<Option<i32>> = open(&s).options.iter().map(|option| option.cost).collect();
                assert_eq!(cost, vec![Some(1), Some(3), Some(1)]);
                let picks = json!([s.card(SEVEN).id, s.card(MENACE).id, s.card(VANILLA).id]);
                s.answer(picks);
                assert_eq!(units_on_board(&s), json!([SEVEN, MENACE, VANILLA, null, null]));
            }

            #[test]
            fn r396_an_x_unit_in_the_graveyard_costs_0_toward_the_budget() {
                crate::register_all();
                let mut s = back(false, json!([BILLY, SEVEN]), &[]);
                s.play(BACK, json!({}));
                let billy = open(&s).options.iter().find(|option| {
                    matches!(option.selection, Selection::Instance { .. }) && offered_def(&s, option) == BILLY
                });
                assert_eq!(billy.and_then(|option| option.cost), Some(0));
            }

            #[test]
            fn r64_a_full_board_leaves_the_rest_in_the_graveyard() {
                crate::register_all();
                let mut s = back(false, json!([VANILLA, TIMMY]), &[MENACE, SEVEN, POINTMASTER, MENACE]);
                s.play(BACK, json!({}));
                let vanilla = s.card(VANILLA).clone();
                let timmy = s.card(TIMMY).clone();
                s.answer(json!([vanilla.id, timmy.id]));
                assert_eq!(s.unit(P1, 5).map(|card| card.id), Some(vanilla.id.clone()));
                s.expect_in_zone(&timmy, "graveyard");
            }

            #[test]
            fn only_your_own_graveyard_the_opponents_units_are_neither_offered_nor_summoned() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [BACK, FILLER], "graveyard": [VANILLA] },
                    "p2": { "hand": [FILLER], "graveyard": [MENACE, SEVEN] },
                }));
                s.play(BACK, json!({}));
                let offered: Vec<String> = open(&s).options.iter().map(|option| offered_def(&s, option)).collect();
                assert_eq!(offered, vec![VANILLA]);

                let mut r = scenario(json!({
                    "p1": { "hand": [{ "def": BACK, "radiant": true }, FILLER], "graveyard": [VANILLA] },
                    "p2": { "hand": [FILLER], "graveyard": [MENACE, SEVEN] },
                }));
                r.play(BACK, json!({}));
                assert_eq!(units_on_board(&r), json!([VANILLA, null, null, null, null]));
                assert_eq!(graveyard_defs(&r, P2), vec![MENACE, SEVEN]);
            }

            #[test]
            fn no_unit_in_your_graveyard_no_prompt_nothing_summoned() {
                crate::register_all();
                let mut s = back(false, json!([FILLER]), &[]);
                s.play(BACK, json!({}));
                assert!(s.state().pending.is_none());
                assert_eq!(units_on_board(&s), json!([null, null, null, null, null]));
            }

            #[test]
            fn r178_then_this_spell_is_exiled_not_sent_to_the_graveyard() {
                crate::register_all();
                let mut s = back(false, json!([VANILLA]), &[]);
                let spell = s.card(BACK).clone();
                s.play(BACK, json!({}));
                let picks = json!([s.card(VANILLA).id]);
                s.answer(picks);
                s.expect_in_zone(&spell, "exile");
                let graveyard: Vec<String> = s.pile(P1, "graveyard").into_iter().map(|card| card.id).collect();
                assert!(!graveyard.contains(&spell.id));
            }

            #[test]
            fn r178_exiled_with_nothing_to_summon_too() {
                crate::register_all();
                let mut s = back(false, json!([]), &[]);
                s.play(BACK, json!({}));
                s.expect_in_zone(BACK, "exile");
            }

            #[test]
            fn r177_the_opponent_reads_only_that_a_prompt_is_open_for_you() {
                crate::register_all();
                let mut s = back(false, json!([VANILLA]), &[]);
                s.play(BACK, json!({}));
                assert_eq!(js(&s.view(P2))["pending"], json!({ "forYou": false, "pendingFor": "p1" }));
            }

            #[test]
            fn s9_3_the_open_pick_survives_a_json_round_trip_and_resumes_through_reduce() {
                crate::register_all();
                let mut s = back(false, json!([POINTMASTER, MENACE]), &[]);
                let spell = s.card(BACK).id.clone();
                s.play(BACK, json!({}));
                let revived: GameState =
                    serde_json::from_str(&serde_json::to_string(s.state()).expect("the state serialises"))
                        .expect("the state parses back");
                assert_eq!(&revived, s.state());
                let pending = must(revived.pending.as_ref(), "revived prompt");
                let menace = s.card(MENACE).id.clone();
                let action: Action = json_as(json!({
                    "type": "answer",
                    "playerId": "p1",
                    "choiceId": pending.id,
                    "selection": [{ "pick": "instance", "instanceId": menace }],
                    "nonce": "back-from-the-gy-round-trip",
                }));
                let result = reduce(&revived, &action);
                assert!(result.error.is_none());
                assert!(result.state.pending.is_none());
                assert!(result.state.work.is_empty());
                assert!(result.events.iter().map(js).any(|event| event["type"] == "exiled" && event["instanceId"] == spell));
            }

            #[test]
            fn r386_an_upgrade_of_the_budget_lets_6_in() {
                crate::register_all();
                let mut s = back(false, json!([POINTMASTER, SEVEN]), &[]);
                step(&mut s, BACK, "budget", 1);
                s.play(BACK, json!({}));
                assert_eq!(open(&s).budget, Some(6));
                let picks = json!([s.card(POINTMASTER).id, s.card(SEVEN).id]);
                s.answer(picks);
                assert_eq!(units_on_board(&s), json!([POINTMASTER, SEVEN, null, null, null]));
            }

            #[test]
            fn r386_a_degrade_of_the_budget_makes_it_4() {
                crate::register_all();
                let mut s = back(false, json!([POINTMASTER, MENACE]), &[]);
                step(&mut s, BACK, "budget", -1);
                s.play(BACK, json!({}));
                assert_eq!(open(&s).budget, Some(4));
                let picks = json!([s.card(POINTMASTER).id, s.card(MENACE).id]);
                s.expect_refused(move |s| s.answer(picks));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn summons_every_unit_in_your_graveyard_oldest_first_with_no_prompt() {
                crate::register_all();
                let mut s = back(true, json!([SEVEN, FILLER, MENACE, VANILLA]), &[]);
                s.play(BACK, json!({}));
                assert!(s.state().pending.is_none());
                assert_eq!(units_on_board(&s), json!([SEVEN, MENACE, VANILLA, null, null]));
                assert_eq!(graveyard_defs(&s, P1), vec![FILLER]);
            }

            #[test]
            fn no_budget_units_of_any_cost_come_back() {
                crate::register_all();
                let mut s = back(true, json!([SEVEN, MENACE, POINTMASTER]), &[]);
                s.play(BACK, json!({}));
                assert_eq!(units_on_board(&s), json!([SEVEN, MENACE, POINTMASTER, null, null]));
            }

            #[test]
            fn r64_until_the_board_is_full_the_newest_stay_in_the_graveyard() {
                crate::register_all();
                let mut s = back(true, json!([SEVEN, MENACE, POINTMASTER, TIMMY]), &[VANILLA, VANILLA, VANILLA]);
                s.play(BACK, json!({}));
                assert_eq!(units_on_board(&s), json!([VANILLA, VANILLA, VANILLA, SEVEN, MENACE]));
                assert_eq!(graveyard_defs(&s, P1), vec![POINTMASTER, TIMMY]);
            }

            #[test]
            fn r1_no_cry_fires() {
                crate::register_all();
                let mut s = back(true, json!([MR_TOKEN, MR_TOKEN]), &[]);
                s.play(BACK, json!({}));
                assert_eq!(units_on_board(&s), json!([MR_TOKEN, MR_TOKEN, null, null, null]));
            }

            #[test]
            fn r178_then_this_spell_is_exiled() {
                crate::register_all();
                let mut s = back(true, json!([VANILLA]), &[]);
                s.play(BACK, json!({}));
                s.expect_in_zone(BACK, "exile");
            }

            #[test]
            fn an_empty_graveyard_summons_nothing_and_the_spell_is_still_exiled() {
                crate::register_all();
                let mut s = back(true, json!([]), &[]);
                s.play(BACK, json!({}));
                assert_eq!(units_on_board(&s), json!([null, null, null, null, null]));
                s.expect_in_zone(BACK, "exile");
            }
        }
    }
}
