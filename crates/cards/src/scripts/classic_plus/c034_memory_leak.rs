//! C+ #34 Memory Leak (SPEC §8.7 row 34): (3) Field Spell, Epic.
//!   Base:    Choose one: "At the end of your turn, Lock a random zone on your opponent's side"; or
//!            "After your opponent plays a Unit or Field Spell, Lock its zone."
//!   Radiant: both, no choice.
//! The mode is declared with the play (R81) and remembered on the instance (`memory.mode`). "A random
//! zone" is one of the opponent's ten not Locked yet (`lockRandomZone`, nothing when all are); "its
//! zone" is where the played card landed, Locked once it resolves (a cast counts, R70).

use jackioh_engine::effects::{chosen_options, lock_played_zone, lock_random_zone, remember};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-034";

const MODE: &str = "mode";
const END_OF_TURN: &str = "endOfTurn";
const AFTER_PLAY: &str = "afterPlay";

/// TS's `ctx & { event }` is the context and the event, two arguments here.
fn opponent_played_unit_or_field_spell(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::CardResolved { player, def_id, .. } = event else {
        return false;
    };
    if *player == ctx.controller {
        return false;
    }
    let type_ = def_of(Some(&*ctx.state), def_id).type_;
    type_ == CardType::Unit || type_ == CardType::FieldSpell
}

fn memory_leak<F>(has: F) -> Script
where
    F: Fn(&EffectContext<'_>, &str) -> bool + Clone + Send + Sync + 'static,
{
    let at_end = has.clone();
    Script {
        end_of_turn: Some(hook(move |ctx| {
            if at_end(&*ctx, END_OF_TURN) {
                vec![lock_random_zone(json_as(json!({ "side": "enemy" })))]
            } else {
                vec![]
            }
        })),
        triggers: vec![TriggerDef::new("memory-leak", &[GameEventType::CardResolved], move |ctx, event| {
            // A trigger that is no trap reads its condition in `run` (only traps consult `when`, R99).
            if has(&*ctx, AFTER_PLAY) && opponent_played_unit_or_field_spell(&*ctx, event) {
                vec![lock_played_zone(json_as(json!({ "event": event })))]
            } else {
                vec![]
            }
        })],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: vec![END_OF_TURN.to_string(), AFTER_PLAY.to_string()],
        }],
        cry: Some(hook(|ctx| {
            let mode = chosen_options(ctx).into_iter().next();
            match mode.as_deref() {
                Some(mode) if mode == END_OF_TURN || mode == AFTER_PLAY => {
                    vec![remember(json_as(json!({ "key": MODE, "value": mode })))]
                }
                _ => vec![],
            }
        })),
        ..memory_leak(|ctx, mode| recalled(ctx, MODE).is_some_and(|value| value.as_str() == Some(mode)))
    };
    let radiant = memory_leak(|_ctx, _mode| true);
    CardScripts { base, radiant }
}

// C+ #34 Memory Leak — SPEC §8.7 row 34, BUILD M9 Classic+ row C+ 34: "Field Spell; the mode is chosen
// with the play (R81) and kept in `memory.mode`: "End of turn" Locks one random zone of the opponent's
// ten not already Locked at each end of your turn (an occupied zone is fine, a Lock evicts nothing; all
// ten Locked, nothing and no random draw, R129); "After your opponent plays a Unit or Field Spell" Locks
// the zone that card went into (a cast counts, R70; a Spell, Trap or Field Trap doesn't; the card
// stays); a Locked unit zone takes a Reborn return there (R175, R688); radiant both at once, no choice".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const LEAK: &str = "classicplus-034";
    const WARDRUM: &str = "classicplus-037";
    const BONE_STORM: &str = "classicplus-036-1"; // (1) Spell: 1 damage to each enemy.
    const DEFENDER: &str = "core-003"; // 1/1 Taunt, Divine Shield, Reborn.
    const MENACE: &str = "core-019"; // (3) 9/9.
    const MANA_WELL: &str = "core-006"; // (3) Field Spell.
    const LUNAR_ECLIPSE: &str = "core-035"; // (1) Spell: 3 damage to a target.
    const SHEEPISH: &str = "core-041"; // (1) Trap.
    const BREAD: &str = "core-018"; // (1) Field Trap.
    const FILLER: &str = "core-005";

    /// TS `type Lane = { row: "units" | "backrow"; lane: number }`, as its JSON.
    fn lane(row: &str, lane: i32) -> Value {
        json!({ "row": row, "lane": lane })
    }

    fn locked_of(s: &Scenario, player: PlayerId) -> Vec<Value> {
        let view = s.view(P1);
        let locks = if player == P1 { view.you.locks } else { view.opponent.locks };
        let units = locks.units.iter().enumerate().filter(|(_, on)| **on).map(|(i, _)| lane("units", i as i32 + 1));
        let backrow = locks.backrow.iter().enumerate().filter(|(_, on)| **on).map(|(i, _)| lane("backrow", i as i32 + 1));
        units.chain(backrow).collect()
    }

    /// p1 plays Memory Leak (base with `mode`, or the Radiant face with none). `p2` may set `hand`,
    /// `field` and `backrow`.
    fn leak(mode: Option<&str>, p2: Value) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": LEAK, "radiant": mode.is_none() }, FILLER],
                "library": [FILLER, FILLER, FILLER, FILLER],
                "mana": 4,
            },
            "p2": {
                "hand": p2.get("hand").cloned().unwrap_or_else(|| json!([FILLER])),
                "field": p2.get("field").cloned().unwrap_or_else(|| json!([])),
                "backrow": p2.get("backrow").cloned().unwrap_or_else(|| json!([])),
                "library": [FILLER, FILLER, FILLER, FILLER],
            },
        }));
        let options = match mode {
            None => json!({}),
            Some(mode) => json!({ "modes": [mode] }),
        };
        s.play(LEAK, options);
        s
    }

    fn locked_events(events: &[GameEvent]) -> Vec<&GameEvent> {
        events.iter().filter(|event| matches!(event, GameEvent::Locked { .. })).collect()
    }

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).unwrap()
    }

    /// TS `toMatchObject` on a flat object.
    fn matches_object(actual: &Value, pattern: &Value) -> bool {
        match (actual, pattern) {
            (Value::Object(actual), Value::Object(pattern)) => pattern
                .iter()
                .all(|(key, want)| actual.get(key).is_some_and(|have| matches_object(have, want))),
            _ => actual == pattern,
        }
    }

    mod c_n34_memory_leak {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r81_declares_its_two_modes_and_keeps_the_chosen_one_on_the_instance() {
                crate::register_all();
                let s = leak(Some("endOfTurn"), json!({}));
                assert_eq!(s.card(LEAK).memory.get("mode"), Some(&json!("endOfTurn")));
                assert_eq!(s.card(LEAK).zone.z(), ZoneName::Field);
                assert_eq!(leak(Some("afterPlay"), json!({})).card(LEAK).memory.get("mode"), Some(&json!("afterPlay")));
            }

            #[test]
            fn refuses_a_play_without_a_mode() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [LEAK, FILLER] }, "p2": { "hand": [FILLER] } }));
                s.expect_refused(|s| s.play(LEAK, json!({})));
            }

            #[test]
            fn end_of_turn_locks_one_random_zone_of_the_opponent_s_at_each_end_of_your_turn() {
                crate::register_all();
                let mut s = leak(Some("endOfTurn"), json!({}));
                assert_eq!(locked_of(&s, P2).len(), 0);
                s.end_turn(); // p1's end of turn: one Lock.
                assert_eq!(locked_of(&s, P2).len(), 1);
                assert_eq!(locked_of(&s, P1).len(), 0);
                s.end_turn(); // p2's end of turn: none.
                assert_eq!(locked_of(&s, P2).len(), 1);
                s.end_turn(); // p1's again.
                assert_eq!(locked_of(&s, P2).len(), 2);
                assert!(
                    locked_events(s.events())
                        .iter()
                        .all(|event| matches!(event, GameEvent::Locked { player, .. } if *player == P2))
                );
            }

            #[test]
            fn r60_the_zone_is_a_uniform_pick_among_the_ten_occupied_zones_included_a_lock_evicts_nothing() {
                crate::register_all();
                let mut seen: IndexSet<String> = IndexSet::new();
                for i in 0..16 {
                    let mut s = scenario(json!({
                        "seed": format!("leak-{i}"),
                        "p1": { "hand": [LEAK, FILLER], "library": [FILLER, FILLER], "mana": 4 },
                        "p2": { "hand": [FILLER], "field": [MENACE, MENACE, MENACE, MENACE, MENACE], "backrow": [MANA_WELL] },
                    }));
                    s.play(LEAK, json!({ "modes": ["endOfTurn"] }));
                    s.end_turn();
                    let lock = locked_of(&s, P2).first().cloned().expect("no lock");
                    seen.insert(format!("{}{}", lock["row"].as_str().unwrap_or_default(), lock["lane"]));
                    assert_eq!(s.pile(P2, "graveyard").len(), 0);
                    assert!(
                        (1..=5).all(|lane| s.unit(P2, lane).map(|unit| unit.def_id).as_deref() == Some(MENACE))
                    );
                }
                assert!(seen.len() > 3);
                assert!(seen.iter().any(|key| key.starts_with("units")));
            }

            #[test]
            fn never_a_zone_that_is_locked_already_and_r129_with_all_ten_locked_it_draws_nothing() {
                crate::register_all();
                let mut s = leak(Some("endOfTurn"), json!({}));
                {
                    let locks = &mut s.state_mut().players.p2.locks;
                    locks.units = locks.units.iter().map(|_| true).collect();
                    locks.backrow = locks.backrow.iter().enumerate().map(|(i, _)| i != 2).collect();
                }
                s.end_turn();
                assert_eq!(locked_of(&s, P2).len(), 10);
                let last = locked_events(s.events()).last().map(|event| js(*event)).unwrap_or(Value::Null);
                assert!(matches_object(&last, &json!({ "player": "p2", "row": "backrow", "lane": 3 })));

                let mut full = leak(Some("endOfTurn"), json!({}));
                {
                    let all = &mut full.state_mut().players.p2.locks;
                    all.units = all.units.iter().map(|_| true).collect();
                    all.backrow = all.backrow.iter().map(|_| true).collect();
                }
                let cursor = full.state().rng_cursor;
                let from = full.events().len();
                full.end_turn();
                assert!(locked_events(&full.events()[from..]).is_empty());
                // p2's turn draws nothing random here either, so the cursor is untouched by the Leak.
                assert_eq!(full.state().rng_cursor, cursor);
            }

            #[test]
            fn the_end_of_turn_mode_does_nothing_after_the_opponent_plays_a_unit() {
                crate::register_all();
                let mut s = leak(Some("endOfTurn"), json!({ "hand": [MENACE, FILLER] }));
                s.end_turn();
                let before = locked_of(&s, P2).len();
                s.play(MENACE, json!({}));
                assert_eq!(locked_of(&s, P2).len(), before);
            }

            #[test]
            fn after_play_the_opponent_s_unit_has_its_zone_locked_once_it_resolves_and_stays_in_it() {
                crate::register_all();
                let mut s = leak(Some("afterPlay"), json!({ "hand": [MENACE, FILLER] }));
                s.end_turn();
                s.play(MENACE, json!({ "zone": 3 }));
                assert_eq!(locked_of(&s, P2), vec![lane("units", 3)]);
                assert_eq!(s.unit(P2, 3).map(|unit| unit.def_id).as_deref(), Some(MENACE));
                let resolved = s
                    .events()
                    .iter()
                    .position(|event| matches!(event, GameEvent::CardResolved { def_id, .. } if def_id == MENACE))
                    .map(|at| at as i64)
                    .unwrap_or(-1);
                let locked = s
                    .events()
                    .iter()
                    .position(|event| matches!(event, GameEvent::Locked { .. }))
                    .map(|at| at as i64)
                    .unwrap_or(-1);
                assert!(locked > resolved);
                // No end-of-turn Lock in this mode.
                s.end_turn();
                assert_eq!(locked_of(&s, P2).len(), 1);
            }

            #[test]
            fn after_play_a_field_spell_s_backrow_zone_is_locked() {
                crate::register_all();
                let mut s = leak(Some("afterPlay"), json!({ "hand": [MANA_WELL, FILLER] }));
                s.end_turn();
                s.play(MANA_WELL, json!({ "zone": 2 }));
                assert_eq!(locked_of(&s, P2), vec![lane("backrow", 2)]);
                assert_eq!(s.backrow(P2, 2).map(|card| card.def_id).as_deref(), Some(MANA_WELL));
            }

            #[test]
            fn after_play_a_spell_a_trap_or_a_field_trap_locks_nothing() {
                crate::register_all();
                let mut s = leak(Some("afterPlay"), json!({ "hand": [LUNAR_ECLIPSE, SHEEPISH, BREAD, FILLER] }));
                s.end_turn();
                s.play(LUNAR_ECLIPSE, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
                s.play(SHEEPISH, json!({}));
                s.play(BREAD, json!({}));
                assert_eq!(locked_of(&s, P2).len(), 0);
            }

            #[test]
            fn after_play_your_own_plays_lock_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [LEAK, MENACE, FILLER], "library": [FILLER], "mana": 7 },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(LEAK, json!({ "modes": ["afterPlay"] }));
                s.play(MENACE, json!({}));
                assert_eq!(locked_of(&s, P1).len(), 0);
                assert_eq!(locked_of(&s, P2).len(), 0);
            }

            #[test]
            fn r70_a_cast_counts_the_opponent_s_wardrum_s_copy_of_their_field_spell_is_locked_too() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": LEAK }, FILLER], "library": [FILLER, FILLER], "mana": 4 },
                    "p2": { "hand": [MANA_WELL, FILLER], "field": [{ "def": WARDRUM, "radiant": true }], "library": [FILLER, FILLER] },
                }));
                s.play(LEAK, json!({ "modes": ["afterPlay"] }));
                s.end_turn();
                s.play(MANA_WELL, json!({ "zone": 1 }));
                assert_eq!(locked_of(&s, P2), vec![lane("backrow", 1)]);
                s.end_turn(); // p2's end of turn: Wardrum casts a copy of Mana Well into an open zone.
                let wells: Vec<i64> = (1..=5)
                    .filter(|lane| s.backrow(P2, *lane).map(|card| card.def_id).as_deref() == Some(MANA_WELL))
                    .map(i64::from)
                    .collect();
                assert_eq!(wells.len(), 2);
                let mut lanes: Vec<i64> = locked_of(&s, P2).iter().filter_map(|lock| lock["lane"].as_i64()).collect();
                lanes.sort();
                assert_eq!(lanes, wells);
            }

            #[test]
            fn r175_r688_a_locked_unit_zone_takes_a_reborn_return_there_a_lock_refuses_plays_not_the_return() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [LEAK, BONE_STORM, BONE_STORM, FILLER], "library": [FILLER, FILLER], "mana": 4 },
                    "p2": { "hand": [DEFENDER, FILLER], "library": [FILLER, FILLER] },
                }));
                s.play(LEAK, json!({ "modes": ["afterPlay"] }));
                s.end_turn();
                s.play(DEFENDER, json!({ "zone": 2 }));
                assert_eq!(locked_of(&s, P2), vec![lane("units", 2)]);
                s.end_turn();
                // Two Bone Storms: the first takes the Divine Shield, the second the body.
                let storms: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| card.def_id == BONE_STORM).collect();
                let (first, second) = match (storms.first(), storms.get(1)) {
                    (Some(first), Some(second)) => (first.clone(), second.clone()),
                    _ => panic!("two Bone Storms in hand"),
                };
                s.play(&first, json!({}));
                s.play(&second, json!({}));
                assert_eq!(s.unit(P2, 2).map(|unit| unit.def_id).as_deref(), Some(DEFENDER));
                assert_eq!(locked_of(&s, P2), vec![lane("units", 2)]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn both_at_once_with_no_choice_an_end_of_turn_lock_and_a_lock_after_each_unit_or_field_spell() {
                crate::register_all();
                let mut s = leak(None, json!({ "hand": [MENACE, FILLER] }));
                assert!(s.card(LEAK).memory.get("mode").is_none());
                s.end_turn(); // p1's end of turn: one random Lock.
                let after = locked_of(&s, P2);
                assert_eq!(after.len(), 1);
                let free = (1..=5).find(|at| !after.iter().any(|lock| *lock == lane("units", *at)));
                s.play(MENACE, json!({ "zone": free }));
                let now = locked_of(&s, P2);
                assert_eq!(now.len(), 2);
                assert!(now.contains(&json!({ "row": "units", "lane": free })));
            }
        }
    }
}
