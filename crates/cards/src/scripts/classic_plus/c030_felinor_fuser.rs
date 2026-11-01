//! C+ #30 Felinor Fuser (SPEC §8.7 row 30; §6.3 Discover and Fuse, R77, R102, R179, R352, R380,
//! R387, R405). (4) Unit, Felinor, Epic, 3/3 → 6/6 since issue #2179 (1/1 → 2/2 from issue #1828).
//!   Base:    "Cry: Discover a (2) Cost or less Felinor Unit, then another. Fuse both into this."
//!   Radiant: "Cry: Discover a Radiant (3) Cost or less Felinor Unit, then another. Fuse both into
//!            this."
//!
//! Two chained Discovers, the shape #98 Heroic Power's Stitching makes them (R352): the first answer
//! re-enters with the pick in the second prompt's data, and the second fuses both picks onto this unit.
//! Each Discover offers three different non-token Felinor-tagged Units of every set (R380, R405) whose
//! printed cost (R65) is (2) or less, (3) or less on the Radiant face, never Felinor Fuser itself (R387).
//! Through v0.5.2 the pool had no cost limit, which `script_v0_5_2` keeps (R767000).
//!
//! The Fuse target is this unit on the field (R77, R102): it keeps instance, zone, damage and position,
//! sums stats, unions keywords, joins texts, costs min(sum, 4), and names the three (R179); fused
//! Cries never run. Radiant ingredients lend their Radiant face to both forms (§6.3, R77).

use jackioh_engine::effects::{chosen_options, discover_from_catalog, fuse_cards};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-030";

/// R352's two Discovers: "Discover a Felinor Unit, then another".
const PICKS: usize = 2;

/// The printed "(2) Cost or less" (base) and "(3) Cost or less" (Radiant) since #2179.
const MAX_COST: i32 = 2;
const MAX_COST_RADIANT: i32 = 3;

/// The resume step each Discover's answer re-enters (§10.6).
const PICKED: &str = "picked";

/// The data key the picks so far travel in, from the first prompt to the second.
const PICKS_KEY: &str = "picks";

/// One face's Discover: the most a Felinor Unit it offers may cost, and its prompt.
#[derive(Clone, Copy)]
struct Offer {
    max_cost: Option<i32>,
    prompt: &'static str,
}

/// R405, R380: a Felinor-tagged Unit of any set, at most `max_cost` printed; `query` leaves tokens out
/// (§5.1).
fn felinor_units(max_cost: Option<i32>) -> Value {
    let mut query = json!({ "tags": ["Felinor"], "type": "Unit" });
    if let Some(max) = max_cost {
        query["costRange"] = json!({ "max": max });
    }
    query
}

/// The picks a paused Cry has made so far, read back out of the prompt's data.
fn picks_so_far(ctx: &EffectContext<'_>) -> Vec<String> {
    match ctx.data.get(PICKS_KEY).and_then(Value::as_array) {
        Some(stored) => stored.iter().filter_map(|pick| pick.as_str().map(str::to_string)).collect(),
        None => Vec::new(),
    }
}

/// One Discover of a Felinor Unit, the picks so far in its data.
fn discover(offer: Offer, picks: &[String]) -> Effect {
    let mut args = json!({
        "step": PICKED,
        "query": felinor_units(offer.max_cost),
        "prompt": offer.prompt,
        "data": {},
    });
    args["data"][PICKS_KEY] = json!(picks);
    discover_from_catalog(json_as(args))
}

fn felinor_fuser(radiant: bool, offer: Offer) -> Script {
    Script {
        cry: Some(hook(move |_ctx| vec![discover(offer, &[])])),
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
                    return vec![discover(offer, &picks)];
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
        base: felinor_fuser(
            false,
            Offer {
                max_cost: Some(MAX_COST),
                prompt: "Discover a (2) Cost or less Felinor Unit",
            },
        ),
        radiant: felinor_fuser(
            true,
            Offer {
                max_cost: Some(MAX_COST_RADIANT),
                prompt: "Discover a Radiant (3) Cost or less Felinor Unit",
            },
        ),
    }
}

/// R767000: through v0.5.2 its Discovers offered a Felinor Unit of any cost; `KEPT_SCRIPTS` folds
/// those replays under it.
pub fn script_v0_5_2() -> CardScripts {
    CardScripts {
        base: felinor_fuser(
            false,
            Offer {
                max_cost: None,
                prompt: "Discover a Felinor Unit",
            },
        ),
        radiant: felinor_fuser(
            true,
            Offer {
                max_cost: None,
                prompt: "Discover a Radiant Felinor Unit",
            },
        ),
    }
}

// C+ #30 Felinor Fuser — SPEC §8.7 row 30: two chained Discovers of Felinor Units of any set
// but Felinor Fuser (R387, R405), first answer in second prompt; then both fused into this unit (R77,
// R102, R179); options reach only chooser (R177); radiant Discovers Radiant Felinor Units.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FUSER: &str = "classicplus-030";
    /// R405: the non-token Felinor-tagged Units of every set, Felinor Fuser aside: Meditative's are §8.8
    /// #25, #30, #67, #87 (all tribes), #93, #94 and #102 (whose card the catalog does not hold yet).
    /// Re-pinned for v0.3.5: the Meditative set joins the pool, and for the Stinkyscape release
    /// (#1884) its #87 Siva 1, a (2). #43 and #94 cost (4), so since issue #2179 neither face offers
    /// them (`CHEAP_ENOUGH` leaves them out). Re-pinned for the Training Grounds release (#2204): the
    /// set's Felinor Units join the pool, #17, #22, #23 and #33 and the snapshots #44 (of Core #12) and
    /// #51 (of Core #43, a (4) too), which a Discover never offers beside their Core card (R2204001).
    const FELINOR_UNITS: [&str; 19] = [
        "core-012",
        "core-043",
        "core-086",
        "classic-047",
        "classicplus-046",
        "meditative-025",
        "meditative-030",
        "meditative-067",
        "meditative-087",
        "meditative-093",
        "meditative-094",
        "meditative-102",
        "stinkyscape-087",
        "traininggrounds-017",
        "traininggrounds-022",
        "traininggrounds-023",
        "traininggrounds-033",
        "traininggrounds-044",
        "traininggrounds-051",
    ];
    /// The two of `FELINOR_UNITS` that cost (4), which the faces' "(2) or less" and "(3) or less" leave out.
    const COSTING_4: [&str; 3] = ["core-043", "meditative-094", "traininggrounds-051"];
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

    /// #2179: one of `FELINOR_UNITS` a face's Discover may offer, costing (3) or less.
    fn cheap_enough(id: &str) -> bool {
        is_felinor_unit(id) && !COSTING_4.contains(&id)
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

    /// §8.7 row 30: one Fuser on `radiant`'s face, set on the field, stands at `size`/`size`.
    fn stands_at(radiant: bool, size: i32) {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "field": [{ "def": FUSER, "radiant": radiant }] } }));
        let fuser = s.unit(P1, 1).expect("the Fuser in lane 1");
        s.expect_stats(&fuser, json!({ "attack": size, "health": size, "maxHealth": size }));
    }

    mod c_n30_felinor_fuser {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn s8_7_it_is_3_3() {
                stands_at(false, 3);
            }

            #[test]
            fn r65_its_discovers_never_offer_a_felinor_that_costs_more_than_2_across_seeds() {
                crate::register_all();
                let mut seen: IndexSet<String> = IndexSet::new();
                for n in 0..30 {
                    let mut s = scenario(json!({ "seed": format!("fuser-cheap-{n}"), "p1": { "hand": [FUSER, FILLER] } }));
                    s.play(FUSER, json!({}));
                    for id in offered(&open(&s)) {
                        assert!(cheap_enough(&id), "{id}");
                        assert!(query_cost(&crate::card_def(&id)) <= 2, "{id}");
                        seen.insert(id);
                    }
                }
                // Across the seeds the pool's (1) and (2) Cost Felinors all come up.
                assert!(seen.len() >= 8, "{seen:?}");
            }

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
                    assert!(cheap_enough(id));
                }
                s.answer(json!(ids.first().cloned().unwrap_or_default()));
                let second = offered(&open(&s));
                assert_eq!(second.len(), 3);
                for id in &second {
                    assert!(cheap_enough(id));
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
                // R2657000: a pick with Can't Defend would take the fusion out of Defense Position at
                // the state check, so the test takes the first offer without it.
                let no_cant_defend = |s: &Scenario| -> String {
                    offered(&open(s))
                        .into_iter()
                        .find(|id| !has_keyword(&def_of(Some(s.state()), id).base.keywords, KeywordKind::CantDefend))
                        .unwrap_or_default()
                };
                let mut picks: Vec<String> = Vec::new();
                picks.push(no_cant_defend(&s));
                s.answer(json!(picks[0]));
                // Between the two Discovers the Fuser stands hurt and in defense, which the Fuse keeps.
                {
                    let live = find_instance_mut(s.state_mut(), &card.id).expect("the Fuser is in the state");
                    live.damage = 1;
                    live.position = Some(Position::Def);
                }
                picks.push(no_cant_defend(&s));
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
                // A seed whose first Discover offers #12 Duplicating Felinors, whose Cry would summon a copy.
                const DUPLICATING: &str = "core-012";
                let mut found: Option<Scenario> = None;
                let mut at = 0;
                while at < 60 && found.is_none() {
                    let mut tried = scenario(json!({
                        "seed": format!("fuser-cries-{at}"),
                        "p1": { "hand": [FUSER, FILLER] },
                        "p2": { "hand": [FILLER], "field": [TIMMY] },
                    }));
                    tried.play(FUSER, json!({ "zone": 2 }));
                    if offered(&open(&tried)).iter().any(|id| id == DUPLICATING) {
                        found = Some(tried);
                    }
                    at += 1;
                }
                let mut s = found.expect("no seed offers Duplicating Felinors first");
                s.answer(json!(DUPLICATING));
                let pick = offered(&open(&s)).first().cloned().unwrap_or_default();
                s.answer(json!(pick));
                let fused_id = s.unit(P1, 2).map(|unit| unit.def_id).unwrap_or_default();
                let parts = fused_id_parts(Some(s.state()), &fused_id).unwrap_or_default();
                assert!(parts.iter().any(|id| id == DUPLICATING));
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
            fn s8_7_it_is_6_6() {
                stands_at(true, 6);
            }

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
            fn r387_r65_the_radiant_discovers_never_offer_the_fuser_nor_a_felinor_costing_more_than_3() {
                crate::register_all();
                for n in 0..20 {
                    let mut s = scenario(json!({
                        "seed": format!("fuser-radiant-cheap-{n}"),
                        "p1": { "hand": [{ "def": FUSER, "radiant": true }, FILLER] },
                    }));
                    s.play(FUSER, json!({}));
                    for id in offered(&open(&s)) {
                        assert!(cheap_enough(&id), "{id}");
                        assert!(query_cost(&crate::card_def(&id)) <= 3, "{id}");
                    }
                }
            }
        }

        /// R767000: the script kept for the replays of v0.5.2 and before, whose Discovers had no cost limit.
        mod v0_5_2 {
            use super::*;

            #[test]
            fn r767000_a_game_on_v0_5_2_may_offer_a_felinor_that_costs_4() {
                crate::under_kept(FUSER, crate::scripts::classic_plus::c030_felinor_fuser::script_v0_5_2(), || {
                    let mut costly = false;
                    for n in 0..30 {
                        let mut s = scenario(json!({ "seed": format!("fuser-kept-{n}"), "p1": { "hand": [FUSER, FILLER] } }));
                        s.play(FUSER, json!({}));
                        let ids = offered(&open(&s));
                        assert!(ids.iter().all(|id| is_felinor_unit(id)));
                        costly |= ids.iter().any(|id| COSTING_4.contains(&id.as_str()));
                    }
                    assert!(costly, "no seed offered a (4) Cost Felinor");
                });
            }
        }
    }
}
