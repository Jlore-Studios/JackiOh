// What setup deals and what it sets aside (SPEC §2.1, §2.4, R9, R225, R635, R640, R748; issues #152, #355).
//
//  - R635: a card that casts on draw is not dealt by setup while another card is left. The opening
//    draw and the mulligan's replacement draws skip it, it stays in its owner's library, and once both
//    mulligans are resolved it is shuffled in. Setup never deals a fatigue draw.
//  - R748: a hand the other cards cannot fill takes cast-on-draw cards, uncast, and each one still in
//    a hand is cast at the start of the game, before turn 1.
//  - R640: a Quickdraw card replaces one of the opening draws, so a seat is dealt at most as many as
//    its opening hand holds, and the others are ordinary cards in the library.
//
// Decks hold a card once (§2.6 L3), so the cards below are fixtures, one def each: `fx-cod-N` casts on
// draw, `fx-qd-N` is a Quickdraw Field Spell, `fx-dual` is both.
//
// Port of `packages/engine/test/setup-aside.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::effects::choose_mode;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::setup_catalog;

const POOL: i32 = 30;
const DUAL: &str = "fx-dual";
/// A cast-on-draw Spell whose cast asks its caster something (R748's casts at the start of the game).
const ASKS: &str = "fx-cod-ask";

fn cod(n: i32) -> String {
    format!("fx-cod-{n}")
}

fn qd(n: i32) -> String {
    format!("fx-qd-{n}")
}

fn is_cod(def_id: &str) -> bool {
    def_id.starts_with("fx-cod-") || def_id == DUAL
}

fn is_qd(def_id: &str) -> bool {
    def_id.starts_with("fx-qd-") || def_id == DUAL
}

fn spell(id: &str, tags: Value, type_: &str) -> CardDef {
    json_as(json!({
        "id": id,
        "index": id,
        "name": id,
        "set": "Core",
        "type": type_,
        "tags": tags,
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": id },
        "radiant": { "keywords": [], "text": id },
    }))
}

/// The shared fixture catalog, plus POOL cast-on-draw Spells, POOL Quickdraw Field Spells and the dual card.
fn register() {
    setup_catalog();
    let mut defs: Vec<CardDef> = Vec::new();
    let mut scripts: Vec<(String, CardScripts)> = Vec::new();
    let mut add = |def: CardDef, flags: Value| {
        let script = Script {
            static_flags: Some(json_as(flags)),
            cry: Some(hook(|_ctx| vec![])),
            ..Script::default()
        };
        scripts.push((
            def.id.clone(),
            CardScripts {
                base: script.clone(),
                radiant: script,
            },
        ));
        defs.push(def);
    };
    for n in 1..=POOL {
        add(spell(&cod(n), json!([]), "Spell"), json!({ "castOnDraw": true }));
        add(
            spell(&qd(n), json!(["Quickdraw"]), "Field Spell"),
            json!({ "quickdraw": true }),
        );
    }
    add(
        spell(DUAL, json!(["Quickdraw"]), "Spell"),
        json!({ "castOnDraw": true, "quickdraw": true }),
    );
    defs.push(spell(ASKS, json!([]), "Spell"));
    let mut resume = IndexMap::new();
    resume.insert("ok", hook(|_ctx| vec![]));
    let asks = Script {
        static_flags: Some(json_as(json!({ "castOnDraw": true }))),
        cry: Some(hook(|_ctx| {
            vec![choose_mode(json_as(
                json!({ "options": ["ok", "fine"], "step": "ok", "prompt": "R748" }),
            ))]
        })),
        resume,
        ..Script::default()
    };
    scripts.push((
        ASKS.to_string(),
        CardScripts {
            base: asks.clone(),
            radiant: asks,
        },
    ));
    let mut catalog = registered_catalog().clone();
    for def in defs {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    for (id, script) in scripts {
        registry.insert(id, script);
    }
    register_scripts(registry);
}

/// `size` cards: `quick` Quickdraw cards, `cast` cast-on-draw ones, the rest vanilla Units.
fn deck_of(quick: i32, cast: i32, size: Option<i32>) -> Vec<String> {
    let rest = size.unwrap_or(DECK_SIZE) - quick - cast;
    let mut deck: Vec<String> = (0..quick).map(|at| qd(at + 1)).collect();
    deck.extend((0..cast).map(|at| cod(at + 1)));
    deck.extend(vanilla_deck(rest, 1));
    deck
}

fn other() -> Vec<String> {
    vanilla_deck(DECK_SIZE, 21)
}

/// TS `start(...)` returned `{ game, begun }`; every test reads `begun` only, so this returns it.
fn start(
    seed: &str,
    decks: (Vec<String>, Vec<String>),
    handicaps: Option<PerPlayerOpt<Handicap>>,
) -> ReduceResult {
    register();
    let game = create_game(&CreateGameOptions {
        seed: seed.into(),
        decks,
        handicaps,
        ..Default::default()
    });
    begin_game(&game)
}

/// TS's module `let nonce`.
static NONCE: AtomicU32 = AtomicU32::new(0);

struct Acted {
    state: GameState,
    events: Vec<GameEvent>,
    action: Action,
}

fn act(state: &GameState, body: Value) -> Acted {
    let n = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let input: ActionInput = json_as(body);
    let kind = input.body.action_type();
    let action = input.with_nonce(format!("aside-{n}"));
    let result = reduce(state, &action);
    if let Some(error) = &result.error {
        panic!("{kind} refused: {error}");
    }
    Acted {
        state: result.state,
        events: result.events,
        action,
    }
}

struct Answered {
    state: GameState,
    events: Vec<GameEvent>,
    log: Vec<Action>,
}

/// Both seats answer: `keep.p1`/`keep.p2` are the ids kept, every card of the hand by default.
fn answer_both(state: &GameState, keep: PerPlayerOpt<Vec<String>>) -> Answered {
    let mut events = Vec::new();
    let mut log = Vec::new();
    let mut next = state.clone();
    for player in [PlayerId::P1, PlayerId::P2] {
        let kept = keep.get(player).cloned().unwrap_or_else(|| {
            next.players[player]
                .hand
                .iter()
                .map(|card| card.id.clone())
                .collect()
        });
        let answered = act(
            &next,
            json!({ "type": "mulligan", "keep": kept, "playerId": player }),
        );
        next = answered.state;
        events.extend(answered.events);
        log.push(answered.action);
    }
    Answered {
        state: next,
        events,
        log,
    }
}

fn keep_all() -> PerPlayerOpt<Vec<String>> {
    PerPlayerOpt::default()
}

fn keep_p1(ids: Vec<String>) -> PerPlayerOpt<Vec<String>> {
    PerPlayerOpt {
        p1: Some(ids),
        p2: None,
    }
}

fn keep_p2(ids: Vec<String>) -> PerPlayerOpt<Vec<String>> {
    PerPlayerOpt {
        p1: None,
        p2: Some(ids),
    }
}

/// What happened in setup: the events up to, and not including, turn 1's start.
fn before_turn_one(events: &[GameEvent]) -> Vec<GameEvent> {
    match events
        .iter()
        .position(|event| event.event_type() == GameEventType::TurnStarted)
    {
        None => events.to_vec(),
        Some(at) => events[..at].to_vec(),
    }
}

fn count(events: &[GameEvent], kind: GameEventType) -> usize {
    events.iter().filter(|event| event.event_type() == kind).count()
}

fn defs_of(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.def_id.clone()).collect()
}

fn ids_of(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// `{ type, player? }` of each event `p1` is shown, as `type` or `type:player`.
fn types_of(state: &GameState) -> Vec<String> {
    view_for(state, PlayerId::P1)
        .events
        .iter()
        .map(|event| {
            let json = serde_json::to_value(event).unwrap();
            let kind = json["type"].as_str().unwrap_or_default().to_string();
            match json.get("player").and_then(Value::as_str) {
                Some(player) => format!("{kind}:{player}"),
                None => kind,
            }
        })
        .collect()
}

fn opponent_hand(state: &GameState) -> Value {
    serde_json::to_value(&view_for(state, PlayerId::P1).opponent.hand).unwrap()
}

fn opponent_library_count(state: &GameState) -> i32 {
    view_for(state, PlayerId::P1).opponent.library_count
}

/// The `drawn` events of `player`, as `(instanceId, defId)`.
fn drawn_by(events: &[GameEvent], player: PlayerId) -> Vec<(String, String)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Drawn {
                player: who,
                instance_id,
                def_id,
                ..
            } if *who == player => Some((instance_id.clone(), def_id.clone())),
            _ => None,
        })
        .collect()
}

mod r635_cast_on_draw_cards_sit_out_the_deal_and_are_shuffled_in_after_the_mulligan {
    use super::*;

    #[test]
    fn r635_the_opening_draw_and_the_mulligans_replacements_cast_nothing_no_cast_on_draw_card_is_dealt() {
        for n in 0..40 {
            let begun = start(&format!("r635-deal-{n}"), (deck_of(0, 8, None), other()), None);
            let state = &begun.state;
            let side = &state.players.p1;

            // Three of the twelve other cards are dealt, and nothing was cast to get there.
            assert_eq!(side.hand.len() as i32, OPENING_DRAW[0]);
            assert!(!defs_of(&side.hand).iter().any(|d| is_cod(d)));
            assert!(side.graveyard.is_empty());
            assert_eq!(count(&begun.events, GameEventType::CardPlayed), 0);
            // They are still in the library, behind every card the draws can reach: the library count is
            // the deck less the hand, as for any deck.
            assert_eq!(side.library.len() as i32, DECK_SIZE - side.hand.len() as i32);
            let bottom = &side.library[side.library.len() - 8..];
            assert!(defs_of(bottom).iter().all(|d| is_cod(d)));

            // The mulligan returns the whole hand: three replacements, none of them a cast-on-draw card, and
            // nothing cast on the way (turn 1's own draw comes after setup's last event).
            let returned = ids_of(&side.hand);
            let answered = answer_both(state, keep_p1(vec![]));
            let dealt = before_turn_one(&answered.events);
            let replacements = drawn_by(&dealt, PlayerId::P1);
            assert_eq!(replacements.len(), returned.len());
            assert!(replacements.iter().all(|(_, def_id)| !is_cod(def_id)));
            assert_eq!(count(&dealt, GameEventType::CardPlayed), 0);
            assert_eq!(count(&dealt, GameEventType::Fatigue), 0);
            assert_eq!(count(&dealt, GameEventType::ShuffledIn), returned.len());
        }
    }

    #[test]
    fn r635_shuffles_them_into_the_library_once_both_mulligans_are_resolved_at_random_places() {
        // p2 draws first on turn 2, so its library is as setup left it. Left at the bottom, the six sit at
        // indices 10 to 15 whatever the seed; shuffled in, their mean place in 16 cards is 7.5.
        let mut places = 0usize;
        let mut seen = 0usize;
        let mut at_top = 0usize;
        for n in 0..150 {
            let begun = start(&format!("r635-shuffle-{n}"), (other(), deck_of(0, 6, None)), None);
            let settled = answer_both(&begun.state, keep_all()).state;
            let library = &settled.players.p2.library;
            assert_eq!(library.len() as i32, DECK_SIZE - OPENING_DRAW[1]);
            for (at, card) in library.iter().enumerate() {
                if !is_cod(&card.def_id) {
                    continue;
                }
                places += at;
                seen += 1;
                if at == 0 {
                    at_top += 1;
                }
            }
        }
        assert_eq!(seen, 150 * 6);
        let mean = places as f64 / seen as f64;
        assert!(mean > 7.0, "{mean}");
        assert!(mean < 8.0, "{mean}");
        assert!(at_top > 0);
    }

    #[test]
    fn r635_says_nothing_to_the_other_seat_its_view_is_the_same_whether_the_deck_holds_cast_on_draw_cards() {
        for n in 0..20 {
            let seed = format!("r635-view-{n}");
            let with_cast = start(&seed, (other(), deck_of(0, 7, None)), None);
            let without = start(&seed, (other(), deck_of(0, 0, None)), None);

            // The deal: the same events, the same counts. The seat that holds the cards is dealt four
            // others either way, and nothing is announced about the ones set aside.
            assert_eq!(types_of(&with_cast.state), types_of(&without.state), "{seed}");
            assert_eq!(opponent_hand(&with_cast.state), json!({ "count": 4 }), "{seed}");
            assert_eq!(
                opponent_library_count(&with_cast.state),
                opponent_library_count(&without.state),
                "{seed}"
            );

            // After the mulligans, where the shuffle-in happens: no `shuffledIn` per card, which would count them.
            let settled_with = answer_both(&with_cast.state, keep_all());
            let settled_without = answer_both(&without.state, keep_all());
            assert_eq!(
                count(&settled_with.events, GameEventType::ShuffledIn),
                0,
                "{seed}"
            );
            assert_eq!(
                types_of(&settled_with.state),
                types_of(&settled_without.state),
                "{seed}"
            );
            assert_eq!(
                opponent_library_count(&settled_with.state),
                opponent_library_count(&settled_without.state),
                "{seed}"
            );
        }
    }

    #[test]
    fn r635_a_card_that_is_both_quickdraw_and_cast_on_draw_is_dealt_as_a_quickdraw_card_never_cast() {
        for n in 0..20 {
            let mut deck = vec![DUAL.to_string()];
            deck.extend(vanilla_deck(DECK_SIZE - 1, 1));
            let begun = start(&format!("r635-dual-{n}"), (other(), deck), None);
            let hand = &begun.state.players.p2.hand;
            assert!(defs_of(hand).contains(&DUAL.to_string()));
            assert!(
                !begun
                    .events
                    .iter()
                    .any(|event| event.event_type() == GameEventType::CardPlayed)
            );
            assert!(begun.state.players.p2.graveyard.is_empty());

            // Returned by the mulligan, it is a cast-on-draw card in the library like the rest of them: it
            // goes back among them and is shuffled in, not cast.
            let dual = hand
                .iter()
                .find(|card| card.def_id == DUAL)
                .expect("the dual card")
                .clone();
            let kept: Vec<String> = hand
                .iter()
                .filter(|card| card.id != dual.id)
                .map(|card| card.id.clone())
                .collect();
            let answered = answer_both(&begun.state, keep_p2(kept));
            assert_eq!(
                count(&before_turn_one(&answered.events), GameEventType::CardPlayed),
                0
            );
            assert!(answered.state.players.p2.graveyard.is_empty());
            assert!(
                answered
                    .state
                    .players
                    .p2
                    .library
                    .iter()
                    .any(|card| card.id == dual.id)
            );
        }
    }

    #[test]
    fn r635_a_game_with_cards_set_aside_folds_from_its_seed_and_log_whatever_the_mulligans_returned() {
        for n in 0..15 {
            let seed = format!("r635-fold-{n}");
            let decks = (deck_of(1, 6, None), deck_of(2, 9, None));
            let begun = start(&seed, decks.clone(), None);
            let p1 = ids_of(&begun.state.players.p1.hand);
            let p2 = ids_of(&begun.state.players.p2.hand);
            let played = answer_both(
                &begun.state,
                PerPlayerOpt {
                    p1: Some(p1[1..].to_vec()),
                    p2: Some(p2[2..].to_vec()),
                },
            );

            let replayed = fold(&FoldArgs {
                seed: seed.clone(),
                decks,
                log: played.log,
                ..Default::default()
            });
            assert_eq!(replayed.errors.len(), 0, "{seed}");
            assert_eq!(hash_state(&replayed.state), hash_state(&played.state), "{seed}");
        }
    }
}

mod r748_a_hand_the_other_cards_cannot_fill_takes_cast_on_draw_cards_cast_at_the_start_of_the_game {
    use super::*;

    /// The `cardPlayed` events, by instance id, in order.
    fn played_in(events: &[GameEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardPlayed { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r748_an_all_cast_on_draw_deck_deals_a_full_hand_of_them_uncast_and_casts_them_at_the_start_of_the_game()
     {
        let begun = start("r748-all", (deck_of(0, DECK_SIZE, None), other()), None);
        let state = &begun.state;
        let hand = &state.players.p1.hand;

        // Three of them, each reported as a draw (R225), and none of them cast in the deal.
        assert_eq!(hand.len() as i32, OPENING_DRAW[0]);
        assert!(defs_of(hand).iter().all(|d| is_cod(d)));
        assert_eq!(
            drawn_by(&begun.events, PlayerId::P1).len() as i32,
            OPENING_DRAW[0]
        );
        assert_eq!(
            state.players.p1.library.len() as i32,
            DECK_SIZE - hand.len() as i32
        );
        assert_eq!(state.players.p1.fatigue_count, 0);
        assert_eq!(count(&begun.events, GameEventType::Fatigue), 0);
        assert_eq!(count(&begun.events, GameEventType::CardPlayed), 0);
        // The mulligan offers them like any other card.
        let offered: Option<Vec<String>> = mulligan_prompt_for(state, PlayerId::P1)
            .map(|prompt| prompt.options.iter().map(|option| option.key.clone()).collect());
        assert_eq!(offered, Some(ids_of(hand)));

        let answered = answer_both(state, keep_all());
        // Once both mulligans are in, the three are cast before turn 1, in hand order, no draw repeated.
        assert_eq!(played_in(&before_turn_one(&answered.events)), ids_of(hand));
        assert_eq!(
            count(&before_turn_one(&answered.events), GameEventType::Fatigue),
            0
        );
        assert!(drawn_by(&before_turn_one(&answered.events), PlayerId::P1).is_empty());
        // Turn 1's draw meets the other seventeen: it casts them all, and the draw after the last one
        // finds the library empty.
        let from = answered
            .events
            .iter()
            .position(|event| event.event_type() == GameEventType::TurnStarted)
            .expect("turn 1 starts");
        let turn_one = &answered.events[from..];
        assert!(answered.state.turn >= 1);
        assert_eq!(
            count(turn_one, GameEventType::CardPlayed) as i32,
            DECK_SIZE - hand.len() as i32
        );
        assert_eq!(answered.state.players.p1.graveyard.len() as i32, DECK_SIZE);
        assert!(answered.state.players.p1.library.is_empty());
        assert_eq!(answered.state.players.p1.fatigue_count, 1);
        assert!(answered.state.result.is_none());
    }

    #[test]
    fn r748_r58_turn_1_of_an_all_cast_on_draw_library_is_still_bounded_by_r58s_cap_as_a_chain_mid_game_is() {
        register();
        let mut game = create_game(&CreateGameOptions {
            seed: "r748-cap".into(),
            decks: (deck_of(0, DECK_SIZE, None), other()),
            ..Default::default()
        });
        // Five more, as a Unstable Clone Machine's or a CN-Virus's copies would add: 25 cards, all of them
        // cast on draw.
        for def_id in [cod(1), cod(2), cod(3), cod(4), cod(5)] {
            let card = new_instance(
                &mut game,
                &def_id,
                PlayerId::P1,
                Zone::Library { player: PlayerId::P1 },
            );
            game.players.p1.library.push(card);
        }
        let begun = begin_game(&game);
        assert_eq!(begun.state.players.p1.hand.len() as i32, OPENING_DRAW[0]);
        assert_eq!(
            begun.state.players.p1.library.len() as i32,
            DECK_SIZE + 5 - OPENING_DRAW[0]
        );

        let settled = answer_both(&begun.state, keep_all()).state;
        // The hand's three at the start of the game, then turn 1's twenty casts; the next card goes to the
        // hand uncast, which ends the chain.
        assert_eq!(
            settled.players.p1.graveyard.len() as i32,
            OPENING_DRAW[0] + CAST_ON_DRAW_CHAIN_CAP
        );
        assert_eq!(settled.players.p1.hand.len(), 1);
        assert_eq!(settled.players.p1.library.len(), 1);
        assert_eq!(settled.players.p1.fatigue_count, 0);
    }

    #[test]
    fn r748_one_other_card_and_two_cast_on_draw_cards_fill_the_hand_and_each_seats_are_cast_player_1s_first()
    {
        // p1: one other card and a hand of three. p2: two other cards and a hand of four.
        let begun = start(
            "r748-one",
            (deck_of(0, DECK_SIZE - 1, None), deck_of(0, DECK_SIZE - 2, None)),
            None,
        );
        let state = &begun.state;
        let p1 = &state.players.p1.hand;
        let p2 = &state.players.p2.hand;
        let others = |cards: &[CardInstance]| -> Vec<String> {
            defs_of(cards).into_iter().filter(|d| !is_cod(d)).collect()
        };
        let casting =
            |cards: &[CardInstance]| -> usize { defs_of(cards).iter().filter(|d| is_cod(d)).count() };
        assert_eq!(others(&p1[..]), vec!["fx-1".to_string()]);
        assert_eq!(casting(&p1[..]), 2);
        assert_eq!(others(&p2[..]).len(), 2);
        assert_eq!(casting(&p2[..]), 2);
        assert_eq!(state.players.p1.library.len() as i32, DECK_SIZE - 3);
        assert_eq!(count(&begun.events, GameEventType::Fatigue), 0);
        assert_eq!(count(&begun.events, GameEventType::CardPlayed), 0);

        let answered = answer_both(state, keep_all());
        let cast: Vec<String> = p1
            .iter()
            .chain(p2.iter())
            .filter(|card| is_cod(&card.def_id))
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(played_in(&before_turn_one(&answered.events)), cast);
        // The other cards stay in the hands.
        let p2_others: Vec<String> = p2
            .iter()
            .filter(|card| !is_cod(&card.def_id))
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(ids_of(&answered.state.players.p2.hand), p2_others);
        assert!(
            answered
                .state
                .players
                .p1
                .hand
                .iter()
                .any(|card| card.def_id == "fx-1")
        );
    }

    #[test]
    fn r748_a_mulligan_the_other_cards_cannot_replace_is_dealt_cast_on_draw_cards_for_the_rest() {
        // Five other cards: three in the hand and two left in the library for the three replacements.
        let begun = start("r748-short", (deck_of(0, DECK_SIZE - 5, None), other()), None);
        let state = &begun.state;
        assert_eq!(state.players.p1.hand.len(), 3);
        assert!(!defs_of(&state.players.p1.hand).iter().any(|d| is_cod(d)));
        assert_eq!(
            state
                .players
                .p1
                .library
                .iter()
                .filter(|card| !is_cod(&card.def_id))
                .count(),
            2
        );

        let answered = answer_both(state, keep_p1(vec![]));
        let dealt = before_turn_one(&answered.events);
        let replacements = drawn_by(&dealt, PlayerId::P1);
        // Three back: the two other cards, and one cast-on-draw card dealt uncast, cast at the start.
        assert_eq!(replacements.len(), 3);
        let filled: Vec<String> = replacements
            .iter()
            .filter(|(_, def_id)| is_cod(def_id))
            .map(|(instance_id, _)| instance_id.clone())
            .collect();
        assert_eq!(filled.len(), 1);
        assert_eq!(played_in(&dealt), filled);
        assert_eq!(count(&dealt, GameEventType::Fatigue), 0);
        assert_eq!(answered.state.players.p1.fatigue_count, 0);
        assert_eq!(answered.state.players.p1.hero.health, 30);
    }

    #[test]
    fn r748_a_cast_on_draw_card_dealt_uncast_that_the_mulligan_returns_goes_back_uncast() {
        // p2 draws nothing on turn 1, so its library is as setup left it.
        let begun = start("r748-returned", (other(), deck_of(0, DECK_SIZE - 2, None)), None);
        let hand = &begun.state.players.p2.hand;
        let waiting: Vec<&CardInstance> = hand.iter().filter(|card| is_cod(&card.def_id)).collect();
        assert_eq!(waiting.len(), 2);
        let (returned, kept) = (waiting[0].clone(), waiting[1].clone());

        let answered = answer_both(
            &begun.state,
            keep_p2(
                hand.iter()
                    .filter(|card| card.id != returned.id)
                    .map(|card| card.id.clone())
                    .collect(),
            ),
        );
        let dealt = before_turn_one(&answered.events);
        // The replacement is another of them, uncast; the returned one is shuffled back and not cast.
        let replacement = drawn_by(&dealt, PlayerId::P2);
        assert_eq!(replacement.len(), 1);
        assert!(is_cod(&replacement[0].1));
        assert!(dealt.iter().any(|event| matches!(
            event,
            GameEvent::ShuffledIn { instance_id, .. } if *instance_id == returned.id
        )));
        let played = played_in(&dealt);
        assert!(!played.contains(&returned.id));
        assert_eq!(played, vec![kept.id.clone(), replacement[0].0.clone()]);
        let back = answered
            .state
            .players
            .p2
            .library
            .iter()
            .find(|card| card.id == returned.id);
        assert_eq!(back.map(|card| card.memory.is_empty()), Some(true));
    }

    #[test]
    fn r748_a_cast_at_the_start_of_the_game_that_asks_holds_turn_1_until_it_is_answered_and_the_game_folds_from_its_log()
     {
        let mut deck = vec![ASKS.to_string()];
        deck.extend((0..DECK_SIZE - 1).map(|at| cod(at + 1)));
        let decks = (deck, other());
        let mut seed = String::new();
        let mut begun: Option<ReduceResult> = None;
        for n in 0..50 {
            if begun.is_some() {
                break;
            }
            let tried = start(&format!("r748-asks-{n}"), decks.clone(), None);
            if tried.state.players.p1.hand.iter().any(|card| card.def_id == ASKS) {
                seed = format!("r748-asks-{n}");
                begun = Some(tried);
            }
        }
        let begun = begun.expect("no seed dealt the asking card");

        let answered = answer_both(&begun.state, keep_all());
        let pending = answered.state.pending.clone().expect("a prompt");
        assert_eq!(pending.kind, PromptKind::Mode);
        assert_eq!(pending.player_id, PlayerId::P1);
        assert_eq!(answered.state.turn, 0);
        assert_eq!(count(&answered.events, GameEventType::TurnStarted), 0);
        // A paused setup is plain data (R113).
        let round_tripped: GameState =
            serde_json::from_value(serde_json::to_value(&answered.state).unwrap()).unwrap();
        assert_eq!(round_tripped, answered.state);

        let done = act(
            &answered.state,
            json!({
                "type": "answer",
                "choiceId": pending.id,
                "selection": [{ "pick": "mode", "option": "ok" }],
                "playerId": "p1",
            }),
        );
        assert!(done.state.turn >= 1);
        assert!(done.state.players.p1.graveyard.len() as i32 >= OPENING_DRAW[0]);

        let mut log = answered.log.clone();
        log.push(done.action.clone());
        let replayed = fold(&FoldArgs {
            seed,
            decks,
            log,
            ..Default::default()
        });
        assert_eq!(replayed.errors.len(), 0);
        assert_eq!(hash_state(&replayed.state), hash_state(&done.state));
    }
}

mod r640_a_quickdraw_card_replaces_one_of_the_opening_draws {
    use super::*;

    fn quick_in(cards: &[CardInstance]) -> usize {
        cards.iter().filter(|card| is_qd(&card.def_id)).count()
    }

    #[test]
    fn r640_a_seat_holding_all_five_quickdraw_cards_is_dealt_only_as_many_as_it_has_draws_p1s_three_and_p2s_four()
     {
        let mut hands: IndexSet<String> = IndexSet::new();
        for n in 0..60 {
            let begun = start(
                &format!("r636-five-{n}"),
                (deck_of(5, 0, None), deck_of(5, 0, None)),
                None,
            );
            let p1 = &begun.state.players.p1;
            let p2 = &begun.state.players.p2;

            assert_eq!(p1.hand.len() as i32, OPENING_DRAW[0]);
            assert_eq!(p2.hand.len() as i32, OPENING_DRAW[1]);
            // All of the hand is Quickdraw cards, and the rest wait in the library as ordinary ones.
            assert_eq!(quick_in(&p1.hand), 3);
            assert_eq!(quick_in(&p2.hand), 4);
            assert_eq!(quick_in(&p1.library), 2);
            assert_eq!(quick_in(&p2.library), 1);
            assert_eq!(p1.library.len() as i32, DECK_SIZE - 3);
            assert_eq!(p2.library.len() as i32, DECK_SIZE - 4);
            // R225: each is reported as a draw, `drawn` then `addedToHand`, and only the dealt ones are.
            assert_eq!(drawn_by(&begun.events, PlayerId::P1).len(), 3);
            assert_eq!(drawn_by(&begun.events, PlayerId::P2).len(), 4);
            assert_eq!(count(&begun.events, GameEventType::Burned), 0);
            let mut defs = defs_of(&p1.hand);
            defs.sort();
            hands.insert(defs.join(","));
        }
        // Which of the five fill the hand is the shuffle's choice, not always the same three.
        assert!(hands.len() > 3);
    }

    #[test]
    fn r640_a_handicapped_seat_is_dealt_up_to_its_larger_hand_four_for_p1_and_five_for_p2_under_medium() {
        let medium = AI_DIFFICULTY.medium;
        let decks = (
            deck_of(5, 0, Some(medium.deck_size)),
            deck_of(5, 0, Some(medium.deck_size)),
        );
        let begun = start(
            "r636-medium",
            decks,
            Some(PerPlayerOpt {
                p1: Some(medium),
                p2: Some(medium),
            }),
        );
        let p1 = &begun.state.players.p1;
        let p2 = &begun.state.players.p2;

        assert_eq!(p1.hand.len() as i32, OPENING_DRAW[0] + medium.extra_opening_cards);
        assert_eq!(p2.hand.len() as i32, OPENING_DRAW[1] + medium.extra_opening_cards);
        assert_eq!(quick_in(&p1.hand), 4);
        assert_eq!(quick_in(&p1.library), 1);
        // Five draws and five Quickdraw cards: no draw is left over and none is missing.
        assert_eq!(quick_in(&p2.hand), 5);
        assert_eq!(quick_in(&p2.library), 0);
    }

    #[test]
    fn r640_the_opening_hand_is_the_tables_size_whether_or_not_the_deck_holds_quickdraw_cards() {
        for n in 0..20 {
            let seed = format!("r636-view-{n}");
            let five = start(&seed, (other(), deck_of(5, 0, None)), None);
            let none = start(&seed, (other(), deck_of(0, 0, None)), None);
            // Were every Quickdraw card added to the hand, a hand of five would say p2 holds at least two.
            assert_eq!(opponent_hand(&five.state), json!({ "count": 4 }), "{seed}");
            assert_eq!(
                opponent_library_count(&five.state),
                opponent_library_count(&none.state),
                "{seed}"
            );
            assert_eq!(types_of(&five.state), types_of(&none.state), "{seed}");
        }
    }

    #[test]
    fn r640_a_deck_of_nothing_but_quickdraw_cards_deals_the_hand_and_burns_nothing() {
        let begun = start("r636-all", (deck_of(DECK_SIZE, 0, None), other()), None);
        let side = &begun.state.players.p1;
        assert_eq!(side.hand.len() as i32, OPENING_DRAW[0]);
        assert!(side.hand.len() as i32 <= HAND_CAP);
        assert_eq!(side.library.len() as i32, DECK_SIZE - side.hand.len() as i32);
        assert_eq!(count(&begun.events, GameEventType::Burned), 0);
    }

    #[test]
    fn r640_the_quickdraw_cards_that_were_not_dealt_are_ordinary_cards_a_mulligans_replacements_can_draw_them()
     {
        let mut drawn_surplus = 0;
        for n in 0..60 {
            let begun = start(&format!("r636-surplus-{n}"), (deck_of(5, 0, None), other()), None);
            let dealt: IndexSet<String> = begun
                .state
                .players
                .p1
                .hand
                .iter()
                .map(|card| card.id.clone())
                .collect();
            let answered = answer_both(&begun.state, keep_p1(vec![]));
            let replacements = drawn_by(&before_turn_one(&answered.events), PlayerId::P1);
            assert_eq!(replacements.len(), 3);
            for (instance_id, def_id) in &replacements {
                if is_qd(def_id) && !dealt.contains(instance_id) {
                    drawn_surplus += 1;
                }
            }
        }
        assert!(drawn_surplus > 0);
    }

    #[test]
    fn r640_r748_two_quickdraw_cards_and_one_cast_on_draw_card_fill_the_hand_uncast() {
        // Two Quickdraw cards, eighteen that cast on draw, and three draws: the two, and one of the
        // eighteen for the third, cast at the start of the game.
        let begun = start("r636-mixed", (deck_of(2, 18, None), other()), None);
        let side = &begun.state.players.p1;
        assert_eq!(defs_of(&side.hand).iter().filter(|d| is_qd(d)).count(), 2);
        assert_eq!(defs_of(&side.hand).iter().filter(|d| is_cod(d)).count(), 1);
        assert_eq!(side.hand.len() as i32, OPENING_DRAW[0]);
        assert_eq!(side.library.len(), 17);
        assert_eq!(side.fatigue_count, 0);
        assert_eq!(count(&begun.events, GameEventType::Fatigue), 0);
        assert_eq!(count(&begun.events, GameEventType::CardPlayed), 0);
    }
}
