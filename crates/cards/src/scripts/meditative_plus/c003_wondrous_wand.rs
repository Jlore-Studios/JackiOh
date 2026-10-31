//! M+ #3 Wondrous Wand (SPEC §8.9 row 3, BUILD M11 row M+ 3): (2, Embiggened 4) Spell, Epic.
//!   Base and Radiant: "Draw {draw|card|cards}. They cost ({setCost}). Embiggened (4): Draw
//!   {embiggenedDraw} instead." (1/2, (1); 2/4, (0) since issue #2179: (0) and 2/5 before)
//!
//! C #79 Risky Die pattern: library noted in remember, draws run (R81, R58, §2.4), then drawn cards get costOverride (R766).
//! Declared parameters read via param (R386, R1748003, #2179). Keeps script_v0_4_4b for replays (R767000).

use jackioh_engine::effects::{ForEachCardArgs, draw, for_each_card, remember, set_cost_override};
use jackioh_engine::prelude::*;
use indexmap::IndexSet;

pub const ID: &str = "meditativeplus-003";

/// Where the resolving card notes its controller's library as the Spell begins.
const LIBRARY_KEY: &str = "wandLibrary";

/// The drawn cards' price in the hand, "They cost (0)" as v0.4.4b printed it before it was the
/// declared `setCost` (R1748003).
const DRAWN_COST: i32 = 0;

/// "They": the cards in its controller's hand now that were in their library as the Spell began.
fn drawn_into_hand(ctx: &EffectContext<'_>) -> Vec<CardInstance> {
    let noted = recalled(ctx, LIBRARY_KEY);
    let library: IndexSet<String> = noted
        .as_ref()
        .and_then(|value| value.as_array())
        .map(|ids| ids.iter().filter_map(|id| id.as_str().map(String::from)).collect())
        .unwrap_or_default();
    zone_cards(&*ctx.state, ctx.controller, OffFieldZone::Hand)
        .iter()
        .filter(|card| library.contains(&card.id))
        .map(CardInstance::clone)
        .collect()
}

/// The Spell; the drawn cards' price is the declared `setCost`, or the printed (0) when `declared` is
/// off.
fn cry(ctx: &mut EffectContext<'_>, declared: bool) -> Vec<Effect> {
    let library: Vec<String> = zone_cards(&*ctx.state, ctx.controller, OffFieldZone::Library)
        .iter()
        .map(|card| card.id.clone())
        .collect();
    // The draw count is the Embiggened one when the play paid the dearer price (R81).
    let key = if ctx.embiggened { "embiggenedDraw" } else { "draw" };
    let count = param(&*ctx, key);
    let cost = if declared {
        param(&*ctx, "setCost")
    } else {
        DRAWN_COST
    };
    vec![
        remember(json_as(json!({ "key": LIBRARY_KEY, "value": library }))),
        draw(json_as(json!({ "count": count }))),
        for_each_card(ForEachCardArgs {
            cards: Arc::new(|now: &mut EffectContext<'_>| {
                drawn_into_hand(now).into_iter().map(|card| card.id).collect()
            }),
            each: Arc::new(move |instance_id: &str| {
                set_cost_override(json_as(json!({
                    "target": { "of": "instance", "instanceId": instance_id },
                    "cost": cost,
                })))
            }),
        }),
    ]
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| cry(ctx, true))),
        ..Script::default()
    };

    // The same script: the Radiant face differs only in its declared counts, read through `param`.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

/// R767000: through v0.4.4b its drawn cards' (0) was printed; `KEPT_SCRIPTS` folds those replays
/// under it.
pub fn script_v0_4_4b() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| cry(ctx, false))),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M+ #3 Wondrous Wand — SPEC §8.9 row 3, BUILD M11 row M+ 3 (R58).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const WAND: &str = "meditativeplus-003";
    const FILLER: &str = "core-005"; // (1) Spell
    const VANILLA: &str = "core-008"; // (1) Unit
    const STOCKPILE: &str = "core-073"; // (2) Field Spell
    const MENACE: &str = "core-019"; // (3) Unit
    const CN_VIRUS: &str = "core-090-1"; // (1) Spell, Cast on draw

    use crate::scenario;

    fn drawn_ids(events: &[GameEvent], player: PlayerId) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Drawn { player: p, instance_id, .. } if *p == player => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn draws_1_and_the_drawn_card_costs_1() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [WAND, VANILLA], "library": [FILLER, FILLER, FILLER] } }));
        s.play(WAND, json!({}));
        assert_eq!(drawn_ids(s.last_events(), P1).len(), 1);
        s.expect_in_zone(FILLER, "hand");
        let drawn = s.card(FILLER);
        assert_eq!(drawn.cost_override, Some(1));
        assert_eq!(effective_cost(s.state(), drawn, Default::default()), 1);
    }

    #[test]
    fn played_embiggened_at_4_draws_2_both_cost_1() {
        crate::register_all();
        // Mana to spare, so the (1) cards do not leave a turn with nothing to do, which ends itself (R82).
        let mut s = scenario(json!({ "p1": { "hand": [WAND, VANILLA], "library": [FILLER, FILLER, FILLER], "mana": 10 } }));
        s.play(WAND, json!({ "embiggen": true }));
        s.expect_mana(P1, 6);
        assert_eq!(drawn_ids(s.last_events(), P1).len(), 2);
        for card in s.hand(P1) {
            if card.def_id == FILLER {
                assert_eq!(card.cost_override, Some(1));
            }
        }
        assert_eq!(s.hand(P1).into_iter().filter(|card| card.def_id == FILLER).count(), 2);
    }

    #[test]
    fn r58_a_card_cast_on_draw_is_not_discounted_and_the_draw_repeats() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [WAND, VANILLA], "library": [CN_VIRUS, MENACE, FILLER] } }));
        s.play(WAND, json!({}));
        assert_eq!(drawn_ids(s.last_events(), P1).len(), 2);
        s.expect_in_zone(CN_VIRUS, "graveyard").expect_in_zone(MENACE, "hand");
        assert_eq!(s.card(CN_VIRUS).cost_override, None);
        assert_eq!(s.card(MENACE).cost_override, Some(1));
    }

    #[test]
    fn s2_4_a_card_burned_at_a_full_hand_is_not_discounted() {
        crate::register_all();
        let mut hand = vec![json!(WAND)];
        hand.extend((0..HAND_CAP - 1).map(|_| json!(VANILLA)));
        let mut s = scenario(json!({ "p1": { "hand": hand, "library": [FILLER, MENACE] } }));
        s.play(WAND, json!({ "embiggen": true }));
        s.expect_in_zone(FILLER, "hand");
        s.expect_in_zone(MENACE, "graveyard");
        assert_eq!(s.card(FILLER).cost_override, Some(1));
        assert_eq!(s.card(MENACE).cost_override, None);
    }

    #[test]
    fn a_card_already_in_your_hand_keeps_its_cost() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [WAND, STOCKPILE], "library": [FILLER, FILLER] } }));
        let held = s.card(STOCKPILE).clone();
        s.play(WAND, json!({}));
        s.expect_in_zone(&held, "hand");
        assert_eq!(s.card(&held).cost_override, None);
    }

    #[test]
    fn r386_both_counts_read_through_param() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [WAND, VANILLA], "library": [FILLER, FILLER, FILLER] } }));
        step_param(s.card_mut(WAND), "draw", 1);
        s.play(WAND, json!({}));
        assert_eq!(drawn_ids(s.last_events(), P1).len(), 2);

        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [WAND, VANILLA], "library": [FILLER, FILLER, FILLER, FILLER], "mana": 10 } }));
        step_param(s.card_mut(WAND), "embiggenedDraw", 1);
        s.play(WAND, json!({ "embiggen": true }));
        assert_eq!(drawn_ids(s.last_events(), P1).len(), 3);
    }

    #[test]
    fn r1748003_set_cost_moves_with_a_buff_or_a_nerf() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [WAND, VANILLA], "library": [FILLER, FILLER, FILLER] } }));
        // A Nerf makes the drawn card cost (2) on the base face; a Buff (0), its floor (#2179's (1)).
        assert_eq!(crate::degrade_number(&mut s, WAND, "setCost"), 2);
        s.play(WAND, json!({}));
        assert_eq!(s.card(FILLER).cost_override, Some(2));

        let mut buffed = scenario(json!({ "p1": { "hand": [WAND, VANILLA], "library": [FILLER, FILLER, FILLER] } }));
        assert_eq!(crate::upgrade_number(&mut buffed, WAND, "setCost"), 0);
        assert!(!crate::can_upgrade_number(&buffed, WAND, "setCost"));
        buffed.play(WAND, json!({}));
        assert_eq!(buffed.card(FILLER).cost_override, Some(0));
    }

    #[test]
    fn r1748003_r767000_the_v0_4_4b_kept_script_keeps_its_printed_set_cost() {
        crate::under_kept(WAND, super::script_v0_4_4b(), || {
            let mut s = scenario(json!({ "p1": { "hand": [WAND, VANILLA], "library": [FILLER, FILLER, FILLER] } }));
            crate::degrade_number(&mut s, WAND, "setCost");
            s.play(WAND, json!({}));
            assert_eq!(s.card(FILLER).cost_override, Some(0), "the kept script prints (0)");
        });
    }

    #[test]
    fn radiant_draws_2_each_costing_0() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [{ "def": WAND, "radiant": true }, VANILLA], "library": [FILLER, FILLER, FILLER] } }));
        s.play(WAND, json!({}));
        assert_eq!(drawn_ids(s.last_events(), P1).len(), 2);
        assert_eq!(s.hand(P1).into_iter().filter(|card| card.def_id == FILLER).count(), 2);
        for card in s.hand(P1) {
            if card.def_id == FILLER {
                assert_eq!(card.cost_override, Some(0));
            }
        }
    }

    #[test]
    fn radiant_embiggened_draws_4_each_costing_0() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": WAND, "radiant": true }, VANILLA],
                "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
            },
        }));
        s.play(WAND, json!({ "embiggen": true }));
        assert_eq!(drawn_ids(s.last_events(), P1).len(), 4);
        assert_eq!(s.hand(P1).into_iter().filter(|card| card.def_id == FILLER).count(), 4);
        for card in s.hand(P1) {
            if card.def_id == FILLER {
                assert_eq!(card.cost_override, Some(0));
            }
        }
    }
}
