//! SPEC §8.12 S+ #77 Black Hole (§3.1, §3.2, §6.3 Open a Black Hole; R1, R386, R482, R1431, R1420,
//! R3535000, R3554000, R3554001, R3561000): (4) Spell, Mythic, Terminus, a card of the held
//! Stinkyscape+ set.
//!   Base:    "Open a Black Hole in midlane. At the start of each player's turn, it draws their permanents
//!            one lane closer. It closes after swallowing {permanents} permanents." (10)
//!   Radiant: "Open a Black Hole in midlane. At the start of the opponent's turn, it draws their
//!            permanents one lane closer."
//!
//! Its Cry (R1) is `open_black_hole`, the engine's: the hole stands in lane 3 of 5, exiles what stood
//! there and swallows what later enters, and draws each player's permanents one lane closer at the start
//! of their turn (R3554000, R3554001). The Radiant face is `opponentOnly`: it pulls only the opponent's
//! permanents, at the start of their turn. The hole closes after `permanents` swallowed, the number the
//! Spell resolved with, so a Degrade or an Upgrade of it moves the closing (R3561000, R386, R482). The
//! Radiant text leaves the closing out, but its face opens the same hole and closes at the printed 10,
//! which nothing moves: `permanents` is tuned on the base face only (R1431).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-077";

fn black_hole(opponent_only: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![open_black_hole(json_as(json!({
                "opponentOnly": opponent_only.then_some(true),
                "capacity": param(ctx, "permanents"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: black_hole(false),
        radiant: black_hole(true),
    }
}

// S+ #77 Black Hole — SPEC §8.12 row 77 (R3554000, R3554001, R3561000).
#[cfg(test)]
mod tests {
    use jackioh_engine::subsystems::ai_policy::choose_action;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const BODY: &str = "stand-in (1) 1/3";
    const TRAP: &str = "stand-in (1) Trap";

    fn lane(lane: i32) -> Value {
        json!({ "def": BODY, "lane": lane })
    }

    fn trap(lane: i32) -> Value {
        json!({ "def": TRAP, "lane": lane, "row": "backrow" })
    }

    fn board(radiant: bool, seed: &str, p1_field: Vec<Value>, p2_field: Vec<Value>) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, BODY],
                "field": p1_field,
                "library": filler(6),
                "mana": 10,
            },
            "p2": { "field": p2_field, "library": filler(6) },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    fn exiled(s: &Scenario, player: PlayerId) -> usize {
        s.pile(player, "exile").len()
    }

    fn lane_of(s: &Scenario, id: &str) -> Option<i32> {
        match &s.card(id).zone {
            Zone::Field { lane, .. } => Some(*lane),
            _ => None,
        }
    }

    #[test]
    fn r3554000_it_opens_in_midlane_and_swallows_what_stands_there() {
        let mut s = board(
            false,
            "c077-open",
            vec![lane(1), lane(3)],
            vec![lane(3), trap(3)],
        );
        let kept = s.unit_at(P1, 1).id;
        s.play(ID, json!({}));
        let hole = s.state().black_hole.expect("the hole is open");
        assert_eq!(hole.lane, BLACK_HOLE_LANE);
        assert_eq!(hole.pulls, None);
        assert_eq!(hole.swallowed_count(), 3);
        assert_eq!(exiled(&s, P1), 1);
        assert_eq!(exiled(&s, P2), 2);
        assert_eq!(lane_of(&s, &kept), Some(1), "lane 1 stands");
        assert!(s.unit(P1, 3).is_none() && s.unit(P2, 3).is_none());
        let shown = s.view(P2).black_hole.expect("the hole is public");
        assert_eq!(shown.capacity, 10);
    }

    #[test]
    fn r3554000_no_play_is_offered_into_midlane() {
        let mut s = board(false, "c077-no-play", vec![], vec![]);
        s.play(ID, json!({}));
        let body = s.hand(P1).iter().find(|card| card.def_id == BODY).expect("a body in hand").id.clone();
        let offered: Vec<i32> = legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play { instance_id, zone, .. } if instance_id == body => zone.map(|zone| zone.lane),
                _ => None,
            })
            .collect();
        assert!(!offered.is_empty(), "the other lanes stay open");
        assert!(!offered.contains(&BLACK_HOLE_LANE), "{offered:?}");
    }

    #[test]
    fn r3554001_each_players_turn_start_pulls_their_permanents_one_lane_closer() {
        let mut s = board(false, "c077-pull", vec![lane(1), lane(5)], vec![lane(2)]);
        let (left, right) = (s.unit_at(P1, 1).id, s.unit_at(P1, 5).id);
        let theirs = s.unit_at(P2, 2).id;
        s.play(ID, json!({}));
        s.end_turn();
        // p2's turn began: its lane 2 unit reached the hole and was swallowed; p1's stand still.
        assert_eq!(s.pile(P2, "exile").iter().filter(|card| card.id == theirs).count(), 1);
        assert_eq!(s.state().black_hole.map(|hole| hole.swallowed_count()), Some(1));
        assert_eq!((lane_of(&s, &left), lane_of(&s, &right)), (Some(1), Some(5)));
        s.end_turn();
        // p1's turn began: their permanents stepped one lane closer.
        assert_eq!((lane_of(&s, &left), lane_of(&s, &right)), (Some(2), Some(4)));
    }

    #[test]
    fn r3561000_it_closes_after_swallowing_its_permanents() {
        let mut s = board(false, "c077-closes", vec![lane(3)], vec![lane(3), trap(3)]);
        set_param(s.card_mut(ID), "permanents", 2);
        s.play(ID, json!({}));
        assert_eq!(s.state().black_hole, None, "it closed after two");
        assert_eq!(exiled(&s, P1), 1);
        assert_eq!(exiled(&s, P2), 1, "the second permanent of the lane went, the third stayed");
        assert!(card_at(s.state(), ZoneSlot { player: P2, row: Row::Backrow, lane: 3 }).is_some(), "the trap still stands");

        // An Upgraded Black Hole closes after its own number.
        let mut s = board(false, "c077-upgraded", vec![], vec![]);
        step_param(s.card_mut(ID), "permanents", 1);
        s.play(ID, json!({}));
        assert_eq!(s.state().black_hole.map(|hole| hole.closes_at()), Some(11));
    }

    #[test]
    fn r3554004_on_a_slashed_board_it_asks_its_half_and_still_opens() {
        let mut s = board(false, "c077-slashed", vec![], vec![]);
        s.state_mut().slashed_lane = Some(2);
        s.play(ID, json!({}));
        let pending = s.state().pending.as_ref().expect("its half is asked");
        assert_eq!(pending.kind, PromptKind::Mode);
        let answers = legal_actions(s.state(), P1)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Answer { .. }))
            .count();
        assert_eq!(answers, 2);
        let state = s.state().clone();
        let mut rng = Rng::new("c077-ai", 0);
        let body = choose_action(&state, P1, &mut rng).expect("the policy answers");
        assert!(matches!(body, ActionBody::Answer { .. }), "{body:?}");
        let done = reduce(&state, &Action::new(body, P1, "c077-ai-answer".to_string()));
        assert_eq!(done.error, None);
        assert!(done.state.black_hole.is_some() && done.state.pending.is_none());
        // The clock answers it too (R79).
        s.act(ActionBody::Timeout, P1, "the caster's clock runs out");
        assert!(s.state().black_hole.is_some() && s.state().pending.is_none());
    }

    #[test]
    fn r3554000_radiant_pulls_only_the_opponent_at_the_start_of_their_turn() {
        let mut s = board(true, "c077-radiant", vec![lane(1)], vec![lane(1)]);
        let (mine, theirs) = (s.unit_at(P1, 1).id, s.unit_at(P2, 1).id);
        s.play(ID, json!({}));
        assert_eq!(s.state().black_hole.and_then(|hole| hole.pulls), Some(P2));
        s.end_turn();
        assert_eq!(lane_of(&s, &theirs), Some(2));
        s.end_turn();
        assert_eq!(lane_of(&s, &mine), Some(1), "their own permanents are left alone");
    }

    #[test]
    fn r3561000_radiant_closes_at_the_printed_number() {
        let mut s = board(true, "c077-radiant-printed", vec![], vec![]);
        step_param(s.card_mut(ID), "permanents", 1);
        s.play(ID, json!({}));
        assert_eq!(
            s.state().black_hole.map(|hole| hole.closes_at()),
            Some(10),
            "tuned on the base face only (R1431)"
        );
    }
}
