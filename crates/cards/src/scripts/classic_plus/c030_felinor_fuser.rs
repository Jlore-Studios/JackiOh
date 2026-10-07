//! C+ #30 Felinor Fuser (SPEC §8.7 row 30; §6.3 Discover and Fuse, R77, R102, R179, R352, R380,
//! R387, R405). (3) Unit, Felinor, Epic, 3/3 → 6/6.
//!   Base:    "Cry: Discover a Felinor Unit, then another. Fuse both into this."
//!   Radiant: "Cry: Discover a Radiant Felinor Unit, then another. Fuse both into this."
//!
//! Two chained Discovers, the shape #98 Heroic Power's Stitching makes them (R352): the first answer
//! re-enters this script with the pick carried in the second prompt's data, and the second answer
//! fuses both picks onto this unit. Each Discover offers three different non-token Felinor-tagged
//! Units (R405: "Felinors" are the creatures) of every set (R380), never Felinor Fuser itself, which
//! `discoverFromCatalog` leaves out by id — every ingredient's id on a fused Fuser (R387).
//!
//! The Fuse is R77 with this unit as the target on the field (`fuseCards`): it keeps its instance,
//! zone, damage, position and radiant flag, sums the stats, unions the keywords, joins the texts, costs
//! min(sum, 4), and its id names the three (R179); the Discovered cards were never cards on a board
//! and cease to exist, so their Cries never run. On the Radiant face the Discovered cards are Radiant:
//! each goes in on its Radiant face, lending it to both of the fusion's forms (§6.3 Fuse), and the
//! kept Radiant instance reads the Radiant form (R77). A Discover with no pool fizzles, and fewer than
//! two picks fuse nothing (R352, R77).

use jackioh_engine::effects::{chosen_options, discover_from_catalog, fuse_cards};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-030";

/// R352's two Discovers: "Discover a Felinor Unit, then another".
const PICKS: usize = 2;

/// The resume step each Discover's answer re-enters (§10.6).
const PICKED: &str = "picked";

/// The data key the picks so far travel in, from the first prompt to the second.
const PICKS_KEY: &str = "picks";

/// R405, R380: a Felinor-tagged Unit of any set; `query` leaves tokens out (§5.1).
fn felinor_units() -> Value {
    json!({ "tags": ["Felinor"], "type": "Unit" })
}

/// The picks a paused Cry has made so far, read back out of the prompt's data.
fn picks_so_far(ctx: &EffectContext<'_>) -> Vec<String> {
    match ctx.data.get(PICKS_KEY).and_then(Value::as_array) {
        Some(stored) => stored.iter().filter_map(|pick| pick.as_str().map(str::to_string)).collect(),
        None => Vec::new(),
    }
}

/// TS's `discover` closure inside `felinorFuser`: one Discover of a Felinor Unit, the picks so far in
/// its data.
fn discover(radiant: bool, picks: &[String]) -> Effect {
    let mut args = json!({
        "step": PICKED,
        "query": felinor_units(),
        "prompt": if radiant { "Discover a Radiant Felinor Unit" } else { "Discover a Felinor Unit" },
        "data": {},
    });
    args["data"][PICKS_KEY] = json!(picks);
    discover_from_catalog(json_as(args))
}

fn felinor_fuser(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| vec![discover(radiant, &[])])),
        resume: IndexMap::from([(
            PICKED,
            hook(move |ctx| {
                let picked = chosen_options(ctx).into_iter().next();
                // A Discover is answered with one of its options (§10.6); an answer naming none adds nothing.
                let Some(picked) = picked else {
                    return vec![];
                };
                let mut picks = picks_so_far(ctx);
                picks.push(picked);
                if picks.len() < PICKS {
                    return vec![discover(radiant, &picks)];
                }
                // The unit fused onto is this one, while it still stands on the field (R77).
                let Some(self_) = ctx.self_.as_ref() else {
                    return vec![];
                };
                let mut args = json!({ "defIds": picks, "targetInstanceId": self_.id });
                if radiant {
                    args["radiantIngredients"] = json!(true);
                }
                vec![fuse_cards(json_as(args))]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: felinor_fuser(false),
        radiant: felinor_fuser(true),
    }
}

// C+ #30 Felinor Fuser — SPEC §8.7 row 30, BUILD M9 Classic+ row C+ 30: "Cry: two chained Discovers,
// each of 3 different non-token Felinor Units of any set but Felinor Fuser (R387, R405), the first
// answer carried in the second prompt; then both are fused into this unit (R77, R102): it keeps its
// instance, zone, damage and position, stats sum, keywords union, texts join, its cost becomes min(sum,
// 4), and its definition id names the three (R179); the fused-in Cries never run, it being on the field
// already; the options reach only the chooser (R177); paused between the prompts the state survives
// JSON and replays; radiant Discovers Radiant Felinor Units and its Radiant face sums the Radiant faces".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FUSER: &str = "classicplus-030";
    /// R405: the non-token Felinor-tagged Units of every set, Felinor Fuser aside.
    const FELINOR_UNITS: [&str; 5] = ["core-012", "core-043", "core-086", "classic-047", "classicplus-046"];
    /// #11 Tempo Timmy: a non-Felinor on the enemy side, which #43 Big Felinor's Cry would destroy.
    const TIMMY: &str = "core-011";
    const FILLER: &str = "core-005";

    fn open(s: &Scenario) -> PendingChoice {
        match &s.state().pending {
            Some(pending) => pending.clone(),
            None => panic!("no prompt is open"),
        }
    }

    fn offered(pending: &PendingChoice) -> Vec<String> {
        pending
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Mode { option } => Some(option.clone()),
                _ => None,
            })
            .collect()
    }

    use crate::js;

    fn is_felinor_unit(id: &str) -> bool {
        FELINOR_UNITS.contains(&id)
    }

    /// Play the Fuser into lane 2 and answer both Discovers with their first option.
    fn fused(radiant: bool, seed: Option<&str>) -> (Scenario, Vec<String>) {
        let mut options = json!({
            "p1": { "hand": [{ "def": FUSER, "radiant": radiant }, FILLER] },
            "p2": { "hand": [FILLER], "field": [TIMMY] },
        });
        if let Some(seed) = seed {
            options["seed"] = json!(seed);
        }
        let mut s = scenario(options);
        s.play(FUSER, json!({ "zone": 2 }));
        let mut picks: Vec<String> = Vec::new();
        for _step in 0..2 {
            let pick = offered(&open(&s)).first().cloned().unwrap_or_default();
            picks.push(pick.clone());
            s.answer(json!(pick));
        }
        (s, picks)
    }

    mod c_n30_felinor_fuser {
        use super::*;

        #[test]
        fn is_a_3_3_felinor_unit_whose_faces_differ_only_in_the_discovered_cards_face() {
            crate::register_all();
            assert_eq!(ID, FUSER);
            assert_eq!(crate::card_def(ID).id, FUSER);
            let scripts = script();
            // TS `expect(base).not.toBe(radiant)`: each face is its own script, built by its own call.
            assert!(!std::sync::Arc::ptr_eq(
                scripts.base.cry.as_ref().unwrap(),
                scripts.radiant.cry.as_ref().unwrap()
            ));
        }

        mod base {
            use super::*;

            #[test]
            fn r405_r387_each_discover_offers_3_different_non_token_felinor_units_of_any_set_never_the_fuser() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [FUSER, FILLER] } }));
                s.play(FUSER, json!({}));
                let first = open(&s);
                assert_eq!(first.kind, PromptKind::Discover);
                assert_eq!(first.player_id, P1);
                let ids = offered(&first);
                assert_eq!(ids.len(), 3);
                assert_eq!(ids.iter().cloned().collect::<IndexSet<String>>().len(), 3);
                for id in &ids {
                    assert!(is_felinor_unit(id));
                }
                s.answer(json!(ids.first().cloned().unwrap_or_default()));
                let second = offered(&open(&s));
                assert_eq!(second.len(), 3);
                for id in &second {
                    assert!(is_felinor_unit(id));
                }
            }

            #[test]
            fn r352_the_first_answer_is_carried_in_the_second_prompt_and_the_second_answer_fuses_both() {
                crate::register_all();
                let (s, picks) = fused(false, None);
                assert!(s.state().pending.is_none());
                let fuser = s.unit(P1, 2).expect("the Fuser left its zone");
                let mut want = picks.clone();
                want.push(FUSER.to_string());
                assert_eq!(fused_id_parts(Some(s.state()), &fuser.def_id), Some(want));
            }

            #[test]
            fn r77_it_keeps_its_instance_zone_damage_and_position_stats_sum_keywords_union_and_texts_join() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [FUSER, FILLER] }, "p2": { "hand": [FILLER] } }));
                let card = s.card(FUSER).clone();
                s.play(FUSER, json!({ "zone": 2 }));
                let mut picks: Vec<String> = Vec::new();
                picks.push(offered(&open(&s)).first().cloned().unwrap_or_default());
                s.answer(json!(picks[0]));
                // Between the two Discovers the Fuser stands hurt and in defence, which the Fuse keeps.
                {
                    let live = find_instance_mut(s.state_mut(), &card.id).expect("the Fuser is in the state");
                    live.damage = 1;
                    live.position = Some(Position::Def);
                }
                picks.push(offered(&open(&s)).first().cloned().unwrap_or_default());
                s.answer(json!(picks[1]));
                let kept = s.unit(P1, 2).expect("the Fuser left its zone");
                assert_eq!(kept.id, card.id);
                assert_eq!(kept.damage, 1);
                assert_eq!(kept.position, Some(Position::Def));
                let fused_def = def_of(Some(s.state()), &kept.def_id).clone();
                let mut ids = picks.clone();
                ids.push(FUSER.to_string());
                let ingredients: Vec<CardDef> = ids.iter().map(|id| def_of(Some(s.state()), id).clone()).collect();
                let attack: i32 = ingredients.iter().map(|entry| entry.base.attack.unwrap_or(0)).sum();
                let health: i32 = ingredients.iter().map(|entry| entry.base.health.unwrap_or(0)).sum();
                // R77: the fused definition prints the sums; the kept instance keeps its 1 damage (an ingredient's
                // own aura, a Felinor Flagbearer's, may raise what the layers show on top of them).
                assert_eq!(fused_def.base.attack, Some(attack));
                assert_eq!(fused_def.base.health, Some(health));
                let stats = s.stats(&kept);
                assert_eq!(stats.max_health - stats.health, 1);
                assert_eq!(fused_def.type_, CardType::Unit);
                let kinds: Vec<KeywordKind> = fused_def.base.keywords.iter().map(|keyword| keyword.kind()).collect();
                for entry in &ingredients {
                    let first_line = entry.base.text.split('\n').next().unwrap_or("");
                    assert!(fused_def.base.text.contains(first_line));
                    for keyword in &entry.base.keywords {
                        assert!(kinds.contains(&keyword.kind()));
                    }
                }
                // R77: min(sum of printed costs, 4) — the Fuser's 3 alone with any Felinor reaches the cap.
                assert_eq!(js(&fused_def.cost), json!(4));
            }

            #[test]
            fn r77_the_fused_in_cries_never_run_the_fuser_is_on_the_field_already() {
                crate::register_all();
                // A seed whose first Discover offers #43 Big Felinor, whose Cry would destroy the enemy Timmy;
                // the second picks #12 Duplicating Felinors when offered, whose Cry would summon a copy.
                const BIG_FELINOR: &str = "core-043";
                const DUPLICATING: &str = "core-012";
                let mut found: Option<Scenario> = None;
                let mut at = 0;
                while at < 40 && found.is_none() {
                    let mut tried = scenario(json!({
                        "seed": format!("fuser-cries-{at}"),
                        "p1": { "hand": [FUSER, FILLER] },
                        "p2": { "hand": [FILLER], "field": [TIMMY] },
                    }));
                    tried.play(FUSER, json!({ "zone": 2 }));
                    if offered(&open(&tried)).iter().any(|id| id == BIG_FELINOR) {
                        found = Some(tried);
                    }
                    at += 1;
                }
                let mut s = found.expect("no seed offers Big Felinor first");
                s.answer(json!(BIG_FELINOR));
                let second = offered(&open(&s));
                let pick = if second.iter().any(|id| id == DUPLICATING) {
                    DUPLICATING.to_string()
                } else {
                    second.first().cloned().unwrap_or_default()
                };
                s.answer(json!(pick));
                let fused_id = s.unit(P1, 2).map(|unit| unit.def_id).unwrap_or_default();
                let parts = fused_id_parts(Some(s.state()), &fused_id).unwrap_or_default();
                assert!(parts.iter().any(|id| id == BIG_FELINOR));
                s.expect_in_zone(TIMMY, "field");
                let summoned = s.events().iter().filter(|event| matches!(event, GameEvent::Summoned { .. })).count();
                assert_eq!(summoned, 1);
            }

            #[test]
            fn r177_the_options_reach_only_the_chooser() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [FUSER, FILLER] }, "p2": { "hand": [FILLER] } }));
                s.play(FUSER, json!({}));
                assert_eq!(js(&s.view(P2).pending), json!({ "forYou": false, "pendingFor": "p1" }));
                for option in offered(&open(&s)) {
                    assert!(!serde_json::to_string(&s.view(P2)).unwrap().contains(&option));
                }
                s.answer(json!(offered(&open(&s)).first().cloned().unwrap_or_default()));
                assert_eq!(js(&s.view(P2).pending), json!({ "forYou": false, "pendingFor": "p1" }));
            }

            #[test]
            fn r113_paused_between_the_prompts_the_state_survives_json_and_the_answer_replays_to_the_same_hash() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [FUSER, FILLER] }, "p2": { "hand": [FILLER] } }));
                s.play(FUSER, json!({}));
                s.answer(json!(offered(&open(&s)).first().cloned().unwrap_or_default()));
                let pending = open(&s);
                let thawed: GameState = serde_json::from_str(&serde_json::to_string(s.state()).unwrap()).unwrap();
                assert_eq!(hash_state(&thawed), hash_state(s.state()));
                let action: Action = json_as(json!({
                    "type": "answer",
                    "choiceId": pending.id,
                    "selection": [pending.options.get(1).map(|option| js(&option.selection))],
                    "playerId": "p1",
                    "nonce": "fuser-roundtrip",
                }));
                let live = reduce(s.state(), &action);
                let again = reduce(&thawed, &action);
                assert!(live.error.is_none());
                assert_eq!(hash_state(&again.state), hash_state(&live.state));
                assert_eq!(again.events, live.events);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r77_discovers_radiant_felinor_units_each_goes_in_on_its_radiant_face_and_the_6_6_sums_the_radiant_faces() {
                crate::register_all();
                let (s, picks) = fused(true, None);
                let kept = s.unit(P1, 2).expect("the Fuser left its zone");
                assert!(kept.radiant);
                let mut want: Vec<Value> = picks.iter().map(|id| json!({ "defId": id, "radiant": true })).collect();
                want.push(json!({ "defId": FUSER }));
                assert_eq!(js(&fused_id_specs(Some(s.state()), &kept.def_id)), Value::Array(want));
                let mut ids = picks.clone();
                ids.push(FUSER.to_string());
                let faces: Vec<CardFace> = ids.iter().map(|id| def_of(Some(s.state()), id).radiant.clone()).collect();
                let attack: i32 = faces.iter().map(|face| face.attack.unwrap_or(0)).sum();
                let health: i32 = faces.iter().map(|face| face.health.unwrap_or(0)).sum();
                // R77: the fused definition's Radiant form prints the sums (an ingredient's own aura, a Felinor
                // Flagbearer's, may raise what the layers then show on top of them).
                let fused_face = def_of(Some(s.state()), &kept.def_id).radiant.clone();
                assert_eq!(fused_face.attack, Some(attack));
                assert_eq!(fused_face.health, Some(health));
            }

            #[test]
            fn r387_the_radiant_discovers_never_offer_the_fuser_either() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": FUSER, "radiant": true }, FILLER] } }));
                s.play(FUSER, json!({}));
                for id in offered(&open(&s)) {
                    assert!(is_felinor_unit(&id));
                }
            }
        }
    }
}
