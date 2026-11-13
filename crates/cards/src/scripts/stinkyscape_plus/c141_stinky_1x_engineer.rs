//! SPEC §8.12 S+ #141 Stinky 1x Engineer, a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    2/3, "Cry: Nerf {nerf} other random permanents."
//!   Radiant: 4/6, "Cry: Destroy {nerf} other random permanents."
//!
//! Engine: "other random permanents" is both fields minus itself (R3553003): the base face `degrade`s
//! `nerf` of them at random (C+ #70's shape: uniform over the scope, different cards, R60, R129),
//! the Radiant face `destroy`s that many, each pick drawn with `ctx.rng` as the Cry resolves, one
//! `destroy` per pick in one state check (R59). A hand and a library are never in scope. (The voice
//! line, "Prod is down. Report the intern.", is S26's render; S+ has no text/audio entries yet.)

use jackioh_engine::effects::{cards_in_scope_aimed, degrade, destroy};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-141";

/// "Nerf {nerf} other random permanents" over both fields minus itself.
fn nerf_others() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![degrade(json_as(json!({
                "scope": { "side": "any", "zones": ["field"], "excludeSelf": true },
                "random": param(&*ctx, "nerf"),
            })))]
        })),
        ..Script::default()
    }
}

/// "Destroy {nerf} other random permanents" over both fields minus itself (#43's shape).
fn destroy_others() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let scope: BoardScope =
                json_as(json!({ "side": "any", "rows": ["units", "backrow"], "excludeSelf": true }));
            let mut left = cards_in_scope_aimed(&*ctx, &scope, TargetAim::Harm);
            let mut picked = Vec::new();
            for _ in 0..param(&*ctx, "nerf") {
                if left.is_empty() {
                    break;
                }
                let at = ctx.rng.int(left.len() as i32) as usize;
                picked.push(left.remove(at));
            }
            picked
                .iter()
                .map(|unit| destroy(json_as(json!({ "target": { "of": "instance", "instanceId": unit.id } }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: nerf_others(),
        radiant: destroy_others(),
    }
}

// S+ #141 Stinky 1x Engineer — SPEC §8.12 row S+ 141 (R3553003; R59, R60, R129).
// Cry Nerfs (base) or destroys (Radiant) 3 other random permanents.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const SHELL: &str = "core-073"; // Anti-oneshot Armor: a Field Spell for the backrow
    const FILLER: &str = "core-005"; // Stockpile: a Spell (§2.5).

    /// The engineer waits in p1's hand; `p1_field`/`p1_backrow`/`p2_field`/`p2_backrow` stand already.
    fn engineer_board(radiant: bool, seed: &str, p1_field: Value, p1_backrow: Value, p2_field: Value, p2_backrow: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": p1_field,
                "backrow": p1_backrow,
                "library": [FILLER],
            },
            "p2": { "field": p2_field, "backrow": p2_backrow, "hand": [FILLER], "library": [FILLER] },
        }))
    }

    fn degraded_ids(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Degraded { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn destroyed_ids(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Destroyed { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// Five other permanents: a unit and a backrow card of p1's, two units and a backrow card of p2's.
    fn wide(radiant: bool, seed: &str) -> Scenario {
        engineer_board(
            radiant,
            seed,
            json!([{ "def": VANILLA, "lane": 1 }]),
            json!([SHELL]),
            json!([{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }]),
            json!([SHELL]),
        )
    }

    /// Exactly three other permanents: p1's unit, p2's unit and p2's backrow card.
    fn narrow(radiant: bool, seed: &str) -> Scenario {
        engineer_board(
            radiant,
            seed,
            json!([{ "def": VANILLA, "lane": 1 }]),
            json!([]),
            json!([{ "def": VANILLA, "lane": 1 }]),
            json!([SHELL]),
        )
    }

    fn others(s: &Scenario) -> Vec<String> {
        let mut ids: Vec<String> = Vec::new();
        for player in [P1, P2] {
            for lane in 1..=5 {
                if let Some(card) = s.unit(player, lane)
                    && card.def_id != ID
                {
                    ids.push(card.id);
                }
            }
            for lane in 1..=2 {
                if let Some(card) = s.backrow(player, lane) {
                    ids.push(card.id);
                }
            }
        }
        ids
    }

    mod base {
        use super::*;

        #[test]
        fn r3553003_other_random_permanents_is_both_fields_minus_self() {
            let mut s = narrow(false, "s141-scope");
            let others = others(&s);
            assert_eq!(others.len(), 3);
            let hand_and_library: Vec<String> = s
                .hand(P1)
                .into_iter()
                .chain(s.pile(P1, "library"))
                .chain(s.hand(P2))
                .chain(s.pile(P2, "library"))
                .map(|card| card.id)
                .collect();

            s.play(ID, json!({ "zone": 2 }));

            // All three others, no more: the hand and the library are never in scope.
            let mut degraded = degraded_ids(&s);
            degraded.sort();
            let mut want = others.clone();
            want.sort();
            assert_eq!(degraded, want);
            for id in &hand_and_library {
                assert!(!degraded.contains(id), "{id} is no permanent");
            }
            s.expect_in_zone(ID, "field");
        }

        #[test]
        fn base_nerfs_3_leaving_self_standing() {
            for n in 0..10 {
                let seed = format!("s141-base-{n}");
                let mut s = wide(false, &seed);
                let me = s.card(ID).id.clone();
                let others = others(&s);
                assert_eq!(others.len(), 5);

                s.play(ID, json!({ "zone": 2 }));

                let degraded = degraded_ids(&s);
                assert_eq!(degraded.len(), 3, "{seed}");
                assert!(!degraded.contains(&me), "{seed}: never itself");
                let distinct: IndexSet<&String> = degraded.iter().collect();
                assert_eq!(distinct.len(), 3, "{seed}: three different permanents");
                s.expect_in_zone(ID, "field");
            }
        }

        #[test]
        fn fewer_than_3_nerfs_them_all() {
            let mut s = engineer_board(
                false,
                "s141-few",
                json!([{ "def": VANILLA, "lane": 1 }]),
                json!([]),
                json!([{ "def": VANILLA, "lane": 1 }]),
                json!([]),
            );
            s.play(ID, json!({ "zone": 2 }));
            assert_eq!(degraded_ids(&s).len(), 2);
            s.expect_in_zone(ID, "field");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn radiant_destroys_3_and_self_survives() {
            for n in 0..10 {
                let seed = format!("s141-radiant-{n}");
                let mut s = wide(true, &seed);
                let me = s.card(ID).id.clone();

                s.play(ID, json!({ "zone": 2 }));

                let destroyed = destroyed_ids(&s);
                assert_eq!(destroyed.len(), 3, "{seed}");
                assert!(!destroyed.contains(&me), "{seed}");
                let distinct: IndexSet<&String> = destroyed.iter().collect();
                assert_eq!(distinct.len(), 3, "{seed}: three different permanents");
                s.expect_in_zone(ID, "field");
                s.expect_stats(ID, json!({ "attack": 4, "health": 6, "maxHealth": 6 }));
            }
        }
    }
}
