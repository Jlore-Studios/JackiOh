//! Patch v0.2.0's prompt kinds (docs/classic-sets.md B5 E17, E18): `number`, `answer`, `cell`,
//! `reward`, `pick`, a `mode` prompt the other player holds, and the other player's hand as a prompt's
//! options. For each one this file proves what the brief asks of a prompt: what `promptAnswers` lists,
//! what `whyAnswerRefused` refuses, what `viewFor` shows the chooser and — R97, R177 — that the other
//! seat learns only that a prompt is open and whose it is; that a timeout answers it with R79's AI
//! policy; that a paused state survives `JSON.parse(JSON.stringify(...))` and answers exactly as the
//! original does; and that a game through it replays from its log (§9.2, §9.3).
//!
//! Port of `packages/engine/test/prompt-kinds.test.ts`.

use serde::Serialize;

use jackioh_engine::effects::{ANSWER_OPTION_IDS, choose_from_hand};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{events_of_type, in_hand, put, set_library, slot};
use crate::rules::fixtures::prompt_harness::{
    act, answer_keys, board, cast_now, expect_replays, hand_card, open_as, replayable, round_trip,
};
use crate::rules::fixtures::prompts::{
    BACK_BUDGET, GLITCH_OPTIONS, PICKLE_OPTIONS, QUEST_REWARDS, QUIZ, acquire, back_from_gy, cross_pick,
    glitch, grunt, mill, mind_melt, numberer, papaya, pickle, prize, quest, quickdraw_of, quiz,
};

fn input(body: Value) -> ActionInput {
    json_as(body)
}

fn json_of(value: impl Serialize) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

/// vitest's `toMatchObject`: every key the expected object names matches, recursively; an array
/// matches element for element and in length.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|found| matches_object(found, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len() && actual.iter().zip(expected).all(|(a, e)| matches_object(a, e))
        }
        _ => actual == expected,
    }
}

/// TS `makeContext(sink, null, { controller })`'s options.
fn by(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

fn mode(option: &str) -> Selection {
    Selection::Mode {
        option: option.to_string(),
    }
}

fn instance(id: &str) -> Selection {
    Selection::Instance {
        instance_id: id.to_string(),
    }
}

fn keys(options: &[PromptOption]) -> Vec<String> {
    options.iter().map(|option| option.key.clone()).collect()
}

fn labels(options: &[PromptOption]) -> Vec<String> {
    options.iter().map(|option| option.label.clone()).collect()
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}

/// The selection an `answer` action carries (TS `answer.selection`).
fn selection_of(action: &ActionBody) -> Vec<Selection> {
    match action {
        ActionBody::Answer { selection, .. } => selection.clone(),
        other => panic!("expected an answer, got {other:?}"),
    }
}

/// `whyAnswerRefused(pending, { playerId, choiceId: pending.id, selection })`.
fn refusal(pending: &PendingChoice, player: PlayerId, selection: Vec<Selection>) -> Option<String> {
    let answer = AnswerInput {
        player_id: player,
        choice_id: pending.id.clone(),
        selection,
    };
    why_answer_refused(pending, &answer).err().map(|why| why.to_string())
}

/// `answerKeys(state, ...keys)` over keys held as owned strings.
fn answer_all(state: &mut GameState, keys: &[String]) {
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    answer_keys(state, &keys);
}

/// R81, R97: the other seat's view of a prompt: that it is open, and whose — the whole of `pending`,
/// so no option, caption, cost or budget travels — and a `promptOpened` event that names the player
/// and the kind and nothing more.
fn expect_only_that_it_is_open(state: &GameState, viewer: PlayerId, holder: PlayerId) {
    let view = view_for(state, viewer);
    assert_eq!(json_of(&view.pending), json!({ "forYou": false, "pendingFor": holder }));
    for opened in events_of_type(&view.events, GameEventType::PromptOpened) {
        let mut names: Vec<String> = json_of(opened)
            .as_object()
            .expect("an event is an object")
            .keys()
            .cloned()
            .collect();
        names.sort();
        assert_eq!(names, ["choiceId", "kind", "player", "type"]);
    }
}

fn for_you(view: Option<PendingView>) -> PendingPromptView {
    match view {
        Some(PendingView::ForYou(prompt)) => prompt,
        _ => panic!("expected the viewer's own prompt"),
    }
}

fn grave_card(state: &mut GameState, player: PlayerId, def_id: &str, cost: Option<i32>) -> CardInstance {
    let mut card = new_instance(state, def_id, player, Zone::Graveyard { player });
    if let Some(cost) = cost {
        card.cost_override = Some(cost);
    }
    state.players[player].graveyard.push(card.clone());
    card
}

fn at0(pending: &PendingChoice) -> Selection {
    pending.options.first().expect("a first option").selection.clone()
}

/// The top card of a unit pile, if there is one.
fn top_id(pile: &Option<Pile>) -> Option<String> {
    pile.as_ref().and_then(|pile| pile.first()).map(|card| card.id.clone())
}

mod e18_a_mode_prompt_the_other_player_holds_classic_8 {
    use super::*;

    #[test]
    fn e18_the_other_player_answers_and_the_step_runs_as_the_asking_cards_controller() {
        let mut state = board("pickle-owner");
        let plain_id = plain.id.clone();
        in_hand(&mut state, &plain_id, P2, 2);
        set_library(&mut state, P1, &[plain_id.as_str(), plain_id.as_str()]);
        set_library(&mut state, P2, &[plain_id.as_str(), plain_id.as_str(), plain_id.as_str()]);
        cast_now(&mut state, &pickle().id, P1, false);

        let first = open_as(&state, PromptKind::Mode, P2);
        assert_eq!(
            keys(&first.options),
            PICKLE_OPTIONS.iter().map(|option| format!("mode:{option}")).collect::<Vec<_>>()
        );
        assert_eq!(
            prompt_answers(&first).iter().map(selection_of).collect::<Vec<_>>(),
            PICKLE_OPTIONS.iter().map(|option| vec![mode(option)]).collect::<Vec<_>>()
        );
        // The card's controller may not answer the other player's prompt.
        assert_eq!(
            refusal(&first, P1, vec![mode("draw")]),
            Some("that prompt belongs to the other player".to_string())
        );
        expect_only_that_it_is_open(&state, P1, P2);
        assert_eq!(
            for_you(view_for(&state, P2).pending).options.iter().map(|option| option.label.clone()).collect::<Vec<_>>(),
            PICKLE_OPTIONS.iter().map(|option| option.to_string()).collect::<Vec<_>>()
        );

        // "You draw" is the asking card's controller's draw, though the other player answered.
        assert_eq!(answer_keys(&mut state, &["mode:draw"]).error, None);
        assert_eq!(state.players.p1.hand.len(), 1);
        assert_eq!(state.players.p2.hand.len(), 2);

        open_as(&state, PromptKind::Mode, P2);
        answer_keys(&mut state, &["mode:exile"]);
        assert_eq!(state.players.p2.library.len(), 2);
        assert_eq!(state.players.p2.exile.len(), 1);

        // "They discard": their own hand pick, held by them, continued as the Pickle's controller.
        open_as(&state, PromptKind::Mode, P2);
        answer_keys(&mut state, &["mode:discard"]);
        let pick = open_as(&state, PromptKind::Hand, P2);
        assert_eq!(
            pick.options.iter().map(|option| option.selection.clone()).collect::<Vec<_>>(),
            state.players.p2.hand.iter().map(|card| instance(&card.id)).collect::<Vec<_>>()
        );
        expect_only_that_it_is_open(&state, P1, P2);
        let kept = state.players.p2.hand[0].clone();
        let gone = state.players.p2.hand[1].clone();
        answer_keys(&mut state, &[format!("instance:{}", gone.id).as_str()]);
        assert_eq!(ids(&state.players.p2.hand), [kept.id.clone()]);
        assert_eq!(ids(&state.players.p2.graveyard), [gone.id.clone()]);
        // Three choices, then no more.
        assert!(state.pending.is_none());
    }

    #[test]
    fn e18_a_paused_other_player_prompt_survives_a_json_round_trip_and_answers_the_same() {
        let mut state = board("pickle-json");
        let plain_id = plain.id.clone();
        in_hand(&mut state, &plain_id, P2, 2);
        set_library(&mut state, P1, &[plain_id.as_str(), plain_id.as_str(), plain_id.as_str()]);
        set_library(&mut state, P2, &[plain_id.as_str(), plain_id.as_str()]);
        cast_now(&mut state, &pickle().id, P1, false);
        let mut copy = round_trip(&state);
        assert_eq!(hash_state(&copy), hash_state(&state));
        for key in ["mode:draw", "mode:discard"] {
            answer_keys(&mut state, &[key]);
            answer_keys(&mut copy, &[key]);
        }
        assert_eq!(hash_state(&copy), hash_state(&state));
        assert_eq!(state.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Hand));
    }

    #[test]
    fn r79_e18_the_other_players_timeout_answers_their_prompt_with_the_ai_policy_and_nothing_more() {
        let mut state = board("pickle-timeout");
        let plain_id = plain.id.clone();
        in_hand(&mut state, &plain_id, P2, 1);
        set_library(&mut state, P1, &[plain_id.as_str(), plain_id.as_str(), plain_id.as_str()]);
        set_library(&mut state, P2, &[plain_id.as_str(), plain_id.as_str()]);
        cast_now(&mut state, &pickle().id, P1, false);
        let before = state.pending.as_ref().expect("Pickle's first choice").id.clone();
        state = act(&state, input(json!({ "type": "timeout", "playerId": "p2" })), None);
        // It answered that one prompt: the next choice is open, still theirs, and the turn is still p1's.
        assert_ne!(state.pending.as_ref().map(|pending| pending.id.clone()), Some(before));
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(P2));
        assert_eq!(state.active, P1);
        // The active player's clock cannot answer the other player's prompt.
        let held = state.pending.as_ref().expect("the second choice").id.clone();
        state = act(&state, input(json!({ "type": "timeout", "playerId": "p1" })), None);
        assert_eq!(state.pending.as_ref().map(|pending| pending.id.clone()), Some(held));
    }

    #[test]
    fn e18_a_pickle_game_replays_from_its_log() {
        let qd = quickdraw_of(&pickle()).id;
        let game = replayable("pickle-replay", &[qd.clone()], &[]);
        let decks = game.decks;
        let mut log = game.log;
        let mut state = game.state;
        let played = hand_card(&state, P1, &qd).id.clone();
        state = act(
            &state,
            input(json!({ "type": "play", "playerId": "p1", "instanceId": played })),
            Some(&mut log),
        );
        for option in ["draw", "exile", "discard"] {
            let pending = open_as(&state, PromptKind::Mode, P2);
            state = act(
                &state,
                input(json!({
                    "type": "answer", "playerId": "p2", "choiceId": pending.id,
                    "selection": [{ "pick": "mode", "option": option }],
                })),
                Some(&mut log),
            );
        }
        let discard = open_as(&state, PromptKind::Hand, P2);
        state = act(
            &state,
            input(json!({ "type": "answer", "playerId": "p2", "choiceId": discard.id, "selection": [at0(&discard)] })),
            Some(&mut log),
        );
        assert!(state.pending.is_none());
        expect_replays("pickle-replay", &decks, &log, &state);
    }
}

mod e18_the_number_kind {
    use super::*;

    #[test]
    fn r81_e18_a_number_declared_with_the_play_travels_in_the_action_from_a_fixed_list() {
        let mut state = board("glitch");
        let card = in_hand(&mut state, &glitch().id, P1, 1).remove(0);
        let plays: Vec<Vec<String>> = legal_actions(&state, P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play { instance_id, modes, .. } if instance_id == card.id => {
                    Some(modes.unwrap_or_default())
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            plays,
            GLITCH_OPTIONS.iter().map(|n| vec![n.to_string()]).collect::<Vec<_>>()
        );
        // TS `act` throws on a refusal: the reducer refuses a number the list does not hold.
        let refused = input(json!({ "type": "play", "playerId": "p1", "instanceId": card.id, "modes": ["11"] }))
            .with_nonce("pk-glitch-11");
        assert!(reduce(&state, &refused).error.is_some());
        let next = act(
            &state,
            input(json!({ "type": "play", "playerId": "p1", "instanceId": card.id, "modes": ["7"] })),
            None,
        );
        assert_eq!(next.players.p2.hero.health, HERO_HEALTH - 7);
    }

    #[test]
    fn e18_a_number_asked_at_resolution_offers_the_range_and_the_answer_is_the_number() {
        let mut state = board("numberer");
        cast_now(&mut state, &numberer().id, P1, false);
        let pending = open_as(&state, PromptKind::Number, P1);
        assert_eq!(keys(&pending.options), ["mode:1", "mode:2", "mode:3"]);
        assert_eq!(prompt_answers(&pending).len(), 3);
        assert_eq!(for_you(view_for(&state, P1).pending).kind, PromptKind::Number);
        expect_only_that_it_is_open(&state, P2, P1);
        let mut copy = round_trip(&state);
        answer_keys(&mut state, &["mode:3"]);
        answer_keys(&mut copy, &["mode:3"]);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 3);
        assert_eq!(hash_state(&copy), hash_state(&state));
    }

    #[test]
    fn r79_e18_a_timeout_answers_a_number_prompt_with_one_of_its_numbers() {
        let mut state = board("numberer-timeout");
        cast_now(&mut state, &numberer().id, P1, false);
        state = act(&state, input(json!({ "type": "timeout", "playerId": "p1" })), None);
        assert!(state.pending.is_none());
        let lost = HERO_HEALTH - state.players.p2.hero.health;
        assert!([1, 2, 3].contains(&lost));
    }
}

/// The first four answer letters (TS `ANSWER_OPTION_IDS.slice(0, 4)`).
fn four_letters() -> Vec<String> {
    ANSWER_OPTION_IDS[..4].iter().map(|id| id.to_string()).collect()
}

mod r465_e18_the_answer_kind_classic_plus_42 {
    use super::*;

    #[test]
    fn r465_an_answer_prompts_key_stays_in_its_resume_data_and_never_reaches_view_for() {
        let mut state = board("quiz-key");
        cast_now(&mut state, &quiz().id, P1, false);
        let pending = open_as(&state, PromptKind::Answer, P1);
        assert_eq!(pending.prompt, QUIZ.statement);
        // Four options under letters, in an order the match rng shuffled; the labels are the answers.
        assert_eq!(
            keys(&pending.options),
            four_letters().iter().map(|id| format!("mode:{id}")).collect::<Vec<_>>()
        );
        assert_eq!(
            sorted(labels(&pending.options)),
            sorted(QUIZ.options.iter().map(|option| option.to_string()).collect())
        );
        let key = answer_key_of(&pending.resume.data).expect("the key").to_string();
        let right = pending
            .options
            .iter()
            .find(|option| option.label == QUIZ.options[QUIZ.correct as usize])
            .expect("the right option");
        assert_eq!(right.key, format!("mode:{key}"));

        // The view is the same whichever option is right: nothing in it tells them apart.
        let mut twin = round_trip(&state);
        let other = four_letters().into_iter().find(|id| *id != key).expect("another letter");
        twin.pending
            .as_mut()
            .expect("the twin's prompt")
            .resume
            .data
            .insert(ANSWER_KEY.to_string(), json!(other));
        for viewer in [P1, P2] {
            assert_eq!(view_for(&twin, viewer), view_for(&state, viewer));
            assert!(!serde_json::to_string(&view_for(&state, viewer)).expect("a view").contains(ANSWER_KEY));
        }
        assert_eq!(
            for_you(view_for(&state, P1).pending).options.iter().map(|option| option.label.clone()).collect::<Vec<_>>(),
            labels(&pending.options)
        );
        expect_only_that_it_is_open(&state, P2, P1);
        assert_eq!(prompt_answers(&pending).len(), 4);
    }

    #[test]
    fn r465_the_right_answer_gains_the_reward_a_wrong_one_nothing_and_the_lists_tail_runs_either_way() {
        for right in [true, false] {
            let mut state = board(&format!("quiz-{right}"));
            cast_now(&mut state, &quiz().id, P1, false);
            let pending = open_as(&state, PromptKind::Answer, P1);
            let key = answer_key_of(&pending.resume.data).expect("the key").to_string();
            let pick = if right {
                key.clone()
            } else {
                four_letters().into_iter().find(|id| *id != key).expect("a wrong letter")
            };
            let mut copy = round_trip(&state);
            answer_keys(&mut state, &[format!("mode:{pick}").as_str()]);
            answer_keys(&mut copy, &[format!("mode:{pick}").as_str()]);
            assert_eq!(state.players.p1.hand.iter().any(|card| card.def_id == prize().id), right);
            // The heal after the question ran after the answer (R113), right or wrong.
            assert_eq!(state.players.p1.hero.health, HERO_HEALTH + 1);
            assert_eq!(hash_state(&copy), hash_state(&state));
        }
    }

    #[test]
    fn r465_r79_a_timeout_answers_the_problem_from_what_it_shows_and_the_key_judges_it_as_anyones() {
        for seed in ["quiz-timeout-1", "quiz-timeout-2", "quiz-timeout-3", "quiz-timeout-4"] {
            let mut state = board(seed);
            cast_now(&mut state, &quiz().id, P1, false);
            let pending = state.pending.clone().expect("the problem");
            let key = answer_key_of(&pending.resume.data).expect("the key").to_string();
            let answers: Vec<ActionBody> = legal_actions(&state, P1)
                .into_iter()
                .filter(|action| matches!(action, ActionBody::Answer { .. }))
                .collect();
            assert_eq!(answers.len(), 4);
            // R79: the policy draws uniformly over the answers, from the match rng at its stored cursor.
            let drawn = &answers[Rng::new(&state.seed, state.rng_cursor).int(answers.len() as i32) as usize];
            let chosen = selection_of(drawn).first().cloned();
            state = act(&state, input(json!({ "type": "timeout", "playerId": "p1" })), None);
            assert!(state.pending.is_none());
            let right = chosen == Some(mode(&key));
            assert_eq!(state.players.p1.hand.iter().any(|card| card.def_id == prize().id), right);
        }
    }

    #[test]
    fn r465_a_kys_test_game_replays_from_its_log() {
        let qd = quickdraw_of(&quiz()).id;
        let game = replayable("quiz-replay", &[qd.clone()], &[]);
        let decks = game.decks;
        let mut log = game.log;
        let mut state = game.state;
        let played = hand_card(&state, P1, &qd).id.clone();
        state = act(
            &state,
            input(json!({ "type": "play", "playerId": "p1", "instanceId": played })),
            Some(&mut log),
        );
        let pending = open_as(&state, PromptKind::Answer, P1);
        let key = answer_key_of(&pending.resume.data).expect("the key").to_string();
        state = act(
            &state,
            input(json!({
                "type": "answer", "playerId": "p1", "choiceId": pending.id,
                "selection": [{ "pick": "mode", "option": key }],
            })),
            Some(&mut log),
        );
        assert!(state.players.p1.hand.iter().any(|card| card.def_id == prize().id));
        expect_replays("quiz-replay", &decks, &log, &state);
    }
}

mod e18_the_cell_kind_classic_plus_62 {
    use super::*;

    #[test]
    fn e18_cells_are_zones_of_both_sides_and_both_rows_a_lane_at_a_time_with_done_after_the_first() {
        let mut state = board("papaya");
        let target = put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));
        let own = put(&mut state, &plain.id, slot(P1, Row::Units, 4), json!({}));
        cast_now(&mut state, &papaya().id, P1, false);
        let first = open_as(&state, PromptKind::Cell, P1);
        assert_eq!(first.options.len(), 20);
        // The chooser's side from the hero outward — backrow, units — then the other side's units, backrow.
        assert_eq!(keys(&first.options[0..2]), ["zone:p1:backrow:1", "zone:p1:backrow:2"]);
        assert_eq!(keys(&first.options[5..7]), ["zone:p1:units:1", "zone:p1:units:2"]);
        assert_eq!(keys(&first.options[10..12]), ["zone:p2:units:1", "zone:p2:units:2"]);
        assert!(!first.options.iter().any(|option| option.selection == Selection::None));
        let cells = for_you(view_for(&state, P1).pending).options;
        assert!(matches_object(
            &json_of(&cells[10]),
            &json!({ "player": "p2", "row": "units", "lane": 1 })
        ));
        expect_only_that_it_is_open(&state, P2, P1);

        answer_keys(&mut state, &["zone:p2:units:2"]);
        let second = open_as(&state, PromptKind::Cell, P1);
        // Lane 2 is used: 16 cells of the four other lanes, and "done".
        assert_eq!(second.options.len(), 17);
        assert!(!second.options.iter().any(|option| option.key.ends_with(":2")));
        assert_eq!(second.options.last().map(|option| option.selection.clone()), Some(Selection::None));
        assert_eq!(prompt_answers(&second).len(), 17);
        let mut copy = round_trip(&state);
        answer_keys(&mut state, &["none"]);
        answer_keys(&mut copy, &["none"]);
        assert_eq!(ids(&state.players.p2.exile), [target.id.clone()]);
        assert_eq!(top_id(&state.players.p1.units[3]), Some(own.id.clone()));
        assert_eq!(hash_state(&copy), hash_state(&state));
    }

    #[test]
    fn e18_four_cells_in_four_lanes_end_the_question_without_a_done() {
        let mut state = board("papaya-four");
        cast_now(&mut state, &papaya().id, P1, false);
        for key in ["zone:p1:backrow:1", "zone:p2:units:2", "zone:p2:backrow:3", "zone:p1:units:4"] {
            open_as(&state, PromptKind::Cell, P1);
            answer_keys(&mut state, &[key]);
        }
        assert!(state.pending.is_none());
    }

    #[test]
    fn r79_e18_a_timeout_answers_a_cell_prompt_with_one_of_its_cells() {
        let mut state = board("papaya-timeout");
        cast_now(&mut state, &papaya().id, P1, false);
        let first = state.pending.as_ref().expect("the first cell").id.clone();
        state = act(&state, input(json!({ "type": "timeout", "playerId": "p1" })), None);
        // The turn clock answers every prompt the answers open in turn, then ends the turn (R79).
        assert!(state.pending.is_none());
        assert_eq!(state.active, P2);
        assert!(first.starts_with('q'));
    }

    #[test]
    fn e18_a_papaya_game_replays_from_its_log() {
        let qd = quickdraw_of(&papaya()).id;
        let game = replayable("papaya-replay", &[qd.clone()], &[]);
        let decks = game.decks;
        let mut log = game.log;
        let mut state = game.state;
        let played = hand_card(&state, P1, &qd).id.clone();
        state = act(
            &state,
            input(json!({ "type": "play", "playerId": "p1", "instanceId": played })),
            Some(&mut log),
        );
        for selection in [
            Selection::Zone {
                player: P2,
                row: Row::Units,
                lane: 3,
            },
            Selection::None,
        ] {
            let pending = open_as(&state, PromptKind::Cell, P1);
            state = act(
                &state,
                input(json!({ "type": "answer", "playerId": "p1", "choiceId": pending.id, "selection": [selection] })),
                Some(&mut log),
            );
        }
        assert!(state.pending.is_none());
        expect_replays("papaya-replay", &decks, &log, &state);
    }
}

mod e18_the_reward_kind_classic_90 {
    use super::*;

    #[test]
    fn r79_e18_a_reward_prompt_is_its_cards_controllers_on_the_other_players_turn_too_with_its_own_clock() {
        let mut state = board("quest");
        let plain_id = plain.id.clone();
        put(&mut state, &quest().id, slot(P1, Row::Backrow, 1), json!({}));
        set_library(&mut state, P2, &[plain_id.as_str(), plain_id.as_str()]);
        state.active = P2;
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            jackioh_engine::draw::draw(&mut sink, P2, 1);
            settle(&mut sink, Default::default());
        }
        state.rng_cursor = rng.cursor();
        let pending = open_as(&state, PromptKind::Reward, P1);
        assert_eq!(
            pending
                .options
                .iter()
                .map(|option| (option.key.clone(), option.label.clone()))
                .collect::<Vec<_>>(),
            QUEST_REWARDS
                .iter()
                .map(|reward| (format!("mode:{}", reward.id), reward.label.to_string()))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            for_you(view_for(&state, P1).pending).options.iter().map(|option| option.label.clone()).collect::<Vec<_>>(),
            QUEST_REWARDS.iter().map(|reward| reward.label.to_string()).collect::<Vec<_>>()
        );
        expect_only_that_it_is_open(&state, P2, P1);
        let mut copy = round_trip(&state);
        answer_keys(&mut copy, &["mode:A"]);
        assert_eq!(copy.players.p1.hero.health, HERO_HEALTH + 6);
        // The non-active player's clock answers it (R79), and the turn stays p2's.
        state = act(&state, input(json!({ "type": "timeout", "playerId": "p1" })), None);
        assert!(state.pending.is_none());
        assert_eq!(state.active, P2);
        let healed = state.players.p1.hero.health == HERO_HEALTH + 6;
        let hit = state.players.p2.hero.health == HERO_HEALTH - 3;
        assert!(healed != hit);
    }

    #[test]
    fn e18_a_quest_game_its_reward_asked_on_the_other_players_turn_replays_from_its_log() {
        let qd = quickdraw_of(&quest()).id;
        let game = replayable("quest-replay", &[qd.clone()], &[]);
        let decks = game.decks;
        let mut log = game.log;
        let dealt = game.state;
        let played = hand_card(&dealt, P1, &qd).id.clone();
        let mut state = act(
            &dealt,
            input(json!({
                "type": "play", "playerId": "p1", "instanceId": played,
                "zone": { "row": "backrow", "lane": 1 },
            })),
            Some(&mut log),
        );
        if state.active == P1 {
            state = act(&state, input(json!({ "type": "endTurn", "playerId": "p1" })), Some(&mut log));
        }
        // p2's start-of-turn draw completed the quest: p1's reward prompt, on p2's turn.
        let pending = open_as(&state, PromptKind::Reward, P1);
        assert_eq!(state.active, P2);
        state = act(
            &state,
            input(json!({
                "type": "answer", "playerId": "p1", "choiceId": pending.id,
                "selection": [{ "pick": "mode", "option": "B" }],
            })),
            Some(&mut log),
        );
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 3);
        expect_replays("quest-replay", &decks, &log, &state);
    }
}

mod e18_the_pick_kind_an_up_to_pile_pick_classic_44 {
    use super::*;

    #[test]
    fn e18_an_up_to_pick_from_a_pile_offers_every_card_with_its_cost_no_discover_limit() {
        let mut state = board("acquire");
        let cards = vec![
            grave_card(&mut state, P1, &plain.id, None),
            grave_card(&mut state, P1, &prize().id, None),
            grave_card(&mut state, P1, &grunt().id, None),
        ];
        cast_now(&mut state, &acquire().id, P1, false);
        let pending = open_as(&state, PromptKind::Pick, P1);
        assert_eq!(
            keys(&pending.options),
            cards.iter().map(|card| format!("instance:{}", card.id)).collect::<Vec<_>>()
        );
        assert_eq!(pending.min, 0);
        assert_eq!(pending.max, 2);
        assert_eq!(pending.budget, None);
        let shown = for_you(view_for(&state, P1).pending);
        assert_eq!(
            shown.options.iter().map(|option| option.cost).collect::<Vec<_>>(),
            vec![Some(1), Some(1), Some(0)]
        );
        assert_eq!(shown.budget, None);
        // Every set of up to two, each once: 1 + 3 + 3.
        let answers = prompt_answers(&pending);
        let distinct: IndexSet<String> = answers
            .iter()
            .map(|answer| serde_json::to_string(&selection_of(answer)).expect("a selection"))
            .collect();
        assert_eq!(distinct.len(), 7);
        assert_eq!(answers.len(), 7);
        expect_only_that_it_is_open(&state, P2, P1);

        let mut copy = round_trip(&state);
        let picks = [format!("instance:{}", cards[0].id), format!("instance:{}", cards[2].id)];
        answer_all(&mut state, &picks);
        answer_all(&mut copy, &picks);
        assert_eq!(ids(&state.players.p1.hand), [cards[0].id.clone(), cards[2].id.clone()]);
        assert_eq!(ids(&state.players.p1.graveyard), [cards[1].id.clone()]);
        assert_eq!(hash_state(&copy), hash_state(&state));
    }

    #[test]
    fn e18_a_radiant_pick_reaches_the_graveyard_and_the_exile_pile_together() {
        let mut state = board("acquire-radiant");
        grave_card(&mut state, P1, &plain.id, None);
        let exiled = new_instance(&mut state, &grunt().id, P1, Zone::Exile { player: P1 });
        state.players.p1.exile.push(exiled.clone());
        cast_now(&mut state, &acquire().id, P1, true);
        let pending = open_as(&state, PromptKind::Pick, P1);
        assert_eq!(pending.options.len(), 2);
        assert_eq!(pending.max, 2);
        answer_all(&mut state, &keys(&pending.options));
        assert!(ids(&state.players.p1.hand).contains(&exiled.id));
    }

    #[test]
    fn e18_a_budgeted_pick_refuses_picks_over_the_budget_and_lists_only_sets_that_fit() {
        let mut state = board("back");
        let costs = [1, 2, 3, 4];
        let units: Vec<CardInstance> =
            costs.iter().map(|cost| grave_card(&mut state, P1, &plain.id, Some(*cost))).collect();
        grave_card(&mut state, P1, &glitch().id, None);
        cast_now(&mut state, &back_from_gy().id, P1, false);
        let pending = open_as(&state, PromptKind::Pick, P1);
        // Units only (the filter), each with its cost; the budget travels in the view.
        assert_eq!(
            pending.options.iter().map(|option| option.cost).collect::<Vec<_>>(),
            costs.iter().map(|cost| Some(*cost)).collect::<Vec<_>>()
        );
        assert_eq!(pending.budget, Some(BACK_BUDGET));
        assert_eq!(for_you(view_for(&state, P1).pending).budget, Some(BACK_BUDGET));
        let over = vec![instance(&units[1].id), instance(&units[3].id)];
        assert!(refusal(&pending, P1, over).unwrap_or_default().contains("budget"));
        let answers = prompt_answers(&pending);
        for answer in &answers {
            let spent: i32 = selection_of(answer)
                .iter()
                .map(|pick| {
                    let option = pending.options.iter().find(|option| {
                        matches!(option.selection, Selection::Instance { .. })
                            && matches!(pick, Selection::Instance { .. })
                            && option.selection == *pick
                    });
                    option.and_then(|option| option.cost).unwrap_or(0)
                })
                .sum();
            assert!(spent <= BACK_BUDGET);
            assert_eq!(refusal(&pending, P1, selection_of(answer)), None);
        }
        // 1+4 and 2+3 are the fullest sets that fit; the empty set is an answer too ("up to").
        assert!(answers.iter().map(|answer| selection_of(answer).len()).any(|length| length == 0));
        let refused = act(&round_trip(&state), input(json!({ "type": "timeout", "playerId": "p1" })), None);
        assert!(refused.pending.is_none());

        answer_all(
            &mut state,
            &[format!("instance:{}", units[0].id), format!("instance:{}", units[3].id)],
        );
        assert_eq!(top_id(&state.players.p1.units[0]), Some(units[0].id.clone()));
        assert_eq!(top_id(&state.players.p1.units[1]), Some(units[3].id.clone()));
        assert_eq!(state.players.p1.graveyard.len(), 3);
    }

    #[test]
    fn e18_a_pick_over_a_long_pile_stays_bounded_and_still_reaches_every_card() {
        let mut state = board("acquire-long");
        let cards: Vec<CardInstance> = (0..30).map(|_| grave_card(&mut state, P1, &plain.id, None)).collect();
        cast_now(&mut state, &acquire().id, P1, true);
        let pending = open_as(&state, PromptKind::Pick, P1);
        assert_eq!(pending.max, 4);
        let answers = prompt_answers(&pending);
        assert_eq!(answers.len(), MAX_PROMPT_ANSWERS);
        let reached: IndexSet<String> = answers
            .iter()
            .flat_map(selection_of)
            .map(|pick| match pick {
                Selection::Instance { instance_id } => instance_id,
                _ => String::new(),
            })
            .collect();
        for card in &cards {
            assert!(reached.contains(&card.id));
        }
        // Each set is listed once, its picks in offered order (R221).
        let distinct: IndexSet<String> = answers
            .iter()
            .map(|answer| serde_json::to_string(&selection_of(answer)).expect("a selection"))
            .collect();
        assert_eq!(distinct.len(), answers.len());
    }

    #[test]
    fn e18_a_back_from_the_gy_game_replays_from_its_log() {
        let back = quickdraw_of(&back_from_gy()).id;
        let mill_card = quickdraw_of(&mill()).id;
        let game = replayable("back-replay", &[mill_card.clone(), back.clone()], &[]);
        let decks = game.decks;
        let mut log = game.log;
        let mut state = game.state;
        // The mill puts three of p1's Units in the graveyard; Back from the GY brings two of them back.
        let milling = hand_card(&state, P1, &mill_card).id.clone();
        state = act(
            &state,
            input(json!({ "type": "play", "playerId": "p1", "instanceId": milling })),
            Some(&mut log),
        );
        assert_eq!(state.players.p1.graveyard.iter().filter(|card| card.def_id != mill_card).count(), 3);
        let returning = hand_card(&state, P1, &back).id.clone();
        state = act(
            &state,
            input(json!({ "type": "play", "playerId": "p1", "instanceId": returning })),
            Some(&mut log),
        );
        let pending = open_as(&state, PromptKind::Pick, P1);
        assert_eq!(pending.budget, Some(BACK_BUDGET));
        let picks: Vec<Selection> = pending.options[..2].iter().map(|option| option.selection.clone()).collect();
        state = act(
            &state,
            input(json!({ "type": "answer", "playerId": "p1", "choiceId": pending.id, "selection": picks })),
            Some(&mut log),
        );
        assert_eq!(state.players.p1.units.iter().filter(|pile| pile.is_some()).count(), 2);
        expect_replays("back-replay", &decks, &log, &state);
    }
}

mod e18_a_pick_across_zones_classic_78s_radiant {
    use super::*;

    #[test]
    fn e18_a_pick_reaches_the_field_the_hand_and_the_deck_and_never_shows_the_decks_order() {
        let mut state = board("cross-pick");
        let on_field = put(&mut state, &plain.id, slot(P1, Row::Units, 3), json!({}));
        let held = in_hand(&mut state, &grunt().id, P1, 1).remove(0);
        let (plain_id, prize_id, glitch_id) = (plain.id.clone(), prize().id, glitch().id);
        let deck = set_library(&mut state, P1, &[plain_id.as_str(), prize_id.as_str(), glitch_id.as_str()]);
        // Library order is the reverse of creation order: the options must not follow it.
        state.players.p1.library = deck.iter().rev().cloned().collect();
        cast_now(&mut state, &cross_pick().id, P1, false);
        let pending = open_as(&state, PromptKind::Pick, P1);
        assert_eq!(
            keys(&pending.options),
            [&on_field, &held, &deck[0], &deck[1]]
                .iter()
                .map(|card| format!("instance:{}", card.id))
                .collect::<Vec<_>>()
        );
        expect_only_that_it_is_open(&state, P2, P1);
        answer_keys(&mut state, &[format!("instance:{}", deck[1].id).as_str()]);
        assert_eq!(ids(&state.players.p1.exile), [deck[1].id.clone()]);
    }
}

mod e17_the_other_players_hand_as_a_prompt_classic_11 {
    use super::*;

    #[test]
    fn e17_a_hand_prompt_over_the_other_players_hand_is_the_choosers_to_answer_and_to_see() {
        let mut state = board("their-hand");
        let hand = in_hand(&mut state, &plain.id, P2, 2);
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let mut ctx = make_context(&mut sink, None, by(P1));
            (choose_from_hand(json_as(json!({ "of": "enemy", "step": "none", "prompt": "Look" }))).apply)(&mut ctx);
        }
        let pending = open_as(&state, PromptKind::Hand, P1);
        assert_eq!(
            keys(&pending.options),
            hand.iter().map(|card| format!("instance:{}", card.id)).collect::<Vec<_>>()
        );
        assert_eq!(
            for_you(view_for(&state, P1).pending).options.iter().map(|option| option.def_id.clone()).collect::<Vec<_>>(),
            vec![Some(plain.id.clone()), Some(plain.id.clone())]
        );
        expect_only_that_it_is_open(&state, P2, P1);
    }

    #[test]
    fn e17_the_chooser_sees_the_other_players_hand_as_the_options_its_holder_sees_a_prompt_and_nothing_else() {
        let mut state = board("mind-melt");
        let mut hand = in_hand(&mut state, &plain.id, P2, 1);
        hand.extend(in_hand(&mut state, &grunt().id, P2, 1));
        hand.extend(in_hand(&mut state, &prize().id, P2, 1));
        hand[2].radiant = true;
        find_instance_mut(&mut state, &hand[2].id).expect("the third card").radiant = true;
        cast_now(&mut state, &mind_melt().id, P1, false);
        let pending = open_as(&state, PromptKind::Pick, P1);
        assert_eq!(pending.min, 1);
        assert_eq!(pending.max, 1);
        assert_eq!(
            keys(&pending.options),
            hand.iter().map(|card| format!("instance:{}", card.id)).collect::<Vec<_>>()
        );
        let shown = for_you(view_for(&state, P1).pending).options;
        assert_eq!(
            shown.iter().map(|option| option.def_id.clone()).collect::<Vec<_>>(),
            hand.iter().map(|card| Some(card.def_id.clone())).collect::<Vec<_>>()
        );
        assert_eq!(shown[2].radiant, Some(true));
        // The hand itself is still a count in the chooser's view: the prompt is the only window.
        assert_eq!(view_for(&state, P1).opponent.hand, HandView::Count { count: 3 });
        expect_only_that_it_is_open(&state, P2, P1);
        assert!(!serde_json::to_string(&view_for(&state, P2).pending).expect("a view").contains(&plain.id));

        let answered = answer_keys(&mut state, &[format!("instance:{}", hand[1].id).as_str()]);
        // The view's events are the actions `reduce` applied (§9.3); this answer went in directly.
        state.applied.push(AppliedAction {
            nonce: "mind-melt-answer".to_string(),
            events: answered.events.clone(),
        });
        assert_eq!(ids(&state.players.p2.exile), [hand[1].id.clone()]);
        // Exile is public: both seats read the card that left.
        for viewer in [P1, P2] {
            let view = view_for(&state, viewer);
            let exiled = events_of_type(&view.events, GameEventType::Exiled).last().map(json_of);
            assert_eq!(exiled.map(|event| event["defId"].clone()), Some(json!(grunt().id)));
        }
    }

    #[test]
    fn e17_radiant_groups_the_other_players_hand_by_the_cost_each_would_be_played_for_and_exiles_a_group() {
        let mut state = board("mind-melt-radiant");
        let ones = in_hand(&mut state, &plain.id, P2, 2);
        let zero = in_hand(&mut state, &grunt().id, P2, 1).remove(0);
        let three = in_hand(&mut state, &plain.id, P2, 1).remove(0);
        find_instance_mut(&mut state, &three.id).expect("the third card").cost_override = Some(3);
        cast_now(&mut state, &mind_melt().id, P1, true);
        let pending = open_as(&state, PromptKind::Mode, P1);
        assert_eq!(keys(&pending.options), ["mode:0", "mode:1", "mode:3"]);
        assert!(pending.options[1].label.contains(&plain.name));
        expect_only_that_it_is_open(&state, P2, P1);
        answer_keys(&mut state, &["mode:1"]);
        assert_eq!(sorted(ids(&state.players.p2.exile)), sorted(ids(&ones)));
        assert_eq!(sorted(ids(&state.players.p2.hand)), sorted(vec![zero.id.clone(), three.id.clone()]));
    }

    #[test]
    fn e17_a_mind_melt_game_replays_from_its_log() {
        let qd = quickdraw_of(&mind_melt()).id;
        let game = replayable("mind-melt-replay", &[qd.clone()], &[]);
        let decks = game.decks;
        let mut log = game.log;
        let mut state = game.state;
        let played = hand_card(&state, P1, &qd).id.clone();
        state = act(
            &state,
            input(json!({ "type": "play", "playerId": "p1", "instanceId": played })),
            Some(&mut log),
        );
        let pending = open_as(&state, PromptKind::Pick, P1);
        state = act(
            &state,
            input(json!({ "type": "answer", "playerId": "p1", "choiceId": pending.id, "selection": [at0(&pending)] })),
            Some(&mut log),
        );
        assert_eq!(state.players.p2.exile.len(), 1);
        expect_replays("mind-melt-replay", &decks, &log, &state);
    }
}
