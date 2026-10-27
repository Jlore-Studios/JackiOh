//! Stinkyscape #6 Tylannt (SPEC §8.10 row 6). Unit 6/4 → 12/8, cost 2, Acclaimed, Human, Mythic; held
//! (R1420).
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
//! Greatest Heist into its controller's leftmost open backrow zone (R64). A summon is no play, so the
//! Heist's Cry (its Discover) does not run (R1; R411's triggered Cry reaches Units alone, B2.7); its first
//! stage, opened as it entered, is then completed with the rest. A full backrow summons nothing, and the
//! completion reaches the other Quests alone.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-006";

/// S #7 The Greatest Heist.
const HEIST: &str = "stinkyscape-007";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| vec![complete_quests()])),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| {
                vec![summon(json_as(json!({ "defId": HEIST }))), complete_quests()]
            })),
            ..Script::default()
        },
    }
}

// Stinkyscape #6 Tylannt — SPEC §8.10 row 6, BUILD M12 row S 6: "Tradable in hand (one Trade a turn, R1838000);
// Cry (played): each Quest of yours on the field has its open quest completed and pays its reward at the next
// state check, In Too Deep's included, the opponent's untouched, a line hearing back untouched (R1854001);
// radiant 12/8, S #7 summoned first, with no Cry (R1873000), its Stage 1 opened and completed". Tradable,
// In Too Deep's line and a line hearing back are the engine's, proven by its tests (R1838000, R1854001).
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

    /// Enough to play anything a test asks.
    const MANA: i32 = 10;

    /// p1 holds Tylannt (Radiant on `radiant`); The Greatest Heist stands in each player's backrow, each
    /// line at Stage 1 (its Cry's Discover answered).
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

    /// The stage `player`'s Heist in backrow lane `lane` stands at.
    fn stage_of(s: &Scenario, player: PlayerId, lane: usize) -> Option<u32> {
        let heist = s.state().players[player].backrow[lane - 1].as_ref()?;
        subsystems::quest_memory_of(heist).and_then(|line| line.stage)
    }

    #[test]
    fn r1854001_its_cry_completes_the_current_stage_of_your_quests_never_the_opponents() {
        let mut s = heists(false, "tylannt");
        assert_eq!((stage_of(&s, P1, 1), stage_of(&s, P2, 1)), (Some(1), Some(1)));
        s.play(ID, json!({ "zone": 1 }));
        // Stage 1's reward summoned an Acclaimed Unit, and Stage 2 is open.
        assert_eq!(stage_of(&s, P1, 1), Some(2));
        assert_eq!(stage_of(&s, P2, 1), Some(1));
        assert!(
            s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::QuestCompleted { player: P1, .. }))
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
    fn r1873000_radiant_summons_the_greatest_heist_without_its_cry_and_completes_its_first_stage_with_the_rest() {
        let mut s = heists(true, "tylannt-radiant");
        s.play(ID, json!({ "zone": 1 }));
        // A summon is no play: the summoned Heist asks no Discover (R1).
        assert!(s.state().pending.is_none());
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
