//! C #70 Book of Plague (SPEC §8.6 row 70). (1) Spell, Book, Epic.
//!   Base:    "Place {tokens|Plague Counter|Plague Counters}." — 5; Radiant: the same text — 10.
//!
//! Five (radiant ten) Plague Counter placements (§6.3) on the single permanent a prompt names (R689),
//! over every permanent on the field (face-down cards offered by id alone, R177); with none on the field
//! nothing is placed. Placements are individual (C #53 answers each; C #27 multiplies). Count is `tokens` (R386).

use jackioh_engine::effects::place_plague_tokens;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-070";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![place_plague_tokens(json_as(json!({ "count": param(&*ctx, "tokens") })))]
        })),
        ..Script::default()
    };
    CardScripts {
        // The same script: the Radiant face's 10 is its declared `tokens`.
        radiant: base.clone(),
        base,
    }
}

// C #70 Book of Plague — SPEC §8.6 row 70, BUILD M9 Classic row C 70: Five placements of one Plague
// Token on the permanent a single prompt names (R689), face-down ones by id alone (R177); with no
// permanent on the field places nothing; individual for C #53 and C #27; tagged Book; radiant: 10 (R386).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOOK: &str = "classic-070";
    const CRAWLER: &str = "classic-053"; // (1) Unit: whenever Plague Counters are placed on this, draw 1.
    const SLIME: &str = "classic-027"; // (0) Unit: Plague Counters placed on this are doubled.
    const PALANTIR: &str = "classic-004"; // (1) Field Spell: when your opponent plays a Book, you may Tribute this to steal it.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const FIENDER: &str = "core-092"; // (2) Unit 5/7 Stack.
    const PAWN: &str = "core-096"; // (1) Trap: answers only an attack that would be lethal.
    const MANA_WELL: &str = "core-006"; // (3) Field Spell.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).
    const X: &str = "core-020"; // library filler.

    use crate::scenario;

    use crate::js;

    fn lib(n: usize) -> Vec<&'static str> {
        vec![X; n]
    }

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(value) => value,
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

    fn placements(s: &Scenario) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CounterChanged { placed: Some(placed), .. } => Some(*placed),
                _ => None,
            })
            .collect()
    }

    fn draws_by(events: &[GameEvent], player: PlayerId) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Drawn { player: drawer, .. } if *drawer == player))
            .count()
    }

    /// Answer the one placement prompt on `card`: every placement lands there (R689).
    fn place_all(s: &mut Scenario, card: &CardInstance) {
        s.answer(json!(card.id));
    }

    fn board(radiant_face: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": BOOK, "radiant": radiant_face }, ANCHOR], "field": [VANILLA], "library": lib(3) },
            "p2": { "hand": [ANCHOR], "field": [MENACE], "backrow": [MANA_WELL, { "def": PAWN, "faceUp": false, "lane": 2 }] },
        }))
    }

    fn sorted(mut ids: Vec<String>) -> Vec<String> {
        ids.sort();
        ids.dedup();
        ids
    }

    /// is a Spell tagged Book and Plague with one number, and one script on both faces
    #[test]
    fn is_a_spell_tagged_book_and_plague_with_one_number_and_one_script_on_both_faces() {
        crate::register_all();
        let def = crate::card_def(BOOK);
        assert_eq!(def.id, BOOK);
        assert_eq!(def.type_, CardType::Spell);
        assert_eq!(def.tags, vec![Tag::Book, Tag::Plague]);
        assert_eq!(
            js(&def.params),
            json!([{ "key": "tokens", "base": 5, "radiant": 10, "better": "up", "step": 1, "min": 1 }])
        );
        // The Radiant face is the base face, its Cry the same one.
        let scripts = script();
        assert!(Arc::ptr_eq(
            scripts.base.cry.as_ref().expect("a Cry"),
            scripts.radiant.cry.as_ref().expect("a Cry")
        ));
    }

    mod base {
        use super::*;

        /// five placements of one token on the one permanent a single prompt names, over every permanent on either side, face-down included
        #[test]
        fn five_placements_of_one_token_on_the_one_permanent_a_single_prompt_names_over_every_permanent_on_either_side_face_down_included() {
            let mut s = board(false);
            let vanilla = s.card(VANILLA).clone();
            let menace = s.card(MENACE).clone();
            let well = s.card(MANA_WELL).clone();
            let pawn = s.card(PAWN).clone();

            s.play(BOOK, json!({}));

            assert_eq!(s.state().pending.as_ref().map(|pending| pending.player_id), Some(P1));
            assert_eq!(
                sorted(option_ids(&s)),
                sorted(vec![vanilla.id.clone(), menace.id.clone(), well.id.clone(), pawn.id.clone()])
            );
            // One answer puts all five on the pick: no second prompt opens.
            s.answer(json!(menace.id));

            assert!(s.state().pending.is_none());
            assert_eq!(placements(&s), vec![1, 1, 1, 1, 1]);
            let counts: Vec<i32> = [&vanilla, &menace, &well, &pawn]
                .iter()
                .map(|card| s.card(*card).counters.plague.unwrap_or(0))
                .collect();
            assert_eq!(counts, vec![0, 5, 0, 0]);
            s.expect_in_zone(BOOK, "graveyard");
        }

        /// R689 the counters cannot be spread: the whole effect lands on the one pick
        #[test]
        fn r689_the_counters_cannot_be_spread_the_whole_effect_lands_on_the_one_pick() {
            let mut s = board(false);
            s.play(BOOK, json!({}));
            let menace = s.card(MENACE).clone();
            place_all(&mut s, &menace);

            assert!(s.state().pending.is_none());
            assert_eq!(s.card(MENACE).counters.plague, Some(5));
        }

        /// §3.2 R13 a card dormant under a Stack pile is no option; the top of the pile is
        #[test]
        fn s3_2_r13_a_card_dormant_under_a_stack_pile_is_no_option_the_top_of_the_pile_is() {
            let mut s = scenario(json!({
                "p1": { "hand": [BOOK, ANCHOR] },
                "p2": { "hand": [ANCHOR], "field": [VANILLA, { "def": FIENDER, "stack": true }] },
            }));
            s.play(BOOK, json!({}));

            assert_eq!(option_ids(&s), vec![s.card(FIENDER).id.clone()]);
        }

        /// with no permanent on the field it places nothing, asks nothing, and still resolves
        #[test]
        fn with_no_permanent_on_the_field_it_places_nothing_asks_nothing_and_still_resolves() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, ANCHOR] }, "p2": { "hand": [ANCHOR] } }));

            s.play(BOOK, json!({}));

            assert!(s.state().pending.is_none());
            assert!(placements(&s).is_empty());
            s.expect_in_zone(BOOK, "graveyard");
        }

        /// R177 a face-down enemy option carries only its id, and placing on it never names it to you
        #[test]
        fn r177_a_face_down_enemy_option_carries_only_its_id_and_placing_on_it_never_names_it_to_you() {
            let mut s = board(false);
            s.play(BOOK, json!({}));
            let pawn = s.card(PAWN).clone();

            let view = js(&s.view(P1));
            let mine = must(Some(&view["pending"]).filter(|pending| !pending.is_null()), "p1's view of the prompt");
            if mine["forYou"] != json!(true) {
                panic!("the prompt is p1's");
            }
            let option = must(
                mine["options"].as_array().and_then(|options| {
                    options.iter().find(|entry| {
                        entry["instanceId"] == json!(pawn.id)
                            || entry["key"].as_str().is_some_and(|key| key.contains(&pawn.id))
                    })
                }),
                "the trap's option",
            );
            assert!(option.get("defId").is_none());
            assert!(!view.to_string().contains(PAWN));
            place_all(&mut s, &pawn);

            assert_eq!(s.card(&pawn).counters.plague, Some(5));
            assert!(!js(&s.view(P1)).to_string().contains(PAWN));
            assert!(!js(&s.view(P1)).to_string().contains("My Pawn"));
        }

        /// §10.6 the opponent sees only that a prompt is open
        #[test]
        fn s10_6_the_opponent_sees_only_that_a_prompt_is_open() {
            let mut s = board(false);
            s.play(BOOK, json!({}));

            assert_eq!(js(&s.view(P2).pending), json!({ "forYou": false, "pendingFor": "p1" }));
        }

        /// each placement is its own: a C #53 Plague Crawler that takes all five draws five times
        #[test]
        fn each_placement_is_its_own_a_c_n53_plague_crawler_that_takes_all_five_draws_five_times() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, ANCHOR], "field": [CRAWLER], "library": lib(6) }, "p2": { "hand": [ANCHOR] } }));
            s.play(BOOK, json!({}));

            let crawler = s.card(CRAWLER).clone();
            place_all(&mut s, &crawler);

            assert_eq!(draws_by(s.events(), P1), 5);
        }

        /// C #27 a Pestilent Slime doubles its share: all five on it put 10
        #[test]
        fn c_n27_a_pestilent_slime_doubles_its_share_all_five_on_it_put_10() {
            let mut s = scenario(json!({
                "p1": { "hand": [BOOK, ANCHOR], "field": [SLIME, { "def": VANILLA, "lane": 2 }] },
                "p2": { "hand": [ANCHOR] },
            }));
            s.play(BOOK, json!({}));
            let slime = s.card(SLIME).clone();

            place_all(&mut s, &slime);

            assert_eq!(s.card(&slime).counters.plague, Some(10));
            assert_eq!(s.card(VANILLA).counters.plague.unwrap_or(0), 0);
            assert_eq!(placements(&s), vec![2, 2, 2, 2, 2]);
        }

        /// C #4 tagged Book: the opponent's Palantir steals it outright, and the stolen Book places nothing
        #[test]
        fn c_n4_tagged_book_the_opponents_palantir_steals_it_outright_and_the_stolen_book_places_nothing() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, ANCHOR], "field": [VANILLA] }, "p2": { "hand": [ANCHOR], "backrow": [PALANTIR] } }));
            let book = s.card(BOOK).clone();

            s.play(&book, json!({}));

            // No "you may": the steal is answered without asking, so no prompt ever opens.
            assert!(s.state().pending.is_none());
            assert!(placements(&s).is_empty());
            assert!(s.hand(P2).iter().any(|card| card.id == book.id));
            // Palantir paid its price.
            s.expect_in_zone(PALANTIR, "graveyard");
        }

        /// §9.3 the open prompt survives a JSON round trip and finishes as the live one does
        #[test]
        fn s9_3_the_open_prompt_survives_a_json_round_trip_and_finishes_as_the_live_one_does() {
            let mut s = board(false);
            s.play(BOOK, json!({}));
            let menace = s.card(MENACE).clone();

            let revived: GameState = serde_json::from_value(js(s.state())).expect("the state round-trips through JSON");
            assert_eq!(&revived, s.state());
            let choice = must(revived.pending.clone(), "the placement prompt");
            let action: Action = json_as(json!({
                "type": "answer",
                "playerId": "p1",
                "choiceId": choice.id,
                "selection": [{ "pick": "instance", "instanceId": menace.id }],
                "nonce": "book-json-0",
            }));
            let result = reduce(&revived, &action);
            assert!(result.error.is_none());
            place_all(&mut s, &menace);

            assert!(result.state.pending.is_none());
            assert_eq!(hash_state(&result.state), hash_state(s.state()));
            assert_eq!(s.card(&menace).counters.plague, Some(5));
        }

        /// R386 an Upgrade places six
        #[test]
        fn r386_an_upgrade_places_six() {
            let mut s = board(false);
            step_param(s.card_mut(BOOK), "tokens", 1);
            s.play(BOOK, json!({}));

            let menace = s.card(MENACE).clone();
            place_all(&mut s, &menace);

            assert!(s.state().pending.is_none());
            assert_eq!(s.card(MENACE).counters.plague, Some(6));
        }
    }

    mod radiant {
        use super::*;

        /// ten placements on the one pick
        #[test]
        fn ten_placements_on_the_one_pick() {
            let mut s = board(true);
            s.play(BOOK, json!({}));
            let menace = s.card(MENACE).clone();

            place_all(&mut s, &menace);

            assert!(s.state().pending.is_none());
            assert_eq!(s.card(&menace).counters.plague, Some(10));
            assert_eq!(placements(&s).len(), 10);
        }

        /// R386 a Degrade places nine
        #[test]
        fn r386_a_degrade_places_nine() {
            let mut s = board(true);
            step_param(s.card_mut(BOOK), "tokens", -1);
            s.play(BOOK, json!({}));

            let menace = s.card(MENACE).clone();
            place_all(&mut s, &menace);

            assert!(s.state().pending.is_none());
            assert_eq!(s.card(MENACE).counters.plague, Some(9));
        }
    }
}
