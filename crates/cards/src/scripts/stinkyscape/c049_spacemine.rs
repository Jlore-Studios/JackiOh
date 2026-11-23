//! SPEC §8.10 Stinkyscape 49 Spacemine, a card of the Stinkyscape set.
//! (2) Unit, Common, 5/1 → 10/2: Starship Piece.
//! "Death: Deal damage equal to its Attack to {targets|random enemy|random enemies}."
//! — targets 1, Radiant 2 (R1860004).
//!
//! Engine: Death (§6.2), on the snapshot it died with (R89): its Attack then, buffs and auras
//! included (R4396000), as one hit to each of N different random enemies — the hero or a Unit —
//! drawn from the match rng (R60), fewer when fewer stand, through the new `damage_random_enemies`
//! verb (`damage_split` re-draws per hit and can hit one enemy twice); at 0 Attack, nothing (R63).
//! Then its copy fuses into the Dormant Starship (R1852000). The Radiant face runs the same
//! script: its target count is the declared `targets` (R386), and its doubled stats double the hit.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-049";

fn spacemine(kept: bool) -> Script {
    Script {
        death: Some(hook(move |ctx| {
            let Some(me) = ctx.self_.clone() else {
                return vec![];
            };
            let attack = if kept {
                unit_view(ctx.state, &me).attack
            } else {
                crate::stats_as_it_died(ctx, &me).0
            };
            if attack <= 0 {
                return vec![];
            }
            vec![damage_random_enemies(json_as(json!({
                "count": param(ctx, "targets"),
                "amount": attack,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // One script: the Radiant face's 2 targets are its declared `targets`, which `param` reads off
    // the running face; its stats are the catalog's.
    let base = spacemine(false);
    CardScripts {
        base: base.clone(),
        radiant: base,
    }
}

/// R767000: through v0.6.9e the Death read its Attack with the auras left off (issue #4396).
pub fn script_v0_6_9e() -> CardScripts {
    let base = spacemine(true);
    CardScripts {
        base: base.clone(),
        radiant: base,
    }
}

// Stinkyscape #49 Spacemine — SPEC §8.10 row 49, BUILD M12 row S 49 (R1852000, R63, R89).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;
    use crate::stinkyscape_family as fam;

    const SPACEMINE: &str = "stinkyscape-049";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const HIT_JOB: &str = "core-016"; // Hit Job: Destroy a Unit.
    const ANCHOR: &str = "core-010"; // Rapid Replenish: Spell (§2.5).
    const WARP: &str = "stinkyscape-023"; // Warp Disruptor: Units have +2 Attack.
    const WARP_ATTACK: i32 = 2;
    const PILLOW: &str = "core-065-1"; // Spikey Pillow: your Units have -2 Attack.
    const NETHER: &str = "core-088"; // Twisting Nether: destroy all permanents.

    fn destroy(s: &mut Scenario, victim: &CardInstance) {
        s.play(HIT_JOB, json!({ "targets": pick::instance(victim) }));
    }

    /// A Spacemine of `radiant` or not on p1's field under a Warp Disruptor p1 has just played, with
    /// one Vanilla enemy per lane in `enemies`.
    fn under_warp(radiant: bool, enemies: &[i32]) -> (Scenario, CardInstance) {
        let field: Vec<Value> = enemies.iter().map(|lane| json!({ "def": VANILLA, "lane": lane })).collect();
        let mut s = scenario(json!({
            "seed": "spacemine-warp",
            "p1": {
                "hand": [HIT_JOB, WARP, ANCHOR],
                "mana": 10,
                "field": [{ "def": SPACEMINE, "radiant": radiant }],
                "library": filler(4),
            },
            "p2": { "hand": [ANCHOR], "field": field, "library": filler(4) },
        }));
        s.play(WARP, json!({ "zone": 1 }));
        let me = s.card(SPACEMINE).clone();
        (s, me)
    }

    fn hits_from(s: &Scenario, source: &str) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage {
                    source_id: Some(from),
                    amount,
                    ..
                } if from == source => Some(*amount),
                _ => None,
            })
            .collect()
    }

    fn targets_of(s: &Scenario, source: &str) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage {
                    source_id: Some(from),
                    target_id,
                    ..
                } if from == source => Some(target_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        /// R4396000: the Death hits for the Attack the card showed as it died, a Warp Disruptor's +2 included.
        #[test]
        fn r4396000_death_deals_the_attack_it_showed_under_a_warp_disruptor() {
            let (mut s, me) = under_warp(false, &[1]);
            assert_eq!(s.stats(SPACEMINE).attack, 5 + WARP_ATTACK);
            destroy(&mut s, &me);

            assert_eq!(hits_from(&s, &me.id), vec![5 + WARP_ATTACK]);
        }

        /// R4396000: an aura that lowers Attack lowers the hit too.
        #[test]
        fn r4396000_death_deals_the_lowered_attack_under_an_aura_that_lowers_it() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [HIT_JOB, ANCHOR],
                    "mana": 10,
                    "field": [SPACEMINE, PILLOW],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "field": [VANILLA], "library": filler(4) },
            }));
            let me = s.card(SPACEMINE).clone();
            assert_eq!(s.stats(SPACEMINE).attack, 3);
            destroy(&mut s, &me);

            assert_eq!(hits_from(&s, &me.id), vec![3]);
        }

        /// R4396000, R463: the aura's source dies in the same pass, gone from the field as the Death
        /// runs, and the hit is still the Attack its `destroyed` event reports.
        #[test]
        fn r4396000_death_deals_its_destroyed_events_attack_when_the_aura_source_dies_with_it() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [NETHER, ANCHOR],
                    "mana": 10,
                    "field": [SPACEMINE, PILLOW],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            let me = s.card(SPACEMINE).clone();
            s.play(NETHER, json!({}));

            let reported: Vec<i32> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Destroyed { instance_id, attack, .. } if *instance_id == me.id => Some(*attack),
                    _ => None,
                })
                .collect();
            assert_eq!(reported, vec![3], "5 less the Pillow's 2, read before either left the field");
            assert_eq!(hits_from(&s, &me.id), reported);
            s.expect_health(PlayerId::P2, HERO_HEALTH - reported[0]);
        }

        /// R767000: the script kept through v0.6.9e leaves the auras off its hit.
        #[test]
        fn r4396000_the_script_kept_for_v0_6_9e_leaves_auras_off_its_hit() {
            crate::under_kept(ID, script_v0_6_9e(), || {
                let (mut s, me) = under_warp(false, &[1]);
                assert_eq!(s.stats(SPACEMINE).attack, 5 + WARP_ATTACK);
                destroy(&mut s, &me);

                assert_eq!(hits_from(&s, &me.id), vec![5]);
            });
        }

        #[test]
        fn r1860004_death_deals_its_attack_as_it_died_buffs_included_to_one_random_enemy() {
            let mut s = scenario(json!({
                "p1": { "hand": [HIT_JOB, ANCHOR], "mana": 10, "field": [SPACEMINE], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "field": [VANILLA], "library": filler(4) },
            }));
            let me = s.card(SPACEMINE).clone();
            s.card_mut(SPACEMINE).buffs.attack += 2;
            let mine = s.card(SPACEMINE).clone();
            destroy(&mut s, &mine);

            assert_eq!(hits_from(&s, &me.id), vec![7]);
            // Then a copy fuses into its owner's Dormant Starship (R1852000).
            let ship = dormant_starship_of(s.state(), PlayerId::P1).cloned();
            assert!(ship.is_some(), "a piece that died fuses a copy into a Dormant Starship");
            assert_eq!(
                starship_pieces_of(&ship.expect("the ship"))
                    .iter()
                    .map(|piece| piece.def_id.as_str())
                    .collect::<Vec<_>>(),
                vec![SPACEMINE]
            );
        }

        #[test]
        fn r1860004_at_0_attack_nothing() {
            let mut s = scenario(json!({
                "p1": { "hand": [HIT_JOB, ANCHOR], "mana": 10, "field": [SPACEMINE], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "field": [VANILLA], "library": filler(4) },
            }));
            // The Attack is driven to 0 with a −5 Attack buff: then nothing happens (R63).
            s.card_mut(SPACEMINE).buffs.attack -= 5;
            let me = s.card(SPACEMINE).clone();
            let mine = me.clone();
            destroy(&mut s, &mine);

            assert!(hits_from(&s, &me.id).is_empty());
        }

        #[test]
        fn targets_reads_through_param() {
            let mut s = scenario(json!({
                "seed": "spacemine-param",
                "p1": { "hand": [HIT_JOB, ANCHOR], "mana": 10, "field": [SPACEMINE], "library": filler(4) },
                "p2": {
                    "hand": [ANCHOR],
                    "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }],
                    "library": filler(4),
                },
            }));
            let targets = crate::upgrade_number(&mut s, SPACEMINE, "targets");
            let me = s.card(SPACEMINE).clone();
            let mine = me.clone();
            destroy(&mut s, &mine);

            let hit = targets_of(&s, &me.id);
            let mut unique = hit.clone();
            unique.sort();
            unique.dedup();
            assert_eq!(
                unique.len(),
                usize::try_from(targets).expect("a count"),
                "{hit:?}"
            );
            assert_eq!(hit.len(), unique.len(), "different enemies");
        }
    }

    mod radiant {
        use super::*;

        /// R4396000: a Radiant Spacemine hits each of its two targets for the Attack it showed as it died.
        #[test]
        fn r4396000_radiant_death_hits_two_enemies_for_the_attack_it_showed_under_a_warp_disruptor() {
            let (mut s, me) = under_warp(true, &[1, 2]);
            assert_eq!(s.stats(SPACEMINE).attack, 10 + WARP_ATTACK);
            destroy(&mut s, &me);

            assert_eq!(hits_from(&s, &me.id), vec![10 + WARP_ATTACK; 2]);
        }

        #[test]
        fn r1860004_radiant_death_hits_two_different_random_enemies() {
            let mut s = scenario(json!({
                "seed": "spacemine-radiant",
                "p1": {
                    "hand": [HIT_JOB, ANCHOR],
                    "mana": 10,
                    "field": [{ "def": SPACEMINE, "radiant": true }],
                    "library": filler(4),
                },
                "p2": {
                    "hand": [ANCHOR],
                    "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }],
                    "library": filler(4),
                },
            }));
            let me = s.card(SPACEMINE).clone();
            let mine = me.clone();
            destroy(&mut s, &mine);

            assert_eq!(hits_from(&s, &me.id), vec![10, 10]);
            let targets = targets_of(&s, &me.id);
            let mut unique = targets.clone();
            unique.sort();
            unique.dedup();
            assert_eq!(unique.len(), 2, "two different enemies: {targets:?}");
        }

        #[test]
        fn r1860004_radiant_one_when_only_one_stands() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [HIT_JOB, ANCHOR],
                    "mana": 10,
                    "field": [{ "def": SPACEMINE, "radiant": true }],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            let me = s.card(SPACEMINE).clone();
            let mine = me.clone();
            destroy(&mut s, &mine);

            assert_eq!(hits_from(&s, &me.id), vec![10]);
            s.expect_health(PlayerId::P2, 20);
        }

        const RADIANT_TARGETS: i32 = 2;

        /// R4334000: a launched Starship holding a Radiant copy hits the Radiant `targets` of
        /// different random enemies when it dies, and shows it.
        #[test]
        fn r4334000_a_starship_holding_a_radiant_copy_uses_its_radiant_number() {
            let (mut s, ship) = fam::piece_ship(
                ID,
                "s049-piece-ship",
                true,
                json!({ "hand": [HIT_JOB] }),
                // Durables outlast the piece's own Death, as it died to build the Starship.
                json!({ "field": [{ "def": fam::DURABLE, "lane": 1 }, { "def": fam::DURABLE, "lane": 2 }] }),
            );
            assert_eq!(fam::shown_number(&s, &ship, "targets"), Some(RADIANT_TARGETS));
            s.state_mut().players.p1.mana.current = 10;
            s.play(HIT_JOB, json!({ "targets": pick::id(&ship) }));
            let hits = crate::hits(s.last_events());
            let mut targets: Vec<String> = hits.iter().map(|(id, _)| id.clone()).collect();
            targets.sort();
            targets.dedup();
            let count = usize::try_from(RADIANT_TARGETS).expect("a count");
            assert_eq!((hits.len(), targets.len()), (count, count), "{hits:?}");
        }

        /// R4396000, R4334000: a launched Starship holding a Spacemine copy hits by the Attack it showed as
        /// it died, with the Warp Disruptor that raised it destroyed in the same pass.
        #[test]
        fn r4396000_a_starship_holding_a_spacemine_hits_by_its_attack_as_it_died() {
            let (mut s, ship) = fam::piece_ship(
                ID,
                "s049-ship-aura",
                false,
                json!({ "hand": [WARP, NETHER] }),
                json!({ "field": [{ "def": fam::DURABLE, "lane": 1 }] }),
            );
            let plain = s.stats(ship.as_str()).attack;
            s.state_mut().players.p1.mana.current = 10;
            s.play(WARP, json!({ "zone": 1 }));
            let shown = s.stats(ship.as_str()).attack;
            assert_eq!(shown, plain + WARP_ATTACK);
            s.play(NETHER, json!({}));

            let amounts: Vec<i32> = crate::hits(s.last_events()).iter().map(|(_, amount)| *amount).collect();
            assert_eq!(amounts, vec![shown]);
        }
    }
}
