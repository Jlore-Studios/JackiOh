//! Stinkyscape #83 Halo (SPEC §8.10 row 83). Unit 3/1 → 6/1, cost 4, Epic.
//!   Base:    "Cry: Discover a Rare Weapon and Equip it into this, {weapons|time|times}." — weapons 2
//!   Radiant: "Cry: Discover an Epic Weapon and Equip it into this, {weapons|time|times}." — weapons 2
//!
//! R4285003: the Cry is one `equip_picks` (R1841002), a standard Discover of three (§6) of the Weapon-tagged
//! cards of the rarity, the pool naming its set so that it reaches them whether or not the set ships (R1420,
//! R1857000), and the one card picked is fused into this `weapons` times (`copies`), each its own fusion
//! (R102) of a fresh copy, not made Radiant, with its Activate (R1751000): no Cry, no hand. The fusion wears
//! the face its kept host wears (R77), so a Radiant Halo wields its Weapons' Radiant faces. Its Radiant 6/1
//! on a 3/1 base is R275's named exception (R4285006). Since #2521 it is no Starship Piece, and the Weapons
//! it takes, each a piece, leave it none (R1852001), so its death fuses nothing (R1852000 reads the
//! keyword). Kept for replays (R767000): through v0.6.7k it was a 2/2 (Radiant 4/4) making `weapons` (3)
//! picks in turn, each a choice of two, "Choose one of 2 Rare Weapons and Equip it into this, 3 times"
//! (`script_v0_6_7k`, issue #4285); through v0.6.6h each pick was a Discover of three, "Discover 3 Rare
//! Weapons" (`script_v0_6_6h`, issue #4154).

use jackioh_engine::effects::equip_picks;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-083";

/// R4285003: "Equip it into this, twice": the declared `weapons`, how many times the one pick is fused in.
const COPIES: i32 = 2;
/// "Discover 3": the picks through v0.6.7k, the declared `weapons` since #2657 (R2657006), printed before,
/// as a replay of an older version reads it (`param_or`).
const PICKS: i32 = 3;
/// R4154104: "choose one of 2": each pick's offer, from issue #4154 through v0.6.7k.
const OPTIONS: i32 = 2;

/// R4285003: one Discover of `rarity` Weapons, its pick Equipped into this `weapons` times.
fn arm_twice(rarity: Rarity) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![equip_picks(json_as(json!({
                "query": crate::query::weapons_of(rarity),
                "copies": param_or(&*ctx, "weapons", COPIES),
            })))]
        })),
        ..Script::default()
    }
}

/// R1862002, R4154104: through v0.6.7k, PICKS times, a choice of `options` `rarity` Weapons (a Discover's
/// three when `None`, through v0.6.6h), Equipped into this.
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
        base: arm_twice(Rarity::Rare),
        radiant: arm_twice(Rarity::Epic),
    }
}

/// R767000, R4285003: after v0.6.6h through v0.6.7k the Cry made `weapons` picks in turn, each a choice of
/// two (issue #4285); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_7k() -> CardScripts {
    CardScripts {
        base: arm(Rarity::Rare, Some(OPTIONS)),
        radiant: arm(Rarity::Epic, Some(OPTIONS)),
    }
}

/// R767000: after v0.5.9f through v0.6.6h each pick was a Discover of three (issue #4154); `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_6_6h() -> CardScripts {
    CardScripts {
        base: arm(Rarity::Rare, None),
        radiant: arm(Rarity::Epic, None),
    }
}

/// R767000, R2657006: the same Discover of three through v0.5.9f, kept apart because those versions
/// declared no `weapons` (it printed "Discover 3", which `param_or` reads); `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_5_9f() -> CardScripts {
    script_v0_6_6h()
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

    /// The catalog row through v0.6.7k: 2/2 (Radiant 4/4), `weapons` 3 (issue #4285).
    fn old_row(def: &mut CardDef) {
        def.params = serde_json::from_value(json!([
            { "key": "weapons", "base": PICKS, "radiant": PICKS, "better": "up", "step": 1, "min": 1, "read": "cry" }
        ]))
        .expect("the v0.6.7k params");
        def.base.attack = Some(2);
        def.base.health = Some(2);
        def.radiant.attack = Some(4);
        def.radiant.health = Some(4);
    }

    /// Halo played on the face `radiant`: `picks` Discovers in turn, each offering `options` of `pool`
    /// (fewer once the pool runs short), the first taken and fused in `copies` times.
    fn arms(radiant: bool, pool: &[&str], options: usize, picks: i32, copies: i32) -> (Scenario, String) {
        let mut s = weapon_board(
            "s083",
            json!({ "hand": [{ "def": ID, "radiant": radiant }] }),
            json!({ "field": [VANILLA] }),
        );
        s.play(ID, json!({ "zone": 1 }));
        let host = s.unit_at(P1, 1).id;
        let mut held: Vec<String> = Vec::new();
        for _ in 0..picks {
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
            let fusions = fused_count(&s);
            s.answer(json!(offered[0]));
            // R77: the fusion wears the face its kept host wears, so a Radiant host takes its picks'
            let def = crate::card_def(&offered[0]);
            let gained = if radiant {
                def.radiant.health
            } else {
                def.base.health
            }
            .unwrap_or_default();
            assert_eq!(
                s.stats(host.as_str()).health,
                before + copies * gained,
                "{}",
                offered[0]
            );
            // R102: each copy is its own fusion.
            assert_eq!(fused_count(&s), fusions + copies as usize, "{}", offered[0]);
            held.push(offered[0].clone());
        }
        assert!(s.state().pending.is_none());
        (s, host)
    }

    fn fused_count(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Fused { .. }))
            .count()
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
        fn r4285003_one_discover_of_three_its_pick_fused_twice() {
            let (s, host) = arms(false, &RARE, DISCOVER_OPTIONS as usize, 1, COPIES);
            armed(&s, &host);
        }

        #[test]
        fn r4285006_it_is_a_3_1() {
            let mut s = crate::scenario(json!({
                "p1": { "field": [ID], "library": crate::filler(4) },
                "p2": { "library": crate::filler(4) },
            }));
            s.expect_stats(ID, json!({ "attack": 3, "health": 1 }));
        }

        /// R767000, R4285003: through v0.6.7k a 2/2 whose Cry offered two Weapons of its rarity, three
        /// times in turn, and equipped each once (issue #4285).
        #[test]
        fn r767000_r4154104_the_script_kept_for_v0_6_7k_offers_two_weapons_three_times() {
            crate::under_kept_as(ID, script_v0_6_7k(), old_row, || {
                let (s, host) = arms(false, &RARE, OPTIONS as usize, PICKS, 1);
                armed(&s, &host);
            });
        }

        /// R767000: through v0.6.6h each pick was a Discover of three (issue #4154).
        #[test]
        fn r767000_r1862002_the_script_kept_for_v0_6_6h_discovers_from_three() {
            crate::under_kept_as(ID, script_v0_6_6h(), old_row, || {
                let (s, host) = arms(false, &RARE, DISCOVER_OPTIONS as usize, PICKS, 1);
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
        fn r4285003_one_discover_of_three_epic_weapons_its_pick_fused_twice() {
            let (s, host) = arms(true, &EPIC, DISCOVER_OPTIONS as usize, 1, COPIES);
            armed(&s, &host);
        }

        #[test]
        fn r4285006_it_is_a_6_1() {
            let mut s = crate::scenario(json!({
                "p1": { "field": [{ "def": ID, "radiant": true }], "library": crate::filler(4) },
                "p2": { "library": crate::filler(4) },
            }));
            s.expect_stats(ID, json!({ "attack": 6, "health": 1 }));
        }

        /// R767000: through v0.6.7k the Radiant face offered two Epic Weapons, three times in turn.
        #[test]
        fn r767000_r4154104_the_script_kept_for_v0_6_7k_offers_two_epic_weapons_three_times() {
            crate::under_kept_as(ID, script_v0_6_7k(), old_row, || {
                let (s, host) = arms(true, &EPIC, OPTIONS as usize, PICKS, 1);
                armed(&s, &host);
            });
        }
    }
}
