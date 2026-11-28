//! SPEC §8.14 FvH #167 Plagiarized: Thermal Throttler, a frozen copy of IA #71 Thermal Throttler (R2195003,
//! FV4), a card of the held Felinors vs Humans set, built by #5235.
//!
//! The code is a copy of the IA card's as it stood when the set was founded, not a re-export, so a later IA
//! patch does not move it (R2195003). Its `ID` is its own, so every read of it names this card; it is named
//! `Plagiarized: Thermal Throttler`, tagged Plagiarized only, and a token it makes stays the IA token. Its
//! change from IA #71: none. The IA file's notes, as they stood:
//!
//! SPEC §8.13 IA #71 Thermal Throttler ((2) Unit, Legendary, 3/6 → 6/12), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Start of turn: Gain {mana} mana this turn for each {health} Health your hero is missing." (mana 1, health 10)
//!   Radiant: "Start of turn: Gain {mana} mana this turn for each {health} Health your hero is missing." (mana 1, health 5)
//!
//! Core #6 Mana Well's shape with a count: at its controller's start of turn (§2.2, §6.2, R68) the
//! declared `mana` for each declared `health` its hero is missing — missing from the hero's
//! starting health (R290, a handicap's `heroHealth`, R4587004; a Duos seat's own HP at setup,
//! R4831000), floored at 0, so a hero above its
//! start gives nothing. The gain is this turn's mana (`gain_mana`, §2.3).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-167";

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

// FvH #167 Plagiarized: Thermal Throttler — SPEC §8.14 row 167; IA #71's tests (SPEC §8.13 row 71, R4587004),
// on this card.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::config::DUOS_POOL_HEALTH;
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

        #[test]
        fn r4831000_a_duos_hero_counts_from_the_own_hp_it_started_on() {
            let mut s = scenario(json!({
                "p1": { "field": [ID], "health": 9, "library": crate::filler(2) },
                "p2": { "library": crate::filler(2) },
            }));
            s.state_mut().players[P1].duos = Some(DuosLife {
                pool: DUOS_POOL_HEALTH,
                pool_cap: DUOS_POOL_HEALTH,
                spent: false,
                own_start: None,
            });
            s.start_turn();
            // 11 missing from its own 20, not 21 from 30: one tick of 1.
            s.expect_health(P1, 9);
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
