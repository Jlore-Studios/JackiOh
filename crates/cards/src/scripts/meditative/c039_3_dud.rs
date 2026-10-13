//! Meditative #39.3 Dud (SPEC §8.8 row 39.3). (0) Spell, CN, Token (printed Common).
//!
//!   Base:    "Deal {damage} damage." — damage 2
//!   Radiant: "Deal {damage} damage." — damage 5
//!
//! Engine: the target is declared with the play (any unit or hero, B1's convention for "Deal N
//! damage"), and `damage` is one instance through §4.4.
//! Tunes: damage 2 ↑.
//!
//! The shape is Classic #16 Book of Flame's, read at this card's numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-039-3";

/// §8 Conventions: any unit or hero, either side.
fn targets() -> Vec<TargetDecl> {
    vec![json_as(json!({ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "hero"] } }))]
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": param(&*ctx, "damage") })))]
        })),
        ..Script::default()
    };

    // The same script: the Radiant face's 5 is its declared `damage`, which `param` reads off the running face.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Meditative #39.3 Dud — SPEC §8.8 row 39.3, BUILD M10 row M 39.3: "One hit of 2 on a declared Unit
// or hero on either side; damage reads through `param()`; radiant 5".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P2: PlayerId = PlayerId::P2;

    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const FILLER: &str = "core-008";

    fn at(s: &Scenario, player: PlayerId, lane: i32) -> Value {
        let Some(unit) = s.unit(player, lane) else {
            panic!("no unit in {player:?} lane {lane}");
        };
        json!([{ "pick": "instance", "instanceId": unit.id }])
    }

    #[test]
    fn declares_one_target_and_runs_one_script_on_both_faces() {
        // One target is declared: played with none, the play is refused.
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [ID, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        s.expect_refused(|s| s.play(ID, json!({})));
        // Both faces run the one script: the Radiant face's 5 is its declared `damage`.
        let CardScripts { base, radiant } = script();
        assert!(Arc::ptr_eq(base.cry.as_ref().unwrap(), radiant.cry.as_ref().unwrap()));
        assert_eq!(radiant.targets, base.targets);
        assert_eq!(base.targets.len(), 1);
    }

    #[test]
    fn deals_2_to_a_declared_unit_or_hero() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [ID, ID, FILLER], "field": [{ "def": MENACE, "lane": 1 }] },
            "p2": { "hand": [FILLER], "field": [{ "def": MENACE, "lane": 2 }] },
        }));
        let prey_id = s.unit(P2, 2).expect("p2's menace").id.clone();

        // A declared enemy unit takes one hit of 2.
        s.play(ID, json!({ "targets": at(&s, P2, 2) }));
        s.expect_stats(&prey_id, json!({ "attack": 9, "health": 7 }));

        // …and so does the enemy hero.
        s.play(ID, json!({ "targets": json!([{ "pick": "hero", "player": "p2" }]) }));
        s.expect_health(P2, 28);
    }

    #[test]
    fn radiant_deals_5() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": true }, FILLER], "field": [{ "def": MENACE, "lane": 1 }] },
            "p2": { "hand": [FILLER], "field": [{ "def": MENACE, "lane": 2 }] },
        }));
        let prey_id = s.unit(P2, 2).expect("p2's menace").id.clone();

        s.play(ID, json!({ "targets": at(&s, P2, 2) }));

        s.expect_stats(&prey_id, json!({ "attack": 9, "health": 4 }));
    }
}
