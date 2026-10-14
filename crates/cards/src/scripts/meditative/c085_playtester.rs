//! M #85 Playtester (SPEC §8.8 row 85): (2) Unit, Rare, 4/5 → 8/10.
//!
//! Base:    "Start of turn: Choose one: add {books|Book of Buff|Books of Buff} or {books|Book of
//!          Nerf|Books of Nerf} to your hand."
//! Radiant: "Start of turn: Choose one: add {books|Radiant Book of Buff|Radiant Books of Buff} or
//!          {books|Radiant Book of Nerf|Radiant Books of Nerf} to your hand."
//! Engine: at its controller's start of turn (R62), `choose_mode` with two modes, then `add_to_hand`
//! of C+ #71 `classicplus-071` or C+ #72 `classicplus-072` (Radiant: `radiant: true`). Both are real
//! cards, named, so no pool and no R387. The prompt is the controller's; a timeout answers it with
//! the AI policy (R79); the opponent sees a prompt open and a hidden card arrive.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-085";

/// C+ #71 Book of Buff, the first mode.
const BUFF_BOOK: &str = "classicplus-071";
/// C+ #72 Book of Nerf, the second mode.
const NERF_BOOK: &str = "classicplus-072";

/// The two modes, worded as the faces print them. The same labels on both faces.
const BUFF: &str = "Book of Buff";
const NERF: &str = "Book of Nerf";

/// The resume step the mode prompt re-enters (§10.6).
const STEP_BOOK: &str = "book";

fn playtester(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(|_ctx| {
            vec![choose_mode(json_as(json!({
                "options": [BUFF, NERF],
                "step": STEP_BOOK,
                "prompt": "Playtester: choose one",
            })))]
        })),
        resume: IndexMap::from([(
            STEP_BOOK,
            hook(move |ctx| {
                let picked = chosen_options(ctx).into_iter().next();
                let id = match picked.as_deref() {
                    Some(NERF) => NERF_BOOK,
                    _ => BUFF_BOOK,
                };
                let n = param(&*ctx, "books").max(0) as usize;
                (0..n)
                    .map(|_| add_to_hand(json_as(json!({ "defId": id, "radiant": radiant }))))
                    .collect()
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The Radiant face's Book is Radiant; the option labels are the same on both faces.
    CardScripts { base: playtester(false), radiant: playtester(true) }
}

// M #85 Playtester — SPEC §8.8 row 85, BUILD M10 row M 85: "At your start of turn only, a mode
// prompt for its controller: Book of Buff (C+ 71) or Book of Nerf (C+ 72) to hand (MD-E8), hidden
// from the opponent, who sees the prompt; a timeout answers by the AI policy (R79); the hand cap
// burns it; books reads through `param()`; radiant 8/10 and the Book is Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PLAYTESTER: &str = "meditative-085";
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// p1's Playtester stands on the field (base unless `radiant_face`); both sides hold and draw
    /// fillers, so turns pass normally.
    fn standing(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [FILLER],
                "field": [{ "def": PLAYTESTER, "radiant": radiant_face, "lane": 1 }],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn option_labels(s: &Scenario) -> Vec<String> {
        s.state()
            .pending
            .as_ref()
            .map(|pending| pending.options.iter().map(|option| option.label.clone()).collect())
            .unwrap_or_default()
    }

    fn reduce_json(state: &GameState, action: Value) -> ReduceResult {
        reduce(state, &json_as::<Action>(action))
    }

    fn must_pending(state: &GameState) -> PendingChoice {
        state.pending.clone().expect("a prompt is open")
    }

    mod m85_playtester {
        use super::*;

        #[test]
        fn is_a_2_4_5_rare_unit_radiant_8_10_with_books_1_to_1() {
            let def = crate::card_def(ID);
            assert_eq!(def.id, PLAYTESTER);
            assert_eq!(def.cost, CardCost::Fixed(2));
            assert_eq!(def.type_, CardType::Unit);
            assert_eq!(def.rarity, Rarity::Rare);
            assert!(def.tags.is_empty());
            assert_eq!(
                [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
                [Some(4), Some(5), Some(8), Some(10)]
            );
            let refs = def.refs.expect("the Books");
            assert_eq!(refs, vec![BUFF_BOOK.to_string(), NERF_BOOK.to_string()]);
            let params = def.params.expect("books");
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].key, "books");
            assert_eq!((params[0].base, params[0].radiant), (1, 1));
        }

        mod base {
            use super::*;

            #[test]
            fn r1161_your_start_of_turn_opens_a_mode_prompt_for_you() {
                let mut s = standing("playtester", false);
                s.start_turn();
                let pending = must_pending(s.state());
                assert_eq!(pending.kind, PromptKind::Mode);
                assert_eq!(pending.player_id, P1);
                assert_eq!(option_labels(&s), [BUFF.to_string(), NERF.to_string()]);
                s.expect_events(json!(["turnStarted", "promptOpened"]));
            }

            #[test]
            fn r1161_the_answer_adds_that_book_hidden_from_the_opponent() {
                for (option, id) in [(BUFF, BUFF_BOOK), (NERF, NERF_BOOK)] {
                    let mut s = standing("playtester-answer", false);
                    let before: Vec<String> =
                        s.hand(P1).iter().map(|card| card.id.clone()).collect();
                    s.start_turn();
                    s.answer(json!(option));
                    let hand = s.hand(P1);
                    // The turn draw plus the Book.
                    assert_eq!(hand.len(), before.len() + 2);
                    let made: Vec<&CardInstance> =
                        hand.iter().filter(|card| !before.contains(&card.id)).collect();
                    assert_eq!(made.len(), 2);
                    let book =
                        made.into_iter().find(|card| card.def_id == id).expect("the Book");
                    assert!(!book.radiant);
                    // Hidden from the opponent: a count, never the card.
                    let view: Value =
                        serde_json::to_value(s.view(P2)).expect("the view serialises");
                    assert_eq!(view["opponent"]["hand"], json!({ "count": hand.len() }));
                    assert!(!serde_json::to_string(&view["events"]).expect("events").contains(id));
                }
            }

            #[test]
            fn no_prompt_at_the_opponents_start_of_turn() {
                let mut s = standing("playtester-foe", false);
                s.end_turn();
                assert!(s.state().pending.is_none());
            }

            #[test]
            fn r79_a_timeout_answers_by_the_ai_policy() {
                crate::register_all();
                let mut s = standing("playtester-timeout", false);
                s.start_turn();
                let open = must_pending(s.state());
                let result =
                    reduce_json(s.state(), json!({ "type": "timeout", "playerId": "p1", "nonce": "playtester-timeout" }));
                assert!(result.error.is_none());
                assert!(result.state.pending.is_none(), "the prompt {} is answered", open.id);
                // A Book arrived in p1's hand.
                let hand = result.state.players.p1.hand.clone();
                assert!(hand.iter().any(|card| card.def_id == BUFF_BOOK || card.def_id == NERF_BOOK));
            }

            #[test]
            fn a_full_hand_burns_the_book() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "playtester-full",
                    "p1": {
                        "hand": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                        "field": [{ "def": PLAYTESTER, "lane": 1 }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.start_turn();
                s.answer(json!(BUFF));
                assert!(s.events().iter().any(|event| matches!(event, GameEvent::Burned { .. })));
                assert!(!s.hand(P1).iter().any(|card| card.def_id == BUFF_BOOK));
            }

            #[test]
            fn r386_books_reads_through_param() {
                crate::register_all();
                let mut s = standing("playtester-param", false);
                crate::upgrade_number(&mut s, PLAYTESTER, "books");
                let before = s.hand(P1).len();
                s.start_turn();
                s.answer(json!(NERF));
                // The turn draw plus two Books.
                assert_eq!(s.hand(P1).len(), before + 3);
                assert_eq!(
                    s.hand(P1).iter().filter(|card| card.def_id == NERF_BOOK).count(),
                    2
                );
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn radiant_the_book_is_radiant() {
                let mut s = standing("playtester-radiant", true);
                let before: Vec<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();
                // The option labels are the same on both faces.
                s.start_turn();
                assert_eq!(option_labels(&s), [BUFF.to_string(), NERF.to_string()]);
                s.answer(json!(BUFF));
                let hand = s.hand(P1);
                let made: Vec<&CardInstance> =
                    hand.iter().filter(|card| !before.contains(&card.id)).collect();
                let book =
                    made.into_iter().find(|card| card.def_id == BUFF_BOOK).expect("the Book");
                assert!(book.radiant);
            }
        }
    }
}
