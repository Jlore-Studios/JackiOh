//! FvH #67 Big Game Hunter (SPEC §8.14 row 67): (1) Unit, Human, Epic, 4/2 → 8/4. Built by S08 (#5258).
//!   Base:    "Cry: Destroy an enemy Unit with {attack} or more Attack." — attack 7
//!   Radiant: "Cry: Destroy every enemy Unit with {attack} or more Attack." — attack 7
//!
//! Base: one declared enemy Unit whose Attack through the layers (§10.4) is the declared `attack` or
//! more as it is chosen (R81), S #12.1 Shadow Word Death's `attackAtLeast` check, then `destroy`ed; it
//! is not needed, so with none to choose the Unit still enters and the Cry does nothing (R90). Radiant:
//! no target; every enemy Unit at `attack` or more as the Cry runs (a Dormant one is not reached,
//! R1839000) is marked and all are collected at one state check (R59). Indestructible keeps its Unit
//! (R46).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-067";

/// The target check: an enemy Unit with the declared Attack or more.
const THRESHOLD: &str = "attackAtLeast";

/// The declared Attack or more, through the layers.
fn meets_threshold(args: TargetCheckArgs<'_>) -> bool {
    let Some(candidate) = args.candidate else {
        return false;
    };
    unit_view(args.state, candidate).attack >= param(&args, "attack")
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "enemy", "of": ["unit"], "check": THRESHOLD }),
        )],
        target_checks: IndexMap::from([(THRESHOLD, target_check(meets_threshold))]),
        cry: Some(hook(|_ctx| vec![destroy(json_as(json!({ "target": { "of": "chosen" } })))])),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(|ctx| {
            let at_least = param(ctx, "attack");
            let state: &GameState = &*ctx.state;
            active_units_of(state, opponent_of(ctx.controller))
                .into_iter()
                .filter(|unit| unit_view(state, unit).attack >= at_least)
                .map(|unit| {
                    destroy(json_as(json!({ "target": { "of": "instance", "instanceId": unit.id } })))
                })
                .collect()
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// FvH #67 Big Game Hunter — SPEC §8.14 row 67.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BIG: &str = "stand-in (1) 7/7";
    const SMALL: &str = "stand-in (1) 6/6";
    const HUGE: &str = "stand-in (1) 9/1";
    const FILLER: &str = "core-005"; // Stockpile: a card in hand

    fn hunt(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": mine },
            "p2": { "hand": [FILLER], "field": theirs },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn destroys_the_chosen_enemy_unit_with_7_or_more_attack() {
            let mut s = hunt(false, json!([]), json!([BIG, SMALL]));
            let big = s.unit_at(P2, 1).id;
            let small = s.unit_at(P2, 2).id;
            s.play(ID, pick::targets(pick::id(&big)));
            s.expect_in_zone(big.as_str(), "graveyard");
            s.expect_in_zone(small.as_str(), "field");
            s.expect_in_zone(ID, "field");
        }

        #[test]
        fn a_unit_under_7_attack_cannot_be_chosen() {
            let mut s = hunt(false, json!([]), json!([SMALL]));
            let small = s.unit_at(P2, 1).id;
            s.expect_refused(|s| s.play(ID, pick::targets(pick::id(&small))));
            s.expect_in_zone(small.as_str(), "field");
            s.expect_in_zone(ID, "hand");
        }

        #[test]
        fn with_no_unit_to_choose_it_still_enters() {
            let mut s = hunt(false, json!([]), json!([SMALL]));
            let small = s.unit_at(P2, 1).id;
            s.play(ID, json!({}));
            s.expect_in_zone(ID, "field");
            s.expect_in_zone(small.as_str(), "field");
        }

        #[test]
        fn the_threshold_tunes() {
            let mut s = hunt(false, json!([]), json!([SMALL]));
            let small = s.unit_at(P2, 1).id;
            set_param(s.card_mut(ID), "attack", 6);
            s.play(ID, pick::targets(pick::id(&small)));
            s.expect_in_zone(small.as_str(), "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn destroys_every_enemy_unit_with_7_or_more_attack() {
            let mut s = hunt(true, json!([BIG]), json!([BIG, SMALL, HUGE]));
            let mine = s.unit_at(P1, 1).id;
            let (big, small, huge) = (s.unit_at(P2, 1).id, s.unit_at(P2, 2).id, s.unit_at(P2, 3).id);
            s.play(ID, json!({}));
            s.expect_in_zone(big.as_str(), "graveyard");
            s.expect_in_zone(huge.as_str(), "graveyard");
            s.expect_in_zone(small.as_str(), "field");
            s.expect_in_zone(mine.as_str(), "field");
        }

        #[test]
        fn the_threshold_tunes() {
            let mut s = hunt(true, json!([]), json!([SMALL]));
            let small = s.unit_at(P2, 1).id;
            set_param(s.card_mut(ID), "attack", 6);
            s.play(ID, json!({}));
            s.expect_in_zone(small.as_str(), "graveyard");
        }
    }
}
