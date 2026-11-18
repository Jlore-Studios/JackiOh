//! Stinkyscape #83 Halo (SPEC §8.10 row 83). Unit 2/2 → 4/4, cost 4, Epic.
//!   Base:    "Cry: Choose one of 2 Rare Weapons and Equip it into this, {weapons|time|times}." — weapons 3
//!   Radiant: "Cry: Choose one of 2 Epic Weapons and Equip it into this, {weapons|time|times}." — weapons 3
//!
//! R1862002, R4154104: the Cry is `equip_picks` (R1841002), 3 picks in turn, each a choice of two
//! (`OPTIONS`, a Discover's prompt offering two) from the Weapon-tagged cards of the rarity, the pool naming its set so that it reaches them whether or not the set ships (R1420,
//! R1857000). Each pick is a fresh copy, not made Radiant, fused into this with its Activate
//! (R1751000): no Cry, no hand, never a Weapon it holds. The fusion wears the face its kept host
//! wears (R77), so a Radiant Halo wields its Weapons' Radiant faces. Since #2521 it is no
//! Starship Piece, and the Weapons it takes, each a piece, leave it none (R1852001), so its death fuses
//! nothing (R1852000 reads the keyword). Kept for replays (R767000): through v0.6.6g each pick was a
//! Discover of three, "Discover 3 Rare Weapons" (`script_v0_6_6g`, issue #4154).

use jackioh_engine::effects::equip_picks;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-083";

/// "Discover 3": the declared `weapons` since #2657 (R2657006), printed before, as a replay of an
/// older version reads it (`param_or`).
const PICKS: i32 = 3;
/// R4154104: "choose one of 2": each pick's offer, since issue #4154.
const OPTIONS: i32 = 2;

/// R1862002, R4154104: PICKS times, a choice of `options` `rarity` Weapons (a Discover's three when
/// `None`, through v0.6.6g), Equipped into this.
fn arm(rarity: Rarity, options: Option<i32>) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut args = json!({
                "query": crate::query::weapons_of(rarity),
                "times": param_or(&*ctx, "weapons", PICKS),
            });
            if let Some(options) = options {
                args["options"] = json!(options);
            }
            vec![equip_picks(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: arm(Rarity::Rare, Some(OPTIONS)),
        radiant: arm(Rarity::Epic, Some(OPTIONS)),
    }
}

/// R767000: through v0.6.6g each pick was a Discover of three (issue #4154); `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_6_6g() -> CardScripts {
    CardScripts {
        base: arm(Rarity::Rare, None),
        radiant: arm(Rarity::Epic, None),
    }
}

// Stinkyscape #83 Halo — SPEC §8.10 row 83, BUILD M12 row S 83 (#2521; R4154104, R1841002, R1751000,
// R1852001).
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

    /// The Rare and the Epic Weapons (§8.10 row 83).
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

    /// Halo played on the face `radiant`, each of its picks offering `options` of `pool` (fewer once the
    /// pool runs short), the first taken.
    fn arms(radiant: bool, pool: &[&str], options: usize) -> (Scenario, String) {
        let mut s = weapon_board(
            "s083",
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
                options.min(pool.len() - held.len()),
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
        fn r4154104_the_cry_offers_two_weapons_of_its_rarity_n_times_in_turn_and_equips_each() {
            let (s, host) = arms(false, &RARE, OPTIONS as usize);
            armed(&s, &host);
        }

        /// R767000: through v0.6.6g each pick was a Discover of three (issue #4154).
        #[test]
        fn r767000_r1862002_the_script_kept_for_v0_6_6g_discovers_from_three() {
            crate::under_kept(ID, script_v0_6_6g(), || {
                let (s, host) = arms(false, &RARE, DISCOVER_OPTIONS as usize);
                armed(&s, &host);
            });
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
        fn r4154104_the_cry_offers_two_epic_weapons_n_times_in_turn_and_equips_each() {
            let (s, host) = arms(true, &EPIC, OPTIONS as usize);
            armed(&s, &host);
        }
    }
}
