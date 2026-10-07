//! Port of `packages/engine/test/viewFor.test.ts`.
//!
//! `viewFor(state, playerId)` — the one window a player has onto a match (SPEC §10.8, BUILD M3-T6).
//!
//! The centrepiece is the hidden-information proof: a board where p2 holds a full hand, a full
//! library and face-down traps, serialized for p1, must name none of it. The forbidden list is
//! derived from the state itself rather than written out here, so the proof cannot quietly stop
//! proving anything when the fixtures change.
//!
//! The rest of §10.8 is the positive half — own hand in full, the opponent's as a count, both
//! libraries as counts, Field Spells public, graveyards and exile in full, the viewer's own prompt
//! options only, mana, health, armor, the clock and the last N events — plus R33's two halves: a
//! face-down trap follows its *controller*, and a Field Trap that has fired is public to both.
//!
//! R97 is here too: the event stream is filtered like the zones it reports on — redacted to the
//! `"hidden"` sentinel, never truncated, and judged by where a card sits *now*.
//!
//! The Trap, Field Trap and "secret" definitions this file needs live here rather than in a shared
//! fixture, as effects-swap.test.ts does for its own Trap (BUILD §0, CLAUDE.md).

use jackioh_engine::effects::steal::steal;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{in_hand, new_game, put, set_library, slot};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// TS's module `let nextIndex = 1300`: every definition below takes the next index, in the order
/// the TS module built them.
const FIRST_INDEX: i32 = 1300;

/// `{ ...base, ...extra }` on two JSON objects: `extra`'s keys replace `base`'s (TS's spread).
fn spread(base: &mut Value, extra: Value) {
    if let (Some(base), Value::Object(extra)) = (base.as_object_mut(), extra) {
        for (key, value) in extra {
            base.insert(key, value);
        }
    }
}

fn def(next_index: &mut i32, name: &str, type_: &str, extra: Value) -> CardDef {
    *next_index += 1;
    let mut literal = json!({
        "id": format!("vf-{name}"),
        "index": (*next_index).to_string(),
        "name": name,
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    });
    spread(&mut literal, extra);
    json_as(literal)
}

fn unit(next_index: &mut i32, name: &str, extra: Value) -> CardDef {
    let mut literal = json!({
        "base": { "attack": 2, "health": 3, "keywords": [], "text": name },
        "radiant": { "attack": 4, "health": 6, "keywords": [], "text": name },
    });
    spread(&mut literal, extra);
    def(next_index, name, "Unit", literal)
}

/// Two-digit names, so no secret id is a prefix of another one.
fn pad(n: i32) -> String {
    if n < 10 { format!("0{n}") } else { n.to_string() }
}

/// This file's definitions, built in TS's module order (so each takes TS's index).
struct Defs {
    /// §9.1's hidden zones, each with definitions of its own, so a leak names its own zone.
    secret_hand: Vec<CardDef>,
    secret_library: Vec<CardDef>,
    /// A face-down Trap and a Field Trap: the two backrow types §10.8 calls "unknown".
    secret_trap: CardDef,
    secret_field_trap: CardDef,
    /// The card under a Stack, whose identity R13 and §3.2 keep from both players.
    secret_buried: CardDef,
    /// A Field Spell in the backrow, which §10.8 makes public.
    public_field: CardDef,
    /// A Stack unit, to bury the secret one under (§3.2).
    stack_top: CardDef,
}

fn defs() -> Defs {
    let mut next_index = FIRST_INDEX;
    let secret_hand = (0..HAND_CAP)
        .map(|i| unit(&mut next_index, &format!("secret-hand-{}", pad(i)), json!({})))
        .collect();
    let secret_library = (0..12)
        .map(|i| def(&mut next_index, &format!("secret-library-{}", pad(i)), "Spell", json!({})))
        .collect();
    let secret_trap = def(&mut next_index, "secret-trap", "Trap", json!({}));
    let secret_field_trap = def(&mut next_index, "secret-field-trap", "Field Trap", json!({}));
    let secret_buried = unit(
        &mut next_index,
        "secret-buried",
        json!({ "base": { "attack": 9, "health": 9, "keywords": [], "text": "buried" } }),
    );
    let public_field = def(&mut next_index, "public-field", "Field Spell", json!({}));
    let stack_top = unit(
        &mut next_index,
        "stack-top",
        json!({
            "base": { "attack": 1, "health": 1, "keywords": [{ "kind": "Stack" }], "text": "stack" },
            "radiant": { "attack": 2, "health": 2, "keywords": [{ "kind": "Stack" }], "text": "stack" },
        }),
    );
    Defs {
        secret_hand,
        secret_library,
        secret_trap,
        secret_field_trap,
        secret_buried,
        public_field,
        stack_top,
    }
}

impl Defs {
    /// TS's `DEFS`.
    fn all(&self) -> Vec<CardDef> {
        let mut all: Vec<CardDef> = Vec::new();
        all.extend(self.secret_hand.iter().cloned());
        all.extend(self.secret_library.iter().cloned());
        all.push(self.secret_trap.clone());
        all.push(self.secret_field_trap.clone());
        all.push(self.secret_buried.clone());
        all.push(self.public_field.clone());
        all.push(self.stack_top.clone());
        all
    }
}

/// p1's main phase on turn 3, with this file's definitions registered alongside the fixtures.
fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog: CardDefs = registered_catalog().clone();
    for d in defs().all() {
        catalog.insert(d.id.clone(), d);
    }
    register_catalog(catalog);
    state.turn = 3;
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    state
}

/// TS's `sinkFor(state)` for one call: a sink whose rng starts at the state's cursor, as reduce
/// does. The cursor is not written back, as TS's callers of a bare `sinkFor` did not.
fn with_sink<T>(state: &mut GameState, f: impl FnOnce(&mut EngineSink<'_>) -> T) -> T {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events: Vec<GameEvent> = Vec::new();
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    f(&mut sink)
}

/// Apply one effect the way `resolve.ts` does (effects-swap.test.ts's helper).
fn run(state: &mut GameState, effect: &Effect, options: HookOptions) {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events: Vec<GameEvent> = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        // TS `{ controller: "p1", ...options }`.
        let options = HookOptions {
            controller: options.controller.or(Some(PlayerId::P1)),
            ..options
        };
        let mut ctx = make_context(&mut sink, None, options);
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
}

/// The card under this id as it stands in the state (TS held the live object).
fn live<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("no card {id} in the state"))
}

fn live_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id} in the state"))
}

fn to_json(value: impl serde::Serialize) -> Value {
    serde_json::to_value(value).expect("a view serialises")
}

/// TS's `JSON.stringify(view)`.
fn serialized(view: &PlayerView) -> String {
    serde_json::to_string(view).expect("a view serialises")
}

fn quoted(id: &str) -> String {
    format!("\"{id}\"")
}

/// `expect(actual).toMatchObject(expected)`: every key `expected` names has a matching value in
/// `actual` (objects recursively), and arrays match element by element at the same length.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|found| matches_object(found, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len()
                && actual.iter().zip(expected).all(|(found, value)| matches_object(found, value))
        }
        _ => actual == expected,
    }
}

fn expect_match(actual: &Value, expected: Value) {
    assert!(matches_object(actual, &expected), "expected {actual} to match {expected}");
}

/// `Object.keys(value)` of a serialised object (absent fields are absent keys, as TS's `undefined`).
fn keys_of(value: &Value) -> Vec<String> {
    value.as_object().map(|object| object.keys().cloned().collect()).unwrap_or_default()
}

/// A TS event literal.
fn event(literal: Value) -> GameEvent {
    json_as(literal)
}

/// §10.8: the viewer's own hand is the full list, never a count.
fn own_hand(view: &PlayerView) -> Vec<CardView> {
    match &view.you.hand {
        HandView::Cards(cards) => cards.clone(),
        HandView::Count { .. } => panic!("the viewer's own hand must travel in full (§10.8)"),
    }
}

/// The options of a prompt that belongs to this viewer (§10.6).
fn own_options(view: &PlayerView) -> PendingPromptView {
    match view.pending.as_ref().expect("expected an open prompt in the view") {
        PendingView::ForYou(pending) => pending.clone(),
        PendingView::Elsewhere(_) => panic!("that prompt belongs to the other player"),
    }
}

/// A card in a pile off the field, put there directly: `graveyard` and `exile` are public (§10.8).
fn put_in_pile(state: &mut GameState, def_id: &str, player: PlayerId, pile: ZoneName) -> CardInstance {
    let zone = Zone::pile(pile, player).expect("a pile zone");
    let card = new_instance(state, def_id, player, zone);
    match pile {
        ZoneName::Graveyard => state.players[player].graveyard.push(card.clone()),
        ZoneName::Exile => state.players[player].exile.push(card.clone()),
        other => panic!("put_in_pile takes graveyard or exile, not {other}"),
    }
    card
}

/// A continuation nothing services: this file is about the view, not about answering (§10.6).
fn inert_resume() -> Resume {
    Resume {
        def_id: String::new(),
        hook: "resume".to_string(),
        step: "none".to_string(),
        radiant: false,
        instance_id: None,
        data: IndexMap::new(),
    }
}

fn mode_option(option: &str) -> PromptOption {
    PromptOption {
        key: format!("mode:{option}"),
        label: option.to_string(),
        selection: Selection::Mode { option: option.to_string() },
        cost: None,
        radiant: None,
    }
}

// ---------------------------------------------------------------------------

mod view_for_10_8_m3_t6 {
    use super::*;

    #[test]
    fn section_10_8_names_nothing_in_the_opponents_hand_library_or_face_down_backrow_derived_from_the_state() {
        let d = defs();
        let mut state = game("hidden-information");

        // p2 fills all three zones §9.1 hides: a full hand, a library and two face-down backrow cards.
        let their_hand: Vec<CardInstance> = d
            .secret_hand
            .iter()
            .map(|def| in_hand(&mut state, &def.id, PlayerId::P2, None).remove(0))
            .collect();
        let library_ids: Vec<String> = d.secret_library.iter().map(|def| def.id.clone()).collect();
        let their_library = set_library(&mut state, PlayerId::P2, &library_ids);
        let their_traps = vec![
            put(&mut state, &d.secret_trap.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default()),
            put(&mut state, &d.secret_field_trap.id, slot(PlayerId::P2, Row::Backrow, 2), Default::default()),
        ];
        assert_eq!(their_hand.len(), HAND_CAP as usize);

        // Public things on the same board, so the proof is not vacuous: an empty view hides everything.
        let my_hand = in_hand(&mut state, &plain().id, PlayerId::P1, Some(3));
        let their_unit = put(&mut state, &plain().id, slot(PlayerId::P2, Row::Units, 1), Default::default());
        let their_field = put(&mut state, &d.public_field.id, slot(PlayerId::P2, Row::Backrow, 3), Default::default());

        // §10.3's event stream is filtered like the zones it reports on: a `drawn` naming a card that is
        // now in p2's hand may not carry its identity to p1 either (§9.1, §10.8's "last N events").
        let drawn = their_hand[0].clone();
        state.applied = vec![AppliedAction {
            nonce: "hidden-1".to_string(),
            events: vec![event(json!({
                "type": "drawn", "player": "p2", "instanceId": drawn.id, "defId": drawn.def_id,
            }))],
        }];

        // The forbidden list comes out of the state, never out of a literal in this file.
        let mut forbidden: IndexSet<String> = IndexSet::new();
        for card in state.players.p2.hand.iter().chain(state.players.p2.library.iter()) {
            forbidden.insert(card.id.clone());
            forbidden.insert(card.def_id.clone());
        }
        for card in state.players.p2.backrow.iter().flatten() {
            let type_ = def_of(Some(&state), &card.def_id).type_;
            if card.face_up == Some(true) || (type_ != CardType::Trap && type_ != CardType::FieldTrap) {
                continue;
            }
            forbidden.insert(card.id.clone());
            forbidden.insert(card.def_id.clone());
        }
        // Every hidden card contributed both of its ids, so the list cannot have gone quietly empty.
        assert_eq!(forbidden.len(), 2 * (their_hand.len() + their_library.len() + their_traps.len()));

        let view = view_for(&state, PlayerId::P1);
        let text = serialized(&view);
        for id in &forbidden {
            assert!(!text.contains(&quoted(id)), "p1's view names {id}");
        }

        // The positive half of §10.8, on the same board.
        assert_eq!(
            own_hand(&view).iter().map(|card| card.instance_id.clone()).collect::<Vec<_>>(),
            my_hand.iter().map(|card| card.id.clone()).collect::<Vec<_>>()
        );
        assert_eq!(view.opponent.hand, HandView::Count { count: HAND_CAP });
        assert_eq!(view.you.library_count, state.players.p1.library.len() as i32);
        assert_eq!(view.opponent.library_count, their_library.len() as i32);
        // Units are public on both sides; the two traps are markers and the Field Spell is not.
        let v = to_json(&view);
        expect_match(
            &v["opponent"]["units"][0],
            json!({ "defId": their_unit.def_id, "attack": 3, "health": 3, "buried": 0 }),
        );
        assert_eq!(v["opponent"]["backrow"][0], json!({ "faceDown": true, "cost": 1 }));
        assert_eq!(v["opponent"]["backrow"][1], json!({ "faceDown": true, "cost": 1 }));
        expect_match(
            &v["opponent"]["backrow"][2],
            json!({
                "faceDown": false,
                "defId": d.public_field.id,
                "instanceId": their_field.id,
                "type": "Field Spell",
            }),
        );
        // The redacted event kept its cue and its place, so the animation still runs (§10.10).
        assert_eq!(view.events.len(), 1);
        assert_eq!(view.events[0].event_type(), GameEventType::Drawn);

        // p2's own view is the mirror: p2 reads p2's hand, and p1's hand is the count there.
        let their_view = view_for(&state, PlayerId::P2);
        assert_eq!(
            own_hand(&their_view).iter().map(|card| card.def_id.clone()).collect::<Vec<_>>(),
            their_hand.iter().map(|card| card.def_id.clone()).collect::<Vec<_>>()
        );
        assert_eq!(their_view.opponent.hand, HandView::Count { count: my_hand.len() as i32 });
        expect_match(
            &to_json(&their_view)["you"]["backrow"][0],
            json!({ "faceDown": false, "defId": d.secret_trap.id }),
        );
    }

    #[test]
    fn section_10_8_the_viewers_own_hand_travels_in_full_and_in_order_and_the_opponents_as_a_count() {
        let d = defs();
        let mut state = game("hands");
        let mine = in_hand(&mut state, &plain().id, PlayerId::P1, Some(2));
        live_mut(&mut state, &mine[1].id).radiant = true;
        let theirs = in_hand(&mut state, &d.secret_trap.id, PlayerId::P2, Some(4));

        let view = view_for(&state, PlayerId::P1);
        expect_match(
            &to_json(own_hand(&view)),
            json!([
                { "instanceId": mine[0].id, "defId": plain().id, "radiant": false, "cost": 1 },
                { "instanceId": mine[1].id, "defId": plain().id, "radiant": true, "cost": 1 },
            ]),
        );
        assert_eq!(own_hand(&view).len(), mine.len());
        // A count and nothing else: no array, no ids, no defs (§10.8).
        assert_eq!(view.opponent.hand, HandView::Count { count: theirs.len() as i32 });
        assert_eq!(keys_of(&to_json(&view.opponent.hand)), vec!["count".to_string()]);
    }

    #[test]
    fn section_9_1_sends_both_libraries_as_a_count_and_no_library_cards_instance_reaches_either_players_view() {
        let d = defs();
        let mut state = game("libraries");
        let library_ids: Vec<String> = d.secret_library.iter().map(|def| def.id.clone()).collect();
        let their_library = set_library(&mut state, PlayerId::P2, &library_ids);
        let my_library = state.players.p1.library.clone();
        assert!(!my_library.is_empty());

        let mine = serialized(&view_for(&state, PlayerId::P1));
        let theirs = serialized(&view_for(&state, PlayerId::P2));

        assert_eq!(view_for(&state, PlayerId::P1).you.library_count, my_library.len() as i32);
        assert_eq!(view_for(&state, PlayerId::P1).opponent.library_count, their_library.len() as i32);
        // Library order is hidden from *both* players, the owner included: no instance id ships. The
        // owner's own library also travels as a list without order (R310, ownLibrary.test.ts), and the
        // opponent's contents never do.
        for card in &my_library {
            assert!(!mine.contains(&quoted(&card.id)));
            assert!(!theirs.contains(&quoted(&card.id)));
        }
        for card in &their_library {
            assert!(!mine.contains(&quoted(&card.id)));
            assert!(!mine.contains(&quoted(&card.def_id)));
            assert!(!theirs.contains(&quoted(&card.id)));
        }
    }

    #[test]
    fn section_10_8_sends_both_graveyards_and_both_exile_piles_in_full() {
        let d = defs();
        let mut state = game("graveyards");
        let my_grave = put_in_pile(&mut state, &d.secret_library[0].id, PlayerId::P1, ZoneName::Graveyard);
        let my_exile = put_in_pile(&mut state, &d.secret_library[1].id, PlayerId::P1, ZoneName::Exile);
        let their_grave = put_in_pile(&mut state, &d.secret_library[2].id, PlayerId::P2, ZoneName::Graveyard);
        let their_exile = put_in_pile(&mut state, &d.secret_library[3].id, PlayerId::P2, ZoneName::Exile);

        for viewer in [PlayerId::P1, PlayerId::P2] {
            let view = view_for(&state, viewer);
            let (own, other) = if viewer == PlayerId::P1 {
                (&view.you, &view.opponent)
            } else {
                (&view.opponent, &view.you)
            };
            expect_match(
                &to_json(&own.graveyard),
                json!([{ "instanceId": my_grave.id, "defId": my_grave.def_id, "radiant": false, "cost": 1 }]),
            );
            expect_match(
                &to_json(&own.exile),
                json!([{ "instanceId": my_exile.id, "defId": my_exile.def_id, "radiant": false, "cost": 1 }]),
            );
            // The opponent's piles too: nothing about a graveyard or an exile pile is hidden.
            assert_eq!(
                other.graveyard.iter().map(|card| card.def_id.clone()).collect::<Vec<_>>(),
                vec![their_grave.def_id.clone()]
            );
            assert_eq!(
                other.exile.iter().map(|card| card.def_id.clone()).collect::<Vec<_>>(),
                vec![their_exile.def_id.clone()]
            );
        }
    }

    #[test]
    fn r351_a_face_down_trap_shows_its_cost_to_both_players_and_its_controllers_view_marks_it_unrevealed() {
        let d = defs();
        let mut state = game("face-down-cost");
        let trap = put(&mut state, &d.secret_trap.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        let field_trap =
            put(&mut state, &d.secret_field_trap.id, slot(PlayerId::P2, Row::Backrow, 2), Default::default());
        let field = put(&mut state, &d.public_field.id, slot(PlayerId::P2, Row::Backrow, 3), Default::default());

        let mine = to_json(view_for(&state, PlayerId::P1));
        let theirs = to_json(view_for(&state, PlayerId::P2));
        // The opponent reads the cost and nothing else; the controller reads the same number.
        let cost = effective_cost(&state, live(&state, &trap.id), Default::default());
        assert_eq!(mine["opponent"]["backrow"][0], json!({ "faceDown": true, "cost": cost }));
        expect_match(
            &theirs["you"]["backrow"][0],
            json!({ "faceDown": false, "cost": cost, "unrevealed": true }),
        );
        // The number is the card's own, as it stands: a costMod moves it for both (R65).
        live_mut(&mut state, &trap.id).cost_mod = 2;
        let cost = effective_cost(&state, live(&state, &trap.id), Default::default());
        assert_eq!(
            to_json(view_for(&state, PlayerId::P1))["opponent"]["backrow"][0],
            json!({ "faceDown": true, "cost": cost })
        );
        expect_match(&to_json(view_for(&state, PlayerId::P2))["you"]["backrow"][0], json!({ "cost": cost }));

        // A Field Trap that has fired is public: no longer unrevealed, and read in full by both (R33).
        expect_match(&theirs["you"]["backrow"][1], json!({ "unrevealed": true }));
        live_mut(&mut state, &field_trap.id).face_up = Some(true);
        assert!(to_json(view_for(&state, PlayerId::P2))["you"]["backrow"][1].get("unrevealed").is_none());
        expect_match(
            &to_json(view_for(&state, PlayerId::P1))["opponent"]["backrow"][1],
            json!({ "faceDown": false, "defId": d.secret_field_trap.id }),
        );
        // A Field Spell is public to both from the start, so it is never unrevealed.
        assert!(to_json(view_for(&state, PlayerId::P2))["you"]["backrow"][2].get("unrevealed").is_none());
        assert!(!field.id.is_empty());
    }

    #[test]
    fn section_10_8_makes_a_backrow_field_spell_public_and_a_backrow_trap_unknown_to_the_opponent() {
        let d = defs();
        let mut state = game("backrow-types");
        let field = put(&mut state, &d.public_field.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        let trap = put(&mut state, &d.secret_trap.id, slot(PlayerId::P2, Row::Backrow, 2), Default::default());
        let field_trap =
            put(&mut state, &d.secret_field_trap.id, slot(PlayerId::P2, Row::Backrow, 3), Default::default());
        live_mut(&mut state, &trap.id).counters.grade = Some(2);

        let mine = view_for(&state, PlayerId::P1);
        let mine_json = to_json(&mine);
        expect_match(
            &mine_json["opponent"]["backrow"][0],
            json!({
                "faceDown": false,
                "instanceId": field.id,
                "defId": d.public_field.id,
                "type": "Field Spell",
            }),
        );
        // Both trap types are "unknown", and the marker carries no identity at all: §10.8 grants the
        // non-controller that the zone is occupied and what it costs (R351), and nothing more.
        assert_eq!(mine_json["opponent"]["backrow"][1], json!({ "faceDown": true, "cost": 1 }));
        assert_eq!(mine_json["opponent"]["backrow"][2], json!({ "faceDown": true, "cost": 1 }));
        for marker in [&mine_json["opponent"]["backrow"][1], &mine_json["opponent"]["backrow"][2]] {
            for field in ["instanceId", "defId", "type", "counters", "radiant"] {
                assert!(!keys_of(marker).contains(&field.to_string()));
            }
        }
        assert!(mine.opponent.backrow[3].is_none());

        // Their controller reads both, with the grade counter a Combo-Index trap carries (§10.1).
        let theirs = to_json(view_for(&state, PlayerId::P2));
        expect_match(
            &theirs["you"]["backrow"][1],
            json!({
                "faceDown": false,
                "instanceId": trap.id,
                "defId": d.secret_trap.id,
                "type": "Trap",
                "counters": { "grade": 2 },
            }),
        );
        expect_match(
            &theirs["you"]["backrow"][2],
            json!({
                "faceDown": false,
                "instanceId": field_trap.id,
                "defId": d.secret_field_trap.id,
                "type": "Field Trap",
            }),
        );
    }

    #[test]
    fn r33_a_stolen_trap_becomes_visible_to_its_thief_and_hidden_from_its_owner_though_ownership_is_unchanged() {
        let d = defs();
        let mut state = game("r33-steal");
        let hidden = put(&mut state, &d.secret_trap.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());

        // Before the steal: p2 controls it and reads it; p1 sees a marker and the id never travels.
        expect_match(
            &to_json(view_for(&state, PlayerId::P2))["you"]["backrow"][0],
            json!({ "faceDown": false, "defId": d.secret_trap.id }),
        );
        assert_eq!(
            to_json(view_for(&state, PlayerId::P1))["opponent"]["backrow"][0],
            json!({ "faceDown": true, "cost": 1 })
        );
        assert!(!serialized(&view_for(&state, PlayerId::P1)).contains(&quoted(&d.secret_trap.id)));

        run(
            &mut state,
            &steal(json_as(json!({ "instanceId": hidden.id }))),
            HookOptions { controller: Some(PlayerId::P1), ..Default::default() },
        );

        // R33's point: control moved, ownership did not, and the view follows control.
        let stolen = live(&state, &hidden.id);
        assert_eq!(stolen.controller, PlayerId::P1);
        assert_eq!(stolen.owner, PlayerId::P2);
        assert_eq!(stolen.face_up, None);
        expect_match(
            &to_json(view_for(&state, PlayerId::P1))["you"]["backrow"][0],
            json!({ "faceDown": false, "instanceId": hidden.id, "defId": d.secret_trap.id }),
        );
        assert_eq!(
            to_json(view_for(&state, PlayerId::P2))["opponent"]["backrow"][0],
            json!({ "faceDown": true, "cost": 1 })
        );
        // The previous controller stops seeing it entirely, even though it still owns the card.
        assert!(!serialized(&view_for(&state, PlayerId::P2)).contains(&quoted(&d.secret_trap.id)));
        assert!(!serialized(&view_for(&state, PlayerId::P2)).contains(&quoted(&hidden.id)));
    }

    #[test]
    fn r33_a_field_trap_that_has_fired_is_face_up_to_both_players() {
        let d = defs();
        let mut state = game("r33-fired");
        let fired =
            put(&mut state, &d.secret_field_trap.id, slot(PlayerId::P2, Row::Backrow, 2), Default::default());
        assert_eq!(
            to_json(view_for(&state, PlayerId::P1))["opponent"]["backrow"][1],
            json!({ "faceDown": true, "cost": 1 })
        );

        // §10.1 reserves `faceUp` for exactly this ("a Field Trap that has fired, R33"); `traps.ts`
        // sets it when the trap fires, and R33's half that lives here is what the view does with it.
        live_mut(&mut state, &fired.id).face_up = Some(true);

        expect_match(
            &to_json(view_for(&state, PlayerId::P1))["opponent"]["backrow"][1],
            json!({
                "faceDown": false,
                "instanceId": fired.id,
                "defId": d.secret_field_trap.id,
                "type": "Field Trap",
            }),
        );
        expect_match(
            &to_json(view_for(&state, PlayerId::P2))["you"]["backrow"][1],
            json!({ "faceDown": false, "defId": d.secret_field_trap.id }),
        );
    }

    #[test]
    fn section_10_8_sends_a_prompts_options_to_its_own_player_only_the_revealed_library_card_included() {
        let mut state = game("prompt-options");
        // §10.8: "a card revealed out of a library is revealed only as an option of the prompt that
        // reveals it: the chooser sees it in full, the opponent sees only that a prompt is open."
        let revealed = state.players.p1.library[0].clone();
        let rest: Vec<CardInstance> = state.players.p1.library[1..].to_vec();
        let pending = with_sink(&mut state, |sink| {
            open_prompt(
                sink,
                OpenPromptArgs {
                    player: PlayerId::P1,
                    kind: PromptKind::Discover,
                    aim: None,
                    prompt: "Put a card from your library into your hand".to_string(),
                    options: vec![PromptOption {
                        key: format!("instance:{}", revealed.id),
                        label: "a card".to_string(),
                        selection: Selection::Instance { instance_id: revealed.id.clone() },
                        cost: None,
                        radiant: None,
                    }],
                    min: Some(1),
                    max: Some(1),
                    budget: None,
                    owner: None,
                    resume: inert_resume(),
                },
            )
        })
        .expect("expected the discover prompt");

        let mine = view_for(&state, PlayerId::P1);
        let own_pending = own_options(&mine);
        assert_eq!(own_pending.choice_id, pending.id);
        assert_eq!(own_pending.kind, PromptKind::Discover);
        assert_eq!(own_pending.min, 1);
        assert_eq!(own_pending.max, 1);
        assert_eq!(own_pending.prompt, "Put a card from your library into your hand");
        assert_eq!(
            to_json(&own_pending.options),
            json!([{
                "key": format!("instance:{}", revealed.id),
                "label": "a card",
                "instanceId": revealed.id,
                "defId": revealed.def_id,
            }])
        );
        // The rest of the library stays hidden from the chooser too.
        for card in &rest {
            assert!(!serialized(&mine).contains(&quoted(&card.id)));
        }

        // The opponent learns that a prompt is open and whose, and nothing else (§10.6, R81).
        let theirs = view_for(&state, PlayerId::P2);
        assert_eq!(to_json(&theirs.pending), json!({ "forYou": false, "pendingFor": "p1" }));
        let text = serialized(&theirs);
        assert!(!text.contains(&quoted(&revealed.id)));
        assert!(!text.contains("a card"));
        assert!(!text.contains("Put a card from your library into your hand"));
    }

    #[test]
    fn r13_section_3_2_a_stack_pile_shows_its_top_card_and_a_count_never_a_buried_cards_identity() {
        let d = defs();
        let mut state = game("stack");
        let buried = put(&mut state, &d.secret_buried.id, slot(PlayerId::P2, Row::Units, 1), Default::default());
        let mut top = new_instance(&mut state, &d.stack_top.id, PlayerId::P2, Zone::Hand { player: PlayerId::P2 });
        assert!(place_on_field(
            &mut state,
            &mut top,
            slot(PlayerId::P2, Row::Units, 1),
            json_as(json!({ "stack": true })),
        ));

        for viewer in [PlayerId::P1, PlayerId::P2] {
            let view = view_for(&state, viewer);
            let side = if viewer == PlayerId::P2 { &view.you } else { &view.opponent };
            expect_match(
                &to_json(&side.units[0]),
                json!({ "instanceId": top.id, "defId": d.stack_top.id, "buried": 1, "attack": 1 }),
            );
            // The buried card is not on the field (R13), so neither player is told what it is.
            let text = serialized(&view);
            assert!(!text.contains(&quoted(&buried.id)));
            assert!(!text.contains(&quoted(&d.secret_buried.id)));
        }
    }

    #[test]
    fn r79_section_10_8_carries_mana_health_armor_locks_the_phase_the_result_and_the_servers_clock() {
        let mut state = game("scalars");
        state.players.p1.hero = HeroState { health: 12, armor: 3 };
        state.players.p1.mana = ManaState { current: 2, max: 4, next_turn_mod: -1, perm_mod: 1 };
        state.players.p2.hero = HeroState { health: 21, armor: 0 };
        state.players.p2.fatigue_count = 2;
        lock_zone(&mut state, slot(PlayerId::P2, Row::Backrow, 4));

        let view = view_for(&state, PlayerId::P1);
        assert_eq!(view.viewer, PlayerId::P1);
        assert_eq!(view.turn, 3);
        assert_eq!(view.active, PlayerId::P1);
        assert_eq!(view.phase, Phase::Main);
        expect_match(&to_json(&view.you.hero), json!({ "health": 12, "armor": 3 }));
        expect_match(&to_json(&view.opponent.hero), json!({ "health": 21, "armor": 0 }));
        // Mana is the pair a player spends from; the two modifiers behind it are bookkeeping (§2.3).
        assert_eq!(view.you.mana, ManaView { current: 2, max: 4 });
        assert_eq!(view.opponent.fatigue_count, 2);
        assert!(view.opponent.locks.backrow[3]);
        assert!(!view.opponent.locks.backrow[0]);
        assert!(view.result.is_none());

        // R79: the engine holds no clock, so the caller passes the one the server is running.
        assert_eq!(view_for_with_clock(&state, PlayerId::P1, Some(75_000)).clock_ms, Some(75_000));
        assert_eq!(view_for(&state, PlayerId::P1).clock_ms, None);

        state.result = Some(json_as(json!({ "winner": "p1", "reason": "hero-death" })));
        assert_eq!(
            to_json(&view_for(&state, PlayerId::P1).result),
            json!({ "winner": "p1", "reason": "hero-death" })
        );
    }

    #[test]
    fn section_10_8_carries_the_last_n_events_for_animation_oldest_first() {
        let mut state = game("events");
        let total = VIEW_EVENT_LIMIT + 5;
        state.applied = (0..total)
            .map(|i| AppliedAction {
                nonce: format!("n{i}"),
                events: vec![event(json!({ "type": "manaChanged", "player": "p1", "current": i, "max": 4 }))],
            })
            .collect();

        let events = view_for(&state, PlayerId::P1).events;
        assert_eq!(events.len(), VIEW_EVENT_LIMIT);
        let current: Vec<i32> = events
            .iter()
            .map(|event| match event {
                GameEvent::ManaChanged { current, .. } => *current,
                _ => -1,
            })
            .collect();
        // The tail of the stream, in the order it happened.
        assert_eq!(
            current,
            (0..VIEW_EVENT_LIMIT).map(|i| (total - VIEW_EVENT_LIMIT + i) as i32).collect::<Vec<_>>()
        );

        // A shorter history travels whole, and an empty one is an empty list rather than a missing field.
        state.applied = vec![AppliedAction {
            nonce: "one".to_string(),
            events: vec![event(json!({ "type": "turnStarted", "player": "p2", "turn": 4 }))],
        }];
        assert_eq!(
            to_json(&view_for(&state, PlayerId::P1).events),
            json!([{ "type": "turnStarted", "player": "p2", "turn": 4 }])
        );
        state.applied = vec![];
        assert_eq!(to_json(&view_for(&state, PlayerId::P1).events), json!([]));
    }

    #[test]
    fn r168_section_10_8s_n_is_a_floor_one_actions_own_event_burst_is_never_truncated() {
        let mut state = game("burst");

        // §10.8's window is the client's only animation channel (BUILD M5-T4), so an action longer
        // than `VIEW_EVENT_LIMIT` must not lose its front. #96 My Pawn's cancel plus the §10.7 AI turn
        // it hands over is 38 events in one `reduce`, and `attackDeclared` / `trapFired` /
        // `attackCancelled` — the three the cancel is made of — are the first three of them.
        let head: Vec<GameEvent> = vec![
            event(json!({ "type": "attackDeclared", "attackerId": "atk", "targetId": "hero-p2", "forced": false })),
            event(json!({
                "type": "attackCancelled", "attackerId": "atk", "targetId": "hero-p2", "byInstanceId": "trap",
            })),
        ];
        let tail: Vec<GameEvent> = (0..VIEW_EVENT_LIMIT + 4)
            .map(|i| event(json!({ "type": "manaChanged", "player": "p1", "current": i, "max": 4 })))
            .collect();
        let burst: Vec<GameEvent> = head.iter().chain(tail.iter()).cloned().collect();
        state.applied = vec![AppliedAction { nonce: "burst".to_string(), events: burst.clone() }];

        let events = view_for(&state, PlayerId::P1).events;
        assert_eq!(events.len(), head.len() + tail.len());
        let types: Vec<GameEventType> = events.iter().map(GameEvent::event_type).collect();
        assert!(types.contains(&GameEventType::AttackDeclared));
        assert!(types.contains(&GameEventType::AttackCancelled));

        // An older action is still trimmed away: the floor widens the window for the NEWEST action
        // only, so the history before it does not grow without bound (§9.3's `NONCE_HISTORY`).
        state.applied = vec![
            AppliedAction {
                nonce: "old".to_string(),
                events: vec![event(json!({ "type": "turnStarted", "player": "p2", "turn": 1 }))],
            },
            AppliedAction { nonce: "burst".to_string(), events: burst },
        ];
        let with_history = view_for(&state, PlayerId::P1).events;
        assert_eq!(with_history.len(), head.len() + tail.len());
        assert!(!with_history.iter().map(GameEvent::event_type).any(|t| t == GameEventType::TurnStarted));
    }

    /// `drawnFor`: the one `drawn` naming `player` in `viewer`'s events, serialised.
    fn drawn_for(state: &GameState, viewer: PlayerId, player: PlayerId) -> Value {
        let events = view_for(state, viewer).events;
        // Redacted, not truncated: both draws are still in the stream for both viewers.
        assert_eq!(events.len(), 2);
        let found = events
            .iter()
            .find(|event| matches!(event, GameEvent::Drawn { player: drawer, .. } if *drawer == player));
        match found {
            Some(found) => to_json(found),
            None => panic!("no {player} draw in {viewer}'s view"),
        }
    }

    #[test]
    fn r97_redacts_an_event_that_names_a_card_the_viewer_may_not_read_rather_than_dropping_it() {
        let mut state = game("r97-redaction");
        // §11 names the sentinel, so the string itself is part of the ruling.
        assert_eq!(HIDDEN_ID, "hidden");

        let theirs = in_hand(&mut state, &plain().id, PlayerId::P2, None).remove(0);
        let mine = in_hand(&mut state, &plain().id, PlayerId::P1, None).remove(0);
        state.applied = vec![AppliedAction {
            nonce: "r97".to_string(),
            events: vec![
                event(json!({ "type": "drawn", "player": "p2", "instanceId": theirs.id, "defId": theirs.def_id })),
                event(json!({ "type": "drawn", "player": "p1", "instanceId": mine.id, "defId": mine.def_id })),
            ],
        }];

        // While the card sits in p2's hand, p1 gets the cue and neither id.
        assert_eq!(
            drawn_for(&state, PlayerId::P1, PlayerId::P2),
            json!({ "type": "drawn", "player": "p2", "instanceId": HIDDEN_ID, "defId": HIDDEN_ID })
        );
        // p1's own draw is never redacted, so the rule is not "redact everything".
        assert_eq!(
            drawn_for(&state, PlayerId::P1, PlayerId::P1),
            json!({ "type": "drawn", "player": "p1", "instanceId": mine.id, "defId": mine.def_id })
        );
        // And the owner reads their own draw, as the hand rule says.
        expect_match(
            &drawn_for(&state, PlayerId::P2, PlayerId::P2),
            json!({ "instanceId": theirs.id, "defId": theirs.def_id }),
        );

        // R97's history rule: readability is judged by where the card sits *now*. p2 plays it, and the
        // same stored `drawn` event reads openly for p1 — same event, different answer.
        state.players.p2.hand.retain(|card| card.id != theirs.id);
        let mut played = theirs.clone();
        assert!(place_on_field(
            &mut state,
            &mut played,
            slot(PlayerId::P2, Row::Units, 1),
            Default::default(),
        ));
        assert_eq!(
            drawn_for(&state, PlayerId::P1, PlayerId::P2),
            json!({ "type": "drawn", "player": "p2", "instanceId": theirs.id, "defId": theirs.def_id })
        );

        // The other direction: a unit bounced into the enemy hand stops reading the moment it lands.
        let mut bounced = state.players.p2.units[0]
            .take()
            .and_then(|mut pile| if pile.is_empty() { None } else { Some(pile.remove(0)) })
            .expect("the unit p2 just placed");
        bounced.zone = Zone::Hand { player: PlayerId::P2 };
        state.players.p2.hand.push(bounced);
        state.applied = vec![AppliedAction {
            nonce: "r97-bounce".to_string(),
            events: vec![event(json!({
                "type": "bounced", "instanceId": theirs.id, "defId": theirs.def_id, "owner": "p2",
            }))],
        }];
        assert_eq!(
            to_json(&view_for(&state, PlayerId::P1).events[0]),
            json!({ "type": "bounced", "instanceId": HIDDEN_ID, "defId": HIDDEN_ID, "owner": "p2" })
        );
        expect_match(
            &to_json(&view_for(&state, PlayerId::P2).events[0]),
            json!({ "instanceId": theirs.id }),
        );
    }

    #[test]
    fn r97_never_reads_a_card_in_a_library_and_blanks_a_shuffled_in_cards_slot_for_both_players() {
        let mut state = game("r97-library");
        let in_library = state.players.p1.library[3].clone();
        state.applied = vec![AppliedAction {
            nonce: "r97-shuffle".to_string(),
            events: vec![event(json!({
                "type": "shuffledIn",
                "player": "p1",
                "instanceId": in_library.id,
                "defId": in_library.def_id,
                "position": 3,
            }))],
        }];

        // An event never reads a card in a library, its owner included (§9.1): the owner's list (R310)
        // names what is left without saying which instance is which.
        for viewer in [PlayerId::P1, PlayerId::P2] {
            let event = view_for(&state, viewer).events[0].clone();
            assert_eq!(event.event_type(), GameEventType::ShuffledIn);
            let GameEvent::ShuffledIn { instance_id, def_id, position, .. } = event else {
                continue;
            };
            assert_eq!(instance_id, HIDDEN_ID);
            assert_eq!(def_id, HIDDEN_ID);
            // §9.1 hides library order without qualifying by player, so the slot is blanked for both.
            assert_ne!(position, 3);
        }
        let as_owner = to_json(&view_for(&state, PlayerId::P1).events[0]);
        let as_opponent = to_json(&view_for(&state, PlayerId::P2).events[0]);
        assert_eq!(as_owner, as_opponent);
    }

    #[test]
    fn section_10_6_tells_each_player_only_that_a_prompt_is_open_when_it_is_not_theirs() {
        let mut state = game("pending-for");
        with_sink(&mut state, |sink| {
            open_prompt(
                sink,
                OpenPromptArgs {
                    player: PlayerId::P2,
                    kind: PromptKind::Mode,
                    aim: None,
                    prompt: "Choose one".to_string(),
                    options: vec![mode_option("burn"), mode_option("freeze")],
                    min: None,
                    max: None,
                    budget: None,
                    owner: None,
                    resume: inert_resume(),
                },
            )
        });

        assert_eq!(
            to_json(&view_for(&state, PlayerId::P1).pending),
            json!({ "forYou": false, "pendingFor": "p2" })
        );
        assert_eq!(
            own_options(&view_for(&state, PlayerId::P2))
                .options
                .iter()
                .map(|option| option.key.clone())
                .collect::<Vec<_>>(),
            vec!["mode:burn".to_string(), "mode:freeze".to_string()]
        );
        let text = serialized(&view_for(&state, PlayerId::P1));
        assert!(!text.contains("burn"));
        assert!(!text.contains("freeze"));
    }
}

/* ----------------------------------------------------------------------------------------- *
 * R169: the player modifiers (§10.1 `mods`) in the view
 * ----------------------------------------------------------------------------------------- */

mod r169_view_for_player_modifiers_10_1_10_3_modifier_changed {
    use super::*;

    /// Installs a modifier the way a card script does, so the id is the engine's own. `literal` is
    /// TS's `DistributiveOmit<PlayerModifier, "id">`: its `expiry` and, beside it, the kind's fields.
    fn install(state: &mut GameState, player: PlayerId, literal: Value) -> PlayerModifier {
        let mut kind = literal;
        let expiry = kind
            .as_object_mut()
            .and_then(|object| object.remove("expiry"))
            .expect("a modifier literal has an expiry");
        let expiry: ModifierExpiry = json_as(expiry);
        let kind: ModifierKind = json_as(kind);
        with_sink(state, |sink| add_modifier(sink, player, expiry, kind))
    }

    #[test]
    fn r169_carries_both_seats_modifiers_as_id_label_in_order_and_nothing_else() {
        let mut state = game("modifiers-both-seats");
        // #77 Professor Curvature, live: its discount is the controller's next turn (R48).
        let curvature = install(
            &mut state,
            PlayerId::P1,
            json!({
                "kind": "costDiscount",
                "amount": 1,
                "minCurrentCost": 4,
                "expiry": { "until": "nextTurnOf", "player": "p1", "fromTurn": 1 },
            }),
        );
        // #78 /fullsend's two turn-scoped riders, in the order the Cry installs them.
        let turn = state.turn;
        let discount = install(
            &mut state,
            PlayerId::P1,
            json!({ "kind": "costDiscount", "amount": 1, "expiry": { "until": "thisTurn", "turn": turn } }),
        );
        let combo = install(
            &mut state,
            PlayerId::P1,
            json!({ "kind": "comboDraw", "amount": 1, "expiry": { "until": "thisTurn", "turn": turn } }),
        );
        // #79 Twinspell on the other seat.
        let echo = install(
            &mut state,
            PlayerId::P2,
            json!({ "kind": "echoNextSpell", "amount": 1, "expiry": { "until": "used" } }),
        );

        let view = view_for(&state, PlayerId::P1);

        assert_eq!(
            view.you.modifiers.iter().map(|modifier| modifier.id.clone()).collect::<Vec<_>>(),
            vec![curvature.id.clone(), discount.id.clone(), combo.id.clone()]
        );
        assert_eq!(
            view.you.modifiers,
            vec![
                ModifierView { id: curvature.id.clone(), label: "(4)+ Cost cards cost (1) less".to_string() },
                ModifierView { id: discount.id.clone(), label: "Your cards cost 1 less".to_string() },
                ModifierView { id: combo.id.clone(), label: "Your cards gain \"Combo: draw 1\"".to_string() },
            ]
        );
        // §10.8 gives a seat no privacy over its own badges, and `modifierChanged` is already public
        // in both directions, so the opponent's list travels too.
        assert_eq!(
            view.opponent.modifiers,
            vec![ModifierView { id: echo.id.clone(), label: "Next Spell gains Echo +1".to_string() }]
        );

        // The mirror view agrees: each seat sees the same two lists, swapped.
        let theirs = view_for(&state, PlayerId::P2);
        assert_eq!(theirs.you.modifiers, view.opponent.modifiers);
        assert_eq!(theirs.opponent.modifiers, view.you.modifiers);

        // `{ id, label }` and no more: no kind, no amount, no expiry, no source.
        for modifier in view.you.modifiers.iter().chain(view.opponent.modifiers.iter()) {
            let mut keys = keys_of(&to_json(modifier));
            keys.sort();
            assert_eq!(keys, vec!["id".to_string(), "label".to_string()]);
        }
    }

    #[test]
    fn r169_a_board_with_no_modifiers_carries_an_empty_list_on_both_seats() {
        let view = view_for(&game("modifiers-empty"), PlayerId::P1);
        assert_eq!(view.you.modifiers, Vec::<ModifierView>::new());
        assert_eq!(view.opponent.modifiers, Vec::<ModifierView>::new());
    }

    #[test]
    fn r169_r48_say_so_on_the_badge_while_the_modifier_is_installed_but_not_yet_live() {
        let mut state = game("modifiers-dormant");
        // The turn #77 was played: `mana.modifierIsLive` is false, so the discount does nothing yet.
        let turn = state.turn;
        install(
            &mut state,
            PlayerId::P1,
            json!({
                "kind": "costDiscount",
                "amount": 2,
                "minCurrentCost": 4,
                "expiry": { "until": "nextTurnOf", "player": "p1", "fromTurn": turn },
            }),
        );

        // The radiant face of #77, so the number is 2.
        assert_eq!(
            view_for(&state, PlayerId::P1).you.modifiers[0].label,
            "(4)+ Cost cards cost (2) less (next turn)"
        );

        // p1's next turn: the discount bites, and the badge stops hedging.
        state.turn += 2;
        assert_eq!(view_for(&state, PlayerId::P1).you.modifiers[0].label, "(4)+ Cost cards cost (2) less");
    }

    #[test]
    fn r169_labels_every_player_modifier_kind_from_the_modifier_alone() {
        let mut state = game("modifiers-labels");
        let turn = state.turn;
        install(
            &mut state,
            PlayerId::P1,
            json!({
                "kind": "costDiscount",
                "amount": 1,
                "onlyType": "Spell",
                "oncePerTurn": true,
                "expiry": { "until": "thisTurn", "turn": turn },
            }),
        );
        install(
            &mut state,
            PlayerId::P1,
            json!({ "kind": "radiantFirstCheapCard", "maxCost": 1, "expiry": { "until": "never" } }),
        );
        install(&mut state, PlayerId::P1, json!({ "kind": "quickstrikerDamage", "expiry": { "until": "never" } }));

        assert_eq!(
            view_for(&state, PlayerId::P1)
                .you
                .modifiers
                .iter()
                .map(|modifier| modifier.label.clone())
                .collect::<Vec<_>>(),
            vec![
                // §8 #35 Lunar Eclipse: "the next Spell you play this turn costs 1 less".
                "Next Spell costs 1 less".to_string(),
                // §8 #64 Gifted Program.
                "First card costing 1 or less becomes Radiant".to_string(),
                // §8 #38 Quickstriker.
                "Your cards gain \"Combo X: X damage to the enemy hero\"".to_string(),
            ]
        );
    }

    #[test]
    fn r169_a_modifiers_source_id_never_travels_so_a_badge_cannot_name_a_face_down_card_9_1() {
        let d = defs();
        let mut state = game("modifiers-source");
        // #79 Twinspell keeps the instance that installed the rider (R30). Point it at a card p1 may
        // not read at all — a face-down trap in p2's backrow — so a leak would be unmistakable.
        let trap = put(&mut state, &d.secret_trap.id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        install(
            &mut state,
            PlayerId::P2,
            json!({ "kind": "echoNextSpell", "amount": 2, "sourceId": trap.id, "expiry": { "until": "used" } }),
        );

        let view = view_for(&state, PlayerId::P1);
        assert_eq!(
            view.opponent.modifiers,
            vec![ModifierView {
                id: state.players.p2.mods[0].id.clone(),
                label: "Next Spell gains Echo +2".to_string(),
            }]
        );
        assert!(!serialized(&view).contains(&quoted(&trap.id)));
        assert!(!serialized(&view).contains(&trap.def_id));
    }
}
