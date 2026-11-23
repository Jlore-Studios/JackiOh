//! S+ #155 The Far Vagabond (SPEC §8.12 row S+ #155; §6.3 Summon Starships; R3539000, R3539001):
//! (4) Spell, Mythic.
//!   Base:    "Summon a random Terminus Starship. It can be launched for (3)."
//!   Radiant: "Summon a random Terminus Starship. It can be launched for (0)."
//!
//! Engine: one Terminus Starship summoned holding its fixed loadout (`summon_starships`, R3539001),
//! priced for its launch by hand (R3539000): (3), or (0) on the Radiant face. Issue #3951 cut the
//! loadouts to 5 or 6 mana of Starship Pieces; `script_v0_6_4c` summons those of v0.6.4c and before
//! (R767000). The two prices are the
//! PRICE numbers `tests/cross/params.rs` r1748003 lists.

use jackioh_engine::prelude::*;

use crate::scripts::stinkyscape_plus::c080_the_last_voyage as voyage;

pub const ID: &str = "stinkyscapeplus-155";

/// R3539000: the price of its Starship's launch by hand.
const LAUNCH_PRICE: i32 = 3;
/// R3539000: the Radiant face's Starship launches for nothing.
const RADIANT_LAUNCH_PRICE: i32 = 0;

fn vagabond(price: i32, blueprints: fn() -> Vec<StarshipBlueprint>) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![summon_starships(json_as(json!({
                "blueprints": blueprints(),
                "count": 1,
                "price": price,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: vagabond(LAUNCH_PRICE, voyage::blueprints),
        radiant: vagabond(RADIANT_LAUNCH_PRICE, voyage::blueprints),
    }
}

/// R767000: the scripts through v0.6.4c, summoning the Terminus Starships' loadouts of then (issue
/// #3951). `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_4c() -> CardScripts {
    CardScripts {
        base: vagabond(LAUNCH_PRICE, voyage::blueprints_v0_6_4c),
        radiant: vagabond(RADIANT_LAUNCH_PRICE, voyage::blueprints_v0_6_4c),
    }
}

// S+ #155 The Far Vagabond — SPEC §8.12 row S+ #155 (R3539000, R3539001).
#[cfg(test)]
mod tests {
    use super::{ID, LAUNCH_PRICE, RADIANT_LAUNCH_PRICE};
    use crate::{filler, scenario};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const VAGABOND: &str = ID;
    const FILLER: &str = "core-005"; // Stockpile: a Spell

    fn vagabonding(radiant: bool) -> Scenario {
        let hand = if radiant {
            json!([{ "def": VAGABOND, "radiant": true }])
        } else {
            json!([VAGABOND])
        };
        scenario(json!({
            "seed": "vagabond",
            "p1": { "hand": hand, "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    #[test]
    fn r3539000_summons_a_terminus_starship_that_launches_for_3() {
        let mut s = vagabonding(false);
        s.play(VAGABOND, json!({}));
        let ship = dormant_starship_of(s.state(), P1)
            .cloned()
            .expect("a Terminus Starship");
        let decls = jackioh_engine::subsystems::activate::abilities_of(s.state(), &ship);
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].label, format!("Launch ({LAUNCH_PRICE})"));
        let mana = s.state().players.p1.mana.current;
        s.activate(ship.id.as_str(), json!({ "ability": STARSHIP_LAUNCH_ABILITY }));
        assert_eq!(s.state().players.p1.mana.current, mana - LAUNCH_PRICE);
        assert!(is_launched_starship(s.card(&ship.id)));
    }

    #[test]
    fn radiant_its_starship_launches_for_0() {
        let mut s = vagabonding(true);
        s.play(VAGABOND, json!({}));
        let ship = dormant_starship_of(s.state(), P1)
            .cloned()
            .expect("a Terminus Starship");
        let decls = jackioh_engine::subsystems::activate::abilities_of(s.state(), &ship);
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].label, format!("Launch ({RADIANT_LAUNCH_PRICE})"));
        let mana = s.state().players.p1.mana.current;
        s.activate(ship.id.as_str(), json!({ "ability": STARSHIP_LAUNCH_ABILITY }));
        assert_eq!(s.state().players.p1.mana.current, mana - RADIANT_LAUNCH_PRICE);
        assert!(is_launched_starship(s.card(&ship.id)));
    }
}
