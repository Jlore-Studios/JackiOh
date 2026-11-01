//! Meditative+ #1 Bob's Plan (SPEC §8.9 row 1, R1185000, R1563003, R2521008, R640, R380, R387, R673). (1)
//! Spell, Quickdraw, Mythic.
//!   Base:    "Add a random Wincon card to your hand, and its plan: a Base Camp for Reach the Summit; a
//!            Star Chart for Alteran Reach; for Aestheticize the Game, Discover a Spa Day, a Garage Sale
//!            or a Leg Day."
//!   Radiant: "Add a random Radiant Wincon card to your hand, and its Radiant plan: a Radiant Base Camp
//!            for Reach the Summit; a Radiant Star Chart for Alteran Reach; for Aestheticize the Game,
//!            Discover a Radiant Spa Day, Garage Sale or Leg Day."
//!
//! THE PICK is one draw of the match rng over the Wincon pool (`{ tags: [Wincon] }`, the non-token
//! cards of the sets that ship, R380, in catalog order, so it replays, R60; never this card, R387),
//! through `pick_generated`, so R673's Glitch odds apply to it as to any card generated into a hand.
//! THE PLAN is the AI generated card `WINCON_PLANS` names for the card picked (R1185000, the brief's
//! MP5 D3), added after it: M+ #1.1 Base Camp for M #8 Reach the Summit, and since #2521 M+ #1.5 Star
//! Chart for S #148 Alteran Reach (R2521008), which travels its starscape map. M #20 Aestheticize the Game
//! has three lines to win on, so since #1563 its plan is a Discover of one card per line (R1563003): M+
//! #1.2 Spa Day (the hero's health), #1.3 Garage Sale (the graveyard) and #1.4 Leg Day (the Units'
//! totals), the chosen one added. A card with no plan (a later set's Wincon card, or a Glitch) is added
//! alone. The Radiant face adds both Radiant. A full hand burns what does not fit, in that order (§2.4).
//! Quickdraw (R640) is a static flag beside the tag.
//!
//! `script_v0_4_0f` keeps the card as it ran through v0.4.0f, Spa Day M #20's only plan, for its
//! replays, and `script_v0_5_11` as it ran through v0.5.11, Alteran Reach added with no plan, for
//! theirs (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-001";

/// R1185000, R1563003: each Wincon card's plans — the AI generated cards that help it win (MP5 D3). One
/// plan is added; several are Discovered.
pub const WINCON_PLANS: &[(&str, &[&str])] = &[
    // M #8 Reach the Summit: levels and mana, M+ #1.1 Base Camp.
    ("meditative-008", &["meditativeplus-001-1"]),
    // S #148 Alteran Reach: the starscape map, M+ #1.5 Star Chart (R2521008).
    ("stinkyscape-148", &["meditativeplus-001-5"]),
    // M #20 Aestheticize the Game: the hero's health, M+ #1.2 Spa Day; the graveyard, M+ #1.3 Garage
    // Sale; the Units' totals, M+ #1.4 Leg Day.
    (
        "meditative-020",
        &["meditativeplus-001-2", "meditativeplus-001-3", "meditativeplus-001-4"],
    ),
];

/// A table of Wincon cards and their plans, as `WINCON_PLANS`.
type Plans = &'static [(&'static str, &'static [&'static str])];

/// R767000: `WINCON_PLANS` through v0.5.11, before S #148 Alteran Reach had its Star Chart (R2521008),
/// so it was added alone.
const WINCON_PLANS_V0_5_11: Plans = &[
    ("meditative-008", &["meditativeplus-001-1"]),
    (
        "meditative-020",
        &["meditativeplus-001-2", "meditativeplus-001-3", "meditativeplus-001-4"],
    ),
];

/// The step a Discovered plan's answer re-enters.
const PLAN: &str = "plan";

/// The plans `table` names for a card; none for a card it does not name.
fn plans_in(table: Plans, def_id: &str) -> &'static [&'static str] {
    table
        .iter()
        .find(|(wincon, _)| *wincon == def_id)
        .map_or(&[], |(_, plans)| *plans)
}

/// The plans `WINCON_PLANS` names for a card; none for a card it does not name.
pub fn plans_of(def_id: &str) -> &'static [&'static str] {
    plans_in(WINCON_PLANS, def_id)
}

/// The Wincon card, from the pool by the match rng (R380, R60, R673).
fn picked_wincon(ctx: &mut EffectContext<'_>) -> Option<String> {
    let pool = crate::query::pool(ID, &json_as(json!({ "tags": ["Wincon"] })));
    pick_generated(&mut *ctx.sink.rng, &pool, Some(&*ctx.sink.state)).map(|def| def.id.clone())
}

/// `table`: `WINCON_PLANS`, or the one a kept script ran (R767000).
fn bobs_plan(radiant: bool, table: Plans) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |ctx| {
            let Some(picked) = picked_wincon(ctx) else {
                return vec![];
            };
            let mut out = vec![add_to_hand(json_as(json!({ "defId": picked, "radiant": radiant })))];
            match plans_in(table, &picked) {
                [] => {}
                [plan] => out.push(add_to_hand(json_as(json!({ "defId": plan, "radiant": radiant })))),
                plans => out.push(discover_from_catalog(json_as(json!({
                    "step": PLAN,
                    "query": { "defId": plans, "withTokens": true },
                    "count": plans.len(),
                    "prompt": "Discover a plan",
                })))),
            }
            out
        })),
        resume: IndexMap::from([(
            PLAN,
            hook(move |ctx| {
                chosen_options(ctx)
                    .into_iter()
                    .take(1)
                    .map(|plan| add_to_hand(json_as(json!({ "defId": plan, "radiant": radiant }))))
                    .collect()
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: bobs_plan(false, WINCON_PLANS),
        radiant: bobs_plan(true, WINCON_PLANS),
    }
}

/// R767000: Bob's Plan through v0.5.11, whose Alteran Reach came with no Star Chart (R2521008);
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11() -> CardScripts {
    CardScripts {
        base: bobs_plan(false, WINCON_PLANS_V0_5_11),
        radiant: bobs_plan(true, WINCON_PLANS_V0_5_11),
    }
}

/// R767000: Bob's Plan through v0.4.0f, whose plan for M #20 was Spa Day alone.
fn bobs_plan_v0_4_0f(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |ctx| {
            let Some(picked) = picked_wincon(ctx) else {
                return vec![];
            };
            let mut out = vec![add_to_hand(json_as(json!({ "defId": picked, "radiant": radiant })))];
            if let Some(plan) = plans_of(&picked).first() {
                out.push(add_to_hand(json_as(json!({ "defId": plan, "radiant": radiant }))));
            }
            out
        })),
        ..Script::default()
    }
}

pub fn script_v0_4_0f() -> CardScripts {
    CardScripts {
        base: bobs_plan_v0_4_0f(false),
        radiant: bobs_plan_v0_4_0f(true),
    }
}

// Meditative+ #1 Bob's Plan — SPEC §8.9 row 1, BUILD M11 row M+ 1: "Quickdraw (R640); adds one
// Wincon card from the shipped pool, M 8, M 20 or S 148, each seen over the seeds, by one rng pick that
// replays; after M 8 a base Base Camp (M+ 1.1); after S 148 a Star Chart (M+ 1.5, R2521008); after M 20
// a Discover of Spa Day, Garage Sale and Leg Day (M+ 1.2 to 1.4), the chosen one added (R1563003); a
// full hand burns the helper, then the card; a Wincon card the plan table does not name (a fixture) is
// added alone; radiant both are Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use crate::def_ids;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    const SUMMIT: &str = "meditative-008";
    const AESTHETICIZE: &str = "meditative-020";
    const BASE_CAMP: &str = "meditativeplus-001-1";
    const SPA_DAY: &str = "meditativeplus-001-2";
    const GARAGE_SALE: &str = "meditativeplus-001-3";
    const LEG_DAY: &str = "meditativeplus-001-4";
    const ALTERAN_REACH: &str = "stinkyscape-148";
    const STAR_CHART: &str = "meditativeplus-001-5";

    fn casting(seed: &str, radiant: bool, hand_extra: usize) -> Scenario {
        crate::register_all();
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend(std::iter::repeat_n(json!(FILLER), hand_extra));
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": [FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
        }))
    }

    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id != FILLER).collect()
    }

    fn offered_plans(s: &Scenario) -> Vec<String> {
        s.state()
            .pending
            .as_ref()
            .map(|prompt| {
                prompt
                    .options
                    .iter()
                    .filter_map(|option| match &option.selection {
                        Selection::Mode { option } => Some(option.clone()),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn answer_plan(s: &mut Scenario, plan: &str) {
        if s.state().pending.is_some() {
            s.answer(json!([{ "pick": "mode", "option": plan }]));
        }
    }

    fn casting_for(wincon: &str, radiant: bool, hand_extra: usize) -> Scenario {
        for n in 0..SEED_SEARCH_BOUND {
            let seed = format!("bobs-plan-for-{n}");
            let mut s = casting(&seed, radiant, hand_extra);
            s.play(ID, json!({}));
            if s.hand(P1).iter().chain(s.pile(P1, "graveyard").iter()).any(|card| card.def_id == wincon) {
                return casting(&seed, radiant, hand_extra);
            }
        }
        panic!("no seed picks {wincon}");
    }

    mod base {
        use super::*;

        #[test]
        fn r640_it_is_quickdraw() {
            assert_eq!(script().base.flags().quickdraw, Some(true));
            assert_eq!(script().radiant.flags().quickdraw, Some(true));
        }

        #[test]
        fn r1185000_each_wincon_card_comes_with_its_plan_and_both_are_seen_over_the_seeds() {
            let mut seen: Vec<String> = Vec::new();
            for n in 0..SEED_SEARCH_BOUND {
                let mut s = casting(&format!("bobs-plan-{n}"), false, 1);
                s.play(ID, json!({}));
                answer_plan(&mut s, SPA_DAY);
                let cards = added(&s);
                let expected = match cards.first().map(|card| card.def_id.as_str()) {
                    Some(SUMMIT) => vec![SUMMIT, BASE_CAMP],
                    Some(AESTHETICIZE) => vec![AESTHETICIZE, SPA_DAY],
                    Some(ALTERAN_REACH) => vec![ALTERAN_REACH, STAR_CHART],
                    other => panic!("seed {n} added {other:?}"),
                };
                assert_eq!(def_ids(&cards), expected, "seed {n}");
                assert!(cards.iter().all(|card| !card.radiant), "seed {n}: the base face adds them base");
                if !seen.contains(&expected[0].to_string()) {
                    seen.push(expected[0].to_string());
                }
                if seen.len() == 3 {
                    return;
                }
            }
            panic!("only {seen:?} were picked in {SEED_SEARCH_BOUND} seeds");
        }

        #[test]
        fn r60_the_pick_replays_from_the_seed() {
            let pick = |seed: &str| {
                let mut s = casting(seed, false, 1);
                s.play(ID, json!({}));
                let mut seen = offered_plans(&s);
                seen.extend(def_ids(&added(&s)));
                seen
            };
            for n in 0..8 {
                let seed = format!("bobs-plan-replay-{n}");
                assert_eq!(pick(&seed), pick(&seed));
            }
        }

        #[test]
        fn r1563003_aestheticize_the_game_discovers_one_of_its_three_plans() {
            for plan in [SPA_DAY, GARAGE_SALE, LEG_DAY] {
                let mut s = casting_for(AESTHETICIZE, false, 1);
                s.play(ID, json!({}));
                let mut offered = offered_plans(&s);
                offered.sort();
                assert_eq!(offered, vec![SPA_DAY, GARAGE_SALE, LEG_DAY]);
                assert_eq!(def_ids(&added(&s)), vec![AESTHETICIZE], "the plan waits for the answer");
                s.answer(json!([{ "pick": "mode", "option": plan }]));
                assert_eq!(def_ids(&added(&s)), vec![AESTHETICIZE, plan]);
                assert!(added(&s).iter().all(|card| !card.radiant));
            }
        }

        #[test]
        fn r1563003_reach_the_summit_still_adds_its_one_plan_with_no_discover() {
            let mut s = casting_for(SUMMIT, false, 1);
            s.play(ID, json!({}));
            assert!(s.state().pending.is_none());
            assert_eq!(def_ids(&added(&s)), vec![SUMMIT, BASE_CAMP]);
        }

        #[test]
        fn r2521008_alteran_reach_comes_with_a_star_chart_with_no_discover() {
            let mut s = casting_for(ALTERAN_REACH, false, 1);
            s.play(ID, json!({}));
            assert!(s.state().pending.is_none());
            let cards = added(&s);
            assert_eq!(def_ids(&cards), vec![ALTERAN_REACH, STAR_CHART]);
            assert!(cards.iter().all(|card| !card.radiant), "the base face adds them base");
        }

        /// R767000: the script kept for the replays of v0.5.11 and before adds Alteran Reach alone.
        #[test]
        fn r767000_the_script_kept_for_v0_5_11_adds_alteran_reach_with_no_star_chart() {
            assert!(plans_in(WINCON_PLANS_V0_5_11, ALTERAN_REACH).is_empty());
            crate::under_kept(ID, script_v0_5_11(), || {
                let mut s = casting_for(ALTERAN_REACH, false, 1);
                s.play(ID, json!({}));
                assert!(s.state().pending.is_none());
                assert_eq!(def_ids(&added(&s)), vec![ALTERAN_REACH]);
            });
        }

        #[test]
        fn r1185000_a_full_hand_burns_the_plan_then_the_card() {
            let mut s = casting_for(SUMMIT, false, 9);
            s.play(ID, json!({}));
            let kept = added(&s);
            assert_eq!(def_ids(&kept), vec![SUMMIT], "the Wincon card fits, its plan burns");
            assert!(s.pile(P1, "graveyard").iter().any(|card| card.def_id == BASE_CAMP));
            let mut s = casting_for(SUMMIT, false, 10);
            s.play(ID, json!({}));
            assert!(added(&s).is_empty());
            let burned = def_ids(&s.pile(P1, "graveyard"));
            assert_eq!(burned.len(), 3, "the card, its plan and Bob's Plan itself: {burned:?}");
            let mut s = casting_for(AESTHETICIZE, false, 9);
            s.play(ID, json!({}));
            s.answer(json!([{ "pick": "mode", "option": LEG_DAY }]));
            assert_eq!(def_ids(&added(&s)), vec![AESTHETICIZE]);
            assert!(s.pile(P1, "graveyard").iter().any(|card| card.def_id == LEG_DAY));
        }

        #[test]
        fn r1185000_a_wincon_card_the_plan_table_does_not_name_is_added_alone() {
            crate::register_all();
            let face = json!({ "keywords": [], "text": "fx-wincon" });
            let wincon: CardDef = json_as(json!({
                "id": "fx-wincon", "index": "fx-wincon", "name": "fx-wincon", "set": "Core",
                "type": "Spell", "tags": ["Wincon"], "rarity": "Common", "token": false, "cost": 0,
                "base": face, "radiant": face,
            }));
            let catalog: CardDefs = registered_catalog()
                .iter()
                .filter(|(id, _)| ![SUMMIT, AESTHETICIZE, ALTERAN_REACH].contains(&id.as_str()))
                .map(|(id, def)| (id.clone(), def.clone()))
                .chain([(wincon.id.clone(), wincon)])
                .collect();
            register_catalog(catalog);
            let mut scripts = registered_scripts();
            scripts.insert(
                "fx-wincon".to_string(),
                CardScripts {
                    base: Script::default(),
                    radiant: Script::default(),
                },
            );
            register_scripts(scripts);
            let mut s = scenario(json!({
                "p1": { "hand": [ID, FILLER], "library": [FILLER, FILLER] },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            }));
            s.play(ID, json!({}));
            assert_eq!(def_ids(&added(&s)), vec!["fx-wincon"]);
            clear_overrides();
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2521008_a_radiant_alteran_reach_comes_with_a_radiant_star_chart() {
            let mut s = casting_for(ALTERAN_REACH, true, 1);
            s.play(ID, json!({}));
            assert!(s.state().pending.is_none());
            let cards = added(&s);
            assert_eq!(def_ids(&cards), vec![ALTERAN_REACH, STAR_CHART]);
            assert!(cards.iter().all(|card| card.radiant));
        }

        #[test]
        fn r1185000_r1563003_the_card_and_its_plan_are_both_radiant() {
            for n in 0..6 {
                let mut s = casting(&format!("bobs-plan-radiant-{n}"), true, 1);
                s.play(ID, json!({}));
                answer_plan(&mut s, GARAGE_SALE);
                let cards = added(&s);
                assert_eq!(cards.len(), 2, "seed {n}");
                assert!(plans_of(&cards[0].def_id).contains(&cards[1].def_id.as_str()));
                assert!(cards.iter().all(|card| card.radiant), "seed {n}");
            }
        }
    }
}
