//! R4205000 (issue #4205): a steal that reaches a card on the field but has no room for it on the thief's
//! side (R15; R3546003's GIGA fit, in `giga.rs`) destroys it where it stands, and nothing prevents that
//! destroy: not Indestructible (R46), not Untributable (R1220), no "would die" replacement (R462's
//! reading), though a destroy-to-exile aura still exiles it (R1849002), where it has died all the same
//! (R4207000). Its Death and Reborn apply on its own side and it goes to its controller's graveyard
//! (R2618000). A give leaves such a card (R803), and a card no steal reaches (R13, R1839000) is neither
//! taken nor destroyed. Fixture cards from `fixtures/stinkyscape_verbs.rs`.

use jackioh_engine::effects::{give_control, steal, steal_all};
use jackioh_engine::state_check::state_check;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::stinkyscape_verbs::*;

/// p1's turn 9, its main phase; each side's setup as given, with plain cards to draw.
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_stinkyscape_verbs();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([PLAIN, PLAIN, PLAIN, PLAIN]);
        }
    }
    let mut s = scenario(json!({ "seed": seed, "active": "p1", "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

/// p1's unit row, full.
fn full_row() -> Value {
    json!([GRUNT, GRUNT, GRUNT, GRUNT, GRUNT])
}

/// Apply `effect` for `controller` with no card running it, then run §4.5's state check as a resolving
/// card's effects are settled; the events of both.
fn settled(s: &mut Scenario, effect: Effect, controller: PlayerId) -> Vec<GameEvent> {
    let state = s.state_mut();
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        {
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
        state_check(&mut sink);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn steal_of(id: &str) -> Effect {
    steal(json_as(json!({ "instanceId": id })))
}

/// The ids of the `destroyed` events among `events`, in order.
fn destroyed(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Destroyed { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

fn control_changed(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::ControlChanged { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

fn row_ids(s: &Scenario, player: PlayerId) -> Vec<Option<String>> {
    (1..=5)
        .map(|lane| s.unit(player, lane).map(|card| card.id))
        .collect()
}

#[test]
fn r4205000_r2618000_a_steal_onto_a_full_row_destroys_the_target_into_its_controllers_graveyard() {
    let mut s = board(
        "r4205000-full-row",
        json!({ "field": full_row() }),
        json!({ "field": [PLAIN] }),
    );
    let target = s.unit_at(P2, 1);
    let row = row_ids(&s, P1);

    let events = settled(&mut s, steal_of(&target.id), P1);

    s.expect_in_zone(&target, "graveyard");
    assert!(s.pile(P2, "graveyard").iter().any(|card| card.id == target.id));
    assert_eq!(destroyed(&events), vec![target.id.clone()]);
    assert_eq!(control_changed(&events), Vec::<String>::new());
    assert_eq!(row_ids(&s, P1), row, "nothing landed on p1's side");
}

#[test]
fn r4205000_its_death_fires_and_reborn_returns_it_on_its_own_side() {
    let mut s = board(
        "r4205000-death",
        json!({ "field": full_row() }),
        json!({ "field": [PHOENIX] }),
    );
    let target = s.unit_at(P2, 1);
    let hand_before = s.hand(P2).len();

    let events = settled(&mut s, steal_of(&target.id), P1);

    assert!(events.iter().any(|event| matches!(
        event,
        GameEvent::Destroyed { instance_id, controller: P2, .. } if *instance_id == target.id
    )));
    assert_eq!(s.hand(P2).len(), hand_before + 1, "its Death drew for p2");
    let back = s.unit(P2, 1).expect("Reborn returns it to p2's lane 1");
    assert_eq!(back.def_id, PHOENIX);
    assert_eq!(back.controller, P2);
    assert_eq!(back.reborn_spent, Some(true));
}

#[test]
fn r4205000_r46_an_indestructible_target_is_destroyed_not_knocked_down() {
    let mut s = board(
        "r4205000-indestructible",
        json!({ "field": full_row() }),
        json!({ "field": [PLAIN] }),
    );
    let target = s.unit_at(P2, 1);
    s.card_mut(&target).granted_keywords = vec![json_as(json!({ "kind": "Indestructible" }))];
    assert!(unit_has(s.state(), s.card(&target), KeywordKind::Indestructible));

    let events = settled(&mut s, steal_of(&target.id), P1);

    s.expect_in_zone(&target, "graveyard");
    assert_eq!(destroyed(&events), vec![target.id.clone()]);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, GameEvent::PositionSwitched { .. })),
        "no R46 knock-down"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, GameEvent::KeywordGranted { lost: Some(true), .. })),
        "no Taunt suppressed"
    );
}

#[test]
fn r4205000_r1220_an_untributable_target_is_destroyed() {
    let mut s = board(
        "r4205000-untributable",
        json!({ "field": full_row() }),
        json!({ "field": [PLAIN] }),
    );
    let target = s.unit_at(P2, 1);
    s.card_mut(&target).granted_keywords = vec![json_as(json!({ "kind": "Untributable" }))];
    assert!(is_untributable(s.state(), s.card(&target)));

    let events = settled(&mut s, steal_of(&target.id), P1);

    s.expect_in_zone(&target, "graveyard");
    assert_eq!(destroyed(&events), vec![target.id.clone()]);
}

#[test]
fn r4205000_r68_steal_all_takes_what_fits_and_destroys_the_rest_in_board_order() {
    let mut s = board(
        "r4205000-steal-all",
        json!({ "field": [GRUNT, GRUNT, GRUNT] }),
        json!({ "field": [PLAIN, PLAIN, PLAIN, PLAIN, PLAIN] }),
    );
    let theirs: Vec<String> = (1..=5).map(|lane| s.unit_at(P2, lane).id).collect();

    let events = settled(&mut s, steal_all(Default::default()), P1);

    assert_eq!(control_changed(&events), theirs[..2].to_vec());
    assert_eq!(s.unit(P1, 4).map(|card| card.id), Some(theirs[0].clone()));
    assert_eq!(s.unit(P1, 5).map(|card| card.id), Some(theirs[1].clone()));
    assert_eq!(destroyed(&events), theirs[2..].to_vec());
    for id in &theirs[2..] {
        s.expect_in_zone(id.as_str(), "graveyard");
    }
}

#[test]
fn r4205000_r1839000_r13_a_dormant_card_and_a_card_beneath_a_stack_are_neither_stolen_nor_destroyed() {
    let mut s = board(
        "r4205000-unreached",
        json!({ "field": full_row() }),
        json!({ "field": [
            { "def": PLAIN, "lane": 1, "slumber": {} },
            { "def": PLAIN, "lane": 2 },
            { "def": GRUNT, "lane": 2, "stack": true },
        ] }),
    );
    let dormant = s.unit_at(P2, 1);
    let top = s.unit_at(P2, 2);
    let beneath = s.state().players[P2].units[1]
        .as_ref()
        .and_then(|pile| pile.get(1))
        .expect("setup: a card beneath p2's lane 2 pile")
        .clone();

    for card in [&dormant, &beneath] {
        let events = settled(&mut s, steal_of(&card.id), P1);
        assert_eq!(events, vec![], "no steal reaches {}", card.def_id);
        assert_eq!(s.card(card).controller, P2);
        assert_eq!(s.card(card).marked_destroyed, None);
    }
    assert_eq!(s.unit(P2, 1).map(|card| card.id), Some(dormant.id.clone()));

    let events = settled(&mut s, steal_all(Default::default()), P1);
    assert_eq!(
        destroyed(&events),
        vec![top.id.clone()],
        "only the pile's top is reached"
    );
    s.expect_in_zone(&dormant, "field");
    s.expect_in_zone(&beneath, "field");
    assert_eq!(s.card(&dormant).controller, P2);
}

#[test]
fn r4205000_r803_give_control_onto_a_full_row_still_leaves_the_card_where_it_is() {
    let mut s = board(
        "r4205000-give",
        json!({ "field": [PLAIN] }),
        json!({ "field": full_row() }),
    );
    let gift = s.unit_at(P1, 1);

    let events = settled(
        &mut s,
        give_control(json_as(json!({ "instanceId": gift.id }))),
        P1,
    );

    assert_eq!(events, vec![]);
    assert_eq!(s.unit(P1, 1).map(|card| card.id), Some(gift.id.clone()));
    assert_eq!(s.card(&gift).controller, P1);
    assert_eq!(s.card(&gift).marked_destroyed, None);
}

#[test]
fn r4205000_r1849002_r4207000_with_disintegrex_acting_the_target_is_exiled_instead_and_still_dies() {
    let mut s = board(
        "r4205000-disintegrex",
        json!({ "field": [DISINTEGREX, GRUNT, GRUNT, GRUNT, GRUNT] }),
        json!({ "field": [PHOENIX] }),
    );
    let target = s.unit_at(P2, 1);
    let hand_before = s.hand(P2).len();

    let events = settled(&mut s, steal_of(&target.id), P1);

    // R4207000: exiled instead of reaching its graveyard, it has died all the same, on its own side.
    assert_eq!(destroyed(&events), vec![target.id.clone()]);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, GameEvent::Exiled { instance_id, .. } if *instance_id == target.id))
    );
    assert_eq!(
        s.unit(P2, 1).map(|card| card.id),
        Some(target.id.clone()),
        "Reborn returns it from exile to p2's lane"
    );
    assert_eq!(s.card(&target).controller, P2);
    assert_eq!(s.hand(P2).len(), hand_before + 1, "its Death drew a card");
}

#[test]
fn r4205000_r282_a_radiant_steal_makes_only_a_card_it_took_radiant() {
    let radiant_steal = |id: &str| steal(json_as(json!({ "instanceId": id, "radiant": true })));

    let mut s = board("r4205000-radiant-taken", json!({}), json!({ "field": [PLAIN] }));
    let taken = s.unit_at(P2, 1);
    let events = settled(&mut s, radiant_steal(&taken.id), P1);
    assert_eq!(control_changed(&events), vec![taken.id.clone()]);
    assert_eq!(s.card(&taken).controller, P1);
    assert!(s.card(&taken).radiant, "the card it took is made Radiant");

    let mut s = board(
        "r4205000-radiant-no-room",
        json!({ "field": full_row() }),
        json!({ "field": [PLAIN] }),
    );
    let target = s.unit_at(P2, 1);
    let events = settled(&mut s, radiant_steal(&target.id), P1);
    assert_eq!(destroyed(&events), vec![target.id.clone()]);
    assert!(
        !s.card(&target).radiant,
        "a card it had no room for is made nothing"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, GameEvent::RadiantSet { .. })),
        "no Make Radiant cue"
    );
}
