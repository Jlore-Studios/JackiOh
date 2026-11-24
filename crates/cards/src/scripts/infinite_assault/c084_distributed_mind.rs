//! SPEC §8.13 IA #84 Distributed Mind ((3) Unit, Rare, 3/5 → 6/10), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Once per turn, when another friendly Unit dies, draw a card."
//!   Radiant: "Twice per turn, when another friendly Unit dies, draw a card."
//!
//! TG #35 Corpse Recycler's trigger on `destroyed`, narrowed to another friendly Unit's death
//! (`is_unit_death`, R4423000): the draw, counted in the card's own memory against this turn alone
//! (R4588002), so deaths at the same moment are counted one by one (R2271000) and leaving the field
//! loses the count (R78).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-084";

/// The draws this card has taken, under this turn's number.
const DRAWN_KEY: &str = "drawnThisTurn";

/// "draw a card".
const DRAWS: i32 = 1;

/// "Once per turn", Radiant "Twice per turn".
const ONCE: i32 = 1;
const TWICE: i32 = 2;

/// R4588002: the draws this card has taken this turn (`state.turn`), counted in its own memory.
fn draws_this_turn(ctx: &EffectContext<'_>) -> i32 {
    let Some(remembered) = recalled(ctx, DRAWN_KEY) else {
        return 0;
    };
    if remembered.get("turn").and_then(Value::as_i64) != Some(i64::from(ctx.state.turn)) {
        return 0;
    }
    remembered
        .get("draws")
        .and_then(Value::as_i64)
        .and_then(|draws| i32::try_from(draws).ok())
        .unwrap_or(0)
}

/// Another friendly Unit's death with draws left under the cap: its controller's (R172), a Unit's by
/// what the card was as it died (`is_unit_death`), never this card's own.
fn answers(ctx: &EffectContext<'_>, event: &GameEvent, cap: i32) -> bool {
    let GameEvent::Destroyed {
        instance_id,
        controller,
        ..
    } = event
    else {
        return false;
    };
    if *controller != ctx.controller || !is_unit_death(&*ctx.state, event) {
        return false;
    }
    if ctx.self_.as_ref().is_some_and(|me| me.id == *instance_id) {
        return false;
    }
    draws_this_turn(ctx) < cap
}

fn distributed_mind(cap: i32) -> Script {
    let trigger = TriggerDef::new("distributed-mind", &[GameEventType::Destroyed], move |ctx, event| {
        if !answers(ctx, event, cap) {
            return vec![];
        }
        vec![
            remember(json_as(json!({
                "key": DRAWN_KEY,
                "value": { "turn": ctx.state.turn, "draws": draws_this_turn(ctx) + 1 },
            }))),
            draw(json_as(json!({ "count": DRAWS }))),
        ]
    })
    .with_when(move |ctx, event| answers(ctx, event, cap));
    Script {
        triggers: vec![trigger],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: distributed_mind(ONCE),
        radiant: distributed_mind(TWICE),
    }
}

// IA #84 Distributed Mind — SPEC §8.13 row 84 (R4588002).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";
    const HIT_JOB: &str = "core-016"; // Hit Job: the kill

    fn board(p1_field: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [HIT_JOB, HIT_JOB, HIT_JOB],
                "field": p1_field,
                "mana": 10,
                "library": crate::filler(6),
            },
            "p2": { "field": [UNIT], "library": crate::filler(2) },
        }))
    }

    /// Kill one of p1's other Units, and count p1's hand after.
    fn kill(s: &mut Scenario) -> usize {
        let unit = active_units_of(s.state(), P1)
            .into_iter()
            .find(|unit| unit.def_id == UNIT)
            .cloned()
            .expect("another friendly Unit to kill");
        s.play(HIT_JOB, pick::targets(pick::instance(&unit)));
        s.hand(P1).len()
    }

    mod base {
        use super::*;

        #[test]
        fn r4588002_two_deaths_draw_once_then_again_next_turn() {
            let mut s = board(json!([{ "def": ID }, UNIT, UNIT, UNIT]));
            assert_eq!(kill(&mut s), 3, "the first death draws");
            assert_eq!(kill(&mut s), 2, "the second death this turn draws nothing");
            s.end_turn();
            s.end_turn();
            // The turn's start drew first, so the hand holds three: the play spent one and the death
            // drew one back.
            assert_eq!(kill(&mut s), 3, "next turn it draws again");
        }

        #[test]
        fn its_own_death_and_an_enemys_draw_nothing() {
            let mut s = board(json!([{ "def": ID }, UNIT]));
            let mind = s.unit_at(P1, 1);
            s.play(HIT_JOB, pick::targets(pick::instance(&mind)));
            assert_eq!(s.hand(P1).len(), 2, "its own death draws nothing");
            let enemy = s.unit_at(P2, 1);
            s.play(HIT_JOB, pick::targets(pick::instance(&enemy)));
            assert_eq!(s.hand(P1).len(), 1, "an enemy's death draws nothing");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4588002_two_of_three_deaths_draw() {
            let mut s = board(json!([{ "def": ID, "radiant": true }, UNIT, UNIT, UNIT]));
            assert_eq!(kill(&mut s), 3, "the first death draws");
            assert_eq!(kill(&mut s), 3, "the second death draws");
            assert_eq!(kill(&mut s), 2, "the third this turn draws nothing");
        }
    }
}
