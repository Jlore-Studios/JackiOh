//! SPEC §8.12 S+ #31 Soul Read, a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    (1) Trap; "Cry: Look at your opponent's hand and choose a card secretly. Reveals
//!            if your opponent plays the chosen card before any of the other cards you saw.
//!            Take control of that card instead. Otherwise, Tribute this Trap."
//!   Radiant: (1) Trap; "Cry:" the same pick; "Reveals if your opponent plays the chosen card.
//!            Take control of that card instead."
//!
//! Engine: E6's secret pick (R3550008): the Cry opens the opponent's hand to its owner alone,
//! who secretly chooses one card; the pick and every card seen are kept on the Trap's own
//! instance memory, so no view and no event ever names them. When the opponent plays the
//! chosen card first, the announce is countered into its owner's hand; when they play
//! another seen card first, the base face tributes itself instead. The Radiant face answers
//! the chosen card alone and waits for it. Only its owner ever sees the hand, and only
//! during the pick.
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-031";

/// Soul Read's answer to an announce: the chosen card is stolen into its owner's hand, another
/// seen card tributes the base trap. The Radiant face answers the chosen card alone.
fn soul_read(radiant: bool) -> Script {
    let trigger = TriggerDef::new("s31-soul-read", &[GameEventType::CardAnnounced], |ctx, event| {
        let Some(card) = ctx.self_.clone() else {
            return Vec::new();
        };
        match secret_pick_hit(&card, event) {
            Some(SecretPickHit::Chosen) => match event {
                GameEvent::CardAnnounced { instance_id, .. } => {
                    vec![counter_play(json_as(json!({
                        "to": "thief",
                        "target": { "of": "instance", "instanceId": instance_id },
                    })))]
                }
                _ => Vec::new(),
            },
            Some(SecretPickHit::OtherSeen) => {
                vec![sacrifice(json_as(json!({ "target": { "of": "self" } })))]
            }
            None => Vec::new(),
        }
    });
    let trigger = if radiant {
        trigger.with_when(|ctx, event| {
            ctx.self_
                .as_ref()
                .is_some_and(|card| secret_pick_hit(card, event) == Some(SecretPickHit::Chosen))
        })
    } else {
        trigger.with_when(|ctx, event| {
            ctx.self_
                .as_ref()
                .is_some_and(|card| secret_pick_hit(card, event).is_some())
        })
    };
    Script {
        cry: Some(hook(|_ctx| vec![secret_pick(json_as(json!({ "step": "pick" })))])),
        resume: IndexMap::from([("pick", hook(|_ctx| vec![keep_secret_pick()]))]),
        triggers: vec![trigger],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: soul_read(false),
        radiant: soul_read(true),
    }
}

// S+ #31 Soul Read — SPEC §8.12 row 31 (R3550008).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::subsystems::ai_policy::policy_actions;
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PLAIN: &str = "stand-in (1) 1/1";
    const RANGER: &str = "stand-in (2) 2/2";

    /// Soul Read set with p2 holding two cards: the pick prompt opens for p1 alone.
    fn reading(seed: &str, radiant: bool) -> Scenario {
        let mut s = weapon_board(
            seed,
            json!({
                "hand": [{ "def": ID, "radiant": radiant }],
                "library": filler(4),
            }),
            json!({ "hand": [PLAIN, RANGER], "library": filler(4) }),
        );
        let trap = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == ID)
            .expect("the trap")
            .id;
        s.play(trap.as_str(), json!({ "zone": 1 }));
        s
    }

    fn option_ids(view: &PlayerView) -> Vec<String> {
        match view.pending.as_ref() {
            Some(PendingView::ForYou(prompt)) => prompt
                .options
                .iter()
                .filter_map(|option| option.instance_id.clone())
                .collect(),
            _ => Vec::new(),
        }
    }

    fn hand_ids(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player).iter().map(|card| card.id.clone()).collect()
    }

    fn countered(s: &Scenario) -> bool {
        s.last_events()
            .iter()
            .any(|event| matches!(event, GameEvent::Countered { .. }))
    }

    #[test]
    fn only_its_owner_sees_the_pick_and_the_ai_and_a_timeout_answer_it() {
        for radiant in [false, true] {
            let s = reading("s031-pick", radiant);
            let seen = hand_ids(&s, P2);
            assert_eq!(seen.len(), 2);
            // Its owner sees the opponent's hand as the options...
            assert_eq!(option_ids(&s.view(P1)), seen);
            // ...and only then: the opponent's view names none of them.
            assert!(option_ids(&s.view(P2)).is_empty());
            // Every option is a legal answer for the owner, and the opponent may only concede.
            let answers = legal_actions(s.state(), P1)
                .into_iter()
                .filter(|action| matches!(action, ActionBody::Answer { .. }))
                .count();
            assert_eq!(answers, seen.len());
            assert_eq!(legal_actions(s.state(), P2), vec![ActionBody::Concede]);
            // The AI policy answers the pick too.
            let policy = policy_actions(s.state(), P1, Default::default());
            assert_eq!(
                policy
                    .iter()
                    .filter(|action| matches!(action, ActionBody::Answer { .. }))
                    .count(),
                seen.len()
            );
            // A timeout answers through the AI policy.
            let timed_out = reduce(s.state(), &Action::new(ActionBody::Timeout, P1, "s031-t"));
            assert_eq!(timed_out.error, None);
            assert!(
                timed_out.state.pending.is_none(),
                "the clock answers the pick through the AI policy"
            );
        }
    }

    #[test]
    fn no_view_tells_which_card_was_chosen() {
        for radiant in [false, true] {
            let mut first = reading("s031-hidden", radiant);
            let mut second = reading("s031-hidden", radiant);
            let plain = hand_ids(&first, P2)[0].clone();
            let ranger = hand_ids(&second, P2)[1].clone();
            first.answer(json!([plain]));
            second.answer(json!([ranger]));
            for viewer in [P1, P2] {
                assert_eq!(
                    serde_json::to_value(first.view(viewer)).expect("the view serializes"),
                    serde_json::to_value(second.view(viewer)).expect("the view serializes"),
                    "no {viewer:?} view tells which card was chosen"
                );
            }
        }
    }

    #[test]
    fn the_chosen_card_played_first_is_stolen() {
        for radiant in [false, true] {
            let mut s = reading("s031-steal", radiant);
            let plain = hand_ids(&s, P2)[0].clone();
            s.answer(json!([plain.clone()]));
            s.end_turn();
            s.play(plain.as_str(), json!({ "zone": 1 }));
            assert!(countered(&s), "the chosen card's play is countered");
            let stolen = s
                .hand(P1)
                .into_iter()
                .find(|card| card.def_id == PLAIN)
                .expect("the chosen card is stolen into its owner's hand");
            assert_eq!(stolen.owner, P1);
        }
    }

    #[test]
    fn another_seen_card_first_tributes_the_base_trap() {
        let mut s = reading("s031-tribute", false);
        let seen = hand_ids(&s, P2);
        let trap = s.backrow(P1, 1).expect("the set trap").id;
        s.answer(json!([seen[0].clone()]));
        s.end_turn();
        // Another seen card played first: the base trap is tributed, the play resolves.
        s.play(seen[1].as_str(), json!({ "zone": 1 }));
        s.expect_in_zone(trap.as_str(), "graveyard");
    }

    #[test]
    fn the_radiant_trap_waits_for_the_chosen_card() {
        let mut s = reading("s031-waits", true);
        let seen = hand_ids(&s, P2);
        let trap = s.backrow(P1, 1).expect("the set trap").id;
        s.answer(json!([seen[0].clone()]));
        s.end_turn();
        // Another seen card played first: the Radiant trap stays set, the play resolves.
        s.play(seen[1].as_str(), json!({ "zone": 1 }));
        s.expect_in_zone(trap.as_str(), "field");
        s.end_turn();
        s.end_turn();
        // The chosen card played later is still stolen; lane 1 holds the other one.
        s.play(seen[0].as_str(), json!({ "zone": 2 }));
        assert!(countered(&s), "the chosen card's play is countered");
        assert!(
            s.hand(P1).iter().any(|card| card.def_id == PLAIN),
            "the chosen card is stolen into its owner's hand"
        );
    }
}
