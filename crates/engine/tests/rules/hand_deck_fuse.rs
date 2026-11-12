//! The hand, deck and fuse verbs Stinkyscape+ E8 (issue #3558) adds, each proved on the
//! `hand_deck_fuse` fixture scripts before a card uses them: hand positions and ends, a hand card's
//! aura on its neighbours, a meeting pair, a Spell filled once with random Fruits, fusing until a
//! cost, an end-of-turn hand fuse of a Grape, bottom starters, a deck cast and casting one card N
//! times. The cards' own tests play them in the real scripts; these pin the verbs themselves, so
//! the engine never imports `crates/cards`.

use jackioh_engine::effects;
use jackioh_engine::subsystems::fuse::fused_ingredients;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::hand_deck_fuse::*;
use crate::rules::fixtures::harness::{act, apply_now, hand_ids, round_trip, run_as_p1};

fn board(p1: Value, p2: Value) -> Scenario {
    register_e8_fixtures();
    let mut s = scenario(json!({ "seed": "e8", "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn hero_pick(player: PlayerId) -> Value {
    json!({ "pick": "hero", "player": player.as_str() })
}

fn instance_pick(id: &str) -> Value {
    json!({ "pick": "instance", "instanceId": id })
}

fn hero_health(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].hero.health
}

fn exile_count(state: &GameState, player: PlayerId) -> usize {
    state.players[player].exile.len()
}

/// R3558000: the hand reads — index 0 is the leftmost card — and an added card takes the named end.
#[test]
fn r3558000_hand_reads_and_adds_at_either_end() {
    let mut s = board(json!({ "hand": [ESSENCE, PAIR_L, PAIR_R] }), json!({}));
    let state = s.state();
    let ids = hand_ids(state, P1);
    assert_eq!(ids.len(), 3);
    assert_eq!(
        hand_leftmost(state, P1).map(|card| card.id.clone()),
        Some(ids[0].clone())
    );
    assert_eq!(
        hand_rightmost(state, P1).map(|card| card.id.clone()),
        Some(ids[2].clone())
    );
    assert_eq!(hand_index(state, &ids[1]), Some((P1, 1)));
    assert_eq!(
        hand_neighbor(state, &ids[1], HandSide::Left).map(|card| card.id.clone()),
        Some(ids[0].clone())
    );
    assert_eq!(
        hand_neighbor(state, &ids[1], HandSide::Right).map(|card| card.id.clone()),
        Some(ids[2].clone())
    );
    assert!(is_leftmost_in_hand(state, &ids[0]));
    assert!(!is_leftmost_in_hand(state, &ids[1]));
    assert!(is_rightmost_in_hand(state, &ids[2]));
    assert!(!is_rightmost_in_hand(state, &ids[1]));
    assert!(hand_neighbor(state, &ids[0], HandSide::Left).is_none());
    assert!(hand_neighbor(state, &ids[2], HandSide::Right).is_none());
    assert!(hand_index(state, "c-nothing").is_none());

    apply_now(
        s.state_mut(),
        effects::add_to_hand(json_as(json!({ "defId": EXILER, "at": "left" }))),
    );
    apply_now(
        s.state_mut(),
        effects::add_to_hand(json_as(json!({ "defId": BOLT }))),
    );
    let ids = hand_ids(s.state(), P1);
    assert_eq!(ids.len(), 5);
    assert_eq!(s.card(&ids[0]).def_id, EXILER);
    assert_eq!(s.card(&ids[4]).def_id, BOLT);
    assert_eq!(s.card(&ids[1]).def_id, ESSENCE);
}

/// R3558001: the leftmost Essence lends its right neighbour Radiance and −1 cost until it moves.
#[test]
fn r3558001_leftmost_essence_lends_radiance_and_discount_until_it_moves() {
    let mut s = board(json!({ "hand": [ESSENCE, PAIR_L] }), json!({}));
    let lent = s.hand(P1)[1].id.clone();
    // The setup's settle synced the lends already.
    assert!(s.card(&lent).radiant);
    assert_eq!(
        s.state().radiance_lends.as_ref().map(|lends| lends.len()),
        Some(1)
    );
    assert_eq!(effective_cost(s.state(), s.card(&lent), Default::default()), 1);

    s.play(ESSENCE, json!({}));
    assert!(!s.card(&lent).radiant);
    assert_eq!(s.state().radiance_lends, None);
    assert!(s.events().iter().any(|event| matches!(
        event,
        GameEvent::Deradianted { instance_id, .. } if instance_id == &lent
    )));
    assert_eq!(effective_cost(s.state(), s.card(&lent), Default::default()), 2);
}

/// R3558001: a lent card played onto the field leaves the hand Radiant and stays Radiant.
#[test]
fn r3558001_a_lent_card_played_stays_radiant() {
    let mut s = board(json!({ "hand": [ESSENCE, PAIR_L], "mana": 10 }), json!({}));
    let lent = s.hand(P1)[1].id.clone();
    assert!(s.card(&lent).radiant);
    s.play(&lent, json!({}));
    let field = s.unit(P1, 1).expect("the played unit");
    assert_eq!(field.id, lent);
    assert!(field.radiant);
    assert_eq!(s.state().radiance_lends, None);
    assert!(!s.events().iter().any(|event| matches!(
        event,
        GameEvent::Deradianted { instance_id, .. } if instance_id == &lent
    )));
}

/// R3558001: a lent card made Radiant on its own is no longer a lend, so it keeps the face.
#[test]
fn r3558001_make_radiant_keeps_a_lent_card_radiant() {
    let mut s = board(json!({ "hand": [ESSENCE, PAIR_L] }), json!({}));
    let lent = s.hand(P1)[1].id.clone();
    apply_now(
        s.state_mut(),
        effects::set_radiant(json_as(json!({ "instanceId": lent }))),
    );
    // Made Radiant on its own: the lend is forgotten, the face kept.
    assert!(s.card(&lent).radiant);
    assert_eq!(s.state().radiance_lends, None);
    s.play(ESSENCE, json!({}));
    assert!(s.card(&lent).radiant);
    assert!(!s.events().iter().any(|event| matches!(
        event,
        GameEvent::Deradianted { instance_id, .. } if instance_id == &lent
    )));
}

/// R3558002: a pair with cards between them meets — Radiant at 0 — once the middle leaves.
#[test]
fn r3558002_pair_meets_when_the_cards_between_leave_radiant_at_cost_0() {
    let mut s = board(json!({ "hand": [ESSENCE, SHIP], "mana": 10 }), json!({}));
    s.play(SHIP, json!({}));
    let ids = hand_ids(s.state(), P1);
    assert_eq!(ids.len(), 3);
    let left = ids[0].clone();
    let right = ids[2].clone();
    assert_eq!(s.card(&left).def_id, PAIR_L);
    assert_eq!(s.card(&right).def_id, PAIR_R);
    // Not adjacent yet: the meet waits.
    assert!(!s.card(&left).radiant);
    assert!(!s.card(&right).radiant);

    // Essence leaves the hand: the pair sits side by side and meets at once.
    s.play(ESSENCE, json!({}));
    for id in [&left, &right] {
        assert!(s.card(id).radiant, "{id} met Radiant");
        assert_eq!(s.card(id).cost_override, Some(0));
        assert_eq!(effective_cost(s.state(), s.card(id), Default::default()), 0);
    }
    assert_eq!(s.state().hand_meets, None);
    assert_eq!(
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::RadiantSet { .. }))
            .count(),
        2
    );
}

/// R3558002: a pair added side by side to an empty hand meets at once.
#[test]
fn r3558002_pair_added_to_an_empty_hand_meets_at_once() {
    let mut s = board(json!({ "hand": [SHIP] }), json!({}));
    s.play(SHIP, json!({}));
    let ids = hand_ids(s.state(), P1);
    assert_eq!(ids.len(), 2);
    for id in &ids {
        assert!(s.card(id).radiant, "{id} met at once");
        assert_eq!(s.card(id).cost_override, Some(0));
    }
    assert_eq!(s.state().hand_meets, None);
}

/// R3558002: a card that leaves the hand ends the meeting, and only the owner sees the mark.
#[test]
fn r3558002_leaving_the_hand_ends_the_meeting_and_only_the_owner_sees_the_mark() {
    let mut s = board(json!({ "hand": [ESSENCE, SHIP], "mana": 10 }), json!({}));
    s.play(SHIP, json!({}));
    let ids = hand_ids(s.state(), P1);
    let (left, mid, right) = (ids[0].clone(), ids[1].clone(), ids[2].clone());
    assert_eq!(s.card(&mid).def_id, ESSENCE);
    // The owner's view marks both waiting cards; the opponent reads no marked count.
    let marks_of = |view: PlayerView, id: &str| -> Vec<CardMark> {
        match view.you.hand {
            HandView::Cards(cards) => cards
                .into_iter()
                .find(|card| card.instance_id == id)
                .and_then(|card| card.marks)
                .unwrap_or_default(),
            HandView::Count { .. } => Vec::new(),
        }
    };
    let owner_marks = marks_of(s.view(P1), &left);
    assert!(owner_marks.iter().any(|mark| mark.mark == "meet"));
    assert!(owner_marks.iter().any(|mark| mark.color == "gold"));
    assert_eq!(s.view(P2).opponent.hand_marked, None);

    // The left card leaves the hand: the meet ends with no meeting — no Radiant from it, no
    // meeting price. (Essence is leftmost again, so it lends the right card Radiance: a lend, not
    // a meeting.)
    s.play(&left, json!({}));
    assert_eq!(s.state().hand_meets, None);
    assert_eq!(s.card(&right).cost_override, None);
    assert!(s.card(&right).radiant);
    assert!(!s.card(&left).radiant);
}

/// R3558003: a Smoothie filled on arrival holds two Fruits, keeps its instance and cost, and both
/// target declarations ask when it is played.
#[test]
fn r3558003_smoothie_holds_two_fruits_keeps_instance_and_cost_and_asks_both_targets() {
    let mut s = board(json!({ "mana": 10 }), json!({ "field": [PAIR_L] }));
    let target = s.unit_at(P2, 1).id.clone();
    apply_now(
        s.state_mut(),
        effects::add_to_hand(json_as(json!({ "defId": SMOOTHIE }))),
    );
    let smoothie = hand_ids(s.state(), P1);
    assert_eq!(smoothie.len(), 1);
    let id = smoothie[0].clone();
    let ingredients = fused_ingredients(s.state(), &s.card(&id).def_id).expect("a filled Smoothie");
    assert_eq!(ingredients.len(), 3);
    assert!(ingredients.contains(&SMOOTHIE.to_string()));
    let fruits: Vec<&String> = ingredients.iter().filter(|def| *def != SMOOTHIE).collect();
    assert_eq!(fruits.len(), 2);
    for fruit in &fruits {
        let def = def_of(Some(s.state()), fruit);
        assert!(def.tags.contains(&Tag::Fruit));
        assert_eq!(def.type_, CardType::Spell);
    }
    assert_eq!(effective_cost(s.state(), s.card(&id), Default::default()), 3);

    // Both Fruits ask their own target: the hero and the unit each take 1.
    let before = hero_health(s.state(), P2);
    s.play(&id, json!({ "targets": [hero_pick(P2), instance_pick(&target)] }));
    assert_eq!(hero_health(s.state(), P2), before - 1);
    assert!(
        find_instance(s.state(), &target).is_none_or(|unit| !matches!(unit.zone, Zone::Field { .. })),
        "the unit took the other 1: {:?}",
        find_instance(s.state(), &target).map(|unit| (&unit.zone, unit.damage))
    );
}

/// R3558003: a filled Smoothie is never filled again.
#[test]
fn r3558003_a_filled_smoothie_is_not_filled_again() {
    let mut s = board(json!({}), json!({}));
    apply_now(
        s.state_mut(),
        effects::add_to_hand(json_as(json!({ "defId": SMOOTHIE }))),
    );
    let id = hand_ids(s.state(), P1)[0].clone();
    let filled = s.card(&id).def_id.clone();
    assert!(fused_ingredients(s.state(), &filled).is_some());
    apply_now(
        s.state_mut(),
        effects::fuse_random_into(json_as(json!({
            "into": { "target": { "of": "instance", "instanceId": id } },
            "query": { "tags": ["Fruit"], "type": "Spell" },
            "count": 2,
            "unfusedOnly": true,
        }))),
    );
    assert_eq!(s.card(&id).def_id, filled);
}

/// R3558004: Droat fuses until the printed costs reach 5 — and the fusion costs 2 in the hand.
#[test]
fn r3558004_fuses_until_printed_costs_reach_5_and_costs_2() {
    let mut s = board(json!({ "hand": [DROAT], "mana": 10 }), json!({}));
    s.play(DROAT, json!({}));
    let fused = s
        .hand(P1)
        .into_iter()
        .find(|card| card.def_id != DROAT)
        .expect("the fused card");
    let ingredients = fused_ingredients(s.state(), &fused.def_id).expect("a fusion");
    assert!(ingredients.len() >= FUSE_MIN_INGREDIENTS);
    let costs: Vec<i32> = ingredients
        .iter()
        .map(|def| query_cost(def_of(Some(s.state()), def)))
        .collect();
    let sum: i32 = costs.iter().sum();
    assert!(sum >= FRUIT_COST_CAP, "printed costs reach 5: {costs:?}");
    assert!(
        sum - costs.last().copied().unwrap_or(0) < FRUIT_COST_CAP,
        "below 5 without the last pick: {costs:?}"
    );
    assert_eq!(fused.cost_override, Some(2));
    assert_eq!(effective_cost(s.state(), &fused, Default::default()), 2);
}

/// R3558004: a pool of 0-cost cards never reaches the cost, so the fuse stops at the cards cap.
#[test]
fn r3558004_a_0_cost_pool_stops_at_fused_cards_cap() {
    let mut s = board(json!({}), json!({}));
    apply_now(
        s.state_mut(),
        effects::fuse_generated(json_as(json!({
            "query": { "defId": [ZERO_FRUIT] },
            "untilCost": FRUIT_COST_CAP,
            "toHand": "self",
        }))),
    );
    let fused = s.hand(P1).first().cloned().expect("the fused card");
    let ingredients = fused_ingredients(s.state(), &fused.def_id).expect("a fusion of 0-cost cards");
    assert_eq!(ingredients.len(), FUSED_CARDS_CAP);
}

/// R3558005: at its controller's End of Turn the fermented card fuses a different Grape into
/// itself — never itself, never the Apple.
#[test]
fn r3558005_end_of_turn_fuses_a_different_grape_never_itself_or_apple() {
    let mut s = board(json!({ "hand": [FERMENTED] }), json!({}));
    let id = s.hand(P1)[0].id.clone();
    s.end_turn();
    let ingredients = fused_ingredients(s.state(), &s.card(&id).def_id).expect("a fused Grape");
    assert_eq!(ingredients.len(), 2);
    assert!(ingredients.contains(&FERMENTED.to_string()));
    let other = ingredients
        .iter()
        .find(|def| *def != FERMENTED)
        .cloned()
        .unwrap()
        .clone();
    assert!(
        [RED_GRAPE, GREEN_GRAPE].contains(&other.as_str()),
        "a different Grape, never itself or the Apple: {other}"
    );
}

/// R3558005: the next End of Turn fuses the other Grape, and the one after fuses nothing.
#[test]
fn r3558005_next_end_of_turn_fuses_the_other_then_none() {
    let mut s = board(json!({ "hand": [FERMENTED] }), json!({}));
    let id = s.hand(P1)[0].id.clone();
    s.end_turn();
    let after_first = s.card(&id).def_id.clone();
    let first = fused_ingredients(s.state(), &after_first).expect("first fuse");
    let first_other = first
        .iter()
        .find(|def| def.as_str() != FERMENTED)
        .cloned()
        .unwrap();
    s.end_turn();
    s.end_turn();
    let after_second = s.card(&id).def_id.clone();
    assert_ne!(after_second, after_first, "the definition moved on");
    let second = fused_ingredients(s.state(), &after_second).expect("second fuse");
    let second_other = second
        .iter()
        .find(|def| def.as_str() != FERMENTED && def.as_str() != first_other)
        .cloned();
    assert!(
        second_other.is_some_and(|other| [RED_GRAPE, GREEN_GRAPE].contains(&other.as_str())),
        "the other Grape: {second:?}"
    );
    let done = after_second;
    s.end_turn();
    s.end_turn();
    assert_eq!(s.card(&id).def_id, done, "the pool is empty: nothing more fuses");
}

/// R3558005: the opponent's End of Turn fuses nothing into the fermented card.
#[test]
fn r3558005_opponents_end_of_turn_fuses_nothing() {
    let mut s = board(json!({ "hand": [FERMENTED] }), json!({}));
    let id = s.hand(P1)[0].id.clone();
    s.end_turn();
    let fused = s.card(&id).def_id.clone();
    // P2's turn ends: not its controller's, so nothing fuses.
    s.end_turn();
    assert_eq!(s.card(&id).def_id, fused);
}

/// R3558006: two bottom starters end last in either order across seeds.
#[test]
fn r3558006_two_bottom_starters_end_last_in_either_order_across_seeds() {
    register_e8_fixtures();
    let mut orders: Vec<(String, String)> = Vec::new();
    for seed in 0..40 {
        let seed_text = format!("e8-bottom-{seed}");
        let mut p1: Vec<String> = vanilla_deck(DECK_SIZE, 1);
        p1[0] = BOTTOM_A.to_string();
        p1[1] = BOTTOM_B.to_string();
        let state = begin_game(&create_game(&CreateGameOptions {
            seed: seed_text,
            decks: (p1, vanilla_deck(DECK_SIZE, 21)),
            ..CreateGameOptions::default()
        }))
        .state;
        // P1 returns its whole hand; P2 keeps everything.
        let kept: Vec<String> = Vec::new();
        let after_p1 = act(
            &state,
            json!({ "type": "mulligan", "keep": kept, "playerId": P1 }),
        );
        let p2_hand = hand_ids(&after_p1, P2);
        let done = act(
            &after_p1,
            json!({ "type": "mulligan", "keep": p2_hand, "playerId": P2 }),
        );
        let library = &done.players[P1].library;
        let tail: Vec<String> = library
            .iter()
            .rev()
            .take(2)
            .map(|card| card.def_id.clone())
            .collect();
        assert!(
            tail.contains(&BOTTOM_A.to_string()) && tail.contains(&BOTTOM_B.to_string()),
            "the pair ends last: {tail:?}"
        );
        orders.push((tail[0].clone(), tail[1].clone()));
    }
    assert!(
        orders.contains(&(BOTTOM_A.to_string(), BOTTOM_B.to_string())),
        "A-then-B occurs"
    );
    assert!(
        orders.contains(&(BOTTOM_B.to_string(), BOTTOM_A.to_string())),
        "B-then-A occurs"
    );
}

/// R3558006: with no bottom starter the deal is the deal as before — every card dealt, none lost.
#[test]
fn r3558006_no_bottom_starter_deals_as_before() {
    register_e8_fixtures();
    for seed in ["e8-plain-a", "e8-plain-b", "e8-plain-c"] {
        let deck = vanilla_deck(DECK_SIZE, 1);
        let state = begin_game(&create_game(&CreateGameOptions {
            seed: seed.to_string(),
            decks: (deck.clone(), vanilla_deck(DECK_SIZE, 21)),
            ..CreateGameOptions::default()
        }))
        .state;
        let p2_hand = hand_ids(&state, P2);
        let after_p1 = act(
            &state,
            json!({ "type": "mulligan", "keep": hand_ids(&state, P1), "playerId": P1 }),
        );
        let done = act(
            &after_p1,
            json!({ "type": "mulligan", "keep": p2_hand, "playerId": P2 }),
        );
        let mut dealt: Vec<String> = done.players[P1]
            .hand
            .iter()
            .chain(done.players[P1].library.iter())
            .map(|card| card.def_id.clone())
            .collect();
        let mut want = deck.clone();
        dealt.sort();
        want.sort();
        assert_eq!(dealt, want, "no card lost or doubled");
        // The opening draw, then turn 1's draw (§2.1 step 5, R10).
        assert_eq!(done.players[P1].hand.len() as i32, OPENING_DRAW[0] + 1);
    }
}

fn animosity_board() -> Scenario {
    board(
        json!({
            "hand": [EXILER, EXILER, EXILER, EXILER, EXILER],
            "graveyard": [BOTTOM_A, BOTTOM_A, BOTTOM_A, BOTTOM_A, BOTTOM_A],
            "library": [ANIMOSITY_FX, BOLT, BOLT],
            "mana": 10,
        }),
        json!({}),
    )
}

fn exile_one() -> Effect {
    effects::exile_random_from(json_as(json!({
        "player": "self",
        "from": "graveyard",
        "count": 1,
    })))
}

fn play_exiler(s: &mut Scenario) {
    let id = s.hand(P1)[0].id.clone();
    s.play(&id, json!({}));
}

/// R3558007: the fifth exiled card casts Animosity from deep in the deck — 2 to the enemy hero,
/// once, ending in the graveyard.
#[test]
fn r3558007_fifth_exiled_card_casts_it_from_deep_in_the_deck() {
    let mut s = animosity_board();
    let before = hero_health(s.state(), P2);
    for _ in 0..4 {
        play_exiler(&mut s);
    }
    assert_eq!(exile_count(s.state(), P1), 4);
    assert_eq!(hero_health(s.state(), P2), before, "four exiled cast nothing");
    assert!(
        s.pile(P1, "library")
            .iter()
            .any(|card| card.def_id == ANIMOSITY_FX)
    );

    play_exiler(&mut s);
    assert_eq!(exile_count(s.state(), P1), 5);
    assert_eq!(hero_health(s.state(), P2), before - 2, "the fifth casts it");
    assert!(
        s.pile(P1, "graveyard")
            .iter()
            .any(|card| card.def_id == ANIMOSITY_FX)
    );
    assert!(
        !s.pile(P1, "library")
            .iter()
            .any(|card| card.def_id == ANIMOSITY_FX)
    );
    assert_eq!(
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Damage { .. }))
            .count(),
        1,
        "cast once"
    );
}

/// R3558007: four exiled cards cast nothing.
#[test]
fn r3558007_four_exiled_cast_nothing() {
    let mut s = animosity_board();
    let before = hero_health(s.state(), P2);
    for _ in 0..4 {
        play_exiler(&mut s);
    }
    assert_eq!(exile_count(s.state(), P1), 4);
    assert_eq!(hero_health(s.state(), P2), before);
    assert!(
        s.pile(P1, "library")
            .iter()
            .any(|card| card.def_id == ANIMOSITY_FX)
    );
}

/// R3558007: a JSON round trip between exiles casts the same fifth cast.
#[test]
fn r3558007_round_trip_casts_the_same() {
    let mut s = animosity_board();
    let before = hero_health(s.state(), P2);
    for _ in 0..4 {
        run_as_p1(s.state_mut(), exile_one());
    }
    assert_eq!(exile_count(s.state(), P1), 4);
    let revived = round_trip(s.state());
    assert_eq!(revived, *s.state());
    *s.state_mut() = revived;
    play_exiler(&mut s);
    assert_eq!(exile_count(s.state(), P1), 5);
    assert_eq!(hero_health(s.state(), P2), before - 2);
    assert!(
        s.pile(P1, "graveyard")
            .iter()
            .any(|card| card.def_id == ANIMOSITY_FX)
    );
}

/// R3558008: each cast asks its own target, the first resolving before the second asks.
#[test]
fn r3558008_each_cast_asks_its_own_target_first_resolving_before_the_second_asks() {
    let mut s = board(
        json!({ "hand": [TWICE], "mana": 10 }),
        json!({ "field": [PAIR_L, PAIR_R] }),
    );
    let p2_before = hero_health(s.state(), P2);
    s.play(TWICE, json!({}));
    // The first cast asks.
    assert!(s.state().pending.is_some(), "the first cast asks");
    s.answer(json!([hero_pick(P2)]));
    // The first resolved before the second asks.
    assert_eq!(hero_health(s.state(), P2), p2_before - 1);
    assert!(s.state().pending.is_some(), "the second cast asks");
    let target = s.unit_at(P2, 1).id.clone();
    s.answer(json!([instance_pick(&target)]));
    assert!(s.state().pending.is_none(), "nothing left to ask");
    assert!(s.unit(P2, 1).is_none(), "the unit took the second 1");
    assert_eq!(hero_health(s.state(), P2), p2_before - 1);
}
