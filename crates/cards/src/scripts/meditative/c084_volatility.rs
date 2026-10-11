//! M #84 Volatility (SPEC §8.8 row 84): (1) Unit, Common, 3/3 → 6/6.
//!
//! Base:    "Buffs and Nerfs are twice as effective on this."
//! Radiant: "Buffs are three times as effective on this."
//! Engine: NEW: ME-TUNEMULT (R1160) — a static flag `tuneMultiplier`, read off the card's running
//! face wherever it is (field, hand, deck), by which each Buff or Nerf application on this card is
//! scaled: still one application and one draw, with the change N times as large. "Twice" and
//! "three times" are kept out of `params`, so a Buff can never raise its own multiplier.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-084";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            static_flags: Some(StaticFlags {
                tune_multiplier: Some(TuneMultiplier {
                    upgrade: Some(VOLATILITY_TUNE_MULTIPLIER),
                    degrade: Some(VOLATILITY_TUNE_MULTIPLIER),
                }),
                ..StaticFlags::default()
            }),
            ..Script::default()
        },
        radiant: Script {
            static_flags: Some(StaticFlags {
                tune_multiplier: Some(TuneMultiplier {
                    upgrade: Some(VOLATILITY_RADIANT_BUFF_MULTIPLIER),
                    degrade: None,
                }),
                ..StaticFlags::default()
            }),
            ..Script::default()
        },
    }
}

// M #84 Volatility — SPEC §8.8 row 84, BUILD M10 row M 84: "One Buff on it is one draw whose change
// is doubled (MD-E7): a cost row of two steps (never below (0)), a stat split of twice the total,
// two keywords, X +2; a Nerf likewise doubled, never removing a harmful keyword; still one
// `upgraded` or `degraded` event; in hand and deck too; a +2/+2 stat buff and KY's Constant are not
// doubled; radiant 6/6, Buffs tripled and Nerfs normal".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const VOLATILITY: &str = "meditative-084";
    const OUTFITTER: &str = "meditative-082";
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    fn upgrades(events: &[GameEvent]) -> Vec<GameEvent> {
        events.iter().filter(|event| matches!(event, GameEvent::Upgraded { .. })).cloned().collect()
    }

    mod m84_volatility {
        use super::*;

        #[test]
        fn is_a_1_3_3_common_unit_radiant_6_6_with_no_params() {
            let def = crate::card_def(ID);
            assert_eq!(def.id, VOLATILITY);
            assert_eq!(def.cost, CardCost::Fixed(1));
            assert_eq!(def.type_, CardType::Unit);
            assert_eq!(def.rarity, Rarity::Common);
            assert!(def.tags.is_empty());
            assert_eq!(
                [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
                [Some(3), Some(3), Some(6), Some(6)]
            );
            assert!(def.params.is_none() || def.params.as_ref().is_some_and(Vec::is_empty));
        }

        #[test]
        fn r1160_both_faces_carry_their_multipliers() {
            let scripts = script();
            assert_eq!(
                scripts.base.static_flags.and_then(|flags| flags.tune_multiplier),
                Some(TuneMultiplier {
                    upgrade: Some(VOLATILITY_TUNE_MULTIPLIER),
                    degrade: Some(VOLATILITY_TUNE_MULTIPLIER),
                })
            );
            assert_eq!(
                scripts.radiant.static_flags.and_then(|flags| flags.tune_multiplier),
                Some(TuneMultiplier {
                    upgrade: Some(VOLATILITY_RADIANT_BUFF_MULTIPLIER),
                    degrade: None,
                })
            );
        }

        #[test]
        fn r1160_a_buff_from_medina_outfitter_in_hand_is_one_event_doubled() {
            // One Buff from an Outfitter's Cry, across the seeds: still one event, with a doubled
            // change — stats of 8, two keywords, or cost at its floor.
            let mut stats = 0;
            let mut keyword = 0;
            let mut cost = 0;
            for seed in 1..=40 {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": format!("vol-cry-{seed}"),
                    "p1": {
                        "mana": 10,
                        "hand": [OUTFITTER, { "def": VOLATILITY }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(OUTFITTER, json!({}));
                let ups = upgrades(s.events());
                assert_eq!(ups.len(), 1, "one application is one event (seed {seed})");
                match &ups[0] {
                    GameEvent::Upgraded { change: TuningChange::Stats { attack, health }, .. } => {
                        assert_eq!(attack + health, 2 * TUNE_STAT_TOTAL);
                        stats += 1;
                    }
                    GameEvent::Upgraded {
                        change: TuningChange::Keyword { also, .. },
                        ..
                    } => {
                        assert_eq!(also.as_ref().map(Vec::len), Some(1));
                        keyword += 1;
                    }
                    GameEvent::Upgraded { change: TuningChange::Cost { delta }, .. } => {
                        // Cost (1), doubled to −2, floored at (0): −1 is what moved.
                        assert_eq!(*delta, -1);
                        cost += 1;
                    }
                    other => panic!("unexpected event {other:?} (seed {seed})"),
                }
            }
            assert!(stats > 0 && keyword > 0 && cost > 0, "each doubled row is drawn");
        }

        #[test]
        fn r1160_radiant_a_buff_is_tripled() {
            let mut tripled = 0;
            for seed in 1..=20 {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": format!("vol-radiant-{seed}"),
                    "p1": {
                        "mana": 10,
                        "hand": [OUTFITTER, { "def": VOLATILITY, "radiant": true }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(OUTFITTER, json!({}));
                let ups = upgrades(s.events());
                assert_eq!(ups.len(), 1);
                match &ups[0] {
                    GameEvent::Upgraded { change: TuningChange::Stats { attack, health }, .. } => {
                        assert_eq!(attack + health, 3 * TUNE_STAT_TOTAL);
                        tripled += 1;
                    }
                    GameEvent::Upgraded {
                        change: TuningChange::Keyword { also, .. },
                        ..
                    } => {
                        assert_eq!(also.as_ref().map(Vec::len), Some(2));
                        tripled += 1;
                    }
                    // The cost row (1, tripled to −3, floored) and the X row (no X here) are
                    // covered by the engine's own R1160 tests; any tripled row counts.
                    GameEvent::Upgraded { .. } => {
                        tripled += 1;
                    }
                    other => panic!("unexpected event {other:?}"),
                }
            }
            assert!(tripled > 0);
        }
    }
}
