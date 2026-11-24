//! SPEC §8.13 IA #160 Zero Trust ((3) Field Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Aura: Your Units in Defense Position have +0/+{health} and are Immune to Spells." (health 3)
//!   Radiant: "Aura: Your Units in Defense Position have +0/+{health} and are Immune to Spells." (health 6)
//!
//! Engine: an aura keyed on `position == DEF` (§10.4 layer 5), read off the instance on every read:
//! the controller's Units standing in Defense Position (R1081000: a Unit standing in the backrow is
//! one of "your Units") have +0/+{health} and are Immune to Spells. The Radiant face's 6 is its
//! declared `health`.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-160";

/// +0/+{health} and Immune to Spells on the controller's own Units in Defense Position (§4.1,
/// §10.4), read off the instance on every read rather than stored. The number is the declared
/// `health` (3, 6 on the Radiant face).
fn defense_aura() -> AuraHook {
    aura_hook(|args: HookArgs<'_>| {
        let n = param(&args, "health");
        let controller = args.self_.controller;
        let state = args.state;
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == controller
                    // R1081000: a Unit standing in the backrow is one of "your units".
                    && in_units_row_or_standing(state, unit)
                    && unit.position.unwrap_or(Position::Atk) == Position::Def
            }),
            mod_: StatMod {
                max_health: Some(n),
                keywords: Some(vec![Keyword::ImmuneToSpells]),
                ..StatMod::default()
            },
        }]
    })
}

pub fn script() -> CardScripts {
    // One aura: the Radiant face's 6 is its declared `health`, read off the face that is up.
    let base = Script {
        aura: Some(defense_aura()),
        ..Script::default()
    };
    CardScripts {
        base: base.clone(),
        radiant: base,
    }
}

// IA #160 Zero Trust — SPEC §8.13 row 160.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/5";
    const HIT_JOB: &str = "core-016";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": true, "lane": 1 }],
                "field": [
                    { "def": UNIT, "position": "DEF" },
                    UNIT,
                ],
                "library": crate::filler(3),
            },
            "p2": { "hand": [HIT_JOB], "field": [UNIT], "library": crate::filler(3), "mana": 8 },
        }))
    }

    fn immune(s: &Scenario, card: &CardInstance) -> bool {
        has_keyword(&s.stats(card).keywords, KeywordKind::ImmuneToSpells)
    }

    mod base {
        use super::*;

        #[test]
        fn a_defense_position_ally_has_0_3_and_is_immune_to_spells() {
            let mut s = board(false);
            let defended = s.unit_at(P1, 1);
            s.expect_stats(&defended, json!({ "attack": 2, "health": 8 }));
            assert!(immune(&s, &defended));
            let standing = s.unit_at(P1, 2);
            s.expect_stats(&standing, json!({ "attack": 2, "health": 5 }));
            assert!(!immune(&s, &standing));
        }

        #[test]
        fn an_immune_unit_is_no_spell_target() {
            let s = board(false);
            let defended = s.unit_at(P1, 1);
            assert!(!pick::instance_ids(&s.play_targets(HIT_JOB)).contains(&defended.id));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_defense_position_ally_has_0_6() {
            let mut s = board(true);
            let defended = s.unit_at(P1, 1);
            s.expect_stats(&defended, json!({ "attack": 2, "health": 11 }));
            assert!(immune(&s, &defended));
        }
    }
}
