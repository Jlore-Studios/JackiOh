//! S+ #33 True Conservative (SPEC §8.12 row 33). (4) Unit, Human, Legendary, 4/4 → 8/8 (3/3 → 6/6 through v0.6.7k, issue #4285).
//!   Base:    "Cry: Transform ALL Units into a version from their Past."
//!   Radiant: "Cry: Transform ALL Units into a version from their Past. Yours become their strongest version."
//!
//! Cry (R1): every Unit on either side with a past becomes a random one of its past versions
//! (`transform_random` over that card's own `past_ids_of`, R3538002, R129); a card with none is
//! untouched. The Radiant face makes its controller's Units their strongest past version instead
//! (R3538003). A Transform fires no Cry (R1) and an Immutable card refuses (R23).
//! No True Conservative, this one or another on either side, becomes one of its own past versions on
//! either face (R4321000): the base face's pool leaves them out (R387), and the Radiant face skips them.
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
                    // R387, R4321000: never one of this card's own versions, as the base face's pool
                    // leaves them out.
                    if past_card_of(&def_id) == ID {
                        continue;
                    }
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
        // Yours: True Conservative itself (never its own past, R4321000) and the stand-in (no past)
        // stand on, core-013 became one of its past versions.
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

    /// R4321000: neither face turns a True Conservative into one of its own past versions, the one
    /// played or another on either side, though its crafted past here (9/9, Radiant 18/18) is its
    /// strongest; core-013 beside it still turns, so the Cry ran.
    #[test]
    fn r4321000_neither_face_turns_a_true_conservative_into_its_own_past() {
        crate::register_all();
        let past_of = |card: &str, attack: i32, health: i32| {
            let mut def = crate::card_def(card).clone();
            def.id = format!("{card}{PAST_ID_MARK}v0");
            def.base.attack = Some(attack);
            def.base.health = Some(health);
            def.radiant.attack = Some(attack * 2);
            def.radiant.health = Some(health * 2);
            def
        };
        let old_past = past_of(OLD, 1, 6);
        register_past_defs(vec![old_past.clone(), past_of(ID, 9, 9)]);
        for radiant in [false, true] {
            let mut s = scenario(json!({
                "seed": format!("true-conservative-itself-{radiant}"),
                "p1": {
                    "hand": [{ "def": ID, "radiant": radiant }, "core-005"],
                    "field": [{ "def": OLD, "lane": 1 }, { "def": ID, "lane": 2 }],
                    "library": ["core-005", "core-005", "core-005", "core-005"],
                    "mana": 4,
                },
                "p2": {
                    "hand": ["core-005"],
                    "field": [{ "def": ID, "lane": 1 }],
                    "library": ["core-005", "core-005", "core-005", "core-005"],
                },
            }));
            s.play(ID, json!({ "zone": 3 }));
            let def_at = |player: PlayerId, lane: i32| s.unit(player, lane).map(|unit| unit.def_id.clone());
            assert_eq!(def_at(P1, 1), Some(old_past.id.clone()), "radiant {radiant}: the Cry ran");
            for (player, lane) in [(P1, 2), (P1, 3), (P2, 1)] {
                assert_eq!(
                    def_at(player, lane),
                    Some(ID.to_string()),
                    "radiant {radiant}: {player:?}'s lane {lane}"
                );
            }
        }
    }

    /// Core #13's past as `stats`, oldest first, each its base face's `(attack, health)` at its cost,
    /// in place of the catalog's history (R3538002, the testkit's `register_past_defs`): their ids.
    fn crafted_past(stats: &[(i32, i32)]) -> Vec<String> {
        crate::register_all();
        let defs: Vec<CardDef> = stats
            .iter()
            .enumerate()
            .map(|(at, &(attack, health))| {
                let mut def = crate::card_def(OLD).clone();
                def.id = format!("{OLD}{PAST_ID_MARK}v{at}");
                def.base.attack = Some(attack);
                def.base.health = Some(health);
                def
            })
            .collect();
        let ids = defs.iter().map(|def| def.id.clone()).collect();
        register_past_defs(defs);
        ids
    }

    /// The defs p1's three core-013s (lanes 1, 2 and 4) and p2's four (lanes 1 to 4) became.
    fn turned(radiant: bool, seed: &str) -> (Vec<String>, Vec<String>) {
        let old = |lane: i32| json!({ "def": OLD, "lane": lane });
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, "core-005"],
                "field": [old(1), old(2), old(4)],
                "library": ["core-005", "core-005", "core-005", "core-005"],
                "mana": 4,
            },
            "p2": {
                "hand": ["core-005"],
                "field": [old(1), old(2), old(3), old(4)],
                "library": ["core-005", "core-005", "core-005", "core-005"],
            },
        }));
        s.play(ID, json!({ "zone": 3 }));
        let defs = |player: PlayerId, lanes: &[i32]| -> Vec<String> {
            lanes.iter().filter_map(|&lane| s.unit(player, lane)).map(|unit| unit.def_id.clone()).collect()
        };
        (defs(P1, &[1, 2, 4]), defs(P2, &[1, 2, 3, 4]))
    }

    #[test]
    fn r3538003_the_strongest_past_has_the_highest_attack_plus_health_wherever_it_stands() {
        // A 3/3 then a 1/6, at one cost: 6 against 7, so the later 1/6 (where a product would rank the
        // 3/3's 9 over its 6).
        let past = crafted_past(&[(3, 3), (1, 6)]);
        let mut s = board(true, "true-conservative-strongest");
        s.play(ID, json!({ "zone": 3 }));
        assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id.clone()), Some(past[1].clone()));
    }

    #[test]
    fn r3538003_radiant_gives_only_its_controllers_units_their_strongest_the_enemys_stay_random() {
        let past = crafted_past(&[(3, 3), (1, 6), (2, 2)]);
        let mut theirs: IndexSet<String> = IndexSet::new();
        for n in 0..5 {
            let (mine, enemy) = turned(true, &format!("true-conservative-sides-{n}"));
            assert_eq!(mine, vec![past[1].clone(); 3], "yours, the 1/6");
            assert!(enemy.iter().all(|def| past.contains(def)), "{enemy:?}");
            theirs.extend(enemy);
        }
        assert!(theirs.len() > 1, "the enemy's are random past versions: {theirs:?}");
    }

    #[test]
    fn r3538003_the_base_face_turns_its_controllers_units_into_random_past_versions_too() {
        let past = crafted_past(&[(3, 3), (1, 6), (2, 2)]);
        let mut yours: IndexSet<String> = IndexSet::new();
        for n in 0..5 {
            let (mine, enemy) = turned(false, &format!("true-conservative-base-sides-{n}"));
            assert!(mine.iter().chain(&enemy).all(|def| past.contains(def)), "{mine:?} {enemy:?}");
            yours.extend(mine);
        }
        assert!(yours.len() > 1, "yours are random past versions, not the strongest: {yours:?}");
    }
}
