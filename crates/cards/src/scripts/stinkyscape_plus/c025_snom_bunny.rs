//! SPEC §8.12 S+ #25 Snom Bunny, a card of the Stinkyscape+ set (R1420, R3535000).
//! A new card, not the shipped Snom Bunny Mind Control.
//!   Base:    5/5, "Can't be in defense position"; "Death: A random enemy Permanent comes under
//!            your control."
//!   Radiant: 10/10, Lucky 1; "Death:" the same.
//!
//! Engine: the Death takes a random enemy permanent (E6's take control, R3550009) through the
//! match rng. On the Radiant face the draw is luck-based (R3567005): it draws again for its
//! Lucky plus its controller's Luck (R1438) and keeps the dearer permanent (`effective_cost`),
//! the earlier draw on a tie. Can't Defend and Lucky 1 are catalog data (R2657000).
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-025";

fn snom_bunny() -> Script {
    Script {
        death: Some(hook(|ctx| {
            let enemies = enemy_permanents(ctx.state, ctx.controller);
            if enemies.is_empty() {
                return Vec::new();
            }
            let state: &GameState = &*ctx.state;
            let costs: Vec<i32> = enemies
                .iter()
                .map(|card| effective_cost(state, card, Default::default()))
                .collect();
            let n = enemies.len() as i32;
            let lucky =
                ctx.self_.as_ref().map_or(0, |me| lucky_on(state, me)) + luck_of(state, ctx.controller);
            let index = if lucky > 0 {
                ctx.rng.lucky(
                    lucky,
                    |rng| rng.int(n),
                    |a, b| {
                        if costs[b as usize] > costs[a as usize] {
                            b
                        } else {
                            a
                        }
                    },
                )
            } else {
                ctx.rng.int(n)
            };
            vec![steal(json_as(
                json!({ "instanceId": enemies[index as usize].id.clone() }),
            ))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: snom_bunny(),
        radiant: snom_bunny(),
    }
}

// S+ #25 Snom Bunny — SPEC §8.12 row 25 (R3550009, R2657000, R3567005).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::cursor_where;

    use crate::{filler, scenario, weapon_board};

    const P1: PlayerId = PlayerId::P1;

    /// A 6/4 Unit whose name holds no M, O, U, S or E.
    const TYLANNT: &str = "stinkyscape-006"; // Tylannt
    /// A cheap enemy permanent and a dear one.
    const CHEAP: &str = "stand-in (1) 1/1";
    const DEAR: &str = "stand-in (5) 5/5";
    /// Hit Job: Destroy target Unit.
    const HIT_JOB: &str = "core-016"; // Hit Job

    /// Kills the Bunny with a Hit Job, so its Death resolves: the def of the enemy
    /// permanent it took, if any.
    fn death_takes(seed: &str, radiant: bool, enemies: Value) -> Option<String> {
        let mut s = weapon_board(
            seed,
            json!({
                "mana": 10,
                "hand": [HIT_JOB],
                "field": [{ "def": ID, "radiant": radiant }],
                "library": filler(4),
            }),
            json!({ "field": enemies, "library": filler(4) }),
        );
        let bunny = s.unit_at(P1, 1).id;
        s.play(
            HIT_JOB,
            json!({ "targets": [{ "pick": "instance", "instanceId": bunny }] }),
        );
        (1..=5)
            .filter_map(|lane| s.unit(P1, lane))
            .find(|unit| unit.def_id == CHEAP || unit.def_id == DEAR)
            .map(|unit| unit.def_id)
    }

    /// `death_takes` against `enemies` in lanes 1 and 2, with p1's `backrow` beside the Bunny and the
    /// match rng set so that the Death's next two draws over the two would be `draws`: the def it took.
    fn death_takes_drawing(
        radiant: bool,
        backrow: Value,
        enemies: [&str; 2],
        draws: (i32, i32),
    ) -> Option<String> {
        let mut s = weapon_board(
            "s025-drawn",
            json!({
                "mana": 10,
                "hand": [HIT_JOB],
                "field": [{ "def": ID, "radiant": radiant }],
                "backrow": backrow,
                "library": filler(4),
            }),
            json!({ "field": enemies, "library": filler(4) }),
        );
        let seed = s.state().seed.clone();
        let cursor = cursor_where(&seed, |cursor| {
            let mut rng = Rng::new(&seed, cursor);
            (rng.int(2), rng.int(2)) == draws
        });
        s.state_mut().rng_cursor = cursor;
        let bunny = s.unit_at(P1, 1).id;
        s.play(
            HIT_JOB,
            json!({ "targets": [{ "pick": "instance", "instanceId": bunny }] }),
        );
        assert_eq!(s.state().rng_cursor, cursor + 2, "the Death's two draws and no other");
        (1..=5)
            .filter_map(|lane| s.unit(P1, lane))
            .find(|unit| enemies.contains(&unit.def_id.as_str()))
            .map(|unit| unit.def_id)
    }

    #[test]
    fn r3567005_on_a_tie_of_cost_the_radiant_death_keeps_the_earlier_draw() {
        // Two enemies of the same cost: the first draw stands, whichever lane it named.
        const FIRST: &str = "stand-in (1) 1/1";
        const SECOND: &str = "stand-in (1) 2/2";
        for (draws, kept) in [((0, 1), FIRST), ((1, 0), SECOND)] {
            let taken = death_takes_drawing(true, json!([]), [FIRST, SECOND], draws);
            assert_eq!(taken, Some(kept.to_string()), "draws {draws:?}");
        }
    }

    /// R3567005, R987: its controller's Luck draws that many more times beside the card's own Lucky, so the
    /// base face too keeps the dearer permanent. No card gives Luck since R3140000, so Feng Shui's kept
    /// v0.5.11r script, "You have Luck 1", lends it.
    #[test]
    fn r3567005_its_controllers_luck_draws_again_and_keeps_the_dearer_permanent() {
        const FENG_SHUI: &str = "meditative-040"; // Feng Shui: "You have Luck 1" through v0.5.11r
        crate::under_kept(FENG_SHUI, crate::scripts::meditative::c040_feng_shui::script_v0_5_11o(), || {
            let taken = death_takes_drawing(false, json!([FENG_SHUI]), [CHEAP, DEAR], (0, 1));
            assert_eq!(taken, Some(DEAR.to_string()), "the second draw, the dearer, is kept");
        });
    }

    #[test]
    fn the_base_face_can_t_defend() {
        let mut s = scenario(json!({
            "seed": "s025-defend",
            "p1": { "mana": 10, "hand": [ID, TYLANNT], "library": filler(4) },
            "p2": { "hand": [TYLANNT], "library": filler(4) },
        }));
        s.play(TYLANNT, json!({ "zone": 2 }));
        s.play(ID, json!({ "zone": 1 }));
        let bunny = s.unit_at(P1, 1).id;
        let control = s.unit_at(P1, 2).id;
        s.end_turn();
        s.end_turn();
        s.switch_position(&control);
        s.expect_refused(|s| s.switch_position(&bunny));
    }

    #[test]
    fn its_death_takes_the_enemy_permanent() {
        for radiant in [false, true] {
            let taken = death_takes("s025-take", radiant, json!([DEAR]));
            assert_eq!(taken, Some(DEAR.to_string()));
        }
    }

    #[test]
    fn its_death_with_no_enemy_permanent_takes_nothing() {
        for radiant in [false, true] {
            let taken = death_takes("s025-empty", radiant, json!([]));
            assert_eq!(taken, None);
        }
    }

    #[test]
    fn r3567005_the_radiant_lucky_roll_takes_the_dearer_permanent_more_often() {
        let mut base = 0;
        let mut radiant = 0;
        for n in 0..20 {
            let enemies = json!([CHEAP, DEAR]);
            if death_takes(&format!("s025-lucky-base-{n}"), false, enemies.clone()) == Some(DEAR.to_string())
            {
                base += 1;
            }
            if death_takes(&format!("s025-lucky-radiant-{n}"), true, enemies) == Some(DEAR.to_string()) {
                radiant += 1;
            }
        }
        assert!(
            radiant > base,
            "the Radiant face takes the dearer permanent more often ({radiant} vs {base})"
        );
    }
}
