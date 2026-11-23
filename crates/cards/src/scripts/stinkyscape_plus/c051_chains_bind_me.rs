//! SPEC §8.12 S+ #51 Chains Bind Me, a card of the Stinkyscape+ set (R1420, R3535000). (2) Spell,
//! Common.
//!   Base:    "Dormant a permanent for {count} turns. When it awakens, Buff it {times} times if it's yours
//!            or Nerf it {times2} times if it's an enemy's." (2; 2, 2)
//!   Radiant: the same (2; 5, 5)
//!
//! A declared target: one Unit of either side (R81, R3557002; Dormant's rules reach no other card,
//! R1839000). `dormant_target` puts it to sleep for `count` of its controller's turns (R1839001), and a
//! delayed step is armed for the start of that controller's `count`th turn from now (R1181000), which
//! comes after the awakening of that turn's start (`turn.rs`: Brittle, awaken, then the delayed
//! effects). The step reads who controls the Unit then: the caster's, it is Buffed `times` times, any
//! other's Nerfed `times2` times. A Unit that left the field in between is forgotten (R174).

use jackioh_engine::effects::{degrade, dormant_target, upgrade};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-051";

/// The step the awakening comes back to (§10.6).
const AWAKENED: &str = "awakened";

fn chains() -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
        cry: Some(hook(|ctx| {
            let Some(Selection::Instance { instance_id }) = ctx.targets.first() else {
                return vec![];
            };
            let Some(unit) = find_instance(&*ctx.state, instance_id) else {
                return vec![];
            };
            let side = if unit.controller == ctx.controller { "self" } else { "enemy" };
            let turns = param(ctx, "count");
            vec![
                dormant_target(json_as(json!({ "target": { "of": "chosen" }, "turns": turns }))),
                delay(json_as(json!({
                    "at": { "phase": "start", "player": side },
                    "step": AWAKENED,
                    "hook": RESUME_HOOK,
                    "data": { "instanceId": instance_id },
                    "turns": turns,
                    "watch": instance_id,
                }))),
            ]
        })),
        resume: IndexMap::from([(
            AWAKENED,
            hook(|ctx| {
                let Some(id) = ctx.data.get("instanceId").and_then(Value::as_str) else {
                    return vec![];
                };
                let Some(unit) = find_instance(&*ctx.state, id) else {
                    return vec![];
                };
                if unit.controller == ctx.controller {
                    vec![upgrade(json_as(json!({ "instanceId": id, "times": param(ctx, "times") })))]
                } else {
                    vec![degrade(json_as(json!({ "instanceId": id, "times": param(ctx, "times2") })))]
                }
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The Radiant face's 5 and 5 are its declared `times` and `times2`.
    let base = chains();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #51 Chains Bind Me — SPEC §8.12 row 51 (R3557002).
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
        scenario(json!({
            "seed": "chains-bind-me",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": [BODY], "library": filler(6), "mana": 10 },
            "p2": { "hand": [FILLER], "field": [BODY], "library": filler(6) },
        }))
    }

    fn changes(events: &[GameEvent], id: &str, kind: &str) -> usize {
        events
            .iter()
            .filter(|event| event.event_type().as_str() == kind)
            .map(crate::js)
            .filter(|event| event["instanceId"] == id)
            .count()
    }

    fn slumber(s: &Scenario, id: &str) -> Option<i32> {
        jackioh_engine::slumber::slumber_turns_of(s.card(id))
    }

    #[test]
    fn r3557002_it_sleeps_two_turns_then_awakens_and_a_friendly_unit_is_buffed_twice() {
        crate::register_all();
        let mut s = board(false);
        let own = s.unit_at(P1, 1).id;
        s.play(ID, pick::targets(pick::id(&own)));
        assert_eq!(slumber(&s, &own), Some(2));
        s.end_turn();
        assert_eq!(slumber(&s, &own), Some(2), "the opponent's turn does not count");
        s.end_turn();
        assert_eq!(slumber(&s, &own), Some(1), "its controller's first turn");
        assert_eq!(changes(s.events(), &own, "upgraded"), 0);
        s.end_turn();
        s.end_turn();
        assert_eq!(slumber(&s, &own), None);
        assert_eq!(changes(s.last_events(), &own, "upgraded"), 2);
        let kinds: Vec<&str> = s.last_events().iter().map(|event| event.event_type().as_str()).collect();
        let awakened = kinds.iter().position(|kind| *kind == "awakened").unwrap_or(usize::MAX);
        let buffed = kinds.iter().position(|kind| *kind == "upgraded").unwrap_or(0);
        assert!(awakened < buffed, "the awakening comes before the Buffs");
    }

    #[test]
    fn r3557002_an_enemy_unit_is_nerfed_twice_when_it_awakens() {
        crate::register_all();
        let mut s = board(false);
        let enemy = s.unit_at(P2, 1).id;
        s.play(ID, pick::targets(pick::id(&enemy)));
        assert_eq!(slumber(&s, &enemy), Some(2));
        s.end_turn();
        assert_eq!(slumber(&s, &enemy), Some(1), "p2's first turn started");
        s.end_turn();
        s.end_turn();
        assert_eq!(slumber(&s, &enemy), None);
        assert_eq!(changes(s.last_events(), &enemy, "degraded"), 2);
        assert_eq!(changes(s.last_events(), &enemy, "upgraded"), 0);
    }

    #[test]
    fn the_radiant_face_changes_five_times() {
        crate::register_all();
        let mut s = board(true);
        let enemy = s.unit_at(P2, 1).id;
        s.play(ID, pick::targets(pick::id(&enemy)));
        s.end_turn().end_turn().end_turn();
        assert_eq!(changes(s.last_events(), &enemy, "degraded"), 5);
    }

    #[test]
    fn r174_a_unit_that_left_the_field_is_forgotten() {
        crate::register_all();
        let mut s = board(false);
        let enemy = s.unit_at(P2, 1).id;
        s.play(ID, pick::targets(pick::id(&enemy)));
        s.state_mut().players[P2].units[0] = None;
        s.end_turn().end_turn().end_turn();
        let all: usize = changes(s.events(), &enemy, "degraded") + changes(s.last_events(), &enemy, "degraded");
        assert_eq!(all, 0);
    }
}
