//! C #79 Risky Die (SPEC §8.6 row 79, BUILD M9 Classic row C 79). (1) Spell, Common.
//!   Base:    "Draw {draw}. They cost (1) less. Then exile each of them that costs more than
//!            ({threshold})." (3, 0)
//!   Radiant: the same at ({threshold}) 1 — "then exile each of them that costs (2) or more".
//!   Engine:  "The cards the three draws put in your hand (a card cast on draw never gets there, R58; a
//!            burned one isn't there): `costMod` −1 each, then exile those whose cost in hand (R65) is
//!            above 0 (Radiant: above 1); an X-cost card counts 0 in hand (R65) and is kept. Tunes: draw
//!            3 ↑; kept threshold 0 ↑."
//!
//! Readings:
//!   - "Draw N" is N draws (§2.4): cast on draw, fatigue, the hand cap and a draw limit each act on
//!     their own draw, and a cast on draw that asks pauses the rest (R113).
//!   - "They" are the cards the draws moved from the library into the hand: the library is noted on
//!     the resolving card as the Spell begins (`remember`, so a pause cannot lose it), and "they" are
//!     then the hand cards that were in it. A card cast on draw, a burned card, a draw a limit stopped
//!     and a fatigue hit put nothing there; a card an effect created in the hand meanwhile (a cast on
//!     draw that adds cards) was never in the library; and cards already in the hand are untouched.
//!   - Each of them gets `costMod` −1, which persists in every zone (R78); then each whose cost in hand
//!     now (R65's `effectiveCost`: a player's discounts and surcharges included, an X-cost card 0) is
//!     more than the threshold is exiled, publicly. The kept ones stay hidden in hand (R97).
//! Both numbers are declared and read through `param` (R386): the draw count, and the kept threshold
//! ("↑": an Upgrade keeps more).

use jackioh_engine::effects::{ForEachCardArgs, draw, exile, for_each_card, remember, set_cost_mod};
use jackioh_engine::prelude::*;
use indexmap::IndexSet;

pub const ID: &str = "classic-079";

/// Where the resolving card notes its controller's library as the Spell begins.
const LIBRARY_KEY: &str = "riskyDieLibrary";

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
        .map(|card| CardInstance::clone(card))
        .collect()
}

/// "They cost (1) less."
const RISKY_DISCOUNT: i32 = -1;

fn cry(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let library: Vec<String> = zone_cards(&*ctx.state, ctx.controller, OffFieldZone::Library)
        .iter()
        .map(|card| card.id.clone())
        .collect();
    vec![
        remember(json_as(json!({ "key": LIBRARY_KEY, "value": library }))),
        draw(json_as(json!({ "count": param(&*ctx, "draw") }))),
        for_each_card(ForEachCardArgs {
            cards: Arc::new(|now: &EffectContext<'_>| {
                drawn_into_hand(now).into_iter().map(|card| card.id).collect()
            }),
            each: Arc::new(|instance_id: &str| {
                set_cost_mod(json_as(json!({
                    "target": { "of": "instance", "instanceId": instance_id },
                    "amount": RISKY_DISCOUNT
                })))
            }),
        }),
        for_each_card(ForEachCardArgs {
            cards: Arc::new(|now: &EffectContext<'_>| {
                drawn_into_hand(now)
                    .into_iter()
                    .filter(|card| effective_cost(&*now.state, card, Default::default()) > param(now, "threshold"))
                    .map(|card| card.id)
                    .collect()
            }),
            each: Arc::new(|instance_id: &str| {
                exile(json_as(json!({ "target": { "of": "instance", "instanceId": instance_id } })))
            }),
        }),
    ]
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(cry)),
        ..Script::default()
    };

    // The same script: the Radiant face differs only in its declared threshold (1), read through `param`.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C #79 Risky Die — SPEC §8.6 row 79, BUILD M9 Classic row C 79: "Draw 3; the cards those draws put in
// your hand (not a cast-on-draw card, a burned one or a draw a limit stopped) cost (1) less (`costMod`,
// R78), then each of them that costs (1) or more is exiled; an X-cost card costs 0 in hand (R65) and
// stays; cards already in your hand are untouched; kept cards are never named in the opponent's view
// and exiled ones are public; radiant: only those that cost (2) or more are exiled; its tuned numbers
// (draw, kept threshold) read through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;
    use std::sync::Arc;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const RISKY: &str = "classic-079";
    const MACHINE: &str = "classic-049"; // Anti-Greed Machine: players can't draw more than 1 card each turn.
    const FILLER: &str = "core-010"; // (0) Spell
    const STOCKPILE: &str = "core-005"; // (1) Spell
    const ARMOR: &str = "core-073"; // (2) Field Spell
    const MENACE: &str = "core-019"; // (3) Unit
    const DIVIDEND: &str = "core-024"; // (X) Spell
    const CN_VIRUS: &str = "core-090-1"; // (1) Spell, Cast on draw
    const HINDER: &str = "core-021"; // (0) Spell, Cast on draw: … Discard 1.
    const VANILLA: &str = "core-008"; // (1) Unit

    /// The harness, after the catalog and every card script are registered (TS's harness did it on import).
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    fn def() -> CardDef {
        crate::register_all();
        crate::card_def(ID)
    }

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialisable")
    }

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(found) => found,
            None => panic!("missing: {what}"),
        }
    }

    /// TS `drawnIds(events, player = "p1")`.
    fn drawn_ids(events: &[GameEvent], player: PlayerId) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Drawn { player: p, instance_id, .. } if *p == player => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// Each hand card's cost by def id, as p1's own view gives it (a later card of the same def wins).
    fn hand_costs(s: &Scenario) -> IndexMap<String, i64> {
        let view = js(&s.view(P1));
        let hand = view["you"]["hand"].as_array().cloned().unwrap_or_default();
        let mut costs = IndexMap::new();
        for card in &hand {
            if let (Some(def_id), Some(cost)) = (card["defId"].as_str(), card["cost"].as_i64()) {
                costs.insert(def_id.to_string(), cost);
            }
        }
        costs
    }

    /// is a (1) Spell; its draw count and kept threshold are declared numbers
    #[test]
    fn is_a_1_spell_its_draw_count_and_kept_threshold_are_declared_numbers() {
        assert_eq!(js(&def().type_), json!("Spell"));
        assert_eq!(js(&def().cost), json!(1));
        assert_eq!(
            js(&def().params),
            json!([
                { "key": "draw", "base": 3, "radiant": 3, "better": "up", "step": 1, "min": 1 },
                { "key": "threshold", "base": 0, "radiant": 1, "better": "up", "step": 1, "min": 0 }
            ])
        );
        let scripts = script();
        assert!(Arc::ptr_eq(
            scripts.base.cry.as_ref().expect("a Cry"),
            scripts.radiant.cry.as_ref().expect("a Cry")
        ));
    }

    /// base
    mod base {
        use super::*;

        /// draws 3; they cost (1) less; those that still cost (1) or more are exiled
        #[test]
        fn draws_3_they_cost_1_less_those_that_still_cost_1_or_more_are_exiled() {
            let mut s = scenario(json!({ "p1": { "hand": [RISKY, VANILLA], "library": [FILLER, STOCKPILE, MENACE, VANILLA] } }));
            s.play(RISKY, json!({}));
            assert_eq!(drawn_ids(s.last_events(), P1).len(), 3);
            s.expect_in_zone(FILLER, "hand")
                .expect_in_zone(STOCKPILE, "hand")
                .expect_in_zone(MENACE, "exile");
            assert_eq!(s.card(STOCKPILE).cost_mod, -1);
            assert_eq!(s.card(FILLER).cost_mod, -1);
            assert_eq!(hand_costs(&s).get(STOCKPILE).copied(), Some(0));
        }

        /// R78 the discount persists: a kept card costs (1) less in later turns
        #[test]
        fn r78_the_discount_persists_a_kept_card_costs_1_less_in_later_turns() {
            let mut s = scenario(json!({ "p1": { "hand": [RISKY, VANILLA], "library": [STOCKPILE, FILLER, FILLER, VANILLA] } }));
            s.play(RISKY, json!({})).end_turn().end_turn();
            assert_eq!(s.card(STOCKPILE).cost_mod, -1);
            assert_eq!(hand_costs(&s).get(STOCKPILE).copied(), Some(0));
        }

        /// cards already in your hand are untouched
        #[test]
        fn cards_already_in_your_hand_are_untouched() {
            let mut s = scenario(json!({ "p1": { "hand": [RISKY, STOCKPILE], "library": [FILLER, FILLER, FILLER] } }));
            let held = s.card(STOCKPILE).clone();
            s.play(RISKY, json!({}));
            s.expect_in_zone(&held, "hand");
            assert_eq!(s.card(&held).cost_mod, 0);
        }

        /// R65 an X-cost card costs 0 in hand and stays
        #[test]
        fn r65_an_x_cost_card_costs_0_in_hand_and_stays() {
            let mut s = scenario(json!({ "p1": { "hand": [RISKY, VANILLA], "library": [DIVIDEND, FILLER, FILLER] } }));
            s.play(RISKY, json!({}));
            s.expect_in_zone(DIVIDEND, "hand");
        }

        /// R58 a card cast on draw never reaches your hand: it is not one of them, and the draw repeats
        #[test]
        fn r58_a_card_cast_on_draw_never_reaches_your_hand_it_is_not_one_of_them_and_the_draw_repeats() {
            let mut s = scenario(json!({ "p1": { "hand": [RISKY, VANILLA], "library": [CN_VIRUS, MENACE, STOCKPILE, FILLER] } }));
            s.play(RISKY, json!({}));
            s.expect_in_zone(CN_VIRUS, "graveyard");
            // The three draws took CN-Virus (cast, the draw repeating into Menace), Stockpile and Filler.
            s.expect_in_zone(MENACE, "exile")
                .expect_in_zone(STOCKPILE, "hand")
                .expect_in_zone(FILLER, "hand");
            assert_eq!(s.card(CN_VIRUS).cost_mod, 0);
        }

        /// §2.4 a burned card is not one of them: it stays in your graveyard at its own cost
        #[test]
        fn s2_4_a_burned_card_is_not_one_of_them_it_stays_in_your_graveyard_at_its_own_cost() {
            let mut hand = vec![RISKY];
            hand.extend(vec![VANILLA; 9]);
            let mut s = scenario(json!({ "p1": { "hand": hand, "library": [FILLER, MENACE, STOCKPILE] } }));
            s.play(RISKY, json!({}));
            // Risky Die left the hand (9), the first draw fills it (10), the next two burn.
            s.expect_in_zone(FILLER, "hand")
                .expect_in_zone(MENACE, "graveyard")
                .expect_in_zone(STOCKPILE, "graveyard");
            assert_eq!(s.card(MENACE).cost_mod, 0);
        }

        /// §2.4 a fatigue draw puts nothing in your hand: from a deck of one, one card is judged and two hits land
        #[test]
        fn s2_4_a_fatigue_draw_puts_nothing_in_your_hand_from_a_deck_of_one_one_card_is_judged_and_two_hits_land() {
            let mut s = scenario(json!({ "p1": { "hand": [RISKY, VANILLA], "library": [MENACE] } }));
            s.play(RISKY, json!({}));
            s.expect_in_zone(MENACE, "exile");
            assert_eq!(s.state().players.p1.fatigue_count, 2);
            s.expect_health(P1, 27);
            let hand: Vec<(String, i32)> = s.hand(P1).into_iter().map(|card| (card.def_id, card.cost_mod)).collect();
            assert_eq!(hand, vec![(VANILLA.to_string(), 0)]);
        }

        /// B5 E3 a draw a limit stopped draws nothing: under Anti-Greed Machine only the first card is one of them
        #[test]
        fn b5_e3_a_draw_a_limit_stopped_draws_nothing_under_anti_greed_machine_only_the_first_card_is_one_of_them() {
            let mut s = scenario(json!({ "p1": { "hand": [RISKY, VANILLA], "field": [MACHINE], "library": [MENACE, STOCKPILE, FILLER] } }));
            s.play(RISKY, json!({}));
            assert_eq!(drawn_ids(s.last_events(), P1).len(), 1);
            s.expect_in_zone(MENACE, "exile");
            let library: Vec<String> = s.pile(P1, "library").into_iter().map(|card| card.def_id).collect();
            assert_eq!(library, [STOCKPILE, FILLER]);
        }

        /// §9.3 a cast on draw resolves inside the draws (R682: no prompt); after a JSON round trip they finish, and only the drawn cards are judged
        #[test]
        fn s9_3_a_cast_on_draw_resolves_inside_the_draws_r682_no_prompt_after_a_json_round_trip_they_finish_and_only_the_drawn_cards_are_judged()
         {
            let mut s = scenario(json!({ "p1": { "hand": [RISKY, VANILLA], "library": [HINDER, MENACE, STOCKPILE, FILLER] } }));
            s.play(RISKY, json!({}));
            // Hinder is cast on the first draw; its discard is random (R682), so with one card held the
            // Vanilla goes with no prompt opening.
            assert!(s.state().pending.is_none());
            let round: GameState = serde_json::from_str(&serde_json::to_string(s.state()).expect("serialisable"))
                .expect("a state reads back");
            assert_eq!(&round, s.state());
            let after = s.state().clone();
            let where_ = |def_id: &str| -> Option<ZoneName> {
                let side = &after.players.p1;
                side.hand
                    .iter()
                    .chain(side.exile.iter())
                    .chain(side.graveyard.iter())
                    .chain(side.library.iter())
                    .find(|card| card.def_id == def_id)
                    .map(|card| card.zone.z())
            };
            assert_eq!(where_(MENACE), Some(ZoneName::Exile));
            assert_eq!(where_(STOCKPILE), Some(ZoneName::Hand));
            assert_eq!(where_(FILLER), Some(ZoneName::Hand));
            assert_eq!(where_(VANILLA), Some(ZoneName::Graveyard));
            assert!(after.pending.is_none());
        }

        /// R97 kept cards are never named in the opponent's view; exiled ones are public
        #[test]
        fn r97_kept_cards_are_never_named_in_the_opponent_s_view_exiled_ones_are_public() {
            let mut s = scenario(json!({ "p1": { "hand": [RISKY, VANILLA], "library": [FILLER, STOCKPILE, MENACE] } }));
            s.play(RISKY, json!({}));
            let theirs = js(&s.view(P2));
            let kept = [s.card(FILLER).id.clone(), s.card(STOCKPILE).id.clone()];
            let text = serde_json::to_string(&theirs["events"]).expect("serialisable");
            for id in &kept {
                assert!(!text.contains(id.as_str()));
            }
            let exiled: Vec<Value> = theirs["opponent"]["exile"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|card| card["defId"].clone())
                .collect();
            assert_eq!(json!(exiled), json!([MENACE]));
        }

        /// R386 its draw count is declared: an Upgrade's step draws 4
        #[test]
        fn r386_its_draw_count_is_declared_an_upgrade_s_step_draws_4() {
            let mut s = scenario(json!({ "p1": { "hand": [RISKY, VANILLA], "library": [FILLER, FILLER, FILLER, FILLER, FILLER] } }));
            step_param(s.card_mut(RISKY), "draw", 1);
            s.play(RISKY, json!({}));
            assert_eq!(drawn_ids(s.last_events(), P1).len(), 4);
        }

        /// R386 its kept threshold is declared: an Upgrade's step keeps a card that costs (1)
        #[test]
        fn r386_its_kept_threshold_is_declared_an_upgrade_s_step_keeps_a_card_that_costs_1() {
            let mut s = scenario(json!({ "p1": { "hand": [RISKY, VANILLA], "library": [ARMOR, FILLER, FILLER] } }));
            step_param(s.card_mut(RISKY), "threshold", 1);
            s.play(RISKY, json!({}));
            s.expect_in_zone(ARMOR, "hand");
            assert_eq!(hand_costs(&s).get(ARMOR).copied(), Some(1));
        }
    }

    /// radiant
    mod radiant {
        use super::*;

        /// only those that still cost (2) or more are exiled: a (2) card kept at (1), a (3) card exiled
        #[test]
        fn only_those_that_still_cost_2_or_more_are_exiled_a_2_card_kept_at_1_a_3_card_exiled() {
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": RISKY, "radiant": true }, VANILLA], "library": [ARMOR, MENACE, STOCKPILE] } }));
            s.play(RISKY, json!({}));
            s.expect_in_zone(ARMOR, "hand")
                .expect_in_zone(STOCKPILE, "hand")
                .expect_in_zone(MENACE, "exile");
            assert_eq!(hand_costs(&s).get(ARMOR).copied(), Some(1));
        }

        /// R386 its threshold steps from (1): a Degrade's step exiles the (2) card too
        #[test]
        fn r386_its_threshold_steps_from_1_a_degrade_s_step_exiles_the_2_card_too() {
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": RISKY, "radiant": true }, VANILLA], "library": [ARMOR, FILLER, FILLER] } }));
            step_param(s.card_mut(RISKY), "threshold", -1);
            s.play(RISKY, json!({}));
            s.expect_in_zone(ARMOR, "exile");
        }

        /// draws 3, and a card cast on draw is still none of them
        #[test]
        fn draws_3_and_a_card_cast_on_draw_is_still_none_of_them() {
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": RISKY, "radiant": true }, VANILLA], "library": [CN_VIRUS, ARMOR, FILLER, FILLER] } }));
            s.play(RISKY, json!({}));
            s.expect_in_zone(CN_VIRUS, "graveyard").expect_in_zone(ARMOR, "hand");
        }
    }
}
