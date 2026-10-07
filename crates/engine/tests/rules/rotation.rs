// Silly Silas's rotation (SPEC §3.1's rotation-topology ruling, R14, §8 #52; BUILD M3-T7).
// The fixture cards these tests need are defined here and registered on top of the shared fixture
// catalog, so no shared fixture has to grow for them (CLAUDE.md, BUILD §0).
//
// Port of `packages/engine/test/rotation.test.ts`.

use jackioh_engine::subsystems::rotation::{RotationResult, rotate_rings};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, new_game, put, slot};

// ---------------------------------------------------------------------------
// Fixture cards.
// ---------------------------------------------------------------------------

/// TS's module `let nextIndex = 700`, written out: each def takes the index it had.
fn unit_def_of(name: &str, index: u32, attack: i32, health: i32, keywords: Value) -> CardDef {
    json_as(json!({
        "id": format!("rot-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (rotation)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": attack, "health": health, "keywords": keywords, "text": name },
        "radiant": { "attack": attack * 2, "health": health * 2, "keywords": keywords, "text": format!("{name} radiant") },
    }))
}

/// A plain body, the control case for every rotation test.
fn plain() -> CardDef {
    unit_def_of("plain", 701, 2, 2, json!([]))
}
/// #52 itself: it is on the field when its Cry resolves, so it rotates with everything else.
fn silas() -> CardDef {
    unit_def_of("silas", 702, 4, 4, json!([]))
}
/// §3.2: a Stack card, so a whole pile can be rotated.
fn stacker() -> CardDef {
    unit_def_of("stack", 703, 3, 3, json!([{ "kind": "Stack" }]))
}
/// A backrow card for the second ring; face-down until it fires (§3, R33).
fn trap() -> CardDef {
    let mut def = unit_def_of("trap", 704, 0, 0, json!([]));
    def.id = "rot-trap".into();
    def.type_ = CardType::Trap;
    def.base = json_as(json!({ "keywords": [], "text": "trap" }));
    def.radiant = json_as(json!({ "keywords": [], "text": "trap radiant" }));
    def
}

fn defs() -> Vec<CardDef> {
    vec![plain(), silas(), stacker(), trap()]
}

/// The shared fixture unit token (R11).
const TOKEN_ID: &str = "fx-token-rush";

/// A Stack card pushed onto an occupied unit zone (§3.2); the harness's `put` fills empty zones.
fn stack_onto(state: &mut GameState, def_id: &str, player: PlayerId, lane: i32) -> CardInstance {
    let mut card = new_instance(state, def_id, player, Zone::Hand { player });
    let ok = place_on_field(
        state,
        &mut card,
        slot(player, Row::Units, lane),
        json_as(json!({ "stack": true })),
    );
    assert!(ok);
    card
}

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    state.turn = 3;
    state.active = PlayerId::P1;
    state
}

/// TS `rotate(state, direction, { perspective?, radiant? })`: one `rotateRings` over a fresh sink.
fn rotate(state: &mut GameState, direction: &str, options: Value) -> (Vec<GameEvent>, RotationResult) {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut args = json!({
        "direction": direction,
        "perspective": options.get("perspective").cloned().unwrap_or_else(|| json!("p1")),
    });
    if let Some(radiant) = options.get("radiant") {
        args["radiant"] = radiant.clone();
    }
    let result = rotate_rings(&mut EngineSink::new(state, &mut events, &mut rng), &json_as(args));
    (events, result)
}

/// Where a card sits now, as "p2 units 5", for readable assertions.
fn where_is(state: &GameState, card: &CardInstance) -> String {
    match find_instance(state, &card.id).map(|found| &found.zone) {
        None => "gone".to_string(),
        Some(Zone::Field { player, row, lane }) => format!("{player} {row} {lane}"),
        Some(zone) => zone.z().to_string(),
    }
}

/// The card as it stands in the state now (TS held the live object).
fn live<'a>(state: &'a GameState, card: &CardInstance) -> &'a CardInstance {
    find_instance(state, &card.id).expect("the card is still in the game")
}

fn live_mut<'a>(state: &'a mut GameState, card: &CardInstance) -> &'a mut CardInstance {
    find_instance_mut(state, &card.id).expect("the card is still in the game")
}

fn of_type(events: &[GameEvent], kind: GameEventType) -> Vec<Value> {
    events_of_type(events, kind)
        .into_iter()
        .map(|event| serde_json::to_value(event).unwrap())
        .collect()
}

fn field_of(events: &[GameEvent], kind: GameEventType, field: &str) -> Vec<Value> {
    of_type(events, kind)
        .into_iter()
        .map(|event| event[field].clone())
        .collect()
}

fn ids(cards: &[&CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

mod r14_rotation_m3_t7 {
    use super::*;

    #[test]
    fn r14_rotates_the_unit_ring_one_step_right_so_your_lane_5_crosses_to_the_opponents_lane_5() {
        let mut state = game("rotate-right");
        let lane1 = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 1), Default::default());
        let lane5 = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 5), Default::default());

        let (events, result) = rotate(&mut state, "right", json!({}));

        assert_eq!(where_is(&state, &lane1), "p1 units 2");
        assert_eq!(where_is(&state, &lane5), "p2 units 5");
        assert!(card_at(&state, slot(PlayerId::P1, Row::Units, 1)).is_none());
        assert_eq!(result.moved, ids(&[&lane1, &lane5]));

        // Control changes only for the card that crossed the centre line (§3.1).
        assert_eq!(live(&state, &lane1).controller, PlayerId::P1);
        assert_eq!(live(&state, &lane5).controller, PlayerId::P2);
        assert_eq!(result.crossed, ids(&[&lane5]));
        assert!(result.bounced.is_empty());

        // One `rotated` for the rotation, whatever moved (§10.3).
        assert_eq!(
            of_type(&events, GameEventType::Rotated),
            vec![json!({ "type": "rotated", "direction": "right" })]
        );
        assert_eq!(
            of_type(&events, GameEventType::ControlChanged),
            vec![json!({ "type": "controlChanged", "instanceId": lane5.id, "controller": "p2", "row": "units", "lane": 5 })]
        );
        assert!(of_type(&events, GameEventType::Bounced).is_empty());
    }

    #[test]
    fn r14_rotates_left_as_the_mirror_of_right_so_your_lane_1_crosses_to_the_opponents_lane_1() {
        let mut state = game("rotate-left");
        let lane1 = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 1), Default::default());
        let lane3 = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 3), Default::default());

        let (events, result) = rotate(&mut state, "left", json!({}));

        assert_eq!(where_is(&state, &lane1), "p2 units 1");
        assert_eq!(where_is(&state, &lane3), "p1 units 2");
        assert_eq!(live(&state, &lane1).controller, PlayerId::P2);
        assert_eq!(result.crossed, ids(&[&lane1]));
        assert_eq!(
            of_type(&events, GameEventType::Rotated),
            vec![json!({ "type": "rotated", "direction": "left" })]
        );

        // And one step back the other way puts the card that stayed home where it started.
        rotate(&mut state, "right", json!({}));
        assert_eq!(where_is(&state, &lane3), "p1 units 3");
    }

    #[test]
    fn r14_turns_the_backrow_ring_independently_of_the_unit_ring() {
        let mut state = game("rotate-both-rings");
        let unit = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 3), Default::default());
        let back = put(&mut state, &trap().id, slot(PlayerId::P1, Row::Backrow, 5), Default::default());
        let enemy_back = put(&mut state, &trap().id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());

        let (events, result) = rotate(&mut state, "right", json!({}));

        // The unit ring turned one step and so did the backrow ring, each on its own zones.
        assert_eq!(where_is(&state, &unit), "p1 units 4");
        assert_eq!(where_is(&state, &back), "p2 backrow 5");
        assert_eq!(where_is(&state, &enemy_back), "p1 backrow 1");
        assert_eq!(live(&state, &back).controller, PlayerId::P2);
        assert_eq!(live(&state, &enemy_back).controller, PlayerId::P1);
        assert_eq!(result.crossed, ids(&[&back, &enemy_back]));
        assert_eq!(of_type(&events, GameEventType::Rotated).len(), 1);
        assert_eq!(
            field_of(&events, GameEventType::ControlChanged, "row"),
            vec![json!("backrow"), json!("backrow")]
        );
    }

    #[test]
    fn r12_a_rotated_card_changes_controller_but_never_owner_and_still_leaves_to_its_owners_zones() {
        let mut state = game("rotate-ownership");
        let card = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 5), Default::default());

        rotate(&mut state, "right", json!({}));
        assert_eq!(live(&state, &card).controller, PlayerId::P2);
        assert_eq!(live(&state, &card).owner, PlayerId::P1);

        // Off the field a card always belongs to its owner (R12, §3.2).
        let mut moving = live(&state, &card).clone();
        move_to_zone(&mut state, &mut moving, OffFieldZone::Graveyard, Default::default());
        let graveyard: Vec<String> = state.players.p1.graveyard.iter().map(|c| c.id.clone()).collect();
        assert_eq!(graveyard, vec![card.id.clone()]);
        assert_eq!(state.players.p2.graveyard.len(), 0);
        assert_eq!(live(&state, &card).controller, PlayerId::P1);
    }

    #[test]
    fn r14_damage_and_buffs_travel_with_a_rotated_card() {
        let mut state = game("rotate-keeps-state");
        let card = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 5), Default::default());
        let turn = state.turn;
        {
            let it = live_mut(&mut state, &card);
            it.damage = 1;
            it.buffs = AttackHealth { attack: 3, health: 4 };
            it.granted_keywords = vec![Keyword::Taunt];
            it.counters = json_as(json!({ "plague": 2 }));
            it.position = Some(Position::Def);
            it.summoned_turn = Some(turn - 1);
            it.exertion = Exertion {
                attacked: true,
                switched: false,
                attacks: None,
            };
        }

        rotate(&mut state, "right", json!({}));

        assert_eq!(where_is(&state, &card), "p2 units 5");
        let it = live(&state, &card);
        assert_eq!(it.damage, 1);
        assert_eq!(it.buffs, AttackHealth { attack: 3, health: 4 });
        assert_eq!(serde_json::to_value(&it.counters).unwrap(), json!({ "plague": 2 }));
        assert_eq!(it.granted_keywords, vec![Keyword::Taunt]);
        // A rotation never takes the card off the field, so R78's reset never runs.
        let view = unit_view(&state, it);
        assert_eq!(view.attack, 5);
        assert_eq!(view.max_health, 6);
        assert_eq!(view.health, 5);
        assert_eq!(view.position, Position::Def);
        // R171: what does not travel across the centre line is readiness. The crossing is an entry on
        // this turn, with a fresh exertion for the new controller.
        assert_eq!(it.summoned_turn, Some(3));
        assert_eq!(
            serde_json::to_value(it.exertion).unwrap(),
            json!({ "attacked": false, "switched": false })
        );
    }

    #[test]
    fn r14_bounces_a_card_whose_destination_is_locked_to_its_owners_hand() {
        let mut state = game("rotate-locked");
        let crossing = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 5), Default::default());
        {
            let it = live_mut(&mut state, &crossing);
            it.damage = 1;
            it.buffs = AttackHealth { attack: 2, health: 2 };
        }
        let staying = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 1), Default::default());
        lock_zone(&mut state, slot(PlayerId::P2, Row::Units, 5));

        let (events, result) = rotate(&mut state, "right", json!({}));

        assert_eq!(where_is(&state, &crossing), "hand");
        assert!(state.players.p1.hand.iter().any(|c| c.id == crossing.id));
        assert!(card_at(&state, slot(PlayerId::P2, Row::Units, 5)).is_none());
        assert_eq!(result.bounced, ids(&[&crossing]));
        assert!(result.crossed.is_empty());
        assert_eq!(
            of_type(&events, GameEventType::Bounced),
            vec![json!({ "type": "bounced", "instanceId": crossing.id, "defId": plain().id, "owner": "p1" })]
        );
        assert!(of_type(&events, GameEventType::ControlChanged).is_empty());
        // It left the field, so it resets on the way to the hand (R78), and costs its printed price.
        let it = live(&state, &crossing);
        assert_eq!(it.damage, 0);
        assert_eq!(it.buffs, AttackHealth { attack: 0, health: 0 });
        assert!(it.cost_override.is_none());

        // The rest of the ring still turned.
        assert_eq!(where_is(&state, &staying), "p1 units 2");
    }

    #[test]
    fn r14_radiant_silly_silas_bounces_the_cards_that_would_cross_to_the_opponent_at_cost_0_and_takes_the_ones_crossing_to_its_side()
     {
        let mut state = game("rotate-radiant");
        let mine = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 5), Default::default());
        let theirs = put(&mut state, &plain().id, slot(PlayerId::P2, Row::Units, 1), Default::default());
        let staying = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 2), Default::default());

        let (events, result) = rotate(&mut state, "right", json!({ "radiant": true }));

        // §8 #52 radiant: "cards that would move to the opponent are bounced to their owner's hand
        // costing 0 instead". p1's lane-5 card would move to p2, so it goes home at 0 (R12).
        assert_eq!(where_is(&state, &mine), "hand");
        assert!(state.players.p1.hand.iter().any(|c| c.id == mine.id));
        assert_eq!(live(&state, &mine).cost_override, Some(0));
        assert_eq!(result.bounced, ids(&[&mine]));
        assert_eq!(
            field_of(&events, GameEventType::Bounced, "instanceId"),
            vec![json!(mine.id)]
        );

        // p2's lane-1 card moves to p1, not to the opponent: the base clause holds, so it crosses and
        // changes control, entering p1's side this turn (R171) and keeping its owner (R12).
        assert_eq!(where_is(&state, &theirs), "p1 units 1");
        let crossed = live(&state, &theirs);
        assert_eq!(crossed.controller, PlayerId::P1);
        assert_eq!(crossed.owner, PlayerId::P2);
        assert_eq!(crossed.summoned_turn, Some(state.turn));
        assert!(crossed.cost_override.is_none());
        assert_eq!(result.crossed, ids(&[&theirs]));
        assert_eq!(
            field_of(&events, GameEventType::ControlChanged, "instanceId"),
            vec![json!(theirs.id)]
        );

        // A card that stays on its own side rotates as usual, at its printed cost.
        assert_eq!(where_is(&state, &staying), "p1 units 3");
        // Ring order from p1's seat: p1's lanes 1 to 5, then p2's 5 down to 1 (§3.1).
        assert_eq!(result.moved, ids(&[&staying, &theirs]));
        assert!(live(&state, &staying).cost_override.is_none());
    }

    #[test]
    fn section_8_c52_silas_rotates_along_with_everything_else() {
        let mut state = game("rotate-silas");
        let me = put(&mut state, &silas().id, slot(PlayerId::P1, Row::Units, 3), Default::default());
        let ally = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 4), Default::default());
        let back = put(&mut state, &trap().id, slot(PlayerId::P1, Row::Backrow, 3), Default::default());

        let (_, result) = rotate(&mut state, "right", json!({}));

        assert_eq!(where_is(&state, &me), "p1 units 4");
        assert_eq!(where_is(&state, &ally), "p1 units 5");
        assert_eq!(where_is(&state, &back), "p1 backrow 4");
        assert_eq!(result.moved, ids(&[&me, &ally, &back]));
    }

    #[test]
    fn r14_rotating_a_full_ring_is_atomic_every_card_moves_one_step_and_none_is_overwritten() {
        let mut state = game("rotate-full-ring");
        let mut placed: IndexMap<String, String> = IndexMap::new();
        for player in [PlayerId::P1, PlayerId::P2] {
            for lane in [1, 2, 3, 4, 5] {
                let card = put(&mut state, &plain().id, slot(player, Row::Units, lane), Default::default());
                placed.insert(format!("{player} units {lane}"), card.id);
            }
        }

        let (_, result) = rotate(&mut state, "right", json!({}));

        // The ring is p1 lanes 1-5 then p2 lanes 5-1, so every card is one step along it (§3.1).
        let expected = [
            ("p1 units 1", "p1 units 2"),
            ("p1 units 2", "p1 units 3"),
            ("p1 units 3", "p1 units 4"),
            ("p1 units 4", "p1 units 5"),
            ("p1 units 5", "p2 units 5"),
            ("p2 units 5", "p2 units 4"),
            ("p2 units 4", "p2 units 3"),
            ("p2 units 3", "p2 units 2"),
            ("p2 units 2", "p2 units 1"),
            ("p2 units 1", "p1 units 1"),
        ];
        for (from, to) in expected {
            let card = placed.get(from).and_then(|id| find_instance(&state, id));
            assert!(card.is_some(), "{from}");
            let at = card.map_or_else(|| "gone".to_string(), |card| where_is(&state, card));
            assert_eq!(at, to, "{from}");
        }

        // Ten cards in, ten cards out: nothing was overwritten and nothing was bounced.
        assert_eq!(result.moved.len(), 10);
        assert!(result.bounced.is_empty());
        assert_eq!(result.crossed.len(), 2);
        assert_eq!(state.players.p1.hand.len(), 0);
        assert_eq!(state.players.p2.hand.len(), 0);
    }

    #[test]
    fn r33_a_face_down_trap_that_crosses_answers_to_its_new_controller_and_stays_face_down() {
        let mut state = game("rotate-trap-visibility");
        let card = put(&mut state, &trap().id, slot(PlayerId::P1, Row::Backrow, 5), Default::default());
        assert!(live(&state, &card).face_up.is_none());

        let (events, _) = rotate(&mut state, "right", json!({}));

        assert_eq!(where_is(&state, &card), "p2 backrow 5");
        assert_eq!(live(&state, &card).controller, PlayerId::P2);
        assert_eq!(live(&state, &card).owner, PlayerId::P1);
        // Who may read it follows from the controller alone, so the card is still face down (R33).
        assert!(live(&state, &card).face_up.is_none());
        assert_eq!(
            of_type(&events, GameEventType::ControlChanged),
            vec![json!({ "type": "controlChanged", "instanceId": card.id, "controller": "p2", "row": "backrow", "lane": 5 })]
        );
    }

    #[test]
    fn section_3_2_a_stack_pile_rotates_whole_keeping_the_same_card_on_top() {
        let mut state = game("rotate-stack");
        let under = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 5), Default::default());
        live_mut(&mut state, &under).damage = 1;
        let top = stack_onto(&mut state, &stacker().id, PlayerId::P1, 5);

        let (_, result) = rotate(&mut state, "right", json!({}));

        assert!(pile_at(&state, slot(PlayerId::P1, Row::Units, 5)).is_none());
        let pile: Option<Vec<String>> =
            pile_at(&state, slot(PlayerId::P2, Row::Units, 5)).map(|pile| pile.iter().map(|c| c.id.clone()).collect());
        assert_eq!(pile, Some(vec![top.id.clone(), under.id.clone()]));
        assert_eq!(
            card_at(&state, slot(PlayerId::P2, Row::Units, 5)).map(|c| c.id.clone()),
            Some(top.id.clone())
        );
        // The dormant card came along and kept its damage (§3.2, R14).
        assert_eq!(live(&state, &under).damage, 1);
        assert_eq!(live(&state, &top).controller, PlayerId::P2);
        assert_eq!(live(&state, &under).controller, PlayerId::P2);
        assert_eq!(result.crossed, ids(&[&top, &under]));
        // Nothing beneath the top resumed, so no Stack note is kept against it (R212, `withPile`).
        let note = state
            .field_exits
            .as_ref()
            .and_then(|exits| exits.uncovered.as_ref())
            .and_then(|uncovered| uncovered.get(&top.id));
        assert!(note.is_none());
    }

    #[test]
    fn r11_a_unit_token_bounced_by_a_locked_destination_ceases_to_exist() {
        let mut state = game("rotate-token-bounce");
        let token = put(&mut state, TOKEN_ID, slot(PlayerId::P1, Row::Units, 5), Default::default());
        lock_zone(&mut state, slot(PlayerId::P2, Row::Units, 5));

        let (events, result) = rotate(&mut state, "right", json!({}));

        assert_eq!(result.bounced, ids(&[&token]));
        assert_eq!(
            field_of(&events, GameEventType::Bounced, "instanceId"),
            vec![json!(token.id)]
        );
        assert!(find_instance(&state, &token.id).is_none());
        assert_eq!(state.players.p1.hand.len(), 0);
        assert_eq!(state.players.p1.graveyard.len(), 0);
    }
}
