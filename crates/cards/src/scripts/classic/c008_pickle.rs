//! C #8 Pickle (SPEC §8.6 row 8, §6.3 Choose one, §10.6; R16, R79, R97, R113, R177, R1279000).
//! Spell, cost 1, Rare.
//!   Base:    "At the start of your opponent's next turn, they choose {choices|time|times}: they discard
//!            {discard|random card|random cards}, they exile the bottom {exile|card|cards} of their deck,
//!            or you draw {draw}." (3; 1, 1, 1)
//!   Radiant: the same words with 3; 2, 2, 2.
//!   Engine:  "Three mode prompts held by the opponent (§10.6), one after another, repeats allowed;
//!            "discard" discards at random from their hand (R682). Only choices that would do something
//!            are offered, and "you draw" always is. The first question waits for the start of the
//!            opponent's next turn while they are not active (R1279000); cast on their turn it asks at
//!            once. Each question runs on their turn clock and a timeout answers it with the AI policy
//!            (R79). Tunes: choices 3 ↑; discard 1 ↑; exile 1 ↑; draw 1 ↑."
//!
//! THE CHOICES are mode prompts the opponent holds (B5 E18, `chooseMode({ by: "enemy" })`): they read
//! the options and answer, and the answered step still runs as this card's controller, so "you draw"
//! draws for you. Each answer asks the next question, so the chain is a named step (`chosen`) and a
//! count carried in the prompt's data (§10.6): `n` is the question being asked, and the numbers the
//! card declares (`param`) are read once as the Spell resolves and carried with it, so every question
//! of one resolution reads the same numbers.
//!
//! THE WAIT (R1279000): no opponent-held prompt opens while its holder is not active, so while the opponent is not
//! the active player the Cry schedules the first question with the engine's `delay` (at the start of
//! the opponent's next turn) carrying the chain, and the `delayed` hook asks it when that turn starts.
//! Each later question follows its answer on the opponent's turn as before.
//!
//! ONLY CHOICES THAT WOULD DO SOMETHING are offered, read as each question is asked: "discard" while
//! their hand holds a card, "exile" while their deck does, and "you draw" always — a draw from an
//! empty deck is fatigue (§2.4), which still does something. The options name no card. R800's guard
//! stops a forced discard only while its holder is not active, so with every question opening on
//! their turn (R1279000) the guard sleeps and "discard" stands as long as their hand holds a card.
//!
//! "DISCARD" is random from their hand (R682: no "of your choice"), so no prompt opens and the next
//! question follows at once. You read only the discard events, never the cards (R177).
//!
//! THE CLOCK is the holder's: every question opens on the opponent's turn, so each runs on their turn
//! clock, and a timeout answers it with the AI policy (R79).

use jackioh_engine::effects::{choose_mode, chosen_options, discard_random, draw, exile_bottom_of_library};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-008";

const DISCARD: &str = "discard";
const EXILE: &str = "exile";
const DRAW: &str = "draw";

/// The card's numbers, read once as it resolves and carried through every question (§10.6).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Numbers {
    choices: i32,
    discard: i32,
    exile: i32,
    draw: i32,
}

/// The chain's data: the numbers, and which question is being asked (1-based). (TS `Numbers & { n }`.)
#[derive(Clone, Copy, Debug, PartialEq)]
struct ChainData {
    numbers: Numbers,
    n: i32,
}

fn read_numbers(ctx: &EffectContext<'_>) -> Numbers {
    Numbers {
        choices: param(ctx, "choices"),
        discard: param(ctx, "discard"),
        exile: param(ctx, "exile"),
        draw: param(ctx, "draw"),
    }
}

/// TS `numberIn(data, key)`: `typeof value === "number" && Number.isInteger(value)`, else None.
fn number_in(data: &IndexMap<String, Value>, key: &str) -> Option<i32> {
    let value = data.get(key)?.as_f64()?;
    if value.is_finite() && value.fract() == 0.0 {
        Some(value as i32)
    } else {
        None
    }
}

/// §10.6's data is JSON, so what comes back is narrowed, never cast.
fn chain_of(ctx: &EffectContext<'_>) -> Option<ChainData> {
    let n = number_in(&ctx.data, "n")?;
    let choices = number_in(&ctx.data, "choices")?;
    let discard_count = number_in(&ctx.data, "discard")?;
    let exile_count = number_in(&ctx.data, "exile")?;
    let draw_count = number_in(&ctx.data, "draw")?;
    Some(ChainData {
        numbers: Numbers {
            choices,
            discard: discard_count,
            exile: exile_count,
            draw: draw_count,
        },
        n,
    })
}

/// TS `{ ...chain }`: the chain as the prompt's data carries it.
fn chain_data(chain: &ChainData) -> Value {
    json!({
        "choices": chain.numbers.choices,
        "discard": chain.numbers.discard,
        "exile": chain.numbers.exile,
        "draw": chain.numbers.draw,
        "n": chain.n,
    })
}

fn cards(count: i32) -> String {
    if count == 1 {
        "1 card".to_string()
    } else {
        format!("{count} cards")
    }
}

/// The captions the opponent reads: from their seat, and naming no card.
fn labels_for(numbers: &Numbers) -> IndexMap<String, String> {
    IndexMap::from([
        (DISCARD.to_string(), format!("Discard {}", cards(numbers.discard))),
        (EXILE.to_string(), format!("Exile the bottom {} of your deck", cards(numbers.exile))),
        (DRAW.to_string(), format!("Your opponent draws {}", cards(numbers.draw))),
    ])
}

/// What an effect built in the same list as the next question will have taken from their piles by
/// the time that question opens: a question's options are read as its list is built, before the
/// discards or the exile ahead of it in that list apply, so the count they take is subtracted here.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Pending {
    hand_loss: Option<i32>,
    deck_loss: Option<i32>,
}

/// Question `chain.n`, offering only what would do something; nothing once every question is asked.
fn ask(ctx: &EffectContext<'_>, chain: &ChainData, pending: Pending) -> Vec<Effect> {
    if chain.n > chain.numbers.choices {
        return vec![];
    }
    let enemy = opponent_of(ctx.controller);
    let state: &GameState = &*ctx.state;
    let mut options: Vec<&str> = Vec::new();
    // R800's guard stops a forced discard only while its holder is not active; every question
    // opens on the holder's turn (R1279000), where the guard sleeps, so no read is needed here.
    if zone_count(state, enemy, OffFieldZone::Hand) - pending.hand_loss.unwrap_or(0) > 0 {
        options.push(DISCARD);
    }
    if zone_count(state, enemy, OffFieldZone::Library) - pending.deck_loss.unwrap_or(0) > 0 {
        options.push(EXILE);
    }
    options.push(DRAW);
    vec![choose_mode(json_as(json!({
        "by": "enemy",
        "options": options,
        "labels": labels_for(&chain.numbers),
        "step": "chosen",
        "prompt": format!("Pickle: your choice ({} of {})", chain.n, chain.numbers.choices),
        "data": chain_data(chain),
    })))]
}

/// The `chosen` step: the opponent's answer to question `n`, then question `n + 1`.
fn chosen(ctx: &EffectContext<'_>) -> Vec<Effect> {
    let Some(chain) = chain_of(ctx) else {
        return vec![];
    };
    let next = ChainData { n: chain.n + 1, ..chain };
    let enemy = opponent_of(ctx.controller);
    match chosen_options(ctx).first().map(String::as_str) {
        Some(DISCARD) => {
            // A hand emptied since the question was asked has nothing to discard: on to the next one.
            // The random discard lands first in the list, so the next question subtracts it (Pending).
            let hand_loss = chain.numbers.discard.min(zone_count(&*ctx.state, enemy, OffFieldZone::Hand));
            if hand_loss == 0 {
                return ask(ctx, &next, Pending::default());
            }
            let mut effects = vec![discard_random(json_as(json!({ "count": chain.numbers.discard, "player": "enemy" })))];
            effects.extend(ask(
                ctx,
                &next,
                Pending {
                    hand_loss: Some(hand_loss),
                    ..Pending::default()
                },
            ));
            effects
        }
        Some(EXILE) => {
            let deck_loss = chain.numbers.exile.min(zone_count(&*ctx.state, enemy, OffFieldZone::Library));
            let mut effects = vec![exile_bottom_of_library(json_as(json!({ "player": "enemy", "count": chain.numbers.exile })))];
            effects.extend(ask(
                ctx,
                &next,
                Pending {
                    deck_loss: Some(deck_loss),
                    ..Pending::default()
                },
            ));
            effects
        }
        Some(DRAW) => {
            let mut effects = vec![draw(json_as(json!({ "count": chain.numbers.draw })))];
            effects.extend(ask(ctx, &next, Pending::default()));
            effects
        }
        _ => vec![],
    }
}

/// The deferred first question's step, re-entered by the `delayed` hook (R1279000).
const DEFERRED_STEP: &str = "deferred";

/// TS `const pickle: Script`.
fn pickle() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let chain = ChainData {
                numbers: read_numbers(ctx),
                n: 1,
            };
            // R1279000: no opponent-held prompt opens while its holder is not active. While the opponent is not
            // the active player the first question waits for the start of their next turn, carrying
            // the chain in the delay's data; on their turn it asks at once, as before.
            if ctx.state.active == opponent_of(ctx.controller) {
                return ask(ctx, &chain, Pending::default());
            }
            vec![delay(json_as(json!({
                "at": { "phase": "start", "player": "enemy" },
                "step": DEFERRED_STEP,
                "data": chain_data(&chain),
            })))]
        })),
        resume: IndexMap::from([("chosen", hook(|ctx| chosen(ctx)))]),
        delayed: Some(hook(|ctx| {
            // R1279000: the wait is over; the chain rode the delay's data, so this asks exactly
            // what the Cry would have asked.
            let Some(chain) = chain_of(ctx) else {
                return vec![];
            };
            ask(ctx, &chain, Pending::default())
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = pickle();

    // The same script: the Radiant face differs only in its declared numbers (discard, exile and draw 2),
    // which `param` reads off the running face.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

/// R767000: the scripts v0.3.7b and every version before it ran, kept for their replays: the first
/// question opened during your turn, on the opponent's own clock (R79).
pub fn script_v0_3_7b() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            let chain = ChainData {
                numbers: read_numbers(ctx),
                n: 1,
            };
            ask(ctx, &chain, Pending::default())
        })),
        resume: IndexMap::from([("chosen", hook(|ctx| chosen(ctx)))]),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C #8 Pickle — SPEC §8.6 row 8, BUILD M9 Classic row C 8: "Three mode prompts held by the opponent,
// one after another, repeats allowed, the first waiting for the start of their next turn while they
// are not active (R1279000): they discard a card at random (R682), they exile the bottom card of
// their deck, or you draw 1; a mode that would do nothing is not offered (discard with an empty
// hand, exile with an empty deck), and "you draw" always is, fatigue included; each prompt runs on
// their turn clock and a timeout answers it with the AI policy (R79); your view names none of their
// remaining hand, their view never names your drawn card, and the mode options name no card; the
// paused prompts survive a JSON round trip; radiant: discard 2, exile the bottom 2, or draw 2; its
// tuned numbers (choices, discard, exile, draw) read through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PICKLE: &str = "classic-008";
    const FILLER: &str = "core-005"; // (1) Spell, p1's spare card (§2.5).
    const PALANTIR: &str = "classic-004"; // (2) Field Spell: steals and casts a (1) Spell the opponent plays.
    // p1's library: distinct definitions, top first.
    const MY_DECK: [&str; 4] = ["core-019", "core-025", "core-012", "core-020"];
    // p2's hand and library: definitions p1 holds nowhere.
    const THEIR_HAND: [&str; 3] = ["core-008", "core-011", "core-015"];
    const THEIR_DECK: [&str; 4] = ["core-006", "core-035", "core-048", "core-039"];

    use crate::js;

    /// TS `JSON.parse(JSON.stringify(state))`.
    fn round_trip(state: &GameState) -> GameState {
        serde_json::from_value(js(state)).expect("the state round-trips")
    }

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(value) => value,
            None => panic!("the scenario has no {what}"),
        }
    }

    fn open(s: &Scenario) -> PendingChoice {
        must(s.state().pending.clone(), "open prompt")
    }

    fn modes(s: &Scenario) -> Vec<String> {
        open(s)
            .options
            .iter()
            .map(|option| match &option.selection {
                Selection::Mode { option } => option.clone(),
                _ => "?".to_string(),
            })
            .collect()
    }

    fn labels(s: &Scenario) -> Vec<String> {
        open(s).options.iter().map(|option| option.label.clone()).collect()
    }

    #[derive(Default)]
    struct Piles {
        my_deck: Option<Vec<&'static str>>,
        their_hand: Option<Vec<&'static str>>,
        their_deck: Option<Vec<&'static str>>,
    }

    fn pickle(radiant_face: bool, piles: Piles) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": PICKLE, "radiant": radiant_face }, FILLER],
                "library": piles.my_deck.unwrap_or_else(|| MY_DECK.to_vec()),
            },
            "p2": {
                "hand": piles.their_hand.unwrap_or_else(|| THEIR_HAND.to_vec()),
                "library": piles.their_deck.unwrap_or_else(|| THEIR_DECK.to_vec()),
            },
        }))
    }

    fn defs(s: &Scenario, player: PlayerId, zone: &str) -> Vec<String> {
        s.pile(player, zone).into_iter().map(|card| card.def_id).collect()
    }

    fn reduce_json(state: &GameState, action: Value) -> ReduceResult {
        reduce(state, &json_as::<Action>(action))
    }

    mod c_n8_pickle {
        use super::*;

        #[test]
        fn declares_its_four_numbers_r386_choices_3_discard_exile_draw_1_radiant_2() {
            crate::register_all();
            assert_eq!(
                js(&crate::card_def(PICKLE).params),
                json!([
                    { "key": "choices", "base": 3, "radiant": 3, "better": "up", "step": 1, "min": 1 },
                    { "key": "discard", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 },
                    { "key": "exile", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 },
                    { "key": "draw", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 },
                ])
            );
            // The same script on both faces: only the numbers differ.
            let scripts = script();
            let (Some(base_cry), Some(radiant_cry)) = (&scripts.base.cry, &scripts.radiant.cry) else {
                panic!("both faces have a Cry");
            };
            assert!(Arc::ptr_eq(base_cry, radiant_cry));
            let (Some(base_step), Some(radiant_step)) =
                (scripts.base.resume.get("chosen"), scripts.radiant.resume.get("chosen"))
            else {
                panic!("both faces have the chosen step");
            };
            assert!(Arc::ptr_eq(base_step, radiant_step));
        }

        mod base {
            use super::*;

            #[test]
            fn r1279000_the_choices_wait_for_the_start_of_the_opponent_s_next_turn() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                // No opponent-held prompt opens while its holder is not active.
                assert!(s.state().pending.is_none());
                s.end_turn();
                let pending = open(&s);
                assert_eq!(s.state().active, P2);
                assert_eq!(pending.player_id, P2);
                assert_eq!(pending.kind, PromptKind::Mode);
                assert_eq!(modes(&s), vec!["discard", "exile", "draw"]);
                // All three answers, then the piles as ever.
                s.answer(json!("exile"));
                s.answer(json!("discard"));
                s.answer(json!("draw"));
                assert!(s.state().pending.is_none());
                assert_eq!(defs(&s, P2, "exile"), vec![THEIR_DECK[3]]);
                let grave = defs(&s, P2, "graveyard");
                assert_eq!(grave.len(), 1);
                // Their turn's opening draw joined their hand before the discard.
                let mut possible = THEIR_HAND.to_vec();
                possible.push(THEIR_DECK[0]);
                assert!(possible.contains(&grave[0].as_str()));
                assert_eq!(defs(&s, P1, "hand"), vec![FILLER, MY_DECK[0]]);
            }

            #[test]
            fn r1279000_a_pickle_cast_on_the_opponent_s_turn_asks_at_once() {
                crate::register_all();
                // p2 plays Pickle into p1's Palantir, which steals and casts it (R831001): the Cry
                // runs as p1 with the holder active, so the first question opens at once.
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": {
                        "hand": [FILLER],
                        "backrow": [{ "def": PALANTIR, "faceUp": true }],
                        "library": MY_DECK.to_vec(),
                    },
                    "p2": {
                        "hand": [PICKLE, FILLER],
                        "library": THEIR_DECK.to_vec(),
                    },
                }));
                s.play(PICKLE, json!({}));
                let pending = open(&s);
                assert_eq!(s.state().active, P2);
                assert_eq!(pending.player_id, P2);
                assert_eq!(pending.kind, PromptKind::Mode);
                assert_eq!(modes(&s), vec!["discard", "exile", "draw"]);
                s.answer(json!("draw"));
                assert_eq!(open(&s).player_id, P2);
                assert_eq!(defs(&s, P1, "hand"), vec![FILLER, MY_DECK[0]]);
            }

            #[test]
            fn r1279000_r767000_a_game_on_v0_3_7_folds_under_the_kept_script_the_first_question_opening_during_your_turn() {
                crate::register_all();
                let shipped = crate::shipped_catalog("v0.3.7").expect("v0.3.7 is carried");
                jackioh_engine::with_shipped(Some(shipped), || {
                    let mut s = pickle(false, Piles::default());
                    s.play(PICKLE, json!({}));
                    // The old script asked at once, answered on the non-active player's clock.
                    let pending = open(&s);
                    assert_eq!(s.state().active, P1);
                    assert_eq!(pending.player_id, P2);
                    assert_eq!(pending.kind, PromptKind::Mode);
                    assert_eq!(modes(&s), vec!["discard", "exile", "draw"]);
                });
            }

            #[test]
            fn three_questions_one_after_another_repeats_allowed_three_draws_draw_you_three_cards() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                s.end_turn();
                for _question in 0..3 {
                    assert_eq!(open(&s).player_id, P2);
                    s.answer(json!("draw"));
                }
                assert!(s.state().pending.is_none());
                let mut expected = vec![FILLER];
                expected.extend_from_slice(&MY_DECK[..3]);
                assert_eq!(defs(&s, P1, "hand"), expected);
                s.expect_in_zone(PICKLE, "graveyard");
            }

            #[test]
            fn r682_discard_discards_a_random_card_of_theirs_at_once_with_no_hand_pick() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                s.end_turn();
                s.answer(json!("discard"));
                // No follow-up pick: the random discard landed and the next question is open.
                assert_eq!(open(&s).kind, PromptKind::Mode);
                let grave = defs(&s, P2, "graveyard");
                assert_eq!(grave.len(), 1);
                // Their turn's opening draw joined their hand before the discard.
                let mut possible = THEIR_HAND.to_vec();
                possible.push(THEIR_DECK[0]);
                assert!(possible.contains(&grave[0].as_str()));
                s.expect_events(json!(["discarded"]));
            }

            #[test]
            fn exile_takes_the_bottom_card_of_their_deck() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                s.end_turn();
                s.answer(json!("exile"));
                // The turn's draw waits behind the open question, so the deck is whole: the
                // exile takes the bottom card and the library keeps the top three.
                assert_eq!(defs(&s, P2, "exile"), vec![THEIR_DECK[3]]);
                assert_eq!(defs(&s, P2, "library"), THEIR_DECK[..3].to_vec());
            }

            #[test]
            fn a_mix_discard_exile_draw_in_any_order() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                s.end_turn();
                s.answer(json!("exile"));
                s.answer(json!("discard"));
                s.answer(json!("draw"));
                assert!(s.state().pending.is_none());
                assert_eq!(defs(&s, P2, "exile"), vec![THEIR_DECK[3]]);
                let grave = defs(&s, P2, "graveyard");
                assert_eq!(grave.len(), 1);
                let mut possible = THEIR_HAND.to_vec();
                possible.push(THEIR_DECK[0]);
                assert!(possible.contains(&grave[0].as_str()));
                assert_eq!(defs(&s, P1, "hand"), vec![FILLER, MY_DECK[0]]);
            }

            #[test]
            fn discard_is_not_offered_while_their_hand_is_empty_nor_exile_while_their_deck_is() {
                crate::register_all();
                let mut s = pickle(
                    false,
                    Piles {
                        their_hand: Some(vec![]),
                        their_deck: Some(vec![]),
                        ..Piles::default()
                    },
                );
                s.play(PICKLE, json!({}));
                s.end_turn();
                assert_eq!(modes(&s), vec!["draw"]);
            }

            #[test]
            fn each_question_reads_their_piles_as_it_is_asked_an_exile_that_empties_their_deck_takes_exile_off_the_next() {
                crate::register_all();
                // The turn's draw waits behind the open question, so one card in the deck is
                // one card at the first question.
                let mut s = pickle(false, Piles { their_deck: Some(vec!["core-006"]), ..Piles::default() });
                s.play(PICKLE, json!({}));
                s.end_turn();
                assert_eq!(modes(&s), vec!["discard", "exile", "draw"]);
                s.answer(json!("exile"));
                assert_eq!(modes(&s), vec!["discard", "draw"]);
            }

            #[test]
            fn a_discard_that_empties_their_hand_takes_discard_off_the_next_question() {
                crate::register_all();
                // No deck to draw from: the one card is offered, then gone.
                let mut s = pickle(
                    false,
                    Piles { their_hand: Some(vec!["core-008"]), their_deck: Some(vec![]), ..Piles::default() },
                );
                s.play(PICKLE, json!({}));
                s.end_turn();
                assert_eq!(modes(&s), vec!["discard", "draw"]);
                s.answer(json!("discard"));
                assert_eq!(defs(&s, P2, "graveyard"), vec!["core-008"]);
                assert_eq!(modes(&s), vec!["draw"]);
            }

            #[test]
            fn s2_4_you_draw_is_always_offered_and_from_an_empty_deck_it_is_fatigue() {
                crate::register_all();
                // Their side keeps cards, so their turn never auto-ends into a fourth fatigue draw.
                let mut s = pickle(
                    false,
                    Piles {
                        my_deck: Some(vec![]),
                        their_hand: Some(vec!["core-008"]),
                        their_deck: Some(vec!["core-006"]),
                    },
                );
                s.play(PICKLE, json!({}));
                s.end_turn();
                s.answer(json!("draw"));
                s.answer(json!("draw"));
                s.answer(json!("draw"));
                assert!(s.state().pending.is_none());
                // Three fatigue draws: 1 + 2 + 3.
                s.expect_health(P1, 24);
            }

            #[test]
            fn r79_a_timeout_on_the_holder_s_turn_answers_every_open_question_with_the_ai_policy_then_ends_the_turn() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                s.end_turn();
                // A question is open for p2 on their turn; the timeout takes it from here.
                open(&s);
                let result = reduce_json(s.state(), json!({ "type": "timeout", "playerId": "p2", "nonce": "pickle-timeout" }));
                assert!(result.error.is_none());
                // The turn clock answered every question in turn, then ended the turn (R79):
                // nothing is open, all three were answered, and p1 is active.
                assert!(result.state.pending.is_none());
                assert_eq!(result.state.active, P1);
                assert_eq!(
                    result
                        .events
                        .iter()
                        .filter(|event| event.event_type() == GameEventType::PromptAnswered)
                        .count(),
                    3
                );
            }

            #[test]
            fn r177_your_view_names_none_of_their_remaining_hand() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                s.end_turn();
                s.answer(json!("discard"));
                let mine = js(&s.view(P1)).to_string();
                for card in s.hand(P2) {
                    assert!(!mine.contains(&format!("\"{}\"", card.id)));
                    assert!(!mine.contains(&card.def_id));
                }
                // The random discard itself is public: the graveyard names it.
                for card in s.pile(P2, "graveyard") {
                    assert!(mine.contains(&card.id));
                }
            }

            #[test]
            fn r97_their_view_never_names_the_card_you_draw() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                s.end_turn();
                s.answer(json!("draw"));
                let drawn = s.card(MY_DECK[0]).clone();
                s.expect_in_zone(&drawn, "hand");
                let theirs = js(&s.view(P2)).to_string();
                assert!(!theirs.contains(&format!("\"{}\"", drawn.id)));
                assert!(!theirs.contains(MY_DECK[0]));
            }

            #[test]
            fn the_mode_options_name_no_card_only_what_each_would_do() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                s.end_turn();
                let view = js(&s.view(P2));
                let theirs = must(Some(&view["pending"]).filter(|pending| !pending.is_null()), "p2's view of the question");
                assert_eq!(theirs["forYou"], json!(true));
                assert_eq!(
                    labels(&s),
                    vec!["Discard 1 card", "Exile the bottom 1 card of your deck", "Your opponent draws 1 card"]
                );
                // p1 reads only that p2 is choosing.
                assert_eq!(js(&s.view(P1))["pending"], json!({ "forYou": false, "pendingFor": "p2" }));
            }

            #[test]
            fn s9_3_the_paused_questions_survive_a_json_round_trip_and_resume_through_reduce() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                s.end_turn();
                s.answer(json!("draw"));
                let revived = round_trip(s.state());
                assert_eq!(&revived, s.state());
                let second = must(revived.pending.clone(), "the second question");
                let answered = reduce_json(
                    &revived,
                    json!({
                        "type": "answer",
                        "playerId": "p2",
                        "choiceId": second.id,
                        "selection": [{ "pick": "mode", "option": "exile" }],
                        "nonce": "pickle-round-trip-1",
                    }),
                );
                assert!(answered.error.is_none());
                let third = must(answered.state.pending.clone(), "the third question");
                assert_eq!(third.player_id, P2);
                // And the discard pick, paused in turn, round-trips too.
                let again = round_trip(&answered.state);
                let picked = reduce_json(
                    &again,
                    json!({
                        "type": "answer",
                        "playerId": "p2",
                        "choiceId": third.id,
                        "selection": [{ "pick": "mode", "option": "draw" }],
                        "nonce": "pickle-round-trip-2",
                    }),
                );
                assert!(picked.error.is_none());
                assert!(picked.state.pending.is_none());
                assert!(picked.state.work.is_empty());
            }

            #[test]
            fn r682_the_random_discard_comes_from_the_match_rng_the_same_game_discards_the_same_card() {
                crate::register_all();
                let mut first = pickle(false, Piles::default());
                first.play(PICKLE, json!({}));
                first.end_turn();
                first.answer(json!("discard"));
                let mut second = pickle(false, Piles::default());
                second.play(PICKLE, json!({}));
                second.end_turn();
                second.answer(json!("discard"));
                let ids = |s: &Scenario| -> Vec<String> {
                    s.events()
                        .iter()
                        .filter_map(|event| match event {
                            GameEvent::Discarded { instance_id, .. } => Some(instance_id.clone()),
                            _ => None,
                        })
                        .collect()
                };
                assert_eq!(ids(&first), ids(&second));
            }

            #[test]
            fn r386_an_upgrade_of_choices_asks_a_fourth_question() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                step_param(s.card_mut(PICKLE), "choices", 1);
                s.play(PICKLE, json!({}));
                s.end_turn();
                for _question in 0..4 {
                    s.answer(json!("draw"));
                }
                assert!(s.state().pending.is_none());
                let mut expected = vec![FILLER];
                expected.extend_from_slice(&MY_DECK);
                assert_eq!(defs(&s, P1, "hand"), expected);
            }

            #[test]
            fn r386_an_upgrade_of_draw_makes_you_draw_2() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                step_param(s.card_mut(PICKLE), "draw", 1);
                s.play(PICKLE, json!({}));
                s.end_turn();
                s.answer(json!("draw"));
                assert_eq!(defs(&s, P1, "hand"), vec![FILLER, MY_DECK[0], MY_DECK[1]]);
            }

            #[test]
            fn r386_an_upgrade_of_discard_makes_them_discard_2_and_of_exile_exiles_2() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                step_param(s.card_mut(PICKLE), "discard", 1);
                step_param(s.card_mut(PICKLE), "exile", 1);
                s.play(PICKLE, json!({}));
                s.end_turn();
                s.answer(json!("discard"));
                let grave = defs(&s, P2, "graveyard");
                assert_eq!(grave.len(), 2);
                let mut possible = THEIR_HAND.to_vec();
                possible.push(THEIR_DECK[0]);
                for def_id in &grave {
                    assert!(possible.contains(&def_id.as_str()));
                }
                s.answer(json!("exile"));
                assert_eq!(defs(&s, P2, "exile"), vec![THEIR_DECK[3], THEIR_DECK[2]]);
            }

            #[test]
            fn r386_a_degrade_of_choices_asks_only_two_questions() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                step_param(s.card_mut(PICKLE), "choices", -1);
                s.play(PICKLE, json!({}));
                s.end_turn();
                s.answer(json!("draw"));
                s.answer(json!("draw"));
                assert!(s.state().pending.is_none());
            }

            #[test]
            fn r800_the_guard_sleeps_on_the_holder_s_own_turn_so_the_deferred_discard_is_offered_and_lands() {
                crate::register_all();
                // p2's Disruptor guards p2's side, but the guard stops only discards off the
                // holder's turn (R800): the deferred question opens on p2's turn, where the
                // guard sleeps, so "discard" is offered and lands with no `discardPrevented`.
                let mut s = scenario(json!({
                    "p1": { "hand": [PICKLE, FILLER], "library": MY_DECK.to_vec() },
                    "p2": {
                        "hand": THEIR_HAND.to_vec(),
                        "library": THEIR_DECK.to_vec(),
                        "backrow": [{ "def": "meditative-001", "faceUp": true, "lane": 1 }],
                    },
                }));
                s.play(PICKLE, json!({}));
                s.end_turn();
                assert_eq!(modes(&s), vec!["discard", "exile", "draw"]);
                s.answer(json!("discard"));
                assert_eq!(defs(&s, P2, "graveyard").len(), 1);
                assert_eq!(
                    s.events()
                        .iter()
                        .filter(|event| event.event_type() == GameEventType::DiscardPrevented)
                        .count(),
                    0
                );
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r682_they_discard_2_cards_at_random() {
                crate::register_all();
                let mut s = pickle(true, Piles::default());
                s.play(PICKLE, json!({}));
                s.end_turn();
                assert_eq!(
                    labels(&s),
                    vec!["Discard 2 cards", "Exile the bottom 2 cards of your deck", "Your opponent draws 2 cards"]
                );
                s.answer(json!("discard"));
                assert_eq!(open(&s).kind, PromptKind::Mode);
                let grave = defs(&s, P2, "graveyard");
                assert_eq!(grave.len(), 2);
                let mut possible = THEIR_HAND.to_vec();
                possible.push(THEIR_DECK[0]);
                for def_id in &grave {
                    assert!(possible.contains(&def_id.as_str()));
                }
            }

            #[test]
            fn a_hand_of_one_card_discards_that_one() {
                crate::register_all();
                // No deck to draw from, so the one card stays the only card to discard.
                let mut s = pickle(
                    true,
                    Piles { their_hand: Some(vec!["core-008"]), their_deck: Some(vec![]), ..Piles::default() },
                );
                s.play(PICKLE, json!({}));
                s.end_turn();
                s.answer(json!("discard"));
                assert!(s.hand(P2).is_empty());
                assert_eq!(defs(&s, P2, "graveyard"), vec!["core-008"]);
            }

            #[test]
            fn they_exile_the_bottom_2_cards_of_their_deck() {
                crate::register_all();
                let mut s = pickle(true, Piles::default());
                s.play(PICKLE, json!({}));
                s.end_turn();
                s.answer(json!("exile"));
                assert_eq!(defs(&s, P2, "exile"), vec![THEIR_DECK[3], THEIR_DECK[2]]);
            }

            #[test]
            fn you_draw_2() {
                crate::register_all();
                let mut s = pickle(true, Piles::default());
                s.play(PICKLE, json!({}));
                s.end_turn();
                s.answer(json!("draw"));
                assert_eq!(defs(&s, P1, "hand"), vec![FILLER, MY_DECK[0], MY_DECK[1]]);
            }

            #[test]
            fn an_exile_of_2_that_empties_their_deck_takes_exile_off_the_next_question() {
                crate::register_all();
                let mut s = pickle(true, Piles { their_deck: Some(vec!["core-006", "core-035"]), ..Piles::default() });
                s.play(PICKLE, json!({}));
                s.end_turn();
                s.answer(json!("exile"));
                assert_eq!(modes(&s), vec!["discard", "draw"]);
            }
        }
    }
}
