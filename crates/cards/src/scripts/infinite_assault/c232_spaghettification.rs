//! IA #232 Spaghettification (SPEC §8.13 row 232), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Exile an enemy Unit and every enemy Unit adjacent to it."
//!   Radiant: "Exile an enemy Unit and every enemy Unit adjacent to it, and Lock their zones."
//!
//! Adjacent is §3.1: the same row, lanes ±1, no wrap, never the target itself; an empty neighbour
//! contributes nothing. The exile pairs `exileAdjacentTo` over enemy Units in the unit row and the
//! backrow (an Animated one included) with a plain `exile` on the chosen Unit — adjacent first,
//! while the target is still in its lane for `adjacent` to read. Immune neighbours are passed by
//! (R4453000). The Radiant face first Locks the chosen and neighbour zones; the lock persists
//! after they leave.
//!
//! Engine: `exileAdjacentTo`, `exile`, `lock` (R4594011). Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-232";

/// The neighbours the exile reaches: enemy Units in the unit row and the backrow.
fn scope() -> BoardScope {
    json_as(json!({ "side": "enemy", "rows": ["units", "backrow"], "types": ["Unit"] }))
}

fn spaghettification(radiant: bool) -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))],
        cry: Some(hook(move |ctx| {
            let scope = scope();
            let chosen = TargetSpec::Chosen { index: None };
            let mut effects = Vec::new();
            if radiant {
                // The zones going to empty: the chosen one beside its neighbours.
                let mut zoned: Vec<(Row, i32)> = Vec::new();
                if let Some(target) = instance_of(ctx, &chosen)
                    && let Zone::Field { row, lane, .. } = target.zone
                {
                    zoned.push((row, lane));
                }
                for neighbour in adjacent_to_aimed(ctx, &chosen, &scope, TargetAim::Harm) {
                    if let Zone::Field { row, lane, .. } = neighbour.zone {
                        zoned.push((row, lane));
                    }
                }
                for (row, lane) in zoned {
                    effects.push(lock(json_as(json!({
                        "zone": { "of": "lane", "row": row, "lane": lane, "player": "enemy" },
                    }))));
                }
            }
            effects.push(exile_adjacent_to(json_as(json!({
                "target": { "of": "chosen" },
                "side": "enemy",
                "rows": ["units", "backrow"],
                "types": ["Unit"],
            }))));
            effects.push(exile(json_as(json!({ "target": { "of": "chosen" } }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: spaghettification(false), radiant: spaghettification(true) }
}

// IA #232 Spaghettification — SPEC §8.13 row 232 (R4594011).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P2;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const VICTIM: &str = "core-008"; // Mr. Vanilla: a 4/4.
    const IMMUNE: &str = "infiniteassault-235"; // The Prime Generator: Immune to Spells.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn cast(radiant: bool, seed: &str, p2_field: Value, lane: i32) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE], "library": crate::filler(4) },
            "p2": { "field": p2_field, "hand": [STOCKPILE], "library": crate::filler(4) },
        }));
        s.play(ID, json!({ "targets": pick::unit_at(&s, P2, lane) }));
        s
    }

    fn exiled(s: &Scenario) -> Vec<String> {
        s.pile(P2, "exile").into_iter().map(|card| card.def_id).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4594011_three_adjacent_are_exiled() {
            // The middle Unit's neighbours are both sides.
            let s = cast(false, "spaghetti-three", json!([VICTIM, VICTIM, VICTIM]), 2);
            assert_eq!(exiled(&s).len(), 3, "the chosen and both neighbours");
        }

        #[test]
        fn r4594011_an_edge_and_a_gap_exile_two() {
            let s = cast(
                false,
                "spaghetti-gap",
                json!([VICTIM, VICTIM, { "def": VICTIM, "lane": 4 }]),
                1,
            );
            // Lane 1's neighbours are lane 2 alone; lane 4 is no neighbour.
            assert_eq!(exiled(&s).len(), 2);
        }

        #[test]
        fn r4594011_an_immune_neighbour_is_passed_by() {
            let s = cast(false, "spaghetti-immune", json!([VICTIM, IMMUNE]), 1);
            assert_eq!(exiled(&s), vec![VICTIM.to_string()]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4594011_the_radiant_face_locks_the_emptied_zones() {
            let s = cast(true, "spaghetti-lock", json!([VICTIM, VICTIM]), 1);
            assert_eq!(exiled(&s).len(), 2);
            for lane in [1, 2] {
                assert!(
                    is_locked(
                        s.state(),
                        ZoneSlot { player: P2, row: Row::Units, lane }
                    ),
                    "lane {lane} stays locked"
                );
            }
        }
    }
}
