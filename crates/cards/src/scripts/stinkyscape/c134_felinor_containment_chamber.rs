//! SPEC §8.10 Stinkyscape 134 Felinor Containment Chamber, a card of the Stinkyscape
//! set. (1) Unit, Common, 2/1 → 4/2: Starship Piece.
//! "Death: Summon a Felinor Token with this Unit's Attack and Health." / Radiant: "Death: Summon
//! a Felinor Token with twice this Unit's Attack and Health. It gains Divine Shield." (R1860007).
//!
//! Engine: Death (§6.2), on the snapshot it died with (R89): `summon` the Felinor Token (§7) with
//! a `statsOverride` of its Attack and its maximum Health then — its current Health is 0 or less
//! as it died — buffs and auras included (R4396000), into the leftmost open unit zone (R64),
//! before its own copy fuses into the Dormant Starship (R1852000), which may then stack on lane 3
//! if the token took the last zone; the Radiant face doubles both and grants Divine Shield
//! through `summon`'s `keywords`

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-134";

/// §7's shared Felinor Token. Read through `card_def` so a wrong id fails as the script is built,
/// not in play.
fn felinor_token() -> String {
    crate::card_def("core-t-felinor").id
}

fn chamber(doubled: bool, shielded: bool, kept: bool) -> Script {
    let token = felinor_token();
    Script {
        death: Some(hook(move |ctx| {
            let Some(this) = ctx.self_.clone() else {
                return vec![];
            };
            let (attack, max_health) = if kept {
                let view = unit_view(ctx.state, &this);
                (view.attack, view.max_health)
            } else {
                crate::stats_as_it_died(ctx, &this)
            };
            let (attack, health) = if doubled {
                (attack * 2, max_health * 2)
            } else {
                (attack, max_health)
            };
            if health <= 0 {
                return vec![];
            }
            let keywords = if shielded {
                json!([{ "kind": "Divine Shield" }])
            } else {
                json!([])
            };
            vec![summon(json_as(json!({
                "defId": token,
                "player": "self",
                "statsOverride": { "attack": attack.max(0), "health": health },
                "keywords": keywords,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: chamber(false, false, false),
        radiant: chamber(true, true, false),
    }
}

/// R767000: through v0.6.9e the Death read its stats with the auras left off (issue #4396).
pub fn script_v0_6_9e() -> CardScripts {
    CardScripts {
        base: chamber(false, false, true),
        radiant: chamber(true, true, true),
    }
}

// Stinkyscape #134 Felinor Containment Chamber — SPEC §8.10 row 134, BUILD M12 row S 134
// (R1852000, R89).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const CHAMBER: &str = "stinkyscape-134";
    const FELINOR: &str = "core-t-felinor"; // Felinor Token: §7's shared token
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const HIT_JOB: &str = "core-016"; // Hit Job: Destroy a Unit.
    const ANCHOR: &str = "core-010"; // Rapid Replenish: Spell (§2.5).
    const WARP: &str = "stinkyscape-023"; // Warp Disruptor: Units have +2 Attack.
    const WARP_ATTACK: i32 = 2;
    const PILLOW: &str = "core-065-1"; // Spikey Pillow: your Units have -2 Attack.
    const NETHER: &str = "core-088"; // Twisting Nether: destroy all permanents.

    /// A Chamber of `radiant` or not on p1's field under a Warp Disruptor p1 has just played.
    fn under_warp(radiant: bool) -> (Scenario, CardInstance) {
        let mut s = scenario(json!({
            "p1": {
                "hand": [HIT_JOB, WARP, ANCHOR],
                "mana": 10,
                "field": [{ "def": CHAMBER, "radiant": radiant }],
                "library": filler(4),
            },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        s.play(WARP, json!({ "zone": 1 }));
        let me = s.card(CHAMBER).clone();
        (s, me)
    }

    /// The `(attack, health)` the token in p1's lane 1 was summoned with, which the Warp Disruptor beside it
    /// adds to without changing: a view of it would read the aura on top.
    fn summoned_with(s: &Scenario) -> (CardInstance, Option<(i32, i32)>) {
        let token = s.unit(PlayerId::P1, 1).expect("the token in lane 1");
        assert_eq!(token.def_id, FELINOR);
        let stats = token.stats_override.map(|stats| (stats.attack, stats.health));
        (token, stats)
    }

    /// What the Chamber's `destroyed` event reports: its `(attack, max_health)` as it died.
    fn reported(s: &Scenario, chamber: &CardInstance) -> Vec<(i32, i32)> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Destroyed {
                    instance_id,
                    attack,
                    max_health,
                    ..
                } if *instance_id == chamber.id => Some((*attack, *max_health)),
                _ => None,
            })
            .collect()
    }

    fn kinds(s: &Scenario, card: &str) -> Vec<String> {
        s.stats(card)
            .keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    mod base {
        use super::*;

        /// R4396000: the token takes the Attack and max Health the card showed as it died, a Warp
        /// Disruptor's +2 Attack included.
        #[test]
        fn r4396000_death_summons_a_token_of_the_stats_it_showed_under_a_warp_disruptor() {
            let (mut s, me) = under_warp(false);
            s.expect_stats(CHAMBER, json!({ "attack": 2 + WARP_ATTACK, "maxHealth": 1 }));
            s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

            let (token, stats) = summoned_with(&s);
            assert_eq!(stats, Some((2 + WARP_ATTACK, 1)));
            assert!(!kinds(&s, token.id.as_str()).contains(&"Divine Shield".to_string()));
        }

        /// R4396000, R463: the aura's source dies in the same pass, gone from the field as the Death runs,
        /// and the token is still the `(attack, maxHealth)` its `destroyed` event reports.
        #[test]
        fn r4396000_death_summons_a_token_of_its_destroyed_events_stats_when_the_aura_source_dies_with_it() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [NETHER, ANCHOR],
                    "mana": 10,
                    "field": [CHAMBER, PILLOW],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            let me = s.card(CHAMBER).clone();
            s.play(NETHER, json!({}));

            let died_as = reported(&s, &me);
            assert_eq!(died_as, vec![(0, 1)], "2 less the Pillow's 2, read before either left the field");
            let (_, stats) = summoned_with(&s);
            assert_eq!(stats, died_as.first().copied());
        }

        /// R767000: the script kept through v0.6.9e leaves the auras off its token.
        #[test]
        fn r4396000_the_script_kept_for_v0_6_9e_leaves_auras_off_its_token() {
            crate::under_kept(ID, script_v0_6_9e(), || {
                let (mut s, me) = under_warp(false);
                s.expect_stats(CHAMBER, json!({ "attack": 2 + WARP_ATTACK, "maxHealth": 1 }));
                s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

                assert_eq!(summoned_with(&s).1, Some((2, 1)));
            });
        }

        #[test]
        fn r1860007_death_summons_a_felinor_token_of_its_attack_and_max_health_buffs_included() {
            let mut s = scenario(json!({
                "p1": { "hand": [HIT_JOB, ANCHOR], "mana": 10, "field": [CHAMBER], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.card_mut(CHAMBER).buffs.attack += 3;
            s.card_mut(CHAMBER).buffs.health += 3;
            let me = s.card(CHAMBER).clone();
            s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

            s.expect_in_zone(CHAMBER, "graveyard");
            // The token lands in the leftmost open unit zone, before the copy fuses (R1852000).
            let token = s.unit(PlayerId::P1, 1).expect("the token in lane 1");
            assert_eq!(token.def_id, FELINOR);
            s.expect_stats(token.id.as_str(), json!({ "attack": 5, "health": 4, "maxHealth": 4 }));
            assert!(!kinds(&s, token.id.as_str()).contains(&"Divine Shield".to_string()));
            let ship = dormant_starship_of(s.state(), PlayerId::P1).cloned();
            assert!(ship.is_some(), "its copy still fuses into a Dormant Starship");
        }

        #[test]
        fn r1860007_a_full_board_summons_none() {
            crate::register_all();
            // The Chamber tops a pile on lane 1; when it dies the Vanilla under it resumes there
            // (§3.2), so no zone is open.
            let mut s = scenario(json!({
                "p1": {
                    "hand": [HIT_JOB, ANCHOR],
                    "mana": 10,
                    "field": [VANILLA, { "def": CHAMBER, "stack": true }, VANILLA, VANILLA, VANILLA, VANILLA],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            let me = s.card(CHAMBER).clone();
            s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

            s.expect_in_zone(CHAMBER, "graveyard");
            // No zone is open, so no token lands — but the copy still fuses (R1852000).
            for lane in 1..=5 {
                assert_ne!(
                    s.unit(PlayerId::P1, lane).map(|unit| unit.def_id),
                    Some(FELINOR.to_string()),
                    "no Felinor Token in lane {lane}"
                );
            }
            assert!(dormant_starship_of(s.state(), PlayerId::P1).is_some());
        }
    }

    mod radiant {
        use super::*;

        /// R4396000: a Radiant Chamber's token doubles the stats it showed as it died, a Warp Disruptor's
        /// +2 Attack included, and has Divine Shield.
        #[test]
        fn r4396000_radiant_death_summons_a_doubled_token_of_the_stats_it_showed_under_a_warp_disruptor() {
            let (mut s, me) = under_warp(true);
            s.expect_stats(CHAMBER, json!({ "attack": 4 + WARP_ATTACK, "maxHealth": 2 }));
            s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

            let (token, stats) = summoned_with(&s);
            assert_eq!(stats, Some(((4 + WARP_ATTACK) * 2, 4)));
            assert!(kinds(&s, token.id.as_str()).contains(&"Divine Shield".to_string()));
        }

        /// R4396000, R463: the same with the aura's source destroyed in the same pass: twice what the
        /// `destroyed` event reports.
        #[test]
        fn r4396000_radiant_death_doubles_its_destroyed_events_stats_when_the_aura_source_dies_with_it() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [NETHER, ANCHOR],
                    "mana": 10,
                    "field": [{ "def": CHAMBER, "radiant": true }, PILLOW],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            let me = s.card(CHAMBER).clone();
            s.play(NETHER, json!({}));

            let died_as = reported(&s, &me);
            assert_eq!(died_as, vec![(2, 2)], "4 less the Pillow's 2, read before either left the field");
            let (token, stats) = summoned_with(&s);
            assert_eq!(stats, Some((4, 4)));
            s.expect_stats(token.id.as_str(), json!({ "attack": 4, "health": 4, "maxHealth": 4 }));
            assert!(kinds(&s, token.id.as_str()).contains(&"Divine Shield".to_string()));
        }

        #[test]
        fn r1860007_radiant_death_summons_a_doubled_token_with_divine_shield() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [HIT_JOB, ANCHOR],
                    "mana": 10,
                    "field": [{ "def": CHAMBER, "radiant": true }],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            let me = s.card(CHAMBER).clone();
            s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

            let token = s.unit(PlayerId::P1, 1).expect("the token in lane 1");
            assert_eq!(token.def_id, FELINOR);
            s.expect_stats(token.id.as_str(), json!({ "attack": 8, "health": 4, "maxHealth": 4 }));
            assert!(kinds(&s, token.id.as_str()).contains(&"Divine Shield".to_string()));
        }
    }
}
