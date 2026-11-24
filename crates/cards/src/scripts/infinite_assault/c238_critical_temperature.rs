//! IA #238 Critical Temperature (SPEC §8.13 row 238), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Start of turn: Deal {damage} damage to each hero, {growth} more each turn this has
//!            been on the field." (2, 1)
//!   Radiant: "Start of turn: Deal {damage} damage to the enemy hero, {growth} more each turn this
//!            has been on the field." (2, 1)
//!
//! The first start deals `damage`; each later start of its controller's turn adds another `growth`
//! per turn seen. The count rides on the card's own memory as `{turn, seen}`, bumped once a turn —
//! an extra firing in the same turn (Meditative #9 Joint Filing) deals the same amount again — and
//! a card that left and came back counts from its first start (R4594013).
//!
//! Engine: `damage` on `startOfTurn`, `remember` (R4594013). Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-238";

/// The turns this has seen, on its own memory: the turn and the count.
const SEEN: &str = "seen";

fn critical(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |ctx| {
            let turn = i64::from(ctx.state.turn);
            // Once a turn: an extra firing in the same turn keeps the count it found.
            let seen = match recalled(&*ctx, SEEN) {
                Some(done) if done.get("turn").and_then(Value::as_i64) == Some(turn) => {
                    done.get("seen").and_then(Value::as_i64).unwrap_or(1).max(1)
                }
                Some(done) => done.get("seen").and_then(Value::as_i64).unwrap_or(0).max(0) + 1,
                None => 1,
            };
            let amount = param(&*ctx, "damage") + param(&*ctx, "growth") * (seen as i32 - 1);
            let mut effects = vec![remember(json_as(json!({
                "key": SEEN,
                "value": { "turn": turn, "seen": seen },
            })))];
            if !radiant {
                effects.push(damage(json_as(json!({
                    "to": { "of": "selfHero" },
                    "amount": amount,
                }))));
            }
            effects.push(damage(json_as(json!({
                "to": { "of": "enemyHero" },
                "amount": amount,
            }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: critical(false), radiant: critical(true) }
}

// IA #238 Critical Temperature — SPEC §8.13 row 238 (R4594013).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FILING: &str = "meditative-009"; // Joint Filing: start/end effects trigger an extra time.
    const JAMMED: &str = "core-036"; // Magic Jammed: destroy the old copy.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn standing(seed: &str, p1_field: Value, p1_hand: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "active": "p1",
            "p1": { "field": p1_field, "hand": p1_hand, "library": crate::filler(6), "health": 30 },
            "p2": { "hand": [STOCKPILE], "library": crate::filler(6), "health": 30 },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r4594013_minus_2_then_3_then_4() {
            let mut s = standing("critical-growth", json!([{ "def": ID }]), json!([STOCKPILE]));
            s.start_turn();
            assert_eq!(s.state().players[P1].hero.health, 30 - 2);
            assert_eq!(s.state().players[P2].hero.health, 30 - 2);
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().players[P1].hero.health, 30 - 2 - 3);
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().players[P1].hero.health, 30 - 2 - 3 - 4);
            assert_eq!(s.state().players[P2].hero.health, 30 - 2 - 3 - 4);
        }

        #[test]
        fn r4594013_joint_filing_does_not_grow_it() {
            let mut s = standing(
                "critical-filing",
                json!([{ "def": ID }, { "def": FILING }]),
                json!([STOCKPILE]),
            );
            s.start_turn();
            // Two firings at the first start's amount, no growth between them.
            assert_eq!(s.state().players[P1].hero.health, 30 - 2 - 2);
            assert_eq!(s.state().players[P2].hero.health, 30 - 2 - 2);
        }

        #[test]
        fn r4594013_a_replayed_copy_counts_from_its_first_start() {
            let mut s = standing(
                "critical-reset",
                json!([{ "def": ID }]),
                json!([{ "def": ID }, JAMMED, STOCKPILE]),
            );
            s.start_turn();
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().players[P1].hero.health, 30 - 2 - 3);
            // The old copy leaves; its replacement counts from its first start. Jammed locks
            // the old zone, so the new copy takes zone 2.
            let old = s.backrow(P1, 1).expect("the old copy").id.clone();
            s.play(JAMMED, json!({ "targets": pick::id(&old) }));
            s.play(ID, json!({ "zone": 2 }));
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().players[P1].hero.health, 30 - 2 - 3 - 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4594013_only_the_enemy_hero_grows() {
            let mut s = standing("critical-radiant", json!([{ "def": ID, "radiant": true }]), json!([STOCKPILE]));
            s.start_turn();
            assert_eq!(s.state().players[P1].hero.health, 30, "its own hero spared");
            assert_eq!(s.state().players[P2].hero.health, 30 - 2);
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().players[P2].hero.health, 30 - 2 - 3);
        }
    }
}
