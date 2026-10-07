//! Port of `packages/engine/test/counterWarning.test.ts`.
//!
//! R667: the counter warning a hand card's view carries (`counterWarning.ts`, `viewFor`), driven with
//! fixture scripts so the engine is proved without `packages/cards`; Classic #87 Plague Chalice's own
//! test proves the real card again (CLAUDE.md, "The engine doesn't depend on packages/cards").
//!
//! Fixtures are prefixed `cw-` and indexed above 1480 so they cannot collide (BUILD §0).

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{in_hand, new_game, put, slot};

/// TS `def(name, type, cost)`, its module `nextIndex` (from 1480) written out per definition.
fn def(name: &str, type_: &str, cost: i32, index: i32) -> CardDef {
    json_as(json!({
        "id": format!("cw-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (counterWarning)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    }))
}

/// Counters every play at cost 2, whoever makes it: a Plague Chalice standing at 2.
fn counter_field() -> CardDef {
    def("counter-field", "Field Spell", 1, 1481)
}

/// The same counter as a Trap, which answers through `traps.ts` and so warns of nothing.
fn counter_trap() -> CardDef {
    def("counter-trap", "Trap", 1, 1482)
}

/// Counters only its controller's opponent at cost 2: a Radiant Chalice.
fn opponents_only() -> CardDef {
    def("opponents-only", "Field Spell", 1, 1483)
}

fn two() -> CardDef {
    def("two", "Spell", 2, 1484)
}

fn one() -> CardDef {
    def("one", "Spell", 1, 1485)
}

fn defs() -> Vec<CardDef> {
    vec![counter_field(), counter_trap(), opponents_only(), two(), one()]
}

/// A counter trigger on `cardAnnounced` that never fires: the warning reads its zone, not its run.
fn announced() -> TriggerDef {
    TriggerDef::new("cw-counter", &[GameEventType::CardAnnounced], |_ctx, _event| vec![])
        .with_when(|_ctx, _event| false)
}

fn counter(opponent_only: bool) -> Script {
    Script {
        triggers: vec![announced()],
        would_counter: Some(would_counter_hook(move |args| {
            args.cost_paid == 2 && !(opponent_only && args.player == args.controller)
        })),
        ..Script::default()
    }
}

fn both(script: Script) -> CardScripts {
    CardScripts { base: script.clone(), radiant: script }
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(counter_field().id, both(counter(false)));
    scripts.insert(counter_trap().id, both(counter(false)));
    scripts.insert(opponents_only().id, both(counter(true)));
    scripts.insert(two().id, both(Script { cry: Some(hook(|_ctx| vec![])), ..Script::default() }));
    scripts.insert(one().id, both(Script { cry: Some(hook(|_ctx| vec![])), ..Script::default() }));
    scripts
}

static NONCE: AtomicU32 = AtomicU32::new(0);

fn act(state: &GameState, body: Value) -> GameState {
    let n = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let mut action = body;
    action["nonce"] = json!(format!("cw{n}"));
    reduce(state, &json_as::<Action>(action)).state
}

/// Past the mulligans, in p1's main phase, each hand holding a (2) and a (1) Spell.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    let keep: Vec<String> = state.players.p1.hand.iter().map(|c| c.id.clone()).collect();
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p1" }));
    let keep: Vec<String> = state.players.p2.hand.iter().map(|c| c.id.clone()).collect();
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }));
    let mut catalog = registered_catalog().clone();
    for card in defs() {
        catalog.insert(card.id.clone(), card);
    }
    register_catalog(catalog);
    let mut all = registered_scripts().clone();
    all.extend(scripts());
    register_scripts(all);
    for player in [PlayerId::P1, PlayerId::P2] {
        in_hand(&mut state, &two().id, player, 1);
        in_hand(&mut state, &one().id, player, 1);
    }
    state
}

/// The definitions `player`'s own view marks `counteredOnPlay`.
fn warned(view: &PlayerView) -> Vec<String> {
    match &view.you.hand {
        HandView::Cards(hand) => hand
            .iter()
            .filter(|card| card.countered_on_play == Some(true))
            .map(|card| card.def_id.clone())
            .collect(),
        HandView::Count { .. } => vec![],
    }
}

mod r667_the_counter_warning_on_the_viewers_hand {
    use super::*;

    #[test]
    fn r667_marks_each_seats_own_hand_cards_that_a_field_card_would_counter_at_every_price_and_no_others() {
        let mut state = playing("r658-field");
        put(&mut state, &counter_field().id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());

        assert_eq!(warned(&view_for(&state, PlayerId::P1)), vec![two().id]);
        assert_eq!(warned(&view_for(&state, PlayerId::P2)), vec![two().id]);
        // Never `false`: a card it would not counter carries no key at all.
        let HandView::Cards(hand) = view_for(&state, PlayerId::P1).you.hand else {
            panic!("p1 reads its own hand");
        };
        assert!(hand.iter().filter(|card| card.def_id != two().id).all(|card| card.countered_on_play.is_none()));
        // R97: the opponent's hand is a count, with nothing on it.
        assert_eq!(
            view_for(&state, PlayerId::P1).opponent.hand,
            HandView::Count { count: state.players.p2.hand.len() as i32 }
        );
    }

    #[test]
    fn r667_a_counter_that_covers_only_its_controllers_opponent_warns_only_them() {
        let mut state = playing("r658-opponent");
        put(&mut state, &opponents_only().id, slot(PlayerId::P1, Row::Backrow, 1), Default::default());

        assert_eq!(warned(&view_for(&state, PlayerId::P1)), Vec::<String>::new());
        assert_eq!(warned(&view_for(&state, PlayerId::P2)), vec![two().id]);
    }

    #[test]
    fn r667_nothing_warns_while_no_card_on_the_field_would_counter_none_at_all_one_in_a_hand_or_a_trap() {
        let none = playing("r658-none");
        assert_eq!(countered_hand_cards(&none, PlayerId::P1).len(), 0);

        let mut held = playing("r658-held");
        in_hand(&mut held, &counter_field().id, PlayerId::P2, 1);
        assert_eq!(warned(&view_for(&held, PlayerId::P1)), Vec::<String>::new());

        // A Trap answers through `traps.ts`, never the trigger dispatch, and face-down it is unreadable (R97).
        let mut trapped = playing("r658-trap");
        put(&mut trapped, &counter_trap().id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        assert_eq!(warned(&view_for(&trapped, PlayerId::P1)), Vec::<String>::new());
    }

    #[test]
    fn r667_the_warning_is_the_viewers_own_it_is_asked_for_one_hand_and_lists_that_hands_ids() {
        let mut state = playing("r658-ids");
        put(&mut state, &counter_field().id, slot(PlayerId::P1, Row::Backrow, 2), Default::default());
        let ids = |state: &GameState, player: PlayerId| -> Vec<String> {
            state.players[player]
                .hand
                .iter()
                .filter(|card| card.def_id == two().id)
                .map(|card| card.id.clone())
                .collect()
        };

        assert_eq!(
            countered_hand_cards(&state, PlayerId::P1).into_iter().collect::<Vec<String>>(),
            ids(&state, PlayerId::P1)
        );
        assert_eq!(
            countered_hand_cards(&state, PlayerId::P2).into_iter().collect::<Vec<String>>(),
            ids(&state, PlayerId::P2)
        );
    }
}
