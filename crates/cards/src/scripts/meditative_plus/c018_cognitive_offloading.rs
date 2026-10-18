//! M+ #18 Cognitive Offloading (SPEC §8.9 row 18; R58, R635, R1183002, R1183003): (1) Spell, Epic.
//!
//! Base:    "Cast on draw: Gain {mana} mana and draw {draw|card|cards}. The AI plays the rest of
//!          your turn."
//! Radiant: the same words; mana 3 and draw 2.
//! Engine: Cast on draw (§2.4, `StaticFlags.cast_on_draw`; never dealt at setup, R635; the draw
//! repeats once it is cast, R58): temporary mana (`gain_mana`, §2.3), a draw, then the autopilot
//! (`autopilot`, R1183002): on its caster's own turn the engine plays the rest of that turn by the
//! §10.7 scorer (`subsystems::autopilot`, R1183003), at most `AUTOPILOT_MAX_ACTIONS` actions, and
//! ends it; drawn on the opponent's turn it gains and draws, and nothing more. Played from the hand
//! (R58's cap, a copy) it does the same. Tunes: mana 1 ↑ (Radiant 3); draw 1 ↑ (Radiant 2).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-018";

fn offloading() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| {
            vec![
                gain_mana(json_as(json!({ "amount": param(ctx, "mana") }))),
                draw(json_as(json!({ "count": param(ctx, "draw") }))),
                autopilot(),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // Both faces run the one script: only the declared mana and draw differ (1 and 1, Radiant 3 and 2).
    CardScripts {
        base: offloading(),
        radiant: offloading(),
    }
}

// M+ #18 Cognitive Offloading — SPEC §8.9 row 18, BUILD M11 row M+ 18: "drawn on your turn it is
// cast: +1 temporary mana, a draw (repeating under R58), then every action until the turn ends is the
// engine's, each one legal, the turn ending within the bound, the seat's own actions refused
// meanwhile; the same seed plays the same turn; drawn on the opponent's turn (an effect's draw) it
// gains and draws only; never dealt at setup (R635); mana and draw read through `param()`; radiant 3
// mana and 2 cards".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// Ceaseless Void, (100): drawn, it is never affordable, so it plays nothing.
    const VOID: &str = "core-100";
    /// Tempo Timmy, (1) 3/3 Rush and First Strike.
    const TIMMY: &str = "core-011";
    /// Mr. Vanilla, (1) 4/4.
    const VANILLA: &str = "core-008";
    /// Pickle, (1): the opponent answers its choices.
    const PICKLE: &str = "classic-008";
    const FILLER: &str = "core-005";

    /// p2 to end turn 9; p1's library is Cognitive Offloading (Radiant if `radiant`) on top of Voids,
    /// so p1's turn 10 draws it. p1 holds `hand` and fields `field`; p2 keeps a card to play.
    fn on_top(seed: &str, radiant: bool, hand: Value, field: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "active": "p2",
            "p1": {
                "hand": hand,
                "field": field,
                "library": [{ "def": ID, "radiant": radiant }, VOID, VOID, VOID, VOID],
            },
            "p2": { "hand": [TIMMY], "library": [FILLER, FILLER, FILLER, FILLER] },
        }))
    }

    fn engaged(s: &Scenario, player: PlayerId) -> bool {
        s.events()
            .iter()
            .any(|event| matches!(event, GameEvent::AutopilotEngaged { player: who, .. } if *who == player))
    }

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    mod mp18_cognitive_offloading {
        use super::*;

        #[test]
        fn drawn_on_your_turn_it_gains_1_mana_and_draws_1_and_the_draw_repeats() {
            let mut s = on_top("offload-mana", false, json!([]), json!([]));
            s.end_turn();
            // Nothing to play, nothing to attack with: the autopilot ends the turn at once, so the
            // mana it leaves is the refresh and the 1 it gained.
            let side = &s.state().players.p1;
            assert_eq!(side.turn_log.unspent_at_end, Some(side.mana.max + 1));
            // Its own draw and the turn's repeated draw (§2.4): two Voids, and it in the graveyard.
            assert_eq!(def_ids(&side.hand), vec![VOID.to_string(), VOID.to_string()]);
            assert!(def_ids(&side.graveyard).contains(&ID.to_string()));
            assert!(engaged(&s, P1));
            assert_eq!(s.state().active, P2, "and the turn is over");
        }

        #[test]
        fn r1183002_then_the_engine_plays_the_rest_of_your_turn_and_ends_it() {
            let mut s = on_top("offload-turn", false, json!([TIMMY]), json!([VANILLA]));
            s.end_turn();
            // It played Tempo Timmy and swung Mr. Vanilla at the empty board's hero for 4; Timmy's Rush
            // reaches no hero the turn it lands (§6.1).
            assert!(
                (1..=5).any(|lane| s.unit(P1, lane).is_some_and(|unit| unit.def_id == TIMMY)),
                "Tempo Timmy was played"
            );
            s.expect_health("p2", HERO_HEALTH - 4);
            assert_eq!(s.state().active, P2, "the turn was ended");
            assert_eq!(s.state().players.p1.autopilot, None, "the status ended with it");
        }

        #[test]
        fn r1183003_each_action_was_one_the_turn_offered_and_the_turn_ended_within_the_bound() {
            let mut s = on_top("offload-bound", false, json!([TIMMY, VANILLA]), json!([VANILLA]));
            let turn = s.state().turn + 1;
            s.end_turn();
            // The autopilot's turn: what p1 played and declared on turn 10.
            let mut taken = 0;
            let mut in_turn = false;
            for event in s.events() {
                match event {
                    GameEvent::TurnStarted { player, turn: t, .. } => in_turn = *player == P1 && *t == turn,
                    GameEvent::CardPlayed { player: P1, .. } | GameEvent::AttackDeclared { .. } if in_turn => {
                        taken += 1
                    }
                    _ => {}
                }
            }
            assert!(taken >= 3, "two plays and attacks were taken");
            assert!(taken <= AUTOPILOT_MAX_ACTIONS as usize);
            assert_eq!(s.state().active, P2);
        }

        #[test]
        fn r1183002_the_seats_own_actions_are_refused_while_the_opponent_answers() {
            let mut s = on_top("offload-pickle", false, json!([PICKLE]), json!([]));
            s.end_turn();
            // The autopilot cast Pickle: p2 chooses, in p1's turn, which waits.
            let pending = s.state().pending.clone().expect("Pickle's choice is open");
            assert_eq!(pending.player_id, P2);
            assert_eq!(s.state().active, P1);
            s.expect_refused_with(|s| s.end_turn(), "prompt");
            let own: Vec<ActionBody> = legal_actions(s.state(), P1)
                .into_iter()
                .filter(|action| action.action_type() != ActionType::Concede)
                .collect();
            assert!(own.is_empty(), "p1 may only concede: {own:?}");
            // p2 answers every choice; then the autopilot finishes p1's turn.
            while s.state().pending.as_ref().is_some_and(|open| open.player_id == P2) {
                let first = s.state().pending.as_ref().expect("open").options[0].key.clone();
                s.answer(json!(first));
            }
            assert_eq!(s.state().active, P2);
            assert_eq!(s.state().players.p1.autopilot, None);
        }

        #[test]
        fn r1183003_the_same_seed_plays_the_same_turn() {
            let run = || {
                let mut s = on_top("offload-same", false, json!([TIMMY, PICKLE]), json!([VANILLA]));
                s.end_turn();
                while let Some(open) = s.state().pending.clone() {
                    s.answer(json!(open.options[0].key.clone()));
                }
                hash_state(s.state())
            };
            assert_eq!(run(), run());
        }

        #[test]
        fn r1183002_drawn_on_the_opponents_turn_it_gains_and_draws_only() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [TIMMY], "library": [ID, VOID, VOID, VOID] },
                "p2": { "hand": [TIMMY], "library": [FILLER, FILLER, FILLER] },
            }));
            let mana_before = s.state().players.p1.mana.current;
            let mut events = Vec::new();
            {
                let state = s.state_mut();
                let (seed, cursor) = (state.seed.clone(), state.rng_cursor);
                let mut rng = Rng::new(&seed, cursor);
                let mut sink = EngineSink::new(state, &mut events, &mut rng);
                {
                    let mut ctx = make_context(
                        &mut sink,
                        None,
                        HookOptions {
                            controller: Some(P2),
                            ..HookOptions::default()
                        },
                    );
                    // p2's effect makes p1 draw: the Cognitive Offloading on top is cast.
                    let draw = jackioh_engine::effects::draw(json_as(json!({ "count": 1, "player": "enemy" })));
                    (draw.apply)(&mut ctx);
                }
                settle(&mut sink, SettleOptions::default());
            }
            let side = &s.state().players.p1;
            assert_eq!(side.mana.current, mana_before + 1, "it gained its mana");
            assert_eq!(
                def_ids(&side.hand),
                vec![TIMMY.to_string(), VOID.to_string(), VOID.to_string()],
                "it drew, and the draw repeated"
            );
            assert_eq!(side.autopilot, None, "nothing plays p1's turn");
            assert!(
                !events
                    .iter()
                    .any(|event| matches!(event, GameEvent::AutopilotEngaged { .. }))
            );
            assert_eq!(s.state().active, P2);
        }

        /// Twenty legal Core cards with no other cast-on-draw card among them, Cognitive Offloading
        /// first.
        const DECK: [&str; 20] = [
            ID, "core-002", "core-005", "core-006", "core-008", "core-011", "core-012", "core-013", "core-015",
            "core-016", "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043",
            "core-044", "core-053", "core-055",
        ];

        #[test]
        fn r635_never_dealt_at_setup_and_a_game_it_plays_replays_from_its_log() {
            crate::register_all();
            let _preview = preview_sets(&[SetName::MeditativePlus]);
            let deck: Vec<String> = DECK.iter().map(|id| id.to_string()).collect();
            // A seed whose shuffle-in puts it on top of p1's library, so turn 1's draw casts it.
            let mut found: Option<(String, GameState, Vec<Action>)> = None;
            for at in 0..300 {
                let seed = format!("offload-deal-{at}");
                let created = create_game(&CreateGameOptions {
                    seed: seed.clone(),
                    decks: (deck.clone(), deck.clone()),
                    ..CreateGameOptions::default()
                });
                let mut state = begin_game(&created).state;
                // R635: setup deals none, so the mulligans open over hands that hold none.
                for player in [P1, P2] {
                    assert!(!def_ids(&state.players[player].hand).contains(&ID.to_string()));
                }
                let mut log: Vec<Action> = Vec::new();
                for (n, player) in [P1, P2].into_iter().enumerate() {
                    let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
                    let action = Action::new(ActionBody::Mulligan { keep }, player, format!("offload-{n}"));
                    let result = reduce(&state, &action);
                    assert!(result.error.is_none(), "{:?}", result.error);
                    state = result.state;
                    log.push(action);
                }
                if def_ids(&state.players.p1.graveyard).contains(&ID.to_string()) {
                    found = Some((seed, state, log));
                    break;
                }
            }
            let Some((seed, state, log)) = found else {
                panic!("no seed in offload-deal-0..299 puts it on top of p1's library");
            };
            // Turn 1 was the autopilot's, played and ended inside the second mulligan's action.
            assert!(state.turn >= 2, "p1's turn 1 is over");
            let replayed = fold(&json_as::<FoldArgs>(json!({
                "seed": seed,
                "decks": [deck, deck],
                "log": log,
            })));
            assert!(replayed.errors.is_empty(), "{:?}", replayed.errors);
            assert_eq!(hash_state(&replayed.state), hash_state(&state));
        }

        #[test]
        fn r386_mana_and_draw_read_through_param() {
            let mut s = on_top("offload-param", false, json!([]), json!([]));
            let top = s.state().players.p1.library[0].id.clone();
            step_param(s.card_mut(top.as_str()), "mana", 1);
            step_param(s.card_mut(top.as_str()), "draw", 1);
            s.end_turn();
            let side = &s.state().players.p1;
            assert_eq!(side.turn_log.unspent_at_end, Some(side.mana.max + 2));
            assert_eq!(side.hand.len(), 3, "two drawn and the turn's repeated draw");
        }

        #[test]
        fn the_radiant_face_gains_3_mana_and_draws_2() {
            let mut s = on_top("offload-radiant", true, json!([]), json!([]));
            s.end_turn();
            let side = &s.state().players.p1;
            assert_eq!(side.turn_log.unspent_at_end, Some(side.mana.max + 3));
            assert_eq!(def_ids(&side.hand), vec![VOID.to_string(); 3]);
            assert!(engaged(&s, P1));
        }
    }
}
