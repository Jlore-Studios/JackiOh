//! Stinkyscape #6 Tylannt (SPEC §8.10 row 6). Unit 6/4 → 12/8, cost 2, Acclaimed, Human, Mythic.
//!   Base:    "Tradable
//!            Cry: Complete the current stage on all your Quests."
//!   Radiant: "Tradable
//!            Cry: Summon The Greatest Heist. Complete the current stage on all your Quests."
//!
//! Tradable is a catalog keyword the engine plays (R1838000). Cry (R1): `complete_quests` (R1854001),
//! every Quest-tagged card its controller has acting on the field, in lane order, has the quests open on
//! it completed, a staged quest's current stage and C #90 In Too Deep's open quests alike, each reward
//! given at the next state check; a line hearing back or done has nothing open, and the opponent's
//! Quests and an untagged card are passed by. R1873000: the Radiant face first summons S #7 The
//! Greatest Heist into its controller's leftmost open backrow zone (R64). The summon names it, so its
//! Cry (its Discover) runs there, as League of Losers' Mid Loser's does (R411), a face-up Field Spell's
//! Cry being triggered as a Unit's is (R1873003); its first stage, opened as it entered, is then

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-006";

/// S #7 The Greatest Heist.
const HEIST: &str = "stinkyscape-007";

/// R411, R1873000: the Heist this Cry summoned, if it is still there, has its Cry triggered.
fn heist_cry() -> Effect {
    for_each_card(ForEachCardArgs {
        cards: Arc::new(|ctx: &mut EffectContext<'_>| {
            summoned_so_far(ctx)
                .into_iter()
                .filter(|id| find_instance(&*ctx.state, id).is_some_and(|card| card.def_id == HEIST))
                .collect()
        }),
        each: Arc::new(|instance_id: &str| trigger_cry(json_as(json!({ "instanceId": instance_id })))),
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| vec![complete_quests()])),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| {
                vec![summon(json_as(json!({ "defId": HEIST }))), heist_cry(), complete_quests()]
            })),
            ..Script::default()
        },
    }
}

// Stinkyscape #6 Tylannt — SPEC §8.10 row 6, BUILD M12 row S 6 (R1838000, R1854001, R1873000,
// R411).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008"; // (1) 4/4.
    const MANA_WELL: &str = "core-006"; // (1) Field Spell.
    const CALL_TO_ARMS: &str = "core-069"; // (2) Spell: Recruit Units of (1) Cost or less; Radiant (2).
    const IN_TOO_DEEP: &str = "classic-090"; // (1) Field Spell, a Quest: quest 1 "Draw 2 cards" opens as it enters.
    const JOB_APPLICATION: &str = "stinkyscape-112"; // A staged Quest whose Stage 1 reward hears back.
    const SEND: &str = "send-resume";
    const FIRST_QUEST: &str = "1";

    const MANA: i32 = 10;

    fn heists(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [HEIST, { "def": ID, "radiant": radiant }], "library": filler(8), "mana": MANA },
            "p2": { "hand": [HEIST], "library": filler(8) },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s.play(HEIST, json!({ "zone": 1 }));
        answer_first(&mut s);
        s.end_turn();
        s.state_mut().players.p2.mana.current = MANA;
        s.play(HEIST, json!({ "zone": 1 }));
        answer_first(&mut s);
        s.end_turn();
        s.state_mut().players.p1.mana.current = MANA;
        s
    }

    fn answer_first(s: &mut Scenario) {
        let pending = s.state().pending.clone().expect("the Heist's Discover");
        let first = match &pending.options[0].selection {
            Selection::Mode { option } => option.clone(),
            other => panic!("a Discover option, not {other:?}"),
        };
        s.answer(json!(first));
    }

    fn stage_of(s: &Scenario, player: PlayerId, lane: usize) -> Option<u32> {
        let heist = s.state().players[player].backrow[lane - 1].as_ref()?;
        subsystems::quest_memory_of(heist).and_then(|line| line.stage)
    }

    #[test]
    fn r1854001_its_cry_completes_the_current_stage_of_your_quests_never_the_opponents() {
        let mut s = heists(false, "tylannt");
        assert_eq!((stage_of(&s, P1, 1), stage_of(&s, P2, 1)), (Some(1), Some(1)));
        s.play(ID, json!({ "zone": 1 }));
        assert_eq!(stage_of(&s, P1, 1), Some(2));
        assert_eq!(stage_of(&s, P2, 1), Some(1));
        assert!(
            s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::QuestCompleted { player: P1, .. }))
        );
    }

    /// p1's main phase with `hand` (each turn ended by the test, R82), p2 holding a card to play.
    fn holding(seed: &str, hand: Value) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": filler(8), "mana": MANA },
            "p2": { "hand": [VANILLA], "library": filler(8) },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    fn in_hand(s: &Scenario, def_id: &str) -> Vec<String> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id == def_id)
            .map(|card| card.id)
            .collect()
    }

    fn backrow_id(s: &Scenario, def_id: &str) -> String {
        s.state()
            .players
            .p1
            .backrow
            .iter()
            .flatten()
            .find(|card| card.def_id == def_id)
            .unwrap_or_else(|| panic!("{def_id} in p1's backrow"))
            .id
            .clone()
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

    #[test]
    fn r1838000_both_faces_are_tradable_in_hand_one_trade_a_turn() {
        for radiant in [false, true] {
            let mut s = holding(
                "tylannt-trade",
                json!([{ "def": ID, "radiant": radiant }, { "def": ID, "radiant": radiant }]),
            );
            let tylannts = in_hand(&s, ID);
            assert!(
                is_tradable(s.state(), s.card(tylannts[0].as_str())),
                "radiant {radiant}: Tradable"
            );
            let (hand, mana) = (s.hand(P1).len(), s.state().players.p1.mana.current);
            s.trade(tylannts[0].as_str());
            s.expect_events(json!(["traded", "shuffledIn", "drawn"]));
            assert_eq!(
                s.hand(P1).len(),
                hand,
                "one card went into the deck and one was drawn"
            );
            assert_eq!(s.state().players.p1.mana.current, mana, "at no mana");
            assert_eq!(trades_left(s.state(), P1), 0);
            s.expect_refused(|s| s.trade(tylannts[1].as_str()));
            assert!(s.hand(P1).iter().any(|card| card.id == tylannts[1]));
            s.end_turn().end_turn();
            s.trade(tylannts[1].as_str());
            s.expect_events(json!(["traded", "shuffledIn", "drawn"]));
        }
    }

    #[test]
    fn r1854001_in_too_deep_s_open_quest_is_completed_too() {
        let mut s = holding("tylannt-in-too-deep", json!([IN_TOO_DEEP, ID]));
        s.play(IN_TOO_DEEP, json!({}));
        let tree = backrow_id(&s, IN_TOO_DEEP);
        assert!(completed(s.events(), &tree).is_empty(), "nothing drawn yet");
        s.play(ID, json!({ "zone": 1 }));
        assert_eq!(completed(s.last_events(), &tree), vec![FIRST_QUEST.to_string()]);
        // Its base face asks for its reward, as any completion of it does (R404).
        let pending = s.state().pending.clone().expect("In Too Deep's reward prompt");
        assert_eq!((pending.kind, pending.player_id), (PromptKind::Reward, P1));
    }

    #[test]
    fn r1854001_a_line_hearing_back_is_untouched() {
        let mut s = holding("tylannt-hearing-back", json!([JOB_APPLICATION, VANILLA, ID]));
        s.play(JOB_APPLICATION, json!({}));
        let application = backrow_id(&s, JOB_APPLICATION);
        let resume = in_hand(&s, VANILLA)[0].clone();
        s.activate(
            application.as_str(),
            json!({ "ability": SEND, "targets": pick::id(&resume) }),
        );
        let waiting = subsystems::quest_memory_of(s.card(application.as_str())).expect("its line");
        assert!(waiting.hear_back.is_some(), "Stage 1's reward hears back");
        s.play(ID, json!({ "zone": 1 }));
        assert!(completed(s.last_events(), &application).is_empty());
        assert_eq!(
            subsystems::quest_memory_of(s.card(application.as_str())),
            Some(waiting),
            "the wait is untouched"
        );
    }

    #[test]
    fn r1_recruited_it_completes_nothing() {
        let mut s = scenario(json!({
            "seed": "tylannt-recruited",
            "p1": {
                "hand": [HEIST, { "def": CALL_TO_ARMS, "radiant": true }],
                "library": [ID, VANILLA],
                "mana": MANA,
            },
            "p2": { "hand": [VANILLA], "library": filler(8) },
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        s.play(HEIST, json!({ "zone": 1 }));
        answer_first(&mut s);
        s.play(CALL_TO_ARMS, json!({}));
        assert!(s.unit(P1, 1).is_some_and(|unit| unit.def_id == ID) || s.unit(P1, 2).is_some_and(|unit| unit.def_id == ID));
        assert_eq!(stage_of(&s, P1, 1), Some(1));
    }

    #[test]
    fn r1873000_radiant_summons_the_greatest_heist_runs_its_cry_then_completes_its_first_stage_with_the_rest() {
        let mut s = heists(true, "tylannt-radiant");
        let hand = s.state().players.p1.hand.len();
        s.play(ID, json!({ "zone": 1 }));
        // The summon names the Heist, so its Cry runs first (R411): its Discover is asked, with the
        let pending = s.state().pending.clone().expect("the summoned Heist's Discover");
        assert_eq!(pending.player_id, P1);
        assert_eq!(pending.prompt, "Discover an Acclaimed Unit or a Jelly Bean");
        assert!(s.state().players.p1.backrow[1].as_ref().is_some_and(|card| card.def_id == HEIST));
        assert_eq!(stage_of(&s, P1, 1), Some(1), "the Heist that stood there");
        answer_first(&mut s);
        assert!(s.state().pending.is_none());
        assert_eq!(s.state().players.p1.hand.len(), hand);
        assert_eq!(stage_of(&s, P1, 1), Some(2), "the Heist that stood there");
        assert_eq!(stage_of(&s, P1, 2), Some(2), "the Heist it summoned");
        assert_eq!(stage_of(&s, P2, 1), Some(1));
    }

    #[test]
    fn r1873000_radiant_with_a_full_backrow_summons_nothing_and_completes_the_rest() {
        let mut s = scenario(json!({
            "seed": "tylannt-full",
            "p1": {
                "hand": [HEIST, { "def": ID, "radiant": true }],
                "backrow": [MANA_WELL, MANA_WELL, MANA_WELL, MANA_WELL],
                "library": filler(8),
                "mana": MANA,
            },
            "p2": { "hand": [VANILLA], "library": filler(8) },
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        s.play(HEIST, json!({ "zone": 5 }));
        answer_first(&mut s);
        s.play(ID, json!({ "zone": 1 }));
        assert!(s.state().pending.is_none(), "no Heist summoned, no Discover");
        assert_eq!(stage_of(&s, P1, 5), Some(2));
        let heists = s
            .state()
            .players
            .p1
            .backrow
            .iter()
            .flatten()
            .filter(|card| card.def_id == HEIST)
            .count();
        assert_eq!(heists, 1);
    }
}
