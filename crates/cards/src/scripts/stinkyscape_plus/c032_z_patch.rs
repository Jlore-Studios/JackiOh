//! SPEC §8.12 S+ #32 Z-Patch, a card of the Stinkyscape+ set (R1420, R3535000). (3) Spell, Epic.
//!   Base:    "Tradable. Buff every permanent {times} times. Then Nerf every permanent {times2} times."
//!   Radiant: the same, "Then Nerf every enemy permanent {times2} times."
//!
//! Tradable is catalog data. Every permanent is a card on the field, a Unit or a backrow card, of
//! either side (§5.1); the Buffs come first, the Nerfs after them, each an Upgrade or Degrade of
//! `times` separate changes (R386). A card that cannot be changed is skipped. The Radiant face's Nerfs
//! reach the caster's opponent's permanents alone, its Buffs still every one. `times` and `times2` are
//! the declared numbers, 5 each (R1748003).

use jackioh_engine::effects::{degrade, upgrade};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-032";

fn patch(nerfs: &'static str) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![
                upgrade(json_as(json!({
                    "scope": { "side": "any", "zones": ["field"] },
                    "times": param(ctx, "times"),
                }))),
                degrade(json_as(json!({
                    "scope": { "side": nerfs, "zones": ["field"] },
                    "times": param(ctx, "times2"),
                }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: patch("any"),
        radiant: patch("enemy"),
    }
}

// S+ #32 Z-Patch — SPEC §8.12 row 32.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    const BODY: &str = "core-019"; // Midrange Menace

    fn board(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": "z-patch",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": [BODY], "library": filler(3), "mana": 10 },
            "p2": { "hand": [FILLER], "field": [BODY], "library": filler(4) },
        }));
        s.play(ID, json!({}));
        s
    }

    /// The ids of a unit's `upgraded` and `degraded` events, in order.
    fn changes(s: &Scenario, id: &str) -> (usize, usize) {
        let of = |kind: &str| {
            s.events()
                .iter()
                .filter(|event| event.event_type().as_str() == kind)
                .map(crate::js)
                .filter(|event| event["instanceId"] == id)
                .count()
        };
        (of("upgraded"), of("degraded"))
    }

    #[test]
    fn base_buffs_every_permanent_five_times_then_nerfs_every_permanent_five_times() {
        crate::register_all();
        let s = board(false);
        for id in [s.unit_at(P1, 1).id, s.unit_at(P2, 1).id] {
            assert_eq!(changes(&s, &id), (5, 5));
        }
        let order: Vec<&str> = s
            .events()
            .iter()
            .map(|event| event.event_type().as_str())
            .filter(|kind| *kind == "upgraded" || *kind == "degraded")
            .collect();
        let first_nerf = order.iter().position(|kind| *kind == "degraded").unwrap_or(0);
        assert!(order[..first_nerf].iter().all(|kind| *kind == "upgraded") && first_nerf == 10);
    }

    #[test]
    fn radiant_nerfs_only_the_enemy_permanents() {
        crate::register_all();
        let s = board(true);
        assert_eq!(changes(&s, &s.unit_at(P1, 1).id), (5, 0));
        assert_eq!(changes(&s, &s.unit_at(P2, 1).id), (5, 5));
    }
}
