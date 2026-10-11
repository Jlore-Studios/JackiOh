//! ME-MARKET (Meditative #42 CN Flea Market, R1000–R1002) through `nm-market`, the fixture Spell
//! running the card's own night market (`fixtures/night_market.rs`): the stall, the yuan prices,
//! the `market` prompt and its deals, barter on the Radiant face, and a timeout that leaves. The card
//! itself is proved again in `crates/cards`.

use jackioh_engine::subsystems::ai_policy::choose_action;
use jackioh_engine::subsystems::night_market::{MARKET_STALL_KEY, yuan_price};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{in_hand, setup_catalog};
use crate::rules::fixtures::night_market::{
    ROCK, ai_card, cn_four, market, market_qd, register_night_market_fixtures,
};
use crate::rules::fixtures::prompt_harness::{
    AnswerResult, Replayable, act, answer_keys, board, cast_now, expect_replays, hand_card, must, open_as,
    round_trip,
};
use crate::rules::fixtures::prompts::register_prompt_fixtures;

/// p1's main phase with the market cast (Radiant on `radiant`) and its prompt open.
fn opened(seed: &str, radiant: bool) -> GameState {
    let mut state = board(seed);
    register_night_market_fixtures();
    cast_now(&mut state, &market().id, P1, radiant);
    state
}

/// The open market's stall, read off its resume data.
fn stall_of(state: &GameState) -> Vec<String> {
    let pending = must(state.pending.as_ref(), "a market prompt");
    json_as(must(
        pending.resume.data.get(MARKET_STALL_KEY).cloned(),
        "the stall",
    ))
}

/// The open prompt's option keys.
fn keys_of(state: &GameState) -> Vec<String> {
    must(state.pending.as_ref(), "a prompt")
        .options
        .iter()
        .map(|option| option.key.clone())
        .collect()
}

/// Answer the open prompt with a selection that need not be one of its options.
fn answer_with(state: &mut GameState, selection: Selection) -> AnswerResult {
    let pending = must(state.pending.clone(), "an open prompt");
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
    let error = {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        match answer_prompt(
            &mut sink,
            &AnswerInput {
                player_id: pending.player_id,
                choice_id: pending.id.clone(),
                selection: vec![selection],
            },
        ) {
            Ok(_) => {
                settle(&mut sink, SettleOptions::default());
                None
            }
            Err(error) => Some(error.message),
        }
    };
    state.rng_cursor = rng.cursor();
    AnswerResult { events, error }
}

/// The ids of a catalog pool, as the stall's shelves draw them.
fn pool(query_json: Value) -> Vec<String> {
    query(&json_as(query_json))
        .into_iter()
        .map(|def| def.id.clone())
        .collect()
}

/// The first `prefix-<n>` seed whose base stall holds `def_id`.
fn stall_holding(prefix: &str, def_id: &str, radiant: bool) -> GameState {
    for n in 0..200 {
        let state = opened(&format!("{prefix}-{n}"), radiant);
        if stall_of(&state).iter().any(|id| id == def_id) {
            return state;
        }
    }
    panic!("no seed's stall holds {def_id}");
}

#[test]
fn r1000_the_stall_is_three_different_cn_cards_two_rocks_and_one_ai_card() {
    for n in 0..20 {
        let state = opened(&format!("nm-stall-{n}"), false);
        let stall = stall_of(&state);
        assert_eq!(stall.len(), 6, "{stall:?}");
        let cn_pool = pool(json!({ "tags": ["CN"] }));
        let ai_pool = pool(json!({ "tags": ["AI"], "token": true }));
        let cn: Vec<&String> = stall[..3].iter().collect();
        assert!(cn.iter().all(|id| cn_pool.contains(id)), "{stall:?}");
        assert!(
            cn.iter().all(|id| **id != market().id),
            "never the market itself (R387)"
        );
        assert!(
            cn[0] != cn[1] && cn[1] != cn[2] && cn[0] != cn[2],
            "all different (R60): {stall:?}"
        );
        assert_eq!(stall[3..5], [ROCK.to_string(), ROCK.to_string()]);
        assert!(ai_pool.contains(&stall[5]), "{stall:?}");
    }
}

#[test]
fn r1000_prices_are_ten_yuan_a_mana_plus_five_a_rarity_rank_doubled_when_radiant() {
    let card = |extra: Value| -> CardDef {
        let mut base = json!({
            "id": "nm-price", "index": "1", "name": "price", "set": "Core", "type": "Spell", "tags": [],
            "rarity": "Common", "token": false, "cost": 1,
            "base": { "keywords": [], "text": "" }, "radiant": { "keywords": [], "text": "" },
        });
        if let (Some(into), Value::Object(from)) = (base.as_object_mut(), extra) {
            for (key, value) in from {
                into.insert(key, value);
            }
        }
        json_as(base)
    };
    for (extra, radiant, price) in [
        (json!({ "rarity": "Common", "cost": 1 }), false, 15),
        (json!({ "rarity": "Rare", "cost": 2 }), false, 30),
        (json!({ "rarity": "Legendary", "cost": 4 }), false, 60),
        (json!({ "rarity": "Mythic", "cost": 0 }), false, 25),
        (json!({ "rarity": "Epic", "cost": "X" }), false, 15),
        (
            json!({ "rarity": "Common", "cost": { "base": 2, "embiggen": 5 } }),
            false,
            25,
        ),
        (
            json!({ "rarity": "Token", "printedRarity": "Rare", "token": true, "cost": 0 }),
            false,
            10,
        ),
        (json!({ "rarity": "Token", "token": true, "cost": 1 }), false, 15),
        (json!({ "rarity": "Rare", "cost": 2 }), true, 60),
    ] {
        assert_eq!(
            yuan_price(&card(extra.clone()), radiant),
            price,
            "{extra} radiant {radiant}"
        );
    }
}

#[test]
fn r1000_only_affordable_lots_are_offered_and_an_unoffered_lot_is_refused() {
    let mut state = stall_holding("nm-dear", &cn_four().id, false);
    let dear = format!("mode:{}", cn_four().id);
    assert!(!keys_of(&state).contains(&dear), "60 yuan is over 50");
    let before = hash_state(&state);
    let refused = answer_with(&mut state, Selection::Mode { option: cn_four().id });
    assert!(refused.error.is_some(), "an unoffered lot is refused");
    assert_eq!(hash_state(&state), before, "the state is unchanged");

    let seed = state.seed.clone();
    let rich = opened(&seed, true);
    assert_eq!(stall_of(&rich), stall_of(&state), "the face draws the same stall");
    assert!(keys_of(&rich).contains(&dear), "80 yuan covers it");
}

#[test]
fn r1000_a_buy_adds_the_base_face_card_and_reopens_with_the_yuan_left() {
    let mut state = opened("nm-buy", false);
    let held = state.players.p1.hand.len();
    let answered = answer_keys(&mut state, &[&format!("mode:{ROCK}")]);
    assert_eq!(answered.error, None);
    assert_eq!(state.players.p1.hand.len(), held + 1);
    let rock = hand_card(&state, P1, ROCK);
    assert!(!rock.radiant, "a bought card arrives on its base face");
    let reopened = open_as(&state, PromptKind::Market, P1);
    assert_eq!(reopened.budget, Some(40));
    // One Rock was sold: the second is still on the stall, alone.
    assert_eq!(stall_of(&state).iter().filter(|id| *id == ROCK).count(), 1);
}

#[test]
fn r1000_leave_closes_the_market_and_unspent_yuan_is_lost() {
    let mut state = opened("nm-leave", false);
    let hand: Vec<String> = state.players.p1.hand.iter().map(|card| card.id.clone()).collect();
    let answered = answer_keys(&mut state, &["none"]);
    assert_eq!(answered.error, None);
    assert_eq!(state.pending, None);
    let after: Vec<String> = state.players.p1.hand.iter().map(|card| card.id.clone()).collect();
    assert_eq!(after, hand, "nothing was bought");
}

#[test]
fn r1000_a_full_hand_offers_no_lot_and_with_nothing_to_do_no_market_opens() {
    let full = |seed: &str| -> GameState {
        let mut state = board(seed);
        register_night_market_fixtures();
        let room = hand_cap_of(&state, P1) - state.players.p1.hand.len() as i32;
        in_hand(&mut state, &ai_card().id, P1, room);
        state
    };
    let mut state = full("nm-full");
    cast_now(&mut state, &market().id, P1, false);
    assert_eq!(state.pending, None, "no lot and no barter: no market");

    let mut state = full("nm-full-radiant");
    cast_now(&mut state, &market().id, P1, true);
    let keys = keys_of(&state);
    assert!(
        keys.iter()
            .all(|key| key.starts_with("instance:") || key == "none"),
        "{keys:?}"
    );
    assert_eq!(keys.len() as i32, hand_cap_of(&state, P1) + 1);
}

#[test]
fn r1001_barter_is_offered_on_the_radiant_face_only() {
    let plain = opened("nm-barter-face", false);
    assert!(!keys_of(&plain).iter().any(|key| key.starts_with("instance:")));
    let radiant = opened("nm-barter-face", true);
    let barters = keys_of(&radiant)
        .into_iter()
        .filter(|key| key.starts_with("instance:"))
        .count();
    assert_eq!(barters, radiant.players.p1.hand.len(), "one barter per hand card");
}

#[test]
fn r1001_a_barter_exiles_the_card_and_adds_its_price_doubled_if_radiant() {
    let mut state = board("nm-barter");
    register_night_market_fixtures();
    let mut traded = in_hand(&mut state, &cn_four().id, P1, 1).remove(0);
    traded.radiant = true;
    if let Some(card) = state.players.p1.hand.iter_mut().find(|card| card.id == traded.id) {
        card.radiant = true;
    }
    cast_now(&mut state, &market().id, P1, true);
    let pending = open_as(&state, PromptKind::Market, P1);
    let key = format!("instance:{}", traded.id);
    let option = must(
        pending.options.iter().find(|option| option.key == key),
        "the barter",
    );
    assert_eq!(
        option.cost,
        Some(-120),
        "a Radiant (4) Legendary: 60 doubled, written negative"
    );
    let answered = answer_keys(&mut state, &[&key]);
    assert_eq!(answered.error, None);
    assert!(state.players.p1.exile.iter().any(|card| card.id == traded.id));
    assert!(
        answered
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::Exiled { instance_id, .. } if *instance_id == traded.id))
    );
    assert_eq!(open_as(&state, PromptKind::Market, P1).budget, Some(80 + 120));
}

#[test]
fn r1000_paused_at_the_market_the_state_survives_json_and_answers_to_the_same_game() {
    let mut state = opened("nm-json", true);
    let mut copy = round_trip(&state);
    assert_eq!(hash_state(&copy), hash_state(&state));
    let key = format!("mode:{ROCK}");
    answer_keys(&mut state, &[&key]);
    answer_keys(&mut copy, &[&key]);
    assert_eq!(hash_state(&copy), hash_state(&state));
    let mut second = round_trip(&state);
    answer_keys(&mut state, &["none"]);
    answer_keys(&mut second, &["none"]);
    assert_eq!(hash_state(&second), hash_state(&state));
}

/// `replayable` with the market fixtures registered before the decks are built: Quickdraw puts
/// `nm-market-qd` in the opening hand, and the fold rebuilds the very same state.
fn replayable_with_market(seed: &str) -> Replayable {
    setup_catalog();
    register_prompt_fixtures();
    register_night_market_fixtures();
    let mut first = vanilla_deck(DECK_SIZE - 1, 1);
    first.push(market_qd().id);
    let decks = (first, vanilla_deck(DECK_SIZE, 21));
    let mut log: Vec<Action> = Vec::new();
    let mut state = begin_game(&create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: decks.clone(),
        ..CreateGameOptions::default()
    }))
    .state;
    for player in [P1, P2] {
        let keep: Vec<String> = state.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        state = act(
            &state,
            json!({ "type": "mulligan", "playerId": player, "keep": keep }),
            Some(&mut log),
        );
    }
    Replayable { state, log, decks }
}

#[test]
fn r1000_a_game_with_a_night_market_replays_from_its_log_to_the_same_hash() {
    let game = replayable_with_market("nm-replay");
    let decks = game.decks.clone();
    let mut log = game.log;
    let mut state = game.state;
    let card = hand_card(&state, P1, &market_qd().id).id.clone();
    state = act(
        &state,
        json!({ "type": "play", "playerId": "p1", "instanceId": card }),
        Some(&mut log),
    );
    let pending = open_as(&state, PromptKind::Market, P1);
    let first = pending.options[0].clone();
    state = act(
        &state,
        json!({ "type": "answer", "playerId": "p1", "choiceId": pending.id, "selection": [first.selection] }),
        Some(&mut log),
    );
    let pending = open_as(&state, PromptKind::Market, P1);
    state = act(
        &state,
        json!({ "type": "answer", "playerId": "p1", "choiceId": pending.id, "selection": [{ "pick": "none" }] }),
        Some(&mut log),
    );
    assert_eq!(state.pending, None);
    expect_replays("nm-replay", &decks, &log, &state);
}

#[test]
fn r97_r1000_the_opponent_sees_only_an_open_prompt_and_a_hidden_card_arriving() {
    let mut state = opened("nm-hidden", false);
    let stall = stall_of(&state);
    let named = |view: &PlayerView| -> Vec<String> {
        let text = serde_json::to_string(view).expect("a view serialises");
        stall
            .iter()
            .filter(|id| text.contains(id.as_str()))
            .cloned()
            .collect()
    };
    let theirs = view_for(&state, P2);
    assert!(matches!(theirs.pending, Some(PendingView::Elsewhere(_))));
    assert_eq!(named(&theirs), Vec::<String>::new(), "no lot before a buy");

    let mine = view_for(&state, P1);
    match &mine.pending {
        Some(PendingView::ForYou(prompt)) => {
            assert_eq!(prompt.budget, Some(50));
            let lots = prompt
                .options
                .iter()
                .filter(|option| option.key.starts_with("mode:"));
            assert!(lots.clone().count() > 0);
            assert!(
                lots.into_iter().all(|option| option.def_id.is_some()),
                "each lot names its card"
            );
        }
        other => panic!("p1's own prompt, not {other:?}"),
    }

    answer_keys(&mut state, &[&format!("mode:{ROCK}")]);
    assert_eq!(
        named(&view_for(&state, P2)),
        Vec::<String>::new(),
        "a hidden card arrived"
    );
}

#[test]
fn r1002_a_timeout_leaves_the_market() {
    let state = opened("nm-timeout", false);
    let hand = state.players.p1.hand.len();
    let timed = reduce(
        &state,
        &json_as::<Action>(json!({ "type": "timeout", "playerId": "p1", "nonce": "nm-t1" })),
    );
    assert_eq!(timed.error, None);
    assert!(
        timed
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::PromptAnswered { .. })),
        "the market was answered"
    );
    assert!(timed.state.players.p1.hand.len() <= hand, "nothing was bought");
    assert!(!timed.state.players.p1.hand.iter().any(|card| card.def_id == ROCK));
    assert_eq!(timed.state.active, P2, "the timeout ended the turn");
}

#[test]
fn r1000_the_random_policy_answers_a_market_until_it_closes() {
    let mut state = opened("nm-policy", true);
    let mut rng = Rng::new("nm-policy-picks", 0);
    let mut answers = 0;
    while state
        .pending
        .as_ref()
        .is_some_and(|pending| pending.kind == PromptKind::Market)
    {
        assert!(answers < 40, "the market closes");
        let chosen = must(choose_action(&state, P1, &mut rng), "an answer");
        let result = reduce(&state, &Action::new(chosen, P1, format!("nm-p{answers}")));
        assert_eq!(result.error, None, "every answer is accepted");
        state = result.state;
        answers += 1;
    }
    assert!(answers > 0);
}
