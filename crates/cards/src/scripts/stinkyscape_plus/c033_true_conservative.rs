//! S+ #33 True Conservative (SPEC §8.12 row 33). (4) Unit, Human, Legendary, 3/3 → 6/6.
//!   Base:    "Cry: Transform ALL Units into a version from their Past."
//!   Radiant: "Cry: Transform ALL Units into a version from their Past. Yours become their strongest version."
//!
//! Cry (R1): every Unit on either side with a past becomes a random one of its past versions
//! (`transform_random` over that card's own `past_ids_of`, R3538002, R129); a card with none is
//! untouched. The Radiant face makes its controller's Units their strongest past version instead
//! (R3538003). A Transform fires no Cry (R1) and an Immutable card refuses (R23).
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-033";

/// R3538003: the strongest of `past` for a unit wearing `wears_radiant`: the past Unit definition
/// with the highest attack + health on that face, then the highest cost, then the latest. `None`
/// when no past version is a Unit.
fn strongest(past: &[String], wears_radiant: bool) -> Option<String> {
    let mut best: Option<(i32, i32, usize, String)> = None;
    for (at, id) in past.iter().enumerate() {
        let Some(def) = past_def(id) else {
            continue;
        };
        if def.type_ != CardType::Unit {
            continue;
        }
        let face = if wears_radiant { &def.radiant } else { &def.base };
        let total = face.attack.unwrap_or(0) + face.health.unwrap_or(0);
        let key = (total, query_cost(def), at);
        if best.as_ref().is_none_or(|held| (held.0, held.1, held.2) < key) {
            best = Some((key.0, key.1, key.2, id.clone()));
        }
    }
    best.map(|held| held.3)
}

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let controller = ctx.controller;
            let mut units: Vec<(PlayerId, String, String, bool)> = Vec::new();
            for player in [controller, opponent_of(controller)] {
                for unit in active_units_of(&*ctx.state, player) {
                    units.push((player, unit.id.clone(), unit.def_id.clone(), unit.radiant));
                }
            }
            let mut effects = Vec::new();
            for (player, id, def_id, wears_radiant) in units {
                let past = past_ids_of(&def_id);
                if past.is_empty() {
                    continue;
                }
                if radiant && player == controller {
                    let Some(strong) = strongest(&past, wears_radiant) else {
                        continue;
                    };
                    effects.push(transform(json_as(json!({
                        "instanceId": id,
                        "defId": strong,
                        "radiant": wears_radiant,
                    }))));
                } else {
                    effects.push(transform_random(json_as(json!({
                        "instanceId": id,
                        "query": { "defId": past, "past": "only", "withTokens": true },
                        "radiant": "keep",
                    }))));
                }
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S+ #33 True Conservative — SPEC §8.12 row 33 (R3538003).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// Core #13 Jlockeed Shredder-10: a Unit with past versions.
    const OLD: &str = "core-013";
    /// A stand-in Unit, which has no past version.
    const NEW: &str = "stand-in (3) 9/9";

    fn board(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, "core-005"],
                "field": [{ "def": OLD, "lane": 1 }, { "def": NEW, "lane": 2 }],
                "library": ["core-005", "core-005", "core-005", "core-005"],
                "mana": 4,
            },
            "p2": {
                "hand": ["core-005"],
                "field": [{ "def": OLD, "lane": 1 }],
                "library": ["core-005", "core-005", "core-005", "core-005"],
            },
        }))
    }

    fn field_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        let mut defs: Vec<String> = (1..=5)
            .filter_map(|lane| s.unit(player, lane))
            .map(|unit| unit.def_id.clone())
            .collect();
        defs.sort();
        defs
    }

    #[test]
    fn r3538003_base_turns_each_unit_with_a_past_into_one_of_them_and_leaves_the_rest() {
        crate::register_all();
        let past = past_ids_of(OLD);
        assert!(!past.is_empty(), "core-013 has a past");
        let mut s = board(false, "true-conservative");
        s.play(ID, json!({ "zone": 3 }));
        // Yours: True Conservative itself (no past) and the stand-in stand on, core-013 became one
        // of its past versions.
        let mine = field_defs(&s, P1);
        assert!(mine.contains(&ID.to_string()), "{mine:?}");
        assert!(mine.contains(&NEW.to_string()), "{mine:?}");
        let turned: Vec<&String> = mine.iter().filter(|def| *def != ID && *def != NEW).collect();
        assert_eq!(turned.len(), 1, "{mine:?}");
        assert!(past.contains(turned[0]), "{mine:?}");
        // And the enemy's.
        let theirs = field_defs(&s, P2);
        assert_eq!(theirs.len(), 1, "{theirs:?}");
        assert!(past.contains(&theirs[0]), "{theirs:?}");
    }

    #[test]
    fn r3538003_radiant_gives_yours_their_strongest_past_and_the_enemy_a_random_one() {
        crate::register_all();
        let past = past_ids_of(OLD);
        assert!(!past.is_empty(), "core-013 has a past");
        let mut s = board(true, "true-conservative-radiant");
        s.play(ID, json!({ "zone": 3 }));
        let mine = field_defs(&s, P1);
        assert!(mine.contains(&ID.to_string()), "{mine:?}");
        assert!(mine.contains(&NEW.to_string()), "{mine:?}");
        let turned: Vec<&String> = mine.iter().filter(|def| *def != ID && *def != NEW).collect();
        assert_eq!(turned.len(), 1, "{mine:?}");
        assert_eq!(turned[0], &strongest(&past, false).expect("a past Unit"), "{mine:?}");
        let theirs = field_defs(&s, P2);
        assert_eq!(theirs.len(), 1, "{theirs:?}");
        assert!(past.contains(&theirs[0]), "{theirs:?}");
    }
}
