//! T-AI-5 Autocomplete (SPEC §8.7 row T-AI-5, §7, B8). (0) Spell, AI, Token.
//!   Base:    "Add a copy of the last Unit, Spell or Field Spell your opponent played to your hand."
//!   Radiant: "… It costs (0)."
//!   Engine:  "Reads the last face-up card the opponent played (the per-game counts, §10.1: a record each
//!            play overwrites and nothing clears) and adds a copy of its definition and face. Traps and
//!            Field Traps are set face-down, so they never count and nothing hidden is copied; the record
//!            never takes an AI generated card (the AI tag, a fused one included), so two Autocompletes
//!            can't feed each other forever. Casts count (R70); a countered card was never played.
//!            Nothing played yet: nothing. Tunes: none."
//!
//! The record is E4's (`last_face_up_played`, R451), which already passes over Traps and the AI tag
//! (`LAST_FACE_UP_SKIPPED_TAGS`), so this card reads it and adds the copy through §6.3's Add to hand: a
//! new card of yours, the hand cap burning it (§2.4), its face the one played (R57).

use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-t-ai-05";

/// §8.7 Radiant: "It costs (0)."
const RADIANT_COST: i32 = 0;

fn autocomplete(ctx: &EffectContext<'_>, cost_override: Option<i32>) -> Vec<Effect> {
    let Some(last) = last_face_up_played(&*ctx.state, opponent_of(ctx.controller)) else {
        return vec![];
    };
    let mut args = json!({ "defId": last.def_id, "radiant": last.radiant });
    if let Some(cost) = cost_override {
        args["costOverride"] = json!(cost);
    }
    vec![add_to_hand(json_as(args))]
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|ctx| autocomplete(ctx, None))),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|ctx| autocomplete(ctx, Some(RADIANT_COST)))),
            ..Script::default()
        },
    }
}

// T-AI-5 Autocomplete — SPEC §8.7 row T-AI-5, BUILD M9 Classic+ row T-AI-5: "Adds a copy, by definition
// and face, of the last Unit, Spell or Field Spell your opponent played face-up (a cast counts, R70),
// skipping AI generated cards so two Autocompletes never feed each other; Traps and Field Traps never
// count, being set face-down, so nothing hidden is copied; nothing played yet, nothing; the record
// outlives the card leaving play; the copy reaches your hand under the sentinel for the opponent (R97);
// radiant the copy costs (0)".
//
// p2 plays first on its own turn, then ends it, and p1 plays Autocomplete on the next.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const AUTOCOMPLETE: &str = "classicplus-t-ai-05";
    const TAX: &str = "classicplus-t-ai-07"; // (1) AI Spell
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const TWINSPELL: &str = "core-079"; // (2) Field Spell
    const BEAR: &str = "core-060"; // (1) Trap
    const HINDER: &str = "core-021"; // (0) Spell, cast on draw
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy target Unit.
    const TIMMY: &str = "core-011"; // (1) Unit: p1's library, so its draw is never a card under test
    const REFUSAL: &str = "classicplus-t-ai-09"; // (1) Trap: counters a Spell that targets one of your Units

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    use crate::js;

    /// TS's `{ ...base, ...extra }` on a side setup.
    fn spread(base: Value, extra: &Value) -> Value {
        let mut out = base;
        if let (Some(into), Some(from)) = (out.as_object_mut(), extra.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        out
    }

    /// p2 makes `plays` on its turn (each a hand card, played in order, with its zone when it takes one),
    /// then p1's turn begins.
    fn after_their_turn(plays: &[(&str, Option<i32>)], p1: Value, p2: Value, radiant_face: bool) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "active": "p2",
            "p1": spread(
                json!({
                    "hand": [{ "def": AUTOCOMPLETE, "radiant": radiant_face }, VANILLA],
                    "library": [TIMMY, TIMMY, TIMMY],
                }),
                &p1,
            ),
            "p2": spread(
                json!({
                    "hand": [VANILLA, STOCKPILE, TWINSPELL, BEAR, TAX, MENACE],
                    "library": [VANILLA, VANILLA, VANILLA],
                    "mana": 9,
                }),
                &p2,
            ),
        }));
        for (card, zone) in plays {
            match zone {
                Some(zone) => s.play(*card, json!({ "zone": zone })),
                None => s.play(*card, json!({})),
            };
        }
        s.end_turn();
        s
    }

    fn copies_in(s: &Scenario, def_id: &str) -> usize {
        s.hand(P1).iter().filter(|card| card.def_id == def_id).count()
    }

    fn copy_of(s: &Scenario, def_id: &str) -> Option<CardInstance> {
        s.hand(P1).into_iter().find(|card| card.def_id == def_id)
    }

    mod t_ai_5_autocomplete {
        use super::*;

        #[test]
        fn is_a_0_ai_spell_token() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(js(&def.cost), json!(0));
            assert_eq!(def.type_, CardType::Spell);
            assert_eq!(js(&def.tags), json!(["AI", "Token"]));
            let scripts = super::super::script();
            assert!(scripts.base.cry.is_some());
            assert!(scripts.radiant.cry.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn r451_adds_a_copy_of_the_last_unit_they_played_at_its_printed_cost() {
                let mut s = after_their_turn(&[(MENACE, Some(1))], json!({}), json!({}), false);
                s.play(AUTOCOMPLETE, json!({}));
                let copy = copy_of(&s, MENACE);
                assert!(copy.is_some());
                assert_eq!(copy.as_ref().map(|card| card.owner), Some(P1));
                assert!(copy.as_ref().is_some_and(|card| card.cost_override.is_none()));
                assert_eq!(copies_in(&s, MENACE), 1);
                assert_eq!(s.unit(P2, 1).map(|unit| unit.def_id), Some(MENACE.to_string()));
            }

            #[test]
            fn r451_the_last_one_counts_a_spell_after_a_unit_is_the_spell() {
                let mut s = after_their_turn(&[(VANILLA, Some(1)), (STOCKPILE, None)], json!({}), json!({}), false);
                s.play(AUTOCOMPLETE, json!({}));
                assert_eq!(copies_in(&s, STOCKPILE), 1);
                assert_eq!(copies_in(&s, VANILLA), 1); // only the one p1 already held
            }

            #[test]
            fn r451_a_field_spell_counts() {
                let mut s = after_their_turn(&[(STOCKPILE, None), (TWINSPELL, Some(1))], json!({}), json!({}), false);
                s.play(AUTOCOMPLETE, json!({}));
                assert_eq!(copies_in(&s, TWINSPELL), 1);
            }

            #[test]
            fn r33_r451_traps_never_count_being_set_face_down_the_face_up_play_before_it_is_copied() {
                let mut s = after_their_turn(&[(STOCKPILE, None), (BEAR, Some(1))], json!({}), json!({}), false);
                s.play(AUTOCOMPLETE, json!({}));
                assert_eq!(copies_in(&s, BEAR), 0);
                assert_eq!(copies_in(&s, STOCKPILE), 1);
            }

            #[test]
            fn r451_ai_generated_cards_are_skipped_so_two_autocompletes_never_feed_each_other() {
                let mut s = after_their_turn(&[(STOCKPILE, None), (TAX, None)], json!({}), json!({}), false);
                s.play(AUTOCOMPLETE, json!({}));
                assert_eq!(copies_in(&s, TAX), 0);
                assert_eq!(copies_in(&s, STOCKPILE), 1);

                let mut twice = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [AUTOCOMPLETE, VANILLA], "library": [VANILLA, VANILLA] },
                    "p2": { "hand": [AUTOCOMPLETE, VANILLA], "library": [VANILLA, VANILLA] },
                }));
                let theirs = twice
                    .hand(P2)
                    .into_iter()
                    .find(|card| card.def_id == AUTOCOMPLETE)
                    .map(|card| card.id)
                    .unwrap_or_else(|| AUTOCOMPLETE.to_string());
                twice.play(theirs.as_str(), json!({})).end_turn();
                // p2's Autocomplete found nothing of p1's, and it is no record of p2's for p1's to find.
                assert_eq!(twice.state().active, P1);
                let hand_before = twice.hand(P1).len();
                twice.play(AUTOCOMPLETE, json!({}));
                assert_eq!(twice.hand(P1).len(), hand_before - 1);
                assert_eq!(copies_in(&twice, AUTOCOMPLETE), 0);
            }

            #[test]
            fn nothing_played_yet_nothing_and_no_rng_draw_r129() {
                let mut s = after_their_turn(&[], json!({}), json!({}), false);
                let hand_before = s.hand(P1).len();
                let cursor = s.state().rng_cursor;
                s.play(AUTOCOMPLETE, json!({}));
                assert_eq!(s.hand(P1).len(), hand_before - 1);
                assert_eq!(s.state().rng_cursor, cursor);
            }

            #[test]
            fn your_own_plays_never_count() {
                let mut s = after_their_turn(&[(STOCKPILE, None)], json!({}), json!({}), false);
                s.play(VANILLA, json!({ "zone": 1 }));
                s.play(AUTOCOMPLETE, json!({}));
                assert_eq!(copies_in(&s, STOCKPILE), 1);
                assert_eq!(copies_in(&s, VANILLA), 0);
            }

            #[test]
            fn r451_the_record_outlives_the_card_leaving_play() {
                let mut s = after_their_turn(
                    &[(MENACE, Some(1))],
                    json!({ "hand": [AUTOCOMPLETE, HIT_JOB], "mana": 8 }),
                    json!({}),
                    false,
                );
                let target = s.unit(P2, 1).map(|unit| unit.id).unwrap_or_default();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": target }] }));
                s.expect_in_zone(MENACE, "graveyard");
                s.play(AUTOCOMPLETE, json!({}));
                assert_eq!(copies_in(&s, MENACE), 1);
            }

            #[test]
            fn r70_a_cast_is_a_play_their_cast_on_draw_card_is_the_last_one() {
                let mut s = after_their_turn(
                    &[(STOCKPILE, None)],
                    json!({}),
                    json!({ "library": [{ "def": HINDER, "radiant": true }, VANILLA, VANILLA, VANILLA] }),
                    false,
                );
                s.play(AUTOCOMPLETE, json!({}));
                assert_eq!(copies_in(&s, HINDER), 1);
                assert_eq!(copy_of(&s, HINDER).map(|card| card.radiant), Some(true));
            }

            #[test]
            fn r448_a_countered_card_was_never_played_the_play_before_it_is_the_last_one() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": {
                        "hand": [AUTOCOMPLETE, VANILLA],
                        "field": [MENACE],
                        "backrow": [{ "def": REFUSAL, "faceUp": false }],
                        "library": [TIMMY, TIMMY, TIMMY],
                    },
                    "p2": { "hand": [STOCKPILE, HIT_JOB, VANILLA], "library": [VANILLA, VANILLA, VANILLA], "mana": 9 },
                }));
                s.play(STOCKPILE, json!({}));
                let target = s.unit(P1, 1).map(|unit| unit.id).unwrap_or_default();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": target }] }));
                assert!(s.events().iter().any(|event| event.event_type() == GameEventType::Countered));
                s.end_turn().play(AUTOCOMPLETE, json!({}));
                assert_eq!(copies_in(&s, HIT_JOB), 0);
                assert_eq!(copies_in(&s, STOCKPILE), 1);
            }

            #[test]
            fn r57_a_copy_by_definition_and_face_a_radiant_play_is_copied_radiant() {
                let mut s = after_their_turn(
                    &[(MENACE, Some(1))],
                    json!({}),
                    json!({ "hand": [{ "def": MENACE, "radiant": true }, VANILLA], "mana": 4 }),
                    false,
                );
                s.play(AUTOCOMPLETE, json!({}));
                assert_eq!(copy_of(&s, MENACE).map(|card| card.radiant), Some(true));
            }

            #[test]
            fn r97_the_copy_reaches_your_hand_under_the_sentinel_for_the_opponent() {
                let mut s = after_their_turn(&[(MENACE, Some(1))], json!({}), json!({}), false);
                s.play(AUTOCOMPLETE, json!({}));
                let copy = copy_of(&s, MENACE).map(|card| card.id).unwrap_or_else(|| "?".to_string());
                let theirs = serde_json::to_string(&s.view(P2)).expect("serialises");
                assert!(!theirs.contains(&copy));
                assert_eq!(js(&s.view(P2))["opponent"]["hand"], json!({ "count": s.hand(P1).len() }));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_copy_costs_0() {
                let mut s = after_their_turn(&[(MENACE, Some(1))], json!({}), json!({}), true);
                s.play(AUTOCOMPLETE, json!({}));
                let copy = copy_of(&s, MENACE);
                assert_eq!(copy.as_ref().and_then(|card| card.cost_override), Some(0));
                let id = copy.map(|card| card.id).unwrap_or_default();
                let shown = js(&s.view(P1))["you"]["hand"]
                    .as_array()
                    .and_then(|cards| cards.iter().find(|card| card["instanceId"] == id.as_str()).cloned());
                assert_eq!(shown.map(|card| card["cost"].clone()), Some(json!(0)));
            }

            #[test]
            fn nothing_played_yet_nothing() {
                let mut s = after_their_turn(&[], json!({}), json!({}), true);
                let hand_before = s.hand(P1).len();
                s.play(AUTOCOMPLETE, json!({}));
                assert_eq!(s.hand(P1).len(), hand_before - 1);
            }
        }
    }
}
