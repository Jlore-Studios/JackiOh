//! S T-AI-2 Mining Subroutine (SPEC §7, §8.10 row T-AI-2; R482, R1840001, R1840002, R1881000): (0) Spell,
//! AI, Token (no printed rarity), an AI generated token of the held Stinkyscape set (R1420), built by S21.
//!   Base:    "Gain a random Common or Rare Stinkyscape Resource."
//!   Radiant: "Gain a random Radiant Common or Rare Stinkyscape Resource."
//! Engine: `gain_resources` (R1840002) from a pool of the Common and Rare Resources named by id: the
//!   rarity roll over the two rarities the pool holds (55 against 25), then one of the rolled rarity, into
//!   the tray, never the hand; one past the tray's cap is sold at once (R1840001). It never gains Axnit,
//!   Narcor or Red Narcor, so a bought copy never pays for itself in Credits (R1881000). Radiant: a
//!   Radiant Resource. Declares no numbers (R482).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-ai-02";

/// R1881000: the Common and Rare Resources (§8.10 T-reknite, T-korrelite, T-gellium).
const POOL: [&str; 3] = [
    "stinkyscape-t-reknite",
    "stinkyscape-t-korrelite",
    "stinkyscape-t-gellium",
];

/// "a random … Resource": one.
const GAINED: i32 = 1;

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![gain_resources(GainResourcesArgs {
                count: GAINED,
                player: None,
                radiant: radiant.then_some(true),
                pool: Some(POOL.iter().map(|id| (*id).to_string()).collect()),
                fill: None,
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S T-AI-2 Mining Subroutine — SPEC §8.10 row T-AI-2, BUILD M12 row S T-AI-2: "gains one random Resource
// into its caster's tray, always Reknite, Korrelite or Gellium, never Axnit, Narcor or Red Narcor (R1881000);
// a full tray sells it at once (R1840001); radiant a Radiant one".
#[cfg(test)]
mod tests {
    use super::POOL;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const MINING: &str = "stinkyscape-t-ai-02";
    const FILLER: &str = "core-005";

    fn cast(radiant: bool, seed: &str, tray: Value) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": MINING, "radiant": radiant }, FILLER], "tray": tray, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(MINING, json!({}));
        s
    }

    #[test]
    fn r1881000_it_gains_one_common_or_rare_resource_into_the_tray() {
        let mut seen: IndexSet<String> = IndexSet::new();
        for i in 0..40 {
            let s = cast(false, &format!("mining-{i}"), json!([]));
            let tray = resource_tray_of(s.state(), P1);
            assert_eq!(tray.len(), 1, "one Resource, in the tray");
            assert!(POOL.contains(&tray[0].def_id.as_str()), "{}", tray[0].def_id);
            assert!(!tray[0].radiant);
            seen.insert(tray[0].def_id.clone());
        }
        assert_eq!(seen.len(), POOL.len(), "each of the three over the seeds: {seen:?}");
    }

    #[test]
    fn r1840001_a_full_tray_sells_the_gain_at_once() {
        let full = json!(vec!["stinkyscape-t-reknite"; RESOURCE_TRAY_CAP as usize]);
        let s = cast(false, "mining-full", full);
        assert_eq!(resource_tray_of(s.state(), P1).len(), RESOURCE_TRAY_CAP as usize);
        assert!(market_credits_of(s.state(), P1) > 0, "the ninth sold for Credits");
    }

    #[test]
    fn radiant_a_radiant_resource() {
        let s = cast(true, "mining-radiant", json!([]));
        let tray = resource_tray_of(s.state(), P1);
        assert_eq!(tray.len(), 1);
        assert!(tray[0].radiant);
        assert!(POOL.contains(&tray[0].def_id.as_str()));
    }
}
