//! C #55 Book of Wildfire (SPEC §8.6 row 55, BUILD M9 Classic row C 55). (1) Spell, Book, Epic.
//!   Base:    "Deal {damage} damage. / End of turn: Become a different Book." (4)
//!   Radiant: "Deal {damage} damage. / End of turn: Become a different Radiant Book." (8)
//!   Engine:  "One targeted hit, as C #16 Book of Flame. The designer's second 'Book of Flame' is
//!            renamed so that the two names never collide and no text that names Book of Flame (C #23
//!            Devil's Pact, C #29 Book of Vital Kill) finds it (R381). At the end of its owner's turn,
//!            while in their hand, it is Replaced by a random other non-token Book of any set (not
//!            Wildfire), on the same face, which keeps swapping each turn (R671). Tunes: damage 4 ↑."
//!
//! "Deal N damage" with no target named is targeted (§8 Conventions, as #68 Twisted Sorcerer's is): one
//! declared pick (R81) of any Unit on top of its pile or either hero, either side, and one §4.4 damage
//! instance from this Spell, so Spell Damage raises it (§4.4 step 0) and Armor and Divine Shield meet
//! it. A Unit Immune to Spells is never offered (R81, E35). With no Unit, the heroes remain, so the
//! Spell always has a target. The number is the declared one, `param(ctx, "damage")` (R386). It is a
//! Book (its tag), which C #4 Palantir's base face answers; it names no card and no card names it.
//!
//! The swap (Patch v0.2.X, #271, R671) is the engine's one Book-swap hand trigger (`BOOK_SWAP_TRIGGER`):
//! on its owner's `turnEnded`, while this is in their hand, it becomes a Book drawn with the match rng
//! from §5.1's pool — every non-token Book (R380), Book of Flame included, but neither this card nor
//! Wildfire — in the same place in the hand and on the same face, so a Radiant Wildfire becomes the
//! Radiant face of that Book. The Book it becomes has that Book's own text and carries the swap as the
//! `swapsBook` enchantment, so it keeps changing every end of its owner's turn while it stays in hand.

use jackioh_engine::effects::damage;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-055";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))]
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": param(ctx, "damage") })))]
        })),
        hand_triggers: vec![book_swap_trigger()],
        ..Script::default()
    };

    // The same script: the Radiant face differs in its declared damage (8), read through `param`, and in
    // the face of the Book it becomes, which the swap reads off the card.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C #55 Book of Wildfire — SPEC §8.6 row 55, BUILD M9 Classic row C 55: "`classic-055`, a Book: one
// targeted hit of 4 on any Unit or hero, either side; nothing names it, so C #23 and C #29 never make
// it (R381); C #4's base face answers it as a Book; radiant 8; its tuned number (damage) reads through
// `param()` (R386)".
//
// The hit is one §4.4 damage instance from the Spell, so Divine Shield, Armor and Spell Damage meet it
// as they meet any Spell's. C #4 Palantir's base face answers it as a Book (its steal prompt), and
// C #29's Radiant face makes a Book of Flame, never this card (R381).
//
// Patch v0.2.X (#271, R671): at the end of its owner's turn, while it is in their hand, it becomes a
// different Book — every non-token Book but Wildfire, Book of Flame included, drawn with the match rng
// — on its own face, in its place in the hand; the Book it becomes has that Book's own text and keeps
// the swap (the `swapsBook` enchantment), so it changes again at each end of its owner's turn.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;
    use std::collections::BTreeSet;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const WILDFIRE: &str = "classic-055";
    const VANILLA: &str = "core-008"; // 4/4, no text
    const MENACE: &str = "core-019"; // 9/9 Taunt
    const DEFENDER: &str = "core-003"; // 1/1 Taunt, Divine Shield, Reborn
    const TOP_LOSER: &str = "classicplus-019-1"; // Radiant: Immune to Spells
    const SOLARIUS: &str = "classicplus-038"; // Spell Damage +2
    const FILLER: &str = "core-005"; // a hand card, so a turn never auto-ends (§2.5)
    const PALANTIR: &str = "classic-004"; // Base: when your opponent plays a Book, you may Tribute this to steal it.
    const VITAL_KILL: &str = "classic-029"; // Radiant: … Add a Book of Flame to your hand.
    const BOOK_OF_FLAME: &str = "classic-016";

    /// An engine value as the JSON the TS test reads (SURFACE §5.1: the same keys and values).
    fn js<T: serde::Serialize + ?Sized>(value: &T) -> Value {
        serde_json::to_value(value).expect("an engine value serialises")
    }

    /// TS `JSON.parse(JSON.stringify(state))`: written and read back field by field, in field order.
    fn round_trip(state: &GameState) -> GameState {
        let text = serde_json::to_string(state).expect("the state serialises");
        serde_json::from_str(&text).expect("the state parses back")
    }

    /// TS `toMatchObject`: every key the pattern names matches, recursively; arrays element by element.
    fn matches_object(actual: &Value, pattern: &Value) -> bool {
        match (actual, pattern) {
            (Value::Object(actual), Value::Object(pattern)) => pattern
                .iter()
                .all(|(key, expected)| actual.get(key).is_some_and(|value| matches_object(value, expected))),
            (Value::Array(actual), Value::Array(pattern)) => {
                actual.len() == pattern.len()
                    && actual.iter().zip(pattern).all(|(value, expected)| matches_object(value, expected))
            }
            _ => actual == pattern,
        }
    }

    fn hero(player: &str) -> Value {
        json!({ "pick": "hero", "player": player })
    }

    fn at(s: &Scenario, card: &str) -> Value {
        json!({ "pick": "instance", "instanceId": s.card(card).id })
    }

    /// The target lists `legalActions` offers for p1's Wildfire, each as its JSON (TS `targets ?? []`).
    fn wildfire_plays(s: &Scenario, player: PlayerId) -> Vec<Value> {
        let id = s.card(WILDFIRE).id.clone();
        legal_actions(s.state(), player)
            .iter()
            .map(js)
            .filter(|action| action["type"] == "play" && action["instanceId"] == id.as_str())
            .map(|action| if action["targets"].is_null() { json!([]) } else { action["targets"].clone() })
            .collect()
    }

    /// Every non-token Book of every set but Wildfire: the swap's pool (R380, R671).
    fn other_books() -> Vec<String> {
        jackioh_engine::catalog::query(&json_as(json!({ "tags": ["Book"] })))
            .iter()
            .map(|book| book.id.clone())
            .filter(|id| id != WILDFIRE)
            .collect()
    }

    /// p1 holds this card (Radiant or not) and a filler, and p2 a filler, so no turn ends on its own.
    fn holding(seed: &str, radiant: bool) -> Scenario {
        scenario(json!({ "seed": seed, "p1": { "hand": [{ "def": WILDFIRE, "radiant": radiant }, FILLER] }, "p2": { "hand": [FILLER] } }))
    }

    /// p1's first hand card: where the Wildfire sits, and where what it became sits (R671).
    fn first_card(s: &Scenario) -> CardInstance {
        s.hand(P1).into_iter().next().expect("p1's hand is empty")
    }

    /// The Book p1's Wildfire is after p1's end of turn, from a game on `seed`.
    fn swapped_on(seed: &str, radiant: bool) -> CardInstance {
        let mut s = holding(seed, radiant);
        s.end_turn();
        first_card(&s)
    }

    fn hits_on(s: &Scenario, target_id: &str) -> Vec<Value> {
        s.events()
            .iter()
            .map(js)
            .filter(|event| event["type"] == "damage" && event["targetId"] == target_id)
            .map(|event| event["amount"].clone())
            .collect()
    }

    mod c_55_book_of_wildfire {
        use super::*;

        #[test]
        fn is_a_1_spell_with_the_book_tag_declaring_one_target_its_damage_a_declared_number_on_both_faces() {
            crate::register_all();
            let def = js(&crate::card_def(ID));
            assert_eq!(def["id"], WILDFIRE);
            assert_eq!(def["type"], "Spell");
            assert_eq!(def["cost"], 1);
            assert_eq!(def["tags"], json!(["Book"]));
            assert_eq!(
                def["params"],
                json!([{ "key": "damage", "base": 4, "radiant": 8, "better": "up", "step": 1, "min": 1 }]),
            );
            let scripts = script();
            assert_eq!(
                js(&scripts.base.targets),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "hero"] } }]),
            );
            // TS `expect(radiant).toBe(base)`: the Radiant face is the same declaration and the same hooks.
            assert_eq!(js(&scripts.radiant.targets), js(&scripts.base.targets));
            assert_eq!(scripts.radiant.cry.is_some(), scripts.base.cry.is_some());
            assert_eq!(
                scripts.radiant.hand_triggers.iter().map(|trigger| trigger.id.clone()).collect::<Vec<String>>(),
                scripts.base.hand_triggers.iter().map(|trigger| trigger.id.clone()).collect::<Vec<String>>(),
            );
            // TS `toEqual([BOOK_SWAP_TRIGGER])`: the one engine trigger, by its id and the events it answers.
            let swap = book_swap_trigger();
            assert_eq!(scripts.base.hand_triggers.len(), 1);
            assert_eq!(scripts.base.hand_triggers[0].id, swap.id);
            assert_eq!(scripts.base.hand_triggers[0].id, BOOK_SWAP_TRIGGER_ID);
            assert_eq!(scripts.base.hand_triggers[0].on, swap.on);
        }

        #[test]
        fn r671_prints_the_swap_on_both_faces_the_radiant_one_naming_a_radiant_book() {
            crate::register_all();
            let def = js(&crate::card_def(ID));
            assert_eq!(def["base"]["text"], "Deal {damage} damage.\nEnd of turn: Become a different Book.");
            assert_eq!(def["radiant"]["text"], "Deal {damage} damage.\nEnd of turn: Become a different Radiant Book.");
            assert!(def.get("refs").is_none());
        }

        #[test]
        fn r381_nothing_names_it_c_23_and_c_29_name_book_of_flame_c_16_never_book_of_wildfire() {
            crate::register_all();
            assert_eq!(js(&crate::card_def("classic-023"))["refs"], json!(["classic-016"]));
            assert_eq!(js(&crate::card_def("classic-029"))["refs"], json!(["classic-016"]));
            assert_eq!(crate::card_def("classic-016").name, "Book of Flame");
            assert_eq!(crate::card_def(ID).name, "Book of Wildfire");
        }

        mod base {
            use super::*;

            #[test]
            fn deals_4_damage_to_a_target_enemy_unit() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [WILDFIRE, FILLER] }, "p2": { "field": [MENACE] } }));
                let target = at(&s, MENACE);
                s.play(WILDFIRE, json!({ "targets": [target] }));
                s.expect_stats(MENACE, json!({ "health": 5 }));
                s.expect_in_zone(WILDFIRE, "graveyard");
            }

            #[test]
            fn deals_4_damage_to_the_enemy_hero_or_to_your_own_hero_any_target_either_side() {
                crate::register_all();
                let mut enemy = scenario(json!({ "p1": { "hand": [WILDFIRE, FILLER] } }));
                enemy.play(WILDFIRE, json!({ "targets": [hero("p2")] })).expect_health(P2, 26);
                let mut own = scenario(json!({ "p1": { "hand": [WILDFIRE, FILLER] } }));
                own.play(WILDFIRE, json!({ "targets": [hero("p1")] })).expect_health(P1, 26);
            }

            #[test]
            fn may_hit_one_of_your_own_units_and_kills_a_4_health_one() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [WILDFIRE, FILLER], "field": [VANILLA] } }));
                let target = at(&s, VANILLA);
                s.play(WILDFIRE, json!({ "targets": [target] }));
                s.expect_in_zone(VANILLA, "graveyard");
            }

            #[test]
            fn legalactions_offers_every_unit_and_both_heroes_one_target_each_and_a_play_with_none_is_refused() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [WILDFIRE, FILLER], "field": [VANILLA] }, "p2": { "field": [MENACE] } }));
                let offered = wildfire_plays(&s, P1);
                assert_eq!(offered.len(), 4);
                assert!(offered.iter().all(|targets| targets.as_array().is_some_and(|list| list.len() == 1)));
                assert!(offered.contains(&json!([hero("p1")])));
                assert!(offered.contains(&json!([hero("p2")])));
                s.expect_refused_with(|s| s.play(WILDFIRE, json!({})), "target");
                s.expect_refused_with(|s| s.play(WILDFIRE, json!({ "targets": [hero("p1"), hero("p2")] })), "at most");
            }

            #[test]
            fn e35_a_unit_immune_to_spells_is_never_offered_and_a_play_naming_it_is_refused() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WILDFIRE, FILLER] },
                    "p2": { "field": [{ "def": TOP_LOSER, "radiant": true }] },
                }));
                let loser = s.card(TOP_LOSER).id.clone();
                assert!(!wildfire_plays(&s, P1).contains(&json!([{ "pick": "instance", "instanceId": loser }])));
                s.expect_refused_with(
                    |s| s.play(WILDFIRE, json!({ "targets": [{ "pick": "instance", "instanceId": loser }] })),
                    "not a legal target",
                );
            }

            #[test]
            fn s4_4_step_1_divine_shield_takes_the_whole_hit_and_the_unit_stays() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [WILDFIRE, FILLER] }, "p2": { "field": [DEFENDER] } }));
                let target = at(&s, DEFENDER);
                s.play(WILDFIRE, json!({ "targets": [target] }));
                s.expect_in_zone(DEFENDER, "field").expect_stats(DEFENDER, json!({ "health": 1 }));
                s.expect_events(json!(["divineShieldLost"]));
            }

            #[test]
            fn s4_4_step_2_armor_reduces_it_a_unit_in_defense_position_takes_3() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [WILDFIRE, FILLER] }, "p2": { "field": [{ "def": MENACE, "position": "DEF" }] } }));
                let target = at(&s, MENACE);
                s.play(WILDFIRE, json!({ "targets": [target] }));
                s.expect_stats(MENACE, json!({ "health": 6 }));
            }

            #[test]
            fn s4_4_step_0_spell_damage_on_your_side_raises_the_hit_solariuss_2_makes_it_6() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [WILDFIRE, FILLER], "field": [SOLARIUS] } }));
                s.play(WILDFIRE, json!({ "targets": [hero("p2")] }));
                assert_eq!(hits_on(&s, "hero-p2"), vec![json!(6)]);
            }

            #[test]
            fn is_a_book_play_the_game_counts_its_book_tag_for_its_player_what_c_4_palantir_answers() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [WILDFIRE, FILLER] } }));
                s.play(WILDFIRE, json!({ "targets": [hero("p2")] }));
                assert_eq!(played_this_game_with_tag(s.state(), P1, Tag::Book), 1);
            }

            #[test]
            fn c_4_palantirs_base_face_answers_it_as_a_book_tributed_at_once_with_no_prompt_and_steals_this() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [WILDFIRE, FILLER] }, "p2": { "backrow": [PALANTIR], "hand": [FILLER] } }));
                let wildfire = s.card(WILDFIRE).clone();
                s.play(&wildfire, json!({ "targets": [hero("p2")] }));
                assert!(s.state().pending.is_none());
                s.expect_in_zone(PALANTIR, "graveyard");
                assert!(matches_object(
                    &js(s.card(&wildfire.id)),
                    &json!({ "owner": "p2", "zone": { "z": "hand", "player": "p2" } }),
                ));
                s.expect_health(P2, 30);
            }

            #[test]
            fn r381_c_29s_radiant_face_adds_a_book_of_flame_never_this_card() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": VITAL_KILL, "radiant": true }, FILLER] } }));
                s.play(VITAL_KILL, json!({ "targets": [hero("p2")] }));
                let added: Vec<String> =
                    s.hand(P1).into_iter().map(|card| card.def_id).filter(|def_id| def_id != FILLER).collect();
                assert_eq!(added, vec![BOOK_OF_FLAME]);
            }

            #[test]
            fn r386_its_damage_is_the_declared_number_an_upgrades_step_makes_it_5_a_degrades_3() {
                crate::register_all();
                let mut up = scenario(json!({ "p1": { "hand": [WILDFIRE, FILLER] } }));
                step_param(up.card_mut(WILDFIRE), "damage", 1);
                up.play(WILDFIRE, json!({ "targets": [hero("p2")] })).expect_health(P2, 25);
                let mut down = scenario(json!({ "p1": { "hand": [WILDFIRE, FILLER] } }));
                step_param(down.card_mut(WILDFIRE), "damage", -1);
                down.play(WILDFIRE, json!({ "targets": [hero("p2")] })).expect_health(P2, 27);
                let graveyard = js(&down.view(P1).you.graveyard);
                let params = graveyard
                    .as_array()
                    .and_then(|cards| cards.iter().find(|card| card["defId"] == WILDFIRE))
                    .map(|card| card["params"].clone());
                assert_eq!(params, Some(json!({ "damage": 3 })));
            }

            #[test]
            fn r97_in_its_owners_hand_the_opponents_view_never_names_it() {
                crate::register_all();
                let s = scenario(json!({ "p1": { "hand": [WILDFIRE, FILLER] } }));
                let theirs = js(&s.view(P2));
                assert_eq!(theirs["opponent"]["hand"], json!({ "count": 2 }));
                assert!(!theirs.to_string().contains(WILDFIRE));
                let own = js(&s.view(P1));
                assert!(own["you"]["hand"].as_array().is_some_and(|hand| hand.iter().any(|card| card["defId"] == WILDFIRE)));
            }

            #[test]
            fn its_play_replays_exactly_from_the_log_a_json_round_trip_of_the_state_plays_the_same() {
                crate::register_all();
                let s = scenario(json!({ "p1": { "hand": [WILDFIRE, FILLER] }, "p2": { "field": [MENACE] } }));
                let round = round_trip(s.state());
                let action: Action = json_as(json!({
                    "type": "play",
                    "playerId": "p1",
                    "nonce": "c55-json",
                    "instanceId": s.card(WILDFIRE).id,
                    "targets": [at(&s, MENACE)],
                }));
                let a = reduce(s.state(), &action);
                let b = reduce(&round, &action);
                assert!(a.error.is_none());
                assert_eq!(b.state, a.state);
            }
        }

        mod base_the_swap_r671 {
            use super::*;

            #[test]
            fn r671_at_the_end_of_its_owners_turn_in_hand_it_becomes_a_different_book_in_its_place_in_the_hand() {
                crate::register_all();
                let mut s = holding("swap-1", false);
                let wildfire = s.card(WILDFIRE).clone();
                s.end_turn();
                let book = first_card(&s);
                assert!(!s.hand(P1).iter().any(|card| card.id == wildfire.id));
                assert!(other_books().contains(&book.def_id));
                assert!(!book.radiant);
                assert_eq!(js(&book.enchantments), json!([{ "kind": "swapsBook", "from": WILDFIRE }]));
                assert_eq!(s.hand(P1).get(1).map(|card| card.def_id.clone()), Some(FILLER.to_string()));
                let swapped = s.events().iter().map(js).find(|event| event["type"] == "transformed");
                assert!(swapped.is_some_and(|event| matches_object(
                    &event,
                    &json!({ "instanceId": wildfire.id, "fromDefId": WILDFIRE, "toDefId": book.def_id, "hiddenFrom": ["p2"] }),
                )));
            }

            #[test]
            fn r671_the_pool_is_every_other_book_never_wildfire_book_of_flame_among_them_the_pick_the_match_rngs() {
                crate::register_all();
                let books = other_books();
                let mut seen: BTreeSet<String> = BTreeSet::new();
                for n in 0..60 {
                    seen.insert(swapped_on(&format!("pool-{n}"), false).def_id);
                }
                assert!(!seen.contains(WILDFIRE));
                assert!(seen.iter().all(|id| books.contains(id)));
                assert!(seen.contains(BOOK_OF_FLAME));
                assert!(seen.len() > 5);
                // The same game picks the same Book.
                assert_eq!(swapped_on("pool-7", false).def_id, swapped_on("pool-7", false).def_id);
            }

            #[test]
            fn r671_not_at_the_end_of_the_opponents_turn_in_the_other_players_hand_it_stays_wildfire() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [FILLER] }, "p2": { "hand": [WILDFIRE, FILLER] } }));
                s.end_turn();
                assert!(s.hand(P2).iter().any(|card| card.def_id == WILDFIRE));
                s.expect_in_zone(WILDFIRE, "hand");
                assert!(!s.events().iter().map(js).any(|event| event["type"] == "transformed"));
            }

            #[test]
            fn r671_only_in_hand_in_the_library_or_the_graveyard_it_stays_wildfire() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "library": [FILLER, WILDFIRE], "graveyard": [WILDFIRE] },
                    "p2": { "hand": [FILLER] },
                }));
                s.end_turn();
                assert!(s.pile(P1, "library").iter().any(|card| card.def_id == WILDFIRE));
                assert_eq!(
                    s.pile(P1, "graveyard").into_iter().map(|card| card.def_id).collect::<Vec<String>>(),
                    vec![WILDFIRE],
                );
                assert!(!s.events().iter().map(js).any(|event| event["type"] == "transformed"));
            }

            #[test]
            fn r671_the_book_it_became_keeps_swapping_unchanged_on_the_opponents_turn_a_different_book_at_its_owners_next() {
                crate::register_all();
                let mut s = holding("swap-2", false);
                s.end_turn();
                let first = first_card(&s);
                s.end_turn(); // p2's turn ends: p1's Book is not theirs to swap.
                assert_eq!(first_card(&s).id, first.id);
                s.end_turn();
                let second = first_card(&s);
                assert_ne!(second.id, first.id);
                assert_ne!(second.def_id, first.def_id);
                assert!(other_books().contains(&second.def_id));
                assert_eq!(js(&second.enchantments), json!([{ "kind": "swapsBook", "from": WILDFIRE }]));
            }

            #[test]
            fn r671_the_book_it_became_has_that_books_own_text_book_of_flame_deals_its_4_to_a_target() {
                crate::register_all();
                let mut seed = 0;
                while seed < 200 && swapped_on(&format!("flame-{seed}"), false).def_id != BOOK_OF_FLAME {
                    seed += 1;
                }
                let mut s = holding(&format!("flame-{seed}"), false);
                s.end_turn().end_turn();
                let flame = first_card(&s);
                assert_eq!(flame.def_id, BOOK_OF_FLAME);
                let before = hero_of(s.state(), P2).health;
                s.play(&flame, json!({ "targets": [hero("p2")] })).expect_health(P2, before - 4);
            }

            #[test]
            fn r671_its_owners_view_shows_the_book_it_became_with_the_swap_riding_it_the_opponents_names_neither() {
                crate::register_all();
                let mut s = holding("swap-3", false);
                s.end_turn();
                let book = first_card(&s);
                let own = js(&s.view(P1));
                let shown = own["you"]["hand"]
                    .as_array()
                    .and_then(|hand| hand.iter().find(|card| card["instanceId"] == book.id.as_str()).cloned());
                assert!(shown.is_some_and(|card| matches_object(
                    &card,
                    &json!({ "defId": book.def_id, "enchantments": [{ "kind": "swapsBook", "from": WILDFIRE }] }),
                )));
                let theirs = js(&s.view(P2)).to_string();
                assert!(!theirs.contains(&book.def_id));
                assert!(!theirs.contains("swapsBook"));
            }

            #[test]
            fn r671_r637_a_temporary_wildfire_becomes_a_temporary_book_which_cleanup_still_discards() {
                crate::register_all();
                let mut s = holding("swap-4", false);
                let id = s.card(WILDFIRE).id.clone();
                find_instance_mut(s.state_mut(), &id)
                    .expect("the Wildfire is in p1's hand")
                    .granted_keywords
                    .push(json_as(json!({ "kind": "Temporary" })));
                s.end_turn();
                assert!(!s.hand(P1).iter().any(|card| card.def_id == WILDFIRE));
                let discarded = s.pile(P1, "graveyard");
                assert_eq!(discarded.len(), 1);
                assert!(discarded.first().is_some_and(|card| other_books().contains(&card.def_id)));
            }

            #[test]
            fn r671_its_end_of_turn_replays_exactly_a_json_round_trip_of_the_state_swaps_to_the_same_book() {
                crate::register_all();
                let s = holding("swap-5", false);
                let round = round_trip(s.state());
                let action: Action = json_as(json!({ "type": "endTurn", "playerId": "p1", "nonce": "c55-swap" }));
                let a = reduce(s.state(), &action);
                let b = reduce(&round, &action);
                assert!(a.error.is_none());
                assert_eq!(hash_state(&b.state), hash_state(&a.state));
                assert_eq!(b.state, a.state);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn deals_8_damage_to_a_target() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": WILDFIRE, "radiant": true }, FILLER] }, "p2": { "field": [MENACE] } }));
                let target = at(&s, MENACE);
                s.play(WILDFIRE, json!({ "targets": [target] }));
                s.expect_stats(MENACE, json!({ "health": 1 }));
            }

            #[test]
            fn deals_8_to_a_hero_either_side() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": WILDFIRE, "radiant": true }, FILLER] } }));
                s.play(WILDFIRE, json!({ "targets": [hero("p1")] })).expect_health(P1, 22);
            }

            #[test]
            fn r386_its_declared_damage_steps_from_8_an_upgrade_makes_it_9_enough_to_kill_the_9_9() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": WILDFIRE, "radiant": true }, FILLER] }, "p2": { "field": [MENACE] } }));
                step_param(s.card_mut(WILDFIRE), "damage", 1);
                let target = at(&s, MENACE);
                s.play(WILDFIRE, json!({ "targets": [target] }));
                s.expect_in_zone(MENACE, "graveyard");
            }

            #[test]
            fn s4_4_the_spell_damage_of_your_side_raises_the_radiant_hit_too_8_2() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": WILDFIRE, "radiant": true }, FILLER], "field": [SOLARIUS] } }));
                s.play(WILDFIRE, json!({ "targets": [hero("p2")] }));
                assert_eq!(hits_on(&s, "hero-p2"), vec![json!(10)]);
            }

            #[test]
            fn r671_at_the_end_of_its_owners_turn_it_becomes_the_radiant_face_of_a_different_book_which_keeps_swapping_radiant() {
                crate::register_all();
                let mut s = holding("radiant-swap-1", true);
                s.end_turn();
                let first = first_card(&s);
                assert!(other_books().contains(&first.def_id));
                assert!(first.radiant);
                assert_eq!(js(&first.enchantments), json!([{ "kind": "swapsBook", "from": WILDFIRE }]));
                s.end_turn().end_turn();
                let second = first_card(&s);
                assert_ne!(second.def_id, first.def_id);
                assert!(second.radiant);
            }

            #[test]
            fn r671_every_radiant_pick_is_a_radiant_book_other_than_wildfire() {
                crate::register_all();
                let books = other_books();
                for n in 0..20 {
                    let book = swapped_on(&format!("radiant-pool-{n}"), true);
                    assert!(books.contains(&book.def_id));
                    assert!(book.radiant);
                }
            }

            #[test]
            fn r671_not_at_the_opponents_end_of_turn_nor_outside_the_hand() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "library": [FILLER, { "def": WILDFIRE, "radiant": true }] },
                    "p2": { "hand": [{ "def": WILDFIRE, "radiant": true }, FILLER] },
                }));
                s.end_turn();
                assert!(!s.events().iter().map(js).any(|event| event["type"] == "transformed"));
            }

            #[test]
            fn r671_its_swap_replays_exactly_from_a_json_round_trip() {
                crate::register_all();
                let s = holding("radiant-swap-2", true);
                let round = round_trip(s.state());
                let action: Action = json_as(json!({ "type": "endTurn", "playerId": "p1", "nonce": "c55-swap" }));
                assert_eq!(reduce(&round, &action).state, reduce(s.state(), &action).state);
            }
        }
    }
}
