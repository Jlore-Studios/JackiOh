//! C+ #43 AI Slop (SPEC §8.7 row 43, E23, R60, R77, R102, R179, R468, R469, R582, R1828001). (3) Spell,
//! Legendary.
//!   Fuse {cards} random (1) Cost AI generated cards and add the result to your hand. It costs (0).
//!   Radiant: the AI generated cards are Radiant.
//!
//! `fuseGenerated` is the whole fusion: independent picks from the AI generated cards whose printed
//! cost (R65) is (1) (R1828001; a pool the text names, so tokens reach it;
//! repeats allowed, R60), fused with no target (R77, R102: the shared type, else the first's; a Token
//! with the AI tag; its id names them, R179), into your hand at `costOverride` 0 (the hand cap burns
//! it). Radiant picks go in on their Radiant face (R469). R582: tuned down to 1, "fuse 1 card" is no
//! fusion (R77), so that one card, from the same pool, is added as it is, at (0).

use jackioh_engine::effects::{add_random_from_catalog, fuse_generated};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-043";

/// R1828001: the printed cost (R65) of every card the pool holds, "(1) Cost".
const POOL_COST: i32 = 1;

/// The AI generated cards, those of printed cost `cost` when it is given.
fn ai_cards(cost: Option<i32>) -> Value {
    let mut pool = json!({ "tags": ["AI"], "token": true });
    if let Some(cost) = cost {
        pool["cost"] = json!(cost);
    }
    pool
}

/// The script of a face that fuses from the AI generated cards of printed cost `cost`, or from all of
/// them when it is `None`.
fn slop(cost: Option<i32>) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let count = param(&*ctx, "cards");
            if count < 2 {
                vec![add_random_from_catalog(json_as(json!({
                    "query": ai_cards(cost),
                    "count": count,
                    "costOverride": 0,
                    "radiant": ctx.radiant,
                })))]
            } else {
                vec![fuse_generated(json_as(json!({ "count": count, "query": ai_cards(cost), "radiant": ctx.radiant })))]
            }
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's picks are Radiant through `ctx.radiant`.
    CardScripts {
        base: slop(Some(POOL_COST)),
        radiant: slop(Some(POOL_COST)),
    }
}

/// R767000: through v0.4.9 the pool was every AI generated card; `KEPT_SCRIPTS` keeps it.
pub fn script_v0_4_9() -> CardScripts {
    CardScripts {
        base: slop(None),
        radiant: slop(None),
    }
}

// C+ #43 AI Slop — SPEC §8.7 row 43, R11, R60, R77, R97, R102, R179, R386, R469, R582, R1828001, BUILD M9
// row C+ 43.
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::POOL_COST;
    use crate::events_json;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SLOP: &str = "classicplus-043";
    const FILLER: &str = "core-005";

    /// The AI generated cards of the sets that ship (§7, R1421): T-AI-1 to T-AI-10 of Classic+ and of
    /// Stinkyscape (#1884), M+ #1.1 Base Camp, #1.2 Spa Day and Bob's Plan's two AI tokens (#1187).
    fn ai_ids() -> Vec<String> {
        let mut ids: Vec<String> = (1..=10).map(|n| format!("classicplus-t-ai-{n:02}")).collect();
        ids.extend((1..=10).map(|n| format!("stinkyscape-t-ai-{n:02}")));
        ids.extend(
            [
                "meditativeplus-001-1",
                "meditativeplus-001-2",
                "meditativeplus-001-3",
                "meditativeplus-001-4",
            ]
            .map(String::from),
        );
        ids
    }

    /// R1828001 (#1828): the (1) Cost AI generated cards, the pool, in the query's index order.
    const ONE_COST: [&str; 13] = [
        "classicplus-t-ai-01",
        "classicplus-t-ai-04",
        "classicplus-t-ai-07",
        "classicplus-t-ai-08",
        "classicplus-t-ai-09",
        "meditativeplus-001-3",
        "stinkyscape-t-ai-01",
        "stinkyscape-t-ai-10",
        "stinkyscape-t-ai-03",
        "stinkyscape-t-ai-04",
        "stinkyscape-t-ai-05",
        "stinkyscape-t-ai-06",
        "stinkyscape-t-ai-09",
    ];

    /// Whether `id` is a card of the pool: an AI generated card of printed cost (1).
    fn in_pool(id: &str) -> bool {
        ai_ids().iter().any(|ai| ai == id) && query_cost(&crate::card_def(id)) == POOL_COST
    }

    use crate::scenario;

    fn slop(seed: &str, radiant: bool, hand: Option<Vec<&str>>) -> Scenario {
        let mut cards = vec![json!({ "def": SLOP, "radiant": radiant })];
        cards.extend(hand.unwrap_or_else(|| vec![FILLER]).into_iter().map(|id| json!(id)));
        scenario(json!({ "seed": seed, "p1": { "hand": cards }, "p2": { "hand": [FILLER] } }))
    }

    fn played(seed: &str, radiant: bool, hand: Option<Vec<&str>>) -> Scenario {
        let mut s = slop(seed, radiant, hand);
        s.play(SLOP, json!({}));
        s
    }

    /// The card AI Slop made: p1's hand card that is not a filler.
    fn made(s: &Scenario) -> CardInstance {
        match s.hand(P1).into_iter().find(|held| held.def_id != FILLER) {
            Some(card) => card,
            None => panic!("the fused card in hand"),
        }
    }

    fn specs(s: &Scenario) -> Vec<FusedIngredient> {
        subsystems::fused_ingredient_specs(s.state(), &made(s).def_id).unwrap_or_default()
    }

    mod base {
        use super::*;

        #[test]
        fn costs_3_refused_with_2_mana() {
            let mut s = slop("slop-cost", false, None);
            crate::expect_costs(&mut s, SLOP, 3, json!({}));
        }

        #[test]
        fn r1828001_the_pool_is_the_thirteen_1_cost_ai_generated_cards() {
            crate::register_all();
            let pool: Vec<String> = crate::query::query(&json_as(super::super::ai_cards(Some(POOL_COST))))
                .iter()
                .map(|def| def.id.clone())
                .collect();
            assert_eq!(pool, ONE_COST.map(String::from).to_vec());
            assert!(ONE_COST.iter().all(|id| in_pool(id)));
        }

        #[test]
        fn r1828001_it_never_fuses_a_card_whose_cost_is_not_1() {
            let mut seen: BTreeSet<String> = BTreeSet::new();
            for n in 0..20 {
                for spec in specs(&played(&format!("slop-cost-{n}"), false, None)) {
                    assert_eq!(query_cost(&crate::card_def(&spec.def_id)), POOL_COST, "{}", spec.def_id);
                    seen.insert(spec.def_id);
                }
            }
            assert!(seen.len() > 1);
            assert!(seen.iter().all(|id| in_pool(id)));
        }

        #[test]
        fn r77_r179_fuses_three_random_ai_generated_cards_into_your_hand_at_0_its_id_names_the_three() {
            let s = played("slop-base", false, None);
            let card = made(&s);
            assert_eq!(specs(&s).len(), 3);
            for spec in specs(&s) {
                assert!(in_pool(&spec.def_id));
                assert_eq!(spec.radiant, None);
            }
            assert_eq!(card.cost_override, Some(0));
            assert!(!card.radiant);
        }

        #[test]
        fn r102_a_token_with_the_ai_tag_of_the_shared_type_else_the_firsts_its_texts_joined() {
            let mut shapes: BTreeSet<&str> = BTreeSet::new();
            for n in 0..12 {
                let s = played(&format!("slop-shape-{n}"), false, None);
                let fused = def_of(Some(s.state()), &made(&s).def_id).clone();
                let parts: Vec<CardDef> = specs(&s).iter().map(|spec| crate::card_def(&spec.def_id)).collect();
                assert!(fused.token);
                assert!(fused.tags.contains(&Tag::Ai));
                let types: IndexSet<CardType> = parts.iter().map(|part| part.type_).collect();
                shapes.insert(if types.len() == 1 { "shared" } else { "mixed" });
                // No AI generated card is a Field Trap, so R102's promotion never applies.
                let expected = if types.len() == 1 { types.first().copied() } else { parts.first().map(|part| part.type_) };
                assert_eq!(Some(fused.type_), expected);
                for part in &parts {
                    assert!(fused.base.text.contains(&part.base.text));
                }
            }
            assert!(shapes.contains("mixed"));
        }

        #[test]
        fn r60_picks_are_independent_a_card_may_come_up_twice() {
            let mut repeated = false;
            let mut n = 0;
            while n < 40 && !repeated {
                let ids: Vec<String> =
                    specs(&played(&format!("slop-repeat-{n}"), false, None)).into_iter().map(|spec| spec.def_id).collect();
                repeated = ids.iter().collect::<IndexSet<_>>().len() < ids.len();
                n += 1;
            }
            assert!(repeated);
        }

        #[test]
        fn r97_r179_the_opponents_view_names_neither_the_card_nor_its_ingredients() {
            let s = played("slop-hidden", false, None);
            let seen = serde_json::to_string(&s.view(P2)).expect("a view is JSON");
            for id in ai_ids() {
                assert!(!seen.contains(&id));
            }
            assert!(!seen.contains(&made(&s).def_id));
            assert!(serde_json::to_string(&s.view(P1)).expect("a view is JSON").contains(&made(&s).def_id));
        }

        #[test]
        fn s2_4_r11_a_full_hand_burns_it_a_fused_unit_token_ceases_to_exist_anything_else_is_in_the_graveyard() {
            let mut seen: BTreeSet<&str> = BTreeSet::new();
            let mut n = 0;
            while n < 40 && seen.len() < 2 {
                let mut s = played(&format!("slop-burn-{n}"), false, Some(vec![FILLER; 10]));
                let burned = events_json(&s).into_iter().find(|event| event["type"] == "burned");
                assert!(burned.is_some());
                assert_eq!(s.hand(P1).len(), 10);
                let burned = burned.unwrap_or(Value::Null);
                let id = burned["instanceId"].as_str().unwrap_or_default().to_string();
                let fused_type = def_of(Some(s.state()), burned["defId"].as_str().unwrap_or_default()).type_;
                seen.insert(if fused_type == CardType::Unit { "Unit" } else { "other" });
                if fused_type == CardType::Unit {
                    s.expect_in_zone(&id, "gone");
                } else {
                    s.expect_in_zone(&id, "graveyard");
                }
                n += 1;
            }
            assert_eq!(seen.into_iter().collect::<Vec<_>>(), vec!["Unit", "other"]);
        }

        #[test]
        fn played_it_counts_once_toward_the_ai_generated_cards_played_this_game_scaling_laws_count() {
            for n in 0..20 {
                let mut s = played(&format!("slop-play-{n}"), false, None);
                let fused = def_of(Some(s.state()), &made(&s).def_id).clone();
                let ingredients = subsystems::fused_ingredients(s.state(), &fused.id).unwrap_or_default();
                if fused.type_ != CardType::Spell || ingredients.len() != 3 {
                    continue;
                }
                assert_eq!(played_this_game_with_tag(s.state(), P1, Tag::Ai), 0);
                let card = made(&s);
                s.play(&card, json!({}));
                assert_eq!(played_this_game_with_tag(s.state(), P1, Tag::Ai), 1);
                return;
            }
            panic!("no seed fused three Spells");
        }

        #[test]
        fn r386_its_card_count_reads_through_param_an_upgrade_fuses_4_a_degrade_2() {
            let mut up = slop("slop-up", false, None);
            step_param(up.card_mut(SLOP), "cards", 1);
            up.play(SLOP, json!({}));
            assert_eq!(specs(&up).len(), 4);
            let mut down = slop("slop-down", false, None);
            step_param(down.card_mut(SLOP), "cards", -1);
            down.play(SLOP, json!({}));
            assert_eq!(specs(&down).len(), 2);
        }

        #[test]
        fn r582_tuned_to_one_card_that_card_is_added_as_it_is_unfused_at_0() {
            let mut s = slop("slop-one", false, None);
            step_param(s.card_mut(SLOP), "cards", -1);
            step_param(s.card_mut(SLOP), "cards", -1);
            s.play(SLOP, json!({}));
            let card = made(&s);
            assert!(in_pool(&card.def_id));
            assert_eq!(card.cost_override, Some(0));
        }

        #[test]
        fn r582_r1828001_tuned_to_one_card_it_is_a_1_cost_one_on_every_seed() {
            for n in 0..20 {
                let mut s = slop(&format!("slop-one-{n}"), false, None);
                step_param(s.card_mut(SLOP), "cards", -1);
                step_param(s.card_mut(SLOP), "cards", -1);
                s.play(SLOP, json!({}));
                assert!(in_pool(&made(&s).def_id), "{}", made(&s).def_id);
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn costs_3_refused_with_2_mana() {
            let mut s = slop("slop-cost", true, None);
            crate::expect_costs(&mut s, SLOP, 3, json!({}));
        }

        #[test]
        fn r1828001_it_never_fuses_a_card_whose_cost_is_not_1() {
            for n in 0..20 {
                for spec in specs(&played(&format!("slop-radiant-cost-{n}"), true, None)) {
                    assert_eq!(query_cost(&crate::card_def(&spec.def_id)), POOL_COST, "{}", spec.def_id);
                    assert!(in_pool(&spec.def_id));
                    assert_eq!(spec.radiant, Some(true));
                }
            }
        }

        #[test]
        fn r469_the_three_go_in_on_their_radiant_faces_and_the_result_is_radiant() {
            let s = played("slop-radiant", true, None);
            assert_eq!(specs(&s).len(), 3);
            for spec in specs(&s) {
                assert_eq!(spec.radiant, Some(true));
            }
            let fused = def_of(Some(s.state()), &made(&s).def_id).clone();
            let parts: Vec<CardDef> = specs(&s).iter().map(|spec| crate::card_def(&spec.def_id)).collect();
            for part in &parts {
                assert!(fused.base.text.contains(&part.radiant.text));
            }
            assert_eq!(made(&s).cost_override, Some(0));
            assert!(made(&s).radiant);
        }

        #[test]
        fn r582_tuned_to_one_card_the_card_is_radiant() {
            let mut s = slop("slop-radiant-one", true, None);
            step_param(s.card_mut(SLOP), "cards", -1);
            step_param(s.card_mut(SLOP), "cards", -1);
            s.play(SLOP, json!({}));
            assert!(made(&s).radiant);
            assert!(in_pool(&made(&s).def_id));
        }
    }

    mod v0_4_9 {
        use super::*;

        #[test]
        fn r767000_r1828001_a_game_on_v0_4_9_folds_under_the_kept_script_its_pool_holds_every_ai_generated_card() {
            crate::register_all();
            jackioh_engine::with_shipped(crate::catalog_at("v0.4.9"), || {
                let mut costs: BTreeSet<i32> = BTreeSet::new();
                for n in 0..20 {
                    for spec in specs(&played(&format!("slop-kept-{n}"), false, None)) {
                        assert!(ai_ids().contains(&spec.def_id));
                        costs.insert(query_cost(&crate::card_def(&spec.def_id)));
                    }
                }
                assert!(costs.iter().any(|cost| *cost != POOL_COST), "costs: {costs:?}");
            });
        }
    }
}
