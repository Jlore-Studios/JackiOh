//! IA #239 Zero-Day (SPEC §8.13 row 239), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Immune to Spells. Cry: Steal every enemy (2) Cost or less Unit. Aura: Your
//!            opponent's Spells have their targets chosen randomly."
//!   Radiant: "Immune to Spells. Cry: Steal every enemy (3) Cost or less Unit. Aura: Your
//!            opponent's Spells have their targets chosen randomly."
//!
//! Immune to Spells is catalog data. The Cry steals every reachable enemy Unit on the field
//! costing no more than the cap now (`cost_now`), in lane order — with no room the card is
//! destroyed where it stands instead (§6.3, R4205000). The aura is `random_enemy_spell_targets`:
//! the opponent's Spells' declared targets and target prompts are drawn at random from the legal
//! ones (R4594014); modes and Discover are not targets. A stolen Zero-Day applies to its new
//! opponent.
//!
//! Engine: `steal`, `random_enemy_spell_targets` (R4594014). Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-239";

/// The Cry steals Units costing this or less now.
const CAP: i32 = 2;
const RADIANT_CAP: i32 = 3;

fn zero_day(cap: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            random_enemy_spell_targets: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |ctx| {
            let enemy = opponent_of(ctx.controller);
            active_units_of(&*ctx.state, enemy)
                .into_iter()
                .filter(|card| {
                    cost_now(&*ctx.state, card) <= cap
                        && reaches(&*ctx.state, ctx.self_.as_ref(), card)
                })
                .map(|card| steal(json_as(json!({ "instanceId": card.id }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: zero_day(CAP), radiant: zero_day(RADIANT_CAP) }
}

// IA #239 Zero-Day — SPEC §8.13 row 239 (R4594014).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const CHEAP: &str = "stand-in (1) 2/2";
    const MID: &str = "stand-in (3) 5/5";
    const DEAR: &str = "stand-in (5) 9/9";
    const FILLER: &str = "core-008"; // Mr. Vanilla: a cheap Unit of another def, so `stolen` skips it.
    const LUNAR: &str = "core-035"; // Lunar Eclipse: deal 3 damage to a target.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn stolen(s: &Scenario) -> Vec<String> {
        s.state().players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .filter(|card| card.def_id == CHEAP || card.def_id == MID || card.def_id == DEAR)
            .map(|card| card.def_id.clone())
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4594014_cheap_units_are_stolen_in_lane_order_dear_ones_kept() {
            let mut s = scenario(json!({
                "seed": "zero-day-cry",
                "p1": { "hand": [{ "def": ID }, STOCKPILE], "library": crate::filler(4) },
                "p2": { "field": [CHEAP, DEAR], "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            let first = s.unit_at(P2, 1).id.clone();
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(stolen(&s), vec![CHEAP.to_string()]);
            // Lane order: the one steal names lane 1's Unit. A field steal reports
            // `ControlChanged` (only hand and library takes are `Stolen`).
            let steals: Vec<String> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::ControlChanged { instance_id, .. } => Some(instance_id.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(steals, vec![first]);
        }

        #[test]
        fn r4594014_a_full_board_destroys_the_pick() {
            let mut s = scenario(json!({
                "seed": "zero-day-full",
                "p1": {
                    "field": [FILLER, FILLER, FILLER, FILLER],
                    "hand": [{ "def": ID }, STOCKPILE],
                    "library": crate::filler(4),
                },
                "p2": { "field": [CHEAP, CHEAP], "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            s.play(ID, json!({ "zone": 5 }));
            // Four fillers plus Zero-Day fill the five zones: both picks are destroyed where
            // they stand instead of joining a full board.
            assert_eq!(s.state().players[P1].units.iter().flatten().flatten().count(), 5);
            assert_eq!(s.state().players[P2].units.iter().flatten().flatten().count(), 0);
            assert!(stolen(&s).is_empty());
        }

        #[test]
        fn r4594014_an_enemy_targeted_spell_lists_no_targets() {
            let mut s = scenario(json!({
                "seed": "zero-day-random",
                "active": "p1",
                "p1": { "hand": [LUNAR], "library": crate::filler(4) },
                "p2": { "field": [{ "def": ID }], "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            let plays = s.plays_of(LUNAR);
            assert_eq!(plays.len(), 1, "the Spell is listed once");
            assert!(
                matches!(&plays[0], ActionBody::Play { targets: None, .. }),
                "with no targets"
            );
            s.play(LUNAR, json!({}));
        }

        #[test]
        fn r4594014_own_spells_keep_theirs() {
            let mut s = scenario(json!({
                "seed": "zero-day-own",
                "active": "p2",
                "p1": { "hand": [STOCKPILE], "library": crate::filler(4) },
                "p2": { "field": [{ "def": ID }], "hand": [LUNAR], "library": crate::filler(4) },
            }));
            s.play(LUNAR, json!({ "targets": pick::hero("p1") }));
            assert_eq!(s.state().players[P1].hero.health, 30 - 3, "as declared");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4594014_the_radiant_cry_steals_up_to_3() {
            let mut s = scenario(json!({
                "seed": "zero-day-radiant",
                "p1": { "hand": [{ "def": ID, "radiant": true }, STOCKPILE], "library": crate::filler(4) },
                "p2": { "field": [CHEAP, MID, DEAR], "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            s.play(ID, json!({ "zone": 1 }));
            let mut took = stolen(&s);
            took.sort();
            assert_eq!(took, vec![CHEAP.to_string(), MID.to_string()]);
        }
    }
}
