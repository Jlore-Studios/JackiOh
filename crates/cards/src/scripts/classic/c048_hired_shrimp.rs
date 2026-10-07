//! C #48 Hired Shrimp (SPEC §8.6 row 48; §5 `loc`, §10.8; R46, R70, R81, R90, R177, R397). Unit 4/3 →
//! 8/6, cost 2, Common.
//!   Base:    "Cry: Destroy a permanent whose card takes more lines of code to implement than this one."
//!   Radiant: "… Valid targets are highlighted."
//!   Engine:  the Cry's declared target (R81) may be any permanent on either side; at resolution it is
//!            destroyed only if its `loc` is greater than Hired Shrimp's, else the Cry fizzles (R397).
//!            Radiant: only permanents whose `loc` is greater are offered, except a face-down card the
//!            chooser may not read, always offered and judged at resolution (R177). A fused card's
//!            `loc` is its ingredients' sum; Indestructible stays.

use jackioh_engine::effects::{TargetSpec, destroy, instance_of, unreadable_by};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-048";

fn loc_of(state: &GameState, def_id: &str) -> i32 {
    def_of(Some(state), def_id).loc.unwrap_or(0)
}

fn longer(state: &GameState, card: &CardInstance, shrimp_def_id: &str) -> bool {
    loc_of(state, &card.def_id) > loc_of(state, shrimp_def_id)
}

/// TS `check?: "longer"`: the Radiant face names its target check in the declaration's filter.
fn shrimp(check: Option<&'static str>) -> Script {
    let mut filter = json!({ "side": "any", "of": ["unit", "backrow"] });
    if let Some(check) = check {
        filter["check"] = json!(check);
    }
    Script {
        targets: vec![TargetDecl::target(1, 1, filter)],
        target_checks: IndexMap::from([(
            "longer",
            target_check(|args| match args.candidate {
                Some(candidate) => {
                    unreadable_by(args.state, candidate).contains(&args.player)
                        || longer(args.state, candidate, &args.self_.def_id)
                }
                None => false,
            }),
        )]),
        cry: Some(hook(|ctx| {
            let target = instance_of(ctx, &TargetSpec::Chosen { index: None });
            let shrimp_def_id = match &ctx.self_ {
                Some(me) => me.def_id.clone(),
                None => ID.to_string(),
            };
            match target {
                Some(target) if longer(&ctx.state, &target, &shrimp_def_id) => {
                    vec![destroy(json_as(json!({ "target": { "of": "chosen" } })))]
                }
                _ => vec![],
            }
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: shrimp(None),
        radiant: shrimp(Some("longer")),
    }
}

// C #48 Hired Shrimp — SPEC §8.6 row 48, BUILD M9 Classic row C 48: "Reads `loc` (§5), public in both
// views; base: the Cry's target is any permanent, either side, face-down cards included, unfiltered,
// never Hired Shrimp itself, which is still in hand when the Cry's target is chosen (R397, R70, R90); at
// resolution the target is destroyed only if its `loc` is greater than Hired Shrimp's own, else the Cry
// fizzles; an Indestructible target survives (R46); no permanent → it enters anyway; a face-down option
// carries only its id (R177); a fused card's `loc` is its ingredients' sum; the test reads both values
// from the catalog, never literals; radiant 8/6: only permanents whose `loc` is greater are offered
// (the highlight) and the target is destroyed, except a face-down card the chooser may not read, which
// is always offered and judged at resolution as on the base face, so the option list never reveals its
// `loc` (R397, R177); none qualifies and no such face-down card → no target; no tuned numbers".
//
// Every `loc` is read off the catalog (`cardDef(id).loc`) and each case first states the comparison it
// relies on. The permanents are Core cards with their own tests — Carnivorous Cube, Mr. Vanilla, Mana
// Well, Bear Honeypot, The Rock — and this set's C #52 Final Gambit, a Trap with fewer lines. Fused
// cards come from Unlicensed Experimentation, which fuses the opponent's played permanent onto one of
// its controller's of that type (R77, R102).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SHRIMP: &str = "classic-048";
    const GAMBIT: &str = "classic-052"; // a Trap with fewer lines than Hired Shrimp
    const CUBE: &str = "core-022"; // Carnivorous Cube: a Unit with many more lines
    const VANILLA: &str = "core-008"; // Mr. Vanilla: a Unit with very few
    const MANA_WELL: &str = "core-006"; // a Field Spell with few
    const HONEYPOT: &str = "core-060"; // Bear Honeypot: a Trap with more
    const ROCK: &str = "core-066"; // The Rock: Indestructible, few lines
    const BIGOT: &str = "core-002";
    const GARY: &str = "core-004";
    const FIENDER: &str = "core-092"; // Felinor Fiender: Stack
    const EXPERIMENT: &str = "core-085"; // Unlicensed Experimentation: fuses an enemy's played permanent onto yours
    const FILLER: &str = "core-005";

    use crate::js;

    fn loc(id: &str) -> i32 {
        match crate::card_def(id).loc {
            Some(value) => value,
            None => panic!("{id} has no loc in the catalog"),
        }
    }

    /// TS's module constant `SHRIMP_LOC`, read when a test asks (the catalog is registered per test).
    fn shrimp_loc() -> i32 {
        loc(SHRIMP)
    }

    fn pick(id: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": id }])
    }

    /// The target ids `legal_actions` offers Hired Shrimp's play, any zone. The TS default for `player`
    /// is "p1"; every caller passes it.
    fn offered(s: &Scenario, player: PlayerId) -> IndexSet<String> {
        let Some(shrimp) = s.hand(player).into_iter().find(|card| card.def_id == SHRIMP) else {
            panic!("Hired Shrimp should be in hand");
        };
        let mut out = IndexSet::new();
        for action in legal_actions(s.state(), player) {
            let ActionBody::Play { instance_id, targets, .. } = action else {
                continue;
            };
            if instance_id != shrimp.id {
                continue;
            }
            for selection in targets.unwrap_or_default() {
                let mut keys: Vec<String> = js(&selection)
                    .as_object()
                    .map(|object| object.keys().cloned().collect())
                    .unwrap_or_default();
                keys.sort();
                assert_eq!(keys, vec!["instanceId", "pick"]);
                if let Selection::Instance { instance_id } = selection {
                    out.insert(instance_id);
                }
            }
        }
        out
    }

    fn set_of(ids: &[&str]) -> IndexSet<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    /// p1 controls `mine` (a Unit) and Unlicensed Experimentation; p2 plays `theirs` (a Unit), which is
    /// fused onto `mine`; then p2's turn ends and p1 holds Hired Shrimp. Returns the scenario and the
    /// fused card. The TS default for `is_radiant` is `false`.
    fn fused(mine: &str, theirs: &str, is_radiant: bool) -> (Scenario, String) {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": SHRIMP, "radiant": is_radiant }, FILLER],
                "field": [mine],
                "backrow": [{ "def": EXPERIMENT, "faceUp": false }],
                "library": [FILLER, FILLER],
            },
            "p2": { "hand": [theirs, FILLER], "library": [FILLER, FILLER], "mana": 4 },
            "active": "p2",
        }));
        s.play(theirs, json!({}));
        s.expect_events(json!(["trapFired"]));
        s.end_turn();
        let Some(card) = s.unit(P1, 1) else {
            panic!("the fused card should stand in p1's lane 1");
        };
        (s, card.id)
    }

    fn destroyed_any(s: &Scenario) -> bool {
        s.events().iter().map(js).any(|event| event["type"] == "destroyed")
    }

    mod c48_hired_shrimp {
        use super::*;

        #[test]
        fn its_lines_of_code_are_catalog_data_public_the_entry_carries_loc_and_it_declares_no_numbers() {
            crate::register_all();
            let def = registered_catalog()[ID].clone();
            assert_eq!(def.id, SHRIMP);
            assert_eq!(def.loc, Some(shrimp_loc()));
            assert!(def.params.is_none());
            let scripts = script();
            assert_eq!(
                js(&scripts.base.targets),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "backrow"] } }]),
            );
            assert_eq!(
                js(&scripts.radiant.targets),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "backrow"], "check": "longer" } }]),
            );
        }

        mod base {
            use super::*;

            #[test]
            fn r397_offers_any_permanent_on_either_side_face_down_cards_included_unfiltered_and_never_itself() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [SHRIMP, FILLER], "field": [VANILLA], "backrow": [MANA_WELL] },
                    "p2": { "hand": [FILLER], "field": [CUBE], "backrow": [{ "def": GAMBIT, "faceUp": false }] },
                }));
                let shrimp = s.card(SHRIMP).clone();
                let want: IndexSet<String> = [VANILLA, MANA_WELL, CUBE, GAMBIT].iter().map(|def| s.card(*def).id.clone()).collect();
                assert_eq!(offered(&s, P1), want);
                assert!(!offered(&s, P1).contains(&shrimp.id));
            }

            #[test]
            fn r13_a_card_dormant_under_a_stack_pile_is_no_permanent_on_the_field_only_the_piles_top_is_offered() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [SHRIMP, FILLER] },
                    "p2": { "hand": [FILLER], "field": [CUBE, { "def": FIENDER, "stack": true }] },
                }));
                assert_eq!(offered(&s, P1), set_of(&[s.card(FIENDER).id.as_str()]));
                let targets = pick(&s.card(CUBE).id);
                s.expect_refused(move |s| s.play(SHRIMP, json!({ "targets": targets })));
            }

            #[test]
            fn r397_destroys_a_target_whose_loc_is_greater_than_its_own() {
                crate::register_all();
                assert!(loc(CUBE) > shrimp_loc());
                let mut s = scenario(json!({ "p1": { "hand": [SHRIMP, FILLER] }, "p2": { "hand": [FILLER], "field": [CUBE] } }));
                let targets = pick(&s.card(CUBE).id);
                s.play(SHRIMP, json!({ "targets": targets }));
                s.expect_in_zone(CUBE, "graveyard").expect_in_zone(SHRIMP, "field");
            }

            #[test]
            fn r397_a_wrong_guess_fizzles_a_target_with_fewer_lines_stays() {
                crate::register_all();
                assert!(loc(VANILLA) < shrimp_loc());
                let mut s = scenario(json!({ "p1": { "hand": [SHRIMP, FILLER] }, "p2": { "hand": [FILLER], "field": [VANILLA] } }));
                let targets = pick(&s.card(VANILLA).id);
                s.play(SHRIMP, json!({ "targets": targets }));
                s.expect_in_zone(VANILLA, "field");
                assert!(!destroyed_any(&s));
            }

            #[test]
            fn r397_more_is_strictly_more_another_hired_shrimp_has_as_many_lines_and_stays() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [SHRIMP, FILLER] }, "p2": { "hand": [FILLER], "field": [SHRIMP] } }));
                let Some(theirs) = s.unit(P2, 1) else {
                    panic!("p2's Hired Shrimp should be on the board");
                };
                s.play(SHRIMP, json!({ "targets": pick(&theirs.id) }));
                s.expect_in_zone(&theirs, "field");
            }

            #[test]
            fn your_own_permanents_may_be_the_target_too() {
                crate::register_all();
                assert!(loc(HONEYPOT) > shrimp_loc());
                let mut s = scenario(json!({
                    "p1": { "hand": [SHRIMP, FILLER], "backrow": [{ "def": HONEYPOT, "faceUp": false }] },
                    "p2": { "hand": [FILLER] },
                }));
                let targets = pick(&s.card(HONEYPOT).id);
                s.play(SHRIMP, json!({ "targets": targets }));
                s.expect_in_zone(HONEYPOT, "graveyard");
            }

            #[test]
            fn r177_a_face_down_card_is_offered_by_its_id_alone_and_judged_at_resolution_a_longer_trap_is_destroyed() {
                crate::register_all();
                assert!(loc(HONEYPOT) > shrimp_loc());
                let mut s = scenario(json!({
                    "p1": { "hand": [SHRIMP, FILLER] },
                    "p2": { "hand": [FILLER], "backrow": [{ "def": HONEYPOT, "faceUp": false }] },
                }));
                let Some(trap) = s.backrow(P2, 1) else {
                    panic!("p2's trap should be set");
                };
                assert_eq!(offered(&s, P1), set_of(&[trap.id.as_str()]));
                assert!(!js(&s.view(P1)).to_string().contains(HONEYPOT));
                s.play(SHRIMP, json!({ "targets": pick(&trap.id) }));
                s.expect_in_zone(&trap, "graveyard");
            }

            #[test]
            fn r177_and_a_shorter_face_down_trap_survives_the_guess_still_face_down_and_unnamed() {
                crate::register_all();
                assert!(loc(GAMBIT) < shrimp_loc());
                let mut s = scenario(json!({
                    "p1": { "hand": [SHRIMP, FILLER] },
                    "p2": { "hand": [FILLER], "backrow": [{ "def": GAMBIT, "faceUp": false }] },
                }));
                let Some(trap) = s.backrow(P2, 1) else {
                    panic!("p2's trap should be set");
                };
                s.play(SHRIMP, json!({ "targets": pick(&trap.id) }));
                s.expect_in_zone(&trap, "field");
                assert_ne!(s.card(&trap).face_up, Some(true));
                assert!(!js(&s.view(P1)).to_string().contains(GAMBIT));
            }

            #[test]
            fn r46_an_indestructible_target_with_more_lines_survives_a_fused_the_rock_and_carnivorous_cube() {
                crate::register_all();
                let (mut s, fused_id) = fused(ROCK, CUBE, false);
                let fused_loc = def_of(Some(s.state()), &s.card(&fused_id).def_id).loc;
                assert_eq!(fused_loc, Some(loc(ROCK) + loc(CUBE)));
                assert!(fused_loc.unwrap_or(0) > shrimp_loc());
                s.play(SHRIMP, json!({ "targets": pick(&fused_id) }));
                s.expect_in_zone(&fused_id, "field");
                assert_eq!(js(&s.stats(&fused_id).position), json!("ATK"));
            }

            #[test]
            fn a_fused_cards_loc_is_its_ingredients_sum_bigot_and_gary_each_have_fewer_lines_together_more() {
                crate::register_all();
                assert!(loc(BIGOT) < shrimp_loc());
                assert!(loc(GARY) < shrimp_loc());
                assert!(loc(BIGOT) + loc(GARY) > shrimp_loc());
                let (mut s, fused_id) = fused(BIGOT, GARY, false);
                let fused_def = def_of(Some(s.state()), &s.card(&fused_id).def_id).clone();
                assert_eq!(fused_def.loc, Some(loc(BIGOT) + loc(GARY)));
                // Public in both views: each seat's view carries the fused definition with its loc (R243).
                for viewer in [P1, P2] {
                    assert_eq!(js(&s.view(viewer))["defs"][fused_def.id.as_str()]["loc"], json!(loc(BIGOT) + loc(GARY)));
                }
                s.play(SHRIMP, json!({ "targets": pick(&fused_id) }));
                s.expect_in_zone(&fused_id, "graveyard");
            }

            #[test]
            fn r90_with_no_permanent_on_the_field_it_enters_anyway_its_cry_finding_nothing() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [SHRIMP, FILLER] }, "p2": { "hand": [FILLER] } }));
                assert_eq!(offered(&s, P1), IndexSet::new());
                s.play(SHRIMP, json!({}));
                s.expect_in_zone(SHRIMP, "field").expect_stats(SHRIMP, json!({ "attack": 4, "health": 3 }));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r397_offers_only_the_permanents_with_more_lines_plus_a_face_down_card_you_may_not_read_whatever_its_lines() {
                crate::register_all();
                assert!(loc(CUBE) > shrimp_loc());
                assert!(loc(VANILLA) < shrimp_loc());
                assert!(loc(MANA_WELL) < shrimp_loc());
                assert!(loc(GAMBIT) < shrimp_loc());
                assert!(loc(HONEYPOT) > shrimp_loc());
                let s = scenario(json!({
                    "p1": {
                        "hand": [{ "def": SHRIMP, "radiant": true }, FILLER],
                        "field": [VANILLA],
                        "backrow": [{ "def": GAMBIT, "faceUp": false }, { "def": HONEYPOT, "faceUp": false }],
                    },
                    "p2": { "hand": [FILLER], "field": [CUBE, VANILLA], "backrow": [MANA_WELL, { "def": GAMBIT, "faceUp": false }] },
                }));
                let mine_honeypot = s.backrow(P1, 2).map(|card| card.id);
                let theirs_cube = s.unit(P2, 1).map(|card| card.id);
                let theirs_gambit = s.backrow(P2, 2).map(|card| card.id);
                // Your own face-down cards you read, so they are judged now; the opponent's is offered unread.
                let want: IndexSet<String> = [mine_honeypot, theirs_cube, theirs_gambit].into_iter().flatten().collect();
                assert_eq!(want.len(), 3);
                assert_eq!(offered(&s, P1), want);
            }

            #[test]
            fn r397_an_offered_target_with_more_lines_is_destroyed() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": SHRIMP, "radiant": true }, FILLER] },
                    "p2": { "hand": [FILLER], "field": [CUBE, VANILLA] },
                }));
                let targets = pick(&s.card(CUBE).id);
                s.play(SHRIMP, json!({ "targets": targets }));
                s.expect_in_zone(CUBE, "graveyard");
                s.expect_stats(SHRIMP, json!({ "attack": 8, "health": 6 }));
            }

            #[test]
            fn r90_a_target_that_is_not_highlighted_is_refused_and_legal_actions_agrees() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": SHRIMP, "radiant": true }, FILLER] },
                    "p2": { "hand": [FILLER], "field": [VANILLA, CUBE] },
                }));
                let vanilla = s.card(VANILLA).clone();
                assert!(!offered(&s, P1).contains(&vanilla.id));
                let targets = pick(&vanilla.id);
                s.expect_refused(move |s| s.play(SHRIMP, json!({ "targets": targets })));
                s.expect_in_zone(SHRIMP, "hand");
            }

            #[test]
            fn r177_a_face_down_card_it_may_not_read_is_judged_at_resolution_a_shorter_trap_survives_unnamed() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": SHRIMP, "radiant": true }, FILLER] },
                    "p2": { "hand": [FILLER], "backrow": [{ "def": GAMBIT, "faceUp": false }] },
                }));
                let Some(trap) = s.backrow(P2, 1) else {
                    panic!("p2's trap should be set");
                };
                s.play(SHRIMP, json!({ "targets": pick(&trap.id) }));
                s.expect_in_zone(&trap, "field");
                assert!(!js(&s.view(P1)).to_string().contains(GAMBIT));
            }

            #[test]
            fn r177_and_a_longer_one_is_destroyed() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": SHRIMP, "radiant": true }, FILLER] },
                    "p2": { "hand": [FILLER], "backrow": [{ "def": HONEYPOT, "faceUp": false }] },
                }));
                let Some(trap) = s.backrow(P2, 1) else {
                    panic!("p2's trap should be set");
                };
                s.play(SHRIMP, json!({ "targets": pick(&trap.id) }));
                s.expect_in_zone(&trap, "graveyard");
            }

            #[test]
            fn r177_the_offer_never_reveals_a_face_down_cards_lines_a_shorter_and_a_longer_trap_are_offered_alike() {
                crate::register_all();
                let short = scenario(json!({
                    "p1": { "hand": [{ "def": SHRIMP, "radiant": true }, FILLER] },
                    "p2": { "hand": [FILLER], "backrow": [{ "def": GAMBIT, "faceUp": false }] },
                }));
                let long = scenario(json!({
                    "p1": { "hand": [{ "def": SHRIMP, "radiant": true }, FILLER] },
                    "p2": { "hand": [FILLER], "backrow": [{ "def": HONEYPOT, "faceUp": false }] },
                }));
                assert_eq!(
                    offered(&short, P1).into_iter().collect::<Vec<_>>(),
                    short.backrow(P2, 1).map(|card| card.id).into_iter().collect::<Vec<_>>(),
                );
                assert_eq!(
                    offered(&long, P1).into_iter().collect::<Vec<_>>(),
                    long.backrow(P2, 1).map(|card| card.id).into_iter().collect::<Vec<_>>(),
                );
            }

            #[test]
            fn none_qualifies_and_no_face_down_card_no_target_and_it_enters_anyway() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": SHRIMP, "radiant": true }, FILLER], "field": [VANILLA] },
                    "p2": { "hand": [FILLER], "field": [VANILLA] },
                }));
                assert_eq!(offered(&s, P1), IndexSet::new());
                s.play(SHRIMP, json!({}));
                s.expect_in_zone(SHRIMP, "field");
                assert!(!destroyed_any(&s));
            }

            #[test]
            fn a_fused_card_qualifies_by_its_ingredients_sum_and_is_destroyed() {
                crate::register_all();
                let (mut s, fused_id) = fused(BIGOT, GARY, true);
                assert!(offered(&s, P1).contains(&fused_id));
                s.play(SHRIMP, json!({ "targets": pick(&fused_id) }));
                s.expect_in_zone(&fused_id, "graveyard");
            }
        }
    }
}
