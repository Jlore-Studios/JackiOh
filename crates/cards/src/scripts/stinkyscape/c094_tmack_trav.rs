//! S #94 Tmack Trav (SPEC §8.10 row 94): (3) Unit, Human, Epic, 9/9 → 18/18, of the held Stinkyscape
//! set (R1420). Not Tnack Trav, S #20 (ST9).
//!   Base:    "Rush / After this destroys a Unit, it may attack again."
//!   Radiant: "Rush, Pierce / After this destroys a Unit, it may attack again."
//! Engine: Rush and Pierce (its hits ignore Armor, R346) are catalog keywords. `after_attack` (§4.2, R1869002): when it survived
//! the combat and the Unit it attacked is in that combat's destroyed ids, `may_attack_again` (B5 E35, as
//! C+ #73.1 Classic Golem's): a fresh exertion this turn, the rest of §4.2's restrictions still applying.
//! Each kill grants one more attack, with no cap; a forced attack counts as a declared one does
//! (R1849001's `afterAttack` fires for both). A hit on the hero, a target that lives, and a kill while
//! defending (no `afterAttack` of its own) give nothing.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-094";

/// A hero target is `hero-<player>` in the combat's facts.
const HERO_TARGET: &str = "hero-";

pub fn script() -> CardScripts {
    let base = Script {
        after_attack: Some(hook(|ctx| {
            let Some(combat) = after_attack_of(ctx) else {
                return Vec::new();
            };
            let killed = combat.survived
                && !combat.target_id.starts_with(HERO_TARGET)
                && combat.destroyed_ids.contains(&combat.target_id);
            if !killed {
                return Vec::new();
            }
            vec![may_attack_again(json_as(json!({ "target": { "of": "self" } })))]
        })),
        ..Script::default()
    };
    // The Radiant face adds Pierce, catalog data.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// S #94 Tmack Trav — SPEC §8.10 row 94, BUILD M12 row S 94.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    const FILLER: &str = "core-005"; // (1) Spell: a card in hand, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const MOTHS: &str = "core-009"; // (2) 1/14: at its controller's start of turn every enemy Unit attacks it.

    /// 2/12: lives through a 9-attack hit.
    fn fauci() -> Value {
        json!({ "def": "core-091", "radiant": true })
    }

    /// p1 has Tmack Trav (`radiant` its face) in unit zone 1; p2 has `p2_field`.
    fn board(radiant: bool, p2_field: Value) -> Scenario {
        crate::scenario(json!({
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "radiant": radiant }], "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "field": p2_field, "library": crate::filler(4) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r1869002_a_kill_frees_it_to_attack_again() {
            let mut s = board(false, json!([VANILLA]));
            let tmack = s.unit_at(P1, 1);
            s.attack(&tmack, s.unit_at(P2, 1));
            assert!(s.unit(P2, 1).is_none(), "the Vanilla died");
            s.attack(&tmack, "hero");
            s.expect_health(P2, HERO_HEALTH - 9);
            // A hit on the hero frees nothing.
            s.expect_refused(|s| s.attack(&tmack, "hero"));
        }

        #[test]
        fn has_rush() {
            let mut s = crate::scenario(json!({
                "p1": { "hand": [ID, FILLER], "library": crate::filler(4) },
                "p2": { "hand": [FILLER], "field": [VANILLA], "library": crate::filler(4) },
            }));
            s.play(ID, json!({ "zone": 1 }));
            let tmack = s.unit_at(P1, 1);
            assert!(has_keyword(&s.stats(&tmack).keywords, KeywordKind::Rush));
            // Rush: a Unit, not the hero, the turn it arrives (§6.1); the kill lifts its sickness too (B5 E35).
            s.expect_refused(|s| s.attack(&tmack, "hero"));
            s.attack(&tmack, s.unit_at(P2, 1));
            s.attack(&tmack, "hero");
            s.expect_health(P2, HERO_HEALTH - 9);
        }

        #[test]
        fn r1869002_a_survivor_ends_its_attacks() {
            let mut s = board(false, json!([fauci()]));
            let tmack = s.unit_at(P1, 1);
            let fauci = s.unit_at(P2, 1);
            s.attack(&tmack, &fauci);
            s.expect_stats(&fauci, json!({ "health": 3 }));
            s.expect_refused(|s| s.attack(&tmack, "hero"));
        }

        #[test]
        fn r1869002_a_kill_while_defending_banks_nothing() {
            let mut s = crate::scenario(json!({
                "active": "p2",
                "p1": { "hand": [FILLER], "field": [ID], "library": crate::filler(4) },
                "p2": { "hand": [FILLER], "field": [VANILLA, fauci()], "library": crate::filler(4) },
            }));
            let tmack = s.unit_at(P1, 1);
            s.attack(s.unit_at(P2, 1), &tmack);
            assert!(s.unit(P2, 1).is_none(), "its strike back killed the Vanilla");
            s.end_turn();
            s.attack(&tmack, s.unit_at(P2, 2));
            s.expect_refused(|s| s.attack(&tmack, "hero"));
        }

        #[test]
        fn r1869002_a_forced_kill_frees_it_too() {
            // Moths at 4 health: the attack its controller's start of turn forces kills it.
            let mut s = board(false, json!([{ "def": MOTHS, "damage": 10 }]));
            let tmack = s.unit_at(P1, 1);
            s.attack(&tmack, "hero");
            assert!(s.card(&tmack.id).exertion.attacked);
            s.end_turn();
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::AttackDeclared { attacker_id, forced: true, .. } if *attacker_id == tmack.id
            )));
            assert!(s.unit(P2, 1).is_none(), "the Moths died");
            assert!(!s.card(&tmack.id).exertion.attacked, "a fresh exertion");
        }

        #[test]
        fn two_kills_three_attacks() {
            let mut s = board(false, json!([VANILLA, VANILLA]));
            let tmack = s.unit_at(P1, 1);
            s.attack(&tmack, s.unit_at(P2, 1));
            s.attack(&tmack, s.unit_at(P2, 2));
            s.attack(&tmack, "hero");
            s.expect_health(P2, HERO_HEALTH - 9);
            s.expect_stats(&tmack, json!({ "health": 1 }));
            s.expect_refused(|s| s.attack(&tmack, "hero"));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_18_18_with_pierce() {
            let mut s = board(true, json!([VANILLA]));
            let tmack = s.unit_at(P1, 1);
            s.expect_stats(&tmack, json!({ "attack": 18, "health": 18 }));
            let keywords = s.stats(&tmack).keywords;
            assert!(has_keyword(&keywords, KeywordKind::Rush) && has_keyword(&keywords, KeywordKind::Pierce));
            s.attack(&tmack, s.unit_at(P2, 1));
            s.attack(&tmack, "hero");
            s.expect_health(P2, HERO_HEALTH - 18);
        }
    }
}
