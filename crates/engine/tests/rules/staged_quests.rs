// Issue #1854: Stinkyscape's staged Quests, an engine primitive proved on the fixture scripts of
// `fixtures/staged_quests.rs`. R1854000 the quest card of stages, its current stage public; R1854001
// "Complete the current stage on all your Quests", which reaches In Too Deep's tree as well; R1854002
// the six goals the set's stages need; R1854003 stealing the enemy's mana crystals; R1854004 "hear
// back in N turns".

use jackioh_engine::subsystems::quests::{QuestMemory, quest_memory_of};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::staged_quests::*;

/// `active`'s turn 9, its main phase; both sides hold `hand`s, the given fields, and plain cards to
/// draw, so no game ends by fatigue (§2.5).
fn board(
    active: &str,
    seed: &str,
    p1_hand: Value,
    p1_field: Value,
    p2_hand: Value,
    p2_field: Value,
) -> Scenario {
    register_staged_quest_fixtures();
    let library = json!([
        plain.id, plain.id, plain.id, plain.id, plain.id, plain.id, plain.id, plain.id
    ]);
    let mut s = scenario(json!({
        "seed": seed,
        "active": active,
        "p1": { "hand": p1_hand, "field": p1_field, "library": library },
        "p2": { "hand": p2_hand, "field": p2_field, "library": library },
    }));
    // R82: each turn ends when the test ends it, never on its own once nothing is left to do.
    s.state_mut().players.p1.auto_end_turn = Some(false);
    s.state_mut().players.p2.auto_end_turn = Some(false);
    s
}

/// `board` on p1's turn with only p1's hand and field.
fn mine(seed: &str, hand: Value, field: Value) -> Scenario {
    board("p1", seed, hand, field, json!([plain.id]), json!([]))
}

/// The quest card p1 plays from hand into its backrow: its id there.
fn play_quest(s: &mut Scenario, def_id: &str) -> String {
    s.play(def_id, json!({}));
    s.state()
        .players
        .p1
        .backrow
        .iter()
        .flatten()
        .find(|card| card.def_id == def_id)
        .unwrap_or_else(|| panic!("{def_id} is not in p1's backrow"))
        .id
        .clone()
}

fn line(s: &Scenario, id: &str) -> QuestMemory {
    quest_memory_of(s.card(id)).unwrap_or_else(|| panic!("{id} has no quest line"))
}

fn pick(id: &str) -> Value {
    json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
}

fn hero_pick(player: &str) -> Value {
    json!({ "targets": [{ "pick": "hero", "player": player }] })
}

fn id_at(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.unit(player, lane)
        .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
        .id
}

/// The quest line `viewer` is shown on the backrow card `id`.
fn shown(s: &Scenario, viewer: PlayerId, id: &str) -> Option<QuestView> {
    let view = s.view(viewer);
    view.you
        .backrow
        .iter()
        .chain(view.opponent.backrow.iter())
        .flatten()
        .find_map(|entry| match entry {
            BackrowView::Public(entry) if entry.instance_id == id => Some(entry.quest.clone()),
            _ => None,
        })
        .flatten()
}

fn stages_shown(s: &Scenario, viewer: PlayerId, id: &str) -> Vec<(String, QuestStageState)> {
    shown(s, viewer, id)
        .and_then(|quest| quest.stages)
        .unwrap_or_default()
        .into_iter()
        .map(|stage| (stage.name, stage.state))
        .collect()
}

fn completed(events: &[GameEvent], id: &str) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::QuestCompleted {
                instance_id, quest, ..
            } if instance_id == id => Some(quest.clone()),
            _ => None,
        })
        .collect()
}

fn progressed(events: &[GameEvent], id: &str) -> Vec<(String, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::QuestProgressed {
                instance_id,
                quest,
                progress,
                ..
            } if instance_id == id => Some((quest.clone(), *progress)),
            _ => None,
        })
        .collect()
}

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|item| item.to_string()).collect()
}

use QuestStageState::{Current, Done, Upcoming};

fn heist_stages(states: [QuestStageState; 3]) -> Vec<(String, QuestStageState)> {
    ["Initiation", "Comradery", "Empty the Vaults"]
        .iter()
        .zip(states)
        .map(|(name, state)| (name.to_string(), state))
        .collect()
}

/// p1's heist played and taken through stage 1 (a Jelly makes the Acclaimed Unit in hand Radiant), with
/// `hand` besides and the fields given.
fn heist_at_stage_two(seed: &str, hand: Value, field: Value, p2_field: Value) -> (Scenario, String) {
    let mut cards = vec![json!(heist.id), json!(jelly.id), json!(acclaimed.id)];
    cards.extend(hand.as_array().cloned().unwrap_or_default());
    let mut s = board("p1", seed, json!(cards), field, json!([plain.id]), p2_field);
    let quest = play_quest(&mut s, &heist.id);
    s.play(jelly.id.as_str(), json!({}));
    assert_eq!(line(&s, &quest).stage, Some(2), "stage 1 done, stage 2 open");
    (s, quest)
}

mod r1854000_staged_quests {
    use super::*;

    #[test]
    fn r1854000_a_staged_quest_opens_its_first_stage_as_it_enters_and_both_seats_see_every_stage() {
        let mut s = mine("r1854000-enter", json!([heist.id]), json!([]));
        let quest = play_quest(&mut s, &heist.id);
        let memory = line(&s, &quest);
        assert_eq!(memory.stage, Some(1));
        assert_eq!(memory.active, strings(&["radiant"]));
        assert_eq!(
            progressed(s.last_events(), &quest),
            vec![("radiant".to_string(), 0)]
        );
        for viewer in [P1, P2] {
            assert_eq!(
                stages_shown(&s, viewer, &quest),
                heist_stages([Current, Upcoming, Upcoming])
            );
            let view = shown(&s, viewer, &quest).expect("the line");
            assert_eq!(view.open.len(), 1);
            assert_eq!(view.open[0].text, "Make an Acclaimed Unit Radiant");
            assert_eq!((view.open[0].progress, view.open[0].goal), (0, 1));
            assert_eq!(view.open[0].rewards[0].text, "Draw a card");
            let stages = view.stages.expect("stages");
            assert_eq!(
                stages[1].goals,
                strings(&["Have 2 Acclaimed Units you control die"])
            );
            assert_eq!(stages[2].reward, "Steal your enemy's mana crystals");
        }
    }

    #[test]
    fn r1854000_a_completed_stage_gives_its_reward_and_the_next_stage_opens_counting_from_then() {
        let mut s = mine(
            "r1854000-next",
            json!([heist.id, jelly.id, acclaimed.id]),
            json!([]),
        );
        let quest = play_quest(&mut s, &heist.id);
        let hand_before = s.hand(P1).len();
        s.play(jelly.id.as_str(), json!({}));
        assert_eq!(completed(s.last_events(), &quest), strings(&["radiant"]));
        // The reward: draw a card (the Jelly left the hand, the draw came back in).
        assert_eq!(s.hand(P1).len(), hand_before - 1 + 1);
        let memory = line(&s, &quest);
        assert_eq!(memory.stage, Some(2));
        assert_eq!(memory.active, strings(&["deaths"]));
        assert_eq!(memory.done, strings(&["radiant"]));
        assert_eq!(memory.progress.get("deaths"), Some(&0));
        for viewer in [P1, P2] {
            assert_eq!(
                stages_shown(&s, viewer, &quest),
                heist_stages([Done, Current, Upcoming])
            );
        }
    }

    #[test]
    fn r1854000_the_last_stage_gives_its_reward_and_the_line_is_done() {
        let (mut s, quest) = heist_at_stage_two(
            "r1854000-last",
            json!([smite.id, smite.id, thief.id]),
            json!([{ "def": acclaimed.id, "lane": 1 }, { "def": acclaimed.id, "lane": 2 }]),
            json!([{ "def": plain.id, "lane": 1 }]),
        );
        let (first, second) = (id_at(&s, P1, 1), id_at(&s, P1, 2));
        s.play(smite.id.as_str(), pick(&first));
        s.play(smite.id.as_str(), pick(&second));
        assert_eq!(
            line(&s, &quest).stage,
            Some(3),
            "two Acclaimed deaths finish stage 2"
        );
        let enemy = id_at(&s, P2, 1);
        s.play(thief.id.as_str(), pick(&enemy));
        assert_eq!(completed(s.last_events(), &quest), strings(&["steal"]));
        let memory = line(&s, &quest);
        assert!(memory.active.is_empty(), "nothing is open after the last stage");
        assert_eq!(memory.done, strings(&["radiant", "deaths", "steal"]));
        assert_eq!(
            s.state().players.p2.mana.max,
            0,
            "the last reward stole p2's crystals"
        );
        for viewer in [P1, P2] {
            assert_eq!(stages_shown(&s, viewer, &quest), heist_stages([Done, Done, Done]));
        }
    }

    #[test]
    fn r1854000_a_stage_of_several_goals_draws_one_from_the_match_rng_as_it_opens() {
        let goals = [PROVE, BEHAVIORAL, HARDBALL];
        for (at, goal) in goals.iter().enumerate() {
            let seed = "r1854000-draw";
            let mut s = mine(seed, json!([application.id]), json!([]));
            let quest = play_quest(&mut s, &application.id);
            let cursor = cursor_where(seed, |cursor| Rng::new(seed, cursor).int(3) == at as i32);
            s.state_mut().rng_cursor = cursor;
            s.activate(quest.as_str(), json!({ "ability": INTERVIEW }));
            let memory = line(&s, &quest);
            assert_eq!(memory.stage, Some(2));
            assert_eq!(memory.active, strings(&[goal]), "the draw at cursor {cursor}");
            assert_eq!(s.state().rng_cursor, cursor + 1, "one draw");
            let stage = shown(&s, P2, &quest)
                .and_then(|view| view.stages)
                .expect("stages")[1]
                .clone();
            assert_eq!(stage.state, Current);
            assert_eq!(
                stage.goals.len(),
                1,
                "the goal drawn alone, once the stage is open"
            );
        }
        // Before it opens, the stage shows every goal it may draw.
        let mut s = mine("r1854000-draw", json!([application.id]), json!([]));
        let quest = play_quest(&mut s, &application.id);
        let stage = shown(&s, P1, &quest)
            .and_then(|view| view.stages)
            .expect("stages")[1]
            .clone();
        assert_eq!(stage.state, Upcoming);
        assert_eq!(stage.goals.len(), 3);
    }

    #[test]
    fn r1854000_a_stage_of_one_goal_opens_it_with_no_draw() {
        let mut s = mine("r1854000-no-draw", json!([heist.id]), json!([]));
        let before = s.state().rng_cursor;
        play_quest(&mut s, &heist.id);
        assert_eq!(
            s.state().rng_cursor,
            before,
            "playing the heist and opening its stage drew nothing"
        );
    }

    #[test]
    fn r1854000_returning_to_a_stage_opens_it_afresh() {
        let mut s = mine("r1854000-return", json!([application.id]), json!([]));
        let quest = play_quest(&mut s, &application.id);
        s.activate(quest.as_str(), json!({ "ability": SEND }));
        assert_eq!(line(&s, &quest).done, strings(&["resume"]));
        s.activate(quest.as_str(), json!({ "ability": REAPPLY }));
        let memory = line(&s, &quest);
        assert_eq!(memory.stage, Some(1));
        assert_eq!(memory.active, strings(&["resume"]), "stage 1 open again");
        assert!(memory.done.is_empty(), "its goal forgotten");
        assert_eq!(memory.hear_back, None, "the wait to hear back is over");
        assert_eq!(
            stages_shown(&s, P2, &quest),
            vec![
                ("Upload Resume".to_string(), Current),
                ("The Interview".to_string(), Upcoming),
                ("The Offer".to_string(), Upcoming),
            ]
        );
    }

    #[test]
    fn r1854000_leaving_the_field_resets_the_line_and_played_again_it_starts_at_stage_1() {
        let (mut s, quest) = heist_at_stage_two("r1854000-reset", json!([recall.id]), json!([]), json!([]));
        s.play(recall.id.as_str(), pick(&quest));
        let bounced = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == heist.id)
            .expect("the heist back in hand");
        assert!(
            quest_memory_of(&bounced).is_none(),
            "R78: the line went with the stay"
        );
        let again = play_quest(&mut s, &heist.id);
        let memory = line(&s, &again);
        assert_eq!(
            (memory.stage, memory.active.clone()),
            (Some(1), strings(&["radiant"]))
        );
    }

    #[test]
    fn r1854000_a_tree_s_line_keeps_its_shape_and_serializes_as_before() {
        let mut s = mine("r1854000-tree", json!([quest_tree.id]), json!([]));
        let quest = play_quest(&mut s, &quest_tree.id);
        let raw = s
            .card(quest.as_str())
            .memory
            .get("quest")
            .cloned()
            .expect("the line");
        let mut keys: Vec<&str> = raw
            .as_object()
            .expect("an object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["active", "auras", "done", "progress", "waiting"]);
        assert_eq!(
            shown(&s, P2, &quest).and_then(|view| view.stages),
            None,
            "a tree has no stages"
        );
    }

    #[test]
    fn r1854000_a_staged_line_survives_a_json_round_trip_mid_wait() {
        let mut s = mine("r1854000-json", json!([application.id]), json!([]));
        let quest = play_quest(&mut s, &application.id);
        s.activate(quest.as_str(), json!({ "ability": SEND }));
        let text = serde_json::to_string(s.state()).expect("serializes");
        let back: GameState = serde_json::from_str(&text).expect("deserializes");
        assert_eq!(hash_state(&back), hash_state(s.state()));
        assert_eq!(
            quest_memory_of(find_instance(&back, &quest).expect("the card")),
            Some(line(&s, &quest))
        );
    }
}

mod r1854001_complete_quests {
    use super::*;

    #[test]
    fn r1854001_tylannt_completes_the_current_stage_of_every_quest_of_yours_a_tree_s_included() {
        let mut s = mine(
            "r1854001-all",
            json!([heist.id, quest_tree.id, tylannt.id]),
            json!([]),
        );
        let staged = play_quest(&mut s, &heist.id);
        let tree = play_quest(&mut s, &quest_tree.id);
        s.play(tylannt.id.as_str(), json!({}));
        let events = s.last_events().to_vec();
        assert_eq!(completed(&events, &staged), strings(&["radiant"]));
        assert_eq!(completed(&events, &tree), strings(&["draws"]));
        // The heist's reward ran and its next stage opened, waiting for its own goal.
        let memory = line(&s, &staged);
        assert_eq!(
            (memory.stage, memory.active.clone()),
            (Some(2), strings(&["deaths"]))
        );
        assert_eq!(memory.progress.get("deaths"), Some(&0));
        // The tree's base face asks for its reward, as any completion of it does (R404).
        assert_eq!(
            s.state().pending.as_ref().map(|prompt| prompt.kind),
            Some(PromptKind::Reward)
        );
    }

    #[test]
    fn r1854001_the_opponent_s_quests_and_untagged_ones_are_passed_by() {
        let mut s = board(
            "p1",
            "r1854001-passed",
            json!([untagged_stages.id, tylannt.id]),
            json!([]),
            json!([plain.id]),
            json!([{ "def": heist.id, "row": "backrow", "lane": 1 }]),
        );
        let untagged = play_quest(&mut s, &untagged_stages.id);
        let theirs = s.backrow(P2, 1).expect("p2's heist").id;
        s.play(tylannt.id.as_str(), json!({}));
        assert!(completed(s.last_events(), &untagged).is_empty());
        assert!(completed(s.last_events(), &theirs).is_empty());
        assert_eq!(line(&s, &untagged).stage, Some(1));
        assert_eq!(line(&s, &theirs).stage, Some(1));
    }

    #[test]
    fn r1854001_a_line_hearing_back_has_nothing_to_complete() {
        let mut s = mine("r1854001-waiting", json!([application.id, tylannt.id]), json!([]));
        let quest = play_quest(&mut s, &application.id);
        s.activate(quest.as_str(), json!({ "ability": SEND }));
        let waiting = line(&s, &quest);
        assert!(waiting.hear_back.is_some());
        s.play(tylannt.id.as_str(), json!({}));
        assert!(completed(s.last_events(), &quest).is_empty());
        assert_eq!(line(&s, &quest), waiting, "the wait is untouched");
    }

    #[test]
    fn r1854001_a_quest_summoned_in_the_same_effect_list_opens_and_completes_its_first_stage() {
        let mut s = mine("r1854001-summoned", json!([radiant_tylannt.id]), json!([]));
        s.play(radiant_tylannt.id.as_str(), json!({}));
        let quest = s.backrow(P1, 1).expect("the summoned heist").id;
        assert_eq!(completed(s.last_events(), &quest), strings(&["radiant"]));
        assert_eq!(line(&s, &quest).stage, Some(2));
    }

    #[test]
    fn r1854001_a_by_effect_goal_waits_for_an_effect_and_its_own_activate_completes_it() {
        let mut s = mine("r1854001-own", json!([application.id, cost_two.id]), json!([]));
        let quest = play_quest(&mut s, &application.id);
        s.play(cost_two.id.as_str(), json!({}));
        s.end_turn().end_turn();
        assert_eq!(
            line(&s, &quest).active,
            strings(&["resume"]),
            "nothing else counts it"
        );
        let open = &shown(&s, P2, &quest).expect("the line").open[0];
        assert_eq!((open.progress, open.goal), (0, 1));
        s.activate(quest.as_str(), json!({ "ability": SEND }));
        assert_eq!(completed(s.last_events(), &quest), strings(&["resume"]));
    }
}

mod r1854002_goals {
    use super::*;

    fn goal_card(kind: GoalKind) -> String {
        GOAL_CARDS[kind].id.clone()
    }

    /// p1 plays the goal card of `kind` first, then the rest of its hand as the test says.
    fn with_goal(kind: GoalKind, hand: Value, field: Value, p2_field: Value) -> (Scenario, String) {
        let mut cards = vec![json!(goal_card(kind))];
        cards.extend(hand.as_array().cloned().unwrap_or_default());
        let mut s = board("p1", kind, json!(cards), field, json!([plain.id]), p2_field);
        let quest = play_quest(&mut s, &goal_card(kind));
        (s, quest)
    }

    fn done(s: &Scenario, quest: &str) -> bool {
        line(s, quest).done == strings(&[ONLY_GOAL])
    }

    #[test]
    fn r1854002_an_acclaimed_unit_of_yours_made_radiant_in_the_hand_or_on_the_field_counts() {
        let (mut s, quest) = with_goal(
            "unitsMadeRadiant",
            json!([jelly.id, acclaimed.id]),
            json!([]),
            json!([]),
        );
        s.play(jelly.id.as_str(), json!({}));
        assert!(done(&s, &quest), "an Acclaimed Unit made Radiant in the hand");

        let (mut s, quest) = with_goal(
            "unitsMadeRadiant",
            json!([glow.id]),
            json!([{ "def": acclaimed.id, "lane": 1 }]),
            json!([]),
        );
        let unit = id_at(&s, P1, 1);
        s.play(glow.id.as_str(), pick(&unit));
        assert!(done(&s, &quest), "an Acclaimed Unit made Radiant on the field");
    }

    #[test]
    fn r1854002_an_untagged_unit_or_an_enemy_acclaimed_unit_made_radiant_counts_for_nothing() {
        let (mut s, quest) = with_goal(
            "unitsMadeRadiant",
            json!([jelly.id, nobody.id]),
            json!([]),
            json!([]),
        );
        s.play(jelly.id.as_str(), json!({}));
        assert!(!done(&s, &quest));
        assert_eq!(line(&s, &quest).progress.get(ONLY_GOAL), Some(&0));

        let (mut s, quest) = with_goal(
            "unitsMadeRadiant",
            json!([glow.id]),
            json!([]),
            json!([{ "def": acclaimed.id, "lane": 1 }]),
        );
        let enemy = id_at(&s, P2, 1);
        s.play(glow.id.as_str(), pick(&enemy));
        assert!(!done(&s, &quest), "the enemy's Acclaimed Unit is not yours");
    }

    #[test]
    fn r1854002_two_of_your_acclaimed_units_dying_counts_and_others_deaths_do_not() {
        let (mut s, quest) = with_goal(
            "unitsDied",
            json!([smite.id, smite.id, smite.id, smite.id]),
            json!([{ "def": acclaimed.id, "lane": 1 }, { "def": nobody.id, "lane": 2 }, { "def": acclaimed.id, "lane": 3 }]),
            json!([{ "def": acclaimed.id, "lane": 1 }]),
        );
        let (first, untagged, last, enemy) = (
            id_at(&s, P1, 1),
            id_at(&s, P1, 2),
            id_at(&s, P1, 3),
            id_at(&s, P2, 1),
        );
        s.play(smite.id.as_str(), pick(&enemy));
        s.play(smite.id.as_str(), pick(&untagged));
        assert_eq!(
            line(&s, &quest).progress.get(ONLY_GOAL),
            Some(&0),
            "the enemy's and an untagged death"
        );
        s.play(smite.id.as_str(), pick(&first));
        assert_eq!(
            progressed(s.last_events(), &quest),
            vec![(ONLY_GOAL.to_string(), 1)]
        );
        s.play(smite.id.as_str(), pick(&last));
        assert!(done(&s, &quest));
    }

    #[test]
    fn r1854002_a_steal_onto_your_side_or_into_your_hand_counts_and_a_card_given_away_does_not() {
        let (mut s, quest) = with_goal(
            "cardsStolen",
            json!([thief.id]),
            json!([]),
            json!([{ "def": plain.id, "lane": 1 }]),
        );
        let enemy = id_at(&s, P2, 1);
        s.play(thief.id.as_str(), pick(&enemy));
        assert!(done(&s, &quest), "a Steal onto your side");

        let (mut s, quest) = with_goal("cardsStolen", json!([pickpocket.id]), json!([]), json!([]));
        s.play(pickpocket.id.as_str(), json!({}));
        assert!(done(&s, &quest), "a card taken from their hand");

        let (mut s, quest) = with_goal("cardsStolen", json!([charity.id, plain.id]), json!([]), json!([]));
        s.play(charity.id.as_str(), json!({}));
        assert!(
            !done(&s, &quest),
            "a card of yours your opponent takes is no steal of yours"
        );
    }

    #[test]
    fn r1854002_a_play_at_the_cost_asked_counts_and_other_costs_do_not() {
        let (mut s, quest) = with_goal(
            "playCosting",
            json!([cost_zero.id, cost_one.id, cost_two.id]),
            json!([]),
            json!([]),
        );
        assert_eq!(shown(&s, P2, &quest).expect("the line").open[0].cost, Some(2));
        s.play(cost_zero.id.as_str(), json!({}));
        s.play(cost_one.id.as_str(), json!({}));
        assert!(!done(&s, &quest));
        s.end_turn().end_turn();
        s.play(cost_two.id.as_str(), json!({}));
        assert!(done(&s, &quest));
    }

    #[test]
    fn r1854002_the_cost_asked_may_be_one_the_quest_card_remembers() {
        let mut s = mine(
            "r1854002-remembered",
            json!([application.id, cost_one.id, cost_two.id]),
            json!([]),
        );
        let quest = play_quest(&mut s, &application.id);
        s.activate(quest.as_str(), json!({ "ability": SEND }));
        let seed = "r1854002-remembered";
        s.state_mut().rng_cursor = cursor_where(seed, |cursor| Rng::new(seed, cursor).int(3) == 0);
        s.activate(quest.as_str(), json!({ "ability": INTERVIEW }));
        assert_eq!(line(&s, &quest).active, strings(&[PROVE]));
        assert_eq!(
            shown(&s, P2, &quest).expect("the line").open[0].cost,
            Some(RESUME_COST)
        );
        s.play(cost_one.id.as_str(), json!({}));
        assert!(completed(s.last_events(), &quest).is_empty());
        s.play(cost_two.id.as_str(), json!({}));
        assert_eq!(completed(s.last_events(), &quest), strings(&[PROVE]));
    }

    #[test]
    fn r1854002_three_different_costs_in_one_turn_count_and_a_new_turn_starts_again() {
        let (mut s, quest) = with_goal(
            "differentCostsInTurn",
            json!([
                cost_one.id,
                cost_one.id,
                cost_two.id,
                cost_zero.id,
                cost_one.id,
                cost_two.id
            ]),
            json!([]),
            json!([]),
        );
        s.play(cost_one.id.as_str(), json!({}));
        s.play(cost_one.id.as_str(), json!({}));
        assert!(
            progressed(s.last_events(), &quest).is_empty(),
            "the same cost again counts nothing"
        );
        s.play(cost_two.id.as_str(), json!({}));
        assert_eq!(shown(&s, P1, &quest).expect("the line").open[0].progress, 2);
        s.end_turn().end_turn();
        assert_eq!(
            shown(&s, P1, &quest).expect("the line").open[0].progress,
            0,
            "a new turn"
        );
        s.play(cost_zero.id.as_str(), json!({}));
        s.play(cost_one.id.as_str(), json!({}));
        assert!(!done(&s, &quest));
        s.play(cost_two.id.as_str(), json!({}));
        assert!(done(&s, &quest));
    }

    #[test]
    fn r1854002_one_instance_of_12_dealt_or_taken_by_your_hero_counts_and_11_does_not() {
        let (mut s, quest) = with_goal(
            "damageInstance",
            json!([near_hit.id, big_hit.id]),
            json!([]),
            json!([]),
        );
        s.play(near_hit.id.as_str(), hero_pick("p2"));
        assert!(!done(&s, &quest), "11 is not 12");
        s.play(big_hit.id.as_str(), hero_pick("p2"));
        assert!(done(&s, &quest), "12 dealt");

        let mut s = board(
            "p2",
            "r1854002-taken",
            json!([goal_card("damageInstance")]),
            json!([{ "def": plain.id, "lane": 1 }]),
            json!([big_hit.id, big_hit.id]),
            json!([]),
        );
        s.end_turn();
        let quest = play_quest(&mut s, &goal_card("damageInstance"));
        s.end_turn();
        let unit = id_at(&s, P1, 1);
        s.play(big_hit.id.as_str(), pick(&unit));
        assert!(
            !done(&s, &quest),
            "12 to a Unit of yours is not your hero taking it"
        );
        s.play(big_hit.id.as_str(), hero_pick("p1"));
        assert!(done(&s, &quest), "12 taken by your hero");
    }
}

mod r1854003_steal_mana_crystals {
    use super::*;

    fn mana(s: &Scenario, player: PlayerId) -> (i32, i32) {
        let side = &s.state().players[player];
        (side.mana.current, side.mana.max)
    }

    /// The `manaChanged` reports after the play's announcement (`cardPlayed`): the steal's, not the
    /// price's.
    fn mana_events(events: &[GameEvent]) -> Vec<(PlayerId, i32, i32)> {
        events
            .iter()
            .skip_while(|event| !matches!(event, GameEvent::CardPlayed { .. }))
            .filter_map(|event| match event {
                GameEvent::ManaChanged { player, current, max } => Some((*player, *current, *max)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r1854003_the_enemy_loses_their_crystals_and_a_thief_at_the_cap_gains_none() {
        let mut s = mine("r1854003-cap", json!([heist_spell.id]), json!([]));
        s.play(heist_spell.id.as_str(), json!({}));
        assert_eq!(mana(&s, P2), (0, 0));
        assert_eq!(mana(&s, P1), (MAX_MANA, MAX_MANA), "already at the cap");
        assert_eq!(
            mana_events(s.last_events()),
            vec![(P2, 0, 0)],
            "only the side that changed"
        );
        let view = s.view(P2);
        assert_eq!((view.you.mana.current, view.you.mana.max), (0, 0));
    }

    #[test]
    fn r1854003_the_thief_gains_them_full_up_to_the_cap_and_never_past_it() {
        let mut s = board(
            "p1",
            "r1854003-early",
            json!([heist_spell.id]),
            json!([]),
            json!([plain.id]),
            json!([]),
        );
        // Turn 3: p1 has started 2 turns (2/2), p2 one (1/1).
        s.state_mut().turn = 3;
        s.state_mut().players.p1.turns_started = 2;
        s.state_mut().players.p2.turns_started = 1;
        s.state_mut().players.p1.mana = ManaState {
            current: 2,
            max: 2,
            next_turn_mod: 0,
            perm_mod: 0,
        };
        s.state_mut().players.p2.mana = ManaState {
            current: 1,
            max: 1,
            next_turn_mod: 0,
            perm_mod: 0,
        };
        s.play(heist_spell.id.as_str(), json!({}));
        assert_eq!(mana(&s, P1), (3, 3), "one crystal gained, full");
        assert_eq!(mana(&s, P2), (0, 0));
        assert_eq!(mana_events(s.last_events()), vec![(P2, 0, 0), (P1, 3, 3)]);
        // p1's crystals grow with its turns, inside the cap.
        let mut maxes = Vec::new();
        for _ in 0..3 {
            s.end_turn().end_turn();
            maxes.push(mana(&s, P1).1);
        }
        assert_eq!(maxes, vec![MAX_MANA, MAX_MANA, MAX_MANA]);
    }

    #[test]
    fn r1854003_the_stolen_crystals_come_back_one_a_turn() {
        let mut s = mine("r1854003-regrow", json!([heist_spell.id]), json!([]));
        s.play(heist_spell.id.as_str(), json!({}));
        let mut maxes = Vec::new();
        for _ in 0..5 {
            s.end_turn();
            maxes.push(mana(&s, P2).1);
            s.end_turn();
        }
        assert_eq!(maxes, vec![1, 2, 3, MAX_MANA, MAX_MANA]);
    }

    #[test]
    fn r1854003_a_player_with_no_maximum_mana_has_nothing_to_steal() {
        let mut s = mine(
            "r1854003-none",
            json!([heist_spell.id, heist_spell.id]),
            json!([]),
        );
        s.play(heist_spell.id.as_str(), json!({}));
        let before = s.state().players.p1.mana;
        s.play(heist_spell.id.as_str(), json!({}));
        assert!(mana_events(s.last_events()).is_empty());
        assert_eq!(s.state().players.p1.mana, before);
    }
}

mod r1854004_hear_back {
    use super::*;

    fn hear_back_shown(s: &Scenario, quest: &str) -> Option<i32> {
        shown(s, P2, quest)
            .and_then(|view| view.stages)
            .and_then(|stages| stages.iter().find_map(|stage| stage.hear_back))
    }

    #[test]
    fn r1854004_the_next_stage_opens_at_the_start_of_the_controller_s_second_turn_before_its_draw() {
        let mut s = mine("r1854004-two", json!([application.id]), json!([]));
        let quest = play_quest(&mut s, &application.id);
        s.activate(quest.as_str(), json!({ "ability": SEND }));
        assert_eq!(
            line(&s, &quest).hear_back.map(|wait| wait.turns),
            Some(HEAR_BACK_TURNS)
        );
        assert_eq!(hear_back_shown(&s, &quest), Some(2));
        s.end_turn();
        assert_eq!(
            hear_back_shown(&s, &quest),
            Some(2),
            "the opponent's turn counts nothing"
        );
        s.end_turn();
        assert_eq!(hear_back_shown(&s, &quest), Some(1));
        assert!(line(&s, &quest).active.is_empty());
        s.end_turn().end_turn();
        let memory = line(&s, &quest);
        assert_eq!(memory.stage, Some(2));
        assert_eq!(memory.hear_back, None);
        assert_eq!(memory.active.len(), 1, "one Interview goal drawn");
        let events = s.last_events();
        let opened = events
            .iter()
            .position(|event| matches!(event, GameEvent::QuestProgressed { instance_id, .. } if *instance_id == quest))
            .expect("the stage's opening report");
        let drew = events
            .iter()
            .position(|event| matches!(event, GameEvent::Drawn { player: P1, .. }))
            .expect("the turn's draw");
        assert!(opened < drew, "it hears back before the turn's draw");
        assert_eq!(
            stages_shown(&s, P2, &quest)[1],
            ("The Interview".to_string(), Current)
        );
    }

    #[test]
    fn r1854004_hear_back_in_0_turns_opens_the_next_stage_at_once() {
        let mut s = mine("r1854004-zero", json!([instant.id]), json!([]));
        let quest = play_quest(&mut s, &instant.id);
        s.activate(quest.as_str(), json!({ "ability": SEND }));
        let memory = line(&s, &quest);
        assert_eq!(
            (memory.stage, memory.active.clone()),
            (Some(2), strings(&["answer"]))
        );
        assert_eq!(memory.hear_back, None);
    }

    #[test]
    fn r1854004_leaving_the_field_ends_the_wait() {
        let mut s = mine("r1854004-leave", json!([application.id, recall.id]), json!([]));
        let quest = play_quest(&mut s, &application.id);
        s.activate(quest.as_str(), json!({ "ability": SEND }));
        s.play(recall.id.as_str(), pick(&quest));
        let again = play_quest(&mut s, &application.id);
        let memory = line(&s, &again);
        assert_eq!((memory.stage, memory.hear_back), (Some(1), None));
        s.end_turn().end_turn().end_turn().end_turn();
        assert_eq!(line(&s, &again).stage, Some(1), "nothing heard back");
    }
}
