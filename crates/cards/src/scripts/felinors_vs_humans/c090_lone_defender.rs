//! SPEC §8.14 FvH #90 Lone Defender ((2) Unit, Human, Common, 3/5 → 6/10), a card of the held Felinors vs
//! Humans set (R5235000).
//!   Base:    "Cry: If you control no other Units, gain Taunt, Divine Shield, Lifesteal and Pierce."
//!   Radiant: the same.
//!
//! Engine: the Cry (R1) reads its controller's acting Units as it runs (`active_units_of`, so a Dormant card
//! and a Stack's buried cards are not counted, R1839000); with none but itself, `grant_keywords` of the four
//! on itself for good (S+ #46 Phase Fabric's verb, R5113001). A summoned one runs no Cry. Declares no
//! numbers. Built by S01 (#5251).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-090";

/// Whether its controller controls no acting Unit but the card itself.
fn alone(ctx: &EffectContext<'_>) -> bool {
    let me = ctx.live_self().map(|card| card.id.clone());
    active_units_of(ctx.state, ctx.controller)
        .iter()
        .all(|unit| Some(&unit.id) == me.as_ref())
}

fn lone_defender() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            if !alone(ctx) {
                return Vec::new();
            }
            vec![grant_keywords(json_as(json!({
                "target": { "of": "self" },
                "keywords": [
                    { "kind": "Taunt" },
                    { "kind": "Divine Shield" },
                    { "kind": "Lifesteal" },
                    { "kind": "Pierce" },
                ],
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // Both faces print the same Cry; the Radiant line is catalog data.
    CardScripts {
        base: lone_defender(),
        radiant: lone_defender(),
    }
}

// FvH #90 Lone Defender — SPEC §8.14 row 90 (R1, R1839000, R5113001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const FRIEND: &str = "stand-in (1) 2/2";
    const FOE: &str = "stand-in (1) 3/3";
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    const GRANTED: [KeywordKind; 4] = [
        KeywordKind::Taunt,
        KeywordKind::DivineShield,
        KeywordKind::Lifesteal,
        KeywordKind::Pierce,
    ];

    /// p1 plays the Defender (`radiant` its face) into lane 3 beside `field`, the enemy holding a Foe:
    /// which of the four keywords it has then.
    fn played_beside(radiant: bool, field: Value) -> Vec<KeywordKind> {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": field,
                "library": filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "field": [FOE], "library": filler(4) },
        }));
        s.play(ID, json!({ "zone": 3 }));
        let keywords = s.stats(s.unit_at(P1, 3)).keywords;
        GRANTED
            .into_iter()
            .filter(|kind| has_keyword(&keywords, *kind))
            .collect()
    }

    #[test]
    fn declares_no_tuned_numbers() {
        crate::register_all();
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn alone_on_its_side_it_gains_taunt_divine_shield_lifesteal_and_pierce() {
            assert_eq!(played_beside(false, json!([])), GRANTED.to_vec(), "an enemy Unit does not count");
        }

        #[test]
        fn beside_another_friendly_unit_it_gains_nothing() {
            assert!(played_beside(false, json!([FRIEND])).is_empty());
        }

        #[test]
        fn r1839000_a_dormant_friendly_unit_does_not_count() {
            let dormant = json!([{ "def": FRIEND, "slumber": { "turns": 2 } }]);
            assert_eq!(played_beside(false, dormant), GRANTED.to_vec());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn alone_on_its_side_it_gains_taunt_divine_shield_lifesteal_and_pierce() {
            assert_eq!(played_beside(true, json!([])), GRANTED.to_vec());
        }

        #[test]
        fn beside_another_friendly_unit_it_gains_nothing() {
            assert!(played_beside(true, json!([FRIEND])).is_empty());
        }
    }
}
