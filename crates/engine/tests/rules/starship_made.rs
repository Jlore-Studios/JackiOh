//! R3539000–R3539004 (issue #3539): the Starships a card makes, a launch price per Starship, a
//! random launch, a whole-board exile and a Starship yard. Proved on `fixtures/starship_made.rs`.
//!
//! A hull on the field is always put there by `put`, a real arrival: a card the setup writes
//! straight into a zone never starts its Dormant slumber or its Starship memory, so a 0/0 hull
//! would die in the setup check as any 0/0 Unit does.

use jackioh_engine::effects::summon_starships;
use jackioh_engine::subsystems::activate::abilities_of;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{put, sink_for, slot};
use crate::rules::fixtures::starship::*;
use crate::rules::fixtures::starship_made::*;

/// p1's or p2's turn, its main phase, with the given hands and other field cards; both libraries
/// hold plain cards to draw. Neither turn ends by itself, so a played spell leaves its caster in
/// main phase.
fn board(active: &str, p1_hand: Value, p1_field: Value, p2_hand: Value, p2_field: Value) -> Scenario {
    board_with_backrow(active, p1_hand, p1_field, json!([]), p2_hand, p2_field)
}

/// `board`, with p1's backrow set as given.
fn board_with_backrow(
    active: &str,
    p1_hand: Value,
    p1_field: Value,
    p1_backrow: Value,
    p2_hand: Value,
    p2_field: Value,
) -> Scenario {
    register_starship_made_fixtures();
    let library = json!([plain.id, plain.id, plain.id, plain.id, plain.id, plain.id]);
    let mut s = scenario(json!({
        "seed": "r3539000",
        "active": active,
        "p1": { "hand": p1_hand, "field": p1_field, "backrow": p1_backrow, "library": library },
        "p2": { "hand": p2_hand, "field": p2_field, "library": library },
    }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

/// A hull arriving onto p1's lane, Dormant with its memory, as a play or summon would bring it.
fn moor(s: &mut Scenario, def_id: &str, lane: i32) -> String {
    put(s.state_mut(), def_id, slot(P1, Row::Units, lane), Default::default()).id
}

/// `moor`, on the card's Radiant face.
fn moor_radiant(s: &mut Scenario, def_id: &str, lane: i32) -> String {
    put(
        s.state_mut(),
        def_id,
        slot(P1, Row::Units, lane),
        json!({ "radiant": true }),
    )
    .id
}

fn offers_activate(s: &Scenario, player: PlayerId, id: &str) -> bool {
    legal_actions(s.state(), player).iter().any(|action| {
        matches!(action, ActionBody::Activate { instance_id, .. } if instance_id == id)
    })
}

fn ship_of(s: &Scenario, player: PlayerId) -> CardInstance {
    dormant_starship_of(s.state(), player)
        .cloned()
        .unwrap_or_else(|| panic!("{player:?} has no Dormant Starship"))
}

fn piece_defs(card: &CardInstance) -> Vec<String> {
    starship_pieces_of(card)
        .into_iter()
        .map(|piece| piece.def_id)
        .collect()
}

/// Applies one effect as `controller`'s, with no card behind it.
fn apply(s: &mut Scenario, effect: Effect, controller: PlayerId) {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
    {
        let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(controller),
                ..Default::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    s.state_mut().rng_cursor = rng.cursor();
}

#[test]
fn r3539000_a_hull_that_cant_be_launched_by_hand_offers_no_launch() {
    let mut s = board("p1", json!([]), json!([]), json!([]), json!([]));
    let id = moor(&mut s, &sealed.id, 1);
    assert!(is_dormant_starship(s.card(&id)));
    assert!(!offers_activate(&s, P1, &id));
    assert!(abilities_of(s.state(), s.card(&id)).is_empty());
    s.expect_refused_with(
        |s| s.activate(id.as_str(), json!({})),
        "that card has no Activate ability",
    );
}

#[test]
fn r3539000_a_priced_hull_launches_for_its_price_or_a_lower_launch_cost() {
    let mut s = board("p1", json!([vagabond.id]), json!([]), json!([]), json!([]));
    s.play(vagabond.id.as_str(), json!({}));
    let ship = ship_of(&s, P1);
    let decls = abilities_of(s.state(), &ship);
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].label, "Launch (3)");
    let mana = s.state().players.p1.mana.current;
    s.activate(ship.id.as_str(), json!({ "ability": STARSHIP_LAUNCH_ABILITY }));
    assert_eq!(s.state().players.p1.mana.current, mana - 3);
    assert!(is_launched_starship(s.card(&ship.id)));

    let mut s = board(
        "p1",
        json!([cheap.id, vagabond.id]),
        json!([]),
        json!([]),
        json!([]),
    );
    s.play(cheap.id.as_str(), json!({}));
    s.play(vagabond.id.as_str(), json!({}));
    let ship = ship_of(&s, P1);
    let decls = abilities_of(s.state(), &ship);
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].label, "Launch (1)");
    let mana = s.state().players.p1.mana.current;
    s.activate(ship.id.as_str(), json!({ "ability": STARSHIP_LAUNCH_ABILITY }));
    assert_eq!(s.state().players.p1.mana.current, mana - 1);
    assert!(is_launched_starship(s.card(&ship.id)));
    assert!(
        !s.state().players.p1.mods.iter().any(|modifier| matches!(
            modifier.kind,
            ModifierKind::StarshipLaunchCost { .. }
        )),
        "the launch spends the launch-cost modifier"
    );
}

#[test]
fn r3539000_a_hull_whose_text_names_its_activate_offers_that_one_alone() {
    let mut s = board("p1", json!([]), json!([]), json!([]), json!([]));
    let id = moor(&mut s, &yard.id, 1);
    let ids: Vec<String> = abilities_of(s.state(), s.card(&id))
        .into_iter()
        .map(|decl| decl.id)
        .collect();
    assert_eq!(ids, vec!["yard".to_string()]);
}

#[test]
fn r3539001_made_starships_stand_backrow_middle_first_holding_their_pieces() {
    let mut s = board("p1", json!([maker.id]), json!([]), json!([]), json!([]));
    s.play(maker.id.as_str(), json!({}));
    for lane in [3, 2] {
        let id = s
            .backrow(P1, lane)
            .unwrap_or_else(|| panic!("a made Starship stands in backrow lane {lane}"))
            .id;
        let ship = s.card(&id);
        assert!(is_dormant_starship(ship), "it is a Dormant Starship");
        assert_eq!(piece_defs(ship), vec![booster.id.clone(), gunner.id.clone()]);
        s.expect_stats(id.as_str(), json!({ "attack": 3, "health": 4 }));
    }
}

#[test]
fn r3539001_made_starships_never_repeat_until_every_blueprint_is_out() {
    let mut s = board("p1", json!([]), json!([]), json!([]), json!([]));
    let cursor = s.state().rng_cursor;
    apply(
        &mut s,
        summon_starships(json_as(json!({
            "blueprints": [
                { "defId": sealed.id, "pieces": [booster.id] },
                { "defId": yard.id, "pieces": [cell_a.id] },
            ],
            "count": 3,
        }))),
        P1,
    );
    assert_ne!(s.state().rng_cursor, cursor, "the picks draw");
    // The summons land in backrow lanes 3, 2 and 4, in that order.
    let holding = |s: &Scenario, lane: i32| -> Vec<String> {
        let id = s
            .backrow(P1, lane)
            .unwrap_or_else(|| panic!("a made Starship stands in backrow lane {lane}"))
            .id;
        piece_defs(s.card(&id))
    };
    let (first, second, third) = (holding(&s, 3), holding(&s, 2), holding(&s, 4));
    assert_ne!(first, second, "no blueprint repeats until every one is out");
    assert!(third == first || third == second, "the third repeats the round");
}

#[test]
fn r3539001_no_open_zone_summons_nothing_and_draws_nothing() {
    let units: Vec<Value> = (1..=5)
        .map(|lane| json!({ "def": plain.id, "lane": lane }))
        .collect();
    let backrow: Vec<Value> = (1..=5)
        .map(|lane| json!({ "def": countdown.id, "lane": lane }))
        .collect();
    let mut s = board_with_backrow("p1", json!([]), json!(units), json!(backrow), json!([]), json!([]));
    let cursor = s.state().rng_cursor;
    apply(
        &mut s,
        summon_starships(json_as(json!({
            "blueprints": [{ "defId": sealed.id, "pieces": [booster.id] }],
            "count": 1,
        }))),
        P1,
    );
    assert_eq!(s.state().rng_cursor, cursor, "no zone, no draw");
    assert!(dormant_starships_of(s.state(), P1).is_empty());
}

#[test]
fn r3539002_exile_board_takes_dormant_cards_and_whole_piles_but_spares_an_immune_card() {
    let mut s = board(
        "p1",
        json!([wipe.id]),
        json!([
            { "def": plain.id, "lane": 2 },
            { "def": lander.id, "lane": 2, "stack": true },
            { "def": shield.id, "lane": 3 },
        ]),
        json!([]),
        json!([]),
    );
    let sealed_id = moor(&mut s, &sealed.id, 1);
    s.play(wipe.id.as_str(), json!({}));
    assert!(s.unit(P1, 1).is_none(), "the Dormant card goes too");
    assert!(s.unit(P1, 2).is_none(), "the whole pile goes");
    assert!(s.unit(P1, 3).is_some(), "the immune card stays");
    let exile: Vec<String> = s.pile(P1, "exile").into_iter().map(|card| card.id).collect();
    assert!(exile.contains(&sealed_id), "the Dormant card reaches exile");
    assert_eq!(exile.len(), 3, "sealed, plain and lander, nothing else");
}

#[test]
fn r3539002_launch_random_starship_launches_only_a_named_hull() {
    let mut s = board("p1", json!([voyage.id]), json!([]), json!([]), json!([]));
    let sealed_id = moor(&mut s, &sealed.id, 1);
    let hull_id = moor(&mut s, &hull.id, 2);
    // A launched hull with no pieces would die at 0/0, awake and judged like any Unit (R3539003
    // judges only Dormant cards): fuse a piece into the named hull first.
    {
        let mut sink = sink_for(s.state_mut());
        fuse_fresh_piece_into(&mut sink, &sealed_id, &cell_a.id, false);
    }
    s.play(voyage.id.as_str(), json!({}));
    assert!(is_launched_starship(s.card(&sealed_id)), "the named hull launches");
    assert!(
        is_dormant_starship(s.card(&hull_id)),
        "the other Dormant Starship stays Dormant"
    );
}

#[test]
fn r3539003_a_dormant_0_0_card_survives_the_state_check() {
    let mut s = board("p1", json!([]), json!([]), json!([]), json!([]));
    let id = moor(&mut s, &yard.id, 1);
    s.end_turn();
    assert_eq!(s.card(&id).zone.z(), ZoneName::Field, "no health judges it");
}

#[test]
fn r3539004_a_yard_fuses_one_random_piece_at_each_start_of_its_controllers_turn() {
    let mut s = board("p1", json!([]), json!([]), json!([]), json!([]));
    let id = moor_radiant(&mut s, &yard.id, 1);
    assert!(piece_defs(s.card(&id)).is_empty());
    s.end_turn();
    s.end_turn();
    let first = piece_defs(s.card(&id));
    assert_eq!(first.len(), 1, "one random piece at the first start of turn");
    assert!([cell_a.id.clone(), cell_b.id.clone()].contains(&first[0]));
    let records = starship_pieces_of(s.card(&id));
    assert_eq!(records[0].radiant, Some(true), "a Radiant yard fuses Radiant");
    s.end_turn();
    s.end_turn();
    assert_eq!(piece_defs(s.card(&id)).len(), 2, "another at the next start of turn");
}

#[test]
fn r3539004_the_yards_activate_exiles_it_and_launches_a_fresh_starship_holding_its_pieces() {
    let mut s = board("p1", json!([]), json!([]), json!([]), json!([]));
    let id = moor(&mut s, &yard.id, 1);
    s.end_turn();
    s.end_turn();
    let held = piece_defs(s.card(&id));
    assert_eq!(held.len(), 1);
    let mana = s.state().players.p1.mana.current;
    s.activate(id.as_str(), json!({ "ability": "yard" }));
    assert_eq!(s.state().players.p1.mana.current, mana, "the launch is free");
    assert_eq!(s.card(&id).zone.z(), ZoneName::Exile, "the shipyard is exiled");
    let launched = launched_starship_of(s.state(), P1)
        .cloned()
        .expect("a fresh Starship launched");
    assert_eq!(hull_def_of(s.state(), &launched.def_id), DORMANT_STARSHIP_DEF_ID);
    assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(launched.id.clone()));
    assert_eq!(piece_defs(&launched), held, "holding its pieces, oldest first");
    assert!(is_launched_starship(&launched));
}

#[test]
fn r3539004_the_yards_activate_is_refused_while_it_holds_no_piece() {
    let mut s = board("p1", json!([]), json!([]), json!([]), json!([]));
    let id = moor(&mut s, &yard.id, 1);
    s.expect_refused_with(
        |s| s.activate(id.as_str(), json!({ "ability": "yard" })),
        "that ability can't be activated now",
    );
}
