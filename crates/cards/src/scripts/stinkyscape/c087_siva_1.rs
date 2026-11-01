//! SPEC §8.10 Stinkyscape 87 Siva 1, a card of the Stinkyscape set.
//!   (2) Unit, Felinor, Acclaimed, Legendary, 2/2 → 4/4.
//!   Base:    "Tradable / Cry: Allure a random enemy Acclaimed Unit."
//!   Radiant: "Tradable / Cry: Allure a random enemy Acclaimed Unit. If there isn't one, summon a random
//!            Acclaimed Unit for your opponent, Frozen, and Allure it."
//! Tradable printed (R1838000). Cry allures random enemy Acclaimed Unit (R1864000, R1045000, R963, R437, R129).
//! Radiant summons random Acclaimed Unit for opponent if none (R1864001, R380, R387), Freezes and Allures it.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-087";

/// The step the engine's delayed Allure stores (`DELAYED_ALLURE_HOOK` resolves it by its data).
const ALLURE_STEP: &str = "unit";

/// R1864000: the enemy Units Allure reaches whose tags hold Acclaimed, in its order: the top of each
/// enemy unit pile in lane order, then the enemy Units standing in the backrow (R1081000), never a
/// Dormant one (R1839000).
fn enemy_acclaimed_units(ctx: &EffectContext<'_>) -> Vec<String> {
    let state: &GameState = &*ctx.state;
    let enemy = opponent_of(ctx.controller);
    let mut units: Vec<&CardInstance> = slots_of(enemy, Row::Units)
        .into_iter()
        .filter_map(|slot| card_at(state, slot))
        .collect();
    units.extend(standing_units_of(state, enemy));
    units
        .into_iter()
        .filter(|unit| !is_slumbering(unit) && tags_of(state, unit).contains(&Tag::Acclaimed))
        .map(|unit| unit.id.clone())
        .collect()
}

/// R1045000, R963: Allure one enemy Unit — stolen at the start of this player's next turn, destroyed
/// with no open zone in its row, aimed at its stay on the field and marked pink while it waits.
fn allure(instance_id: &str) -> Effect {
    delay(json_as(json!({
        "at": { "phase": "start", "player": "self" },
        "step": ALLURE_STEP,
        "hook": DELAYED_ALLURE_HOOK,
        "data": { "instanceId": instance_id },
        "watch": instance_id,
        "mark": { "mark": ALLURE_MARK, "color": ALLURE_MARK_COLOR },
    })))
}

/// R1864000: the one random pick, drawn as the clause resolves; none to pick draws nothing (R129).
fn one_enemy_acclaimed_unit(now: &mut EffectContext<'_>) -> Vec<String> {
    let ids = enemy_acclaimed_units(now);
    now.rng.pick(&ids).cloned().into_iter().collect()
}

/// R1864001: the Units this Cry summoned onto the opponent's side (R136).
fn summoned_enemy_units(now: &mut EffectContext<'_>) -> Vec<String> {
    let state: &GameState = &*now.state;
    let controller = now.controller;
    summoned_so_far(now)
        .into_iter()
        .filter(|id| find_instance(state, id).is_some_and(|unit| unit.controller != controller))
        .collect()
}

fn siva(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            if radiant && enemy_acclaimed_units(ctx).is_empty() {
                return vec![
                    summon_random(json_as(json!({
                        "query": { "type": "Unit", "tags": ["Acclaimed"] },
                        "player": "enemy",
                    }))),
                    for_each_card(ForEachCardArgs {
                        cards: Arc::new(summoned_enemy_units),
                        each: Arc::new(|instance_id: &str| -> Effect {
                            freeze(json_as(json!({ "target": { "of": "instance", "instanceId": instance_id } })))
                        }),
                    }),
                    for_each_card(ForEachCardArgs {
                        cards: Arc::new(summoned_enemy_units),
                        each: Arc::new(allure),
                    }),
                ];
            }
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(one_enemy_acclaimed_unit),
                each: Arc::new(allure),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: siva(false),
        radiant: siva(true),
    }
}

// S #87 Siva 1 — SPEC §8.10 row 87, BUILD M12 row S 87 (R62, R82).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, js, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const WARDRUM: &str = "classicplus-037";
    const BIG_MAX: &str = "classic-080";
    const YILEGPT: &str = "meditative-049";
    /// The Acclaimed Units of the sets that ship, Siva 1 aside (R387): the Stinkyscape set's own since
    /// its release (#1884).
    const SHIPPED_ACCLAIMED: [&str; 11] = [
        WARDRUM,
        BIG_MAX,
        YILEGPT,
        "stinkyscape-006",
        "stinkyscape-026",
        "stinkyscape-037",
        "stinkyscape-088",
        "stinkyscape-109",
        "stinkyscape-110",
        "stinkyscape-111",
        "stinkyscape-115",
    ];
    const VANILLA: &str = "core-008";
    const FILLER: &str = "core-005";

    fn siva_against(seed: &str, radiant: bool, p1_field: Value, p2_field: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": p1_field,
                "library": filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "field": p2_field, "library": filler(4) },
        }))
    }

    fn play_siva(s: &mut Scenario) {
        let siva = s.card(ID).id.clone();
        s.play(&siva, json!({}));
    }

    fn until_active(s: &mut Scenario, player: PlayerId) {
        if s.state().active != player {
            s.end_turn();
        }
        if s.state().active != player {
            s.end_turn();
        }
        assert_eq!(s.state().active, player);
    }

    fn allure_mark() -> Value {
        json!([{ "mark": ALLURE_MARK, "color": ALLURE_MARK_COLOR }])
    }

    fn marks_in(view: &PlayerView, instance_id: &str) -> Value {
        let view = js(view);
        for side in ["you", "opponent"] {
            for unit in view[side]["units"].as_array().into_iter().flatten() {
                if unit.get("instanceId").and_then(Value::as_str) == Some(instance_id) {
                    return unit.get("marks").cloned().unwrap_or(Value::Null);
                }
            }
        }
        json!("not on the field")
    }

    fn allured_in_both_views(s: &Scenario, instance_id: &str) -> bool {
        [P1, P2]
            .into_iter()
            .all(|seat| marks_in(&s.view(seat), instance_id) == allure_mark())
    }

    fn new_enemy_units(s: &Scenario, before: &[String]) -> Vec<CardInstance> {
        (1..=UNIT_ZONES)
            .filter_map(|lane| s.unit(P2, lane))
            .filter(|unit| !before.contains(&unit.id))
            .collect()
    }

    fn enemy_unit_ids(s: &Scenario) -> Vec<String> {
        (1..=UNIT_ZONES)
            .filter_map(|lane| s.unit(P2, lane).map(|unit| unit.id))
            .collect()
    }

    #[test]
    fn r1864000_the_cry_allures_the_enemy_acclaimed_unit_and_steals_it_next_turn() {
        for radiant in [false, true] {
            let mut s = siva_against(
                "siva-allure",
                radiant,
                json!([]),
                json!([{ "def": WARDRUM, "lane": 2 }, { "def": VANILLA, "lane": 3 }]),
            );
            let wardrum = s.unit_at(P2, 2).id.clone();
            let vanilla = s.unit_at(P2, 3).id.clone();
            let before = enemy_unit_ids(&s);
            play_siva(&mut s);
            assert!(allured_in_both_views(&s, &wardrum), "radiant {radiant}: Wardrum is marked");
            assert_eq!(marks_in(&s.view(P1), &vanilla), Value::Null, "a Unit with no Acclaimed is not");
            assert!(new_enemy_units(&s, &before).is_empty(), "radiant {radiant}: one to Allure, so nothing summoned");
            assert_eq!(s.card(&wardrum).controller, P2, "nothing is stolen on the turn it is played");

            until_active(&mut s, P2);
            until_active(&mut s, P1);
            assert_eq!(s.card(&wardrum).controller, P1, "radiant {radiant}: stolen at p1's next start");
            assert_eq!(s.card(&vanilla).controller, P2);
            assert!(s.state().delayed.is_empty());
        }
    }

    #[test]
    fn r1864000_a_granted_acclaimed_tag_counts() {
        let mut s = siva_against("siva-granted", false, json!([]), json!([{ "def": VANILLA, "lane": 4 }]));
        let vanilla = s.unit_at(P2, 4).id.clone();
        s.card_mut(&vanilla).granted_tags = Some(vec![Tag::Acclaimed]);
        play_siva(&mut s);
        assert!(allured_in_both_views(&s, &vanilla), "its granted Acclaimed makes it the pick");
    }

    #[test]
    fn r1864000_one_random_pick_among_several() {
        let two_wardrums = |seed: &str| {
            siva_against(
                seed,
                false,
                json!([]),
                json!([{ "def": WARDRUM, "lane": 2 }, { "def": WARDRUM, "lane": 4 }]),
            )
        };
        let marked_lane = |seed: &str| -> i32 {
            let mut s = two_wardrums(seed);
            let (left, right) = (s.unit_at(P2, 2).id.clone(), s.unit_at(P2, 4).id.clone());
            play_siva(&mut s);
            match (allured_in_both_views(&s, &left), allured_in_both_views(&s, &right)) {
                (true, false) => 2,
                (false, true) => 4,
                other => panic!("{seed}: exactly one Wardrum is Allured, not {other:?}"),
            }
        };
        let left = seed_where("siva-pick-left", |seed| marked_lane(seed) == 2);
        let right = seed_where("siva-pick-right", |seed| marked_lane(seed) == 4);
        assert_eq!(marked_lane(&left), 2);
        assert_eq!(marked_lane(&right), 4);
        let mut s = two_wardrums(&left);
        play_siva(&mut s);
        assert_eq!(s.state().delayed.len(), 1);
    }

    #[test]
    fn r1864000_r129_with_none_the_base_face_does_nothing() {
        let mut s = siva_against("siva-none", false, json!([]), json!([{ "def": VANILLA, "lane": 2 }]));
        let before = enemy_unit_ids(&s);
        let cursor = s.state().rng_cursor;
        play_siva(&mut s);
        assert_eq!(s.state().rng_cursor, cursor, "nothing to pick draws nothing (R129)");
        assert!(s.state().delayed.is_empty());
        assert!(new_enemy_units(&s, &before).is_empty(), "the base face summons nothing");
    }

    #[test]
    fn r1864000_with_no_room_the_allured_unit_is_destroyed() {
        let mut s = siva_against(
            "siva-no-room",
            false,
            json!([
                { "def": VANILLA, "lane": 1 },
                { "def": VANILLA, "lane": 2 },
                { "def": VANILLA, "lane": 3 },
                { "def": VANILLA, "lane": 4 },
            ]),
            json!([{ "def": WARDRUM, "lane": 2 }]),
        );
        let wardrum = s.unit_at(P2, 2).id.clone();
        play_siva(&mut s);
        assert!(allured_in_both_views(&s, &wardrum));
        until_active(&mut s, P2);
        until_active(&mut s, P1);
        assert!(
            s.state().players.p2.graveyard.iter().any(|card| card.id == wardrum),
            "no room on p1's side: destroyed (R1045000)"
        );
    }

    #[test]
    fn r1864001_radiant_with_none_summons_a_frozen_acclaimed_unit_for_the_opponent_and_allures_it() {
        let mut seen: Vec<String> = Vec::new();
        for i in 0..12 {
            let mut s = siva_against(&format!("siva-radiant-{i}"), true, json!([]), json!([{ "def": VANILLA, "lane": 1 }]));
            let before = enemy_unit_ids(&s);
            play_siva(&mut s);
            let summoned = new_enemy_units(&s, &before);
            assert_eq!(summoned.len(), 1, "one Acclaimed Unit summoned for the opponent");
            let unit = summoned[0].clone();
            assert!(
                SHIPPED_ACCLAIMED.contains(&unit.def_id.as_str()),
                "{} is a shipped Acclaimed Unit",
                unit.def_id
            );
            assert_ne!(unit.def_id, ID, "never Siva 1 itself (R387)");
            assert_eq!(unit.controller, P2);
            assert!(s.card(&unit.id).frozen.is_some(), "it arrives Frozen");
            assert!(allured_in_both_views(&s, &unit.id), "and Allured");
            until_active(&mut s, P2);
            until_active(&mut s, P1);
            assert_eq!(s.card(&unit.id).controller, P1, "stolen at p1's next start");
            if !seen.contains(&unit.def_id) {
                seen.push(unit.def_id.clone());
            }
        }
        assert!(seen.len() > 1, "the pool is random: {seen:?}");
    }

    #[test]
    fn r1864001_radiant_with_a_full_enemy_row_summons_nothing() {
        let full: Vec<Value> = (1..=UNIT_ZONES)
            .map(|lane| json!({ "def": VANILLA, "lane": lane }))
            .collect();
        let mut s = siva_against("siva-full", true, json!([]), json!(full));
        let before = enemy_unit_ids(&s);
        let cursor = s.state().rng_cursor;
        play_siva(&mut s);
        assert_eq!(enemy_unit_ids(&s), before, "no room: nothing summoned");
        assert_eq!(s.state().rng_cursor, cursor, "and no draw (R129)");
        assert!(s.state().delayed.is_empty());
    }

    #[test]
    fn r1838000_both_faces_are_tradable_in_hand() {
        for radiant in [false, true] {
            let mut s = siva_against("siva-trade", radiant, json!([]), json!([]));
            let siva = s.card(ID).clone();
            assert!(is_tradable(s.state(), &siva), "radiant {radiant}: the face prints Tradable");
            s.trade(&siva.id);
            s.expect_events(json!(["traded", "shuffledIn", "drawn"]));
        }
    }
}
