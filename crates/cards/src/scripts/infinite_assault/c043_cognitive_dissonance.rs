//! IA #43 Cognitive Dissonance (SPEC §8.13 row 43), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Reveals when your opponent plays a Unit: After it resolves, transform it into a
//!            random Unit whose Cost is ({step}) lower." (1)
//!   Radiant: "Reveals when your opponent plays a Unit: After it resolves, transform it into a
//!            random Unit whose Cost is ({step}) lower." (2)
//!
//! §10.5 step 7's `cardResolved`, after the Cry and every Echo repeat (R427; never a Recruit,
//! copy, token or Reborn: none of them a play, R1, R61). A `run` returning `[]` still consumes
//! the trap, so every condition lives in `when`. The new Unit is drawn from the shipped sets'
//! Units (R380) printed at the played card's printed cost (R65, R4577006) minus the step, keeping
//! the face (R4577006); with no Unit at that cost — the step reaching below (0) among them —
//! nothing changes (R129), and Immutable refuses it (R23).
//! (The wording is C2's tune: "whose Cost is ({step}) lower" for the step that sets it apart,
//! R1748003.)
//!
//! Engine: `transform_random` (R4586009). Built by C2 (#4586).

use jackioh_engine::prelude::*;
use jackioh_engine::query::left_field_since_resolved;

pub const ID: &str = "infiniteassault-043";

/// "Your opponent plays a Unit": the announce resolved into a Unit still answerable as the play
/// it was (Core #41's shape, R427, R174).
fn opponents_unit(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::CardResolved { player, permanent, .. } = event else {
        return false;
    };
    if *player == ctx.controller {
        return false;
    }
    if played_type_of(&*ctx.state, event) != Some(CardType::Unit) {
        return false;
    }
    *permanent || !left_field_since_resolved(&*ctx.state, event)
}

fn dissonance() -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "cognitive-dissonance",
            &[GameEventType::CardResolved],
            move |ctx, event| {
                let GameEvent::CardResolved {
                    instance_id,
                    def_id,
                    ..
                } = event
                else {
                    return vec![];
                };
                // R4577006: the played card's printed cost minus the step; below (0) is no pool
                // (R61, R129).
                let printed = query_cost(def_of(Some(&*ctx.state), def_id));
                let cost = printed - param(&*ctx, "step");
                if cost < 0 {
                    return vec![];
                }
                vec![transform_random(json_as(json!({
                    "instanceId": instance_id,
                    "query": { "type": "Unit", "cost": cost },
                    "radiant": "keep",
                })))]
            },
        )
        .with_when(opponents_unit)],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: dissonance(),
        // The same script: the Radiant face's (2) is its declared `step`.
        radiant: dissonance(),
    }
}

// IA #43 Cognitive Dissonance — SPEC §8.13 row 43 (R4586009).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P2;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const TWO: &str = "core-001"; // Big D-fender: (2).
    const ONE: &str = "core-008"; // Mr. Vanilla: (1).
    const THREE: &str = "stand-in (3) 5/5";

    fn armed(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "active": "p2",
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant }],
                "hand": [STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": {
                "hand": [TWO, ONE, THREE, STOCKPILE],
                "library": crate::filler(6),
            },
        }))
    }

    fn cost_of(s: &Scenario, lane: i32) -> i32 {
        let unit = s.unit(P2, lane).expect("the transformed Unit");
        query_cost(def_of(Some(s.state()), &unit.def_id))
    }

    mod base {
        use super::*;

        #[test]
        fn r4586009_a_played_2_becomes_a_random_1_keeping_its_face() {
            let mut s = armed(false, "dissonance-base");
            s.play(TWO, json!({ "zone": 1 }));
            s.expect_in_zone(ID, "graveyard");
            let unit = s.unit(P2, 1).expect("a Unit stands there");
            assert!(!unit.radiant, "the face is kept");
            assert_eq!(cost_of(&s, 1), 1);
        }

    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586009_a_played_3_becomes_a_random_1() {
            let mut s = armed(true, "dissonance-radiant");
            s.play(THREE, json!({ "zone": 1 }));
            s.expect_in_zone(ID, "graveyard");
            assert_eq!(cost_of(&s, 1), 1, "3 minus the Radiant step 2");
        }

        #[test]
        fn r4586009_below_0_nothing_changes() {
            let mut s = armed(true, "dissonance-floor");
            s.play(ONE, json!({ "zone": 1 }));
            s.expect_in_zone(ONE, "field");
            assert_eq!(s.unit(P2, 1).expect("still itself").def_id, ONE);
            s.expect_in_zone(ID, "graveyard");
        }
    }
}
