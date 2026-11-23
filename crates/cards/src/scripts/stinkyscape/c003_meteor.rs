//! S #3 Meteor (SPEC §8.10 row S 3; §3.1, §4.4, §6.3 Lock, R81, R1848001): (3) Spell, Epic.
//!   Base:    "Deal {damage} damage to a Unit, trampling onto the Units adjacent to it. Lock its zone."
//!            — 15.
//!   Radiant: "Deal {damage} damage to a Unit, trampling onto the Units adjacent to it and then its
//!            hero. Lock its zone." — 15 (20 before #2521's balance pass, which tuned it down; it still
//!            tramples on to the hero).
//!
//! Engine: a declared target, any Unit on either side (R81); one `damage` whose `trampleOnto` sends
//! its excess past the target's Health onto the Units beside it in its row on its side (§3.1), one
//! drawn from the match rng when there are two, each counting it only up to its Health, a neighbor
//! Immune to Spells passed by and one that takes the hit whole stopping the rest (R1848001); with no
//! neighbor the excess is lost. The Radiant face's `adjacentAndHero` sends what passes both on to
//! the target's controller's hero. Then `lock` on the target's zone (§6.3 Lock), in the text's order:
//! a target the hit killed still stands there until the state check, so its zone is Locked whether
//! it died or not.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-003";

/// R81: any Unit on either side, picked with the play.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

/// R1848001: `onto` is `adjacent` on the base face, `adjacentAndHero` on the Radiant one.
fn meteor(onto: &'static str) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            vec![
                damage(json_as(json!({
                    "to": { "of": "chosen" },
                    "amount": param(&*ctx, "damage"),
                    "trampleOnto": onto,
                }))),
                lock(json_as(json!({ "zone": { "of": "chosen" } }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: meteor("adjacent"),
        radiant: meteor("adjacentAndHero"),
    }
}

// S #3 Meteor — SPEC §8.10 row S 3, BUILD M12 row S 3 (R1848001, #2521).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{degrade_number, filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const METEOR: &str = "stinkyscape-003";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const GENN: &str = "stand-in (4) 14/14 R42/42";
    const JILLIAX: &str = "core-056"; // Jilliax: Rush, Taunt, Lifesteal, Divine Shield
    const FILLER: &str = "core-005";

    fn hero_health(s: &Scenario, player: PlayerId) -> i32 {
        s.state().players[player].hero.health
    }

    fn has_divine_shield(s: &Scenario, card: &CardInstance) -> bool {
        s.stats(card)
            .keywords
            .iter()
            .any(|keyword| keyword.kind() == KeywordKind::DivineShield)
    }

    fn meteor_board(radiant: bool, units: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": METEOR, "radiant": radiant }, FILLER] },
            "p2": { "hand": [FILLER], "field": units, "library": filler(4) },
        }))
    }

    /// R4453000: [VANILLA ×3] on `seed`, lane 1 stamped Dormant (white-box, as the engine's tests do):
    /// the sleeper, the lane-2 target and the lane-3 Unit.
    fn dormant_board(seed: &str, radiant: bool) -> (Scenario, [CardInstance; 3]) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": METEOR, "radiant": radiant }, FILLER] },
            "p2": { "hand": [FILLER], "field": [VANILLA, VANILLA, VANILLA], "library": filler(4) },
        }));
        let units = [1, 2, 3].map(|lane| s.unit_at(P2, lane));
        s.card_mut(&units[0]).slumber = Some(Slumber { turns: Some(2) });
        (s, units)
    }

    mod base {
        use super::*;

        #[test]
        fn r1848001_15_tramples_onto_both_neighbours_and_the_rest_is_lost() {
            let mut s = meteor_board(false, json!([VANILLA, VANILLA, VANILLA]));
            let units: Vec<CardInstance> = (1..=3).map(|lane| s.unit_at(P2, lane)).collect();

            s.play(METEOR, json!({ "targets": pick::instance(&units[1]) }));

            // 4 to the target, 4 to each neighbor, and the last 3 lost.
            for unit in &units {
                s.expect_in_zone(unit, "graveyard");
            }
            assert_eq!(hero_health(&s, P2), HERO_HEALTH);
        }

        #[test]
        fn a_divine_shield_neighbour_stops_it() {
            let mut s = meteor_board(
                false,
                json!([
                    { "def": VANILLA, "lane": 1 },
                    { "def": JILLIAX, "lane": 2 },
                    { "def": VANILLA, "lane": 3 },
                ]),
            );
            let target = s.unit_at(P2, 1);
            let shielded = s.unit_at(P2, 2);
            let past_it = s.unit_at(P2, 3);

            s.play(METEOR, json!({ "targets": pick::instance(&target) }));

            s.expect_in_zone(&target, "graveyard");
            s.expect_in_zone(&shielded, "field");
            assert_eq!(s.card(&shielded).damage, 0, "its shield takes the hit whole");
            assert!(!has_divine_shield(&s, &shielded));
            assert_eq!(s.card(&past_it).damage, 0, "lane 3 is beside lane 2, not the target");
        }

        #[test]
        fn an_immune_neighbour_is_passed_by() {
            let mut s = meteor_board(false, json!([VANILLA, VANILLA, VANILLA]));
            let immune = s.unit_at(P2, 1);
            let target = s.unit_at(P2, 2);
            let other = s.unit_at(P2, 3);
            s.card_mut(&immune).granted_keywords.push(Keyword::ImmuneToSpells);

            s.play(METEOR, json!({ "targets": pick::instance(&target) }));

            s.expect_in_zone(&target, "graveyard");
            s.expect_in_zone(&immune, "field");
            assert_eq!(s.card(&immune).damage, 0);
            s.expect_in_zone(&other, "graveyard");
        }

        #[test]
        fn r4453000_a_dormant_neighbour_is_passed_by() {
            // A Dormant card acts not (§6.1 Dormant): it is no neighbour, so whichever way a coin would
            // have landed the 11 left go on to lane 3, never stopping on a card no effect reaches.
            for n in 0..8 {
                let (mut s, [sleeper, target, other]) = dormant_board(&format!("meteor-dormant-{n}"), false);
                s.play(METEOR, json!({ "targets": pick::instance(&target) }));
                s.expect_in_zone(&target, "graveyard");
                s.expect_in_zone(&other, "graveyard");
                s.expect_in_zone(&sleeper, "field");
                assert_eq!(s.card(&sleeper).damage, 0, "seed {n}");
            }
        }

        #[test]
        fn with_no_neighbour_the_excess_is_lost() {
            let mut s = meteor_board(false, json!([{ "def": VANILLA, "lane": 3 }]));
            let target = s.unit_at(P2, 3);

            s.play(METEOR, json!({ "targets": pick::instance(&target) }));

            s.expect_in_zone(&target, "graveyard");
            assert_eq!(hero_health(&s, P2), HERO_HEALTH);
        }

        fn no_play_into(s: &Scenario, lane: i32) -> bool {
            let plays: Vec<Option<ZoneChoice>> = s
                .plays_of(VANILLA)
                .iter()
                .filter_map(|action| match action {
                    ActionBody::Play { zone, .. } => Some(*zone),
                    _ => None,
                })
                .collect();
            assert!(!plays.is_empty(), "the Unit has other zones to go to");
            plays
                .iter()
                .all(|zone| *zone != Some(ZoneChoice { row: Row::Units, lane }))
        }

        fn locked_event(s: &Scenario, lane: i32) -> bool {
            s.last_events().iter().any(|event| {
                matches!(
                    event,
                    GameEvent::Locked { player: P2, row: Row::Units, lane: at } if *at == lane
                )
            })
        }

        #[test]
        fn the_zone_is_locked_whether_it_died_or_not() {
            let mut s = scenario(json!({
                "p1": { "hand": [METEOR, FILLER], "library": filler(4) },
                "p2": { "hand": [VANILLA, FILLER], "field": [{ "def": VANILLA, "lane": 2 }], "library": filler(4) },
            }));
            let killed = s.unit_at(P2, 2);
            s.play(METEOR, json!({ "targets": pick::instance(&killed) }));
            s.expect_in_zone(&killed, "graveyard");
            assert!(locked_event(&s, 2));
            s.end_turn();
            assert_eq!(s.state().active, P2);
            assert!(no_play_into(&s, 2));

            let mut s = scenario(json!({
                "p1": { "hand": [METEOR, FILLER] },
                "p2": { "hand": [FILLER], "field": [{ "def": GENN, "radiant": true, "lane": 4 }] },
            }));
            let survivor = s.unit_at(P2, 4);
            s.play(METEOR, json!({ "targets": pick::instance(&survivor) }));
            s.expect_in_zone(&survivor, "field");
            assert_eq!(s.card(&survivor).damage, 15);
            assert!(locked_event(&s, 4));
            assert!(is_locked(
                s.state(),
                ZoneSlot {
                    player: P2,
                    row: Row::Units,
                    lane: 4,
                }
            ));
        }

        #[test]
        fn damage_reads_through_param() {
            let mut s = meteor_board(false, json!([{ "def": VANILLA, "lane": 1 }, { "def": GENN, "lane": 2 }]));
            assert_eq!(degrade_number(&mut s, METEOR, "damage"), 11);
            let target = s.unit_at(P2, 1);
            let genn = s.unit_at(P2, 2);

            s.play(METEOR, json!({ "targets": pick::instance(&target) }));

            s.expect_in_zone(&target, "graveyard");
            assert_eq!(s.stats(&genn).health, 7);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1848001_15_and_the_rest_hits_its_controllers_hero() {
            let mut s = meteor_board(true, json!([VANILLA, VANILLA, VANILLA]));
            let units: Vec<CardInstance> = (1..=3).map(|lane| s.unit_at(P2, lane)).collect();

            s.play(METEOR, json!({ "targets": pick::instance(&units[1]) }));

            for unit in &units {
                s.expect_in_zone(unit, "graveyard");
            }
            // 15 less 4 for each of the three is 3 on their hero (#2521: the Radiant face deals 15).
            assert_eq!(hero_health(&s, P2), HERO_HEALTH - 3);
            assert_eq!(hero_health(&s, P1), HERO_HEALTH);
        }

        #[test]
        fn r4453000_radiant_sends_the_rest_to_the_hero() {
            // 15 less 4 for the target and 4 for lane 3 is 7 on their hero; the Dormant lane 1 takes none.
            for n in 0..8 {
                let (mut s, [sleeper, target, _]) = dormant_board(&format!("meteor-dormant-radiant-{n}"), true);
                s.play(METEOR, json!({ "targets": pick::instance(&target) }));
                assert_eq!(hero_health(&s, P2), HERO_HEALTH - 7, "seed {n}");
                assert_eq!(s.card(&sleeper).damage, 0, "seed {n}");
            }
        }
    }
}
