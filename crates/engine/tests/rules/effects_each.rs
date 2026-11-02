//! `forEachCard`: one effect for each card of a set a clause reads off the board (R113, R66).
//! A list a prompt split is continued by building it again and skipping what already ran, which is
//! exact only for a list whose shape does not hang on the board. A clause over a board set reads it
//! once, as the list reaches it, and a pause inside it resumes over that same set (`EffectPart.memo`).
//! #94 Genn's Greed and #30 radiant use it.

use std::sync::OnceLock;

use jackioh_engine::effects::{ForEachCardArgs, choose_mode, draw_from_library, for_each_card};
use std::sync::Arc;

use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{Bench, new_game, set_library};

const DRAWER: &str = "each-drawer";

fn drawer() -> CardDef {
    json_as(json!({
        "id": DRAWER,
        "index": "each-drawer",
        "name": "each drawer",
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": "draw every card in your library; ask after the first" },
        "radiant": { "keywords": [], "text": "draw every card in your library; ask after the first" },
    }))
}

/// Which card the question follows, set once the library's ids are known: unset for "never".
type AsksAfter = Arc<OnceLock<String>>;

/// "Draw every card in your library", the first draw followed by a question: by the answer the first
/// card has left the library, so a list rebuilt from the board would, resumed by index, skip a card.
fn script(asks_after: AsksAfter) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            let asks_after = asks_after.clone();
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(|ctx: &mut EffectContext<'_>| {
                    zone_cards(ctx.state, ctx.controller, OffFieldZone::Library)
                        .into_iter()
                        .map(|card| card.id.clone())
                        .collect()
                }),
                each: Arc::new(move |instance_id: &str| {
                    let asks_after = asks_after.clone();
                    let instance_id = instance_id.to_string();
                    lazy_part("each-draw", move |_ctx, _memo| {
                        let mut effects =
                            vec![draw_from_library(json_as(json!({ "instanceId": instance_id })))];
                        if asks_after.get().map(String::as_str) == Some(instance_id.as_str()) {
                            effects.push(choose_mode(json_as(
                                json!({ "options": ["ok"], "step": "ok", "prompt": "a question" }),
                            )));
                        }
                        EffectPart { effects, memo: None }
                    })
                }),
            })]
        })),
        resume: IndexMap::from([("ok", hook(|_ctx| vec![]))]),
        ..Script::default()
    }
}

fn board(seed: &str, asks_after: AsksAfter) -> (GameState, Vec<CardInstance>) {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    catalog.insert(DRAWER.to_string(), drawer());
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    registry.insert(
        DRAWER.to_string(),
        CardScripts {
            base: script(asks_after.clone()),
            radiant: script(asks_after),
        },
    );
    register_scripts(registry);
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    // A copy: the library array itself empties as the cards are drawn.
    let library = set_library(
        &mut state,
        PlayerId::P1,
        &[plain.id.clone(), plain.id.clone(), plain.id.clone()],
    );
    state.players[PlayerId::P1].hand = vec![];
    (state, library)
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

mod for_each_card_r113_r66 {
    use super::*;

    #[test]
    fn r113_applies_its_effect_to_every_card_of_the_set_in_the_set_s_order() {
        let (mut state, library) = board("each-plain", Arc::new(OnceLock::new()));
        let card = new_instance(
            &mut state,
            DRAWER,
            PlayerId::P1,
            Zone::Resolving { player: PlayerId::P1 },
        );
        let mut sink = Bench::new(&state);
        run_hook_resumable(&mut sink.sink(&mut state), &card, "cry", Default::default());

        assert!(state.pending.is_none());
        assert_eq!(ids(&state.players[PlayerId::P1].hand), ids(&library));
        assert_eq!(state.players[PlayerId::P1].library, Vec::<CardInstance>::new());
    }

    #[test]
    fn r113_resumes_after_a_question_over_the_set_it_began_with_though_the_board_it_was_read_from_has_moved()
    {
        let asks: AsksAfter = Arc::new(OnceLock::new());
        let (mut state, library) = board("each-paused", asks.clone());
        let first = library[0].id.clone();
        asks.set(first.clone()).expect("set once");
        let card = new_instance(
            &mut state,
            DRAWER,
            PlayerId::P1,
            Zone::Resolving { player: PlayerId::P1 },
        );
        let mut sink = Bench::new(&state);
        run_hook_resumable(&mut sink.sink(&mut state), &card, "cry", Default::default());

        let Some(pending) = state.pending.clone() else {
            panic!("expected the question");
        };
        assert_eq!(ids(&state.players[PlayerId::P1].hand), vec![first.clone()]);

        // The continuation is data, so it survives the round trip a stored game makes (§9.3).
        let mut stored: GameState =
            serde_json::from_value(serde_json::to_value(&state).expect("the state serializes"))
                .expect("and parses");
        let mut answered = Bench::new(&stored);
        let refused = answer_prompt(
            &mut answered.sink(&mut stored),
            &AnswerInput {
                player_id: PlayerId::P1,
                choice_id: pending.id.clone(),
                selection: vec![Selection::Mode {
                    option: "ok".to_string(),
                }],
            },
        );
        assert!(refused.is_ok());

        // Both cards still owed are drawn: the clause resumed over the three it read, not the two left.
        assert!(stored.pending.is_none());
        assert_eq!(ids(&stored.players[PlayerId::P1].hand), ids(&library));
        assert_eq!(stored.players[PlayerId::P1].library, Vec::<CardInstance>::new());
    }
}
