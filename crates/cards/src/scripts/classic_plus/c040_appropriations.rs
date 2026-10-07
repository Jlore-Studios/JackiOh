//! C+ #40 Appropriations (SPEC §8.7 row 40, E38, E39, R58, R60, R80, R81, R177, R311, R348, R380, R581).
//! (X) Spell, Epic. Choose one, with the play (R81); X is at least 1 (R348):
//!   Military:   your Units on the field, in your hand and in your deck get +2X Attack and Rush.
//!   Education:  shuffle 2X random Radiant Books into your deck; they have Cast on draw and aim at
//!               enemies when they harm and at your side when they help.
//!   Culture:    each card on your field, in your hand and in your deck has a 10X% chance to become Radiant.
//!   Healthcare: your Units on the field, in your hand and in your deck get +2X Health and Armor X.
//!   Radiant:    +5X Attack; 5X Books; 25X%; +7X Health and Armor 2X.
//!
//! Military and Healthcare are E38's buffs and grants over your field (tops of piles), hand and deck,
//! carried onto the field as a card enters, silent where a card is hidden (R440). Education's Books are
//! non-token Books of every set (R380), repeats allowed (R60), each Radiant and enchanted (E39), R80's
//! cap turning the rest away. Culture is `radiantChance`'s roll per card, #42 Eugenics's reading
//! (R177, R581): every card is rolled, a Radiant one too, so neither the draws nor the cues count the
//! hidden Radiant cards; a library card made Radiant is listed as it went in (R311). Tunes: none beyond X.

use jackioh_engine::effects::{
    buff_cards, chosen_options, grant_keyword_cards, radiant_chance, shuffle_random_from_catalog,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-040";

const MODES: &[&str] = &["Military", "Education", "Culture", "Healthcare"];

/// One face's multiples of X: attack, Books, percent, health, Armor.
#[derive(Clone, Copy)]
struct Face {
    attack: i32,
    books: i32,
    percent: i32,
    health: i32,
    armor: i32,
}

const BASE: Face = Face { attack: 2, books: 2, percent: 10, health: 2, armor: 1 };
const RADIANT: Face = Face { attack: 5, books: 5, percent: 25, health: 7, armor: 2 };
const ALL: i32 = 100;

/// Your Units on the field, in your hand and in your deck (an E38 card scope).
fn units() -> Value {
    json!({ "side": "self", "zones": ["field", "hand", "library"], "rows": ["units"], "types": ["Unit"] })
}

fn appropriations(face: Face) -> Script {
    Script {
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: MODES.iter().map(|mode| mode.to_string()).collect(),
        }],
        cry: Some(hook(move |ctx| {
            let x = ctx.x;
            match chosen_options(ctx).first().map(String::as_str) {
                Some("Military") => vec![
                    buff_cards(json_as(json!({ "scope": units(), "attack": face.attack * x }))),
                    grant_keyword_cards(json_as(json!({ "scope": units(), "keyword": { "kind": "Rush" } }))),
                ],
                Some("Education") => vec![shuffle_random_from_catalog(json_as(json!({
                    "query": { "tags": ["Book"] },
                    "count": face.books * x,
                    "radiant": true,
                    "enchantments": [{ "kind": "castOnDraw" }, { "kind": "targetEnemies" }],
                })))],
                Some("Culture") => vec![radiant_chance(json_as(json!({
                    "zone": ["field", "hand", "library"],
                    "chance": f64::min(1.0, f64::from(face.percent * x) / f64::from(ALL)),
                })))],
                Some("Healthcare") => vec![
                    buff_cards(json_as(json!({ "scope": units(), "health": face.health * x }))),
                    grant_keyword_cards(json_as(json!({
                        "scope": units(),
                        "keyword": { "kind": "Armor", "n": face.armor * x },
                    }))),
                ],
                _ => vec![],
            }
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: appropriations(BASE),
        radiant: appropriations(RADIANT),
    }
}

// C+ #40 Appropriations — SPEC §8.7 row 40, E38, E39, R60, R80, R81, R97, R177, R311, R348, R380,
// R440, R581, BUILD M9 row C+ 40. Education's verb is proved in packages/engine/test/shuffle-random.test.ts
// (a Book it made is cast as it is drawn, R58), the grants and buffs in the engine's E38 tests.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const APPROPRIATIONS: &str = "classicplus-040";
    const TIMMY: &str = "core-011"; // (1) 3/3 Rush, First Strike
    const VANILLA: &str = "core-008"; // (1) 4/4
    const MENACE: &str = "core-019"; // (3) 9/9 Taunt
    const ARMORED: &str = "core-025"; // (4) 7/7 Armor 7
    const ECLIPSE: &str = "core-035"; // a Spell
    const FILLER: &str = "core-005";
    const FIENDER: &str = "core-092"; // Felinor Fiender, Stack

    use crate::scenario;

    use crate::merged;

    fn appropriations(radiant: bool, p1: Value, p2: Value) -> Scenario {
        scenario(json!({
            "p1": merged(
                json!({
                    "hand": [{ "def": APPROPRIATIONS, "radiant": radiant }, VANILLA, ECLIPSE],
                    "field": [TIMMY],
                    "library": [MENACE, FILLER],
                }),
                p1,
            ),
            "p2": merged(json!({ "hand": [VANILLA], "field": [TIMMY], "library": [FILLER] }), p2),
        }))
    }

    fn mine(s: &Scenario, def_id: &str, zone: &str) -> CardInstance {
        match s.pile(P1, zone).into_iter().find(|held| held.def_id == def_id) {
            Some(card) => card,
            None => panic!("{def_id} in p1's {zone}"),
        }
    }

    fn kind_of(keyword: &Keyword) -> String {
        serde_json::to_value(keyword).expect("a keyword is JSON")["kind"].as_str().unwrap_or_default().to_string()
    }

    fn keywords_of(s: &Scenario, card: &CardInstance) -> Vec<String> {
        s.stats(card).keywords.iter().map(kind_of).collect()
    }

    fn unit(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        s.unit(player, lane).expect("a unit in that lane")
    }

    fn view(s: &Scenario, seat: PlayerId) -> Value {
        serde_json::to_value(s.view(seat)).expect("a view is JSON")
    }

    fn view_events(s: &Scenario, seat: PlayerId) -> Vec<Value> {
        view(s, seat)["events"].as_array().cloned().unwrap_or_default()
    }

    fn events_json(s: &Scenario) -> Vec<Value> {
        s.events().iter().map(|event| serde_json::to_value(event).expect("an event is JSON")).collect()
    }

    fn listed(s: &Scenario) -> Vec<Value> {
        view(s, P1)["you"]["ownLibrary"]["cards"].as_array().cloned().unwrap_or_default()
    }

    fn listed_count(entries: &[Value]) -> i64 {
        entries.iter().map(|entry| entry["count"].as_i64().unwrap_or(0)).sum()
    }

    #[test]
    fn r81_declares_its_four_modes_with_the_play_on_both_faces() {
        let def = crate::card_def(APPROPRIATIONS);
        assert_eq!(def.id, APPROPRIATIONS);
        assert_eq!(serde_json::to_value(def.cost).expect("a cost is JSON"), json!("X"));
        let scripts = super::script();
        for face in [&scripts.base, &scripts.radiant] {
            assert_eq!(
                serde_json::to_value(&face.modes).expect("modes are JSON"),
                json!([{ "kind": "mode", "options": ["Military", "Education", "Culture", "Healthcare"] }])
            );
        }
    }

    #[test]
    fn r348_x_is_at_least_1() {
        let mut s = appropriations(false, json!({}), json!({}));
        s.expect_refused_with(|s| s.play(APPROPRIATIONS, json!({ "x": 0, "modes": ["Military"] })), "X");
    }

    mod military {
        use super::*;

        #[test]
        fn e38_your_units_on_the_field_in_your_hand_and_in_your_deck_get_2x_attack_and_rush_spells_and_the_opponents_units_do_not() {
            let mut s = appropriations(false, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Military"] }));
            let timmy = unit(&s, P1, 1);
            s.expect_stats(&timmy, json!({ "attack": 7, "health": 3 }));
            assert!(keywords_of(&s, &timmy).contains(&"Rush".to_string()));
            for card in [mine(&s, VANILLA, "hand"), mine(&s, MENACE, "library")] {
                assert_eq!(card.buffs.attack, 4);
                assert!(card.granted_keywords.iter().map(kind_of).any(|kind| kind == "Rush"));
            }
            assert_eq!(mine(&s, ECLIPSE, "hand").buffs.attack, 0);
            assert_eq!(mine(&s, FILLER, "library").buffs.attack, 0);
            let theirs = unit(&s, P2, 1);
            s.expect_stats(&theirs, json!({ "attack": 3 }));
        }

        #[test]
        fn r13_a_unit_dormant_under_a_stack_is_not_on_the_field_only_the_piles_top_is_buffed() {
            let mut s = appropriations(false, json!({ "field": [VANILLA, { "def": FIENDER, "stack": true }] }), json!({}));
            let buried = s.state().players.p1.units[0].as_ref().expect("a pile in lane 1")[1].clone();
            s.play(APPROPRIATIONS, json!({ "x": 1, "modes": ["Military"] }));
            assert_eq!(buried.def_id, VANILLA);
            assert_eq!(s.card(&buried).buffs.attack, 0);
            assert!(s.card(&buried).granted_keywords.is_empty());
            assert_eq!(s.card(FIENDER).buffs.attack, 2);
        }

        #[test]
        fn e38_a_hand_unit_carries_the_buff_and_rush_onto_the_field_as_it_enters() {
            let mut s = appropriations(false, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 1, "modes": ["Military"] }));
            s.play(VANILLA, json!({ "zone": 2 }));
            let vanilla = unit(&s, P1, 2);
            s.expect_stats(&vanilla, json!({ "attack": 6, "health": 4 }));
            assert!(keywords_of(&s, &vanilla).contains(&"Rush".to_string()));
        }

        #[test]
        fn r97_r440_hand_and_deck_changes_are_hidden_from_the_opponent_the_decks_are_unread_by_its_owner_r311() {
            let mut s = appropriations(false, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 1, "modes": ["Military"] }));
            let hidden = [mine(&s, VANILLA, "hand").id, mine(&s, MENACE, "library").id];
            let named = |events: &[Value]| -> Vec<String> {
                events
                    .iter()
                    .filter(|event| event["type"] == "buffed" || event["type"] == "keywordGranted")
                    .filter_map(|event| event["instanceId"].as_str().map(str::to_string))
                    .collect()
            };
            let theirs = named(&view_events(&s, P2));
            for id in &hidden {
                assert!(!theirs.contains(id));
            }
            assert!(listed(&s).contains(&json!({ "defId": MENACE, "radiant": false, "count": 1 })));
        }

        #[test]
        fn radiant_5x_attack_and_rush() {
            let mut s = appropriations(true, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Military"] }));
            let timmy = unit(&s, P1, 1);
            s.expect_stats(&timmy, json!({ "attack": 13 }));
            assert_eq!(mine(&s, VANILLA, "hand").buffs.attack, 10);
        }
    }

    mod education {
        use super::*;

        #[test]
        fn e39_r380_shuffles_2x_random_non_token_books_into_your_deck_radiant_with_cast_on_draw_and_target_enemies_riding_them() {
            let mut s = appropriations(false, json!({ "library": [] }), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Education"] }));
            let library = s.pile(P1, "library");
            assert_eq!(library.len(), 4);
            for card in &library {
                assert!(crate::card_def(&card.def_id).tags.contains(&Tag::Book));
                assert!(!crate::card_def(&card.def_id).token);
                assert!(card.radiant);
                assert!(has_enchantment(card, EnchantmentKind::CastOnDraw));
                assert!(has_enchantment(card, EnchantmentKind::TargetEnemies));
            }
        }

        #[test]
        fn r656_every_books_target_declaration_says_which_it_aims_help_for_heal_and_buff_harm_by_default() {
            let scripts = crate::scripts_of();
            let aimed = |id: &str| -> Vec<Option<TargetAim>> {
                scripts[id]
                    .base
                    .targets
                    .iter()
                    .map(|decl| if decl.kind == PromptKind::Target { decl.aim } else { None })
                    .collect()
            };
            assert_eq!(aimed("classic-003"), vec![Some(TargetAim::Help)]); // Book of Heal
            assert_eq!(aimed("classicplus-057"), vec![Some(TargetAim::Help)]); // Book of Stats
            assert_eq!(aimed("classicplus-071"), vec![Some(TargetAim::Help)]); // Book of Buff
            assert_eq!(aimed("classicplus-010"), vec![Some(TargetAim::Help)]); // New Wraps
            assert_eq!(aimed("classic-016"), vec![None]); // Book of Flame
            assert_eq!(aimed("classic-012"), vec![None]); // Book of Blood
            assert_eq!(aimed("classic-055"), vec![None]); // Book of Wildfire
            assert_eq!(aimed("classic-029"), vec![None]); // Book of Vital Kill
        }

        #[test]
        fn r656_an_education_book_that_helps_aims_friends_a_drawn_book_of_heal_offers_only_your_side() {
            let mut s = scenario(json!({
                "seed": "appropriations-aim",
                "p1": { "hand": [VANILLA], "field": [TIMMY], "library": ["classic-003"] },
                "p2": { "hand": [VANILLA], "field": [TIMMY], "library": [FILLER] },
            }));
            let Some(book) = s.pile(P1, "library").into_iter().find(|card| card.def_id == "classic-003") else {
                panic!("Book of Heal in p1's library");
            };
            find_instance_mut(s.state_mut(), &book.id).expect("the Book").enchantments =
                Some(json_as(json!([{ "kind": "castOnDraw" }, { "kind": "targetEnemies" }])));
            let timmy = unit(&s, P1, 1);
            find_instance_mut(s.state_mut(), &timmy.id).expect("Timmy").damage = 1;
            s.end_turn().end_turn();
            // The draw casts the Book, which asks its caster for a target aimed at friends.
            let pending = s.state().pending.clone();
            assert_eq!(pending.as_ref().map(|prompt| prompt.kind), Some(PromptKind::Target));
            let selections: Vec<Selection> = pending
                .as_ref()
                .map(|prompt| prompt.options.iter().map(|option| option.selection.clone()).collect())
                .unwrap_or_default();
            assert!(!selections.is_empty());
            for selection in &selections {
                match selection {
                    Selection::Hero { player } => assert_eq!(*player, P1),
                    Selection::Instance { instance_id } => assert_eq!(*instance_id, timmy.id),
                    other => panic!("unexpected pick {other:?}"),
                }
            }
            let key = pending.and_then(|prompt| prompt.options.first().map(|option| option.key.clone())).unwrap_or_default();
            s.answer(json!(key));
            assert!(events_json(&s).iter().any(|event| event["type"] == "healed" && event["targetId"] == timmy.id.as_str()));
        }

        #[test]
        fn r311_the_shuffle_in_is_open_to_its_owner_the_deck_list_shows_the_radiant_books_the_opponent_sees_a_count() {
            let mut s = appropriations(false, json!({ "library": [] }), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 1, "modes": ["Education"] }));
            let entries = listed(&s);
            assert_eq!(listed_count(&entries), 2);
            for entry in &entries {
                assert_eq!(entry["radiant"], true);
            }
            let theirs_view = view(&s, P2);
            assert_eq!(theirs_view["opponent"]["libraryCount"], 2);
            assert!(theirs_view["opponent"].get("ownLibrary").is_none());
            // R97: which Books went in is the owner's to know, not the opponent's.
            let theirs = serde_json::to_string(&theirs_view).expect("a view is JSON");
            for card in s.pile(P1, "library") {
                assert!(!theirs.contains(&card.def_id));
            }
        }

        #[test]
        fn e39_r58_a_book_it_made_is_cast_as_it_is_drawn_its_enchantment_riding_it_from_the_deck() {
            let mut s = appropriations(false, json!({ "library": [] }), json!({ "library": [FILLER, FILLER] }));
            s.play(APPROPRIATIONS, json!({ "x": 1, "modes": ["Education"] }));
            let books: Vec<String> = s.pile(P1, "library").into_iter().map(|card| card.id).collect();
            s.end_turn().end_turn();
            // A Book that asks for a target asks its caster; any answer will do here.
            let mut n = 0;
            while n < 6 {
                let Some(pending) = s.state().pending.clone() else {
                    break;
                };
                let key = pending.options.first().map(|option| option.key.clone()).unwrap_or_default();
                s.answer(json!(key));
                n += 1;
            }
            let mut cast: Vec<String> = events_json(&s)
                .iter()
                .filter(|event| event["type"] == "cardPlayed")
                .filter_map(|event| event["instanceId"].as_str().map(str::to_string))
                .filter(|id| books.contains(id))
                .collect();
            // The turn's draw casts one; a Book that draws casts the next as it comes (R58's chain).
            let library: Vec<String> = s.pile(P1, "library").into_iter().map(|card| card.id).collect();
            let mut drawn: Vec<String> = books.iter().filter(|id| !library.contains(id)).cloned().collect();
            assert!(!drawn.is_empty());
            cast.sort();
            drawn.sort();
            assert_eq!(cast, drawn);
            assert!(!s.hand(P1).iter().any(|card| books.contains(&card.id)));
        }

        #[test]
        fn r80_a_full_deck_turns_the_rest_away() {
            let library: Vec<&str> = (0..LIBRARY_CAP - 1).map(|_| FILLER).collect();
            let mut s = appropriations(false, json!({ "library": library }), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Education"] }));
            assert_eq!(s.pile(P1, "library").len(), LIBRARY_CAP as usize);
            assert_eq!(events_json(&s).iter().filter(|event| event["type"] == "libraryOverflow").count(), 3);
        }

        #[test]
        fn radiant_5x_books() {
            let mut s = appropriations(true, json!({ "library": [] }), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Education"] }));
            assert_eq!(s.pile(P1, "library").len(), 10);
        }
    }

    mod culture {
        use super::*;

        #[test]
        fn r60_each_card_on_your_field_in_your_hand_and_in_your_deck_rolls_its_own_10x_over_seeds_some_turn_and_some_do_not() {
            let mut turned = 0;
            let mut stayed = 0;
            for n in 0..10 {
                let mut s = scenario(json!({
                    "seed": format!("culture-{n}"),
                    "p1": { "hand": [APPROPRIATIONS, VANILLA, ECLIPSE], "field": [TIMMY], "library": [MENACE, FILLER] },
                    "p2": { "hand": [VANILLA], "field": [TIMMY] },
                }));
                s.play(APPROPRIATIONS, json!({ "x": 4, "modes": ["Culture"] }));
                let mut cards = s.hand(P1);
                cards.extend(s.pile(P1, "library"));
                cards.push(unit(&s, P1, 1));
                for card in &cards {
                    if card.radiant {
                        turned += 1;
                    } else {
                        stayed += 1;
                    }
                }
                assert_eq!(s.unit(P2, 1).map(|card| card.radiant), Some(false));
                assert_eq!(s.hand(P2).first().map(|card| card.radiant), Some(false));
            }
            assert!(turned > 0);
            assert!(stayed > 0);
        }

        #[test]
        fn r581_every_card_is_rolled_a_radiant_one_too_so_the_draws_never_count_the_hidden_radiant_cards() {
            let rolls = |hand_radiant: bool| -> u32 {
                let mut s = appropriations(
                    false,
                    json!({
                        "hand": [APPROPRIATIONS, { "def": VANILLA, "radiant": hand_radiant }, ECLIPSE],
                        "field": [TIMMY],
                        "library": [MENACE, FILLER],
                    }),
                    json!({}),
                );
                let cursor = s.state().rng_cursor;
                s.play(APPROPRIATIONS, json!({ "x": 1, "modes": ["Culture"] }));
                s.state().rng_cursor - cursor
            };
            // Field 1 + hand 2 + deck 2: five rolls whether or not the hand's Vanilla is Radiant already.
            assert_eq!(rolls(false), 5);
            assert_eq!(rolls(true), 5);
        }

        #[test]
        fn r311_r177_radiant_at_x_4_is_100_every_card_turns_the_decks_listed_as_they_went_in_the_hands_hidden_from_the_opponent() {
            let mut s = appropriations(true, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 4, "modes": ["Culture"] }));
            let mut cards = s.hand(P1);
            cards.extend(s.pile(P1, "library"));
            for card in &cards {
                assert!(card.radiant);
            }
            let timmy = unit(&s, P1, 1);
            s.expect_stats(&timmy, json!({ "attack": 6, "health": 6 }));
            let entries = listed(&s);
            assert_eq!(listed_count(&entries), 2);
            for entry in &entries {
                assert_eq!(entry["radiant"], false);
            }
            let hand_ids: Vec<String> = s.hand(P1).into_iter().map(|card| card.id).collect();
            let cues: Vec<Value> = view_events(&s, P2).into_iter().filter(|event| event["type"] == "radiantSet").collect();
            // Field 1 + hand 2 + deck 2: every card is cued, the hand's and the deck's under the sentinel.
            assert_eq!(cues.len(), 5);
            for event in &cues {
                let id = event["instanceId"].as_str().unwrap_or_default().to_string();
                assert!(!hand_ids.contains(&id));
            }
        }

        #[test]
        fn r581_r177_a_hand_card_already_radiant_is_cued_like_the_rest_so_the_opponents_cues_never_count_it() {
            let cues = |hand_radiant: bool| -> usize {
                let mut s = appropriations(
                    true,
                    json!({
                        "hand": [{ "def": APPROPRIATIONS, "radiant": true }, { "def": VANILLA, "radiant": hand_radiant }, ECLIPSE],
                        "field": [TIMMY],
                        "library": [MENACE, FILLER],
                    }),
                    json!({}),
                );
                s.play(APPROPRIATIONS, json!({ "x": 4, "modes": ["Culture"] }));
                view_events(&s, P2).into_iter().filter(|event| event["type"] == "radiantSet").count()
            };
            assert_eq!(cues(true), cues(false));
        }
    }

    mod healthcare {
        use super::*;

        #[test]
        fn e38_your_units_everywhere_get_2x_health_and_armor_x_which_stacks_with_printed_armor() {
            let mut s = appropriations(false, json!({ "field": [TIMMY, ARMORED] }), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Healthcare"] }));
            let timmy = unit(&s, P1, 1);
            s.expect_stats(&timmy, json!({ "attack": 3, "health": 7 }));
            assert_eq!(s.stats(&timmy).armor, 2);
            assert_eq!(s.stats(unit(&s, P1, 2)).armor, 9);
            let vanilla = mine(&s, VANILLA, "hand");
            assert_eq!(vanilla.buffs.health, 4);
            assert_eq!(vanilla.granted_keywords, vec![Keyword::Armor { n: 2 }]);
            assert_eq!(mine(&s, MENACE, "library").buffs.health, 4);
            let theirs = unit(&s, P2, 1);
            s.expect_stats(&theirs, json!({ "health": 3 }));
        }

        #[test]
        fn radiant_7x_health_and_armor_2x() {
            let mut s = appropriations(true, json!({}), json!({}));
            s.play(APPROPRIATIONS, json!({ "x": 1, "modes": ["Healthcare"] }));
            let timmy = unit(&s, P1, 1);
            s.expect_stats(&timmy, json!({ "health": 10 }));
            assert_eq!(s.stats(&timmy).armor, 2);
        }
    }
}
