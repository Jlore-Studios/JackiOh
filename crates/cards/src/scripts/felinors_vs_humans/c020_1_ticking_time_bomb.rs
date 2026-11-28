//! FvH #20.1 Ticking Time Bomb (SPEC §8.14 row 20.1): (1) Unit, Token (printed Common), 0/2 → 0/4, the
//! token FvH #20 Bomb summons.
//!   Base:    "Can't Defend, Brittle 2\nActivate: Tribute this.\nDeath: Deal {damage} damage to both heroes
//!            and all Units." (3)
//!   Radiant: "Can't Defend, Brittle 3\nActivate: Tribute this.\nDeath: Deal {damage} damage to all
//!            enemies." (3)
//!
//! Can't Defend (R2657000) and Brittle (R385) are catalog keywords; Brittle's crumble is an ordinary
//! destroy, so its Death runs. Activate, once a turn (R384): Tribute this as its cost (C #84's shape), a
//! Sacrifice that runs its Death (R1840006, R1182000). Death: `damage_all` of the declared `damage` over
//! every Unit on both sides and both heroes; Radiant, over the enemy Units and the enemy hero. Its source
//! is the dead token (R59). Tunes: damage 3 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-020-1";

/// "Activate" is once a turn (R384).
const USES: i32 = 1;

fn tribute_this() -> ActivationDecl {
    ActivationDecl {
        id: "tribute".to_string(),
        label: "Tribute this".to_string(),
        uses: ActivationUses::Count(USES),
        cost: Some(ActivationCost {
            tribute_self: Some(true),
            ..ActivationCost::default()
        }),
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(|_ctx| vec![]),
    }
}

/// "Deal {damage} damage to both heroes and all Units" (base) / "to all enemies" (Radiant).
fn explode(radiant: bool) -> Hook {
    hook(move |ctx| {
        let side = if radiant { "enemy" } else { "any" };
        vec![damage_all(json_as(json!({
            "amount": param(&*ctx, "damage"),
            "heroes": true,
            "side": side,
        })))]
    })
}

fn face(radiant: bool) -> Script {
    Script {
        activations: vec![tribute_this()],
        death: Some(explode(radiant)),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// FvH #20.1 Ticking Time Bomb — SPEC §8.14 row 20.1 (R384, R385, R1840006, R1182000, R2657000, R59).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A textless Spell each seat can always play, so no turn ends by itself (§2.5).
    const SPELL: &str = "stand-in (1) Spell";
    const ALLY: &str = "stand-in (1) 4/4 R12/12";
    const ENEMY: &str = "stand-in (2) 5/5";
    /// The heroes' health before any hit (§2.1).
    const HERO: i32 = 30;
    /// Its printed `damage`.
    const DAMAGE: i32 = 3;

    /// p1 holds the token on the field beside a 4/4; p2 a 5/5.
    fn primed(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [SPELL],
                "field": [{ "def": ID, "radiant": radiant }, ALLY],
                "library": filler(6),
            },
            "p2": { "hand": [SPELL], "field": [ENEMY], "library": filler(6) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r1840006_its_activate_tributes_it_and_its_death_deals_3_to_every_unit_and_both_heroes() {
            let mut s = primed(false);
            let bomb = s.card(ID).id.clone();
            s.activate(ID, json!({}));
            assert!(s.unit(P1, 1).is_none_or(|unit| unit.id != bomb), "tributed");
            s.expect_stats(ALLY, json!({ "health": 4 - DAMAGE }));
            s.expect_stats(ENEMY, json!({ "health": 5 - DAMAGE }));
            s.expect_health(P1, HERO - DAMAGE);
            s.expect_health(P2, HERO - DAMAGE);
        }

        #[test]
        fn r2657000_its_printed_cant_defend_keeps_it_out_of_defense_position() {
            let mut s = primed(false);
            s.expect_refused(|s| s.switch_position(ID));
            // The 4/4 beside it switches: the refusal is the Bomb's own keyword.
            s.switch_position(ALLY);
        }

        #[test]
        fn r385_its_brittle_2_crumbles_it_and_the_crumble_is_a_destroy_so_its_death_runs() {
            let mut s = scenario(json!({
                "p1": { "hand": [ID, SPELL], "library": filler(6) },
                "p2": { "hand": [SPELL], "field": [ENEMY], "library": filler(6) },
            }));
            s.play(ID, json!({ "zone": 1 }));
            for _ in 0..4 {
                s.end_turn();
            }
            assert_eq!(s.count("crumbled"), 1);
            s.expect_stats(ENEMY, json!({ "health": 5 - DAMAGE }));
            s.expect_health(P2, HERO - DAMAGE);
        }

        #[test]
        fn r386_its_damage_reads_through_param() {
            let mut s = primed(false);
            let bomb = s.card(ID).id.clone();
            set_param(s.card_mut(&bomb), "damage", DAMAGE + 1);
            s.activate(&bomb, json!({}));
            s.expect_health(P2, HERO - DAMAGE - 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1840006_its_death_deals_3_to_the_enemy_units_and_the_enemy_hero_only() {
            let mut s = primed(true);
            s.activate(ID, json!({}));
            s.expect_stats(ENEMY, json!({ "health": 5 - DAMAGE }));
            s.expect_health(P2, HERO - DAMAGE);
            s.expect_stats(ALLY, json!({ "health": 4 }));
            s.expect_health(P1, HERO);
        }

        #[test]
        fn r385_its_brittle_3_outlasts_the_base_faces_two_turns() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": ID, "radiant": true }, SPELL], "library": filler(6) },
                "p2": { "hand": [SPELL], "library": filler(6) },
            }));
            s.play(ID, json!({ "zone": 1 }));
            for _ in 0..4 {
                s.end_turn();
            }
            assert_eq!(s.count("crumbled"), 0);
            s.expect_in_zone(ID, "field");
        }
    }
}
