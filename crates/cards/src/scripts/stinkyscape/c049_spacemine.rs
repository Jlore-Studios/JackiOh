//! SPEC §8.10 Stinkyscape 49 Spacemine, a card of the Stinkyscape set.
//! (2) Unit, Common, 5/1 → 10/2: Starship Piece.
//! "Death: Deal damage equal to its Attack to {targets|random enemy|random enemies}."
//! — targets 1, Radiant 2 (R1860004).
//!
//! Engine: Death (§6.2), on the snapshot it died with (R89): its Attack then, buffs included, as
//! one hit to each of N different random enemies — the hero or a Unit — drawn from the match rng
//! (R60), fewer when fewer stand, through the new `damage_random_enemies` verb (`damage_split`
//! re-draws per hit and can hit one enemy twice); at 0 Attack, nothing (R63). Then its copy fuses
//! into the Dormant Starship (R1852000). The Radiant face runs the same script: its target count
//! is the declared `targets` (R386), and its doubled stats double the hit.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-049";

fn spacemine() -> Script {
    Script {
        death: Some(hook(|ctx| {
            let attack = ctx
                .self_
                .as_ref()
                .map(|me| unit_view(ctx.state, me).attack)
                .unwrap_or(0);
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
    let base = spacemine();
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

    const SPACEMINE: &str = "stinkyscape-049";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    fn destroy(s: &mut Scenario, victim: &CardInstance) {
        s.play(HIT_JOB, json!({ "targets": pick::instance(victim) }));
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

        #[test]
        fn r1860004_death_deals_its_attack_as_it_died_buffs_included_to_one_random_enemy() {
            crate::register_all();
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
            crate::register_all();
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
            crate::register_all();
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

        #[test]
        fn r1860004_radiant_death_hits_two_different_random_enemies() {
            crate::register_all();
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
            crate::register_all();
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
    }
}
