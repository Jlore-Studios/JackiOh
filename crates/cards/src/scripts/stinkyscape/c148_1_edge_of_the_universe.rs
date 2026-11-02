//! S #148.1 Edge of the Universe (SPEC §7 row 148.1, §8.10 row S 148.1; §6.3 Escape the Universe, R850,
//! R1843002–R1843004): (0) Spell, Wincon, a token (printed Mythic) of the Stinkyscape set,
//! S #148 Alteran Reach's at the edge of its starscape map (R1847001).
//!   Base:    "Escape the Universe: you win the game, and it is a draw for your opponent."
//!   Radiant: "Escape the Universe: you win the game, and you choose whether it is a draw or a loss for
//!            your opponent."
//!
//! Engine: Escape the Universe is `flee` (R1843002): its caster's `fled`, taking hold at the state check
//! that closes its list, a hero at 0 losing first (R850). The game ends `fled`, a win for the caster and
//! a draw for the other player on the ladder, the record and every seat's result (R1843004). Radiant: a
//! `mode` prompt as it resolves, a draw or a loss for the opponent, the loss ending it `fled-decisive`

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-148-1";

/// R1843003: the Radiant face's two answers, what the flight leaves the other player. The client
/// already translates both words (zh-CN.json), and the prompt keeps its default "Choose one".
const DRAW: &str = "Draw";
const LOSS: &str = "Loss";
/// The step the answer resumes at (§10.6).
const STEP_FLEE: &str = "flee";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| vec![flee(FleeArgs::default())])),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![choose_mode(json_as(
                json!({ "options": [DRAW, LOSS], "step": STEP_FLEE }),
            ))]
        })),
        resume: IndexMap::from([(
            STEP_FLEE,
            hook(|ctx| {
                let opponent = match chosen_options(ctx).first().map(String::as_str) {
                    Some(LOSS) => FleeOpponent::Loss,
                    _ => FleeOpponent::Draw,
                };
                vec![flee(FleeArgs {
                    player: None,
                    opponent: Some(opponent),
                })]
            }),
        )]),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// S #148.1 Edge of the Universe — SPEC §7 row 148.1, BUILD M12 row S 148.1.
#[cfg(test)]
mod tests {
    use super::{DRAW, ID, LOSS, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008";
    const FILLER: &str = "core-005";

    /// The health loss that kills a hero outright, whatever it stands at (losing health is not damage,
    /// R18).
    const LETHAL: i32 = 100 * HERO_HEALTH;

    fn cast(seed: &str, radiant: bool) -> Scenario {
        let mut s = crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": crate::filler(4) },
            "p2": { "field": [VANILLA], "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.play(ID, json!({}));
        s
    }

    fn result_of(s: &Scenario) -> (Winner, GameOverReason) {
        let result = s.state().result.expect("the game is over");
        (result.winner, result.reason)
    }

    fn flights(events: &[GameEvent]) -> Vec<(PlayerId, FleeOpponent)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Fled { player, opponent } => Some((*player, *opponent)),
                _ => None,
            })
            .collect()
    }

    fn tail(events: &[GameEvent]) -> Vec<GameEventType> {
        events.iter().rev().take(2).map(GameEvent::event_type).collect()
    }

    fn answers(state: &GameState, player: PlayerId) -> Vec<ActionBody> {
        legal_actions(state, player)
            .into_iter()
            .filter(|body| matches!(body, ActionBody::Answer { .. }))
            .collect()
    }

    fn lethal_then_escape(s: &mut Scenario) -> Vec<GameEvent> {
        let edge = s.card(ID).clone();
        let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            {
                let mut ctx = make_context(
                    &mut sink,
                    Some(&edge),
                    HookOptions {
                        controller: Some(P1),
                        ..HookOptions::default()
                    },
                );
                let mut list = vec![jackioh_engine::effects::lose_health(json_as(
                    json!({ "player": "self", "amount": LETHAL }),
                ))];
                list.extend((script().base.cry.expect("the cry"))(&mut ctx));
                apply_effects(&list, &mut ctx);
            }
            settle(&mut sink, SettleOptions::default());
        }
        s.state_mut().rng_cursor = rng.cursor();
        events
    }

    mod s148_1_edge_of_the_universe {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r1843002_casting_it_wins_and_leaves_the_opponent_a_draw() {
                let s = cast("s148.1-flee", false);
                assert!(s.state().pending.is_none(), "the base face asks nothing");
                assert_eq!(result_of(&s), (Winner::P1, GameOverReason::Fled));
                let events = s.last_events().to_vec();
                assert_eq!(flights(&events), vec![(P1, FleeOpponent::Draw)]);
                assert_eq!(tail(&events), vec![GameEventType::GameOver, GameEventType::Fled]);
                let result = s.state().result.expect("over");
                assert_eq!(outcome_for(&result, P1), SeatOutcome::Win);
                assert_eq!(outcome_for(&result, P2), SeatOutcome::Draw);
                for viewer in [P1, P2] {
                    let view = s.view(viewer);
                    assert_eq!(view.result.map(|shown| shown.reason), Some(GameOverReason::Fled));
                    assert_eq!(flights(&view.events), vec![(P1, FleeOpponent::Draw)], "{viewer} reads the escape");
                }
            }

            #[test]
            fn r850_r1843002_a_hero_already_at_0_in_the_same_list_loses_first() {
                let mut s = crate::scenario(json!({
                    "seed": "s148.1-doomed",
                    "p1": { "hand": [ID], "library": crate::filler(4) },
                    "p2": { "field": [VANILLA], "library": crate::filler(4) },
                }));
                let events = lethal_then_escape(&mut s);
                assert_eq!(result_of(&s), (Winner::P2, GameOverReason::HeroDeath));
                assert_eq!(s.state().players.p1.fled, Some(FleeOpponent::Draw), "the flight was held");
                assert!(flights(&events).is_empty(), "and never ended the game");
            }

            #[test]
            fn declares_no_numbers() {
                assert!(crate::card_def(ID).params.is_none());
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r1843003_the_caster_is_asked_and_only_they_answer() {
                let s = cast("s148.1-asked", true);
                let state = s.state();
                assert!(state.result.is_none(), "the flight waits for the choice");
                let pending = state.pending.as_ref().expect("a prompt is open");
                assert_eq!((pending.player_id, pending.kind), (P1, PromptKind::Mode));
                let keys: Vec<&str> = pending.options.iter().map(|option| option.key.as_str()).collect();
                assert_eq!(keys, vec!["mode:Draw", "mode:Loss"]);
                let labels: Vec<&str> = pending.options.iter().map(|option| option.label.as_str()).collect();
                assert_eq!(labels, vec![DRAW, LOSS]);
                let offered = answers(state, P1);
                assert_eq!(offered.len(), 2);
                assert!(answers(state, P2).is_empty(), "the other seat answers nothing");
                let refused = reduce(state, &Action::new(offered[0].clone(), P2, "p2-answers".to_string()));
                assert!(refused.error.is_some(), "the other seat may not answer the caster's question");
                assert!(
                    matches!(s.view(P2).pending, Some(PendingView::Elsewhere(_))),
                    "the other seat sees only that a prompt is open"
                );
                assert!(matches!(s.view(P1).pending, Some(PendingView::ForYou(_))));
                // Paused on the question, the state survives JSON (§9.3).
                let back: GameState = json_as(crate::js(state));
                assert_eq!(&back, state);
            }

            #[test]
            fn r1843003_a_loss_ends_it_fled_decisive_and_a_draw_fled() {
                let mut s = cast("s148.1-loss", true);
                s.answer(json!("mode:Loss"));
                assert_eq!(result_of(&s), (Winner::P1, GameOverReason::FledDecisive));
                assert_eq!(flights(s.last_events()), vec![(P1, FleeOpponent::Loss)]);
                let result = s.state().result.expect("over");
                assert_eq!(outcome_for(&result, P1), SeatOutcome::Win);
                assert_eq!(outcome_for(&result, P2), SeatOutcome::Loss);

                let mut s = cast("s148.1-draw", true);
                s.answer(json!("mode:Draw"));
                assert_eq!(result_of(&s), (Winner::P1, GameOverReason::Fled));
                assert_eq!(flights(s.last_events()), vec![(P1, FleeOpponent::Draw)]);
                let result = s.state().result.expect("over");
                assert_eq!(outcome_for(&result, P2), SeatOutcome::Draw);
            }

            #[test]
            fn r1843003_r79_a_timeout_answers_the_choice() {
                let mut s = cast("s148.1-timeout", true);
                s.act(ActionBody::Timeout, P1, "the caster's clock runs out");
                let (winner, reason) = result_of(&s);
                assert_eq!(winner, Winner::P1);
                assert!(
                    matches!(reason, GameOverReason::Fled | GameOverReason::FledDecisive),
                    "{reason}"
                );
            }

            #[test]
            fn r1843003_r44_the_random_policy_answers_the_choice() {
                let s = cast("s148.1-policy", true);
                let state = s.state().clone();
                let mut rng = Rng::new("s148.1-policy", 0);
                let body = jackioh_engine::subsystems::ai_policy::choose_action(&state, P1, &mut rng)
                    .expect("the policy answers");
                assert!(matches!(body, ActionBody::Answer { .. }), "{body:?}");
                let next = reduce(&state, &Action::new(body, P1, "policy-answer".to_string()));
                assert!(next.error.is_none(), "{:?}", next.error);
                let result = next.state.result.expect("the answer ends the game");
                assert_eq!(result.winner, Winner::P1);
                assert!(matches!(
                    result.reason,
                    GameOverReason::Fled | GameOverReason::FledDecisive
                ));
            }
        }
    }
}
