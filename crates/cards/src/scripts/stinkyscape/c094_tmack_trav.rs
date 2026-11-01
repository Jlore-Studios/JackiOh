//! S #94 Tmack Trav (SPEC §8.10 row 94): (3) Unit, Human, Epic, 9/9 → 18/18, of the Stinkyscape
//! set. Not Tnack Trav, S #20 (ST9).
//!   Base:    "Rush / After this destroys a Unit, it may attack again."
//!   Radiant: "Rush, Pierce / After this destroys a Unit, it may attack again."
//! Engine: Rush and Pierce (its hits ignore Armor, R346) are catalog keywords. `after_attack` (§4.2, R1869002): when it survived
//! the combat and the Unit it attacked is in that combat's destroyed ids, `may_attack_again` (B5 E35, as
//! C+ #73.1 Classic Golem's): a fresh exertion this turn, the rest of §4.2's restrictions still applying.
//! Each kill grants one more attack, with no cap; a forced attack counts as a declared one does
//! (R1849001's `afterAttack` fires for both). A hit on the hero, a target that lives, and a kill while
//! defending (no `afterAttack` of its own) give nothing. The fresh exertion keeps its summoning sickness
//! (`keepSick`, R2521002), unlike the Golem's: on its summon turn Rush lets it attack again a Unit only,
//! the hero neither offered nor allowed (§6.1); on a later turn the extra attack may hit the hero.
//! Through v0.5.11 the kill lifted its sickness, which `script_v0_5_11` keeps (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-094";

/// A hero target is `hero-<player>` in the combat's facts.
const HERO_TARGET: &str = "hero-";

/// "After this destroys a Unit, it may attack again": `keep_sick` since issue #2521 (R2521002).
fn tmack(keep_sick: bool) -> Script {
    Script {
        after_attack: Some(hook(move |ctx| {
            let Some(combat) = after_attack_of(ctx) else {
                return Vec::new();
            };
            let killed = combat.survived
                && !combat.target_id.starts_with(HERO_TARGET)
                && combat.destroyed_ids.contains(&combat.target_id);
            if !killed {
                return Vec::new();
            }
            // R2521002: it may attack again, still summoning sick if it entered this turn; v0.5.11's lifted
            // the sickness, which a refresh does only when it says so since R2657004 (`liftSick`).
            let mut args = json!({ "target": { "of": "self" } });
            if keep_sick {
                args["keepSick"] = json!(true);
            } else {
                args["liftSick"] = json!(true);
            }
            vec![may_attack_again(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = tmack(true);
    // The Radiant face adds Pierce, catalog data.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

/// R767000: through v0.5.11 its kill lifted its summoning sickness, so its summon turn's extra attack
/// could hit the hero; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11() -> CardScripts {
    let base = tmack(false);
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

    use super::{ID, script_v0_5_11};

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

    /// p1 holds Tmack Trav (`radiant` its face) to play this turn; p2 has two Vanillas, in lanes 1 and 2.
    fn in_hand(radiant: bool) -> Scenario {
        crate::scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "field": [VANILLA, VANILLA], "library": crate::filler(4) },
        }))
    }

    /// Whether `legal_actions` offers `attacker` an attack on `target`.
    fn offers(s: &Scenario, attacker: &CardInstance, target: &str) -> bool {
        legal_actions(s.state(), P1).iter().any(|action| {
            matches!(
                action,
                ActionBody::Attack { attacker_id, target_id, .. }
                    if *attacker_id == attacker.id && target_id == target
            )
        })
    }

    /// R2521002: played this turn, its kill frees it to attack a Unit again, never the hero.
    fn r2521002_summon_turn(radiant: bool) {
        let mut s = in_hand(radiant);
        s.play(ID, json!({ "zone": 1 }));
        let tmack = s.unit_at(P1, 1);
        s.attack(&tmack, s.unit_at(P2, 1));
        assert!(s.unit(P2, 1).is_none(), "the Vanilla died");
        assert!(!s.card(&tmack.id).exertion.attacked, "a fresh exertion");
        assert_eq!(s.card(&tmack.id).summoned_turn, Some(s.state().turn), "still summoning sick");
        // The hero is neither offered nor allowed (it was offered `hero-p2` before #2521)...
        assert!(!offers(&s, &tmack, "hero-p2"));
        s.expect_refused_with(|s| s.attack(&tmack, "hero"), "Rush cannot hit the hero");
        // ...an enemy Unit is.
        let next = s.unit_at(P2, 2);
        assert!(offers(&s, &tmack, &next.id));
        s.attack(&tmack, &next);
        assert!(s.unit(P2, 2).is_none(), "the second Vanilla died");
        s.expect_health(P2, HERO_HEALTH);
        s.expect_refused(|s| s.attack(&tmack, "hero"));
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
            let mut s = in_hand(false);
            s.play(ID, json!({ "zone": 1 }));
            let tmack = s.unit_at(P1, 1);
            assert!(has_keyword(&s.stats(&tmack).keywords, KeywordKind::Rush));
            // Rush: a Unit, not the hero, the turn it arrives (§6.1).
            s.expect_refused(|s| s.attack(&tmack, "hero"));
            s.attack(&tmack, s.unit_at(P2, 1));
            assert!(s.unit(P2, 1).is_none(), "the Vanilla died");
        }

        #[test]
        fn r2521002_a_kill_on_its_summon_turn_frees_it_to_attack_a_unit_and_never_the_hero() {
            r2521002_summon_turn(false);
        }

        #[test]
        fn r2521002_on_a_later_turn_its_extra_attack_may_hit_the_hero() {
            let mut s = in_hand(false);
            s.play(ID, json!({ "zone": 1 }));
            let tmack = s.unit_at(P1, 1);
            s.attack(&tmack, s.unit_at(P2, 1));
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().active, P1);
            s.attack(&tmack, s.unit_at(P2, 2));
            assert!(s.unit(P2, 2).is_none(), "the second Vanilla died");
            assert!(offers(&s, &tmack, "hero-p2"));
            s.attack(&tmack, "hero");
            s.expect_health(P2, HERO_HEALTH - 9);
        }

        /// R767000: the script kept for the replays of v0.5.11 and before, whose kill lifted the sickness.
        #[test]
        fn r767000_a_game_on_v0_5_11_lets_its_summon_turn_kill_free_it_to_hit_the_hero() {
            crate::under_kept(ID, script_v0_5_11(), || {
                let mut s = in_hand(false);
                s.play(ID, json!({ "zone": 1 }));
                let tmack = s.unit_at(P1, 1);
                s.attack(&tmack, s.unit_at(P2, 1));
                assert!(offers(&s, &tmack, "hero-p2"));
                s.attack(&tmack, "hero");
                s.expect_health(P2, HERO_HEALTH - 9);
            });
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

        #[test]
        fn r2521002_a_kill_on_its_summon_turn_frees_it_to_attack_a_unit_and_never_the_hero() {
            r2521002_summon_turn(true);
        }
    }
}
