//! C #76 Plague Bringer (SPEC §8.6 row 76). (2) Unit, Rare, 4/4 → 8/8.
//!   Base:    "Rush
//!             Cry: Place {tokens|Plague Counter|Plague Counters}. Draw {draw}." — 2 tokens, draw 1
//!   Radiant: the same text — 4 tokens, draw 2
//!   Engine:  "Plague Counters (§6.3): two (Radiant four) placements of 1, each on a permanent you choose
//!            (either side, face-down cards included, repeats allowed), one prompt per token. Tunes:
//!            tokens 2 ↑; draw 1 ↑."
//!
//! Rush is printed on both faces. `placePlagueTokens` asks one `target` prompt naming the single
//! permanent every placement lands on (R689; a face-down card the chooser may not read offered by its
//! id alone, R177) and parks the draw behind it (R113). Both numbers are declared (R386).

use jackioh_engine::effects::{draw, place_plague_tokens};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-076";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![
                place_plague_tokens(json_as(json!({ "count": param(&*ctx, "tokens") }))),
                draw(json_as(json!({ "count": param(&*ctx, "draw") }))),
            ]
        })),
        ..Script::default()
    };

    // The same script: the Radiant face's 4 tokens and draw 2 are its declared numbers.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C #76 Plague Bringer — SPEC §8.6 row 76, BUILD M9 Classic row C 76: "Rush; Cry: two placements of one
// Plague Counter, both on the one permanent a single prompt names (R689), on any permanent either side
// (a face-down option carries only its id, R177), then draw 1; radiant 8/8: four placements, draw 2;
// its tuned numbers (tokens, draw) read through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;
    use std::collections::BTreeSet;
    use std::sync::Arc;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BRINGER: &str = "classic-076";
    const CRAWLER: &str = "classic-053"; // (1) Unit: whenever Plague Counters are placed on this, draw 1.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const PAWN: &str = "core-096"; // (1) Trap: answers only an attack that would be lethal.
    const MANA_WELL: &str = "core-006"; // (3) Field Spell.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).
    const X: &str = "core-020"; // library filler.

    /// The harness, after the catalog and every card script are registered (TS's harness did it on import).
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    fn def() -> &'static CardDef {
        crate::register_all();
        crate::card_def(ID)
    }

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialisable")
    }

    fn lib(n: usize) -> Vec<&'static str> {
        vec![X; n]
    }

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(found) => found,
            None => panic!("missing: {what}"),
        }
    }

    fn option_ids(s: &Scenario) -> Vec<String> {
        must(s.state().pending.as_ref(), "an open prompt")
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Instance { instance_id } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn draws_by(events: &[GameEvent], player: PlayerId) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Drawn { player: p, .. } if *p == player))
            .count()
    }

    /// Answer the one placement prompt on `card`: every placement lands there (R689).
    fn place_all(s: &mut Scenario, card: &CardInstance) {
        s.answer(json!(card.id));
    }

    /// TS `stepParam(s.card(ref), key, delta)`: the live card, found again by id.
    fn step(s: &mut Scenario, card: &str, key: &str, delta: i32) {
        let id = s.card(card).id.clone();
        step_param(must(find_instance_mut(s.state_mut(), &id), "the card to tune"), key, delta);
    }

    fn board(radiant_face: bool, library: usize) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": BRINGER, "radiant": radiant_face }, ANCHOR], "library": lib(library) },
            "p2": {
                "hand": [ANCHOR],
                "field": [{ "def": VANILLA, "lane": 2 }],
                "backrow": [MANA_WELL, { "def": PAWN, "faceUp": false, "lane": 2 }],
                "health": 20
            }
        }))
    }

    /// declares its two numbers and one script on both faces; Rush is printed
    #[test]
    fn declares_its_two_numbers_and_one_script_on_both_faces_rush_is_printed() {
        assert_eq!(def().id, BRINGER);
        assert_eq!(
            js(&def().params),
            json!([
                { "key": "tokens", "base": 2, "radiant": 4, "better": "up", "step": 1, "min": 1 },
                { "key": "draw", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 }
            ])
        );
        assert_eq!(js(&def().base.keywords), json!([{ "kind": "Rush" }]));
        assert_eq!(js(&def().radiant.keywords), json!([{ "kind": "Rush" }]));
        let scripts = script();
        assert!(Arc::ptr_eq(
            scripts.base.cry.as_ref().expect("a Cry"),
            scripts.radiant.cry.as_ref().expect("a Cry")
        ));
    }

    /// base
    mod base {
        use super::*;

        /// is a 4/4 with Rush: it attacks a Unit the turn it enters
        #[test]
        fn is_a_4_4_with_rush_it_attacks_a_unit_the_turn_it_enters() {
            let mut s = board(false, 4);
            s.play(BRINGER, json!({}));
            let bringer = s.card(BRINGER).clone();
            place_all(&mut s, &bringer);
            let theirs = must(s.unit(P2, 2), "p2's Vanilla");

            s.expect_stats(&bringer, json!({ "attack": 4, "health": 4 }));
            s.attack(&bringer, &theirs);

            s.expect_in_zone(&theirs, "graveyard");
            s.expect_refused(|s| s.attack(&bringer, "hero"));
        }

        /// Cry: two placements on the one permanent a single prompt names, over every permanent on either side, itself and face-down cards included
        #[test]
        fn cry_two_placements_on_the_one_permanent_a_single_prompt_names_over_every_permanent_on_either_side_itself_and_face_down_cards_included()
         {
            let mut s = board(false, 4);
            s.play(BRINGER, json!({}));
            let bringer = s.card(BRINGER).clone();

            let offered: BTreeSet<String> = option_ids(&s).into_iter().collect();
            let expected: BTreeSet<String> = [
                bringer.id.clone(),
                s.card(VANILLA).id.clone(),
                s.card(MANA_WELL).id.clone(),
                s.card(PAWN).id.clone(),
            ]
            .into_iter()
            .collect();
            assert_eq!(offered, expected);
            // One answer puts both on the pick: no second prompt opens.
            let vanilla_id = s.card(VANILLA).id.clone();
            s.answer(json!(vanilla_id));

            assert!(s.state().pending.is_none());
            assert_eq!(s.card(VANILLA).counters.plague, Some(2));
            assert_eq!(s.card(MANA_WELL).counters.plague.unwrap_or(0), 0);
        }

        /// R689 no spreading; then draw 1, after the one answer
        #[test]
        fn r689_no_spreading_then_draw_1_after_the_one_answer() {
            let mut s = board(false, 4);
            s.play(BRINGER, json!({}));
            let vanilla = s.card(VANILLA).clone();

            s.answer(json!(vanilla.id));

            assert!(s.state().pending.is_none());
            assert_eq!(s.card(&vanilla).counters.plague, Some(2));
            assert_eq!(draws_by(s.last_events(), P1), 1);
        }

        /// R177 a face-down enemy option carries only its id, and the placement on it never names it to you
        #[test]
        fn r177_a_face_down_enemy_option_carries_only_its_id_and_the_placement_on_it_never_names_it_to_you() {
            let mut s = board(false, 4);
            s.play(BRINGER, json!({}));
            let pawn = s.card(PAWN).clone();

            let view = js(&s.view(P1));
            let mine = &view["pending"];
            assert!(!mine.is_null(), "missing: p1's view of the prompt");
            if mine["forYou"] != json!(true) {
                panic!("the prompt is p1's");
            }
            let options = mine["options"].as_array().cloned().unwrap_or_default();
            let option = must(
                options.iter().find(|entry| {
                    entry["instanceId"] == json!(pawn.id)
                        || entry["key"].as_str().is_some_and(|key| key.contains(pawn.id.as_str()))
                }),
                "the trap's option",
            );
            assert!(option.get("defId").is_none());
            place_all(&mut s, &pawn);

            assert_eq!(s.card(&pawn).counters.plague, Some(2));
            assert!(!serde_json::to_string(&s.view(P1)).expect("serialisable").contains(PAWN));
            let theirs = js(&s.view(P2));
            let backrow = theirs["you"]["backrow"].as_array().cloned().unwrap_or_default();
            assert!(backrow.iter().any(|card| !card.is_null() && card["defId"] == json!(PAWN)));
        }

        /// §10.6 the opponent sees only that a prompt is open
        #[test]
        fn s10_6_the_opponent_sees_only_that_a_prompt_is_open() {
            let mut s = board(false, 4);
            s.play(BRINGER, json!({}));
            assert_eq!(js(&s.view(P2))["pending"], json!({ "forYou": false, "pendingFor": "p1" }));
        }

        /// each placement is its own: a C #53 Plague Crawler that takes both draws twice
        #[test]
        fn each_placement_is_its_own_a_c_n53_plague_crawler_that_takes_both_draws_twice() {
            let mut s = scenario(json!({
                "p1": { "hand": [BRINGER, ANCHOR], "field": [CRAWLER], "library": lib(4) },
                "p2": { "hand": [ANCHOR] }
            }));
            s.play(BRINGER, json!({}));

            let crawler = s.card(CRAWLER).clone();
            place_all(&mut s, &crawler);

            // Two Crawler draws and the Bringer's own.
            assert_eq!(draws_by(s.events(), P1), 3);
        }

        /// §9.3 the open prompt survives a JSON round trip
        #[test]
        fn s9_3_the_open_prompt_survives_a_json_round_trip() {
            let mut s = board(false, 4);
            s.play(BRINGER, json!({}));
            let vanilla = s.card(VANILLA).clone();

            let revived: GameState = serde_json::from_str(&serde_json::to_string(s.state()).expect("serialisable"))
                .expect("a state reads back");
            assert_eq!(&revived, s.state());
            let choice = must(revived.pending.clone(), "the placement prompt");
            let action: Action = json_as(json!({
                "type": "answer",
                "playerId": "p1",
                "choiceId": choice.id,
                "selection": [{ "pick": "instance", "instanceId": vanilla.id }],
                "nonce": "bringer-json"
            }));
            let result = reduce(&revived, &action);
            assert!(result.error.is_none());
            s.answer(json!(vanilla.id));

            assert!(result.state.pending.is_none());
            assert_eq!(hash_state(&result.state), hash_state(s.state()));
            assert_eq!(draws_by(&result.events, P1), 1);
        }

        /// §2.4 with an empty deck the draw is fatigue
        #[test]
        fn s2_4_with_an_empty_deck_the_draw_is_fatigue() {
            let mut s = board(false, 0);
            s.play(BRINGER, json!({}));
            let vanilla = s.card(VANILLA).clone();
            place_all(&mut s, &vanilla);

            let fatigue = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Fatigue { player: PlayerId::P1, .. }))
                .count();
            assert_eq!(fatigue, 1);
        }

        /// R386 an Upgrade places three and draws 2
        #[test]
        fn r386_an_upgrade_places_three_and_draws_2() {
            let mut s = board(false, 4);
            step(&mut s, BRINGER, "tokens", 1);
            step(&mut s, BRINGER, "draw", 1);
            s.play(BRINGER, json!({}));

            let vanilla = s.card(VANILLA).clone();
            place_all(&mut s, &vanilla);

            assert!(s.state().pending.is_none());
            assert_eq!(s.card(VANILLA).counters.plague, Some(3));
            assert_eq!(draws_by(s.last_events(), P1), 2);
        }
    }

    /// radiant
    mod radiant {
        use super::*;

        /// is an 8/8 with Rush; four placements on the one pick, then draw 2
        #[test]
        fn is_an_8_8_with_rush_four_placements_on_the_one_pick_then_draw_2() {
            let mut s = board(true, 4);
            s.play(BRINGER, json!({}));
            let bringer = s.card(BRINGER).clone();

            s.expect_stats(&bringer, json!({ "attack": 8, "health": 8 }));
            let vanilla = s.card(VANILLA).clone();
            place_all(&mut s, &vanilla);

            assert!(s.state().pending.is_none());
            assert_eq!(s.card(VANILLA).counters.plague, Some(4));
            assert_eq!(draws_by(s.last_events(), P1), 2);
        }

        /// R386 a Degrade places three and draws 1
        #[test]
        fn r386_a_degrade_places_three_and_draws_1() {
            let mut s = board(true, 4);
            step(&mut s, BRINGER, "tokens", -1);
            step(&mut s, BRINGER, "draw", -1);
            s.play(BRINGER, json!({}));

            let vanilla = s.card(VANILLA).clone();
            place_all(&mut s, &vanilla);

            assert!(s.state().pending.is_none());
            assert_eq!(s.card(VANILLA).counters.plague, Some(3));
            assert_eq!(draws_by(s.last_events(), P1), 1);
        }

        /// §10.6 the opponent's Menace is offered too, and the opponent sees only that a prompt is open
        #[test]
        fn s10_6_the_opponent_s_menace_is_offered_too_and_the_opponent_sees_only_that_a_prompt_is_open() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": BRINGER, "radiant": true }, ANCHOR], "library": lib(3) },
                "p2": { "hand": [ANCHOR], "field": [MENACE] }
            }));
            s.play(BRINGER, json!({}));

            let menace_id = s.card(MENACE).id.clone();
            assert!(option_ids(&s).contains(&menace_id));
            assert_eq!(js(&s.view(P2))["pending"], json!({ "forYou": false, "pendingFor": "p1" }));
        }
    }
}
