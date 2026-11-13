//! SPEC §8.12 S+ #84 Bogos Binted, a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    5/6; "Cry: Take a snapshot of all your other permanents. Death: Reset your board
//!            to that state."
//!   Radiant: 10/12; "Cry:" the same; "Death: Reset your board to that state but they're Radiant."
//!
//! Engine: E6's side snapshot and its restore (R3550007): the Cry keeps every other permanent
//! of its controller's side on its own memory, and the Death rebuilds that side from it —
//! Radiant on the Radiant face — with no Cry.
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-084";

fn bogos(radiant: bool) -> Script {
    Script {
        cry: Some(hook(|_ctx| vec![snapshot_side()])),
        death: Some(hook(move |_ctx| {
            vec![restore_side_snapshot(RestoreSideArgs {
                radiant: radiant.then_some(true),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: bogos(false),
        radiant: bogos(true),
    }
}

// S+ #84 Bogos Binted — SPEC §8.12 row 84 (R3550007).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    /// Hit Job: Destroy target Unit.
    const HIT_JOB: &str = "core-016"; // Hit Job

    /// Kills `id` with a Hit Job, so its Death resolves.
    fn hit_job(s: &mut Scenario, id: &str) {
        s.play(
            HIT_JOB,
            json!({ "targets": [{ "pick": "instance", "instanceId": id }] }),
        );
    }

    #[test]
    fn its_death_resets_your_side_to_the_snapshot() {
        for radiant in [false, true] {
            let mut s = weapon_board(
                "s084-restore",
                json!({
                    "mana": 10,
                    "field": [VANILLA],
                    "hand": [{ "def": ID, "radiant": radiant }, VANILLA, HIT_JOB],
                    "library": filler(4),
                }),
                json!({}),
            );
            s.play(ID, json!({ "zone": 2 }));
            let bogos = s.unit_at(P1, 2).id;
            // After the Cry, a second vanilla joins: the side no longer matches the snapshot.
            s.play(VANILLA, json!({ "zone": 3 }));
            hit_job(&mut s, &bogos);
            // The snapshot's cards stand where they stood; the extra is back in hand.
            assert!(s.unit(P1, 2).is_none(), "Bogos is gone");
            assert!(s.unit(P1, 3).is_none(), "the extra left the field");
            assert!(
                s.hand(P1).iter().any(|card| card.def_id == VANILLA),
                "the extra went back to hand"
            );
        }
    }

    #[test]
    fn the_radiant_death_rebuilds_it_radiant() {
        let mut s = weapon_board(
            "s084-radiant",
            json!({
                "mana": 10,
                "field": [VANILLA],
                "hand": [{ "def": ID, "radiant": true }, HIT_JOB],
                "library": filler(4),
            }),
            json!({}),
        );
        s.play(ID, json!({ "zone": 2 }));
        let bogos = s.unit_at(P1, 2).id;
        hit_job(&mut s, &bogos);
        assert!(
            s.card(&s.unit_at(P1, 1).id).radiant,
            "the rebuilt side is Radiant"
        );
    }
}
