//! Stinkyscape #81 Radix (SPEC §8.10 row 81). Unit 2/2 → 4/4, cost 3, Rare.
//!   Base:    "Cry: Discover 2 Rare Weapons and Equip them into this."
//!   Radiant: "Cry: Discover 2 Epic Weapons and Equip them into this."
//!
//! R1862002: the Cry is `equip_picks` (R1841002), 2 Discovers of three in turn from the Weapon-tagged
//! cards of the rarity, the pool naming its set so that it reaches them whether or not the set ships (R1420,
//! R1857000). Each pick is a fresh copy, not made Radiant, fused into this with its Activate
//! (R1751000): no Cry, no hand, never a Weapon it holds. The fusion wears the face its kept host
//! wears (R77), so a Radiant Radix wields its Weapons' Radiant faces. Since #2521 it is no
//! Starship Piece, and the Weapons it takes, each a piece, leave it none (R1852001), so its death fuses
//! nothing (R1852000 reads the keyword).

use jackioh_engine::effects::equip_picks;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-081";

/// "Discover 2": the declared `weapons` since #2657 (R2657006), printed before, as a replay of an
/// older version reads it (`param_or`).
const PICKS: i32 = 2;

/// R1862002: "Discover PICKS `rarity` Weapons and Equip them into this".
fn arm(rarity: Rarity) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![equip_picks(json_as(json!({
                "query": { "set": SetName::Stinkyscape, "tags": [Tag::Weapon], "rarity": rarity },
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

// Stinkyscape #81 Radix — SPEC §8.10 row 81, BUILD M12 row S 81 (#2521; R1841002, R1751000, R1852001).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::effects::EQUIP_PICK_STEP;
    use jackioh_engine::subsystems::activate::activate_actions_for;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005"; // (1) Spell.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.

    /// The Rare and the Epic Weapons (§8.10 row 81).
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
            "s081",
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
            let offered: Vec<String> = pending
                .options
                .iter()
                .filter_map(|option| match &option.selection {
                    Selection::Mode { option } => Some(option.clone()),
                    _ => None,
                })
                .collect();
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

    /// The host takes its Weapons' Activates that turn, and is no Starship Piece: each Weapon is one,
    /// the kept host none (R1852001).
    fn armed(s: &Scenario, host: &str) {
        let card = s.card(host).clone();
        assert!(!activate_actions_for(s.state(), P1, &card).is_empty());
        assert!(!unit_has(s.state(), &card, KeywordKind::StarshipPiece));
    }

    mod base {
        use super::*;

        #[test]
        fn r1862002_the_cry_discovers_n_weapons_of_its_rarity_in_turn_and_equips_each() {
            let (s, host) = arms(false, &RARE);
            armed(&s, &host);
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
            armed(&s, &host);
        }
    }
}
