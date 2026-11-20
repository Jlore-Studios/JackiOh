//! Stinkyscape #82 Phalanx (SPEC §8.10 row 82). Unit 3/1 → 6/2, cost 4, Epic (3/3 → 6/6 through v0.6.7k,
//! issue #4285; 4/4 → 8/8 with Armor 3 and 6 until #2657).
//!   Base:    "Armor 2
//!            Cry: Discover {weapons|Rare Weapon|Rare Weapons} and Equip them into this." — weapons 2
//!   Radiant: "Armor 4
//!            Cry: Discover {weapons|Epic Weapon|Epic Weapons} and Equip them into this." — weapons 2
//!
//! R1862002: the Cry is `equip_picks` (R1841002), 2 Discovers of three in turn from the Weapon-tagged
//! cards of the rarity, the pool naming its set so that it reaches them whether or not the set ships (R1420,
//! R1857000). Each pick is a fresh copy, not made Radiant, fused into this with its Activate
//! (R1751000): no Cry, no hand, never a Weapon it holds. The fusion wears the face its kept host
//! wears (R77), so a Radiant Phalanx wields its Weapons' Radiant faces. Since #2521 it is no
//! Starship Piece, and the Weapons it takes, each a piece, leave it none (R1852001), so its death fuses
//! nothing (R1852000 reads the keyword). Its Armor is a catalog keyword, which a fusion keeps (R77). Since
//! issue #4285 the Weapons print no Equip, which the pick never read: they fuse in as before (R4285001).

use jackioh_engine::effects::equip_picks;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-082";

/// "Discover 2": the declared `weapons` since #2657 (R2657006), printed before, as a replay of an
/// older version reads it (`param_or`).
const PICKS: i32 = 2;

/// R1862002: "Discover PICKS `rarity` Weapons and Equip them into this".
fn arm(rarity: Rarity) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![equip_picks(json_as(json!({
                "query": crate::query::weapons_of(rarity),
                "times": param_or(&*ctx, "weapons", PICKS),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: arm(Rarity::Rare),
        radiant: arm(Rarity::Epic),
    }
}

// Stinkyscape #82 Phalanx — SPEC §8.10 row 82, BUILD M12 row S 82 (#2521; R1841002, R1751000, R1852001).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::effects::EQUIP_PICK_STEP;
    use jackioh_engine::subsystems::activate::activate_actions_for;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: a Spell
    const HIT_JOB: &str = "core-016"; // Hit Job: Destroy a Unit.

    /// The Rare and the Epic Weapons (§8.10 row 82).
    const RARE: [&str; 6] = [
        "stinkyscape-050",
        "stinkyscape-051",
        "stinkyscape-052",
        "stinkyscape-053",
        "stinkyscape-140",
        "stinkyscape-141",
    ];
    const EPIC: [&str; 6] = [
        "stinkyscape-054",
        "stinkyscape-055",
        "stinkyscape-056",
        "stinkyscape-057",
        "stinkyscape-142",
        "stinkyscape-143",
    ];

    fn arms(radiant: bool, pool: &[&str]) -> (Scenario, String) {
        let mut s = weapon_board(
            "s082",
            json!({ "hand": [{ "def": ID, "radiant": radiant }] }),
            json!({ "field": [VANILLA] }),
        );
        s.play(ID, json!({ "zone": 1 }));
        let host = s.unit_at(P1, 1).id;
        let mut held: Vec<String> = Vec::new();
        for _ in 0..PICKS {
            let pending = s.state().pending.clone().expect("a Discover is open");
            assert_eq!((pending.kind, pending.player_id), (PromptKind::Discover, P1));
            assert_eq!(pending.resume.step, EQUIP_PICK_STEP);
            let offered = s.prompt_modes();
            assert_eq!(
                offered.len(),
                (DISCOVER_OPTIONS as usize).min(pool.len() - held.len()),
                "{offered:?}"
            );
            assert!(
                offered
                    .iter()
                    .all(|id| pool.contains(&id.as_str()) && !held.contains(id)),
                "radiant {radiant}: {offered:?}"
            );
            let before = s.stats(host.as_str()).health;
            s.answer(json!(offered[0]));
            // R77: the fusion wears the face its kept host wears, so a Radiant host takes its picks'
            let def = crate::card_def(&offered[0]);
            let gained = if radiant {
                def.radiant.health
            } else {
                def.base.health
            }
            .unwrap_or_default();
            assert_eq!(s.stats(host.as_str()).health, before + gained, "{}", offered[0]);
            held.push(offered[0].clone());
        }
        assert!(s.state().pending.is_none());
        (s, host)
    }

    /// The host takes its Weapons' Activates that turn, with its `armor`, and is no Starship Piece: each
    /// Weapon is one, the kept host none (R1852001).
    fn armed(s: &Scenario, host: &str, armor: i32) {
        let card = s.card(host).clone();
        assert_eq!(s.stats(host).armor, armor);
        assert!(!activate_actions_for(s.state(), P1, &card).is_empty());
        assert!(!unit_has(s.state(), &card, KeywordKind::StarshipPiece));
    }

    mod base {
        use super::*;

        #[test]
        fn r1862002_the_cry_discovers_n_weapons_of_its_rarity_in_turn_and_equips_each() {
            let (s, host) = arms(false, &RARE);
            armed(&s, &host, 2);
        }

        /// R4285001: the Weapons print no Equip since issue #4285, and each pick fuses one in all the same.
        #[test]
        fn r4285001_its_picks_still_fuse_and_print_no_equip() {
            crate::register_all();
            for weapon in RARE.iter().chain(EPIC.iter()) {
                let def = crate::card_def(weapon);
                for face in [&def.base, &def.radiant] {
                    assert!(!face.keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Equip), "{weapon}");
                }
            }
            let (s, host) = arms(false, &RARE);
            let fusions = s
                .events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Fused { result_instance_id, .. } if *result_instance_id == host))
                .count();
            assert_eq!(fusions, PICKS as usize);
            armed(&s, &host, 2);
        }

        /// Issue #4285: 3/1, from 3/3, before its Weapons.
        #[test]
        fn it_is_a_3_1_and_radiant_6_2() {
            for (radiant, stats) in [(false, (3, 1)), (true, (6, 2))] {
                let s = crate::scenario(json!({
                    "p1": { "field": [{ "def": ID, "radiant": radiant }], "library": crate::filler(4) },
                    "p2": { "library": crate::filler(4) },
                }));
                let me = s.stats(ID);
                assert_eq!((me.attack, me.health), stats, "radiant {radiant}");
            }
        }

        #[test]
        fn its_death_fuses_nothing_it_is_no_starship_piece() {
            let mut s = crate::scenario(json!({
                "p1": { "hand": [HIT_JOB, FILLER], "field": [ID], "library": crate::filler(4) },
                "p2": { "hand": [FILLER], "library": crate::filler(4) },
            }));
            let me = s.card(ID).clone();
            assert!(!unit_has(s.state(), &me, KeywordKind::StarshipPiece));
            s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));
            s.expect_in_zone(&me, "graveyard");
            assert!(dormant_starship_of(s.state(), P1).is_none(), "no Dormant Starship");
            assert!(!s.events().iter().any(|event| matches!(event, GameEvent::StarshipFused { .. })));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1862002_the_cry_discovers_n_epic_weapons_in_turn_and_equips_each() {
            let (s, host) = arms(true, &EPIC);
            armed(&s, &host, 4);
        }
    }
}
