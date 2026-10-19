//! M+ #3 Wondrous Wand (SPEC §8.9 row 3, BUILD M11 row M+ 3): (2, Embiggened 4) Spell, Epic.
//!   Base and Radiant: "Draw {draw|card|cards}. They cost (0). Embiggened (4): Draw
//!   {embiggenedDraw} instead." (1/2; 2/5)
//!
//! C #79 Risky Die's pattern: the caster's library is noted on the resolving card as the Spell
//! begins (`remember`, so a pause cannot lose it), the draws run (the count read off
//! `instance.embiggened`, R81), then each noted card now in the hand gets a `costOverride` of 0
//! (C #79's "they": a card cast on draw never gets there, R58; a burned one isn't there,
//! §2.4; a card created meanwhile was never in the library; cards already in the hand are
//! untouched). The override is the card's cost in every zone until it reaches a graveyard or an
//! exile pile (R766).
//! Both counts are declared and read through `param` (R386).

use jackioh_engine::effects::{ForEachCardArgs, draw, for_each_card, remember, set_cost_override};
use jackioh_engine::prelude::*;
use indexmap::IndexSet;

pub const ID: &str = "meditativeplus-003";

/// Where the resolving card notes its controller's library as the Spell begins.
const LIBRARY_KEY: &str = "wandLibrary";

/// The drawn cards' price in the hand.
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

fn cry(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let library: Vec<String> = zone_cards(&*ctx.state, ctx.controller, OffFieldZone::Library)
        .iter()
        .map(|card| card.id.clone())
        .collect();
    // The draw count is the Embiggened one when the play paid the dearer price (R81).
    let key = if ctx.embiggened { "embiggenedDraw" } else { "draw" };
    let count = param(&*ctx, key);
    vec![
        remember(json_as(json!({ "key": LIBRARY_KEY, "value": library }))),
        draw(json_as(json!({ "count": count }))),
        for_each_card(ForEachCardArgs {
            cards: Arc::new(|now: &mut EffectContext<'_>| {
                drawn_into_hand(now).into_iter().map(|card| card.id).collect()
            }),
            each: Arc::new(|instance_id: &str| {
                set_cost_override(json_as(json!({
                    "target": { "of": "instance", "instanceId": instance_id },
                    "cost": DRAWN_COST,
                })))
            }),
        }),
    ]
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(cry)),
        ..Script::default()
    };

    // The same script: the Radiant face differs only in its declared counts, read through `param`.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// M+ #3 Wondrous Wand — SPEC §8.9 row 3, BUILD M11 row M+ 3: "at (2) draws 1 and the drawn card
// costs (0); played Embiggened at (4) draws 2, both (0); a card cast on draw is not discounted and
// the draw repeats (R58); a card burned at a full hand is not either; a card already in the hand
// keeps its cost; both counts read through `param()`; radiant 2 cards, Embiggened 5".
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

    /// Instance IDs of cards drawn by `player`.
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
    fn draws_1_and_the_drawn_card_costs_0() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [WAND, VANILLA], "library": [FILLER, FILLER, FILLER] } }));
        s.play(WAND, json!({}));
        assert_eq!(drawn_ids(s.last_events(), P1).len(), 1);
        s.expect_in_zone(FILLER, "hand");
        let drawn = s.card(FILLER);
        assert_eq!(drawn.cost_override, Some(0));
        assert_eq!(effective_cost(s.state(), drawn, Default::default()), 0);
    }

    #[test]
    fn played_embiggened_at_4_draws_2_both_cost_0() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [WAND, VANILLA], "library": [FILLER, FILLER, FILLER] } }));
        s.play(WAND, json!({ "embiggen": true }));
        s.expect_mana(P1, 0);
        assert_eq!(drawn_ids(s.last_events(), P1).len(), 2);
        for card in s.hand(P1) {
            if card.def_id == FILLER {
                assert_eq!(card.cost_override, Some(0));
            }
        }
        assert_eq!(s.hand(P1).into_iter().filter(|card| card.def_id == FILLER).count(), 2);
    }

    #[test]
    fn r58_a_card_cast_on_draw_is_not_discounted_and_the_draw_repeats() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [WAND, VANILLA], "library": [CN_VIRUS, MENACE, FILLER] } }));
        s.play(WAND, json!({}));
        // The one draw took CN-Virus (cast, the draw repeating into Menace).
        assert_eq!(drawn_ids(s.last_events(), P1).len(), 2);
        s.expect_in_zone(CN_VIRUS, "graveyard").expect_in_zone(MENACE, "hand");
        assert_eq!(s.card(CN_VIRUS).cost_override, None);
        assert_eq!(s.card(MENACE).cost_override, Some(0));
    }

    #[test]
    fn s2_4_a_card_burned_at_a_full_hand_is_not_discounted() {
        crate::register_all();
        // The Wand and nine more: after the Wand is played the hand holds nine, so the first of
        // the Embiggened two draws fills it and the second burns.
        let mut hand = vec![json!(WAND)];
        hand.extend((0..HAND_CAP - 1).map(|_| json!(VANILLA)));
        let mut s = scenario(json!({ "p1": { "hand": hand, "library": [FILLER, MENACE] } }));
        s.play(WAND, json!({ "embiggen": true }));
        s.expect_in_zone(FILLER, "hand");
        s.expect_in_zone(MENACE, "graveyard");
        assert_eq!(s.card(FILLER).cost_override, Some(0));
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
        let mut s = scenario(json!({ "p1": { "hand": [WAND, VANILLA], "library": [FILLER, FILLER, FILLER, FILLER] } }));
        step_param(s.card_mut(WAND), "embiggenedDraw", 1);
        s.play(WAND, json!({ "embiggen": true }));
        assert_eq!(drawn_ids(s.last_events(), P1).len(), 3);
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
    fn radiant_embiggened_draws_5_each_costing_0() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": WAND, "radiant": true }, VANILLA],
                "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
            },
        }));
        s.play(WAND, json!({ "embiggen": true }));
        assert_eq!(drawn_ids(s.last_events(), P1).len(), 5);
        assert_eq!(s.hand(P1).into_iter().filter(|card| card.def_id == FILLER).count(), 5);
        for card in s.hand(P1) {
            if card.def_id == FILLER {
                assert_eq!(card.cost_override, Some(0));
            }
        }
    }
}
