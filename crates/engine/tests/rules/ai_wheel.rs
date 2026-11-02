//! Stinkyscape's "the AI at the wheel" (issue #1844, part of #1836): the autopilot aura (R1844000),
//! the Pareto hint (R1844001) and the helpful cast (R1844002), each through `fixtures::ai_wheel`. No
//! card reaches them yet: Stinkyscape #46, #47, #147 and #147.2 come with the set.

use jackioh_engine::effects::{HelpfulCastArgs, autopilot, helpful_cast, pareto_hint};
use jackioh_engine::subsystems::autopilot::{AuraReach, aura_reach, due};
use jackioh_engine::subsystems::pareto::{judge_play, pareto_hint_ids};
use jackioh_engine::subsystems::perfect_hand::{RankPerfectHandOptions, rank_perfect_hand};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::ai_wheel::{
    ASK, ASK_DAMAGE, ASK_ENEMY, CHARGER, DEAR, FULLCYCLER, HALF_ACTIONS, HALFCYCLER, HALFCYCLER_MORE, HELPER,
    HELPER_FIELD, HINT_FIELD, HINT_UNIT, MORE_ACTIONS, register_ai_wheel,
};
use crate::rules::fixtures::harness::in_hand;

const FILLER: &str = "fx-10";

fn filler(count: usize) -> Value {
    json!(vec![FILLER; count])
}

/// `effects` applied for p1 as a card's list is, then settled (`effects_cast.rs`'s `run`, no running card).
fn run(state: &mut GameState, effects: Vec<Effect>) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(P1),
                ..Default::default()
            },
        );
        apply_effects(&effects, &mut ctx);
        settle(&mut ctx.sink, Default::default());
    }
    state.rng_cursor = rng.cursor();
    events
}

fn plays_by(events: &[GameEvent], player: PlayerId) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, GameEvent::CardPlayed { player: by, .. } if *by == player))
        .count()
}

fn engaged(events: &[GameEvent], player: PlayerId) -> bool {
    events
        .iter()
        .any(|event| matches!(event, GameEvent::AutopilotEngaged { player: by, .. } if *by == player))
}

mod r1844000_the_autopilot_aura {
    use super::*;

    /// p1 to act on turn 9 with `aura` in the backrow; p2 with a card to play, so their turn won't end.
    fn piloted(seed: &str, aura: &str, hand: Value, library: Value) -> Scenario {
        register_ai_wheel();
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "backrow": [aura], "library": library },
            "p2": { "hand": ["fx-8"], "library": filler(6) },
        }))
    }

    #[test]
    fn r1844000_a_count_plays_the_first_actions_and_hands_the_turn_back() {
        let mut s = piloted(
            "aw-half",
            &HALFCYCLER.id,
            json!(["fx-1", "fx-2", "fx-3"]),
            filler(6),
        );
        s.end_turn().end_turn();
        let events = s.last_events();
        assert!(engaged(events, P1), "p1's turn began piloted");
        assert_eq!(plays_by(events, P1), HALF_ACTIONS as usize, "the count's plays");
        assert_eq!(s.state().active, P1, "then the turn is p1's again");
        assert_eq!(s.state().players[P1].autopilot, None, "handed back for good");
        assert!(
            legal_actions(s.state(), P1)
                .iter()
                .any(|action| action.action_type() == ActionType::Play),
            "with plays left to make"
        );
    }

    #[test]
    fn r1844000_its_last_actions_prompt_is_answered_then_the_players_own_are_theirs() {
        let mut s = piloted(
            "aw-ask",
            &HALFCYCLER.id,
            json!([ASK.id, ASK.id, ASK.id]),
            json!(vec![ASK.id.clone(); 6]),
        );
        s.end_turn().end_turn();
        assert_eq!(plays_by(s.last_events(), P1), HALF_ACTIONS as usize);
        // Each counted play asked p1 for a target, and the autopilot answered: the enemy hero, twice.
        s.expect_health("p2", HERO_HEALTH - HALF_ACTIONS * ASK_DAMAGE);
        assert!(s.state().pending.is_none(), "the last play's prompt was answered");
        assert_eq!(s.state().players[P1].autopilot, None);
        // A prompt p1's own play opens is p1's to answer.
        s.play(ASK.id.as_str(), json!({}));
        let pending = s.state().pending.clone().expect("p1's target prompt stays open");
        assert_eq!(pending.player_id, P1);
    }

    #[test]
    fn r1844000_a_question_its_last_action_puts_to_the_opponent_hands_the_turn_back_once_answered() {
        let mut s = piloted(
            "aw-ask-enemy",
            &HALFCYCLER.id,
            json!([ASK_ENEMY.id, ASK_ENEMY.id]),
            json!(vec![ASK_ENEMY.id.clone(); 6]),
        );
        s.end_turn().end_turn();
        for asked in 1..=HALF_ACTIONS {
            let pending = s.state().pending.clone().expect("p2's question is open");
            assert_eq!(pending.player_id, P2, "question {asked}");
            assert!(
                s.state().players[P1].autopilot.is_some(),
                "the turn waits, still piloted"
            );
            s.answer(json!("left"));
        }
        assert_eq!(
            plays_by(s.events(), P1),
            HALF_ACTIONS as usize,
            "the count, and no more"
        );
        assert_eq!(s.state().active, P1);
        assert!(s.state().pending.is_none());
        assert_eq!(
            s.state().players[P1].autopilot,
            None,
            "p2's answer to the last counted action handed the turn back"
        );
        in_hand(s.state_mut(), &ASK.id, P1, 1);
        s.play(ASK.id.as_str(), json!({}));
        let pending = s.state().pending.clone().expect("p1's target prompt stays open");
        assert_eq!(pending.player_id, P1, "p1's own prompt is p1's");
    }

    #[test]
    fn r1844000_a_whole_turn_aura_plays_the_turn_through() {
        let mut s = piloted(
            "aw-full",
            &FULLCYCLER.id,
            json!(["fx-1", "fx-2", "fx-3"]),
            filler(6),
        );
        s.end_turn().end_turn();
        let events = s.last_events();
        assert!(engaged(events, P1));
        assert!(
            plays_by(events, P1) >= MORE_ACTIONS as usize,
            "past any count: every card it could play"
        );
        assert_eq!(s.state().active, P2, "and the turn ended");
        assert_eq!(s.state().players[P1].autopilot, None);
    }

    #[test]
    fn r1844000_an_aura_played_mid_turn_waits_for_the_next_turn() {
        register_ai_wheel();
        let mut s = scenario(json!({
            "seed": "aw-mid",
            "p1": { "hand": [HALFCYCLER.id, "fx-1", "fx-2"], "library": filler(6) },
            "p2": { "hand": ["fx-8"], "library": filler(6) },
        }));
        s.play(HALFCYCLER.id.as_str(), json!({ "zone": 1 }));
        assert!(!engaged(s.last_events(), P1), "nothing engages mid-turn");
        assert_eq!(s.state().players[P1].autopilot, None);
        assert_eq!(due(s.state()), None);
        assert_eq!(plays_by(s.events(), P1), 1, "p1's own play alone");
        s.end_turn().end_turn();
        assert!(engaged(s.last_events(), P1), "its next turn is piloted");
    }

    #[test]
    fn r1844000_the_aura_is_read_live() {
        let mut s = piloted("aw-live", &HALFCYCLER.id, json!(["fx-1"]), filler(6));
        let turn = s.state().turn;
        let status = |aura_actions| {
            Some(Autopilot {
                turn,
                actions: 0,
                aura_actions,
            })
        };
        s.state_mut().players[P1].autopilot = status(Some(0));
        assert_eq!(due(s.state()), Some(P1), "a count with actions left");
        s.state_mut().players[P1].autopilot = status(Some(HALF_ACTIONS));
        assert_eq!(due(s.state()), None, "a spent count");

        register_ai_wheel();
        let mut bare = scenario(json!({
            "p1": { "hand": ["fx-1"], "library": filler(6) },
            "p2": { "hand": ["fx-8"], "library": filler(6) },
        }));
        bare.state_mut().players[P1].autopilot = status(Some(0));
        assert_eq!(due(bare.state()), None, "an aura's turn with no aura left");
        bare.state_mut().players[P1].autopilot = status(None);
        assert_eq!(due(bare.state()), Some(P1), "Cognitive Offloading's needs none");
    }

    #[test]
    fn r1844000_the_largest_reach_holds() {
        register_ai_wheel();
        let counts = scenario(json!({
            "p1": { "backrow": [HALFCYCLER.id, HALFCYCLER_MORE.id], "library": filler(6) },
            "p2": { "library": filler(6) },
        }));
        assert_eq!(
            aura_reach(counts.state(), P1),
            Some(AuraReach::Actions(MORE_ACTIONS)),
            "the largest count, never their sum"
        );
        assert_eq!(aura_reach(counts.state(), P2), None, "the other side has none");
        let whole = scenario(json!({
            "p1": { "backrow": [HALFCYCLER_MORE.id, FULLCYCLER.id], "library": filler(6) },
            "p2": { "library": filler(6) },
        }));
        assert_eq!(
            aura_reach(whole.state(), P1),
            Some(AuraReach::Turn),
            "a whole turn beats any count"
        );
    }

    #[test]
    fn r1844000_offloading_takes_an_aura_turn_whole() {
        let mut s = piloted("aw-offload", &HALFCYCLER.id, json!(["fx-1"]), filler(6));
        let turn = s.state().turn;
        s.state_mut().players[P1].autopilot = Some(Autopilot {
            turn,
            actions: 1,
            aura_actions: Some(1),
        });
        let events = run(s.state_mut(), vec![autopilot()]);
        assert!(events.iter().any(|event| matches!(
            event,
            GameEvent::AutopilotEngaged { player: P1, turn: t } if *t == turn
        )));
        assert_eq!(
            s.state().players[P1].autopilot,
            Some(Autopilot {
                turn,
                actions: 1,
                aura_actions: None,
            }),
            "the rest of the turn is whole, its actions still counted toward the bound"
        );
        assert!(run(s.state_mut(), vec![autopilot()]).is_empty(), "once a turn");
    }

    #[test]
    fn r1844000_a_searchs_step_leaves_it_owed() {
        register_ai_wheel();
        let s = scenario(json!({
            "active": "p2",
            "p1": { "hand": ["fx-1", "fx-2"], "backrow": [HALFCYCLER.id], "library": filler(6) },
            "p2": { "hand": ["fx-8"], "library": filler(6) },
        }));
        let end_turn = Action::new(ActionBody::EndTurn, P2, "search:end".to_string());
        let owed = reduce_unpiloted(s.state(), &end_turn);
        assert!(owed.error.is_none());
        assert!(engaged(&owed.events, P1), "engaged all the same");
        assert_eq!(owed.state.active, P1);
        assert_eq!(due(&owed.state), Some(P1), "owed, not played");
        assert_eq!(plays_by(&owed.events, P1), 0);
        let played = reduce(s.state(), &end_turn);
        assert_eq!(
            plays_by(&played.events, P1),
            HALF_ACTIONS as usize,
            "a game's step plays it"
        );
    }

    #[test]
    fn r1844000_absent_from_a_game_without_it() {
        register_ai_wheel();
        let mut s = scenario(json!({
            "p1": { "hand": ["fx-1", "fx-2"], "library": filler(6) },
            "p2": { "hand": ["fx-8"], "library": filler(6) },
        }));
        s.play("fx-1", json!({ "zone": 1 })).end_turn();
        let text = serde_json::to_string(s.state()).expect("serializes");
        assert!(!text.contains("auraActions"), "no key, no hash change");
        assert!(!text.contains("paretoHint"), "no key, no hash change");
        for viewer in [P1, P2] {
            let view = serde_json::to_string(&s.view(viewer)).expect("serializes");
            assert!(!view.contains("paretoOptimal"), "{viewer:?}");
        }
    }
}

mod r1844001_the_pareto_hint {
    use super::*;

    /// p1 to act on turn 9 with `hand` and `backrow`; p2 with `theirs` in hand.
    fn hinting(seed: &str, hand: Value, backrow: Value, theirs: Value) -> Scenario {
        register_ai_wheel();
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "backrow": backrow, "library": filler(6) },
            "p2": { "hand": theirs, "library": filler(6) },
        }))
    }

    fn hand_id(s: &Scenario, def_id: &str) -> String {
        s.hand(P1)
            .into_iter()
            .find(|card| card.def_id == def_id)
            .map(|card| card.id)
            .expect("in p1's hand")
    }

    /// The hand cards p1's view flags, in hand order.
    fn flagged(s: &Scenario) -> Vec<String> {
        let HandView::Cards(hand) = s.view(P1).you.hand else {
            panic!("p1's own hand is full cards");
        };
        hand.into_iter()
            .filter(|card| card.pareto_optimal == Some(true))
            .map(|card| card.instance_id)
            .collect()
    }

    #[test]
    fn r1844001_the_cards_the_judge_calls_optimal() {
        let mut s = hinting("aw-hint", json!(["fx-1", DEAR.id]), json!([]), json!(["fx-8"]));
        assert!(flagged(&s).is_empty(), "no hint until it is shown");
        run(s.state_mut(), vec![pareto_hint()]);
        let (best, dear) = (hand_id(&s, "fx-1"), hand_id(&s, &DEAR.id));
        assert_eq!(pareto_hint_ids(s.state(), P1), vec![best.clone()]);
        assert!(judge_play(s.state(), P1, &best), "the judge agrees");
        assert!(!judge_play(s.state(), P1, &dear));
        assert_eq!(flagged(&s), vec![best], "the view marks it alone");
    }

    #[test]
    fn r1844001_ties_share_it() {
        let mut s = hinting("aw-tie", json!(["fx-1", "fx-2"]), json!([]), json!(["fx-8"]));
        run(s.state_mut(), vec![pareto_hint()]);
        assert_eq!(flagged(&s), vec![hand_id(&s, "fx-1"), hand_id(&s, "fx-2")]);
    }

    #[test]
    fn r1844001_the_viewers_alone() {
        let mut s = hinting("aw-alone", json!(["fx-1", DEAR.id]), json!([]), json!(["fx-8"]));
        run(s.state_mut(), vec![pareto_hint()]);
        let mine = serde_json::to_string(&s.view(P1)).expect("serializes");
        let theirs = serde_json::to_string(&s.view(P2)).expect("serializes");
        assert!(mine.contains("\"paretoOptimal\":true"));
        assert!(
            !theirs.contains("paretoOptimal"),
            "never on the other seat's view"
        );
    }

    #[test]
    fn r1844001_the_verb_on_its_own_turn_only_until_cleanup() {
        register_ai_wheel();
        let mut theirs = scenario(json!({
            "active": "p2",
            "p1": { "hand": ["fx-1"], "library": filler(6) },
            "p2": { "hand": ["fx-8"], "library": filler(6) },
        }));
        assert!(run(theirs.state_mut(), vec![pareto_hint()]).is_empty());
        assert_eq!(
            theirs.state().players[P1].pareto_hint,
            None,
            "on p2's turn it does nothing"
        );

        let mut s = hinting(
            "aw-verb",
            json!([HINT_UNIT.id, "fx-1"]),
            json!([]),
            json!(["fx-8"]),
        );
        let turn = s.state().turn;
        s.play(HINT_UNIT.id.as_str(), json!({ "zone": 1 }));
        assert_eq!(s.state().players[P1].pareto_hint, Some(turn));
        assert_eq!(flagged(&s), vec![hand_id(&s, "fx-1")]);
        s.end_turn();
        assert_eq!(s.state().players[P1].pareto_hint, None, "cleared at cleanup");
        s.end_turn();
        assert!(flagged(&s).is_empty(), "the next turn has none");
    }

    #[test]
    fn r1844001_the_aura_hints_each_of_its_turns() {
        let mut s = hinting(
            "aw-aura",
            json!(["fx-1", "fx-2"]),
            json!([HINT_FIELD.id]),
            json!(["fx-8", "fx-9"]),
        );
        assert!(!pareto_hint_ids(s.state(), P1).is_empty(), "on p1's turn");
        s.end_turn();
        assert!(pareto_hint_ids(s.state(), P1).is_empty(), "never on p2's");
        assert!(
            pareto_hint_ids(s.state(), P2).is_empty(),
            "nor for p2, who has no aura"
        );
        s.end_turn();
        assert!(!flagged(&s).is_empty(), "and on p1's next");
        assert_eq!(s.state().players[P1].pareto_hint, None, "an aura stores nothing");
    }

    #[test]
    fn r1844001_hidden_cards_never_move_it() {
        let hinted = |theirs: Value| {
            let mut s = hinting("aw-hidden", json!(["fx-1", DEAR.id, "fx-3"]), json!([]), theirs);
            run(s.state_mut(), vec![pareto_hint()]);
            pareto_hint_ids(s.state(), P1)
        };
        let plain = hinted(json!(["fx-8", "fx-9"]));
        assert!(!plain.is_empty());
        assert_eq!(hinted(json!([CHARGER.id, ASK.id])), plain);
    }
}

mod r1844002_the_helpful_cast {
    use super::*;

    /// p2's health: `CHARGER`'s 4 Attack takes it, so the Charge card is the pool's lethal pick.
    const ENEMY_HEALTH: i32 = 4;

    /// p1 to act on turn 9 as `p1` sets up; p2 at `ENEMY_HEALTH` with a card to play.
    fn helping(seed: &str, p1: Value) -> Scenario {
        register_ai_wheel();
        scenario(json!({
            "seed": seed,
            "p1": p1,
            "p2": { "hand": ["fx-8"], "library": filler(6), "health": ENEMY_HEALTH },
        }))
    }

    fn chargers(s: &Scenario) -> Vec<CardInstance> {
        (1..=5)
            .filter_map(|lane| s.unit(P1, lane))
            .filter(|unit| unit.def_id == CHARGER.id)
            .collect()
    }

    #[test]
    fn r1844002_casts_the_top_pick_free() {
        let mut s = helping("aw-help", json!({ "hand": ["fx-1"], "library": filler(6) }));
        let ranked = rank_perfect_hand(s.state(), P1, RankPerfectHandOptions::default());
        assert_eq!(ranked[0].def.id, CHARGER.id, "the pool's lethal pick");
        let mana = s.state().players[P1].mana.current;
        let events = run(s.state_mut(), vec![helpful_cast(HelpfulCastArgs::default())]);
        assert!(events.iter().any(|event| matches!(
            event,
            GameEvent::CardPlayed { player: P1, def_id, cost_paid: 0, .. } if *def_id == CHARGER.id
        )));
        assert_eq!(s.state().players[P1].mana.current, mana, "free");
        let cast = chargers(&s);
        assert_eq!(cast.len(), 1);
        assert!(!cast[0].radiant);
    }

    #[test]
    fn r1844002_the_radiant_face_casts_radiant() {
        let mut s = helping("aw-help-r", json!({ "hand": ["fx-1"], "library": filler(6) }));
        let ranked = rank_perfect_hand(
            s.state(),
            P1,
            RankPerfectHandOptions {
                radiant: Some(true),
                self_def_id: None,
            },
        );
        assert_eq!(ranked[0].def.id, CHARGER.id);
        run(
            s.state_mut(),
            vec![helpful_cast(HelpfulCastArgs { radiant: Some(true) })],
        );
        let cast = chargers(&s);
        assert_eq!(cast.len(), 1);
        assert!(cast[0].radiant, "cast Radiant");
    }

    #[test]
    fn r1844002_played_from_hand() {
        let mut s = helping(
            "aw-help-hand",
            json!({ "hand": [HELPER.id, "fx-1"], "library": filler(6) }),
        );
        s.play(HELPER.id.as_str(), json!({}));
        assert_eq!(chargers(&s).len(), 1);
        assert!(
            s.state().players[P1]
                .graveyard
                .iter()
                .any(|card| card.def_id == HELPER.id)
        );
    }

    #[test]
    fn r1844002_at_the_start_of_its_controllers_turn() {
        let mut s = helping(
            "aw-help-start",
            json!({ "hand": ["fx-1"], "backrow": [HELPER_FIELD.id], "library": filler(6) }),
        );
        s.end_turn();
        assert!(chargers(&s).is_empty(), "not on the opponent's turn");
        s.end_turn();
        assert_eq!(s.state().active, P1);
        assert_eq!(chargers(&s).len(), 1, "cast as p1's turn began");
        let mana = s.state().players[P1].mana;
        assert_eq!(mana.current, mana.max, "at no cost");
    }

    #[test]
    fn r1844002_the_same_state_casts_the_same_card() {
        let cast = || {
            let mut s = helping("aw-help-same", json!({ "hand": ["fx-1"], "library": filler(6) }));
            let events = run(s.state_mut(), vec![helpful_cast(HelpfulCastArgs::default())]);
            (hash_state(s.state()), events)
        };
        assert_eq!(cast(), cast());
    }
}
