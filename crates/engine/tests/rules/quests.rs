//! The quests subsystem (docs/classic-sets.md B5 E33; SPEC §8.6 row 90, §10.1, §10.6, §10.8; R404),
//! proved through the quest fixture cards so the engine half of Classic #90 In Too Deep stands without
//! the cards crate: each goal counted and not counted, completion at the state check on either
//! player's turn, the base face's `reward` prompt, the Radiant face's every reward and path, a pause
//! mid-reward through JSON, a replay from the log, both views, and the two random picks the rewards
//! need. The real card's own test file proves the same with its real tree.

use std::sync::atomic::{AtomicU32, Ordering};

use serde::Serialize;

use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::subsystems::quests::{quest_book_of, quest_memory_of};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{
    events_of_type, in_hand, new_game, put, set_library, setup_catalog, slot,
};
use crate::rules::fixtures::prompt_harness::{answer_keys, open_as, round_trip};
use crate::rules::fixtures::quests::{
    BOLT, GOAL_CARDS, GOALS, GoalKind, ONLY_QUEST, RECALL, TREE, TREE_BUFF, TREE_HEAL, TREE_PING, banish,
    banish_any, blank, bless, bolt, draw_one, draw_then_recruit, draw_two, limiter, mill, recall,
    register_quest_fixtures, slay, tree,
};
use crate::rules::fixtures::scripts::cn_virus;

const RUSH_TOKEN: &str = "fx-token-rush";

fn four_mana() -> ManaState {
    ManaState {
        current: 4,
        max: 4,
        next_turn_mod: 0,
        perm_mod: 0,
    }
}

/// p1's main phase on turn 3 with 4 mana each side, the quest fixtures registered.
fn quest_board(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    register_quest_fixtures();
    state.turn = 3;
    state.active = P1;
    state.phase = Phase::Main;
    state.players.p1.mana = four_mana();
    state.players.p2.mana = four_mana();
    // R345: each turn ends when the test ends it, never on its own (R82).
    state.players.p1.auto_end_turn = Some(false);
    state.players.p2.auto_end_turn = Some(false);
    state
}

struct Step {
    state: GameState,
    events: Vec<GameEvent>,
}

static NONCE: AtomicU32 = AtomicU32::new(0);

fn act(state: &GameState, body: Value, log: Option<&mut Vec<Action>>) -> Step {
    let nonce = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let action = json_as::<ActionInput>(body).with_nonce(format!("qt{nonce}"));
    let result = reduce(state, &action);
    if let Some(error) = &result.error {
        panic!("{} refused: {error}", action.action_type());
    }
    if let Some(log) = log {
        log.push(action);
    }
    Step {
        state: result.state,
        events: result.events,
    }
}

fn play(state: &GameState, card: &CardInstance, targets: &[&str], log: Option<&mut Vec<Action>>) -> Step {
    let targets: Vec<Value> = targets
        .iter()
        .map(|id| match id.strip_prefix("hero-") {
            Some(player) => json!({ "pick": "hero", "player": player }),
            None => json!({ "pick": "instance", "instanceId": id }),
        })
        .collect();
    act(
        state,
        json!({ "type": "play", "playerId": card.owner, "instanceId": card.id, "targets": targets }),
        log,
    )
}

fn hand(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    in_hand(state, def_id, player, 1)
        .into_iter()
        .next()
        .expect(def_id)
}

fn play_new(state: &mut GameState, def_id: &str, player: PlayerId, targets: &[&str]) -> Step {
    let card = hand(state, def_id, player);
    play(state, &card, targets, None)
}

struct Played {
    state: GameState,
    events: Vec<GameEvent>,
    card: CardInstance,
}

/// A quest card played by p1 into the backrow: its first quest opens as it enters.
fn play_quest_card(state: &mut GameState, def_id: &str, player: PlayerId) -> Played {
    let card = hand(state, def_id, player);
    let step = play(state, &card, &[], None);
    let card = find_instance(&step.state, &card.id)
        .expect("the quest card")
        .clone();
    Played {
        state: step.state,
        events: step.events,
        card,
    }
}

fn json_of(value: impl Serialize) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

/// Every key the expected object names matches, recursively; an array matches element for element
/// and in length.
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

/// The card's quest line, as JSON.
fn memory_of(state: &GameState, card: &CardInstance) -> Value {
    let live = find_instance(state, &card.id).expect("the card");
    json_of(quest_memory_of(live).expect("a quest line"))
}

fn progress_of(state: &GameState, card: &CardInstance, quest: &str) -> i64 {
    memory_of(state, card)["progress"][quest].as_i64().unwrap_or(0)
}

/// A number a goal names.
fn goal_number(goal: impl Serialize, field: &str) -> i64 {
    json_of(goal)[field].as_i64().expect("a goal number")
}

fn goal(state: &mut GameState, kind: GoalKind, player: PlayerId) -> Played {
    let def_id = GOAL_CARDS[kind].id.clone();
    play_quest_card(state, &def_id, player)
}

fn completed(events: &[GameEvent], card: &CardInstance) -> Vec<String> {
    events_of_type(events, GameEventType::QuestCompleted)
        .into_iter()
        .map(json_of)
        .filter(|event| event["instanceId"] == json!(card.id))
        .map(|event| event["quest"].as_str().unwrap_or_default().to_string())
        .collect()
}

fn field_of(events: &[GameEvent], kind: GameEventType, field: &str) -> Vec<Value> {
    events_of_type(events, kind)
        .into_iter()
        .map(|event| json_of(event)[field].clone())
        .collect()
}

/// The card view of a face-up backrow card as `viewer` is shown it.
fn shown_to(state: &GameState, viewer: PlayerId, card: &CardInstance) -> Option<PublicBackrowView> {
    let view = view_for(state, viewer);
    let side = if view.you.player == card.controller {
        view.you
    } else {
        view.opponent
    };
    side.backrow.into_iter().flatten().find_map(|entry| match entry {
        BackrowView::Public(entry) if entry.instance_id == card.id => Some(entry),
        _ => None,
    })
}

fn end_turn(state: &GameState) -> Step {
    let player = state.active;
    act(state, json!({ "type": "endTurn", "playerId": player }), None)
}

fn live_mut<'a>(state: &'a mut GameState, card: &CardInstance) -> &'a mut CardInstance {
    find_instance_mut(state, &card.id).expect("the card")
}

fn has_indestructible(state: &GameState, unit: &CardInstance) -> bool {
    let live = find_instance(state, &unit.id).expect("unit");
    has_keyword(&unit_view(state, live).keywords, KeywordKind::Indestructible)
}

mod r404_e33_quests_the_first_quest_opens_as_the_card_enters {
    use super::*;

    #[test]
    fn r404_a_played_quest_card_opens_its_first_quest_reported_at_0_and_counts_from_then_on() {
        let mut state = quest_board("q-open");
        in_hand(&mut state, &plain.id, P1, 1);
        let Played {
            state: after,
            events,
            card,
        } = goal(&mut state, "draws", P1);
        assert_eq!(
            memory_of(&after, &card),
            json!({ "active": [ONLY_QUEST], "progress": { ONLY_QUEST: 0 }, "done": [], "auras": [], "waiting": [] })
        );
        assert_eq!(
            json_of(events_of_type(&events, GameEventType::QuestProgressed)),
            json!([{
                "type": "questProgressed",
                "player": "p1",
                "instanceId": card.id,
                "quest": ONLY_QUEST,
                "progress": 0,
                "goal": goal_number(&GOALS["draws"], "count"),
            }])
        );
        // The play's own events came before the opening: the quest has counted nothing.
        assert_eq!(progress_of(&after, &card, ONLY_QUEST), 0);
    }

    #[test]
    fn r404_a_card_placed_on_the_field_without_a_play_opens_at_the_first_event_or_check_after_it_counting_what_follows()
     {
        let mut state = quest_board("q-placed");
        let card = put(
            &mut state,
            &GOAL_CARDS["draws"].id,
            slot(P1, Row::Backrow, 1),
            json!({}),
        );
        assert!(quest_memory_of(&card).is_none());
        let Step { state: after, events } = play_new(&mut state, &draw_two().id, P1, &[]);
        assert_eq!(completed(&events, &card), [ONLY_QUEST]);
        assert_eq!(memory_of(&after, &card)["done"], json!([ONLY_QUEST]));
    }

    #[test]
    fn r212_a_quest_card_recruited_after_two_draws_in_the_same_list_does_not_count_them() {
        let mut state = quest_board("q-recruit");
        let (plain_id, draws_id) = (plain.id.clone(), GOAL_CARDS["draws"].id.clone());
        set_library(
            &mut state,
            P1,
            &[
                plain_id.as_str(),
                plain_id.as_str(),
                draws_id.as_str(),
                plain_id.as_str(),
            ],
        );
        let after = play_new(&mut state, &draw_then_recruit().id, P1, &[]).state;
        let card = after
            .players
            .p1
            .backrow
            .iter()
            .flatten()
            .find(|card| card.def_id == draws_id)
            .expect("the recruited card")
            .clone();
        assert_eq!(memory_of(&after, &card)["active"], json!([ONLY_QUEST]));
        assert_eq!(progress_of(&after, &card, ONLY_QUEST), 0);
    }
}

mod e33_quests_each_goal_counted_and_not_counted {
    use super::*;

    #[test]
    fn r541_draws_your_draws_count_a_burned_one_and_a_cast_on_draw_one_included_the_opponents_do_not() {
        let plain_id = plain.id.clone();
        let mut state = quest_board("q-draws");
        set_library(
            &mut state,
            P1,
            &[
                cn_virus().id.as_str(),
                plain_id.as_str(),
                plain_id.as_str(),
                plain_id.as_str(),
            ],
        );
        let Played {
            state: mut s1, card, ..
        } = goal(&mut state, "draws", P1);
        // One draw that casts the CN-Virus and draws on (R58): two draws.
        let step = play_new(&mut s1, &draw_one().id, P1, &[]);
        assert_eq!(
            field_of(&step.events, GameEventType::Drawn, "player")
                .iter()
                .filter(|player| **player == json!("p1"))
                .count(),
            2
        );
        assert_eq!(completed(&step.events, &card), [ONLY_QUEST]);

        // A burned draw: the hand is full.
        let mut state = quest_board("q-draws-burn");
        set_library(
            &mut state,
            P1,
            &[plain_id.as_str(), plain_id.as_str(), plain_id.as_str()],
        );
        let Played {
            state: mut s1, card, ..
        } = goal(&mut state, "draws", P1);
        in_hand(&mut s1, &plain_id, P1, (HAND_CAP - 1) as _);
        let mut burned = play_new(&mut s1, &draw_one().id, P1, &[]);
        assert_eq!(events_of_type(&burned.events, GameEventType::Burned).len(), 0);
        in_hand(&mut burned.state, &plain_id, P1, 1);
        let full = play_new(&mut burned.state, &draw_one().id, P1, &[]);
        // The hand held 10 when the draw came: the card burned, and it was a draw all the same.
        assert_eq!(events_of_type(&full.events, GameEventType::Burned).len(), 1);
        assert_eq!(completed(&full.events, &card), [ONLY_QUEST]);

        // The opponent's draws: p2's turn starts with a draw, which counts for nothing of p1's.
        let mut state = quest_board("q-draws-theirs");
        let Played {
            state: mut s1, card, ..
        } = goal(&mut state, "draws", P1);
        in_hand(&mut s1, &plain_id, P2, 1);
        let theirs = end_turn(&s1);
        assert!(
            field_of(&theirs.events, GameEventType::Drawn, "player")
                .iter()
                .any(|player| *player == json!("p2"))
        );
        assert_eq!(progress_of(&theirs.state, &card, ONLY_QUEST), 0);
    }

    #[test]
    fn r541_draws_a_draw_a_limit_stopped_never_happened_and_a_fatigue_draw_takes_no_card() {
        let plain_id = plain.id.clone();
        let mut state = quest_board("q-draws-limit");
        set_library(
            &mut state,
            P1,
            &[plain_id.as_str(), plain_id.as_str(), plain_id.as_str()],
        );
        put(&mut state, &limiter().id, slot(P1, Row::Backrow, 2), json!({}));
        let Played {
            state: mut s1, card, ..
        } = goal(&mut state, "draws", P1);
        let limited = play_new(&mut s1, &draw_two().id, P1, &[]);
        assert_eq!(
            events_of_type(&limited.events, GameEventType::DrawLimited).len(),
            1
        );
        assert_eq!(progress_of(&limited.state, &card, ONLY_QUEST), 1);

        let mut state = quest_board("q-draws-fatigue");
        set_library(&mut state, P1, &[] as &[&str]);
        let Played {
            state: mut s1, card, ..
        } = goal(&mut state, "draws", P1);
        let tired = play_new(&mut s1, &draw_two().id, P1, &[]);
        assert_eq!(events_of_type(&tired.events, GameEventType::Fatigue).len(), 2);
        assert_eq!(progress_of(&tired.state, &card, ONLY_QUEST), 0);
    }

    #[test]
    fn r404_deck_emptied_by_draw_the_draw_that_takes_your_decks_last_card_a_mill_that_empties_it_does_not_an_empty_deck_at_the_opening_completes_it_at_once()
     {
        let plain_id = plain.id.clone();
        let mut state = quest_board("q-deck");
        set_library(&mut state, P1, &[plain_id.as_str(), plain_id.as_str()]);
        let Played {
            state: mut s1, card, ..
        } = goal(&mut state, "deckEmptiedByDraw", P1);
        let mut one = play_new(&mut s1, &draw_one().id, P1, &[]);
        assert!(
            events_of_type(&one.events, GameEventType::Drawn)
                .first()
                .map(json_of)
                .is_none_or(|drawn| drawn.get("emptied").is_none())
        );
        assert_eq!(progress_of(&one.state, &card, ONLY_QUEST), 0);
        let last = play_new(&mut one.state, &draw_one().id, P1, &[]);
        assert_eq!(
            field_of(&last.events, GameEventType::Drawn, "emptied").first(),
            Some(&json!(true))
        );
        assert_eq!(completed(&last.events, &card), [ONLY_QUEST]);

        let mut state = quest_board("q-deck-mill");
        set_library(&mut state, P1, &[plain_id.as_str()]);
        let Played {
            state: mut s1, card, ..
        } = goal(&mut state, "deckEmptiedByDraw", P1);
        let mut milled = play_new(&mut s1, &mill().id, P1, &[]);
        assert_eq!(milled.state.players.p1.library.len(), 0);
        assert_eq!(progress_of(&milled.state, &card, ONLY_QUEST), 0);
        // ... and the fatigue draw after it takes no card either.
        let tired = play_new(&mut milled.state, &draw_one().id, P1, &[]);
        assert_eq!(memory_of(&tired.state, &card)["done"], json!([]));

        let mut state = quest_board("q-deck-empty");
        set_library(&mut state, P1, &[] as &[&str]);
        let opened = goal(&mut state, "deckEmptiedByDraw", P1);
        assert_eq!(
            field_of(&opened.events, GameEventType::QuestProgressed, "progress").first(),
            Some(&json!(1))
        );
        assert_eq!(completed(&opened.events, &opened.card), [ONLY_QUEST]);
    }

    #[test]
    fn r404_enemy_permanents_destroyed_by_anything_counted_on_the_side_it_died_on_destroyed_controller() {
        let mut state = quest_board("q-kills");
        let theirs = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        let mine = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        // A unit p2 owns that p1 controls (stolen), and one p1 owns that p2 controls.
        let stolen = put(&mut state, &plain.id, slot(P1, Row::Units, 2), json!({}));
        live_mut(&mut state, &stolen).owner = P2;
        let lent = put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));
        live_mut(&mut state, &lent).owner = P1;
        let token = put(&mut state, RUSH_TOKEN, slot(P2, Row::Units, 3), json!({}));
        let Played {
            state: mut s1, card, ..
        } = goal(&mut state, "enemyPermanentsDestroyed", P1);

        let mut own = play_new(&mut s1, &slay().id, P1, &[mine.id.as_str()]);
        assert_eq!(progress_of(&own.state, &card, ONLY_QUEST), 0);
        let mut steal = play_new(&mut own.state, &slay().id, P1, &[stolen.id.as_str()]);
        let died = events_of_type(&steal.events, GameEventType::Destroyed)
            .first()
            .map(json_of);
        assert!(
            died.is_some_and(|died| matches_object(&died, &json!({ "owner": "p2", "controller": "p1" })))
        );
        assert_eq!(progress_of(&steal.state, &card, ONLY_QUEST), 0);
        let mut lent_step = play_new(&mut steal.state, &slay().id, P1, &[lent.id.as_str()]);
        let died = events_of_type(&lent_step.events, GameEventType::Destroyed)
            .first()
            .map(json_of);
        assert!(
            died.is_some_and(|died| matches_object(&died, &json!({ "owner": "p1", "controller": "p2" })))
        );
        assert_eq!(progress_of(&lent_step.state, &card, ONLY_QUEST), 1);
        // A token is a permanent too, and an ordinary death names its owner's side.
        let tok = play_new(&mut lent_step.state, &slay().id, P1, &[token.id.as_str()]);
        assert_eq!(
            field_of(&tok.events, GameEventType::Destroyed, "controller").first(),
            Some(&json!("p2"))
        );
        assert_eq!(completed(&tok.events, &card), [ONLY_QUEST]);
        assert_eq!(
            find_instance(&tok.state, &theirs.id).map(|card| card.zone.z()),
            Some(ZoneName::Field)
        );
    }

    #[test]
    fn r404_unspent_mana_at_turn_end_your_turns_end_with_enough_mana_left_not_less_and_not_the_opponents() {
        let plain_id = plain.id.clone();
        let mut state = quest_board("q-mana");
        in_hand(&mut state, &plain_id, P1, 1);
        in_hand(&mut state, &plain_id, P2, 1);
        let Played { state: s1, card, .. } = goal(&mut state, "unspentManaAtTurnEnd", P1);
        // 4 mana, 1 paid: 3 left.
        assert_eq!(
            i64::from(s1.players.p1.mana.current),
            goal_number(&GOALS["unspentManaAtTurnEnd"], "mana")
        );
        let ended = end_turn(&s1);
        assert_eq!(completed(&ended.events, &card), [ONLY_QUEST]);

        let mut state = quest_board("q-mana-short");
        in_hand(&mut state, &plain_id, P1, 1);
        in_hand(&mut state, &plain_id, P2, 1);
        state.players.p1.mana.current = goal_number(&GOALS["unspentManaAtTurnEnd"], "mana") as i32;
        let Played { state: s1, card, .. } = goal(&mut state, "unspentManaAtTurnEnd", P1);
        let mut short = end_turn(&s1);
        assert_eq!(progress_of(&short.state, &card, ONLY_QUEST), 0);
        // p2's turn ends with 4 unspent: not p1's turn.
        assert_eq!(short.state.active, P2);
        short.state.players.p2.mana.current = 4;
        let theirs = end_turn(&short.state);
        let unspent = events_of_type(&theirs.events, GameEventType::TurnEnded)
            .into_iter()
            .map(json_of)
            .find(|event| event["player"] == json!("p2"))
            .and_then(|event| event["unspentMana"].as_i64());
        assert!(unspent.is_some_and(|unspent| unspent >= 3));
        assert_eq!(progress_of(&theirs.state, &card, ONLY_QUEST), 0);
    }

    #[test]
    fn r404_damage_to_enemies_damage_your_cards_deal_to_the_enemy_hero_and_enemy_units_never_to_your_side_never_theirs()
     {
        let mut state = quest_board("q-damage");
        let target = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        let mine = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let Played {
            state: mut s1, card, ..
        } = goal(&mut state, "damageToEnemies", P1);
        let mut self_ = play_new(&mut s1, &bolt().id, P1, &["hero-p1"]);
        let mut own_unit = play_new(&mut self_.state, &bolt().id, P1, &[mine.id.as_str()]);
        assert_eq!(progress_of(&own_unit.state, &card, ONLY_QUEST), 0);
        let mut unit = play_new(&mut own_unit.state, &bolt().id, P1, &[target.id.as_str()]);
        assert_eq!(progress_of(&unit.state, &card, ONLY_QUEST), BOLT as i64);
        let hero = play_new(&mut unit.state, &bolt().id, P1, &["hero-p2"]);
        // 3 + 3 = 6, shown capped at the goal of 5.
        assert_eq!(
            progress_of(&hero.state, &card, ONLY_QUEST),
            goal_number(&GOALS["damageToEnemies"], "amount")
        );
        assert_eq!(completed(&hero.events, &card), [ONLY_QUEST]);
    }

    #[test]
    fn r404_r1361_damage_to_enemies_a_hit_armor_takes_whole_is_no_damage_and_counts_for_nothing() {
        let mut state = quest_board("q-damage-armor");
        let target = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        live_mut(&mut state, &target)
            .granted_keywords
            .push(Keyword::Armor { n: BOLT });
        let Played {
            state: mut s1, card, ..
        } = goal(&mut state, "damageToEnemies", P1);
        let blocked = play_new(&mut s1, &bolt().id, P1, &[target.id.as_str()]);
        // R63, R1361: the Armor took the bolt whole, which is reported and is no damage instance, so
        // the quest that counts damage to enemies has nothing to count.
        assert_eq!(
            events_of_type(&blocked.events, GameEventType::DamageAbsorbed).len(),
            1
        );
        assert!(events_of_type(&blocked.events, GameEventType::Damage).is_empty());
        assert_eq!(progress_of(&blocked.state, &card, ONLY_QUEST), 0);
    }

    #[test]
    fn r404_damage_to_enemies_a_unit_token_that_dies_in_the_combat_still_dealt_its_damage_the_opponents_hits_count_for_nothing()
     {
        let mut state = quest_board("q-damage-token");
        let token = put(&mut state, RUSH_TOKEN, slot(P1, Row::Units, 1), json!({}));
        let wall = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        live_mut(&mut state, &wall).damage = 0;
        let Played { state: s1, card, .. } = goal(&mut state, "damageToEnemies", P1);
        // The 3/3 Rush Token and the 3/3 trade: both die in the check after the combat, before dispatch.
        let mut fight = act(
            &s1,
            json!({ "type": "attack", "playerId": "p1", "attackerId": token.id, "targetId": wall.id }),
            None,
        );
        assert!(find_instance(&fight.state, &token.id).is_none());
        assert_eq!(progress_of(&fight.state, &card, ONLY_QUEST), 3);

        // The opponent's bolt on p1's hero, on the opponent's turn.
        in_hand(&mut fight.state, &plain.id, P1, 1);
        let mut turn = end_turn(&fight.state);
        let theirs = play_new(&mut turn.state, &bolt().id, P2, &["hero-p1"]);
        assert!(
            field_of(&theirs.events, GameEventType::Damage, "targetId")
                .iter()
                .any(|id| *id == json!("hero-p1"))
        );
        assert_eq!(progress_of(&theirs.state, &card, ONLY_QUEST), 3);
    }

    #[test]
    fn r404_r11_cards_exiled_either_players_card_entering_an_exile_pile_a_unit_token_ceases_to_exist_instead()
    {
        let mut state = quest_board("q-exile");
        let theirs = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        let mine = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let token = put(&mut state, RUSH_TOKEN, slot(P2, Row::Units, 2), json!({}));
        let Played {
            state: mut s1, card, ..
        } = goal(&mut state, "cardsExiled", P1);
        let mut tok = play_new(&mut s1, &banish().id, P1, &[token.id.as_str()]);
        assert_eq!(events_of_type(&tok.events, GameEventType::Exiled).len(), 1);
        assert_eq!(progress_of(&tok.state, &card, ONLY_QUEST), 0);
        let mut one = play_new(&mut tok.state, &banish().id, P1, &[theirs.id.as_str()]);
        assert_eq!(progress_of(&one.state, &card, ONLY_QUEST), 1);
        let two = play_new(&mut one.state, &banish().id, P1, &[mine.id.as_str()]);
        assert_eq!(completed(&two.events, &card), [ONLY_QUEST]);
    }

    #[test]
    fn r404_permanents_controlled_read_off_the_board_at_the_state_check_this_card_included_the_opponents_do_not_count()
     {
        let mut state = quest_board("q-board");
        put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));
        put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let Played {
            state: mut s1,
            events,
            card,
        } = goal(&mut state, "permanentsControlled", P1);
        assert!(completed(&events, &card).is_empty());
        let open = shown_to(&s1, P1, &card)
            .and_then(|shown| shown.quest)
            .map(|quest| json_of(&quest.open[0]));
        assert!(open.is_some_and(|open| matches_object(&open, &json!({ "progress": 2, "goal": 3 }))));
        let unit = hand(&mut s1, &plain.id, P1);
        let placed = act(
            &s1,
            json!({ "type": "play", "playerId": "p1", "instanceId": unit.id }),
            None,
        );
        assert_eq!(completed(&placed.events, &card), [ONLY_QUEST]);
    }

    #[test]
    fn r404_unit_totals_your_units_total_attack_and_total_health_at_once_through_the_layers() {
        let mut state = quest_board("q-stats");
        put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        let Played {
            state: mut s1,
            events,
            card,
        } = goal(&mut state, "unitTotals", P1);
        assert!(completed(&events, &card).is_empty());
        let unit = hand(&mut s1, &plain.id, P1);
        let placed = act(
            &s1,
            json!({ "type": "play", "playerId": "p1", "instanceId": unit.id }),
            None,
        );
        assert_eq!(completed(&placed.events, &card), [ONLY_QUEST]);
    }

    #[test]
    fn r404_units_in_graveyard_units_in_your_graveyard_by_type_a_spell_there_is_not_one() {
        let mut state = quest_board("q-grave");
        let a = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let Played {
            state: mut s1, card, ..
        } = goal(&mut state, "unitsInGraveyard", P1);
        let mut first = play_new(&mut s1, &slay().id, P1, &[a.id.as_str()]);
        // One Unit and a Slay (a Spell) lie there now.
        assert_eq!(first.state.players.p1.graveyard.len(), 2);
        assert_eq!(memory_of(&first.state, &card)["done"], json!([]));
        let b = put(&mut first.state, &plain.id, slot(P1, Row::Units, 2), json!({}));
        let second = play_new(&mut first.state, &slay().id, P1, &[b.id.as_str()]);
        assert_eq!(completed(&second.events, &card), [ONLY_QUEST]);
    }
}

mod e33_quests_the_tree_base_face_a_reward_prompt {
    use super::*;

    fn tree_after_first_quest(seed: &str) -> (GameState, CardInstance) {
        let mut state = quest_board(seed);
        let plain_id = plain.id.clone();
        set_library(&mut state, P1, &[plain_id.as_str(); 5]);
        let Played {
            state: mut s1, card, ..
        } = play_quest_card(&mut state, &tree().id, P1);
        let s2 = play_new(&mut s1, &draw_two().id, P1, &[]).state;
        (s2, card)
    }

    #[test]
    fn r404_a_completed_quest_is_a_reward_prompt_for_its_controller_its_options_the_rewards_on_offer() {
        let (mut state, card) = tree_after_first_quest("t-prompt");
        let pending = open_as(&state, PromptKind::Reward, P1);
        assert_eq!(
            pending
                .options
                .iter()
                .map(|o| (o.key.clone(), o.label.clone()))
                .collect::<Vec<_>>(),
            vec![
                ("mode:heal".to_string(), "Heal your hero 2".to_string()),
                ("mode:ask".to_string(), "Deal 1 damage to a target".to_string()),
            ]
        );
        assert!(matches_object(
            &memory_of(&state, &card),
            &json!({ "active": [], "done": ["draws"] })
        ));
        // The other seat sees that a prompt is open, never the options.
        assert_eq!(
            json_of(&view_for(&state, P2).pending),
            json!({ "forYou": false, "pendingFor": "p1" })
        );

        let health = state.players.p1.hero.health;
        assert_eq!(answer_keys(&mut state, &["mode:heal"]).error, None);
        assert_eq!(state.players.p1.hero.health, health + TREE_HEAL);
        assert!(matches_object(
            &memory_of(&state, &card),
            &json!({ "active": ["kill"], "done": ["draws"], "waiting": [] })
        ));
    }

    #[test]
    fn r404_a_reward_that_asks_a_question_pauses_survives_a_json_round_trip_and_opens_its_quest_after_the_answer()
     {
        let (mut state, card) = tree_after_first_quest("t-pause");
        let foe = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        answer_keys(&mut state, &["mode:ask"]);
        let aim = open_as(&state, PromptKind::Target, P1);
        assert!(
            aim.options
                .iter()
                .any(|o| o.key == format!("instance:{}", foe.id))
        );
        // Paused mid-reward: the quest it leads to is not open yet.
        assert_eq!(memory_of(&state, &card)["active"], json!([]));

        let mut copy = round_trip(&state);
        assert_eq!(hash_state(&copy), hash_state(&state));
        let key = format!("instance:{}", foe.id);
        answer_keys(&mut state, &[key.as_str()]);
        answer_keys(&mut copy, &[key.as_str()]);
        assert_eq!(hash_state(&copy), hash_state(&state));
        assert_eq!(
            find_instance(&state, &foe.id).map(|foe| foe.damage),
            Some(TREE_PING)
        );
        assert_eq!(memory_of(&state, &card)["active"], json!(["board"]));
    }

    #[test]
    fn r404_the_next_quest_counts_only_what_comes_after_it_opens_a_rewards_own_draws_come_first() {
        let mut state = quest_board("t-waiting");
        let plain_id = plain.id.clone();
        set_library(&mut state, P1, &[plain_id.as_str(); 8]);
        put(&mut state, &plain_id, slot(P1, Row::Units, 1), json!({}));
        let Played {
            state: mut s1, card, ..
        } = play_quest_card(&mut state, &tree().id, P1);
        // Quest 1 (draw 2) → "ask" → quest "board" (3 permanents: the tree, the unit, one more).
        let mut s = play_new(&mut s1, &draw_two().id, P1, &[]).state;
        answer_keys(&mut s, &["mode:ask"]);
        answer_keys(&mut s, &["hero:p2"]);
        assert_eq!(memory_of(&s, &card)["active"], json!(["board"]));
        s = play_new(&mut s, &plain_id, P1, &[]).state;
        // "board" completes: choose "hand" (draw 2) → "fresh" (draw 2), which the reward's draws do not feed.
        open_as(&s, PromptKind::Reward, P1);
        answer_keys(&mut s, &["mode:hand"]);
        assert!(matches_object(
            &memory_of(&s, &card),
            &json!({ "active": ["fresh"], "waiting": [] })
        ));
        assert_eq!(progress_of(&s, &card, "fresh"), 0);
        let more = play_new(&mut s, &draw_two().id, P1, &[]);
        assert_eq!(completed(&more.events, &card), ["fresh"]);
    }

    #[test]
    fn r404_completion_is_noticed_on_the_other_players_turn_and_the_prompt_is_the_cards_controllers() {
        let (mut state, _card) = tree_after_first_quest("t-their-turn");
        answer_keys(&mut state, &["mode:heal"]);
        in_hand(&mut state, &plain.id, P1, 1);
        let their_unit = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        let mut turn = end_turn(&state);
        assert_eq!(turn.state.active, P2);
        // p2 destroys its own unit: an enemy permanent of p1's, destroyed by anything.
        let killed = play_new(&mut turn.state, &slay().id, P2, &[their_unit.id.as_str()]);
        let pending = open_as(&killed.state, PromptKind::Reward, P1);
        assert_eq!(
            pending.options.iter().map(|o| o.key.clone()).collect::<Vec<_>>(),
            ["mode:both"]
        );
        assert_eq!(killed.state.active, P2);
    }

    #[test]
    fn r404_r78_an_aura_reward_holds_while_the_card_stands_and_leaving_the_field_ends_it_and_resets_the_line()
    {
        let mut state = quest_board("t-aura");
        in_hand(&mut state, &plain.id, P2, 1);
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let card = put(&mut state, &tree().id, slot(P1, Row::Backrow, 1), json!({}));
        live_mut(&mut state, &card).memory.insert(
            "quest".to_string(),
            json!({ "active": ["mana"], "progress": { "mana": 0 }, "done": ["draws", "kill"], "auras": [], "waiting": [] }),
        );
        let mut ended = end_turn(&state);
        // The end of p1's turn with 4 unspent completes "mana": its one reward is still a prompt.
        open_as(&ended.state, PromptKind::Reward, P1);
        answer_keys(&mut ended.state, &["mode:aura"]);
        let mut s = ended.state;
        assert!(matches_object(
            &memory_of(&s, &card),
            &json!({ "active": [], "auras": ["aura"] })
        ));
        assert!(has_indestructible(&s, &unit));
        // Both views show the quest line.
        for viewer in [P1, P2] {
            let shown = shown_to(&s, viewer, &card).map(json_of);
            assert!(shown.is_some_and(|shown| matches_object(
                &shown,
                &json!({ "quest": { "open": [], "auras": [{ "id": "aura", "text": "Aura: your Units have Indestructible" }] } })
            )));
        }

        // p2 exiles the tree on its turn: the aura goes with it, and the line resets.
        let gone = play_new(&mut s, &banish_any().id, P2, &[card.id.as_str()]);
        assert_eq!(
            find_instance(&gone.state, &card.id).map(|tree| tree.zone.z()),
            Some(ZoneName::Exile)
        );
        assert!(quest_memory_of(find_instance(&gone.state, &card.id).expect("tree")).is_none());
        assert!(!has_indestructible(&gone.state, &unit));
    }
}

/// Fuses `ingredients` onto `target`; the sink's cursor is not written back.
fn fuse_onto(
    state: &mut GameState,
    ingredients: Vec<CardInstance>,
    target: &CardInstance,
) -> Option<CardInstance> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    fuse(
        &mut sink,
        FuseArgs {
            ingredients,
            target: Some(target.clone()),
            ..FuseArgs::default()
        },
    )
}

mod r102_e33_quests_a_fused_quest_card_carries_its_tree {
    use super::*;

    #[test]
    fn r102_a_fuse_that_keeps_the_quest_card_keeps_its_line_and_the_fusion_carries_the_tree_it_counts_on_and_asks_its_reward()
     {
        let mut state = quest_board("t-fuse-kept");
        let plain_id = plain.id.clone();
        set_library(&mut state, P1, &[plain_id.as_str(); 3]);
        let card = put(&mut state, &tree().id, slot(P1, Row::Backrow, 1), json!({}));
        live_mut(&mut state, &card).memory.insert(
            "quest".to_string(),
            json!({ "active": ["draws"], "progress": { "draws": 1 }, "done": [], "auras": [], "waiting": [] }),
        );
        let food = put(&mut state, &blank().id, slot(P1, Row::Backrow, 2), json!({}));

        let fused = fuse_onto(&mut state, vec![food], &card).expect("a fusion");
        assert_eq!(
            (fused.id.clone(), fused.def_id == tree().id),
            (card.id.clone(), false)
        );
        assert_eq!(
            quest_book_of(&state, &fused).map(|book| book
                .quests
                .iter()
                .map(|quest| quest.id.clone())
                .collect::<Vec<_>>()),
            Some(
                TREE.quests
                    .iter()
                    .map(|quest| quest.id.clone())
                    .collect::<Vec<_>>()
            )
        );
        assert!(matches_object(
            &memory_of(&state, &card),
            &json!({ "active": ["draws"], "progress": { "draws": 1 } })
        ));

        let mut drawn = play_new(&mut state, &draw_one().id, P1, &[]);
        assert_eq!(completed(&drawn.events, &card), ["draws"]);
        assert_eq!(
            open_as(&drawn.state, PromptKind::Reward, P1)
                .options
                .iter()
                .map(|o| o.key.clone())
                .collect::<Vec<_>>(),
            ["mode:heal", "mode:ask"]
        );
        assert_eq!(answer_keys(&mut drawn.state, &["mode:heal"]).error, None);
        assert!(matches_object(
            &memory_of(&drawn.state, &card),
            &json!({ "active": ["kill"], "done": ["draws"] })
        ));
    }

    #[test]
    fn r102_a_quest_card_fused_onto_another_permanent_gives_the_fusion_its_tree_the_first_quest_opens_there_and_counts()
     {
        let mut state = quest_board("t-fuse-ingredient");
        let plain_id = plain.id.clone();
        set_library(&mut state, P1, &[plain_id.as_str(); 3]);
        let card = put(&mut state, &tree().id, slot(P1, Row::Backrow, 1), json!({}));
        let kept = put(&mut state, &blank().id, slot(P1, Row::Backrow, 2), json!({}));

        let fused = fuse_onto(&mut state, vec![card], &kept).expect("a fusion");
        assert_eq!(
            quest_book_of(&state, &fused).map(|book| book.first),
            Some(TREE.first.clone())
        );
        let drawn = play_new(&mut state, &draw_two().id, P1, &[]);
        assert_eq!(completed(&drawn.events, &kept), ["draws"]);
        open_as(&drawn.state, PromptKind::Reward, P1);
    }
}

mod e33_quests_the_tree_radiant_face_every_reward_every_path {
    use super::*;

    #[test]
    fn r404_a_completed_quest_grants_every_reward_it_offers_then_opens_every_quest_they_lead_to() {
        let mut state = quest_board("t-radiant");
        let plain_id = plain.id.clone();
        set_library(&mut state, P1, &[plain_id.as_str(); 3]);
        let mut card = hand(&mut state, &tree().id, P1);
        card.radiant = true;
        live_mut(&mut state, &card).radiant = true;
        let mut s1 = play(&state, &card, &[], None).state;
        let health = s1.players.p1.hero.health;
        let mut s = play_new(&mut s1, &draw_two().id, P1, &[]).state;
        // "heal" healed at once; "ask" asks its target with no reward prompt.
        assert_eq!(s.players.p1.hero.health, health + TREE_HEAL);
        open_as(&s, PromptKind::Target, P1);
        assert_eq!(memory_of(&s, &card)["active"], json!([]));
        answer_keys(&mut s, &["hero:p2"]);
        assert_eq!(s.players.p2.hero.health, HERO_HEALTH - TREE_PING);
        assert_eq!(memory_of(&s, &card)["active"], json!(["kill", "board"]));
    }

    #[test]
    fn r404_a_quest_reached_by_two_paths_opens_once_and_a_reward_two_completed_quests_offer_is_granted_by_each()
     {
        let mut state = quest_board("t-two-paths");
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        put(&mut state, &plain.id, slot(P1, Row::Units, 2), json!({}));
        let foe = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        let card = put(
            &mut state,
            &tree().id,
            slot(P1, Row::Backrow, 1),
            json!({ "radiant": true }),
        );
        // Both paths of quest 1 are open, as the Radiant face leaves them.
        live_mut(&mut state, &card).memory.insert(
            "quest".to_string(),
            json!({ "active": ["kill", "board"], "progress": { "kill": 0 }, "done": ["draws"], "auras": [], "waiting": [] }),
        );
        // The first check of the play finds 3 permanents ("board"), and the death it collects then
        // completes "kill": "both" granted by each, "mana" opened once, "hand" drawing 2 and opening "fresh".
        let plain_id = plain.id.clone();
        set_library(&mut state, P1, &[plain_id.as_str(); 4]);
        let step = play_new(&mut state, &slay().id, P1, &[foe.id.as_str()]);
        assert_eq!(completed(&step.events, &card), ["board", "kill"]);
        let buffed: Vec<Value> = events_of_type(&step.events, GameEventType::Buffed)
            .into_iter()
            .map(json_of)
            .collect();
        assert_eq!(buffed.len(), 2);
        assert!(
            buffed
                .iter()
                .all(|e| e["attack"] == json!(TREE_BUFF) && e["health"] == json!(TREE_BUFF))
        );
        let memory = memory_of(&step.state, &card);
        let active: Vec<&str> = memory["active"]
            .as_array()
            .expect("the open quests")
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(active.iter().filter(|id| **id == "mana").count(), 1);
        assert_eq!(active, ["mana", "fresh"]);
        assert!(find_instance(&step.state, &unit.id).is_some());
    }
}

mod e33_quests_the_view_hidden_information_and_replay {
    use super::*;

    #[test]
    fn r404_both_views_carry_the_open_quests_their_progress_and_the_rewards_on_offer_the_events_name_the_public_card()
     {
        let mut state = quest_board("t-view");
        let plain_id = plain.id.clone();
        set_library(&mut state, P1, &[plain_id.as_str(); 3]);
        let Played {
            state: mut s1, card, ..
        } = play_quest_card(&mut state, &tree().id, P1);
        let s = play_new(&mut s1, &draw_one().id, P1, &[]).state;
        for viewer in [P1, P2] {
            let view = view_for(&s, viewer);
            assert_eq!(
                json_of(shown_to(&s, viewer, &card).and_then(|shown| shown.quest)),
                json!({
                    "open": [
                        {
                            "id": "draws",
                            "text": "Draw 2 cards",
                            "progress": 1,
                            "goal": 2,
                            "rewards": [
                                { "id": "heal", "text": "Heal your hero 2" },
                                { "id": "ask", "text": "Deal 1 damage to a target" },
                            ],
                        },
                    ],
                    "auras": [],
                })
            );
            for id in field_of(&view.events, GameEventType::QuestProgressed, "instanceId") {
                assert_eq!(id, json!(card.id));
            }
        }
    }

    #[test]
    fn r404_a_game_through_a_quest_its_reward_and_the_next_quest_folds_from_its_log_to_the_same_hash_s9_2() {
        setup_catalog();
        register_quest_fixtures();
        let mut first_deck = vanilla_deck(DECK_SIZE - 2, 1);
        first_deck.extend([tree().id, draw_two().id]);
        let decks = (first_deck, vanilla_deck(DECK_SIZE, 21));
        let seed = "t-replay";
        let mut log: Vec<Action> = Vec::new();
        let mut state = begin_game(&create_game(&CreateGameArgs {
            seed: seed.to_string(),
            decks: decks.clone(),
            ..Default::default()
        }))
        .state;
        for player in [P1, P2] {
            let keep: Vec<String> = state.players[player].hand.iter().map(|c| c.id.clone()).collect();
            state = act(
                &state,
                json!({ "type": "mulligan", "playerId": player, "keep": keep }),
                Some(&mut log),
            )
            .state;
        }
        let tree_card = state
            .players
            .p1
            .hand
            .iter()
            .find(|c| c.def_id == tree().id)
            .expect("the tree in hand")
            .clone();
        state = play(&state, &tree_card, &[], Some(&mut log)).state;
        state = act(
            &state,
            json!({ "type": "endTurn", "playerId": "p1" }),
            Some(&mut log),
        )
        .state;
        state = act(
            &state,
            json!({ "type": "endTurn", "playerId": "p2" }),
            Some(&mut log),
        )
        .state;
        // p1's second turn drew a card: 1 of 2. Draw Two finishes it.
        assert_eq!(progress_of(&state, &tree_card, "draws"), 1);
        let twice = state
            .players
            .p1
            .hand
            .iter()
            .find(|c| c.def_id == draw_two().id)
            .expect("Draw Two in hand")
            .clone();
        state = play(&state, &twice, &[], Some(&mut log)).state;
        let pending = open_as(&state, PromptKind::Reward, P1);
        state = act(
            &state,
            json!({
                "type": "answer", "playerId": "p1", "choiceId": pending.id,
                "selection": [{ "pick": "mode", "option": "heal" }],
            }),
            Some(&mut log),
        )
        .state;
        assert_eq!(memory_of(&state, &tree_card)["active"], json!(["kill"]));

        let folded = fold(&FoldArgs {
            seed: seed.to_string(),
            decks,
            log,
            ..FoldArgs::default()
        });
        assert!(folded.errors.is_empty());
        assert_eq!(hash_state(&folded.state), hash_state(&state));
    }
}

mod r60_e33_the_random_picks_in_too_deeps_rewards_need_effects_random_picks_ts {
    use super::*;

    #[test]
    fn r60_buff_random_unit_one_of_your_units_drawn_from_the_match_rng_none_nothing() {
        let mut state = quest_board("rp-bless");
        let mut none = play_new(&mut state, &bless().id, P1, &[]);
        assert!(events_of_type(&none.events, GameEventType::Buffed).is_empty());
        let a = put(&mut none.state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let b = put(&mut none.state, &plain.id, slot(P1, Row::Units, 2), json!({}));
        put(&mut none.state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        let one = play_new(&mut none.state, &bless().id, P1, &[]);
        let buffed = field_of(&one.events, GameEventType::Buffed, "instanceId");
        assert_eq!(buffed.len(), 1);
        assert!([json!(a.id), json!(b.id)].contains(&buffed[0]));
    }

    #[test]
    fn r60_return_random_from_graveyard_n_different_cards_fewer_if_fewer_lie_there_a_full_hand_burning_them_back_once_each()
     {
        let plain_id = plain.id.clone();
        let mut state = quest_board("rp-recall");
        for _ in 0..3 {
            let card = new_instance(&mut state, &plain_id, P1, Zone::Graveyard { player: P1 });
            state.players.p1.graveyard.push(card);
        }
        let back = play_new(&mut state, &recall().id, P1, &[]);
        let added: Vec<Value> = events_of_type(&back.events, GameEventType::AddedToHand)
            .into_iter()
            .map(json_of)
            .filter(|e| e["defId"] == json!(plain_id))
            .collect();
        assert_eq!(added.len(), RECALL as usize);
        let distinct: IndexSet<String> = added.iter().map(|e| e["instanceId"].to_string()).collect();
        assert_eq!(distinct.len(), RECALL as usize);

        let mut full = quest_board("rp-recall-full");
        for _ in 0..3 {
            let card = new_instance(&mut full, &plain_id, P1, Zone::Graveyard { player: P1 });
            full.players.p1.graveyard.push(card);
        }
        in_hand(&mut full, &plain_id, P1, HAND_CAP as _);
        let burned = play_new(&mut full, &recall().id, P1, &[]);
        let burns = field_of(&burned.events, GameEventType::Burned, "instanceId");
        assert_eq!(burns.len(), RECALL as usize);
        let distinct: IndexSet<String> = burns.iter().map(Value::to_string).collect();
        assert_eq!(distinct.len(), RECALL as usize);
    }
}
