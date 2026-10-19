//! The Meditative+ batch "outside data and the autopilot" (issue #1183, docs/meditative-plus-set.md
//! MP5): the lowest win rate (R1183000), the two-pick fuse (R1183001), the autopilot (R1183002,
//! R1183003) and the code-block Discover (R1183005), each through `fixtures::outside_data`. The real
//! cards (M+ #2 NA Tech, #18 Cognitive Offloading, #25 KY's Intuition) cover the same cases again.
//! Beside them, the win-rate table a fold under a shipped catalog reads (R1266000, issue #1266).

use jackioh_engine::config::AUTOPILOT_MAX_ACTIONS;
use jackioh_engine::effects::{CODE_OFFERS_KEY, autopilot};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::outside_data::{
    FIRST_UNIT, SECOND_UNIT, ask_enemy, code_seer, fuse_same, fuse_two, offload, offload_on_draw, refill,
    register_outside_data, zap,
};

const FILLER: &str = "fx-10";

fn filler(count: usize) -> Value {
    json!(vec![FILLER; count])
}

fn table(cards: Vec<(&str, i32, i32)>) -> WinRateTable {
    WinRateTable {
        patch: "v0.3.X".to_string(),
        source: WinRateSource::Provisional,
        cards: cards
            .into_iter()
            .map(|(id, wins, games)| WinRateRow {
                id: id.to_string(),
                wins,
                games,
            })
            .collect(),
    }
}

/// The events of one type in the scenario's log.
fn count_of(s: &Scenario, type_: GameEventType) -> usize {
    s.events()
        .iter()
        .filter(|event| event.event_type() == type_)
        .count()
}

mod r1183000_the_lowest_win_rate {
    use super::*;

    #[test]
    fn r1183000_the_trailer_is_the_lowest_exact_rate_past_the_floor() {
        register_outside_data();
        register_win_rates(table(vec![
            ("fx-1", 30, 40),
            ("fx-2", 11, 40),
            ("fx-3", 10, 40),
            // A thin row never trails, however low.
            ("fx-4", 0, 19),
        ]));
        // 10/40 is below 11/40 by one win: compared exactly, never as a rounded float.
        assert_eq!(win_rate_trailer(&[]), Some("fx-3".to_string()));
        assert_eq!(win_rate_leader(""), Some("fx-1".to_string()));
    }

    #[test]
    fn r1183000_ties_go_to_more_games_then_the_catalog_id() {
        register_outside_data();
        register_win_rates(table(vec![("fx-1", 30, 40), ("fx-5", 5, 20), ("fx-6", 10, 40)]));
        // 5/20 and 10/40 tie on the rate: more games, as R1100's leader breaks it.
        assert_eq!(win_rate_trailer(&[]), Some("fx-6".to_string()));
        register_win_rates(table(vec![("fx-1", 30, 40), ("fx-7", 10, 40), ("fx-6", 10, 40)]));
        assert_eq!(win_rate_trailer(&[]), Some("fx-6".to_string()));
    }

    #[test]
    fn r1183000_the_trailer_skips_the_ids_it_is_given_tokens_and_missing_ids() {
        register_outside_data();
        register_win_rates(table(vec![
            ("fx-1", 30, 40),
            ("fx-token-rush", 0, 40),
            ("no-such-card", 0, 40),
        ]));
        // With the leader skipped, the one eligible row is gone: no trailer.
        assert_eq!(win_rate_trailer(&["fx-1"]), None);
        assert_eq!(win_rate_trailer(&[]), Some("fx-1".to_string()));
        register_win_rates(table(vec![]));
        assert_eq!(win_rate_trailer(&[]), None);
    }
}

mod r1266000_a_fold_reads_its_versions_table {
    use super::*;

    #[test]
    fn r1266000_a_fold_reads_the_table_its_shipped_catalog_carries_never_the_builds() {
        register_outside_data();
        register_win_rates(table(vec![("fx-1", 30, 40), ("fx-2", 10, 40)]));
        let empty: &'static ShippedCatalog = Box::leak(Box::new(ShippedCatalog::new(
            "v-empty",
            registered_catalog().clone(),
            IndexMap::new(),
        )));
        let kept: &'static ShippedCatalog = Box::leak(Box::new(
            ShippedCatalog::new("v-kept", registered_catalog().clone(), IndexMap::new())
                .with_win_rates(table(vec![("fx-2", 30, 40), ("fx-1", 10, 40)])),
        ));
        assert_eq!(win_rate_leader(""), Some("fx-1".to_string()));
        // A version that shipped with the empty table names no card, whatever the build's holds.
        assert_eq!(with_shipped(Some(empty), || win_rate_leader("")), None);
        // One that shipped with its own reads both ends off it.
        assert_eq!(
            with_shipped(Some(kept), || (win_rate_leader(""), win_rate_trailer(&[]))),
            (Some("fx-2".to_string()), Some("fx-1".to_string()))
        );
        assert_eq!(
            win_rate_leader(""),
            Some("fx-1".to_string()),
            "the build's again once the fold returns"
        );
        clear_overrides();
    }
}

mod r1183001_a_fuse_of_two_picks {
    use super::*;

    fn casting(seed: &str, spell: &str, fillers: usize) -> Scenario {
        register_outside_data();
        let mut hand = vec![json!(spell)];
        hand.extend(vec![json!(FILLER); fillers]);
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    #[test]
    fn r1183001_two_different_cards_fused_into_one_hand_card_at_its_set_cost() {
        for n in 0..6 {
            let mut s = casting(&format!("od-fuse-{n}"), &fuse_two.id, 1);
            s.play(fuse_two.id.as_str(), json!({}));
            let fused: Vec<CardInstance> = s
                .hand(P1)
                .into_iter()
                .filter(|card| card.def_id != FILLER)
                .collect();
            assert_eq!(fused.len(), 1, "one fused card in hand");
            let def = def_of(Some(s.state()), &fused[0].def_id);
            let mut names: Vec<&str> = def.name.split(" + ").collect();
            names.sort();
            assert_eq!(
                names,
                vec!["Fixture Unit 1", "Fixture Unit 2"],
                "the picks are {FIRST_UNIT} and {SECOND_UNIT}, never one twice"
            );
            assert_eq!(fused[0].cost_override, Some(2), "it costs (2)");
        }
    }

    #[test]
    fn r1183001_a_pick_only_an_earlier_pick_could_fill_fuses_nothing() {
        let mut s = casting("od-fuse-same", &fuse_same.id, 1);
        let before = s.hand(P1).len();
        s.play(fuse_same.id.as_str(), json!({}));
        assert_eq!(s.hand(P1).len(), before - 1, "nothing was added");
        assert_eq!(count_of(&s, GameEventType::Fused), 0);
    }

    #[test]
    fn r1183001_a_full_hand_burns_the_fused_card_with_no_price() {
        let mut s = casting("od-fuse-burn", &fuse_two.id, HAND_CAP as usize);
        s.play(fuse_two.id.as_str(), json!({}));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        let burned: Vec<&CardInstance> = s
            .state()
            .players
            .p1
            .graveyard
            .iter()
            .filter(|card| card.def_id != fuse_two.id)
            .collect();
        assert_eq!(burned.len(), 1, "the fused card burned");
        assert_eq!(burned[0].cost_override, None, "a burned card keeps no price");
    }
}

mod r1183002_the_autopilot {
    use super::*;

    /// p1 to act on turn 9 with `hand` and a ready 2/2 in lane 1; p2 with an empty board and a card
    /// to play, so their turn does not end by itself.
    fn board(seed: &str, hand: Value) -> Scenario {
        register_outside_data();
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "field": ["fx-3"], "library": filler(6), "mana": 4 },
            "p2": { "hand": ["fx-5"], "library": filler(6) },
        }))
    }

    #[test]
    fn r1183002_it_plays_the_rest_of_the_turn_and_ends_it() {
        let mut s = board("od-auto", json!([offload.id, "fx-1"]));
        let turn = s.state().turn;
        s.play(offload.id.as_str(), json!({}));
        assert!(s.events().iter().any(|event| matches!(
            event,
            GameEvent::AutopilotEngaged { player: P1, turn: t } if *t == turn
        )));
        // It played the 2/2 from hand, swung the ready 2/2 at the empty board's hero, and ended.
        assert!(
            (1..=5).any(|lane| s.unit(P1, lane).is_some_and(|unit| unit.def_id == "fx-1")),
            "the hand's Unit was played"
        );
        s.expect_health("p2", HERO_HEALTH - 2);
        assert_eq!(s.state().active, P2, "the turn was ended");
        assert_eq!(
            s.state().players[P1].autopilot,
            None,
            "the status ended with the turn"
        );
    }

    #[test]
    fn r1183002_on_the_opponents_turn_it_does_nothing() {
        register_outside_data();
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        let mut events = Vec::new();
        {
            let state = s.state_mut();
            let (seed, cursor) = (state.seed.clone(), state.rng_cursor);
            let mut rng = Rng::new(&seed, cursor);
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..HookOptions::default()
                },
            );
            (autopilot().apply)(&mut ctx);
        }
        assert!(events.is_empty(), "no autopilot, no event");
        assert_eq!(s.state().players[P1].autopilot, None);
        assert_eq!(s.state().players[P2].autopilot, None);
    }

    #[test]
    fn r1183002_drawn_at_the_start_of_its_turn_it_plays_that_turn() {
        register_outside_data();
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "library": filler(4) },
            "p2": { "hand": ["fx-5"], "library": [offload_on_draw.id, FILLER, FILLER, FILLER] },
        }));
        s.end_turn();
        assert!(
            s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::AutopilotEngaged { player: P2, .. }))
        );
        assert!(
            (1..=5).any(|lane| s.unit(P2, lane).is_some_and(|unit| unit.def_id == "fx-5")),
            "the autopilot played p2's Unit"
        );
        assert_eq!(s.state().active, P1, "and ended p2's turn");
    }

    #[test]
    fn r1183002_the_seat_waits_locked_out_while_the_other_player_answers() {
        let mut s = board("od-auto-wait", json!([offload.id, ask_enemy.id]));
        s.play(offload.id.as_str(), json!({}));
        // The autopilot played the question for p2 and waits on its answer, still p1's turn.
        let pending = s.state().pending.clone().expect("p2's question is open");
        assert_eq!(pending.player_id, P2);
        assert_eq!(s.state().active, P1);
        assert!(s.state().players[P1].autopilot.is_some());
        // p1's own actions are refused meanwhile, its end of turn among them.
        s.expect_refused_with(|s| s.end_turn(), "prompt");
        assert_eq!(
            legal_actions(s.state(), P1)
                .iter()
                .filter(|action| action.action_type() != ActionType::Concede)
                .count(),
            0,
            "p1 may only concede"
        );
        // p2's answer hands the turn back to the autopilot, which ends it.
        s.answer(json!("left"));
        assert_eq!(s.state().active, P2);
        assert_eq!(s.state().players[P1].autopilot, None);
    }

    #[test]
    fn r1183002_the_status_is_absent_from_a_game_without_it() {
        register_outside_data();
        let s = scenario(json!({ "p1": { "hand": [FILLER] }, "p2": { "hand": [FILLER] } }));
        let text = serde_json::to_string(s.state()).expect("serialises");
        assert!(!text.contains("autopilot"), "D9: no key, no hash change");
    }

    #[test]
    fn r1183002_both_seats_read_the_autopilot_event() {
        let mut s = board("od-auto-view", json!([offload.id]));
        s.play(offload.id.as_str(), json!({}));
        for viewer in [P1, P2] {
            let events = serde_json::to_value(view_for(s.state(), viewer).events).expect("serialises");
            assert!(
                events.to_string().contains("\"autopilotEngaged\""),
                "{viewer:?} reads it"
            );
        }
    }
}

mod r1183003_what_the_autopilot_takes {
    use super::*;

    #[test]
    fn r1183003_its_own_prompt_is_answered_by_the_best_outcome() {
        register_outside_data();
        let mut s = scenario(json!({
            "seed": "od-zap",
            "p1": { "hand": [offload.id, zap.id], "library": filler(6), "mana": 4 },
            "p2": { "hand": ["fx-5"], "field": ["fx-6"], "library": filler(6) },
        }));
        s.play(offload.id.as_str(), json!({}));
        // 3 damage kills p2's only 2/2 (the enemy board cleared), before any other aim.
        assert!(s.unit(P2, 1).is_none(), "p2's Unit was the target");
        s.expect_health("p1", HERO_HEALTH);
        assert_eq!(s.state().active, P2);
    }

    #[test]
    fn r1183003_the_bound_hands_the_rest_of_the_turn_to_a_timeout() {
        register_outside_data();
        let mut s = scenario(json!({
            "seed": "od-bound",
            "p1": { "hand": [offload.id, refill.id], "library": filler(6) },
            "p2": { "hand": ["fx-5"], "library": filler(6) },
        }));
        s.play(offload.id.as_str(), json!({}));
        let refills = s
            .events()
            .iter()
            .filter(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if *def_id == refill.id))
            .count();
        assert_eq!(
            refills, AUTOPILOT_MAX_ACTIONS as usize,
            "one action each, up to the bound"
        );
        assert_eq!(s.state().active, P2, "then the turn ended");
    }

    #[test]
    fn r1183003_the_same_state_plays_the_same_turn() {
        let run = || {
            let mut s = super::autopilot_board("od-same");
            s.play(offload.id.as_str(), json!({}));
            (hash_state(s.state()), s.events().to_vec())
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn r1183003_each_choice_is_a_legal_action() {
        let s = super::autopilot_board("od-legal");
        let mut state = s.state().clone();
        state.players[P1].autopilot = Some(Autopilot {
            turn: state.turn,
            actions: 0,
        });
        let chosen = jackioh_engine::subsystems::autopilot::choose(&state, P1).expect("an action");
        assert!(legal_actions(&state, P1).contains(&chosen));
        // It never concedes, activates, switches or emotes.
        assert!(matches!(
            chosen.action_type(),
            ActionType::Play | ActionType::Attack | ActionType::Answer | ActionType::EndTurn
        ));
    }
}

/// A board for the autopilot's determinism and legality cases: a Unit and a target prompt in hand, a
/// ready Unit on each side.
fn autopilot_board(seed: &str) -> Scenario {
    register_outside_data();
    scenario(json!({
        "seed": seed,
        "p1": { "hand": [offload.id, "fx-1", zap.id], "field": ["fx-3"], "library": filler(6), "mana": 4 },
        "p2": { "hand": ["fx-5"], "field": ["fx-6"], "library": filler(6) },
    }))
}

mod r1183005_the_code_discover {
    use super::*;

    const BLOCKS: [(&str, &str); 4] = [
        ("fx-1", "fn one() {}"),
        ("fx-2", "fn two() {}"),
        ("fx-3", "fn three() {}"),
        ("fx-4", "fn four() {}"),
    ];

    fn seeing(seed: &str, blocks: &[(&str, &str)]) -> Scenario {
        register_outside_data();
        register_code_blocks(blocks.to_vec());
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [code_seer.id, FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(code_seer.id.as_str(), json!({}));
        s
    }

    #[test]
    fn r1183005_three_distinct_cards_each_shown_as_its_code_alone() {
        let s = seeing("od-code", &BLOCKS);
        let pending = s.state().pending.clone().expect("the Discover is open");
        assert_eq!(pending.kind, PromptKind::Discover);
        assert_eq!(pending.options.len(), 3);
        let codes: Vec<&str> = BLOCKS.iter().map(|(_, code)| *code).collect();
        let mut seen: Vec<String> = Vec::new();
        for option in &pending.options {
            assert!(
                codes.contains(&option.label.as_str()),
                "the label is a code block"
            );
            assert!(!seen.contains(&option.label), "three different cards");
            seen.push(option.label.clone());
        }

        // The chooser reads the code and nothing that names the card; the opponent reads nothing.
        let mine = serde_json::to_value(view_for(s.state(), P1).pending).expect("serialises");
        for option in mine["options"].as_array().expect("options") {
            assert_eq!(option["code"], json!(true));
            assert!(option.get("defId").is_none());
            assert!(
                option["key"]
                    .as_str()
                    .is_some_and(|key| key.starts_with("mode:code-"))
            );
        }
        for (id, _) in BLOCKS {
            assert!(!mine.to_string().contains(id), "no id in the chooser's prompt");
        }
        let theirs = serde_json::to_value(view_for(s.state(), P2).pending).expect("serialises");
        assert_eq!(theirs["forYou"], json!(false));
        assert!(theirs.get("options").is_none());
    }

    #[test]
    fn r1183005_the_pick_goes_to_the_hand_radiant() {
        let mut s = seeing("od-code-pick", &BLOCKS);
        let pending = s.state().pending.clone().expect("the Discover is open");
        let behind = pending.resume.data[CODE_OFFERS_KEY][1]
            .as_str()
            .expect("the second place's card")
            .to_string();
        s.answer(json!("code-1"));
        let added: Vec<CardInstance> = s
            .hand(P1)
            .into_iter()
            .filter(|card| card.def_id != FILLER)
            .collect();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].def_id, behind);
        assert!(added[0].radiant);
    }

    #[test]
    fn r1183005_only_cards_with_a_code_block_are_offered() {
        let s = seeing("od-code-two", &BLOCKS[..2]);
        let pending = s.state().pending.clone().expect("the Discover is open");
        assert_eq!(pending.options.len(), 2);
    }
}
