//! Stinkyscape+ E5 (issue #3548, R3548000–R3548004): Tipsy X and its blur, the Warhero Token's
//! game-wide worth, Refresh for Activates, and infinite mana, proved on fixture scripts
//! (`fixtures::tipsy_warhero_refresh`, the `testkit` feature's `scenario()`).

use jackioh_engine::effects;
use jackioh_engine::effects::{REFRESH_ABILITY_ID, is_refresh_ability};
use jackioh_engine::subsystems::activate::ability_uses_this_turn;
use jackioh_engine::testkit::*;
use serde_json::json;

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{apply_now, play_random_game_from, round_trip, setup_catalog};
use crate::rules::fixtures::tipsy_warhero_refresh::*;

use PlayerId::{P1, P2};

/// The side's pile as the `scenario()` JSON names it: the active side has started 5 turns and the
/// other 4, each on an empty library but four plain units, played in `main` with no auto-end.
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_e5_fixtures();
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

fn hand_id(s: &Scenario, player: PlayerId, def: &str) -> String {
    s.hand(player)
        .iter()
        .find(|card| card.def_id == def)
        .unwrap_or_else(|| panic!("no {def} in hand"))
        .id
        .clone()
}

/// Every card on `player`'s field: each unit pile's top, then the backrow.
fn field_cards(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    let side = &state.players[player];
    side.units
        .iter()
        .flatten()
        .filter_map(|pile| pile.first().cloned())
        .chain(side.backrow.iter().flatten().cloned())
        .collect()
}

fn field_id(s: &Scenario, player: PlayerId, def: &str) -> String {
    field_cards(s.state(), player)
        .into_iter()
        .find(|card| card.def_id == def)
        .unwrap_or_else(|| panic!("no {def} on the field"))
        .id
        .clone()
}

fn tipsy_count(s: &Scenario, player: PlayerId) -> Option<i32> {
    s.state().players[player].tipsy.map(|counter| counter.count)
}

/// The Tipsy `counterChanged` reports so far: each new value with what the play added.
fn tipsy_reports(s: &Scenario) -> Vec<(i32, Option<i32>)> {
    s.events()
        .iter()
        .filter_map(|event| match event {
            GameEvent::CounterChanged {
                counter: CounterKind::Tipsy,
                value,
                placed,
                ..
            } => Some((*value, *placed)),
            _ => None,
        })
        .collect()
}

/// The Warhero `counterChanged` reports so far: each new worth with the growth that raised it.
fn warhero_reports(s: &Scenario) -> Vec<(i32, Option<i32>)> {
    s.events()
        .iter()
        .filter_map(|event| match event {
            GameEvent::CounterChanged {
                counter: CounterKind::Warhero,
                value,
                placed,
                ..
            } => Some((*value, *placed)),
            _ => None,
        })
        .collect()
}

fn lists_activate(state: &GameState, instance: &str) -> bool {
    legal_actions(state, P1)
        .iter()
        .any(|body| matches!(body, ActionBody::Activate { instance_id, .. } if instance_id == instance))
}

fn lists_play(state: &GameState, instance: &str) -> bool {
    legal_actions(state, P1)
        .iter()
        .any(|body| matches!(body, ActionBody::Play { instance_id, .. } if instance_id == instance))
}

/// Whether p1's own view of p1's field blurs any card.
fn has_blur(s: &Scenario) -> bool {
    s.view(P1)
        .you
        .units
        .into_iter()
        .flatten()
        .any(|unit| unit.tipsy_blur.is_some())
}

/// R3548000: playing a card with Tipsy X adds its X to its controller's counter, reported once per
/// play on both seats' views of it.
#[test]
fn r3548000_playing_a_tipsy_card_adds_its_x_to_its_players_counter() {
    let mut s = board(
        "r3548000-adds",
        json!({ "hand": [TIPSY_SPELL, TIPSY_SPELL] }),
        json!({}),
    );
    s.play(TIPSY_SPELL, json!({}));
    assert_eq!(tipsy_count(&s, P1), Some(2));
    s.play(TIPSY_SPELL, json!({}));
    assert_eq!(tipsy_count(&s, P1), Some(4));
    assert_eq!(tipsy_reports(&s), vec![(2, Some(2)), (4, Some(2))]);
    assert_eq!(s.view(P1).you.tipsy, Some(4));
    assert_eq!(s.view(P2).opponent.tipsy, Some(4));
}

/// R3548000: the counter drops at its owner's turn start only — never on the opponent's — and is
/// gone at 0, its view key absent with it.
#[test]
fn r3548000_the_counter_drops_at_its_owners_turn_start_and_is_absent_at_zero() {
    let mut s = board("r3548000-decays", json!({ "hand": [TIPSY_SPELL] }), json!({}));
    s.play(TIPSY_SPELL, json!({}));
    assert_eq!(tipsy_count(&s, P1), Some(2));
    s.end_turn();
    assert_eq!(tipsy_count(&s, P1), Some(2));
    assert_eq!(tipsy_count(&s, P2), None);
    s.end_turn();
    assert_eq!(tipsy_count(&s, P1), Some(1));
    s.end_turn();
    s.end_turn();
    assert_eq!(s.state().players[P1].tipsy, None);
    assert_eq!(s.view(P1).you.tipsy, None);
    let json = serde_json::to_value(s.state()).expect("the state serializes");
    assert!(json["players"]["p1"].get("tipsy").is_none());
    assert_eq!(tipsy_reports(&s).last(), Some(&(0, None)));
}

/// R3548001: a card drawn, played or summoned while Tipsy is up is stamped with whose counter, how
/// high and on which turn; a card arriving sober carries nothing. The played card reads the level
/// already there — its own Tipsy counts after it.
#[test]
fn r3548001_a_card_drawn_played_or_summoned_while_tipsy_is_stamped() {
    let mut s = board(
        "r3548001-stamped",
        json!({ "hand": [PLAIN, TIPSY_SPELL, PLAIN], "library": [PLAIN, PLAIN] }),
        json!({}),
    );
    let turn = s.state().turn;
    let sober_id = hand_id(&s, P1, PLAIN);
    s.play(sober_id.as_str(), json!({ "zone": 1 }));
    let sober = field_cards(s.state(), P1)
        .into_iter()
        .find(|card| card.id == sober_id)
        .expect("the sober plain on the field");
    assert_eq!(sober.tipsy, None);
    s.play(TIPSY_SPELL, json!({}));
    let stamp = TipsyStamp {
        player: P1,
        level: 2,
        turn,
    };
    let stamped_id = hand_id(&s, P1, PLAIN);
    s.play(stamped_id.as_str(), json!({ "zone": 2 }));
    let stamped = field_cards(s.state(), P1)
        .into_iter()
        .find(|card| card.id == stamped_id)
        .expect("the stamped plain on the field");
    assert_eq!(stamped.tipsy, Some(stamp));
    // The spell raised the counter to 2 and a plain card adds none.
    assert_eq!(tipsy_count(&s, P1), Some(2));
    let before: Vec<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();
    s.end_turn();
    s.end_turn();
    let drawn = s
        .hand(P1)
        .into_iter()
        .find(|card| !before.contains(&card.id))
        .expect("a drawn card");
    assert_eq!(
        drawn.tipsy,
        Some(TipsyStamp {
            player: P1,
            level: 1,
            turn: turn + 2,
        })
    );
    apply_now(s.state_mut(), effects::summon(json_as(json!({ "defId": PLAIN }))));
    let summoned = field_cards(s.state(), P1)
        .into_iter()
        .find(|card| card.def_id == PLAIN && card.id != sober_id && card.id != stamped_id)
        .expect("a summoned plain");
    assert_eq!(
        summoned.tipsy,
        Some(TipsyStamp {
            player: P1,
            level: 1,
            turn: turn + 2,
        })
    );
}

/// The first seed whose played plain blurs on its owner's view, for the board a test builds from `hand`:
/// the blur is seeded by the card's instance, so a test whose hand holds more cards looks for its own
/// seed. The spell is played first, then the plain.
fn blurring_seed_for(hand: Value) -> String {
    for index in 0..50 {
        let seed = format!("r3548001-blur-{index}");
        let mut s = board(&seed, json!({ "hand": hand.clone() }), json!({}));
        s.play(TIPSY_SPELL, json!({}));
        s.play(PLAIN, json!({ "zone": 2 }));
        if has_blur(&s) {
            return seed;
        }
    }
    panic!("no seed in range blurs the played plain");
}

/// `blurring_seed_for` the spell and the plain alone: the seed-pinned tests share it.
fn blurring_seed() -> String {
    blurring_seed_for(json!([TIPSY_SPELL, PLAIN]))
}

/// R3548001: a stamped card blurs on its own player's view alone — the opponent's view of the same
/// card carries nothing.
#[test]
fn r3548001_only_the_tipsy_players_own_view_carries_the_blur() {
    let seed = blurring_seed();
    let mut s = board(&seed, json!({ "hand": [TIPSY_SPELL, PLAIN] }), json!({}));
    s.play(TIPSY_SPELL, json!({}));
    s.play(PLAIN, json!({ "zone": 2 }));
    assert!(has_blur(&s), "the seed blurs the played plain");
    for unit in s.view(P2).opponent.units.into_iter().flatten() {
        assert_eq!(unit.tipsy_blur, None);
    }
}

/// R3548001: rebuilding the view from a reloaded state shows the same blur, and building views
/// never moves the match rng.
#[test]
fn r3548001_a_reload_shows_the_same_blur_and_the_rng_is_untouched() {
    let seed = blurring_seed();
    let mut s = board(&seed, json!({ "hand": [TIPSY_SPELL, PLAIN] }), json!({}));
    s.play(TIPSY_SPELL, json!({}));
    s.play(PLAIN, json!({ "zone": 2 }));
    let cursor = s.state().rng_cursor;
    let before: Vec<Option<TipsyBlur>> = s
        .view(P1)
        .you
        .units
        .into_iter()
        .flatten()
        .map(|unit| unit.tipsy_blur)
        .collect();
    assert!(before.iter().any(Option::is_some));
    let reloaded: GameState = round_trip(s.state());
    let after: Vec<Option<TipsyBlur>> = view_for(&reloaded, P1)
        .you
        .units
        .into_iter()
        .flatten()
        .map(|unit| unit.tipsy_blur)
        .collect();
    assert_eq!(before, after);
    assert_eq!(s.state().rng_cursor, cursor);
}

/// Every Tipsy blur either seat's view carries: whose view, which card, and what blurs.
fn blurs_of(state: &GameState) -> Vec<(PlayerId, String, Option<TipsyBlur>)> {
    let mut out = Vec::new();
    for viewer in [P1, P2] {
        let view = view_for(state, viewer);
        for hand in [&view.you.hand, &view.opponent.hand] {
            if let HandView::Cards(cards) = hand {
                for card in cards {
                    out.push((viewer, card.instance_id.clone(), card.tipsy_blur.clone()));
                }
            }
        }
        for unit in view.you.units.iter().chain(view.opponent.units.iter()).flatten() {
            out.push((viewer, unit.instance_id.clone(), unit.tipsy_blur.clone()));
        }
    }
    out
}

/// R3548001: folding the recorded log reaches a state whose views blur exactly as the live game's —
/// the blur folds with the stamp, never with a fresh draw.
#[test]
fn r3548001_a_replay_folds_to_the_same_blur() {
    register_e5_fixtures();
    let mut deck = vec![
        TIPSY_SPELL.to_string(),
        TIPSY_UNIT.to_string(),
        WARHERO.to_string(),
        PLAIN.to_string(),
        EXPENSIVE.to_string(),
        X_SPELL.to_string(),
        ACTIVATOR.to_string(),
        PRICED.to_string(),
        COOLDOWN.to_string(),
        REFRESHER.to_string(),
        REACTOR.to_string(),
        SLEEPY_REACTOR.to_string(),
    ];
    deck.extend(vanilla_deck(13, 31));
    // The blur is seed-pinned and a finished game is where the counter is down, so the test takes the
    // first seed whose playout shows a blur at some step, and folds the log up to that step.
    let live_game = |seed: &str| {
        // `play_random_game` would replace the registered catalog (`new_game`'s `setup_catalog`), so the
        // game is created here, with the fixtures registered.
        let created = create_game(&CreateGameOptions {
            seed: seed.to_string(),
            decks: (deck.clone(), deck.clone()),
            ..CreateGameOptions::default()
        });
        (
            begin_game(&created).state,
            play_random_game_from(begin_game(&created).state, seed, (deck.clone(), deck.clone())),
        )
    };
    let blurs = |state: &GameState| blurs_of(state).iter().any(|(_, _, blur)| blur.is_some());
    let found = (0..60).find_map(|index| {
        let seed = format!("r3548001-replay-{index}");
        let (mut state, live) = live_game(&seed);
        for (step, action) in live.log.iter().enumerate() {
            state = reduce(&state, action).state;
            if blurs(&state) {
                return Some((seed, live, step, state));
            }
        }
        None
    });
    let (seed, live, step, at_step) = found.expect("a playout that blurs a card at some step");
    setup_catalog();
    register_e5_fixtures();
    let folded = fold(&FoldArgs {
        seed: seed.clone(),
        decks: live.decks.clone(),
        log: live.log[..=step].to_vec(),
        ..Default::default()
    });
    assert_eq!(folded.errors.len(), 0);
    assert_eq!(hash_state(&folded.state), hash_state(&at_step));
    assert!(blurs(&at_step), "the live game blurs a card at the step");
    assert_eq!(blurs_of(&folded.state), blurs_of(&at_step));
}

/// R3548001: from 2 to 1 the stamped card keeps its blur — only the odds for new cards fall; at 0
/// every blur clears, and a counter risen again blurs only what arrived since.
#[test]
fn r3548001_two_to_one_keeps_the_blur_and_zero_clears_it() {
    let seed = blurring_seed_for(json!([TIPSY_SPELL, TIPSY_SPELL, PLAIN]));
    let mut s = board(
        &seed,
        json!({ "hand": [TIPSY_SPELL, TIPSY_SPELL, PLAIN] }),
        json!({}),
    );
    s.play(TIPSY_SPELL, json!({}));
    s.play(PLAIN, json!({ "zone": 2 }));
    assert!(has_blur(&s));
    s.end_turn();
    s.end_turn();
    assert_eq!(tipsy_count(&s, P1), Some(1));
    assert!(has_blur(&s), "from 2 to 1 nothing clears");
    s.end_turn();
    s.end_turn();
    assert_eq!(tipsy_count(&s, P1), None);
    assert!(!has_blur(&s), "at 0 everything clears");
    s.play(TIPSY_SPELL, json!({}));
    assert_eq!(tipsy_count(&s, P1), Some(2));
    assert!(!has_blur(&s), "still clear after the counter rises again");
}

/// R3548001: the chance is ten points a level, capped at fifty.
#[test]
fn r3548001_the_chance_grows_with_the_counter_and_stops_at_the_cap() {
    assert_eq!(blur_chance(0), 0);
    assert_eq!(blur_chance(1), 10);
    assert_eq!(blur_chance(2), 20);
    assert_eq!(blur_chance(5), 50);
    assert_eq!(blur_chance(20), 50);
}

/// R3548002: no Warhero Token minted anywhere yet, so the worth is absent; minting the first one —
/// wherever it arrives — starts it at 0.
#[test]
fn r3548002_the_worth_starts_at_zero_when_the_first_warhero_is_minted() {
    let mut s = board("r3548002-minted", json!({}), json!({}));
    assert_eq!(s.state().warhero_worth, None);
    let card = new_instance(s.state_mut(), WARHERO, P1, Zone::Hand { player: P1 });
    s.state_mut().players[P1].hand.push(card);
    assert_eq!(s.state().warhero_worth, Some(0));
}

/// R3572002: a card reads the game-wide worth through `warhero_worth`: 0 until a Warhero Token has
/// been printed, then the game's value.
#[test]
fn r3572002_warhero_worth_is_zero_until_a_token_is_printed_then_the_game_value() {
    let mut s = board("r3572002-worth", json!({}), json!({}));
    assert_eq!(s.state().warhero_worth, None);
    assert_eq!(warhero_worth(s.state()), 0);
    let card = new_instance(s.state_mut(), WARHERO, P1, Zone::Hand { player: P1 });
    s.state_mut().players[P1].hand.push(card);
    assert_eq!(warhero_worth(s.state()), 0, "printed, and still worth 0");
    s.state_mut().warhero_worth = Some(7);
    assert_eq!(warhero_worth(s.state()), 7);
}

/// R3548002: once a Token has been minted the worth grows by 0 to 10 at the end of every turn,
/// reported with its growth; with none minted it stays absent and draws nothing.
#[test]
fn r3548002_the_worth_grows_by_zero_to_ten_at_the_end_of_every_turn() {
    let mut s = board("r3548002-grows", json!({}), json!({}));
    s.end_turn();
    s.end_turn();
    assert_eq!(s.state().warhero_worth, None);
    assert_eq!(warhero_reports(&s), vec![]);
    let card = new_instance(s.state_mut(), WARHERO, P1, Zone::Hand { player: P1 });
    s.state_mut().players[P1].hand.push(card);
    for _ in 0..4 {
        let worth = s.state().warhero_worth.unwrap_or(0);
        s.end_turn();
        let grown = s.state().warhero_worth.expect("the worth once minted");
        assert!((worth..=worth + 10).contains(&grown));
        assert_eq!(warhero_reports(&s).last(), Some(&(grown, Some(grown - worth))));
    }
}

/// R3548002: playing a Warhero Token pays its worth in Credits — twice on its Radiant face — and
/// discarding one pays the same.
#[test]
fn r3548002_playing_or_discarding_one_pays_its_worth_radiant_double() {
    for radiant in [false, true] {
        let mut s = board(
            &format!("r3548002-pays-{radiant}"),
            json!({ "hand": [{ "def": WARHERO, "radiant": radiant }] }),
            json!({}),
        );
        s.state_mut().warhero_worth = Some(7);
        s.play(WARHERO, json!({}));
        assert_eq!(market_credits_of(s.state(), P1), if radiant { 14 } else { 7 });
    }
    let mut s = board(
        "r3548002-discards",
        json!({ "hand": [{ "def": WARHERO }, { "def": WARHERO, "radiant": true }] }),
        json!({}),
    );
    s.state_mut().warhero_worth = Some(7);
    let base = s
        .hand(P1)
        .into_iter()
        .find(|card| !card.radiant)
        .expect("the base token");
    apply_now(
        s.state_mut(),
        effects::discard(json_as(
            json!({ "target": { "of": "instance", "instanceId": base.id } }),
        )),
    );
    assert_eq!(market_credits_of(s.state(), P1), 7);
    apply_now(
        s.state_mut(),
        effects::discard_random(json_as(json!({ "player": "self", "count": 1 }))),
    );
    assert_eq!(market_credits_of(s.state(), P1), 21);
}

/// R3548002: the hand card quotes the worth even with the Marketplace closed, and the Token joins
/// no Resource pool — no draw lists it, no requote moves it.
#[test]
fn r3548002_the_hand_card_quotes_its_worth_and_joins_no_resource_pool() {
    let mut s = board("r3548002-quotes", json!({ "hand": [WARHERO] }), json!({}));
    assert!(s.state().marketplace.is_none());
    s.state_mut().warhero_worth = Some(7);
    let card = s.hand(P1).into_iter().next().expect("the token");
    assert_eq!(sell_quote(s.state(), &card), Some(7));
    let preview = preview_of(s.state(), &card, P1, ConditionZone::Hand).expect("a quoted hand card");
    assert!(
        preview
            .iter()
            .any(|value| value.label == "Credits" && value.value == 7)
    );
    assert!(!resource_def_ids().contains(&WARHERO_TOKEN_DEF_ID.to_string()));
}

/// R3548003: Refresh gives back every used Activate but the Refreshes — the spent ability is listed
/// again whole, while the Refresh that paid for it stays spent.
#[test]
fn r3548003_refresh_gives_back_every_used_activate_but_refreshes() {
    let mut s = board(
        "r3548003-refresh",
        json!({ "field": [{ "def": ACTIVATOR, "lane": 1 }, { "def": REFRESHER, "lane": 2 }] }),
        json!({}),
    );
    let zap = field_id(&s, P1, ACTIVATOR);
    let refresh = field_id(&s, P1, REFRESHER);
    s.activate(zap.as_str(), json!({}));
    s.activate(zap.as_str(), json!({}));
    assert_eq!(ability_uses_this_turn(s.state(), s.card(zap.as_str()), "zap"), 2);
    assert!(!lists_activate(s.state(), &zap));
    s.activate(refresh.as_str(), json!({}));
    assert_eq!(ability_uses_this_turn(s.state(), s.card(zap.as_str()), "zap"), 0);
    assert!(lists_activate(s.state(), &zap));
    assert_eq!(
        ability_uses_this_turn(s.state(), s.card(refresh.as_str()), REFRESH_ABILITY_ID),
        1
    );
    assert!(!lists_activate(s.state(), &refresh));
}

/// R3548003: Refresh keeps cooldowns as they are and names every Refresh, fused ones included.
#[test]
fn r3548003_refresh_keeps_cooldowns_and_names_fused_refreshes() {
    let mut s = board(
        "r3548003-cooldowns",
        json!({ "field": [{ "def": COOLDOWN, "lane": 1 }] }),
        json!({}),
    );
    let slow = field_id(&s, P1, COOLDOWN);
    s.activate(slow.as_str(), json!({}));
    let cooldowns = s.card(slow.as_str()).memory.get("cooldowns").cloned();
    assert!(cooldowns.is_some());
    apply_now(s.state_mut(), effects::refresh_activates());
    assert_eq!(s.card(slow.as_str()).memory.get("cooldowns"), cooldowns.as_ref());
    assert_eq!(
        ability_uses_this_turn(s.state(), s.card(slow.as_str()), "slow"),
        0
    );
    assert!(is_refresh_ability(REFRESH_ABILITY_ID));
    assert!(is_refresh_ability("refresh#2"));
    assert!(!is_refresh_ability("zap"));
}

/// R3548004: while the awakened Reactor acts, any price is listed and any cost paid at 0 mana —
/// playing a (9) and using a mana-(3) ability spends nothing.
#[test]
fn r3548004_infinite_mana_pays_any_price_and_spends_none() {
    let mut s = board(
        "r3548004-pays",
        json!({
            "mana": 0,
            "hand": [EXPENSIVE],
            "field": [{ "def": REACTOR, "lane": 1 }, { "def": PRICED, "lane": 2 }],
        }),
        json!({}),
    );
    let expensive = hand_id(&s, P1, EXPENSIVE);
    assert!(lists_play(s.state(), &expensive));
    s.play(expensive.as_str(), json!({ "zone": 1 }));
    let pricey = field_id(&s, P1, PRICED);
    assert!(lists_activate(s.state(), &pricey));
    s.activate(pricey.as_str(), json!({}));
    assert_eq!(s.state().players[P1].mana.current, 0);
}

/// R3548004: a Dormant giver sleeps through its own flag — nothing is free — until it awakens.
#[test]
fn r3548004_a_dormant_card_gives_none_until_it_awakens() {
    let mut s = board(
        "r3548004-dormant",
        json!({
            "mana": 0,
            "hand": [EXPENSIVE],
            "field": [{ "def": SLEEPY_REACTOR, "lane": 1, "slumber": { "turns": 1 } }],
        }),
        json!({}),
    );
    let expensive = hand_id(&s, P1, EXPENSIVE);
    assert!(!lists_play(s.state(), &expensive));
    assert_eq!(s.view(P1).you.infinite_mana, None);
    s.end_turn();
    s.end_turn();
    assert!(lists_play(s.state(), &expensive));
    assert_eq!(s.view(P1).you.infinite_mana, Some(true));
}

/// R3548004: both seats see the flag while the giver acts, and it ends as the card leaves.
#[test]
fn r3548004_it_ends_as_the_card_leaves_and_both_seats_see_it() {
    let mut s = board(
        "r3548004-leaves",
        json!({ "field": [{ "def": REACTOR, "lane": 1 }] }),
        json!({}),
    );
    assert_eq!(s.view(P1).you.infinite_mana, Some(true));
    assert_eq!(s.view(P2).opponent.infinite_mana, Some(true));
    let reactor = field_id(&s, P1, REACTOR);
    apply_now(
        s.state_mut(),
        effects::bounce(json_as(
            json!({ "target": { "of": "instance", "instanceId": reactor } }),
        )),
    );
    assert_eq!(s.view(P1).you.infinite_mana, None);
    assert_eq!(s.view(P2).opponent.infinite_mana, None);
}

/// R3548004: an X is still chosen out of the mana held — infinity never widens its range.
#[test]
fn r3548004_x_is_still_chosen_out_of_held_mana() {
    let s = board(
        "r3548004-x",
        json!({ "mana": 5, "hand": [X_SPELL], "field": [{ "def": REACTOR, "lane": 1 }] }),
        json!({}),
    );
    let spell = s.hand(P1).into_iter().next().expect("the X spell");
    assert_eq!(legal_x_values(s.state(), P1, &spell), (1..=5).collect::<Vec<_>>());
}
