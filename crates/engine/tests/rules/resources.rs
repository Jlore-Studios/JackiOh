//! Stinkyscape's Resources and Credits (#1840, R1840000–R1840006): each player's Resource tray beside
//! the hand, holding at most `RESOURCE_TRAY_CAP`, with a Resource gained past it sold at once; the
//! public Credits; "Gain a random Resource" by `RESOURCE_RARITY_ODDS`, a restricted pool and a Radiant
//! gain; a tray Resource played with `play`; "discard a Resource" and "fill the tray"; and the
//! asteroid's mining node. Proved on the fixtures of `fixtures/resources.rs`; the effect harness is
//! `effects_jade.rs`'s.

use jackioh_engine::effects::{
    discard_hand, discard_random, discard_resource, gain_market_credits, gain_resources, give_from_hand,
};
use jackioh_engine::testkit::*;
use jackioh_engine::view_for::{HIDDEN_ID, view_for};

use crate::rules::fixtures::harness::{run_effect, setup_catalog, spread};
use crate::rules::fixtures::resources::*;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// The fixture catalog with the Resources and the asteroid, and their scripts.
fn register() {
    setup_catalog();
    register_catalog(resources_catalog(registered_catalog().clone()));
    let mut scripts = registered_scripts();
    scripts.extend(RESOURCE_SCRIPTS.clone());
    register_scripts(scripts);
}

/// Turn 9, `active`'s main phase. Each side holds a vanilla Unit and a library to draw, so no turn
/// ends by itself (§2.5); `p1` and `p2` add to each side's setup.
fn board(seed: &str, active: &str, p1: Value, p2: Value) -> Scenario {
    register();
    let side = |extra: Value| {
        spread(
            json!({ "hand": ["fx-1"], "library": ["fx-2", "fx-3", "fx-4"] }),
            extra,
        )
    };
    scenario(json!({ "seed": seed, "active": active, "p1": side(p1), "p2": side(p2) }))
}

fn gain(state: &mut GameState, player: PlayerId, args: Value) -> Vec<GameEvent> {
    run_effect(state, gain_resources(json_as(args)), player)
}

fn tray(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    resource_tray_of(state, player)
}

fn tray_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    tray(state, player).into_iter().map(|card| card.id).collect()
}

fn types_of(events: &[GameEvent]) -> Vec<GameEventType> {
    events.iter().map(GameEvent::event_type).collect()
}

fn sold(events: &[GameEvent]) -> Vec<i32> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::ResourceSold { credits, .. } => Some(*credits),
            _ => None,
        })
        .collect()
}

fn play_of(id: &str) -> ActionBody {
    ActionBody::Play {
        instance_id: id.to_string(),
        zone: None,
        x: None,
        embiggen: None,
        equip: None,
        tributes: None,
        targets: None,
        modes: None,
        plague: None,
        face_down: None,
    }
}

fn plays_naming(state: &GameState, player: PlayerId, id: &str) -> usize {
    legal_actions(state, player)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if instance_id == id))
        .count()
}

/// The rarity a fixture Resource rolls as.
fn rarity_of(def_id: &str) -> &'static str {
    RESOURCES
        .iter()
        .find(|(id, _)| *id == def_id)
        .map(|(_, rarity)| *rarity)
        .unwrap_or_else(|| panic!("{def_id} is no fixture Resource"))
}

#[test]
fn r1840000_a_gained_resource_reaches_the_tray_not_the_hand() {
    let s = board("r1840000-gain", "p1", json!({}), json!({}));
    let mut state = s.state().clone();
    let hand = state.players[P1].hand.clone();
    assert_eq!(
        state.players[P1].resource_tray, None,
        "no tray before the first Resource"
    );
    let events = gain(&mut state, P1, json!({ "count": 1, "pool": [COMMON_A] }));
    let held = tray(&state, P1);
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].def_id, COMMON_A);
    assert_eq!(held[0].zone, Zone::Tray { player: P1 });
    assert_eq!(state.players[P1].hand, hand, "the hand is untouched");
    assert_eq!(
        events,
        vec![GameEvent::ResourceGained {
            player: P1,
            instance_id: held[0].id.clone(),
            def_id: COMMON_A.to_string(),
            radiant: None,
        }]
    );
    assert!(tray_holds(&state, P1, &[COMMON_A]));
    assert!(!tray_holds(&state, P2, &[COMMON_A]));

    // A full hand takes no Resource away: it is gained all the same, and nothing burns.
    while (state.players[P1].hand.len() as i32) < HAND_CAP {
        let card = new_instance(&mut state, "fx-5", P1, Zone::Hand { player: P1 });
        state.players[P1].hand.push(card);
    }
    let events = gain(&mut state, P1, json!({ "count": 1, "pool": [RARE] }));
    assert_eq!(types_of(&events), vec![GameEventType::ResourceGained]);
    assert_eq!(tray(&state, P1).len(), 2);
    assert_eq!(state.players[P1].hand.len() as i32, HAND_CAP);
}

#[test]
fn r1840000_hand_verbs_never_reach_the_tray() {
    let s = board(
        "r1840000-hand-verbs",
        "p1",
        json!({ "hand": ["fx-1", "fx-2", "fx-3"], "tray": [COMMON_A, RARE] }),
        json!({}),
    );
    let base = s.state().clone();
    let held = tray_ids(&base, P1);
    assert_eq!(held.len(), 2);
    let verbs: Vec<(&str, Effect)> = vec![
        ("discardHand", discard_hand(json_as(json!({})))),
        ("discardRandom", discard_random(json_as(json!({ "count": 3 })))),
        (
            "giveFromHand",
            give_from_hand(json_as(json!({ "from": "self", "cards": "all" }))),
        ),
    ];
    for (name, verb) in verbs {
        let mut state = base.clone();
        run_effect(&mut state, verb, P1);
        assert!(state.players[P1].hand.is_empty(), "{name} empties the hand");
        assert_eq!(tray_ids(&state, P1), held, "{name} leaves the tray as it was");
        assert!(
            tray(&state, P2).is_empty(),
            "{name} gives the other player no Resource"
        );
    }
}

#[test]
fn r1840000_the_opponent_sees_the_tray_count_only() {
    let s = board(
        "r1840000-view",
        "p1",
        json!({ "tray": [COMMON_A, { "def": RARE, "radiant": true }] }),
        json!({}),
    );
    let mut state = s.state().clone();
    let ids = tray_ids(&state, P1);
    match view_for(&state, P1).you.resource_tray {
        Some(HandView::Cards(cards)) => {
            assert_eq!(
                cards
                    .iter()
                    .map(|card| card.instance_id.clone())
                    .collect::<Vec<_>>(),
                ids
            );
            assert_eq!(cards[1].def_id, RARE);
            assert!(cards[1].radiant);
        }
        other => panic!("the owner reads their tray: {other:?}"),
    }
    assert_eq!(
        view_for(&state, P2).opponent.resource_tray,
        Some(HandView::Count { count: 2 })
    );

    // The gain's own event, as each seat's view carries it.
    let events = gain(
        &mut state,
        P1,
        json!({ "count": 1, "radiant": true, "pool": [EPIC] }),
    );
    let gained = tray(&state, P1).last().cloned().expect("the Epic gained");
    state.applied.push(AppliedAction {
        nonce: "gain".to_string(),
        events,
    });
    let gained_in = |viewer: PlayerId| -> GameEvent {
        view_for(&state, viewer)
            .events
            .into_iter()
            .find(|event| event.event_type() == GameEventType::ResourceGained)
            .expect("the view carries the gain")
    };
    assert_eq!(
        gained_in(P1),
        GameEvent::ResourceGained {
            player: P1,
            instance_id: gained.id.clone(),
            def_id: EPIC.to_string(),
            radiant: Some(true),
        }
    );
    assert_eq!(
        gained_in(P2),
        GameEvent::ResourceGained {
            player: P1,
            instance_id: HIDDEN_ID.to_string(),
            def_id: HIDDEN_ID.to_string(),
            radiant: None,
        },
        "the other player learns a Resource was gained, not which"
    );

    // A game without a Resource or a Credit carries neither key anywhere.
    let fresh = board("r1840000-fresh", "p1", json!({}), json!({}));
    let state_json = serde_json::to_value(fresh.state()).expect("a state serialises");
    for player in ["p1", "p2"] {
        let side = &state_json["players"][player];
        assert!(side.get("resourceTray").is_none(), "{player}'s state");
        assert!(side.get("marketCredits").is_none(), "{player}'s state");
    }
    for viewer in [P1, P2] {
        let view = serde_json::to_value(view_for(fresh.state(), viewer)).expect("a view serialises");
        for seat in ["you", "opponent"] {
            assert!(view[seat].get("resourceTray").is_none(), "{viewer:?}'s {seat}");
            assert!(view[seat].get("marketCredits").is_none(), "{viewer:?}'s {seat}");
        }
    }
}

#[test]
fn r1840000_both_trays_read_once_the_game_is_over() {
    let s = board("r1840000-over", "p1", json!({ "tray": [COMMON_A] }), json!({}));
    let mut state = s.state().clone();
    let out = reduce(&state, &Action::new(ActionBody::Concede, P1, "concede"));
    assert_eq!(out.error, None);
    state = out.state;
    assert!(
        matches!(view_for(&state, P2).opponent.resource_tray, Some(HandView::Cards(ref cards)) if cards.len() == 1),
        "R434: the game's end reveals the tray as it reveals the hand"
    );
}

#[test]
fn r1840001_a_resource_gained_past_the_cap_is_sold_within_its_range() {
    let full: Vec<&str> = (0..RESOURCE_TRAY_CAP).map(|_| COMMON_A).collect();
    let s = board("r1840001-sale", "p1", json!({ "tray": full }), json!({}));
    let mut state = s.state().clone();
    let held = tray_ids(&state, P1);
    assert_eq!(held.len() as i32, RESOURCE_TRAY_CAP);

    let events = gain(&mut state, P1, json!({ "count": 1, "pool": [RARE] }));
    assert_eq!(
        tray_ids(&state, P1),
        held,
        "a ninth Resource leaves the tray at 8"
    );
    assert_eq!(
        types_of(&events),
        vec![GameEventType::ResourceSold, GameEventType::MarketCreditsChanged]
    );
    let credits = sold(&events)[0];
    assert!(
        (BASE_SELL.0..=BASE_SELL.1).contains(&credits),
        "the base face sells for {BASE_SELL:?}, got {credits}"
    );
    assert_eq!(state.players[P1].market_credits, Some(credits));
    assert_eq!(market_credits_of(&state, P1), credits);
    let GameEvent::ResourceSold {
        instance_id, radiant, ..
    } = &events[0]
    else {
        panic!("a sale first");
    };
    assert_eq!(*radiant, None);
    assert!(
        find_instance(&state, instance_id).is_none(),
        "the sold Resource ceased to exist"
    );

    let events = gain(
        &mut state,
        P1,
        json!({ "count": 1, "radiant": true, "pool": [RARE] }),
    );
    let radiant_credits = sold(&events)[0];
    assert!(
        (RADIANT_SELL.0..=RADIANT_SELL.1).contains(&radiant_credits),
        "the Radiant face sells for {RADIANT_SELL:?}, got {radiant_credits}"
    );
    assert_eq!(state.players[P1].market_credits, Some(credits + radiant_credits));
    assert!(matches!(
        events.last(),
        Some(GameEvent::MarketCreditsChanged { player: P1, value }) if *value == credits + radiant_credits
    ));
}

#[test]
fn r1840002_the_roll_follows_the_rarity_table() {
    let s = board("r1840002-odds", "p1", json!({}), json!({}));
    let state = s.state();
    let mut rng = Rng::new("resource-odds", 0);
    const ROLLS: i32 = 10_000;
    let mut by_rarity: IndexMap<&str, i32> = IndexMap::new();
    let mut commons: IndexSet<String> = IndexSet::new();
    for _ in 0..ROLLS {
        let id = roll_resource(state, &mut rng, None).expect("the catalog holds Resources");
        let rarity = rarity_of(&id);
        *by_rarity.entry(rarity).or_default() += 1;
        if rarity == "Common" {
            commons.insert(id);
        }
    }
    for (rarity, weight) in RESOURCE_RARITY_ODDS {
        let seen = f64::from(by_rarity.get(rarity.as_str()).copied().unwrap_or(0)) * 100.0 / f64::from(ROLLS);
        let slack = if *rarity == PrintedRarity::Mythic {
            0.5
        } else {
            2.0
        };
        assert!(
            (seen - f64::from(*weight)).abs() <= slack,
            "{rarity:?} rolled {seen}% against its {weight}%"
        );
    }
    assert_eq!(
        commons.len(),
        2,
        "each Common of the rarity rolled is picked uniformly"
    );
}

#[test]
fn r1840002_a_restricted_pool_rolls_only_its_own_rarities() {
    let s = board("r1840002-pool", "p1", json!({}), json!({}));
    let mut state = s.state().clone();
    let mut rng = Rng::new("resource-pool", 0);
    let pool = vec![LEGENDARY.to_string(), MYTHIC.to_string()];
    let mut seen: IndexSet<String> = IndexSet::new();
    for _ in 0..1000 {
        seen.insert(
            roll_resource(&state, &mut rng, Some(pool.as_slice())).expect("the pool holds Resources"),
        );
    }
    let mut seen: Vec<String> = seen.into_iter().collect();
    seen.sort();
    assert_eq!(seen, vec![LEGENDARY.to_string(), MYTHIC.to_string()]);

    // A pool that holds no Resource gains nothing and draws nothing.
    for pool in [vec![], vec!["fx-1".to_string()]] {
        let mut rng = Rng::new("resource-pool", 7);
        assert_eq!(roll_resource(&state, &mut rng, Some(pool.as_slice())), None);
        assert_eq!(rng.cursor(), 7, "{pool:?} draws nothing");
        let cursor = state.rng_cursor;
        let events = gain(&mut state, P1, json!({ "count": 3, "pool": pool }));
        assert!(events.is_empty(), "{pool:?} gains nothing");
        assert_eq!(state.rng_cursor, cursor);
        assert_eq!(state.players[P1].resource_tray, None);
    }
}

#[test]
fn r1840002_a_radiant_gain_makes_the_resource_radiant() {
    let s = board("r1840002-radiant", "p1", json!({}), json!({}));
    let mut state = s.state().clone();
    let events = gain(&mut state, P1, json!({ "count": 2, "radiant": true }));
    assert_eq!(events.len(), 2);
    for event in &events {
        assert!(
            matches!(
                event,
                GameEvent::ResourceGained {
                    radiant: Some(true),
                    ..
                }
            ),
            "{event:?}"
        );
    }
    assert!(tray(&state, P1).iter().all(|card| card.radiant));
    let events = gain(&mut state, P1, json!({ "count": 1 }));
    assert!(matches!(
        events.as_slice(),
        [GameEvent::ResourceGained { radiant: None, .. }]
    ));
    assert!(
        !tray(&state, P1)[2].radiant,
        "a plain gain gives a plain Resource"
    );
}

#[test]
fn r1840003_a_resource_is_played_from_the_tray() {
    let mut s = board("r1840003-play", "p1", json!({ "tray": [COMMON_A] }), json!({}));
    let id = tray_ids(s.state(), P1).remove(0);
    let health = s.state().players[P2].hero.health;
    assert_eq!(plays_naming(s.state(), P1, &id), 1, "legal_actions lists it");
    s.act(play_of(&id), P1, "play the tray Resource");
    assert_eq!(
        s.state().players[P1].resource_tray,
        None,
        "the tray is empty again"
    );
    assert_eq!(
        s.state().players[P2].hero.health,
        health - COMMON_A_DAMAGE,
        "its Cry ran"
    );
    assert!(s.last_events().iter().any(|event| matches!(
        event,
        GameEvent::CardPlayed { instance_id, from: Some(PlayedFrom::Tray), .. } if *instance_id == id
    )));
    s.expect_in_zone(id.as_str(), "graveyard");
    assert!(s.pile(P1, "graveyard").iter().any(|card| card.id == id));
}

#[test]
fn r1840003_the_listing_and_the_refusal_agree() {
    // On p2's turn: p2 naming p1's tray card, and p1 playing its own.
    let s = board("r1840003-refuse", "p2", json!({ "tray": [COMMON_A] }), json!({}));
    let id = tray_ids(s.state(), P1).remove(0);
    for (player, nonce) in [(P2, "theirs"), (P1, "off-turn")] {
        assert_eq!(plays_naming(s.state(), player, &id), 0, "{nonce}: not listed");
        let out = reduce(s.state(), &Action::new(play_of(&id), player, nonce));
        assert!(out.error.is_some(), "{nonce}: refused");
        assert_eq!(
            tray_ids(&out.state, P1),
            vec![id.clone()],
            "{nonce}: the tray is as it was"
        );
    }
}

#[test]
fn r1840004_discard_resource_takes_the_oldest_match_and_it_ceases_to_exist() {
    let s = board(
        "r1840004-discard",
        "p1",
        json!({ "tray": [COMMON_A, RARE, COMMON_A] }),
        json!({}),
    );
    let mut state = s.state().clone();
    let ids = tray_ids(&state, P1);
    let discard = |state: &mut GameState, def_ids: Value| {
        run_effect(state, discard_resource(json_as(json!({ "defIds": def_ids }))), P1)
    };

    let events = discard(&mut state, json!([COMMON_A]));
    assert_eq!(
        events,
        vec![GameEvent::ResourceDiscarded {
            player: P1,
            instance_id: ids[0].clone(),
            def_id: COMMON_A.to_string(),
        }]
    );
    assert_eq!(tray_ids(&state, P1), vec![ids[1].clone(), ids[2].clone()]);
    assert!(find_instance(&state, &ids[0]).is_none(), "it ceased to exist");
    assert!(state.players[P1].graveyard.iter().all(|card| card.id != ids[0]));

    assert!(
        discard(&mut state, json!([EPIC])).is_empty(),
        "no match, no discard"
    );
    let events = discard(&mut state, json!([]));
    assert!(
        matches!(events.as_slice(), [GameEvent::ResourceDiscarded { instance_id, .. }] if *instance_id == ids[1]),
        "any Resource, the oldest first"
    );
    discard(&mut state, json!([]));
    assert_eq!(state.players[P1].resource_tray, None, "an emptied tray is absent");
    assert!(discard(&mut state, json!([])).is_empty());
}

#[test]
fn r1840004_fill_tray_gains_up_to_the_cap() {
    let s = board(
        "r1840004-fill",
        "p1",
        json!({ "tray": [COMMON_A, COMMON_A, COMMON_A, COMMON_A, COMMON_A] }),
        json!({}),
    );
    let mut state = s.state().clone();
    let events = gain(&mut state, P1, json!({ "fill": true }));
    assert_eq!(
        types_of(&events),
        vec![GameEventType::ResourceGained; 3],
        "3 fill the tray to 8, and none is sold"
    );
    assert_eq!(tray(&state, P1).len() as i32, RESOURCE_TRAY_CAP);
    let cursor = state.rng_cursor;
    assert!(
        gain(&mut state, P1, json!({ "fill": true })).is_empty(),
        "a full tray gains nothing"
    );
    assert_eq!(state.rng_cursor, cursor);
}

#[test]
fn r1840005_credits_are_public_and_absent_until_gained() {
    let s = board("r1840005-credits", "p1", json!({}), json!({}));
    let mut state = s.state().clone();
    let events = run_effect(
        &mut state,
        gain_market_credits(json_as(json!({ "amount": 20 }))),
        P1,
    );
    assert_eq!(
        events,
        vec![GameEvent::MarketCreditsChanged {
            player: P1,
            value: 20
        }]
    );
    assert_eq!(view_for(&state, P1).you.market_credits, Some(20));
    assert_eq!(view_for(&state, P2).opponent.market_credits, Some(20));
    assert_eq!(view_for(&state, P1).opponent.market_credits, None);
    for amount in [0, -5] {
        let events = run_effect(
            &mut state,
            gain_market_credits(json_as(json!({ "amount": amount, "player": "enemy" }))),
            P1,
        );
        assert!(events.is_empty(), "a gain of {amount} does nothing");
        assert_eq!(state.players[P2].market_credits, None);
    }
}

#[test]
fn r1840006_a_mining_node_pays_its_resources_on_death() {
    for radiant in [false, true] {
        let mut s = board(
            &format!("r1840006-mine-{radiant}"),
            "p1",
            json!({ "field": [{ "def": ASTEROID, "radiant": radiant }] }),
            json!({}),
        );
        let asteroid = s.unit(P1, 1).expect("the asteroid stands in lane 1");
        let health = s.state().players[P1].hero.health;
        s.activate(asteroid.id.as_str(), json!({}));
        let damage = if radiant { 0 } else { ASTEROID_DAMAGE };
        assert_eq!(
            s.state().players[P1].hero.health,
            health - damage,
            "radiant {radiant}: the hero takes the mine's damage"
        );
        s.expect_in_zone(asteroid.id.as_str(), "graveyard");
        let held = tray(s.state(), P1);
        assert_eq!(
            held.len() as i32,
            ASTEROID_YIELD,
            "radiant {radiant}: its Death paid"
        );
        assert!(
            held.iter().all(|card| card.radiant == radiant),
            "radiant {radiant}"
        );
        assert!(
            tray(s.state(), P2).is_empty(),
            "the Resources are its controller's"
        );
    }
}
