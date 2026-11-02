//! S T-AI-6 Probe Relay (SPEC §7, §8.10 row T-AI-6; §2.4, R65, R482): (1) Spell, AI, Token (no printed
//! rarity), an AI generated token of the Stinkyscape set, built by S21.
//!   Base:    "Add 2 Scanning Probe Mk1s to your hand. They cost (0)."
//!   Radiant: "Add 2 Scanning Probe Mk2s to your hand. They cost (0)."
//! Engine: two `add_to_hand`s of S #166 Scanning Probe Mk1 (Radiant: S #167 Scanning Probe Mk2), named, so
//!   they reach the hand whether or not the set ships, each at a `costOverride` of 0 (R65); a full hand burns
//!   what does not fit (§2.4). Declares no numbers (R482).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-ai-06";

/// S #166 Scanning Probe Mk1 and S #167 Scanning Probe Mk2.
const PROBE_MK1: &str = "stinkyscape-166";
const PROBE_MK2: &str = "stinkyscape-167";
/// "Add 2", "They cost (0)".
const PROBES: usize = 2;
const PROBE_COST: i32 = 0;

fn face(probe: &'static str) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            (0..PROBES)
                .map(|_| {
                    add_to_hand(AddToHandArgs {
                        def_id: Some(probe.to_string()),
                        cost_override: Some(PROBE_COST),
                        ..AddToHandArgs::default()
                    })
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(PROBE_MK1),
        radiant: face(PROBE_MK2),
    }
}

// S T-AI-6 Probe Relay — SPEC §8.10 row T-AI-6, BUILD M12 row S T-AI-6.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const RELAY: &str = "stinkyscape-t-ai-06";
    const PROBE_MK1: &str = "stinkyscape-166";
    const PROBE_MK2: &str = "stinkyscape-167";
    const FILLER: &str = "core-005";

    fn cast(radiant: bool, fillers: usize) -> Scenario {
        let mut hand = vec![json!({ "def": RELAY, "radiant": radiant })];
        hand.extend((0..fillers).map(|_| json!(FILLER)));
        let mut s = scenario(json!({
            "seed": "probe-relay",
            "p1": { "hand": hand, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(RELAY, json!({}));
        s
    }

    fn probes(s: &Scenario, def_id: &str) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id == def_id).collect()
    }

    #[test]
    fn it_adds_two_scanning_probe_mk1s_that_cost_0() {
        let s = cast(false, 1);
        let added = probes(&s, PROBE_MK1);
        assert_eq!(added.len(), 2);
        for probe in &added {
            assert_eq!(effective_cost(s.state(), probe, CostOptions::default()), 0);
            assert!(!probe.radiant);
        }
    }

    #[test]
    fn s2_4_a_full_hand_burns_what_does_not_fit() {
        let s = cast(false, 9);
        assert_eq!(probes(&s, PROBE_MK1).len(), 1);
        assert!(s.last_events().iter().any(|event| matches!(event, GameEvent::Burned { def_id, .. } if def_id == PROBE_MK1)));
    }

    #[test]
    fn radiant_two_scanning_probe_mk2s_that_cost_0() {
        let s = cast(true, 1);
        assert!(probes(&s, PROBE_MK1).is_empty());
        let added = probes(&s, PROBE_MK2);
        assert_eq!(added.len(), 2);
        assert!(added.iter().all(|probe| effective_cost(s.state(), probe, CostOptions::default()) == 0));
    }
}
