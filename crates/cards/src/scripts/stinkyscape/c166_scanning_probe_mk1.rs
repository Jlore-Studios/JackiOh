//! SPEC §8.10 Stinkyscape 166 Scanning Probe Mk1 (row S 166; §2.4, §6.1 Lucky, §6.3; R987, R1438,
//! R1855003). (1) Spell, Common, a card of the Stinkyscape set.
//!   Base:    "5% chance to add a Radiant Anomaly Found! to your hand. / 20% chance to add an
//!            Anomaly Found! to your hand instead. / Otherwise, your future Scanning Probes have +1% and
//!            +4% to these chances."
//!   Radiant: "Lucky 1", and the same text.
//!
//! Engine: `scan_for_anomaly` (#1855's Anomaly scan, R1855003), one roll of 100 against its
//! controller's chances, 5 for a Radiant Anomaly Found! and 20 more for a plain one, each earlier miss
//! of that player's adding 1 and 4, the count per player and shared by both Probes (ST-S20-1); a find
//! steps are the engine's (`ANOMALY_*`, R1748003).

use jackioh_engine::prelude::*;

use super::t_anomaly_found;

pub const ID: &str = "stinkyscape-166";

/// R2476000: in its owner's hand, the two chances its scan rolls at now (`anomaly_chances` of their
/// misses so far, which both Probes share): after "5% chance" the Radiant one, after "20% chance" the
/// plain one, each label the printed number (`ANOMALY_RADIANT_CHANCE`, `ANOMALY_PLAIN_CHANCE`). A miss
/// is silent, so the count stays in its owner's hand, never on a public card. S #167 shows the same.
pub fn probe_preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if ctx.zone != ConditionZone::Hand {
            return vec![];
        }
        let (radiant, plain) = anomaly_chances(anomaly_misses_of(ctx.state, ctx.controller));
        vec![
            count_preview(&format!("{ANOMALY_RADIANT_CHANCE}% chance"), radiant),
            count_preview(&format!("{ANOMALY_PLAIN_CHANCE}% chance"), plain),
        ]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| {
            vec![scan_for_anomaly(ScanForAnomalyArgs {
                def_id: t_anomaly_found::ID.to_string(),
            })]
        })),
        preview: Some(probe_preview()),
        ..Script::default()
    };
    // The same script: the Radiant face's Lucky 1 is its printed keyword, which the scan reads.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #166 Scanning Probe Mk1 — SPEC §8.10 row S 166, BUILD M12 row S 166: "one roll: a Radiant Anomaly Found! below 5, a plain one below 25, else nothing seen and the next scan's bands 6 and 24; a full hand burns the find; radiant Lucky 1, two rolls, the best kept" (R1855003).
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::anomaly_chances;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const PROBE: &str = "stinkyscape-166";
    const OTHER_PROBE: &str = "stinkyscape-167";
    const ANOMALY: &str = "stinkyscape-t-anomaly-found";
    const FILLER: &str = "core-005";
    const SEEDS: u32 = 400;

    fn setup(seed: &str, hand: Value, misses: Option<i32>) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": [FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        s.state_mut().players[P1].anomaly_misses = misses;
        s
    }

    fn finds(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id == ANOMALY).collect()
    }

    fn scan(seed: &str, radiant: bool, misses: Option<i32>) -> (Option<bool>, Option<i32>) {
        let mut s = setup(seed, json!([{ "def": PROBE, "radiant": radiant }]), misses);
        s.play(PROBE, json!({}));
        let found = finds(&s);
        assert!(found.len() <= 1, "one roll finds at most one");
        (
            found.first().map(|card| card.radiant),
            s.state().players[P1].anomaly_misses,
        )
    }

    fn rates(radiant: bool) -> (u32, u32) {
        let (mut shiny, mut plain) = (0, 0);
        for seed in 0..SEEDS {
            match scan(&format!("probe-166-{radiant}-{seed}"), radiant, None) {
                (Some(true), None) => shiny += 1,
                (Some(false), None) => plain += 1,
                (None, Some(1)) => {}
                other => panic!("a find keeps the count, a miss adds one: {other:?}"),
            }
        }
        (shiny, plain)
    }

    #[test]
    fn r1855003_at_95_misses_every_scan_finds_a_radiant_anomaly_found_and_keeps_the_count() {
        for radiant in [false, true] {
            assert_eq!(
                scan("probe-166-sure", radiant, Some(95)),
                (Some(true), Some(95)),
                "radiant {radiant}"
            );
        }
    }

    #[test]
    fn r1855003_the_misses_are_shared_with_the_other_probe() {
        let seed = (0..SEEDS)
            .map(|seed| format!("probe-166-shared-{seed}"))
            .find(|seed| scan(seed, false, None).0.is_none())
            .expect("a seed whose scan misses");
        let mut s = setup(&seed, json!([PROBE, OTHER_PROBE]), None);
        s.play(PROBE, json!({}));
        assert_eq!(s.state().players[P1].anomaly_misses, Some(1), "the miss counts");
        // At 95 misses the other probe always finds; at 1 its bands are 6 and 24 (R1855003).
        assert_eq!(anomaly_chances(1), (6, 24));
        s.state_mut().players[P1].anomaly_misses = Some(95);
        s.play(OTHER_PROBE, json!({}));
        assert_eq!(finds(&s).len(), 1, "the other probe scans on the same count");
    }

    /// R2476000: in hand, after "5% chance" and "20% chance", the chances its scan rolls at: 5 and 20,
    /// then 6 and 24 after one miss; the opponent reads nothing of the hand.
    #[test]
    fn r2476000_in_hand_it_previews_the_chances_its_misses_have_grown_to() {
        for radiant in [false, true] {
            let mut s = setup("probe-166-preview", json!([{ "def": PROBE, "radiant": radiant }, FILLER]), None);
            let probe = s.card(PROBE).id.clone();
            let chances = |radiant: i32, plain: i32| {
                vec![("5% chance".to_string(), radiant, None), ("20% chance".to_string(), plain, None)]
            };
            assert_eq!(crate::preview_seen(&s, P1, &probe), chances(5, 20));
            s.state_mut().players[P1].anomaly_misses = Some(1);
            assert_eq!(crate::preview_seen(&s, P1, &probe), chances(6, 24));
            assert!(crate::preview_seen(&s, PlayerId::P2, &probe).is_empty(), "radiant {radiant}");
        }
    }

    #[test]
    fn r1855003_a_full_hand_burns_the_find() {
        let mut hand = vec![json!(PROBE)];
        hand.extend((0..HAND_CAP).map(|_| json!(FILLER)));
        let mut s = setup("probe-166-burn", Value::Array(hand), Some(95));
        s.play(PROBE, json!({}));
        assert!(finds(&s).is_empty(), "nothing reaches a full hand");
        assert!(
            s.last_events()
                .iter()
                .any(|event| matches!(event, GameEvent::Burned { .. })),
            "the find burns"
        );
    }

    mod base {
        use super::*;

        #[test]
        fn r1855003_one_roll_finds_a_radiant_one_at_5_percent_and_a_plain_one_at_20() {
            let (shiny, plain) = rates(false);
            assert!((6..=36).contains(&shiny), "{shiny} Radiant finds in {SEEDS}");
            assert!((56..=106).contains(&plain), "{plain} plain finds in {SEEDS}");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1855003_lucky_1_rolls_twice_and_keeps_the_best() {
            let (shiny, plain) = rates(true);
            assert!((145..=205).contains(&(shiny + plain)), "{} finds in {SEEDS}", shiny + plain);
            assert!((21..=58).contains(&shiny), "{shiny} Radiant finds in {SEEDS}");
        }
    }
}
