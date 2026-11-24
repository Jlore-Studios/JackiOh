//! SPEC §8.13 IA #119 Snapshot Lich ((4) Unit, Legendary, 6/10 → 12/20), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Take a snapshot of your permanents.\nStart of turn: If you have fewer permanents than the snapshot, reset your board to it."
//!   Radiant: "Cry: Take a snapshot of your permanents.\nStart of turn: If you have fewer permanents than the snapshot, reset your board to it, and draw a card."
//!
//! S+ #84 Bogos Binted's snapshot: the Cry keeps `snapshot_side` on its memory. At its controller's
//! start of turn it returns nothing unless `side_shrank` counts fewer permanents — the Lich's other
//! permanents, counted the way R3550007 counts them (R4589006) — than the note holds; then
//! `reset_side_keeping_self` rebuilds the side with the Lich left standing and its note kept, so it
//! may reset again later. The Radiant face also draws a card.

use jackioh_engine::effects::{draw, snapshot_side};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-119";

/// "draw a card", the Radiant face's.
const DRAW: i32 = 1;

fn lich(radiant: bool) -> Script {
    Script {
        cry: Some(hook(|_ctx| vec![snapshot_side()])),
        start_of_turn: Some(hook(move |ctx| {
            let view: &EffectContext = ctx;
            let Some(live) = view.live_self().cloned() else {
                return vec![];
            };
            if !side_shrank(view.state, &live) {
                return vec![];
            }
            let mut effects = vec![reset_side_keeping_self()];
            if radiant {
                effects.push(draw(json_as(json!({ "count": DRAW }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: lich(false),
        radiant: lich(true),
    }
}

// IA #119 Snapshot Lich — SPEC §8.13 row 119 (R4589006).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::effects::SIDE_SNAPSHOT_KEY;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const FODDER: &str = "stand-in (1) 2/2";
    const FILLER: &str = "stand-in (0) Spell";

    /// p1 plays the Lich beside its Fodder, kills the Fodder, and passes two turns: its next
    /// start of turn resets the side.
    fn shrunk(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FODDER, HIT_JOB],
                "field": [],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        let lich = s.unit_at(P1, 1).id.clone();
        s.play(FODDER, json!({ "zone": 2 }));
        let fodder = s.unit_at(P1, 2);
        s.play(HIT_JOB, pick::targets(pick::instance(&fodder)));
        s.end_turn();
        s.end_turn();
        // The assertions read the Lich by its id: it never left its zone.
        assert_eq!(s.unit_at(P1, 1).id, lich, "the Lich keeps its zone");
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r4589006_resets_when_fewer() {
            let s = shrunk(false);
            assert_eq!(s.unit_at(P1, 2).def_id, FODDER, "its snapshot is back");
            assert!(
                s.card(s.unit_at(P1, 1).id.as_str()).memory.contains_key(SIDE_SNAPSHOT_KEY),
                "its note is kept, so it may reset again later"
            );
            assert_eq!(s.hand(P1).len(), 1, "one turn draw, no reset draw");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4589006_radiant_resets_and_draws() {
            let s = shrunk(true);
            assert_eq!(s.unit_at(P1, 2).def_id, FODDER, "its snapshot is back");
            assert_eq!(s.hand(P1).len(), 2, "one turn draw and the reset's draw");
        }
    }
}
