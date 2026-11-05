//! C #70 Book of Plague (SPEC §8.6 row 70). (1) Spell, Book, Plague, Epic.
//!   Base:    "Choose one: Place {tokens|Plague Counter|Plague Counters}, or add {cards|random Plague card|
//!            random Plague cards} to your hand." — tokens 5, cards 3
//!   Radiant: the same with 10 counters and Radiant Plague cards — tokens 10, cards 3
//!
//! Two declared modes (R81), as System Schematic's: the player's pick travels in the `play` action. The
//! first is five (radiant ten) Plague Counter placements (§6.3) on the single permanent a prompt names
//! (R689), over every permanent on the field (face-down cards offered by id alone, R177); with none on the
//! field nothing is placed. Placements are individual (C #53 answers each; C #27 multiplies). Count is
//! `tokens` (R386). The second is `add_random_from_catalog` of the Plague tag: `cards` non-token cards of
//! the shipped sets that carry it, never this one (R380, R387), repeats allowed (R60), Radiant ones on the
//! Radiant face (R74), a full hand burning the rest (§2.4). Through v0.5.11o (`script_v0_5_11o`, kept,
//! R767000) it was the placement alone, with no choice.

use jackioh_engine::effects::{add_random_from_catalog, place_plague_tokens};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-070";

/// The two modes' words: `modeText.ts` gives each its label (R81).
const COUNTERS: &str = "counters";
const CARDS: &str = "cards";

pub fn script() -> CardScripts {
    CardScripts {
        base: book(false),
        radiant: book(true),
    }
}

fn book(radiant: bool) -> Script {
    Script {
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: vec![COUNTERS.to_string(), CARDS.to_string()],
        }],
        cry: Some(hook(move |ctx| {
            let chosen = chosen_options(ctx);
            match chosen.first().map(String::as_str) {
                Some(COUNTERS) => vec![place_plague_tokens(json_as(json!({ "count": param(&*ctx, "tokens") })))],
                Some(CARDS) => vec![add_random_from_catalog(json_as(json!({
                    "query": { "tags": ["Plague"] },
                    "count": param(&*ctx, "cards"),
                    "radiant": radiant,
                })))],
                _ => Vec::new(),
            }
        })),
        ..Script::default()
    }
}

/// R767000: the script v0.5.11o and the versions before it ran, the placement alone with no mode;
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![place_plague_tokens(json_as(json!({ "count": param(&*ctx, "tokens") })))]
        })),
        ..Script::default()
    };
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// C #70 Book of Plague — SPEC §8.6 row 70, BUILD M9 Classic row C 70: Choose one: five placements of one Plague
// Token on the permanent a single prompt names (R689), face-down ones by id alone (R177); with no
// permanent on the field places nothing; individual for C #53 and C #27; tagged Book; radiant: 10 (R386); or
// add three random Plague cards to your hand (Radiant ones on the Radiant face, never this one, R380, R387).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOOK: &str = "classic-070";
    const CRAWLER: &str = "classic-053"; // (1) Unit: whenever Plague Counters are placed on this, draw 1.
    const SLIME: &str = "classic-027"; // (0) Unit: Plague Counters placed on this are doubled.
    const PALANTIR: &str = "classic-004"; // (2) Field Spell: when your opponent plays a (2) Cost Spell, Tribute this to steal it and cast it yourself.
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

    /// The Plague cards a Book of Plague can add: every non-token card of the shipped sets that carries the Plague
    /// tag, but this one (R380, R387).
    fn plague_pool() -> Vec<String> {
        crate::register_all();
        crate::query::pool(ID, &json_as(json!({ "tags": ["Plague"] })))
            .iter()
            .map(|def| def.id.clone())
            .collect()
    }

    /// p1 holds a Book of Plague (`radiant`'s face), p2 has a board to put counters on.
    fn adding(seed: &str, radiant: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": BOOK, "radiant": radiant }, ANCHOR], "field": [VANILLA], "library": lib(3) },
            "p2": { "hand": [ANCHOR], "field": [MENACE] },
        }))
    }

    /// The cards the Book added to p1's hand: everything in it but the Anchor.
    fn added_to_hand(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id != ANCHOR).collect()
    }

    mod modes {
        use super::*;

        /// Issue #3139: two declared modes, chosen with the play (R81): "counters" and "cards".
        #[test]
        fn r81_it_offers_two_modes_place_counters_or_add_plague_cards_on_both_faces() {
            for radiant in [false, true] {
                let s = adding("book-modes", radiant);
                let held = s.card(BOOK).id.clone();
                let offered: Vec<Vec<String>> = legal_actions(s.state(), P1)
                    .iter()
                    .filter_map(|action| match action {
                        ActionBody::Play { instance_id, modes, .. } if *instance_id == held => modes.clone(),
                        _ => None,
                    })
                    .collect();
                assert_eq!(offered, vec![vec![COUNTERS.to_string()], vec![CARDS.to_string()]], "radiant {radiant}");
            }
        }

        #[test]
        fn a_param_declares_the_cards_it_adds_3_on_both_faces() {
            crate::register_all();
            let declared: Vec<(String, i64, i64)> = crate::card_def(ID)
                .params
                .iter()
                .flatten()
                .map(|param| (param.key.clone(), param.base.into(), param.radiant.into()))
                .collect();
            assert_eq!(
                declared,
                vec![("tokens".to_string(), 5, 10), ("cards".to_string(), 3, 3)]
            );
        }

        /// The cards mode asks nothing and places nothing: three Plague cards land in the hand.
        #[test]
        fn the_cards_mode_adds_3_random_plague_cards_to_your_hand_and_opens_no_prompt() {
            let mut s = adding("book-cards", false);
            s.play(BOOK, json!({ "modes": [CARDS] }));

            assert!(s.state().pending.is_none());
            assert!(placements(&s).is_empty());
            assert_eq!(s.card(MENACE).counters.plague.unwrap_or(0), 0);
            let added = added_to_hand(&s);
            assert_eq!(added.len(), 3);
            s.expect_in_zone(BOOK, "graveyard");
            let pool = plague_pool();
            for card in &added {
                assert!(pool.contains(&card.def_id), "{}", card.def_id);
                assert!(!card.radiant, "{}", card.def_id);
            }
        }

        /// R380, R387, R60: the pool is the non-token Plague cards but this one; over many seeds every one is
        /// offered, never a token, never Book of Plague itself, and picks may repeat.
        #[test]
        fn r380_r387_r60_the_pool_is_the_other_non_token_plague_cards_and_picks_may_repeat() {
            let pool = plague_pool();
            assert!(!pool.is_empty());
            assert!(!pool.iter().any(|id| id == ID), "never itself");
            for id in &pool {
                let def = crate::card_def(id);
                assert!(!def.token, "{id} is a token");
                assert!(def.tags.contains(&Tag::Plague), "{id}");
            }
            let mut seen: IndexSet<String> = IndexSet::new();
            let mut repeated = false;
            for n in 0..150 {
                for radiant in [false, true] {
                    let mut s = adding(&format!("book-pool-{n}"), radiant);
                    s.play(BOOK, json!({ "modes": [CARDS] }));
                    let added = added_to_hand(&s);
                    assert_eq!(added.len(), 3);
                    let defs: IndexSet<&str> = added.iter().map(|card| card.def_id.as_str()).collect();
                    repeated |= defs.len() < 3;
                    seen.extend(added.iter().map(|card| card.def_id.clone()));
                }
            }
            assert_eq!(seen, pool.iter().cloned().collect::<IndexSet<String>>(), "every Plague card is offered");
            assert!(repeated, "a pick may repeat a card");
        }

        /// R74: the Radiant face adds Radiant Plague cards.
        #[test]
        fn r74_the_radiant_face_adds_3_radiant_plague_cards() {
            let mut s = adding("book-cards-radiant", true);
            s.play(BOOK, json!({ "modes": [CARDS] }));

            let added = added_to_hand(&s);
            assert_eq!(added.len(), 3);
            let pool = plague_pool();
            for card in &added {
                assert!(card.radiant, "{}", card.def_id);
                assert!(pool.contains(&card.def_id), "{}", card.def_id);
            }
            assert!(s.state().pending.is_none());
        }

        #[test]
        fn r386_an_upgrade_of_the_cards_adds_4() {
            let mut s = adding("book-cards-up", false);
            step_param(s.card_mut(BOOK), "cards", 1);
            s.play(BOOK, json!({ "modes": [CARDS] }));
            assert_eq!(added_to_hand(&s).len(), 4);
        }

        #[test]
        fn s2_4_a_full_hand_burns_the_cards_it_has_no_room_for() {
            let mut s = scenario(json!({
                "seed": "book-cards-full",
                "p1": { "hand": [BOOK, ANCHOR, ANCHOR, ANCHOR, ANCHOR, ANCHOR, ANCHOR, ANCHOR, ANCHOR, ANCHOR], "library": lib(3) },
                "p2": { "hand": [ANCHOR] },
            }));
            s.play(BOOK, json!({ "modes": [CARDS] }));
            // Nine Anchors stay; the Book left, so room for one.
            assert_eq!(s.hand(P1).len(), 10);
            assert_eq!(added_to_hand(&s).len(), 1);
        }

        /// R767000: through v0.5.11o it placed counters with no choice.
        #[test]
        fn r767000_the_script_kept_for_v0_5_11o_places_the_counters_with_no_mode() {
            let kept = script_v0_5_11o();
            assert!(kept.base.modes.is_empty() && kept.radiant.modes.is_empty());
            crate::under_kept(BOOK, kept, || {
                let mut s = board(false);
                s.play(BOOK, json!({}));
                let menace = s.card(MENACE).clone();
                place_all(&mut s, &menace);
                assert_eq!(s.card(&menace).counters.plague, Some(5));
            });
        }
    }

    mod base {
        use super::*;

        #[test]
        fn five_placements_of_one_token_on_the_one_permanent_a_single_prompt_names_over_every_permanent_on_either_side_face_down_included() {
            let mut s = board(false);
            let vanilla = s.card(VANILLA).clone();
            let menace = s.card(MENACE).clone();
            let well = s.card(MANA_WELL).clone();
            let pawn = s.card(PAWN).clone();

            s.play(BOOK, json!({ "modes": [COUNTERS] }));

            assert_eq!(s.state().pending.as_ref().map(|pending| pending.player_id), Some(P1));
            assert_eq!(
                sorted(option_ids(&s)),
                sorted(vec![vanilla.id.clone(), menace.id.clone(), well.id.clone(), pawn.id.clone()])
            );
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
            s.play(BOOK, json!({ "modes": [COUNTERS] }));
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
            s.play(BOOK, json!({ "modes": [COUNTERS] }));

            assert_eq!(option_ids(&s), vec![s.card(FIENDER).id.clone()]);
        }

        #[test]
        fn with_no_permanent_on_the_field_it_places_nothing_asks_nothing_and_still_resolves() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, ANCHOR] }, "p2": { "hand": [ANCHOR] } }));

            s.play(BOOK, json!({ "modes": [COUNTERS] }));

            assert!(s.state().pending.is_none());
            assert!(placements(&s).is_empty());
            s.expect_in_zone(BOOK, "graveyard");
        }

        /// R177 a face-down enemy option carries only its id, and placing on it never names it to you
        #[test]
        fn r177_a_face_down_enemy_option_carries_only_its_id_and_placing_on_it_never_names_it_to_you() {
            let mut s = board(false);
            s.play(BOOK, json!({ "modes": [COUNTERS] }));
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
            s.play(BOOK, json!({ "modes": [COUNTERS] }));

            assert_eq!(js(&s.view(P2).pending), json!({ "forYou": false, "pendingFor": "p1" }));
        }

        /// each placement is its own: a C #53 Plague Crawler that takes all five draws five times
        #[test]
        fn each_placement_is_its_own_a_c_n53_plague_crawler_that_takes_all_five_draws_five_times() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, ANCHOR], "field": [CRAWLER], "library": lib(6) }, "p2": { "hand": [ANCHOR] } }));
            s.play(BOOK, json!({ "modes": [COUNTERS] }));

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
            s.play(BOOK, json!({ "modes": [COUNTERS] }));
            let slime = s.card(SLIME).clone();

            place_all(&mut s, &slime);

            assert_eq!(s.card(&slime).counters.plague, Some(10));
            assert_eq!(s.card(VANILLA).counters.plague.unwrap_or(0), 0);
            assert_eq!(placements(&s), vec![2, 2, 2, 2, 2]);
        }

        /// R3139000: Palantir answers a (2) Cost Spell; the Book costs (1), so it is played as usual. A Book
        /// made dear by a `costMod` of 1 is stolen, and its thief casts it with a random mode (R452).
        #[test]
        fn r3139000_r831001_c_n4_the_opponents_palantir_ignores_it_at_1_and_steals_it_at_2() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, ANCHOR], "field": [VANILLA] }, "p2": { "hand": [ANCHOR], "backrow": [PALANTIR] } }));
            s.play(BOOK, json!({ "modes": [COUNTERS] }));
            s.answer(json!(s.card(VANILLA).id));
            s.expect_in_zone(PALANTIR, "field");
            assert_eq!(s.card(VANILLA).counters.plague, Some(5));
            assert_eq!(s.card(BOOK).zone, Zone::Graveyard { player: P1 });

            let (mut counters, mut cards) = (false, false);
            for n in 0..24 {
                let mut s = scenario(json!({
                    "seed": format!("book-palantir-{n}"),
                    "p1": { "hand": [{ "def": BOOK, "costMod": 1 }, ANCHOR], "field": [VANILLA], "mana": 4 },
                    "p2": { "hand": [ANCHOR], "backrow": [PALANTIR] },
                }));
                let book = s.card(BOOK).clone();

                s.play(&book, json!({ "modes": [COUNTERS] }));

                s.expect_in_zone(PALANTIR, "graveyard");
                assert!(s.state().pending.is_none(), "n {n}");
                assert_eq!(s.card(&book).zone, Zone::Graveyard { player: P2 }, "n {n}");
                let added = s.hand(P2).iter().filter(|card| card.def_id != ANCHOR).count();
                if s.card(VANILLA).counters.plague == Some(5) {
                    counters = true;
                    assert_eq!(added, 0, "n {n}");
                } else {
                    cards = true;
                    assert_eq!(added, 3, "n {n}");
                    assert_eq!(s.card(VANILLA).counters.plague.unwrap_or(0), 0, "n {n}");
                }
            }
            assert!(counters && cards, "the thief's random mode took both: counters {counters}, cards {cards}");
        }

        /// §9.3 the open prompt survives a JSON round trip and finishes as the live one does
        #[test]
        fn s9_3_the_open_prompt_survives_a_json_round_trip_and_finishes_as_the_live_one_does() {
            let mut s = board(false);
            s.play(BOOK, json!({ "modes": [COUNTERS] }));
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
            s.play(BOOK, json!({ "modes": [COUNTERS] }));

            let menace = s.card(MENACE).clone();
            place_all(&mut s, &menace);

            assert!(s.state().pending.is_none());
            assert_eq!(s.card(MENACE).counters.plague, Some(6));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn ten_placements_on_the_one_pick() {
            let mut s = board(true);
            s.play(BOOK, json!({ "modes": [COUNTERS] }));
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
            s.play(BOOK, json!({ "modes": [COUNTERS] }));

            let menace = s.card(MENACE).clone();
            place_all(&mut s, &menace);

            assert!(s.state().pending.is_none());
            assert_eq!(s.card(MENACE).counters.plague, Some(9));
        }
    }
}
