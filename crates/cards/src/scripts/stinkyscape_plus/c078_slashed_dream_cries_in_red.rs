//! SPEC §8.12 S+ #78 Slashed Dream Cries in Red (§3.1, §3.2, §6.3 Slash the board; R1, R1420,
//! R1748003, R3535000, R3554002, R3554003, R3554004, R3561001): (4) Spell, Mythic, Terminus, a card of the held
//! Stinkyscape+ set.
//!   Base:    "Slash the board into two."
//!   Radiant: "Slash the board into two. You have control over the slash: lane 2, 3 or 4."
//!
//! Its Cry (R1) is `slash_board`, the engine's: lane 3 (`SLASH_BASE_LANE`) is exiled with every
//! permanent in it and voided for the rest of the game, the board is two halves, and a second slash
//! does nothing (R3554002, R3554003). The Radiant face first asks its caster for the lane, a `mode`
//! prompt over `SLASH_LANES` (`ask_slash_lane`), so `legal_actions`, the AI, the random policy and the
//! timeout answer it too; the answer comes back to `slashLane` (§10.6), and a missing one slashes
//! lane 3. The card declares no numbers: its lanes 2, 3 and 4 are the engine's table, which
//! `tests/cross/params.rs` names (R1748003). Cast on an already slashed board it asks its half first,
//! which changes nothing for it (R3554004); the Radiant face's own slash asks none afterwards (R3561001).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-078";

/// The step the Radiant face's lane pick comes back to (§10.6).
const SLASH_STEP: &str = "slashLane";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| vec![slash_board(json_as(json!({})))])),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| vec![ask_slash_lane(SLASH_STEP)])),
            resume: IndexMap::from([(
                SLASH_STEP,
                hook(|ctx| vec![slash_board(json_as(json!({ "lane": chosen_slash_lane(ctx) })))]),
            )]),
            ..Script::default()
        },
    }
}

// S+ #78 Slashed Dream Cries in Red — SPEC §8.12 row 78 (R3554002, R3554003, R3554004).
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

    fn answers(s: &Scenario) -> usize {
        legal_actions(s.state(), P1)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Answer { .. }))
            .count()
    }

    #[test]
    fn r3554002_it_exiles_the_middle_lane_and_voids_it() {
        let mut s = board(
            false,
            "c078-slash",
            vec![lane(2), lane(3)],
            vec![lane(3), trap(3)],
        );
        let kept = s.unit_at(P1, 2).id;
        s.play(ID, json!({}));
        assert_eq!(s.state().slashed_lane, Some(SLASH_BASE_LANE));
        assert_eq!(exiled(&s, P1), 1);
        assert_eq!(exiled(&s, P2), 2);
        assert!(s.unit(P1, 3).is_none() && s.unit(P2, 3).is_none());
        assert_eq!(s.unit_at(P1, 2).id, kept, "lane 2 stands");
        assert_eq!(s.view(P1).slashed_lane, Some(3));
        assert_eq!(s.view(P2).slashed_lane, Some(3));
        let body = s.hand(P1).iter().find(|card| card.def_id == BODY).expect("a body in hand").id.clone();
        let offered: Vec<i32> = legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play { instance_id, zone, .. } if instance_id == body => zone.map(|zone| zone.lane),
                _ => None,
            })
            .collect();
        assert!(!offered.is_empty() && !offered.contains(&SLASH_BASE_LANE), "{offered:?}");
    }

    #[test]
    fn r3554003_the_board_is_two_halves_and_a_unit_attacks_only_its_own() {
        let mut s = board(false, "c078-halves", vec![lane(1)], vec![lane(2), lane(4)]);
        let (attacker, near, far) = (s.unit_at(P1, 1).id, s.unit_at(P2, 2).id, s.unit_at(P2, 4).id);
        s.play(ID, json!({}));
        assert_eq!(half_of_lane(s.state(), 1), Some(Half::Left));
        assert_eq!(half_of_lane(s.state(), 4), Some(Half::Right));
        assert_eq!(half_of_lane(s.state(), SLASH_BASE_LANE), None);
        let attacks = |target: &str| {
            legal_actions(s.state(), P1).iter().any(|action| {
                matches!(
                    action,
                    ActionBody::Attack { attacker_id, target_id }
                        if *attacker_id == attacker && target_id == target
                )
            })
        };
        assert!(attacks(&near), "its own half");
        assert!(!attacks(&far), "across the slash");
    }

    #[test]
    fn r3554002_a_second_slash_changes_nothing() {
        let mut s = board(false, "c078-second", vec![lane(3)], vec![]);
        let kept = s.unit_at(P1, 3).id;
        s.state_mut().slashed_lane = Some(2);
        s.play(ID, json!({}));
        // On a slashed board a Spell asks its half first (R3554004), which changes nothing here.
        s.answer(json!("left"));
        assert_eq!(s.state().slashed_lane, Some(2));
        assert_eq!(s.unit_at(P1, 3).id, kept, "lane 3 was not slashed");
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3554002_radiant_picks_the_lane_to_slash() {
            let mut s = board(true, "c078-pick", vec![lane(2), lane(3)], vec![]);
            let (two, three) = (s.unit_at(P1, 2).id, s.unit_at(P1, 3).id);
            s.play(ID, json!({}));
            let pending = s.state().pending.as_ref().expect("the lane pick is open");
            assert_eq!(pending.kind, PromptKind::Mode);
            let options = s.prompt_modes();
            assert_eq!(options.len(), pending.options.len(), "a mode each");
            let lanes: Vec<String> = SLASH_LANES.iter().map(|lane| lane.to_string()).collect();
            assert_eq!(options, lanes);
            assert_eq!(s.state().slashed_lane, None, "nothing is slashed before the answer");
            s.answer(json!("4"));
            assert_eq!(s.state().slashed_lane, Some(4));
            assert_eq!(s.unit_at(P1, 2).id, two);
            assert_eq!(s.unit_at(P1, 3).id, three, "lane 3 stands: it was not the one slashed");
            assert_eq!(half_of_lane(s.state(), 3), Some(Half::Left));
        }

        #[test]
        fn r3561001_the_board_it_slashed_asks_no_half_afterwards() {
            let mut s = board(true, "c078-no-half", vec![lane(1)], vec![lane(5)]);
            s.play(ID, json!({}));
            s.answer(json!("3"));
            assert_eq!(s.state().slashed_lane, Some(3));
            assert!(s.state().pending.is_none(), "{:?}", s.state().pending);
            assert_eq!(s.view(P1).slashed_lane, Some(3));
        }

        #[test]
        fn r3554002_every_answerer_answers_the_lane_pick() {
            let mut s = board(true, "c078-answerers", vec![], vec![]);
            s.play(ID, json!({}));
            assert_eq!(answers(&s), 3, "`legal_actions` offers each lane");
            let state = s.state().clone();
            let mut rng = Rng::new("c078-policy", 0);
            let body = choose_action(&state, P1, &mut rng).expect("the policy answers");
            assert!(matches!(body, ActionBody::Answer { .. }), "{body:?}");
            let next = reduce(&state, &Action::new(body, P1, "c078-policy-answer".to_string()));
            assert_eq!(next.error, None);
            assert!(next.state.pending.is_none(), "{:?}", next.state.pending);
            assert!(SLASH_LANES.contains(&next.state.slashed_lane.unwrap_or(0)));

            // The clock answers it (R79) on a fresh board.
            let mut s = board(true, "c078-timeout", vec![], vec![]);
            s.play(ID, json!({}));
            s.act(ActionBody::Timeout, P1, "the caster's clock runs out");
            assert!(s.state().pending.is_none());
            assert!(SLASH_LANES.contains(&s.state().slashed_lane.unwrap_or(0)));
        }
    }
}
