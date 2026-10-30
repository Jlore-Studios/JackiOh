//! R2521002 (issue #2521): `mayAttackAgain` with `keepSick` (S #94 Tmack Trav's "After this destroys a
//! Unit, it may attack again") gives the fresh exertion and leaves summoning sickness as it is (§4.1): a
//! Rush unit that kills on its summon turn may attack again a Unit only, the hero neither offered nor
//! allowed (§6.1), and one that entered on an earlier turn may hit the hero. Without it the verb lifts
//! the sickness for the turn as ever (B5 E35, C+ #73.1 Classic Golem).
//!
//! Fixtures are this file's own: `fx-` defs indexed from 25210 (BUILD §0).

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::unit_def;
use crate::rules::fixtures::harness::{both, setup_catalog};

/// The target id of p2's hero.
const ENEMY_HERO: &str = "hero-p2";
/// The keeper's and the lifter's attack: each kills a fodder and lives.
const STRIKER_ATTACK: i32 = 9;

/// "Rush / After this destroys a Unit, it may attack again", sickness kept (Tmack Trav's shape).
fn keeper() -> CardDef {
    unit_def(
        25210,
        json!({ "attack": STRIKER_ATTACK, "health": 9, "keywords": [{ "kind": "Rush" }] }),
    )
}

/// The same with the verb's old reading, the kill lifting its sickness (Classic Golem's shape).
fn lifter() -> CardDef {
    unit_def(
        25211,
        json!({ "attack": STRIKER_ATTACK, "health": 9, "keywords": [{ "kind": "Rush" }] }),
    )
}

/// A 1/1 to kill.
fn fodder() -> CardDef {
    unit_def(25212, json!({ "attack": 1, "health": 1 }))
}

/// After a kill of the Unit it attacked, `mayAttackAgain` on itself, `keepSick` when `keep_sick`.
fn again_after_a_kill(keep_sick: bool) -> Script {
    Script {
        after_attack: Some(hook(move |ctx| {
            let Some(combat) = after_attack_of(ctx) else {
                return Vec::new();
            };
            let killed = combat.survived
                && combat.target_id != ENEMY_HERO
                && combat.destroyed_ids.contains(&combat.target_id);
            if !killed {
                return Vec::new();
            }
            let mut args = json!({ "target": { "of": "self" } });
            if keep_sick {
                args["keepSick"] = json!(true);
            }
            vec![effects::may_attack_again(json_as(args))]
        })),
        ..Script::default()
    }
}

fn register() {
    setup_catalog();
    let mut catalog = registered_catalog().clone();
    for def in [keeper(), lifter(), fodder()] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts();
    scripts.insert(keeper().id, both(again_after_a_kill(true)));
    scripts.insert(lifter().id, both(again_after_a_kill(false)));
    register_scripts(scripts);
}

/// p1 holds `p1_hand` and has `p1_field`; p2 has three fodders, in lanes 1 to 3. Neither turn ends by itself.
fn board(seed: &str, p1_hand: Value, p1_field: Value) -> Scenario {
    register();
    let prey = fodder().id;
    let mut s = scenario(json!({
        "seed": seed,
        "p1": { "hand": p1_hand, "field": p1_field, "library": [prey, prey] },
        "p2": { "hand": [prey], "field": [prey, prey, prey], "library": [prey, prey] },
    }));
    s.state_mut().players.p1.auto_end_turn = Some(false);
    s.state_mut().players.p2.auto_end_turn = Some(false);
    s
}

/// Whether `legal_actions` offers `attacker` an attack on `target`.
fn offers(s: &Scenario, attacker: &str, target: &str) -> bool {
    legal_actions(s.state(), P1).iter().any(|action| {
        matches!(
            action,
            ActionBody::Attack { attacker_id, target_id, .. }
                if attacker_id == attacker && target_id == target
        )
    })
}

mod r2521002_may_attack_again_keeps_summoning_sickness {
    use super::*;

    #[test]
    fn r2521002_a_rush_unit_that_kills_on_its_summon_turn_attacks_again_a_unit_only() {
        let mut s = board("r2521002-summon-turn", json!([keeper().id, fodder().id]), json!([]));
        s.play(keeper().id.as_str(), json!({ "zone": 1 }));
        let striker = s.unit_at(P1, 1).id;
        s.attack(striker.as_str(), s.unit_at(P2, 1));
        assert!(s.unit(P2, 1).is_none(), "the fodder died");
        let live = s.card(striker.as_str());
        assert!(!live.exertion.attacked, "a fresh exertion");
        assert_eq!(live.summoned_turn, Some(s.state().turn), "still summoning sick");

        assert!(!offers(&s, &striker, ENEMY_HERO), "the hero is not offered");
        s.expect_refused_with(
            |s| s.attack(striker.as_str(), "hero"),
            "Rush cannot hit the hero on its summon turn",
        );
        let next = s.unit_at(P2, 2).id;
        assert!(offers(&s, &striker, &next), "an enemy Unit is");
        s.attack(striker.as_str(), next.as_str());
        assert!(s.unit(P2, 2).is_none(), "the second fodder died");
        s.expect_health(P2, HERO_HEALTH);
    }

    #[test]
    fn r2521002_one_that_entered_on_an_earlier_turn_may_hit_the_hero_after_its_kill() {
        let mut s = board(
            "r2521002-earlier",
            json!([fodder().id]),
            json!([{ "def": keeper().id, "lane": 1 }]),
        );
        let striker = s.unit_at(P1, 1).id;
        s.attack(striker.as_str(), s.unit_at(P2, 1));
        assert!(s.unit(P2, 1).is_none(), "the fodder died");
        assert!(offers(&s, &striker, ENEMY_HERO));
        s.attack(striker.as_str(), "hero");
        s.expect_health(P2, HERO_HEALTH - STRIKER_ATTACK);
    }

    #[test]
    fn r2521002_played_its_sickness_ends_with_the_turn_and_a_later_kill_frees_it_to_hit_the_hero() {
        let mut s = board("r2521002-next-turn", json!([keeper().id, fodder().id]), json!([]));
        s.play(keeper().id.as_str(), json!({ "zone": 1 }));
        let striker = s.unit_at(P1, 1).id;
        s.attack(striker.as_str(), s.unit_at(P2, 1));
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P1);
        s.attack(striker.as_str(), s.unit_at(P2, 2));
        assert!(offers(&s, &striker, ENEMY_HERO));
        s.attack(striker.as_str(), "hero");
        s.expect_health(P2, HERO_HEALTH - STRIKER_ATTACK);
    }

    #[test]
    fn r2521002_without_keep_sick_the_kill_lifts_the_sickness_for_the_turn_as_ever() {
        let mut s = board("r2521002-lifted", json!([lifter().id, fodder().id]), json!([]));
        s.play(lifter().id.as_str(), json!({ "zone": 1 }));
        let striker = s.unit_at(P1, 1).id;
        s.attack(striker.as_str(), s.unit_at(P2, 1));
        assert_eq!(s.card(striker.as_str()).summoned_turn, None, "the sickness is lifted");
        assert!(offers(&s, &striker, ENEMY_HERO));
        s.attack(striker.as_str(), "hero");
        s.expect_health(P2, HERO_HEALTH - STRIKER_ATTACK);
    }

    #[test]
    fn r2521002_keep_sick_travels_as_keep_sick_and_is_left_off_when_absent() {
        let args: effects::MayAttackAgainArgs =
            json_as(json!({ "target": { "of": "self" }, "keepSick": true }));
        assert_eq!(args.keep_sick, Some(true));
        assert_eq!(
            serde_json::to_value(&args).expect("serialises")["keepSick"],
            json!(true)
        );
        let plain: effects::MayAttackAgainArgs = json_as(json!({ "target": { "of": "self" } }));
        assert_eq!(plain.keep_sick, None);
        let wire = serde_json::to_value(&plain).expect("serialises");
        assert!(wire.get("keepSick").is_none(), "{wire}");
    }
}
