//! C+ #12.4 Powder Spray (SPEC §8.7 row 12.4): (1) Spell, Pancake, Token (printed Legendary).
//!   Both faces: "Deal {damage} damage to each enemy." — 3, Radiant 6.
//! One hit each on every enemy unit on top of its pile and the enemy hero, all before the state check (R59).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-012-4";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![damage_all(json_as(json!({ "amount": param(&*ctx, "damage"), "side": "enemy", "heroes": true })))]
        })),
        ..Script::default()
    };
    // The Radiant face is the same text at 6, a catalog value.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C+ #12.4 Powder Spray — SPEC §8.7 row 12.4, BUILD M9 Classic+ row C+ 12.4: "3 damage to the enemy
// hero and to each enemy Unit, separate instances all landing before the state check (R59); Spell
// Damage raises each hit; an Immune to Spells Unit takes none; your side is untouched; the damage reads
// through `param()`; radiant 6".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const SPRAY: &str = "classicplus-012-4";
    const MENACE: &str = "core-019"; // 9/9
    const POINTMASTER: &str = "core-020"; // 7/1
    const SOLARIUS: &str = "classicplus-038"; // Spell Damage +2
    const TOP_LOSER: &str = "classicplus-019-1"; // Radiant: Immune to Spells
    const FILLER: &str = "core-005";

    use crate::merged;

    fn spray(radiant: bool, p1: Value, p2: Value) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": merged(json!({ "hand": [{ "def": SPRAY, "radiant": radiant }, FILLER], "field": [MENACE] }), p1),
            "p2": merged(json!({ "hand": [FILLER], "field": [MENACE, POINTMASTER] }), p2),
        }));
        s.play(SPRAY, json!({}));
        s
    }

    /// TS `s.stats(s.unit(player, lane) ?? "").health`.
    fn health_at(s: &Scenario, player: PlayerId, lane: i32) -> i32 {
        s.stats(s.unit(player, lane).map(|unit| unit.id).unwrap_or_default()).health
    }

    mod base {
        use super::*;

        #[test]
        fn t_3_damage_to_the_enemy_hero_and_each_enemy_unit_your_side_untouched() {
            crate::register_all();
            let mut s = spray(false, json!({}), json!({}));
            s.expect_health(P2, HERO_HEALTH - 3).expect_health(P1, HERO_HEALTH);
            assert_eq!(health_at(&s, P2, 1), 6);
            assert_eq!(health_at(&s, P1, 1), 9);
        }

        #[test]
        fn r59_separate_instances_all_landing_before_the_state_check() {
            crate::register_all();
            let s = spray(false, json!({}), json!({}));
            let hits: Vec<usize> = s
                .events()
                .iter()
                .enumerate()
                .filter(|(_, event)| event.event_type() == GameEventType::Damage)
                .map(|(i, _)| i)
                .collect();
            let deaths: Vec<usize> = s
                .events()
                .iter()
                .enumerate()
                .filter(|(_, event)| event.event_type() == GameEventType::Destroyed)
                .map(|(i, _)| i)
                .collect();
            assert_eq!(hits.len(), 3);
            assert_eq!(deaths.len(), 1); // the Pointmaster
            assert!(hits.iter().max() < deaths.iter().min());
        }

        #[test]
        fn spell_damage_raises_each_hit() {
            crate::register_all();
            let mut s = spray(false, json!({ "field": [SOLARIUS] }), json!({}));
            s.expect_health(P2, HERO_HEALTH - 5);
            assert_eq!(health_at(&s, P2, 1), 4);
        }

        #[test]
        fn an_immune_to_spells_unit_takes_none() {
            crate::register_all();
            let mut s = spray(false, json!({}), json!({ "field": [{ "def": TOP_LOSER, "radiant": true }] }));
            assert_eq!(s.stats(TOP_LOSER).health, 10);
            s.expect_health(P2, HERO_HEALTH - 3);
        }

        #[test]
        fn r386_the_damage_reads_through_param_an_upgrade_deals_4() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [SPRAY, FILLER] }, "p2": { "hand": [FILLER] } }));
            step_param(s.card_mut(SPRAY), "damage", 1);
            s.play(SPRAY, json!({}));
            s.expect_health(P2, HERO_HEALTH - 4);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn t_6_damage_to_each_enemy() {
            crate::register_all();
            let mut s = spray(true, json!({}), json!({}));
            s.expect_health(P2, HERO_HEALTH - 6);
            assert_eq!(health_at(&s, P2, 1), 3);
        }
    }
}
