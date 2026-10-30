//! The Meditative+ batch "outside data and the autopilot" (issue #1183, docs/meditative-plus-set.md
//! MP5): the lowest win rate (R1183000), the two-pick fuse (R1183001) and the code-block Discover
//! (R1183005), each through `fixtures::outside_data`; the autopilot's tests (R1183002, R1183003) are
//! `autopilot.rs`'s, the subsystem's own home. The real cards (M+ #2 NA Tech, #25 KY's Intuition)
//! cover the same cases again.
//! Beside them, the win-rate table a fold under a shipped catalog reads (R1266000, issue #1266).

use jackioh_engine::effects::CODE_OFFERS_KEY;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::outside_data::{
    FIRST_UNIT, SECOND_UNIT, code_seer, fuse_same, fuse_two, register_outside_data,
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
        let mine = serde_json::to_value(view_for(s.state(), P1).pending).expect("serializes");
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
        let theirs = serde_json::to_value(view_for(s.state(), P2).pending).expect("serializes");
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
