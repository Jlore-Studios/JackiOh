//! SPEC §8.13 IA #166 Aegis Prime ((4) Unit, Legendary, 6/12 → 12/24), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Taunt. Aura: While you control another Taunt Unit, your hero is immune to damage."
//!   Radiant: "Taunt. Aura: Your hero is immune to damage."
//!
//! Engine: a hero guard (as Argusland's): immunity is a damage cap of 0, judged at each hit (§4.4
//! step 3). On the base face it holds while another of the controller's Units has Taunt — read
//! through the layers (R4591006), so a Unit in Defense Position counts, never itself; on the
//! Radiant face it always holds.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-166";

/// Immunity as Argusland's guard reads it: a hero damage cap of 0 (MD-C21).
const IMMUNE_CAP: i32 = 0;

fn aegis(radiant: bool) -> Script {
    Script {
        hero_guard: Some(read_hook(move |args| {
            if !radiant {
                // R4591006: "another Taunt Unit" reads layered keywords, so a Unit in Defense
                // counts; never itself.
                let another = active_units_of(args.state, args.self_.controller)
                    .iter()
                    .any(|unit| unit.id != args.self_.id && unit_has(args.state, unit, KeywordKind::Taunt));
                if !another {
                    return vec![];
                }
            }
            vec![HeroGuard {
                cap: Some(IMMUNE_CAP),
                ..HeroGuard::default()
            }]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: aegis(false),
        radiant: aegis(true),
    }
}

// IA #166 Aegis Prime — SPEC §8.13 row 166 (R4591006).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const TAUNT: &str = "stand-in (1) 2/2 Taunt";
    const UNIT: &str = "stand-in (1) 2/2";
    const TRUE_STRIKE: &str = "core-044";

    fn board(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "seed": "aegis-prime",
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": field,
                "library": crate::filler(3),
                "mana": 8,
            },
            "p2": { "hand": [TRUE_STRIKE], "library": crate::filler(3), "mana": 8 },
        }))
    }

    fn strike_hero(s: &mut Scenario) {
        s.play(TRUE_STRIKE, json!({ "targets": pick::hero("p1") }));
    }

    mod base {
        use super::*;

        #[test]
        fn r4591006_with_another_taunt_the_hero_takes_no_damage() {
            let mut s = board(false, json!([TAUNT]));
            s.play(ID, json!({ "zone": 2 }));
            s.end_turn();
            strike_hero(&mut s);
            s.expect_health(P1, 30);
        }

        #[test]
        fn r4591006_without_another_taunt_the_hero_takes_damage() {
            let mut s = board(false, json!([UNIT]));
            s.play(ID, json!({ "zone": 2 }));
            s.end_turn();
            strike_hero(&mut s);
            s.expect_health(P1, 26);
        }

        #[test]
        fn r4591006_a_defense_position_unit_counts_as_taunt() {
            let mut s = board(false, json!([{ "def": UNIT, "position": "DEF" }]));
            s.play(ID, json!({ "zone": 2 }));
            s.end_turn();
            strike_hero(&mut s);
            s.expect_health(P1, 30);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_hero_is_always_immune() {
            let mut s = board(true, json!([UNIT]));
            s.play(ID, json!({ "zone": 2 }));
            s.end_turn();
            strike_hero(&mut s);
            s.expect_health(P1, 30);
        }
    }
}
