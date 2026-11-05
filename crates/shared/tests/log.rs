//! The game log's lines (`src/log/`, #3108): the events a view gives as readable sentences, as the web's
//! `Log.test.tsx` held them before the lines moved here, in both locales, through the one function behind the
//! `log` body (`apply`), as a host calls it: JSON in, JSON out. A line reads as it always has; these tests
//! are what says so, and that a line about a card the viewer may not read never names it (CLAUDE.md rule 7).

use jackioh_engine::catalog::registered_catalog;
use jackioh_engine::wire::PlayerView;
use jackioh_shared::face::data::{ZH_CN, find};
use jackioh_shared::face::{Locale, Span};
use jackioh_shared::log::{LogAnswer, LogCall, LogLine, apply};
use serde_json::{Value, json};

fn ready() {
    jackioh_cards::register_all();
}

fn name_of(def_id: &str) -> String {
    ready();
    registered_catalog()
        .get(def_id)
        .map_or_else(|| def_id.to_string(), |def| def.name.clone())
}

fn chinese_name_of(def_id: &str) -> String {
    ready();
    let call: jackioh_shared::face::FaceCall = serde_json::from_value(json!({
        "call": "card", "locale": "zh-CN", "source": { "defId": def_id, "radiant": false }
    }))
    .expect("a FaceCall");
    match jackioh_shared::face::apply(&call) {
        jackioh_shared::face::FaceAnswer::Face { face } => face.name,
        other => panic!("a face, got {other:?}"),
    }
}

// ---- a board to read the events on

fn side(player: &str, over: Value) -> Value {
    let mut side = json!({
        "player": player,
        "hero": { "health": 30, "armor": 0, "powers": [] },
        "modifiers": [],
        "mana": { "current": 4, "max": 4 },
        "hand": [],
        "libraryCount": 12,
        "graveyard": [], "exile": [], "resolving": [],
        "units": [null, null, null, null, null],
        "backrow": [null, null, null, null, null],
        "locks": { "units": [false, false, false, false, false], "backrow": [false, false, false, false, false] },
        "reserved": { "units": [false, false, false, false, false], "backrow": [false, false, false, false, false] },
        "fatigueCount": 0,
    });
    for (key, value) in over.as_object().expect("an object") {
        side[key] = value.clone();
    }
    side
}

fn unit(owner: &str, instance_id: &str, def_id: &str, over: Value) -> Value {
    let mut unit = json!({
        "instanceId": instance_id, "defId": def_id, "radiant": false, "cost": 3,
        "owner": owner, "controller": owner, "attack": 3, "maxHealth": 4, "health": 4,
        "keywords": [], "armor": 0, "position": "ATK", "counters": {}, "buried": 0, "canAct": true,
    });
    for (key, value) in over.as_object().expect("an object") {
        unit[key] = value.clone();
    }
    unit
}

/// A view from p1's seat with the given sides and events.
fn view(you: Value, opponent: Value, events: Value) -> PlayerView {
    ready();
    serde_json::from_value(json!({
        "viewer": "p1", "turn": 3, "active": "p1", "phase": "main",
        "you": you, "opponent": opponent,
        "pending": null, "events": events, "result": null, "clockMs": null,
    }))
    .expect("a PlayerView")
}

fn bare(events: Value) -> PlayerView {
    view(
        side("p1", json!({})),
        side("p2", json!({ "hand": { "count": 4 } })),
        events,
    )
}

/// Your Panther `u1` and the opponent's Vanilla `e1`.
fn board(events: Value) -> PlayerView {
    view(
        side(
            "p1",
            json!({ "units": [unit("p1", "u1", "core-032", json!({})), null, null, null, null] }),
        ),
        side(
            "p2",
            json!({ "hand": { "count": 4 }, "units": [unit("p2", "e1", "core-008", json!({})), null, null, null, null] }),
        ),
        events,
    )
}

fn call(view: &PlayerView, locale: Locale) -> Vec<Option<LogLine>> {
    match apply(&LogCall::View {
        view: Box::new(view.clone()),
        locale,
    }) {
        LogAnswer::Lines { lines } => lines,
        other => panic!("lines, got {other:?}"),
    }
}

/// The sentences of the view's events that have a line.
fn said(view: &PlayerView, locale: Locale) -> Vec<String> {
    call(view, locale)
        .into_iter()
        .flatten()
        .map(|line| line.text.full)
        .collect()
}

fn english(view: &PlayerView) -> Vec<String> {
    said(view, Locale::En)
}

fn cards(view: &PlayerView) -> Vec<(String, Option<String>)> {
    call(view, Locale::En)
        .into_iter()
        .flatten()
        .filter_map(|line| line.card)
        .map(|card| (card.def_id, card.instance_id))
        .collect()
}

// ---- redacted events

#[test]
fn a_hidden_card_that_was_buffed_is_not_given_a_size_and_a_public_buff_is_named_in_full() {
    let view = board(json!([
        { "type": "buffed", "instanceId": "hidden", "attack": 0, "health": 0 },
        { "type": "buffed", "instanceId": "u1", "attack": 2, "health": 1 },
    ]));
    let lines = english(&view);
    assert_eq!(lines[0], "A hidden card was buffed");
    assert_eq!(lines[1], format!("{} gained +2/+1", name_of("core-032")));
    assert!(!lines.join("\n").contains("+0/+0"));
}

#[test]
fn an_opponents_trap_the_viewer_may_not_read_fires_as_a_face_down_trap() {
    let view = bare(json!([
        { "type": "trapFired", "instanceId": "hidden", "defId": "hidden", "controller": "p2", "row": "backrow", "lane": 2 }
    ]));
    assert_eq!(english(&view), ["The opponent's face-down trap fired"]);
    assert!(cards(&view).is_empty(), "the sentinel opens no card");
}

#[test]
fn a_card_set_face_down_reads_as_a_trap_with_the_cost_its_back_shows_while_it_stands_there() {
    let set = json!({ "type": "summoned", "player": "p2", "instanceId": "hidden", "defId": "hidden", "row": "backrow", "lane": 3 });
    let with_cost = view(
        side("p1", json!({})),
        side(
            "p2",
            json!({ "hand": { "count": 4 }, "backrow": [null, null, { "faceDown": true, "cost": 2 }, null, null] }),
        ),
        json!([set]),
    );
    assert_eq!(
        english(&with_cost),
        ["The opponent's backrow lane 3: a face-down trap was set, (2) Cost"]
    );
    // Gone from the zone (it fired), or a back with no cost: the line claims none.
    let gone = english(&bare(json!([set])));
    assert_eq!(gone, ["The opponent's backrow lane 3: a face-down trap was set"]);
    assert!(!gone.join("\n").contains("hidden"));
}

#[test]
fn a_prompt_names_what_it_asks_for_and_a_number_a_cards_text_changed_names_its_key_in_words() {
    let view = board(json!([
        { "type": "promptOpened", "player": "p2", "choiceId": "c1", "kind": "pick" },
        { "type": "numberChanged", "instanceId": "u1", "defId": "core-032", "key": "drawLimit", "value": 2 },
        { "type": "numberChanged", "instanceId": "u1", "defId": "core-032", "key": "cost", "value": 1 },
        { "type": "promptOpened", "player": "p2", "choiceId": "c2", "kind": "market" },
        { "type": "promptOpened", "player": "p2", "choiceId": "c3", "kind": "craft" },
        { "type": "promptOpened", "player": "p2" },
    ]));
    let text = english(&view).join("\n");
    assert!(text.contains("must choose cards to take"));
    assert!(text.contains("draw limit became 2"));
    assert!(text.contains("now costs (1)"));
    assert!(text.contains("must choose a deal at the night market"));
    assert!(text.contains("Opponent must choose a card to craft"));
    // The other seat's prompt says whose it is, not what it asks for, since its event names no kind.
    assert!(english(&view).contains(&"Opponent must choose".to_string()));
    assert!(!text.contains("(pick)") && !text.contains("drawLimit") && !text.contains("(market)"));
}

#[test]
fn a_unit_that_has_left_the_board_is_named_from_the_windows_own_public_events_and_an_id_is_never_printed() {
    let view = bare(json!([
        { "type": "summoned", "player": "p2", "instanceId": "c46", "defId": "core-002", "row": "units", "lane": 1 },
        { "type": "attackDeclared", "attackerId": "c46", "targetId": "hero-p1", "forced": false },
        { "type": "damage", "sourceId": "c46", "targetId": "c46", "amount": 12, "combat": true },
        { "type": "attackDeclared", "attackerId": "c99", "targetId": "c98", "forced": false },
    ]));
    let name = name_of("core-002");
    let lines = english(&view);
    assert_eq!(lines[1], format!("{name} attacked your hero"));
    assert_eq!(lines[2], format!("{name} took 12 damage in combat"));
    // Nothing public ever named c99 or c98: they read as units, not as ids.
    assert_eq!(lines[3], "A unit attacked a unit");
    assert!(!lines.join("\n").contains("c46") && !lines.join("\n").contains("c99"));
    // The line about a card says which, and a line that names none opens none.
    assert_eq!(
        cards(&view),
        [
            ("core-002".to_string(), Some("c46".to_string())),
            ("core-002".to_string(), Some("c46".to_string())),
            ("core-002".to_string(), Some("c46".to_string())),
        ]
    );
}

#[test]
fn mana_and_summons_are_in_words_a_modifier_prints_its_label_and_finished_resolving_is_left_out() {
    let mut you = side("p1", json!({}));
    you["modifiers"] = json!([{ "id": "m1", "label": "Next Spell gains Echo +1" }]);
    let view = view(
        you,
        side("p2", json!({ "hand": { "count": 4 } })),
        json!([
            { "type": "manaChanged", "player": "p1", "current": 2, "max": 2 },
            { "type": "summoned", "player": "p1", "instanceId": "x1", "defId": "core-015", "row": "units", "lane": 1 },
            { "type": "cardResolved", "player": "p1", "instanceId": "x1", "defId": "core-015", "permanent": true, "costPaid": 1 },
            { "type": "modifierChanged", "player": "p1", "modifierId": "m1", "added": true },
            { "type": "modifierChanged", "player": "p2", "modifierId": "m-gone", "added": false },
        ]),
    );
    let lines = english(&view);
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], "You have 2/2 mana");
    assert_eq!(lines[1], format!("{} entered your lane 1", name_of("core-015")));
    assert_eq!(lines[2], "You gained \"Next Spell gains Echo +1\"");
    assert_eq!(lines[3], "Opponent lost an effect");
    // The log keeps the event's place: a bookkeeping event has no line, the others are in order.
    let all = call(&view, Locale::En);
    assert_eq!(all.len(), 5);
    assert!(all[2].is_none());
}

#[test]
fn a_curve_reads_as_drawn_across_the_board_and_as_the_equation_its_cells_make() {
    let view = bare(json!([
        { "type": "curveDrawn", "player": "p1", "cells": [{ "player": "p2", "row": "units", "lane": 3 }] },
        { "type": "curveDrawn", "player": "p2", "cells": [] },
        { "type": "curveDrawn", "player": "p1", "cells": [
            { "player": "p1", "row": "backrow", "lane": 1 },
            { "player": "p2", "row": "units", "lane": 2 },
            { "player": "p2", "row": "backrow", "lane": 3 },
            { "player": "p2", "row": "backrow", "lane": 4 },
            { "player": "p2", "row": "units", "lane": 5 },
        ] },
        { "type": "curveDrawn", "player": "p2", "cells": [
            { "player": "p2", "row": "backrow", "lane": 1 },
            { "player": "p1", "row": "units", "lane": 2 },
            { "player": "p1", "row": "backrow", "lane": 3 },
            { "player": "p1", "row": "backrow", "lane": 4 },
            { "player": "p1", "row": "units", "lane": 5 },
        ] },
        { "type": "curveDrawn", "player": "p1", "cells": [
            { "player": "p1", "row": "units", "lane": 2 },
            { "player": "p2", "row": "units", "lane": 2 },
        ] },
    ]));
    assert_eq!(
        english(&view),
        [
            "You drew the curve y = 2 across the board",
            "Opponent drew a curve across the board",
            "You drew the curve y = −¹⁄₂x² + ⁵⁄₂x across the board",
            "Opponent drew the curve y = −¹⁄₂x² + ⁵⁄₂x across the board",
            "You drew a curve across the board",
        ]
    );
}

#[test]
fn a_starscape_travel_reads_as_the_node_its_players_map_reaches() {
    let view = bare(json!([
        { "type": "starscapeTravelled", "player": "p1", "node": 3, "reward": "heal" },
        { "type": "starscapeTravelled", "player": "p2", "node": 20, "reward": "edge" },
    ]));
    assert_eq!(
        english(&view),
        [
            "Your starscape map reaches node 3",
            "The opponent's starscape map reaches node 20"
        ]
    );
}

#[test]
fn the_game_ends_in_words_and_a_kept_secret_names_no_choice_until_it_is_revealed() {
    let ended = bare(json!([{ "type": "gameOver", "winner": "p2", "reason": "concede" }]));
    assert_eq!(english(&ended), ["Opponent won. You conceded."]);
    assert_eq!(
        english(&bare(
            json!([{ "type": "gameOver", "winner": "draw", "reason": "turn-cap" }])
        )),
        ["Draw. The turn limit was reached."]
    );
    // Two fleers draw; the other player's flight is a draw for the viewer (R1843004).
    assert_eq!(
        english(&bare(json!([
            { "type": "gameOver", "winner": "draw", "reason": "fled" },
            { "type": "gameOver", "winner": "p2", "reason": "fled" },
            { "type": "gameOver", "winner": "p1", "reason": "fled-decisive" },
        ]))),
        [
            "Draw. Both players escaped the universe.",
            "Opponent won. Your opponent escaped the universe. You draw.",
            "You won. You escaped the universe and left your opponent behind.",
        ]
    );
    let secrets = bare(json!([
        { "type": "secretChosen", "player": "p1", "secretId": "secret-1" },
        { "type": "secretRevealed", "player": "p1", "secretId": "secret-1", "choice": "greed" },
        { "type": "predicted", "player": "p2", "secretId": "secret-1", "guess": "attack", "outcome": "won" },
    ]));
    let lines = english(&secrets);
    assert_eq!(
        lines,
        [
            "You kept a secret",
            "Your secret was Greed",
            "Opponent guessed Attack: the reward is canceled"
        ]
    );
    assert!(!lines.join("\n").contains("greed"));
}

// ---- the card a line opens

#[test]
fn a_line_about_a_card_names_it_in_the_reader_and_says_which_card_it_opens() {
    let view = bare(json!([
        { "type": "cardPlayed", "player": "p2", "instanceId": "c7", "defId": "core-032", "costPaid": 2 }
    ]));
    let line = call(&view, Locale::En)
        .into_iter()
        .next()
        .flatten()
        .expect("a line");
    assert_eq!(line.event, "cardPlayed");
    assert_eq!(
        line.text.full,
        format!("Opponent played {} for 2", name_of("core-032"))
    );
    let card = line.card.expect("the card played");
    assert_eq!(
        (card.def_id.as_str(), card.radiant, card.instance_id.as_deref()),
        ("core-032", false, Some("c7"))
    );
}

#[test]
fn a_line_that_names_no_card_opens_none_the_sentinel_a_hero_a_draw_a_mana_change() {
    let view = bare(json!([
        { "type": "cardPlayed", "player": "p2", "instanceId": "hidden", "defId": "hidden", "costPaid": 1 },
        { "type": "summoned", "player": "p2", "instanceId": "hidden", "defId": "hidden", "row": "backrow", "lane": 3 },
        { "type": "damage", "sourceId": null, "targetId": "hero-p1", "amount": 3, "combat": false },
        // Even the viewer's own draw, whose card it may read: the log never names a draw.
        { "type": "drawn", "player": "p1", "instanceId": "c1", "defId": "core-011" },
        { "type": "manaChanged", "player": "p1", "current": 2, "max": 2 },
        { "type": "trapFired", "instanceId": "hidden", "defId": "hidden", "controller": "p2", "row": "backrow", "lane": 2 },
    ]));
    assert!(cards(&view).is_empty());
    let all = serde_json::to_string(&call(&view, Locale::En)).expect("JSON");
    assert!(!all.contains("core-0"), "{all}");
}

#[test]
fn a_line_about_a_radiant_card_opens_its_radiant_face_and_a_radiant_set_says_so() {
    let view = view(
        side(
            "p1",
            json!({ "units": [unit("p1", "u1", "core-011", json!({ "radiant": true })), null, null, null, null] }),
        ),
        side("p2", json!({ "hand": { "count": 4 } })),
        json!([
            { "type": "buffed", "instanceId": "u1", "attack": 1, "health": 1 },
            { "type": "radiantSet", "instanceId": "c5", "defId": "core-005", "zone": { "z": "graveyard", "player": "p1" } },
            { "type": "deradianted", "instanceId": "u1", "defId": "core-011", "zone": { "z": "field", "player": "p1", "row": "units", "lane": 1 } },
        ]),
    );
    let lines: Vec<LogLine> = call(&view, Locale::En).into_iter().flatten().collect();
    let radiant: Vec<bool> = lines
        .iter()
        .map(|line| line.card.as_ref().is_some_and(|card| card.radiant))
        .collect();
    assert_eq!(radiant, [true, true, false]);
    assert_eq!(
        lines[1].text.full,
        format!("{} became Radiant", name_of("core-005"))
    );
}

// ---- the reader's language

#[test]
fn a_reader_of_chinese_reads_the_lines_in_chinese_with_every_card_from_the_chinese_table() {
    let view = board(json!([
        { "type": "drawn", "player": "p1", "instanceId": "c1", "defId": "core-011" },
        { "type": "cardPlayed", "player": "p2", "instanceId": "c7", "defId": "core-032", "costPaid": 2 },
        { "type": "turnStarted", "player": "p1", "turn": 3 },
        { "type": "damage", "sourceId": null, "targetId": "hero-p1", "amount": 3, "combat": true },
    ]));
    let zh = said(&view, Locale::ZhCn);
    assert_eq!(zh[0], "你抽了一张牌");
    assert_eq!(zh[1], format!("对手打出了{}，花费2", chinese_name_of("core-032")));
    assert_eq!(zh[2], "第3回合：你");
    assert_eq!(zh[3], "你的英雄在战斗中受到了3点伤害");
}

#[test]
fn a_card_the_view_shows_in_chinese_is_named_in_chinese_whichever_field_the_event_names_it_by() {
    let chinese = chinese_name_of("meditative-023");
    let view = view(
        side(
            "p1",
            json!({ "units": [unit("p1", "u5", "meditative-023", json!({ "chinese": true })), null, null, null, null] }),
        ),
        side("p2", json!({ "hand": { "count": 4 } })),
        json!([
            { "type": "buffed", "instanceId": "u5", "attack": 2, "health": 2 },
            { "type": "upgraded", "instanceId": "u5", "defId": "meditative-023", "change": { "kind": "stats", "attack": 1, "health": 1 } },
            { "type": "destroyed", "instanceId": "u5", "defId": "meditative-023", "owner": "p1", "controller": "p1", "attack": 3, "maxHealth": 5, "killerId": null },
            // A translation gets no line of its own: the card's face says it.
            { "type": "translated", "instanceId": "u5" },
        ]),
    );
    assert_ne!(chinese, name_of("meditative-023"));
    assert_eq!(
        english(&view),
        [
            format!("{chinese} gained +2/+2"),
            format!("{chinese} was buffed"),
            format!("{chinese} was destroyed")
        ]
    );
}

// ---- the lines of the Meditative play-through (#552) and the later sets

#[test]
fn a_nerf_a_buff_and_a_changed_number_on_a_card_the_viewer_may_not_read_open_with_a_capital() {
    let view = bare(json!([
        { "type": "degraded", "instanceId": "hidden", "defId": "hidden", "change": { "kind": "none" } },
        { "type": "upgraded", "instanceId": "hidden", "defId": "hidden", "change": { "kind": "none" } },
        { "type": "numberChanged", "instanceId": "hidden", "defId": "hidden", "key": "hidden", "value": 0 },
        { "type": "crumbled", "instanceId": "hidden", "defId": "hidden", "owner": "p2", "zone": "field" },
    ]));
    assert_eq!(
        english(&view),
        [
            "A card was nerfed",
            "A card was buffed",
            "A card changed",
            "A card crumbled"
        ]
    );
}

#[test]
fn a_hidden_card_turned_into_another_hidden_card_says_only_that_it_was_transformed() {
    let view = bare(json!([
        { "type": "transformed", "instanceId": "hidden", "fromDefId": "hidden", "toDefId": "hidden", "newInstanceId": "hidden" },
        { "type": "transformed", "instanceId": "c1", "fromDefId": "meditative-037", "toDefId": "core-005", "newInstanceId": "c2" },
    ]));
    assert_eq!(
        english(&view),
        [
            "A hidden card was transformed".to_string(),
            format!("{} became {}", name_of("meditative-037"), name_of("core-005")),
        ]
    );
}

#[test]
fn a_line_that_opens_on_the_viewers_own_seat_opens_with_a_capital() {
    let view = bare(json!([
        { "type": "locked", "player": "p1", "row": "units", "lane": 2 },
        { "type": "unlocked", "player": "p1", "row": "backrow", "lane": 1 },
        { "type": "trapFired", "instanceId": "c1", "defId": "core-005", "controller": "p1", "row": "backrow", "lane": 1 },
        { "type": "healthSet", "player": "p1", "health": 10, "sourceId": null },
        { "type": "trapFired", "instanceId": "c1", "defId": "core-018", "controller": "p1", "row": "backrow", "lane": 1 },
        { "type": "trapFired", "instanceId": "c1", "defId": "core-018", "controller": "p1", "row": "backrow", "lane": 1, "again": true },
        { "type": "trapFired", "instanceId": "c9", "defId": "classic-005", "controller": "p2", "row": "backrow", "lane": 3, "again": true },
        { "type": "trapFired", "instanceId": "b41", "defId": "core-041", "controller": "p2", "row": "backrow", "lane": 2 },
    ]));
    assert_eq!(
        english(&view),
        [
            "Your lane 2 was locked".to_string(),
            "Your backrow lane 1 was unlocked".to_string(),
            format!("Your trap {} fired", name_of("core-005")),
            "Your hero's health was set to 10".to_string(),
            format!("Your trap {} fired", name_of("core-018")),
            format!("Your {} triggered", name_of("core-018")),
            format!("The opponent's {} triggered", name_of("classic-005")),
            format!("The opponent's trap {} fired", name_of("core-041")),
        ]
    );
}

#[test]
fn a_permanent_held_beneath_a_card_names_the_card_taken_and_the_card_it_is_held_beneath() {
    let view = view(
        side(
            "p1",
            json!({ "backrow": [{
                "faceDown": false, "instanceId": "b1", "defId": "core-020", "radiant": false, "cost": 1, "type": "Trap",
                "owner": "p1", "controller": "p1", "buried": 0, "counters": {}
            }, null, null, null, null] }),
        ),
        side(
            "p2",
            json!({ "hand": { "count": 4 }, "units": [unit("p2", "e1", "core-008", json!({})), null, null, null, null] }),
        ),
        json!([{ "type": "held", "player": "p2", "instanceId": "e1", "defId": "core-008", "holderId": "b1" }]),
    );
    assert_eq!(
        english(&view),
        [format!(
            "{} is held beneath {}",
            name_of("core-008"),
            name_of("core-020")
        )]
    );
}

#[test]
fn the_starship_fuse_line_counts_one_piece_as_a_piece_and_more_as_pieces() {
    let fused = |pieces: u32| {
        english(&bare(json!([
            { "type": "starshipFused", "player": "p1", "instanceId": "u1", "pieceId": "u2", "defId": "core-008", "pieces": pieces }
        ])))
        .remove(0)
    };
    assert!(fused(1).ends_with("Dormant Starship (1 piece)"));
    assert!(fused(3).ends_with("Dormant Starship (3 pieces)"));
}

#[test]
fn turn_lines_name_the_seat_and_an_extra_turn_says_so() {
    let view = bare(json!([
        { "type": "turnStarted", "player": "p1", "turn": 3 },
        { "type": "turnStarted", "player": "p1", "turn": 4, "extra": true },
        { "type": "turnEnded", "player": "p2", "turn": 4, "unspentMana": 2 },
        { "type": "turnAutoEnded", "player": "p2", "turn": 4 },
        { "type": "rolledBack", "player": "p1", "turnsAgo": 1, "sides": ["p1"] },
        { "type": "rolledBack", "player": "p1", "turnsAgo": 2, "sides": ["p1"] },
    ]));
    assert_eq!(
        english(&view),
        [
            "Turn 3: You",
            "Turn 4: You (extra turn)",
            "Opponent ended turn 4 with 2 mana unspent",
            "Opponent had no moves left, so turn 4 ended",
            "The board went back 1 turn",
            "The board went back 2 turns",
        ]
    );
}

// ---- #2252: a play names what it aimed at, and a hit off combat what dealt it

const SPELL: &str = "core-044";

fn announce(targets: Value, over: Value) -> Value {
    let mut event = json!({
        "type": "cardAnnounced", "player": "p2", "instanceId": "s1", "defId": SPELL, "cardType": "Spell",
        "costPaid": 1, "targets": targets,
    });
    for (key, value) in over.as_object().expect("an object") {
        event[key] = value.clone();
    }
    event
}

fn play(over: Value) -> Value {
    let mut event =
        json!({ "type": "cardPlayed", "player": "p2", "instanceId": "s1", "defId": SPELL, "costPaid": 1 });
    for (key, value) in over.as_object().expect("an object") {
        event[key] = value.clone();
    }
    event
}

fn hit(over: Value) -> Value {
    let mut event =
        json!({ "type": "damage", "sourceId": "s1", "targetId": "u1", "amount": 4, "combat": false });
    for (key, value) in over.as_object().expect("an object") {
        event[key] = value.clone();
    }
    event
}

#[test]
fn a_spell_played_at_an_enemy_unit_logs_one_play_line_naming_it_and_its_hit_names_the_spell() {
    let panther = name_of("core-032");
    let spell = name_of(SPELL);
    let view = board(json!([
        announce(json!(["u1"]), json!({})),
        play(json!({})),
        hit(json!({}))
    ]));
    assert_eq!(
        english(&view),
        [
            format!("Opponent played {spell} for 1, aiming at {panther}"),
            format!("{spell} dealt 4 damage to {panther}"),
        ]
    );
    assert_eq!(
        cards(&view)
            .into_iter()
            .map(|(def_id, _)| def_id)
            .collect::<Vec<_>>(),
        [SPELL, "core-032"]
    );
}

#[test]
fn a_play_names_each_target_the_viewer_may_read_a_hero_among_them_joined_as_the_readers_list() {
    let view = board(json!([
        announce(
            json!(["e1", "hero-p2", "hidden"]),
            json!({ "player": "p1", "instanceId": "s2" })
        ),
        play(json!({ "player": "p1", "instanceId": "s2" })),
    ]));
    let vanilla = name_of("core-008");
    assert_eq!(
        english(&view),
        [format!(
            "You played {} for 1, aiming at {vanilla}; the opponent's hero",
            name_of(SPELL)
        )]
    );
    let zh = said(&view, Locale::ZhCn);
    assert!(zh[0].contains('；'), "{zh:?}");
}

#[test]
fn a_face_down_set_a_play_with_no_targets_and_targets_the_view_hid_log_as_they_did() {
    let spell = name_of(SPELL);
    let view = board(json!([
        announce(
            json!(["hidden"]),
            json!({ "instanceId": "hidden", "defId": "hidden", "cardType": "Trap", "costPaid": 2, "row": "backrow", "lane": 3, "faceDown": true })
        ),
        play(json!({ "instanceId": "hidden", "defId": "hidden", "costPaid": 2 })),
        // The same id on both on purpose: `faceDown` alone has to stop the pairing.
        announce(
            json!(["e1"]),
            json!({ "player": "p1", "instanceId": "t1", "defId": "core-041", "cardType": "Trap", "row": "backrow", "lane": 1, "faceDown": true })
        ),
        play(json!({ "player": "p1", "instanceId": "t1", "defId": "core-041" })),
        announce(json!([]), json!({ "instanceId": "s3" })),
        play(json!({ "instanceId": "s3" })),
        announce(json!(["hidden"]), json!({ "instanceId": "s4" })),
        play(json!({ "instanceId": "s4" })),
        announce(json!(["c99"]), json!({ "instanceId": "s5" })),
        play(json!({ "instanceId": "s5" })),
    ]));
    assert_eq!(
        english(&view),
        [
            "Opponent played a hidden card for 2".to_string(),
            format!("You played {} for 1", name_of("core-041")),
            format!("Opponent played {spell} for 1"),
            format!("Opponent played {spell} for 1"),
            format!("Opponent played {spell} for 1"),
        ]
    );
}

#[test]
fn a_hit_names_its_source_off_combat_only_and_only_where_the_view_names_it() {
    let panther = name_of("core-032");
    let vanilla = name_of("core-008");
    let absorbed = |over: Value| {
        let mut event = json!({ "type": "damageAbsorbed", "sourceId": "e1", "targetId": "u1", "absorbed": 3, "combat": false });
        for (key, value) in over.as_object().expect("an object") {
            event[key] = value.clone();
        }
        event
    };
    let view = board(json!([
        hit(json!({ "sourceId": "e1", "amount": 3, "combat": true })),
        hit(json!({ "sourceId": "hidden", "amount": 2 })),
        hit(json!({ "sourceId": null, "targetId": "hero-p1", "amount": 3 })),
        hit(json!({ "sourceId": "c99", "amount": 1 })),
        hit(json!({ "sourceId": "e1", "amount": 1, "absorbed": 2 })),
        hit(json!({ "sourceId": "hero-p2", "targetId": "hero-p1", "amount": 2 })),
        absorbed(json!({})),
        absorbed(json!({ "combat": true })),
        absorbed(json!({ "sourceId": "hidden" })),
        hit(json!({ "sourceId": null, "amount": 2, "combat": true, "absorbed": 1 })),
    ]));
    assert_eq!(
        english(&view),
        [
            format!("{panther} took 3 damage in combat"),
            format!("{panther} took 2 damage"),
            "Your hero took 3 damage".to_string(),
            format!("{panther} took 1 damage"),
            format!("{vanilla} dealt 1 damage to {panther}, 2 absorbed by Armor"),
            "The opponent's hero dealt 2 damage to your hero".to_string(),
            format!("{panther}'s Armor absorbed 3 damage from {vanilla}"),
            format!("{panther}'s Armor absorbed 3 damage in combat"),
            format!("{panther}'s Armor absorbed 3 damage"),
            format!("{panther} took 2 damage in combat, 1 absorbed by Armor"),
        ]
    );
}

#[test]
fn each_play_pairs_with_its_own_announce_by_the_event_itself() {
    let spell = name_of(SPELL);
    let view = board(json!([
        announce(json!(["u1"]), json!({})),
        play(json!({})),
        announce(json!(["hero-p1"]), json!({})),
        play(json!({})),
    ]));
    assert_eq!(
        english(&view),
        [
            format!("Opponent played {spell} for 1, aiming at {}", name_of("core-032")),
            format!("Opponent played {spell} for 1, aiming at your hero"),
        ]
    );
    // A play whose announce is out of reach reads as before.
    assert_eq!(
        english(&board(json!([play(json!({})), hit(json!({}))]))),
        [
            format!("Opponent played {spell} for 1"),
            format!("{spell} dealt 4 damage to {}", name_of("core-032")),
        ]
    );
}

// ---- keywords, rolls and the rest

#[test]
fn a_keyword_granted_or_lost_is_a_glossary_term_in_the_line() {
    let view = board(json!([
        { "type": "keywordGranted", "instanceId": "u1", "keyword": { "kind": "Divine Shield" } },
        { "type": "keywordGranted", "instanceId": "u1", "keyword": { "kind": "Taunt" }, "lost": true },
    ]));
    let panther = name_of("core-032");
    let lines: Vec<LogLine> = call(&view, Locale::En).into_iter().flatten().collect();
    assert_eq!(lines[0].text.full, format!("{panther} gained Divine Shield"));
    assert_eq!(lines[1].text.full, format!("{panther} lost Taunt"));
    let term = lines[0].text.spans.iter().find_map(|span| match span {
        Span::Keyword { entry, children } => Some((entry.id.as_str(), children.clone())),
        _ => None,
    });
    assert_eq!(
        term,
        Some((
            "Divine Shield",
            vec![Span::Text {
                text: "Divine Shield".to_string()
            }]
        ))
    );
    // The spans say the same words as the line.
    for line in &lines {
        let flat: String = line
            .text
            .spans
            .iter()
            .map(|span| match span {
                Span::Text { text } => text.clone(),
                Span::Keyword { children, .. } => children
                    .iter()
                    .map(|child| match child {
                        Span::Text { text } => text.clone(),
                        _ => String::new(),
                    })
                    .collect(),
                _ => String::new(),
            })
            .collect();
        assert_eq!(flat, line.text.full);
    }
    // A reader of Chinese reads the term, and its entry, in Chinese.
    let zh: Vec<LogLine> = call(&view, Locale::ZhCn).into_iter().flatten().collect();
    assert!(zh[0].text.full.ends_with("获得了圣盾"), "{}", zh[0].text.full);
}

#[test]
fn a_call_to_chaos_names_its_effects_in_the_words_the_reveal_shows_even_when_the_card_is_hidden() {
    let view = bare(json!([
        { "type": "chaosRolled", "player": "p2", "instanceId": "c1", "defId": "core-095",
          "effects": ["Heal your hero 30", "Summon a Chaos Golem", "Cast a random Call to Chaos"] },
        { "type": "chaosRolled", "player": "p2", "instanceId": "hidden", "defId": "hidden", "effects": ["Cast a random Call to Chaos", "Something new"] },
    ]));
    assert_eq!(
        english(&view),
        [
            format!(
                "{} rolled: Heal the caster's hero 30; Summon a Chaos Golem; Cast a random Call to Chaos",
                name_of("core-095")
            ),
            "Call to Chaos rolled: Cast a random Call to Chaos; Something new".to_string(),
        ]
    );
}

#[test]
fn coins_and_dice_read_in_the_words_the_panel_lands_on() {
    let rolled = |kind: &str, sides: i32, rolls: Value, def_id: &str| {
        json!({ "type": "rolled", "player": "p1", "instanceId": if def_id == "hidden" { "hidden" } else { "c1" },
            "defId": def_id, "kind": kind, "sides": sides, "rolls": rolls })
    };
    let name = name_of("core-032");
    let view = bare(json!([
        rolled("coin", 2, json!([{ "faces": [1], "kept": 0 }]), "core-032"),
        rolled(
            "coin",
            2,
            json!([{ "faces": [1], "kept": 0 }, { "faces": [2, 1], "kept": 1 }, { "faces": [2], "kept": 0 }]),
            "core-032"
        ),
        rolled("die", 6, json!([{ "faces": [4], "kept": 0 }]), "core-032"),
        rolled(
            "die",
            6,
            json!([{ "faces": [4], "kept": 0 }, { "faces": [2, 5], "kept": 1 }]),
            "hidden"
        ),
    ]));
    assert_eq!(
        english(&view),
        [
            format!("{name} flipped a coin: heads"),
            format!("{name} flipped 3 coins: 2 heads, 1 tails (Lucky rolled 1 more)"),
            format!("{name} rolled a 6-sided die: 4"),
            "A hidden card rolled 2 6-sided dice: 4; 5 (Lucky rolled 1 more)".to_string(),
        ]
    );
}

#[test]
fn glitch_marks_and_the_rest_of_the_late_sets_read_as_one_sentence_each() {
    let view = bare(json!([
        { "type": "glitched", "player": "p1", "outcome": "swap" },
        { "type": "fengShui", "instanceId": "c1", "sourceId": "c2", "player": "p2", "outcome": "positive" },
        { "type": "marked", "instanceId": "hidden", "mark": "steal", "color": "purple", "added": true },
        { "type": "marked", "instanceId": "hidden", "mark": "steal", "color": "purple", "added": false },
        { "type": "marked", "instanceId": "hidden", "mark": "novel", "color": "purple", "added": true },
        { "type": "swapped", "what": "health" },
        { "type": "rotated", "direction": "left" },
        { "type": "redirected", "what": "damage", "fromId": "a", "toId": "hero-p1", "byInstanceId": null },
        { "type": "voteCast", "player": "p2", "instanceId": "hidden", "defId": "hidden", "votes": 1, "needed": 3 },
        { "type": "votedOut", "player": "p1", "ballots": [{ "instanceId": "hidden", "defId": "hidden" }] },
        { "type": "fled", "player": "p2", "opponent": "loss" },
        { "type": "marketBought", "player": "p2", "instanceId": "hidden", "defId": "hidden", "radiant": null, "credits": 4 },
        { "type": "stolen", "instanceId": "c1", "defId": "hidden", "from": "p1", "to": "p2", "zone": "library" },
        { "type": "libraryOverflow", "player": "p1", "instanceId": "hidden", "defId": "hidden", "outcome": "ceased" },
        { "type": "counterChanged", "instanceId": "hidden", "counter": "plague", "value": 2 },
        { "type": "emoted", "player": "p2", "emote": "thanks" },
    ]));
    let lines = english(&view);
    assert!(lines[0].ends_with(": the players swap seats"), "{}", lines[0]);
    assert_eq!(
        lines[1..],
        [
            "The opponent's play was auspicious (吉)",
            "A card was marked (Steal)",
            "A card was marked (Mark)",
            "Both players swapped health",
            "The board rotated left",
            "A hit was redirected to your hero",
            "A hidden card voted to remove your opponent (1 of 3)",
            "A hidden card agreed: you removed from the game",
            "Opponent escaped the universe, leaving the other side a loss",
            "Opponent bought a card for 4 Credits",
            "Opponent stole a card from your deck",
            "Your deck is full: a card ceased to exist",
            "A unit plague counters: 2",
            "Opponent emoted",
        ]
        .map(String::from)
    );
}

#[test]
fn a_gap_says_events_may_be_missing_in_the_readers_language() {
    let line = |locale| match apply(&LogCall::Gap { locale }) {
        LogAnswer::Line { line } => line,
        other => panic!("a line, got {other:?}"),
    };
    let english = line(Locale::En);
    assert_eq!(
        (english.event.as_str(), english.text.full.as_str()),
        ("gap", "Some events may be missing here")
    );
    assert!(english.card.is_none());
    assert_eq!(line(Locale::ZhCn).text.full, "此处可能缺少一些事件");
}

#[test]
fn the_answer_is_json_a_host_can_read_with_one_entry_for_each_event() {
    let view = board(json!([
        { "type": "drawn", "player": "p1", "instanceId": "c1", "defId": "core-011" },
        { "type": "manaSpent", "player": "p1", "amount": 1, "for": "play" },
        { "type": "cardPlayed", "player": "p2", "instanceId": "c7", "defId": "core-032", "costPaid": 2 },
    ]));
    let request = serde_json::to_value(LogCall::View {
        view: Box::new(view),
        locale: Locale::ZhCn,
    })
    .expect("JSON");
    assert_eq!(request["call"], "view");
    assert_eq!(request["locale"], "zh-CN");
    let call: LogCall = serde_json::from_value(request).expect("a LogCall");
    let answer = serde_json::to_value(apply(&call)).expect("JSON");
    assert_eq!(answer["answer"], "lines");
    let lines = answer["lines"].as_array().expect("an array");
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[1], Value::Null);
    assert_eq!(lines[2]["event"], "cardPlayed");
    assert_eq!(lines[2]["card"]["defId"], "core-032");
    assert!(lines[0].get("card").is_none(), "a line with no card carries none");
}

// ---- every sentence has its translation

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether `before` ends in a call of `call` (`s(`, `say(`) that is not the tail of a longer name.
fn ends_in_call(before: &str, call: &str) -> bool {
    before
        .trim_end()
        .strip_suffix(call)
        .is_some_and(|rest| !rest.ends_with(is_ident))
}

/// The English sentences the log writes: each literal that is the first argument of `s(` or `say(`, an arm's
/// value (`=> "…"`) or a line of its own in the log's sources, which the dictionary must hold for a reader of
/// Chinese (the web's `i18n.test.ts` held its `t()` calls to `zh-CN.json` the same way).
fn sentences() -> Vec<String> {
    let mut out = Vec::new();
    for source in [
        include_str!("../src/log/lines.rs"),
        include_str!("../src/log/names.rs"),
    ] {
        let source = source.split("#[cfg(test)]").next().unwrap_or(source);
        for line in source
            .lines()
            .map(str::trim)
            .filter(|line| !line.starts_with("//"))
        {
            let mut rest = line;
            while let Some(open) = rest.find('"') {
                let body = &rest[open + 1..];
                let mut close = 0;
                while close < body.len()
                    && !(body.as_bytes()[close] == b'"'
                        && (close == 0 || body.as_bytes()[close - 1] != b'\\'))
                {
                    close += 1;
                }
                if close >= body.len() {
                    break;
                }
                let before = &rest[..open];
                let after = body[close + 1..].trim();
                let alone = before.trim().is_empty() && (after.is_empty() || after == ",");
                let sentence = ends_in_call(before, "s(")
                    || ends_in_call(before, "say(")
                    || ends_in_call(before, "unwrap_or(")
                    || ends_in_call(before, "Some(")
                    || before.trim_end().ends_with("=>")
                    || alone;
                if sentence && !body[..close].is_empty() {
                    out.push(body[..close].replace("\\\"", "\""));
                }
                rest = &body[close + 1..];
            }
        }
    }
    out
}

/// The sentences the sources build or pass by name, so the scan cannot see them.
const BUILT: &[&str] = &[
    "Both players swapped health",
    "Both players swapped board",
    "Both players swapped library",
    "Some events may be missing here",
    "Unknown effect",
    "ATK",
    "DEF",
];

/// The sentences that have no entry on purpose: the web's dictionary has none for them either, so they read
/// as written in every language.
const READ_AS_WRITTEN: &[&str] = &["X", "Duo", "Mark", "Unknown effect"];

#[test]
fn every_sentence_the_log_writes_has_a_chinese_entry() {
    let mut found = sentences();
    assert!(found.len() > 150, "the scan found only {} sentences", found.len());
    found.extend(BUILT.iter().map(|sentence| sentence.to_string()));
    // The words a Call to Chaos effect is shown by.
    for (_, names) in jackioh_shared::face::data::CHAOS_NAMES {
        found.extend(names.iter().map(|(_, name)| name.to_string()));
    }
    let missing: Vec<&String> = found
        .iter()
        .filter(|sentence| !READ_AS_WRITTEN.contains(&sentence.as_str()) && find(ZH_CN, sentence).is_none())
        .collect();
    assert!(missing.is_empty(), "no zh-CN entry for: {missing:#?}");
}

#[test]
fn every_chinese_entry_keeps_the_tokens_of_its_english_key() {
    fn tokens(text: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = text;
        while let Some(open) = rest.find('{') {
            match rest[open..].find('}') {
                Some(close) => {
                    out.push(rest[open + 1..open + close].to_string());
                    rest = &rest[open + close + 1..];
                }
                None => break,
            }
        }
        out.sort();
        out
    }
    for (english, chinese) in ZH_CN {
        assert_eq!(tokens(english), tokens(chinese), "{english:?} -> {chinese:?}");
    }
}
