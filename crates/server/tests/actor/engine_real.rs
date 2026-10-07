//! `src/match/engine.real.ts` — the real `EnginePort`, and until now the only file in `src/` with no
//! test at all.
//!
//! WHY IT NEEDS ONE. Every other test in this suite installs a scripted port through
//! `setEnginePort`, which is what lets the actor, the clock and the recovery tests run without the
//! engine in the process — and is exactly why nothing noticed when this file was missing its
//! `registerAll()` call. `createGame` looks its card definitions up in the engine's *registered*
//! catalog (`packages/engine/src/state.ts`: `validateDeck` throws `"core-001" is not in the catalog
//! (§9.4 L6)` when it is empty), so without that call every real match died on the first card of the
//! first deck while all 200-odd server tests stayed green. This file is the one that would have
//! caught it: it builds the port for real, with the real §8 catalog, and plays a card.
//!
//! It asks only what the adapter is responsible for — that the engine is reachable, registered and
//! driveable through the port's own surface. The rules those calls run are `packages/engine`'s and
//! `packages/cards`' business, and the fuzz suite plays 1,000 whole games of them (BUILD §4).
//!
//! Its two legal decks come from `decksTheEngineAccepts` (`test/fakes/engine.ts`), the probe this
//! file used to hold privately — the reasoning for probing the size rather than importing
//! `DECK_SIZE` moved with it, and so did the "refused every deck size" failure that catches an
//! unregistered catalog. It is shared now because the real-engine blocks of `actor.test.ts` and
//! `recovery.test.ts` need the same two decks.
//!
//! Port of `apps/server/test/match/engine.real.test.ts`. The Rust server has no `EnginePort`, no
//! dynamic import and no `EngineUnavailableError` (SURFACE §11.3): the port's methods are plain
//! functions in `actor::engine` that call `jackioh_engine` directly, and this file drives them. The
//! probe is a private copy here (it is three lines of `support::engine`'s TS original and this file
//! is its only reason to exist), catching `create_game`'s panic where TS caught its throw.

use std::panic::{AssertUnwindSafe, catch_unwind};

use jackioh_engine::{Action, ActionBody, CardDef, CreateGameArgs, FoldArgs, GameState, Phase, PlayerId};
use jackioh_server::actor::engine::{
    begin_game, create_game, deal_random_deck, hash_state, legal_actions, reduce, snapshot, summarize_game,
    view_for,
};
use serde_json::{Value, json};

/// Actions that would end the game or that only the server may send (R79, R84). The walk below
/// avoids them for the same reason SPEC §10.7's policy does: it is looking for the first card play,
/// not for a way out of the match.
const NEVER_CHOOSE: &[&str] = &[
    "concede",
    "offerDraw",
    "answerDraw",
    "timeout",
    "disconnectExpired",
    "ceilingReached",
];

/// TS `catalog.isToken` (`src/api/catalog.ts`): a token by flag or by tag.
fn is_token(def: &CardDef) -> bool {
    def.token || def.tags.iter().any(|tag| tag.as_str() == "Token")
}

/// TS `loadCatalog()`'s `cardIds`, tokens filtered out: every deckable id, in catalog order. The
/// catalog is registered first, as `enginePort()` did.
fn deckable_pool() -> Vec<String> {
    jackioh_cards::register_all();
    jackioh_cards::CATALOG
        .iter()
        .filter(|(_, def)| !is_token(def))
        .map(|(id, _)| id.clone())
        .collect()
}

fn args(seed: &str, decks: &(Vec<String>, Vec<String>)) -> CreateGameArgs {
    CreateGameArgs {
        seed: seed.to_string(),
        decks: decks.clone(),
        ..CreateGameArgs::default()
    }
}

/// What a caught panic said: `create_game` panics with TS's refusal text (SURFACE §4.4.9).
fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        return (*text).to_string();
    }
    if let Some(text) = payload.downcast_ref::<String>() {
        return text.clone();
    }
    "a panic without a message".to_string()
}

/// Two legal, disjoint decks for the **real** engine port — the counterpart of `fakeDeck` for the
/// files that drive `actor::engine` (`engine_real.rs`, and the real-engine blocks of
/// `match_actor.rs` and `recovery.rs`).
///
/// The deck size is not written here and not imported either: BUILD §2 keeps `DECK_SIZE` in
/// `packages/engine/src/config.ts`, nothing restates it, and the port is meant to be the one place
/// the server reaches the engine's rules. So the size is whatever the engine accepts: the slices grow
/// until `createGame` stops objecting.
///
/// That loop is also an assertion. With the card catalog unregistered *every* size is refused, so
/// that failure surfaces here as "the real engine refused every deck size", with the engine's own
/// sentences attached, rather than as a shapeless panic inside whatever called this.
fn decks_the_engine_accepts(pool: &[String], seed: &str) -> (GameState, (Vec<String>, Vec<String>)) {
    let mut refusals: Vec<String> = Vec::new();
    let mut size = 1;
    while size * 2 <= pool.len() {
        let decks = (pool[..size].to_vec(), pool[size..size * 2].to_vec());
        let game = args(seed, &decks);
        match catch_unwind(AssertUnwindSafe(|| create_game(&game))) {
            Ok(state) => return (state, decks),
            Err(payload) => {
                let refusal = panic_text(payload.as_ref());
                if !refusals.contains(&refusal) {
                    refusals.push(refusal);
                }
            }
        }
        size += 1;
    }
    panic!(
        "the real engine refused every deck size built from the catalog:\n  {}",
        refusals.join("\n  ")
    );
}

/// The instance ids of a seat's hand, read through its own view (§10.8).
fn hand_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    let view = serde_json::to_value(view_for(state, player)).expect("PlayerView serialises");
    view["you"]["hand"]
        .as_array()
        .map(|cards| {
            cards
                .iter()
                .filter_map(|card| card["instanceId"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

mod the_real_engine_port_src_match_engine_real_ts {
    use super::*;

    #[test]
    fn builds_without_the_engine_reporting_a_missing_export() {
        // TS: `enginePort()` threw `EngineUnavailableError` when `@jackioh/engine` was missing any of
        // `REQUIRED_ENGINE_EXPORTS`; getting a port back at all was that check passing. Rust links the
        // engine (SURFACE §11.3), so a missing export is a compile error and naming the port's
        // functions here is that check. What `enginePort()` did besides is left to check at run time:
        // the catalog is registered, so the port has cards to deal from.
        let _create = create_game;
        let _reduce = reduce;
        let _snapshot = snapshot;
        jackioh_cards::register_all();
        assert!(!jackioh_engine::registered_catalog().is_empty());
    }

    #[test]
    fn registers_the_card_catalog_so_create_game_accepts_a_deck_of_real_card_ids() {
        let pool = deckable_pool();

        let (state, decks) = decks_the_engine_accepts(&pool, "engine-real-createGame");

        // The decks really are §8 cards, so an empty registered catalog could not have produced this.
        assert_eq!(decks.0[0], pool[0]);
        assert_eq!(snapshot(&state).phase, Phase::Setup);
        assert_eq!(snapshot(&state).result, None);
        // Nothing is owed before the deal; `beginGame` opens both mulligans at once (R265).
        assert_eq!(snapshot(&state).mulligan_owed, Vec::<PlayerId>::new());
        let begun = begin_game(&state).state;
        let opened = snapshot(&begun);
        assert_eq!(opened.phase, Phase::Mulligan);
        assert_eq!(opened.turn, 0);
        assert_eq!(opened.pending_for, None);
        assert_eq!(opened.mulligan_owed, vec![PlayerId::P1, PlayerId::P2]);
    }

    #[test]
    fn reduces_a_first_real_card_play_without_throwing() {
        let pool = deckable_pool();

        let (created, _decks) = decks_the_engine_accepts(&pool, "engine-real-firstplay");
        let mut state = begin_game(&created).state;

        // Walk the real game through the port's own surface — `snapshot` for whose turn it is,
        // `legalActions` for what may be done, `reduce` to do it — until a card is played.
        let mut played: Option<ActionBody> = None;
        let mut steps = 0;
        while played.is_none() && steps < 400 && snapshot(&state).result.is_none() {
            let at = snapshot(&state);
            // With a prompt open only its holder may act (§9.3); while both mulligans are open (R265),
            // the first seat still owing one; otherwise it is the active player's.
            let player = at
                .pending_for
                .or_else(|| at.mulligan_owed.first().copied())
                .unwrap_or(at.active);
            let options: Vec<ActionBody> = legal_actions(&state, player)
                .into_iter()
                .filter(|body| !NEVER_CHOOSE.contains(&body.action_type().as_str()))
                .collect();
            let choice = options
                .iter()
                .find(|body| matches!(body, ActionBody::Play { .. }))
                .or_else(|| options.iter().find(|body| !matches!(body, ActionBody::EndTurn)))
                .or_else(|| options.first())
                .cloned();
            let Some(choice) = choice else { break };

            let action = Action::new(choice.clone(), player, format!("step-{steps}"));
            let result = reduce(&state, &action);

            // `legalActions` and `reduce` are the same engine: anything offered must be accepted.
            assert_eq!(
                result.error,
                None,
                "{} was offered but refused",
                choice.action_type().as_str()
            );
            state = result.state;
            if matches!(choice, ActionBody::Play { .. }) {
                played = Some(choice);
            }
            steps += 1;
        }

        assert!(played.is_some(), "no card was played in {steps} actions");
        // A play of a card that came out of the deck, not an engine-invented instance.
        assert!(snapshot(&state).turn > 0);
        // The port's other two projections work on a state a real card has passed through.
        assert!(!hash_state(&state).is_empty());
        let view = serde_json::to_value(view_for(&state, PlayerId::P1)).expect("PlayerView serialises");
        assert!(!view.is_null());
    }
}

/// R258's deal, through the real port: `buildAiDeck` over the registered catalog, nothing banned.
/// The deck size is not imported (see `decks_the_engine_accepts`): it is whatever size the real
/// engine accepts, and the dealt pair must be a game `createGame` takes as it is.
mod all_randoms_deal_r258_src_match_engine_real_ts {
    use super::*;

    #[test]
    fn r258_deals_twenty_distinct_deckable_cards_the_real_engine_accepts_the_same_for_the_same_seed() {
        let pool = deckable_pool();
        // The smallest deck the engine accepts is the one size it accepts: L2's `DECK_SIZE`.
        let size = decks_the_engine_accepts(&pool, "r258-size").1.0.len();

        let p1 = deal_random_deck("r258-match:p1-deck");
        let p2 = deal_random_deck("r258-match:p2-deck");

        for deck in [&p1, &p2] {
            assert_eq!(deck.len(), size);
            // Distinct (MAX_COPIES is one) and deckable: every id a catalog card and none a Token.
            let mut distinct = deck.clone();
            distinct.sort();
            distinct.dedup();
            assert_eq!(distinct.len(), deck.len());
            for card_id in deck {
                let def = jackioh_cards::CATALOG.get(card_id);
                assert!(def.is_some(), "{card_id}");
                assert!(!def.is_some_and(is_token), "{card_id}");
            }
        }
        // The dealt pair is a game the engine starts, exactly as the match row will hand it over.
        let state = create_game(&args("r258-match", &(p1.clone(), p2.clone())));
        assert_eq!(snapshot(&state).phase, Phase::Setup);

        // Seeded: the same seed deals the same deck, in the same order, every time and in any process…
        assert_eq!(deal_random_deck("r258-match:p1-deck"), p1);
        // (TS asked a second, freshly built port here; the Rust port is functions, so asking again is it.)
        assert_eq!(deal_random_deck("r258-match:p1-deck"), p1);
        // …and another seed deals another deck.
        assert_ne!(p2, p1);
    }
}

/// R376's record, through the real port: a real match played to a concede and summarized off its
/// log, as `api/game-records.ts` summarizes every live match. What each field means is proved in
/// `packages/engine` and `packages/cards`; this proves the adapter hands the engine's answer over.
mod a_finished_matchs_record_r376_src_match_engine_real_ts {
    use super::*;

    fn fold_args(seed: &str, decks: &(Vec<String>, Vec<String>), log: &[Action]) -> FoldArgs {
        serde_json::from_value(json!({ "seed": seed, "decks": [decks.0, decks.1], "log": log }))
            .expect("FoldArgs from TS's literal")
    }

    /// TS's `act`: applies one action as `player`, which must be accepted, and logs it.
    fn act(state: &mut GameState, log: &mut Vec<Action>, player: PlayerId, body: ActionBody) {
        let action = Action::new(body, player, format!("r376-{}", log.len()));
        let result = reduce(state, &action);
        assert_eq!(
            result.error,
            None,
            "{} refused",
            action.body.action_type().as_str()
        );
        log.push(action);
        *state = result.state;
    }

    #[test]
    fn r376_summarizes_a_real_match_off_its_log_and_makes_nothing_of_an_unfinished_one() {
        let pool = deckable_pool();
        let (created, decks) = decks_the_engine_accepts(&pool, "r376-real");
        let mut state = begin_game(&created).state;

        let mut log: Vec<Action> = Vec::new();
        // Both keep their hands (R265), p1 ends its first turn, p2 concedes on its own.
        let keep = hand_ids(&state, PlayerId::P1);
        act(&mut state, &mut log, PlayerId::P1, ActionBody::Mulligan { keep });
        let keep = hand_ids(&state, PlayerId::P2);
        act(&mut state, &mut log, PlayerId::P2, ActionBody::Mulligan { keep });
        assert!(summarize_game(&fold_args("r376-real", &decks, &log)).is_none());
        act(&mut state, &mut log, PlayerId::P1, ActionBody::EndTurn);
        act(&mut state, &mut log, PlayerId::P2, ActionBody::Concede);

        let summary =
            summarize_game(&fold_args("r376-real", &decks, &log)).expect("a finished game's summary");
        let summary: Value = serde_json::to_value(&summary).expect("GameSummary serialises");
        assert_eq!(summary["first"], json!("p1"));
        assert_eq!(summary["winner"], json!("p1"));
        assert_eq!(summary["reason"], json!("concede"));
        assert_eq!(summary["turns"], json!(2));
        assert_eq!(summary["seats"]["p1"]["deck"], json!(decks.0));
        assert_eq!(
            summary["seats"]["p1"]["opening"].as_array().map(Vec::len),
            Some(3)
        );
        // §2.1, R244: the seat going second opens with its four cards and The Coin.
        assert_eq!(
            summary["seats"]["p2"]["opening"].as_array().map(Vec::len),
            Some(5)
        );
        assert_eq!(summary["seats"]["p2"]["drawn"].as_array().map(Vec::len), Some(1));
    }
}
