//! SPEC §8.13 IA #71 Thermal Throttler ((2) Unit, Legendary, 3/6 → 6/12), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Start of turn: Gain {mana} mana this turn for each {health} Health your hero is missing." (mana 1, health 10)
//!   Radiant: "Start of turn: Gain {mana} mana this turn for each {health} Health your hero is missing." (mana 1, health 5)
//!
//! Core #6 Mana Well's shape with a count: at its controller's start of turn (§2.2, §6.2, R68) the
//! declared `mana` for each declared `health` its hero is missing — missing from the hero's
//! starting health (R290, a handicap's `heroHealth`, R4587004), floored at 0, so a hero above its
//! start gives nothing. The gain is this turn's mana (`gain_mana`, §2.3).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-071";

pub fn script() -> CardScripts {
    let base = Script {
        start_of_turn: Some(hook(|ctx| {
            // R4587004: missing from the starting health, floored at 0; rejected: R72's fixed 30.
            let missing =
                (starting_health_of(ctx.state, ctx.controller) - hero_of(ctx.state, ctx.controller).health)
                    .max(0);
            let ticks = missing / param(&*ctx, "health");
            if ticks <= 0 {
                return vec![];
            }
            vec![gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") * ticks })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 5 is its declared `health`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #71 Thermal Throttler — SPEC §8.13 row 71 (R4587004).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    /// The Throttler on p1's field, and p1's turn started.
    fn turn_starts(radiant: bool, health: i32) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "field": [{ "def": ID, "radiant": radiant }],
                "health": health,
                "library": crate::filler(2),
            },
            "p2": { "library": crate::filler(2) },
        }));
        s.start_turn();
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r4587004_one_mana_for_each_10_missing() {
            // 21 missing: two ticks of 1.
            let mut s = turn_starts(false, 9);
            s.expect_mana(P1, 6);
        }

        #[test]
        fn r4587004_above_the_start_it_gives_nothing() {
            let mut s = turn_starts(false, 35);
            s.expect_mana(P1, 4);
        }

        #[test]
        fn r4587004_a_handicap_hero_counts_from_its_starting_health() {
            let mut s = scenario(json!({
                "p1": { "field": [ID], "health": 30, "library": crate::filler(2) },
                "p2": { "library": crate::filler(2) },
            }));
            s.state_mut().players[P1].handicap = Some(Handicap {
                hero_health: Some(40),
                ..HUMAN_HANDICAP
            });
            s.start_turn();
            // 10 missing from the handicap's 40: one tick of 1.
            s.expect_health(P1, 30);
            s.expect_mana(P1, 5);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4587004_one_mana_for_each_5_missing() {
            // 11 missing: two ticks of 1.
            let mut s = turn_starts(true, 19);
            s.expect_mana(P1, 6);
        }
    }
}
