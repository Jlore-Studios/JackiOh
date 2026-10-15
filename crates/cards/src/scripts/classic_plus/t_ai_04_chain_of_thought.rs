//! T-AI-4 Chain of Thought (SPEC §8.7 row T-AI-4, §7, B8). (1) Spell, AI, Token.
//!   Base:    "Draw 1. If it costs (1) or less, repeat this, up to 4 more times."
//!   Radiant: "Draw 1. If it costs (2) or less, repeat this, up to 4 more times."
//!   Engine:  "'It' is the card the draw put in your hand, its cost read per R65 as it arrives (an X-cost
//!            card as 0). A card cast on draw never gets there (R58), a burned one isn't there, and a
//!            fatigue draw brings none, so each ends the chain, as does a draw the draw limit stops (§2.4,
//!            `drawLimited`). At most five draws (`CHAIN_OF_THOUGHT_REPEATS`, 4 repeats after the first);
//!            a prompt inside one parks the rest on `state.work` (R158). Tunes: none."
//!
//! The chain is one engine verb (`draw_while_cheap`, effects/datacenter.rs) over `draw_one` and the card
//! that draw put in the hand (`card_this_draw_put_in_hand`, R596): a cast-on-draw card ends it, the card
//! its cast's own repeat brings included, so the only prompt a draw can open ends the chain where it stands.

use jackioh_engine::effects::draw_while_cheap;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-t-ai-04";

/// TS's `{ base, radiant } as const`: one number per face.
#[derive(Clone, Copy)]
struct ByFaceCost {
    base: i32,
    radiant: i32,
}

/// §8.7: "If it costs (1) or less", Radiant "(2) or less". An AI card declares no params (B8).
const MAX_COST: ByFaceCost = ByFaceCost { base: 1, radiant: 2 };

fn chain(max_cost: i32) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![draw_while_cheap(json_as(json!({ "maxCost": max_cost, "repeats": CHAIN_OF_THOUGHT_REPEATS })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: chain(MAX_COST.base),
        radiant: chain(MAX_COST.radiant),
    }
}

// T-AI-4 Chain of Thought — SPEC §8.7 row T-AI-4, BUILD M9 Classic+ row T-AI-4: "Draws 1; if the card
// that draw put in your hand costs (1) or less (its current cost; an X-cost card counts 0, R65) it draws
// again, at most `CHAIN_OF_THOUGHT_REPEATS` (4) more times, 5 draws in all; a cast-on-draw card (never in
// hand, R58), a burned card or a fatigue draw ends the chain; radiant the threshold is (2)".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const CHAIN: &str = "classicplus-t-ai-04";
    const TIMMY: &str = "core-011"; // (1) Unit
    const RAPID: &str = "core-010"; // (0) Spell
    const D_FENDER: &str = "core-001"; // (2) Unit
    const MENACE: &str = "core-019"; // (3) Unit
    const ADAPTIVE_UI: &str = "core-074"; // (X) Spell
    const HINDER: &str = "core-021"; // (0) Spell, cast on draw; the base face discards 1 at random with no prompt (R431, R682)
    const VANILLA: &str = "core-008"; // (1) Unit, the spare in hand
    const PALANTIR: &str = "classic-004"; // (1) Field Spell: "Aura: Your opponent can't draw more than 1 card each turn."

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    use crate::js;

    fn chain(library: Value, radiant_face: bool, hand: Option<Vec<Value>>) -> Scenario {
        crate::register_all();
        let mut cards = vec![json!({ "def": CHAIN, "radiant": radiant_face })];
        cards.extend(hand.unwrap_or_else(|| vec![json!(VANILLA)]));
        scenario(json!({
            "p1": { "hand": cards, "library": library },
            "p2": { "hand": [VANILLA], "library": [VANILLA] },
        }))
    }

    fn drawn(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Drawn { player: PlayerId::P1, .. }))
            .count()
    }

    fn count(s: &Scenario, kind: &str) -> usize {
        s.events().iter().filter(|event| js(*event)["type"] == kind).count()
    }

    fn library_defs(s: &Scenario) -> Vec<String> {
        s.pile(P1, "library").iter().map(|card| card.def_id.clone()).collect()
    }

    mod t_ai_4_chain_of_thought {
        use super::*;

        #[test]
        fn is_a_1_ai_spell_token_the_chains_bound_is_chain_of_thought_repeats_4() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(js(&def.cost), json!(1));
            assert_eq!(def.type_, CardType::Spell);
            assert_eq!(js(&def.tags), json!(["AI", "Token"]));
            assert_eq!(CHAIN_OF_THOUGHT_REPEATS, 4);
            let scripts = super::super::script();
            assert!(scripts.base.cry.is_some());
            assert!(scripts.radiant.cry.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn draws_1_and_again_while_the_card_drawn_costs_1_or_less_5_draws_in_all_at_most() {
                let mut s = chain(json!([TIMMY, RAPID, TIMMY, RAPID, TIMMY, TIMMY, TIMMY]), false, None);
                s.play(CHAIN, json!({}));
                assert_eq!(drawn(&s), (CHAIN_OF_THOUGHT_REPEATS + 1) as usize);
                assert_eq!(s.pile(P1, "library").len(), 2);
            }

            #[test]
            fn a_card_that_costs_more_is_still_drawn_and_ends_the_chain() {
                let mut s = chain(json!([TIMMY, D_FENDER, TIMMY]), false, None);
                s.play(CHAIN, json!({}));
                assert_eq!(drawn(&s), 2);
                s.expect_in_zone(D_FENDER, "hand");
                assert_eq!(library_defs(&s), vec![TIMMY]);
            }

            #[test]
            fn a_first_card_that_costs_more_ends_it_at_once() {
                let mut s = chain(json!([MENACE, TIMMY]), false, None);
                s.play(CHAIN, json!({}));
                assert_eq!(drawn(&s), 1);
            }

            #[test]
            fn r65_an_x_cost_card_counts_0() {
                let mut s = chain(json!([ADAPTIVE_UI, TIMMY, D_FENDER]), false, None);
                s.play(CHAIN, json!({}));
                assert_eq!(drawn(&s), 3);
            }

            #[test]
            fn r65_its_current_cost_a_2_card_that_costs_1_less_goes_on_a_1_card_that_costs_1_more_stops() {
                let mut cheaper = chain(json!([{ "def": D_FENDER, "costMod": -1 }, TIMMY, D_FENDER]), false, None);
                cheaper.play(CHAIN, json!({}));
                assert_eq!(drawn(&cheaper), 3);
                let mut dearer = chain(json!([{ "def": TIMMY, "costMod": 1 }, TIMMY]), false, None);
                dearer.play(CHAIN, json!({}));
                assert_eq!(drawn(&dearer), 1);
            }

            #[test]
            fn r596_a_card_cast_on_draw_ends_the_chain_though_its_casts_own_repeat_brings_a_1_card() {
                let mut s = chain(json!([{ "def": HINDER, "radiant": true }, TIMMY, TIMMY, TIMMY]), false, None);
                s.play(CHAIN, json!({}));
                s.expect_in_zone(HINDER, "graveyard");
                // The cast's repeat drew the first Timmy; the chain itself drew nothing more.
                assert_eq!(s.hand(P1).iter().filter(|card| card.def_id == TIMMY).count(), 1);
                assert_eq!(s.pile(P1, "library").len(), 2);
            }

            #[test]
            fn s2_4_r4_a_burned_card_ends_the_chain() {
                let spare: Vec<Value> = (0..9).map(|_| json!(VANILLA)).collect();
                let mut s = chain(json!([TIMMY, TIMMY, TIMMY]), false, Some(spare));
                s.play(CHAIN, json!({}));
                // 9 left in hand after the play: the first Timmy fits, the second burns, and the chain ends.
                assert_eq!(count(&s, "burned"), 1);
                assert_eq!(s.pile(P1, "library").len(), 1);
            }

            #[test]
            fn s2_4_a_fatigue_draw_ends_the_chain_after_one_hit() {
                let mut s = chain(json!([]), false, None);
                s.play(CHAIN, json!({}));
                assert_eq!(count(&s, "fatigue"), 1);
                s.expect_health(P1, 29);
            }

            #[test]
            fn r457_a_draw_the_draw_limit_stops_ends_the_chain_it_is_tried_once_and_not_again() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [CHAIN, VANILLA], "library": [TIMMY, TIMMY, TIMMY] },
                    "p2": { "hand": [VANILLA], "backrow": [{ "def": PALANTIR, "faceUp": true }], "library": [VANILLA] },
                }));
                s.play(CHAIN, json!({}));
                assert_eq!(drawn(&s), 1);
                assert_eq!(count(&s, "drawLimited"), 1);
                assert_eq!(s.pile(P1, "library").len(), 2);
            }

            #[test]
            fn r97_the_opponent_sees_the_draws_under_the_sentinel() {
                let mut s = chain(json!([TIMMY, D_FENDER]), false, None);
                s.play(CHAIN, json!({}));
                let theirs = serde_json::to_string(&s.view(P2)).expect("serialises");
                assert!(!theirs.contains(&s.card(D_FENDER).id));
                assert_eq!(js(&s.view(P2))["opponent"]["hand"], json!({ "count": 3 }));
            }

            #[test]
            fn r682_r431_hinders_random_discard_asks_nothing_mid_draw_no_prompt_opens_and_the_chain_has_ended() {
                let mut s = chain(json!([TIMMY, HINDER, TIMMY, TIMMY]), false, None);
                s.play(CHAIN, json!({}));
                // R682: "Discard 1" names no "of your choice", so the discard is random and no
                // hand prompt pauses the draw; the cast-on-draw card still ends the chain (R58, R596).
                assert!(s.state().pending.is_none());

                // Timmy, then Hinder cast (discarding one card at random), whose repeat drew the
                // second Timmy; the third stays. The chain itself drew nothing more.
                assert_eq!(drawn(&s), 3);
                s.expect_in_zone(HINDER, "graveyard");
                s.expect_in_zone(CHAIN, "graveyard");
                assert_eq!(s.pile(P1, "graveyard").len(), 3);
                assert_eq!(library_defs(&s), vec![TIMMY]);
                assert_eq!(s.hand(P1).len(), 2);
                assert!(s.hand(P1).iter().any(|card| card.def_id == TIMMY));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_threshold_is_2_a_2_card_goes_on_a_3_card_stops() {
                let mut s = chain(json!([D_FENDER, TIMMY, MENACE, TIMMY]), true, None);
                s.play(CHAIN, json!({}));
                assert_eq!(drawn(&s), 3);
                assert_eq!(library_defs(&s), vec![TIMMY]);
            }

            #[test]
            fn still_5_draws_at_most() {
                let mut s = chain(json!([D_FENDER, D_FENDER, D_FENDER, D_FENDER, D_FENDER, D_FENDER]), true, None);
                s.play(CHAIN, json!({}));
                assert_eq!(drawn(&s), (CHAIN_OF_THOUGHT_REPEATS + 1) as usize);
            }

            #[test]
            fn r596_a_card_cast_on_draw_still_ends_it() {
                let mut s = chain(json!([{ "def": HINDER, "radiant": true }, D_FENDER, D_FENDER]), true, None);
                s.play(CHAIN, json!({}));
                assert_eq!(s.pile(P1, "library").len(), 1);
            }
        }
    }
}
