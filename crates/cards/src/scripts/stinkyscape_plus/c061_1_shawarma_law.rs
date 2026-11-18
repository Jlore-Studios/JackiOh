//! SPEC §8.12 S+ #61.1 Shawarma Law, a card of the Stinkyscape+ set (R1420, R3535000). (2) Field Spell,
//! Token (printed Epic).
//!   Base:    "End of turn: Every Unit eats a Shawarma, with one of these effects. First Shawarma: It gains
//!            +{health} Health. Second Shawarma: It gains Can't attack. Third Shawarma: It gains Can't
//!            Defend and Brittle {brittle2}. A Unit eats at most three Shawarmas."
//!   Radiant: the same, then "Your Units stop at one Shawarma."
//!
//! R3566006: at its controller's End of turn (R62) every non-Dormant Unit of both sides eats a Shawarma.
//! The count is the Unit's own, in its `memory` under `shawarmasEaten` (`remember_onto`), so every Law
//! shares it and R78 clears it when the Unit leaves the field. The count read at the start is the one
//! served: the Units are all read before the first effect. On the Radiant face the controller's Units that
//! have eaten one eat no more; the enemy's go on. The numbers are the declared `health` and `brittle2`
//! (R386); the rung each count serves is below. R4154302: a Unit that has eaten three eats no more, its
//! count staying at three; through v0.6.6h (`script_v0_6_6h`, R767000) every Shawarma after the third
//! froze it.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-061-1";

/// The memory key a Unit counts the Shawarmas it has eaten under.
const EATEN: &str = "shawarmasEaten";

/// The rungs of the ladder: the Shawarma a Unit eats as its first, second and third. R4154302: the third is
/// the last a Unit eats.
const FIRST: i64 = 1;
const SECOND: i64 = 2;
const THIRD: i64 = 3;

/// A Unit that will eat: its id, its controller and the Shawarmas it has eaten so far.
struct Eater {
    id: String,
    controller: PlayerId,
    eaten: i64,
}

fn eaters(ctx: &EffectContext<'_>) -> Vec<Eater> {
    [ctx.controller, opponent_of(ctx.controller)]
        .into_iter()
        .flat_map(|player| active_units_of(ctx.state, player))
        .map(|unit| Eater {
            id: unit.id.clone(),
            controller: unit.controller,
            eaten: unit.memory.get(EATEN).and_then(Value::as_i64).unwrap_or(0),
        })
        .collect()
}

/// What the Shawarma a Unit eats as its `n`th does: past the third, which only the v0.6.6h face serves, Freeze.
fn serving(ctx: &EffectContext<'_>, id: &str, n: i64) -> Vec<Effect> {
    let at = json!({ "of": "instance", "instanceId": id });
    match n {
        FIRST => vec![buff(json_as(
            json!({ "target": at, "health": param(ctx, "health") }),
        ))],
        SECOND => vec![grant_keyword(json_as(
            json!({ "target": at, "keyword": { "kind": "Can't attack" } }),
        ))],
        THIRD => vec![
            grant_keyword(json_as(
                json!({ "target": at, "keyword": { "kind": "Can't Defend" } }),
            )),
            give_brittle(json_as(json!({ "instanceId": id, "n": param(ctx, "brittle2") }))),
        ],
        _ => vec![freeze(json_as(json!({ "target": at })))],
    }
}

/// The face: today's when `through_v0_6_6h` is false, else the one v0.6.6h and before ran, whose Units ate
/// on past the third and froze.
fn law(radiant: bool, through_v0_6_6h: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            let mut effects = Vec::new();
            for unit in eaters(ctx) {
                if radiant && unit.controller == ctx.controller && unit.eaten >= FIRST {
                    continue;
                }
                if !through_v0_6_6h && unit.eaten >= THIRD {
                    continue;
                }
                let n = unit.eaten + 1;
                effects.push(remember_onto(json_as(json!({
                    "target": { "of": "instance", "instanceId": unit.id },
                    "key": EATEN,
                    "value": n,
                }))));
                effects.extend(serving(ctx, &unit.id, n));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: law(false, false),
        radiant: law(true, false),
    }
}

/// R767000: what it ran through v0.6.6h, before issue #4154: every Shawarma after the third froze the Unit.
pub fn script_v0_6_6h() -> CardScripts {
    CardScripts {
        base: law(false, true),
        radiant: law(true, true),
    }
}

// S+ #61.1 Shawarma Law — SPEC §8.12 row 61.1 (R3566006).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::{EATEN, ID, script_v0_6_6h};
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const UNIT: &str = "stand-in (1) 4/4";
    const FILLER: &str = "core-005";

    fn board(radiant: bool, laws: usize) -> Scenario {
        let backrow: Vec<Value> = (0..laws)
            .map(|_| json!({ "def": ID, "radiant": radiant }))
            .collect();
        scenario(json!({
            "p1": { "hand": [FILLER], "field": [UNIT], "backrow": backrow, "library": filler(12) },
            "p2": { "hand": [FILLER], "field": [UNIT], "library": filler(12) },
        }))
    }

    /// p1's turn ends (the Law serves), then p2's.
    fn round(s: &mut Scenario) {
        s.end_turn().end_turn();
    }

    fn eaten(s: &Scenario, unit: &CardInstance) -> i64 {
        s.card(unit)
            .memory
            .get(EATEN)
            .and_then(Value::as_i64)
            .unwrap_or(0)
    }

    fn has(s: &Scenario, unit: &CardInstance, kind: KeywordKind) -> bool {
        has_keyword(&s.stats(unit).keywords, kind)
    }

    #[test]
    fn r3566006_the_first_shawarma_gives_3_health_on_both_sides() {
        let mut s = board(false, 1);
        let (mine, theirs) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
        round(&mut s);
        for unit in [&mine, &theirs] {
            s.expect_stats(unit, json!({ "attack": 4, "health": 7 }));
            assert_eq!(eaten(&s, unit), 1);
        }
    }

    #[test]
    fn r3566006_r4154302_the_ladder_runs_cant_attack_then_cant_defend_and_brittle_2_then_stops() {
        let mut s = board(false, 1);
        let unit = s.unit_at(P2, 1);
        round(&mut s);
        assert!(!has(&s, &unit, KeywordKind::CantAttack));
        round(&mut s);
        assert!(
            has(&s, &unit, KeywordKind::CantAttack),
            "the second: Can't attack"
        );
        assert!(!has(&s, &unit, KeywordKind::CantDefend));
        round(&mut s);
        assert!(has(&s, &unit, KeywordKind::CantDefend), "the third: Can't defend");
        assert_eq!(active_brittle_count(s.card(&unit)), Some(2), "and Brittle 2");
        assert_eq!(eaten(&s, &unit), 3);
        // R4154302: no fourth. Look before p2's turn, when a Freeze would still hold.
        s.end_turn();
        assert!(s.card(&unit).frozen.is_none(), "no Freeze");
        assert_eq!(eaten(&s, &unit), 3, "it eats at most three");
    }

    #[test]
    fn r4154302_radiant_enemies_stop_at_three_too() {
        let mut s = board(true, 1);
        let (mine, theirs) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
        for _ in 0..3 {
            round(&mut s);
        }
        // The fourth end of turn serves the enemy nothing either, and the ally stopped at one.
        s.end_turn();
        assert_eq!(eaten(&s, &theirs), 3);
        assert!(s.card(&theirs).frozen.is_none());
        assert_eq!(eaten(&s, &mine), 1);
    }

    /// R767000: through v0.6.6h every Shawarma after the third froze the Unit (issue #4154).
    #[test]
    fn r767000_the_script_kept_for_v0_6_6h_freezes_from_the_fourth() {
        crate::under_kept(ID, script_v0_6_6h(), || {
            let mut s = board(false, 1);
            let unit = s.unit_at(P2, 1);
            for _ in 0..3 {
                round(&mut s);
            }
            assert!(s.card(&unit).frozen.is_none());
            // The fourth: Freeze, which thaws at the end of the Unit's own turn, so look before p2's.
            s.end_turn();
            assert!(s.card(&unit).frozen.is_some(), "the fourth: Freeze");
            assert_eq!(eaten(&s, &unit), 4);
        });
    }

    #[test]
    fn r3566006_the_opponents_end_of_turn_serves_nothing() {
        let mut s = board(false, 1);
        let unit = s.unit_at(P1, 1);
        s.end_turn();
        assert_eq!(eaten(&s, &unit), 1, "p1's own end of turn served");
        s.end_turn();
        assert_eq!(eaten(&s, &unit), 1, "p2's end of turn served nothing");
    }

    #[test]
    fn r3566006_radiant_allies_stop_at_one_enemies_go_on() {
        let mut s = board(true, 1);
        let (mine, theirs) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
        round(&mut s);
        round(&mut s);
        assert_eq!(eaten(&s, &mine), 1, "the ally ate one and stopped");
        assert_eq!(eaten(&s, &theirs), 2, "the enemy goes on");
        s.expect_stats(&mine, json!({ "health": 7 }));
        assert!(!has(&s, &mine, KeywordKind::CantAttack));
        assert!(has(&s, &theirs, KeywordKind::CantAttack));
    }

    #[test]
    fn r3566006_two_laws_share_the_count() {
        let mut s = board(false, 2);
        let unit = s.unit_at(P2, 1);
        round(&mut s);
        // Both Laws served at once: the count read first is 0 for both, so each serves the first.
        assert!(eaten(&s, &unit) >= 1);
        round(&mut s);
        assert!(eaten(&s, &unit) >= 2);
        assert_eq!(
            eaten(&s, &unit) as usize,
            2 * 2 - (4 - eaten(&s, &unit) as usize),
            "the count never resets between Laws"
        );
    }
}
