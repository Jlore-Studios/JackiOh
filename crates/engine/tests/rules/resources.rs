//! Stinkyscape's Resources and Credits (#1840, R1840000–R1840006; #2250, R2250000, R2250001): a
//! Resource is a card of its holder's hand, gained into it and burned by a full one, and it pays its
//! player Credits as it is played or discarded from there. Proved on the fixtures of
//! `fixtures/resources.rs`; the effect harness is `effects_jade.rs`'s.

use jackioh_engine::effects::{cast, discard_random, discard_resource, gain_market_credits, gain_resources};
use jackioh_engine::testkit::*;
use jackioh_engine::view_for::{HIDDEN_ID, view_for};

use crate::rules::fixtures::harness::{run_effect, setup_catalog, spread, types_of};
use crate::rules::fixtures::resources::*;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

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

/// The Resources in `player`'s hand, oldest first (R2250000).
fn held(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    resources_in_hand_of(state, player)
}

fn held_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    held(state, player).into_iter().map(|card| card.id).collect()
}

fn credit_counts(events: &[GameEvent]) -> Vec<i32> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::MarketCreditsChanged { value, .. } => Some(*value),
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

fn rarity_of(def_id: &str) -> &'static str {
    RESOURCES
        .iter()
        .find(|(id, _)| *id == def_id)
        .map(|(_, rarity)| *rarity)
        .unwrap_or_else(|| panic!("{def_id} is no fixture Resource"))
}

fn fill_hand(state: &mut GameState, player: PlayerId) {
    while (state.players[player].hand.len() as i32) < HAND_CAP {
        let card = new_instance(state, "fx-5", player, Zone::Hand { player });
        state.players[player].hand.push(card);
    }
}

#[test]
fn r1840000_r2250000_a_gained_resource_joins_the_hand_and_a_full_hand_burns_it() {
    let s = board("r1840000-gain", "p1", json!({}), json!({}));
    let mut state = s.state().clone();
    let hand = state.players[P1].hand.len();
    let events = gain(&mut state, P1, json!({ "count": 1, "pool": [COMMON_A] }));
    let gained = held(&state, P1);
    assert_eq!(gained.len(), 1);
    assert_eq!(gained[0].def_id, COMMON_A);
    assert_eq!(gained[0].zone, Zone::Hand { player: P1 });
    assert_eq!(state.players[P1].hand.len(), hand + 1, "it takes a hand slot");
    assert!(events.iter().any(|event| matches!(
        event,
        GameEvent::AddedToHand { player: P1, instance_id, def_id, .. }
            if *instance_id == gained[0].id && def_id == COMMON_A
    )));
    assert!(hand_holds_any(&state, P1, &[COMMON_A]));
    assert!(!hand_holds_any(&state, P2, &[COMMON_A]));

    // R4: a full hand burns it, and it pays nothing as it burns (R2250001).
    fill_hand(&mut state, P1);
    let credits = state.players[P1].market_credits;
    let events = gain(&mut state, P1, json!({ "count": 1, "pool": [RARE] }));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, GameEvent::Burned { def_id, owner: P1, .. } if def_id == RARE))
    );
    assert_eq!(state.players[P1].hand.len() as i32, HAND_CAP);
    assert!(!hand_holds_any(&state, P1, &[RARE]));
    assert_eq!(
        state.players[P1].market_credits, credits,
        "a burned Resource pays nothing"
    );
}

#[test]
fn r2250000_the_other_player_sees_a_hand_card_gained_not_which() {
    let s = board("r2250000-view", "p1", json!({}), json!({}));
    let mut state = s.state().clone();
    let events = gain(
        &mut state,
        P1,
        json!({ "count": 1, "radiant": true, "pool": [EPIC] }),
    );
    let gained = held(&state, P1).last().cloned().expect("the Epic gained");
    assert!(gained.radiant);
    state.applied.push(AppliedAction {
        nonce: "gain".to_string(),
        events,
    });
    let added_in = |viewer: PlayerId| -> GameEvent {
        view_for(&state, viewer)
            .events
            .into_iter()
            .find(|event| event.event_type() == GameEventType::AddedToHand)
            .expect("the view carries the gain")
    };
    assert!(matches!(
        added_in(P1),
        GameEvent::AddedToHand { instance_id, def_id, .. } if instance_id == gained.id && def_id == EPIC
    ));
    assert!(
        matches!(
            added_in(P2),
            GameEvent::AddedToHand { instance_id, def_id, .. } if instance_id == HIDDEN_ID && def_id == HIDDEN_ID
        ),
        "the other player learns a card was gained, not which (R97)"
    );
    match view_for(&state, P1).you.hand {
        HandView::Cards(cards) => assert!(cards.iter().any(|card| card.instance_id == gained.id)),
        other => panic!("the owner reads their hand: {other:?}"),
    }
    let count = state.players[P1].hand.len() as i32;
    assert_eq!(view_for(&state, P2).opponent.hand, HandView::Count { count });

    // A game without a Credit carries the key nowhere.
    let fresh = board("r2250000-fresh", "p1", json!({}), json!({}));
    let state_json = serde_json::to_value(fresh.state()).expect("a state serializes");
    for player in ["p1", "p2"] {
        assert!(
            state_json["players"][player].get("marketCredits").is_none(),
            "{player}'s state"
        );
    }
    for viewer in [P1, P2] {
        let view = serde_json::to_value(view_for(fresh.state(), viewer)).expect("a view serializes");
        for seat in ["you", "opponent"] {
            assert!(view[seat].get("marketCredits").is_none(), "{viewer:?}'s {seat}");
        }
    }
}

#[test]
fn r1840001_r2250001_a_resource_played_pays_credits_within_its_faces_range() {
    for radiant in [false, true] {
        let mut s = board(
            &format!("r2250001-play-{radiant}"),
            "p1",
            json!({ "hand": ["fx-1", { "def": COMMON_A, "radiant": radiant }] }),
            json!({}),
        );
        let id = held_ids(s.state(), P1).remove(0);
        let health = s.state().players[P2].hero.health;
        s.act(play_of(&id), P1, "play the Resource");
        let events = s.last_events().to_vec();
        let paid = credit_counts(&events);
        assert_eq!(paid.len(), 1, "radiant {radiant}: one payment");
        let (low, high) = if radiant { RADIANT_SELL } else { BASE_SELL };
        assert!(
            (low..=high).contains(&paid[0]),
            "radiant {radiant}: {} pays in {low}..={high}",
            paid[0]
        );
        assert_eq!(market_credits_of(s.state(), P1), paid[0]);
        // Right after its `cardPlayed`, before its Cry (§10.5 step 4).
        let kinds = types_of(&events);
        let at = |kind: GameEventType| {
            kinds
                .iter()
                .position(|each| *each == kind)
                .unwrap_or_else(|| panic!("no {kind:?} in {kinds:?}"))
        };
        assert!(at(GameEventType::CardPlayed) < at(GameEventType::MarketCreditsChanged));
        assert!(at(GameEventType::MarketCreditsChanged) < at(GameEventType::Damage));
        assert_eq!(
            s.state().players[P2].hero.health,
            health - COMMON_A_DAMAGE,
            "its Cry ran"
        );
    }
}

#[test]
fn r2250001_a_discarded_resource_pays_and_a_cast_by_an_effect_pays_nothing() {
    let s = board("r2250001-discard", "p1", json!({ "hand": [COMMON_A] }), json!({}));
    let mut state = s.state().clone();
    state.players[P1].hand.retain(|card| card.def_id == COMMON_A);
    let id = held_ids(&state, P1).remove(0);
    let events = run_effect(&mut state, discard_random(json_as(json!({ "count": 1 }))), P1);
    assert!(events.iter().any(|event| matches!(
        event,
        GameEvent::Discarded { instance_id, owner: P1, .. } if *instance_id == id
    )));
    let paid = credit_counts(&events);
    assert_eq!(paid.len(), 1);
    assert!((BASE_SELL.0..=BASE_SELL.1).contains(&paid[0]));
    assert!(state.players[P1].graveyard.iter().any(|card| card.id == id));

    // Cast by an effect, it is no play of its player's: no Credits.
    let s = board("r2250001-cast", "p1", json!({ "hand": [COMMON_A] }), json!({}));
    let mut state = s.state().clone();
    let id = held_ids(&state, P1).remove(0);
    let events = run_effect(
        &mut state,
        cast(json_as(
            json!({ "target": { "of": "instance", "instanceId": id } }),
        )),
        P1,
    );
    assert!(
        events
            .iter()
            .any(|event| event.event_type() == GameEventType::CardPlayed),
        "{:?}",
        types_of(&events)
    );
    assert!(credit_counts(&events).is_empty(), "a cast pays nothing");
    assert_eq!(state.players[P1].market_credits, None);
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
        assert!(held(&state, P1).is_empty());
    }
}

#[test]
fn r1840002_a_radiant_gain_makes_the_resource_radiant() {
    let s = board("r1840002-radiant", "p1", json!({}), json!({}));
    let mut state = s.state().clone();
    gain(&mut state, P1, json!({ "count": 2, "radiant": true }));
    let gained = held(&state, P1);
    assert_eq!(gained.len(), 2);
    assert!(gained.iter().all(|card| card.radiant));
    gain(&mut state, P1, json!({ "count": 1 }));
    assert!(
        !held(&state, P1)[2].radiant,
        "a plain gain gives a plain Resource"
    );
}

#[test]
fn r1840003_r2250000_a_resource_is_played_from_the_hand_like_any_card() {
    let mut s = board("r1840003-play", "p1", json!({ "hand": [COMMON_A] }), json!({}));
    let id = held_ids(s.state(), P1).remove(0);
    assert_eq!(plays_naming(s.state(), P1, &id), 1, "legal_actions lists it");
    s.act(play_of(&id), P1, "play the Resource");
    assert!(held(s.state(), P1).is_empty(), "it left the hand");
    assert!(s.last_events().iter().any(|event| matches!(
        event,
        GameEvent::CardPlayed { instance_id, from: None, .. } if *instance_id == id
    )));
    s.expect_in_zone(id.as_str(), "graveyard");

    // On p2's turn: p2 naming p1's card, and p1 playing its own, are neither listed nor allowed.
    let s = board("r1840003-refuse", "p2", json!({ "hand": [COMMON_A] }), json!({}));
    let id = held_ids(s.state(), P1).remove(0);
    for (player, nonce) in [(P2, "theirs"), (P1, "off-turn")] {
        assert_eq!(plays_naming(s.state(), player, &id), 0, "{nonce}: not listed");
        let out = reduce(s.state(), &Action::new(play_of(&id), player, nonce));
        assert!(out.error.is_some(), "{nonce}: refused");
        assert_eq!(held_ids(&out.state, P1), vec![id.clone()], "{nonce}: still held");
    }
}

#[test]
fn r1840004_r2250000_discard_resource_takes_the_oldest_match_from_the_hand() {
    let s = board(
        "r1840004-discard",
        "p1",
        json!({ "hand": ["fx-1", COMMON_A, RARE, COMMON_A] }),
        json!({}),
    );
    let mut state = s.state().clone();
    let ids = held_ids(&state, P1);
    let discard = |state: &mut GameState, def_ids: Value| {
        run_effect(state, discard_resource(json_as(json!({ "defIds": def_ids }))), P1)
    };

    let events = discard(&mut state, json!([COMMON_A]));
    assert!(events.iter().any(|event| matches!(
        event,
        GameEvent::Discarded { instance_id, owner: P1, .. } if *instance_id == ids[0]
    )));
    assert_eq!(
        credit_counts(&events).len(),
        1,
        "it pays as any discard of it does (R2250001)"
    );
    assert_eq!(held_ids(&state, P1), vec![ids[1].clone(), ids[2].clone()]);
    assert!(state.players[P1].graveyard.iter().any(|card| card.id == ids[0]));

    assert!(
        discard(&mut state, json!([EPIC])).is_empty(),
        "no match, no discard"
    );
    let events = discard(&mut state, json!([]));
    assert!(
        events.iter().any(|event| matches!(
            event,
            GameEvent::Discarded { instance_id, .. } if *instance_id == ids[1]
        )),
        "any Resource, the oldest first"
    );
    discard(&mut state, json!([]));
    assert!(held(&state, P1).is_empty());
    assert!(discard(&mut state, json!([])).is_empty());
    assert!(
        state.players[P1].hand.iter().any(|card| card.def_id == "fx-1"),
        "a card that is no Resource stays"
    );
}

#[test]
fn r1840004_r2250000_fill_gains_up_to_the_hand_cap() {
    let s = board(
        "r1840004-fill",
        "p1",
        json!({ "hand": ["fx-1", "fx-2"] }),
        json!({}),
    );
    let mut state = s.state().clone();
    let room = HAND_CAP - state.players[P1].hand.len() as i32;
    let events = gain(&mut state, P1, json!({ "fill": true }));
    assert_eq!(
        types_of(&events)
            .iter()
            .filter(|kind| **kind == GameEventType::AddedToHand)
            .count() as i32,
        room,
        "the hand fills to its cap, and none burns"
    );
    assert_eq!(state.players[P1].hand.len() as i32, HAND_CAP);
    let cursor = state.rng_cursor;
    assert!(
        gain(&mut state, P1, json!({ "fill": true })).is_empty(),
        "a full hand gains nothing"
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
        let gained = held(s.state(), P1);
        assert_eq!(
            gained.len() as i32,
            ASTEROID_YIELD,
            "radiant {radiant}: its Death paid"
        );
        assert!(
            gained.iter().all(|card| card.radiant == radiant),
            "radiant {radiant}"
        );
        assert!(
            held(s.state(), P2).is_empty(),
            "the Resources are its controller's"
        );
    }
}
