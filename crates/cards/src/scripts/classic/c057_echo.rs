//! C #57 Echo (SPEC §8.6 row 57, BUILD M9 Classic row C 57). (1) Spell, Epic.
//!   Base:    "This has the text of the last Spell either player played."
//!   Radiant: "Echo 1 / This has the text of the last Spell either player played."
//!   Engine:  "The per-game record of the last Spell played by anyone (§10.1), `lastSpell = { defId,
//!            radiant }`, overwritten by every Spell play and cast (R70) and never cleared; a countered
//!            Spell was never played and is not recorded. In hand, Echo's view carries that Spell's text
//!            on the face it was played on, under Echo's name (as R243 carries a fused card's); played,
//!            Echo declares and resolves that Spell's choices and script. Echo keeps its own name, type
//!            and (1) Cost. A played Echo records the Spell it copied, never Echo, so it can't copy itself
//!            into a loop; with no Spell played yet it has no text and does nothing (R399). … Radiant:
//!            plus Echo 1 (§6.2), which repeats the copied text. Tunes: Radiant Echo 1 ↑, a numbered
//!            keyword that Degrade and Upgrade move as an X (R386), not a `params` entry."
//!
//! The card is a flag and a record; the text it has is the engine's B5 E14 subsystem
//! (`engine/src/subsystems/copiedText.ts`), with its own engine tests through fixture cards:
//!   - `copiesLastSpell` makes the running text the last Spell's face (R399): its declared targets,
//!     modes and X (R545: chosen up to the mana left once Echo's own (1) is paid), its resolution with
//!     Echo as "this", its prompt continuations and its declared numbers (`param`), its Echo X, its
//!     Cast on draw, its `preview` and `conditionMet` (R546, R547), and the owner's hand view
//!     (`CardView.copies`).
//!   - The copy is fixed as the play begins and kept through the play (R546); in hand and in a deck it
//!     follows the record live.
//!   - `recordsPlayAs` (B5 E4) records the Spell Echo copied, on its face, and nothing when it copied
//!     nothing, so two Echoes never loop (R399).
//!   - The Radiant face's "Echo 1" is §6.2's Echo X (`staticFlags.echo`), a numbered keyword Degrade and
//!     Upgrade move (R386), added to any Echo the copied face prints (R546). Its number is a keyword's,
//!     not a declared `params` entry, so nothing here reads `param`.
//!
//! "Echo" is also a rules word: the "Echo 1" printed on other cards is never a reference to this card
//! (R381, `test/references.test.ts`).

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-057";

/// B5 E4, R399: the Spell Echo copied, on its face, or nothing when it copied nothing.
fn records_play_as() -> RecordsPlayAsHook {
    read_hook(|args| subsystems::copied_text_of(args.state, args.self_))
}

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags { copies_last_spell: Some(true), ..StaticFlags::default() }),
        records_play_as: Some(records_play_as()),
        ..Script::default()
    };

    // §6.2: Echo 1 — the copied text resolves once more, with fresh prompts.
    let radiant = Script {
        static_flags: Some(StaticFlags { copies_last_spell: Some(true), echo: Some(1), ..StaticFlags::default() }),
        records_play_as: Some(records_play_as()),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #57 Echo — SPEC §8.6 row 57, R399, BUILD M9 Classic row C 57: "Reads the game-wide last Spell played
// by either player (`lastSpell = { defId, radiant }`), casts included, a countered Spell never counted
// (it was never played) and a Field Spell never a Spell; in its owner's hand its view carries that
// Spell's face under Echo's name (as R243 carries a fused card's), hidden from the opponent like any hand
// card; played, it declares and resolves that Spell's choices and script on the face recorded; a played
// Echo records the Spell it copied, never Echo, so two Echoes never loop (R399); with no Spell played
// yet it has no text and resolves to nothing; radiant: also Echo 1, one more resolution with fresh
// prompts (§6.2); its name is a rules word, so "Echo 1" in #51 and #79 is no reference to it (R381); its
// tuned number (radiant Echo) reads through `param()` (R386)" — a numbered keyword SPEC's row says
// Degrade and Upgrade move as an X, not a `params` entry, so its R386 proof steps that X.
//
// The text is B5 E14 (`engine/src/subsystems/copiedText.ts`, its engine tests in
// `engine/test/copied-text.test.ts`), with this workstream's rulings R545 (an X-cost text), R546 (the
// copy fixed as the play begins; "this" is Echo) and R547 (its static text: Cast on draw yes, the
// end-of-turn return no).
#[cfg(test)]
mod tests {
    use super::{script, ID};
    use jackioh_engine::testkit::*;

    const ECHO: &str = "classic-057";
    const WILDFIRE: &str = "classic-055"; // (1) Spell: Deal {damage} damage (4, Radiant 8), a declared target.
    const GRAND: &str = "classic-072"; // Trap: counters the opponent's non-Unit plays.
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const RAPID: &str = "core-010"; // (0) Spell: Combo 3: Draw 3.
    const ARMOR: &str = "core-073"; // (2) Field Spell
    const ADAPTIVE: &str = "core-074"; // (X) Spell: Deal X to a target; heal X; draw X; an X/X Ghoul.
    const REMINISCE: &str = "core-072"; // (1) Spell: Discover a card from your GY. It costs (1) less. Exile this.
    const DREAM: &str = "core-023"; // (1) Spell: … End of turn: Return this to your hand.
    const CN_VIRUS: &str = "core-090-1"; // (1) Spell, Cast on draw: take 1 damage.
    const MENACE: &str = "core-019"; // (3) Unit 9/9
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const PALANTIR: &str = "classic-004"; // Base: when your opponent plays a Book, Tribute this to steal it (mandatory, no prompt).
    const PILE_ON: &str = "classic-060"; // (5) Spell: Recruit every permanent in your deck; to the bottom of the deck, not the GY.
    /// Pile On with C+ #39 Book Worm fused in, as R179 names it: a Spell, its first ingredient's type.
    const FUSED_PILE_ON: &str = "t-1:classic-060+classicplus-039";

    /// TS `AT_P2: Selection[]`, as the JSON the harness takes.
    fn at_p2() -> Value {
        json!([{ "pick": "hero", "player": "p2" }])
    }

    use crate::js;

    /// TS `JSON.parse(JSON.stringify(state))`: written and read back field by field, in field order.
    fn round_trip(state: &GameState) -> GameState {
        let text = serde_json::to_string(state).expect("the state serialises");
        serde_json::from_str(&text).expect("the state parses back")
    }

    fn hits(events: &[GameEvent], target: &str) -> Vec<i64> {
        events
            .iter()
            .map(js)
            .filter(|event| event["type"] == "damage" && event["targetId"] == target)
            .filter_map(|event| event["amount"].as_i64())
            .collect()
    }

    /// The legal `play` actions of the Echo in hand, as JSON (TS's `Extract<…, { type: "play" }>`).
    fn echo_plays(s: &Scenario, player: PlayerId) -> Vec<Value> {
        let id = s.card(ECHO).id.clone();
        legal_actions(s.state(), player)
            .iter()
            .map(js)
            .filter(|action| action["type"] == "play" && action["instanceId"] == id.as_str())
            .collect()
    }

    fn own_view(s: &Scenario, player: PlayerId) -> Option<Value> {
        let view = js(&s.view(player));
        view["you"]["hand"]
            .as_array()
            .and_then(|hand| hand.iter().find(|card| card["defId"] == ECHO).cloned())
    }

    mod c_57_echo {
        use super::*;

        #[test]
        fn is_a_1_spell_that_copies_the_last_spells_text_its_radiant_face_adds_echo_1_a_numbered_keyword() {
            crate::register_all();
            let def = js(&registered_catalog()[ID]);
            assert_eq!(def["type"], "Spell");
            assert_eq!(def["cost"], 1);
            assert!(def["params"].is_null());
            let scripts = script();
            assert_eq!(js(&scripts.base.static_flags), json!({ "copiesLastSpell": true }));
            assert_eq!(js(&scripts.radiant.static_flags), json!({ "copiesLastSpell": true, "echo": 1 }));
            assert!(scripts.base.cry.is_none());
        }

        #[test]
        fn r381_its_name_is_a_rules_word_echo_1_in_51_and_79_is_no_reference_to_it() {
            crate::register_all();
            let core_051 = js(&registered_catalog()["core-051"]);
            assert!(core_051["radiant"]["text"].as_str().is_some_and(|text| text.contains("Echo 1")));
            assert!(!core_051["refs"].as_array().is_some_and(|refs| refs.iter().any(|id| *id == ECHO)));
            let core_079 = js(&registered_catalog()["core-079"]);
            assert!(!core_079["refs"].as_array().is_some_and(|refs| refs.iter().any(|id| *id == ECHO)));
        }

        mod base {
            use super::*;

            #[test]
            fn r399_with_no_spell_played_yet_it_has_no_text_no_choices_it_resolves_to_nothing_and_records_nothing() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [ECHO, VANILLA] } }));
                let card = own_view(&s, PlayerId::P1).expect("Echo in p1's hand view");
                assert!(card.get("copies").is_none());
                // TS `[undefined]`: one play, with no `targets` key (an absent key reads as null).
                assert_eq!(
                    echo_plays(&s, PlayerId::P1).iter().map(|play| play["targets"].clone()).collect::<Vec<Value>>(),
                    vec![Value::Null],
                );
                s.play(ECHO, json!({}));
                assert!(s
                    .last_events()
                    .iter()
                    .map(js)
                    .filter(|event| event["type"] == "damage" || event["type"] == "drawn")
                    .collect::<Vec<Value>>()
                    .is_empty());
                s.expect_in_zone(ECHO, "graveyard");
                s.expect_mana(PlayerId::P1, 3);
                assert!(last_spell_played(s.state()).is_none());
            }

            #[test]
            fn r399_it_has_the_text_of_the_last_spell_either_player_played_the_opponents_spell() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [ECHO, VANILLA], "library": [MENACE, MENACE, MENACE, MENACE] },
                    "p2": { "hand": [STOCKPILE, VANILLA], "library": [VANILLA, VANILLA] },
                    "active": "p2",
                }));
                s.play(STOCKPILE, json!({}));
                s.end_turn();
                assert_eq!(
                    js(&subsystems::copied_text_of(s.state(), s.card(ECHO))),
                    json!({ "defId": STOCKPILE, "radiant": false }),
                );
                s.play(ECHO, json!({}));
                // Stockpile's text: draw 2, heal your hero 2 — for Echo's player.
                assert_eq!(
                    s.last_events()
                        .iter()
                        .map(js)
                        .filter(|event| event["type"] == "drawn" && event["player"] == "p1")
                        .count(),
                    2,
                );
            }

            #[test]
            fn r399_r70_a_cast_is_a_play_a_spell_cast_on_draw_is_the_last_spell() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [STOCKPILE, ECHO], "library": [CN_VIRUS, VANILLA, VANILLA] } }));
                s.play(STOCKPILE, json!({}));
                assert_eq!(js(&last_spell_played(s.state())), json!({ "defId": CN_VIRUS, "radiant": false }));
                assert_eq!(
                    own_view(&s, PlayerId::P1).map(|card| card["copies"].clone()),
                    Some(json!({ "defId": CN_VIRUS, "radiant": false })),
                );
            }

            #[test]
            fn r399_a_countered_spell_was_never_played_and_is_never_the_last_spell() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WILDFIRE, ECHO], "backrow": [GRAND] },
                    "p2": { "hand": [STOCKPILE, VANILLA] },
                }));
                s.play(WILDFIRE, json!({ "targets": at_p2() }));
                s.end_turn();
                s.play(STOCKPILE, json!({}));
                assert!(s.last_events().iter().map(js).any(|event| event["type"] == "countered"));
                assert_eq!(js(&last_spell_played(s.state())), json!({ "defId": WILDFIRE, "radiant": false }));
            }

            #[test]
            fn r399_a_field_spell_is_never_a_spell_playing_one_leaves_the_record_as_it_was() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [WILDFIRE, ARMOR, ECHO], "library": [VANILLA], "mana": 9 } }));
                s.play(WILDFIRE, json!({ "targets": at_p2() }));
                s.play(ARMOR, json!({}));
                assert_eq!(js(&last_spell_played(s.state())), json!({ "defId": WILDFIRE, "radiant": false }));
            }

            #[test]
            fn r399_r243_in_its_owners_hand_its_view_carries_that_spells_face_and_numbers_under_echos_name_and_cost() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": WILDFIRE, "radiant": true }, ECHO] } }));
                s.play(WILDFIRE, json!({ "targets": at_p2() }));
                let card = own_view(&s, PlayerId::P1).expect("Echo in p1's hand view");
                assert_eq!(card["defId"], ECHO);
                assert_eq!(card["cost"], 1);
                assert_eq!(card["copies"]["defId"], WILDFIRE);
                assert_eq!(card["copies"]["radiant"], true);
                assert_eq!(card["copies"]["params"]["damage"], 8);
            }

            #[test]
            fn r97_hidden_from_the_opponent_like_any_hand_card_their_view_never_names_it_nor_what_it_copies() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [WILDFIRE, ECHO] } }));
                s.play(WILDFIRE, json!({ "targets": at_p2() }));
                let theirs = js(&s.view(PlayerId::P2));
                assert_eq!(theirs["opponent"]["hand"], json!({ "count": 1 }));
                assert!(!theirs.to_string().contains(ECHO));
                assert!(!theirs.to_string().contains("copies"));
            }

            #[test]
            fn r399_played_it_declares_the_copied_spells_choices_legalactions_offers_exactly_the_copied_spells_targets() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [WILDFIRE, ECHO, WILDFIRE] }, "p2": { "field": [MENACE] } }));
                let wildfires: Vec<String> =
                    s.hand(PlayerId::P1).iter().filter(|card| card.def_id == WILDFIRE).map(|card| card.id.clone()).collect();
                let (Some(first), Some(second)) = (wildfires.first().cloned(), wildfires.get(1).cloned()) else {
                    panic!("setup");
                };
                s.play(&first, json!({ "targets": at_p2() }));
                let wildfire_targets: Vec<Value> = legal_actions(s.state(), PlayerId::P1)
                    .iter()
                    .map(js)
                    .filter(|action| action["type"] == "play" && action["instanceId"] == second.as_str())
                    .map(|action| action["targets"].clone())
                    .collect();
                assert_eq!(
                    echo_plays(&s, PlayerId::P1).iter().map(|play| play["targets"].clone()).collect::<Vec<Value>>(),
                    wildfire_targets,
                );
                s.expect_refused_with(
                    |s| s.play(ECHO, json!({})),
                    "target",
                );
            }

            #[test]
            fn r399_r386_it_resolves_the_copied_script_on_the_face_recorded_with_that_faces_declared_number() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": WILDFIRE, "radiant": true }, ECHO] }, "p2": { "field": [MENACE] } }));
                s.play(WILDFIRE, json!({ "targets": at_p2() }));
                let menace = s.card(MENACE).id.clone();
                s.play(ECHO, json!({ "targets": [{ "pick": "instance", "instanceId": menace }] }));
                s.expect_stats(MENACE, json!({ "health": 1 }));
                s.expect_in_zone(ECHO, "graveyard");
                s.expect_mana(PlayerId::P1, 2);
            }

            #[test]
            fn r399_a_played_echo_records_the_spell_it_copied_never_itself_so_a_second_echo_copies_the_same_spell() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [WILDFIRE, ECHO, ECHO, VANILLA], "mana": 9 }, "p2": { "hand": [VANILLA] } }));
                let echoes: Vec<String> =
                    s.hand(PlayerId::P1).iter().filter(|card| card.def_id == ECHO).map(|card| card.id.clone()).collect();
                let (Some(one), Some(two)) = (echoes.first().cloned(), echoes.get(1).cloned()) else {
                    panic!("setup");
                };
                s.play(WILDFIRE, json!({ "targets": at_p2() }));
                s.play(&one, json!({ "targets": at_p2() }));
                assert_eq!(js(&last_spell_played(s.state())), json!({ "defId": WILDFIRE, "radiant": false }));
                s.play(&two, json!({ "targets": at_p2() }));
                assert_eq!(hits(s.events(), "hero-p2"), vec![4, 4, 4]);
                assert_eq!(js(&last_spell_played(s.state())), json!({ "defId": WILDFIRE, "radiant": false }));
            }

            #[test]
            fn r546_it_keeps_its_own_name_cost_type_and_tags_copying_a_book_makes_no_book_play_and_c_4_palantir_does_not_ask() {
                crate::register_all();
                // p2's own Book: Palantir answers only the opponent's Books, so it resolves.
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [ECHO, VANILLA] },
                    "p2": { "hand": [WILDFIRE, VANILLA], "backrow": [PALANTIR] },
                }));
                s.play(WILDFIRE, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
                assert!(s.state().pending.is_none());
                s.expect_in_zone(PALANTIR, "field");
                let card = own_view(&s, PlayerId::P1).expect("Echo in p1's hand view");
                assert_eq!(card["defId"], ECHO);
                assert_eq!(card["cost"], 1);
                assert_eq!(card["copies"]["defId"], WILDFIRE);
                s.end_turn();
                // p1's Echo copies the resolved Book: the copy is no Book play, so Palantir does not answer.
                s.play(ECHO, json!({ "targets": at_p2() }));
                assert!(s.state().pending.is_none());
                s.expect_in_zone(PALANTIR, "field");
                let played = s
                    .last_events()
                    .iter()
                    .map(js)
                    .find(|event| event["type"] == "cardPlayed")
                    .expect("a cardPlayed event");
                assert_eq!(played["defId"], ECHO);
                assert_eq!(played["costPaid"], 1);
                assert_eq!(hits(s.events(), "hero-p2"), vec![4]);
                assert_eq!(played_this_game_with_tag(s.state(), PlayerId::P1, Tag::Book), 0);
            }

            #[test]
            fn r545_an_x_cost_text_x_is_chosen_with_the_play_from_1_up_to_the_mana_left_once_echos_1_is_paid() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [ADAPTIVE, ECHO], "library": [VANILLA, VANILLA, VANILLA], "mana": 9 } }));
                s.play(ADAPTIVE, json!({ "x": 1, "targets": at_p2() }));
                s.state_mut().players.p1.mana.current = 4;
                // TS `[...new Set(…)]`: the X values, first-seen order, each once.
                let mut xs: Vec<Value> = Vec::new();
                for play in echo_plays(&s, PlayerId::P1) {
                    if !xs.contains(&play["x"]) {
                        xs.push(play["x"].clone());
                    }
                }
                assert_eq!(xs, vec![json!(1), json!(2), json!(3)]);
                s.expect_refused_with(
                    |s| s.play(ECHO, json!({ "x": 4, "targets": at_p2() })),
                    "X is above",
                );
                s.play(ECHO, json!({ "x": 3, "targets": at_p2() }));
                assert_eq!(hits(s.last_events(), "hero-p2"), vec![3]);
                s.expect_mana(PlayerId::P1, 3);
                // The first Adaptive UI's 1/1 Ghoul holds lane 1; Echo's 3/3 lands beside it (R64).
                let ghost = s.unit(PlayerId::P1, 2).map(|unit| unit.id.clone());
                let stats = ghost.map(|id| s.stats(&id)).expect("a unit in p1's lane 2");
                assert_eq!(stats.attack, 3);
                assert_eq!(stats.health, 3);
            }

            #[test]
            fn r545_with_nothing_left_once_echos_1_is_paid_an_x_cost_text_cannot_be_played_absent_and_refused() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [ADAPTIVE, ECHO], "library": [VANILLA, VANILLA, VANILLA], "mana": 9 } }));
                s.play(ADAPTIVE, json!({ "x": 1, "targets": at_p2() }));
                s.state_mut().players.p1.mana.current = 1;
                assert!(echo_plays(&s, PlayerId::P1).is_empty());
                s.expect_refused_with(
                    |s| s.play(ECHO, json!({ "x": 1, "targets": at_p2() })),
                    "X is above",
                );
                s.expect_in_zone(ECHO, "hand");
            }

            #[test]
            fn r546_a_prompt_in_the_copied_text_continues_the_copied_script_this_is_echo_exiled_where_reminisce_says_so() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [REMINISCE, ECHO], "graveyard": [MENACE, VANILLA] } }));
                s.play(REMINISCE, json!({}));
                s.answer(json!(MENACE));
                s.play(ECHO, json!({}));
                let pending = js(&s.state().pending);
                assert_eq!(pending["resume"]["defId"], REMINISCE);
                let round = round_trip(s.state());
                let vanilla = s.pile(PlayerId::P1, "graveyard").iter().find(|card| card.def_id == VANILLA).map(|card| card.id.clone());
                let option = pending["options"].as_array().and_then(|options| {
                    options
                        .iter()
                        .find(|entry| entry["selection"]["pick"] == "instance" && entry["selection"]["instanceId"] == json!(vanilla))
                        .cloned()
                });
                let action: Action = json_as(json!({
                    "type": "answer",
                    "playerId": "p1",
                    "nonce": "c57-round-trip",
                    "choiceId": pending["id"].as_str().unwrap_or(""),
                    "selection": option.map_or(json!([]), |entry| json!([entry["selection"]])),
                }));
                let live = reduce(s.state(), &action);
                let revived = reduce(&round, &action);
                assert!(live.error.is_none());
                assert_eq!(revived.state, live.state);
                let hand = &live.state.players.p1.hand;
                assert_eq!(hand.iter().find(|card| Some(&card.id) == vanilla.as_ref()).map(|card| card.cost_mod), Some(-1));
                // "Exile this": the Echo, not a Reminisce, goes to exile.
                assert_eq!(
                    live.state.players.p1.exile.iter().map(|card| card.def_id.as_str()).collect::<Vec<&str>>(),
                    vec![REMINISCE, ECHO],
                );
            }

            #[test]
            fn r547_an_echo_drawn_while_the_last_spell_casts_on_draw_is_cast_and_the_draw_repeats() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [STOCKPILE, VANILLA], "library": [CN_VIRUS, ECHO, MENACE, VANILLA] } }));
                s.play(STOCKPILE, json!({}));
                // CN-Virus is cast (1 damage), the draw repeats into Echo, which copies it and is cast too (1).
                assert_eq!(hits(s.last_events(), "hero-p1"), vec![1, 1]);
                s.expect_in_zone(ECHO, "graveyard");
                assert!(s.last_events().iter().map(js).any(|event| event["type"] == "cardPlayed" && event["defId"] == ECHO));
            }

            #[test]
            fn r547_the_copied_end_of_turn_return_is_not_had_an_echo_copying_reoccurring_dream_stays_in_the_graveyard() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [DREAM, ECHO, VANILLA] }, "p2": { "hand": [VANILLA] } }));
                s.play(DREAM, json!({}));
                s.play(ECHO, json!({}));
                s.end_turn();
                s.expect_in_zone(DREAM, "hand");
                s.expect_in_zone(ECHO, "graveyard");
            }

            #[test]
            fn r547_the_copied_faces_yellow_glow_answers_for_echo_in_hand_and_its_combo_resolves_for_echos_play() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [RAPID, RAPID, RAPID, ECHO], "library": [MENACE, MENACE, MENACE, MENACE] } }));
                let rapids: Vec<String> =
                    s.hand(PlayerId::P1).iter().filter(|held| held.def_id == RAPID).map(|held| held.id.clone()).collect();
                for card in &rapids {
                    s.play(card, json!({}));
                }
                assert_eq!(own_view(&s, PlayerId::P1).map(|card| card["conditionActive"].clone()), Some(json!(true)));
                s.play(ECHO, json!({}));
                assert_eq!(s.last_events().iter().map(js).filter(|event| event["type"] == "drawn").count(), 3);
            }

            #[test]
            fn r399_a_fused_pile_on_back_at_the_bottom_of_its_deck_is_still_the_last_spell_and_echo_has_its_text() {
                crate::register_all();
                // Part 40's sweep (`sweep:easy:core-076:2`): Pile On with Book Worm fused in (R77, R179) goes
                // back to the bottom of its deck by Pile On's own clause, where neither player sees it. The
                // last Spell names it, and its definition stays the state's (`transient_defs`), so Echo is
                // priced, listed and resolves its text.
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [ECHO, VANILLA], "library": [STOCKPILE, MENACE, VANILLA, STOCKPILE] },
                    "p2": { "hand": [PILE_ON], "field": [VANILLA], "library": [STOCKPILE, STOCKPILE] },
                }));
                let fused = subsystems::rebuild_fused_def(s.state_mut(), FUSED_PILE_ON, PlayerId::P2)
                    .expect("Pile On and Book Worm fuse");
                assert_eq!(fused.type_, CardType::Spell);
                let pile_on = s.card(PILE_ON).id.clone();
                s.card_mut(&pile_on).def_id = FUSED_PILE_ON.to_string();
                s.play(&pile_on, json!({}));
                s.end_turn();
                assert_eq!(s.pile(PlayerId::P2, "library").last().map(|card| card.id.clone()), Some(pile_on));
                assert_eq!(js(&last_spell_played(s.state())), json!({ "defId": FUSED_PILE_ON, "radiant": false }));
                assert!(s.state().transient_defs.contains_key(FUSED_PILE_ON));

                assert_eq!(
                    echo_plays(&s, PlayerId::P1).iter().map(|play| (play["targets"].clone(), play["x"].clone())).collect::<Vec<_>>(),
                    vec![(Value::Null, Value::Null)],
                );
                assert_eq!(
                    own_view(&s, PlayerId::P1).map(|card| card["copies"]["defId"].clone()),
                    Some(json!(FUSED_PILE_ON)),
                );
                let permanents = s.pile(PlayerId::P1, "library").iter().filter(|card| card.def_id != STOCKPILE).count();
                assert_eq!(permanents, 2, "p1 drew its top Stockpile; a Menace and a Vanilla are left to Recruit");
                s.play(ECHO, json!({}));
                s.expect_mana(PlayerId::P1, 3);
                // Pile On's text, for Echo's player: every permanent in p1's deck is Recruited.
                assert!(s.pile(PlayerId::P1, "library").iter().all(|card| card.def_id == STOCKPILE));
                let units = (1..=5).filter(|lane| s.unit(PlayerId::P1, *lane).is_some()).count();
                assert_eq!(units, permanents);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn s6_2_echo_1_the_copied_text_resolves_once_more_asking_a_fresh_pick_for_the_repeat() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WILDFIRE, { "def": ECHO, "radiant": true }, VANILLA] },
                    "p2": { "field": [MENACE], "hand": [VANILLA] },
                }));
                s.play(WILDFIRE, json!({ "targets": at_p2() }));
                s.play(ECHO, json!({ "targets": at_p2() }));
                assert!(js(&s.state().pending)["prompt"].as_str().is_some_and(|prompt| prompt.contains("Echo")));
                let menace = s.card(MENACE).id.clone();
                s.answer(json!([{ "pick": "instance", "instanceId": menace }]));
                assert_eq!(hits(s.events(), "hero-p2"), vec![4, 4]);
                s.expect_stats(MENACE, json!({ "health": 5 }));
            }

            #[test]
            fn r399_a_radiant_echo_after_a_played_echo_copies_the_spell_that_echo_copied_never_echo_echo_1_repeats_that_text() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WILDFIRE, ECHO, { "def": ECHO, "radiant": true }, VANILLA], "mana": 9 },
                    "p2": { "hand": [VANILLA] },
                }));
                let plain = s.hand(PlayerId::P1).iter().find(|card| card.def_id == ECHO && !card.radiant).map(|card| card.id.clone());
                let shining = s.hand(PlayerId::P1).iter().find(|card| card.def_id == ECHO && card.radiant).map(|card| card.id.clone());
                let (Some(plain), Some(shining)) = (plain, shining) else {
                    panic!("setup");
                };
                s.play(WILDFIRE, json!({ "targets": at_p2() }));
                s.play(&plain, json!({ "targets": at_p2() }));
                assert_eq!(
                    js(&subsystems::copied_text_of(s.state(), s.card(&shining))),
                    json!({ "defId": WILDFIRE, "radiant": false }),
                );
                s.play(&shining, json!({ "targets": at_p2() }));
                s.answer(at_p2());
                assert_eq!(hits(s.events(), "hero-p2"), vec![4, 4, 4, 4]);
            }

            #[test]
            fn r386_its_echo_is_a_numbered_keyword_upgrade_moves_as_an_x_one_step_more_is_one_more_repeat() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [WILDFIRE, { "def": ECHO, "radiant": true }, VANILLA] }, "p2": { "hand": [VANILLA] } }));
                let id = s.card(ECHO).id.clone();
                let echo = find_instance_mut(s.state_mut(), &id).expect("the Echo is in p1's hand");
                let tuning = tuning_of(echo);
                tuning.x = Some(add_step(tuning.x.as_ref(), "Echo", 1));
                s.play(WILDFIRE, json!({ "targets": at_p2() }));
                s.play(ECHO, json!({ "targets": at_p2() }));
                s.answer(at_p2());
                s.answer(at_p2());
                assert_eq!(hits(s.events(), "hero-p2"), vec![4, 4, 4, 4]);
            }

            #[test]
            fn r399_with_nothing_to_copy_it_does_nothing_and_its_echo_repeats_nothing() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": ECHO, "radiant": true }, VANILLA] } }));
                s.play(ECHO, json!({}));
                assert!(s.state().pending.is_none());
                assert!(!s.last_events().iter().map(js).any(|event| event["type"] == "damage"));
                s.expect_in_zone(ECHO, "graveyard");
            }
        }
    }
}
