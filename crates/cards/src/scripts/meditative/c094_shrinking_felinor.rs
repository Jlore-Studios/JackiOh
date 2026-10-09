//! M #94 Shrinking Felinor (SPEC §8.8 row 94): (4) Unit, Felinor, Rare, 9/11 → 18/22.
//!
//! Base:    "Death: Summon a Shrinking Felinor whose base stats are this one's
//!           −{shrinkAttack}/−{shrinkHealth}."
//! Radiant: "Death: Summon a Radiant Shrinking Felinor whose base stats are this one's
//!           −{shrinkAttack}/−{shrinkHealth}."
//! Engine:
//! - **What it reads:** `ctx.self_` is the card as it died, its `statsOverride` still on it (R78), and
//!   `base_stats_of` reads its §10.4 layer-1 stats off that: the override as its face wears it, else its
//!   printed stats (R1181). Buffs, tuning, auras and damage are not base stats. Reading the graveyard
//!   copy instead would restart every chain at 9/11, since R78 clears the override there.
//! - **What it summons:** a fresh Shrinking Felinor on the same face, not a copy (R57), so no buff, granted
//!   keyword or tuning carries, into the leftmost open unit zone (R64). Its `statsOverride` is the attack
//!   less `shrinkAttack` (at least 0) and the health less `shrinkHealth`.
//! - **The end:** a body whose health would be 0 or less is not summoned, so the chain ends: 9/11, 6/8,
//!   3/5, 0/2 on the base face and 18/22 down to 4/1 on the Radiant one. Each shrink is a declared
//!   number (R386) that never tunes below 1, so the chain always ends.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-094";

fn shrinking(radiant: bool) -> Script {
    Script {
        death: Some(hook(move |ctx| {
            let Some(this) = ctx.self_.clone() else {
                return vec![];
            };
            let base = base_stats_of(ctx.state, &this);
            let health = base.health - param(&*ctx, "shrinkHealth");
            // R1181: a body whose health would be 0 or less is not summoned, which ends the chain.
            if health <= 0 {
                return vec![];
            }
            let attack = (base.attack - param(&*ctx, "shrinkAttack")).max(0);
            vec![summon(json_as(json!({
                "defId": ID,
                "player": "self",
                "radiant": radiant,
                "statsOverride": { "attack": attack, "health": health },
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: shrinking(false),
        radiant: shrinking(true),
    }
}

// M #94 Shrinking Felinor — SPEC §8.8 row 94, BUILD M10 row M 94: "Its Death summons a fresh Shrinking
// Felinor on its face with base stats 3/3 smaller (attack floored at 0), no buffs or keywords carried
// (MD-E20): 9/11, 6/8, 3/5, 0/2, then none, since the health would be 0 or less; a full board summons
// none; the shrink reads through `param()`; radiant 18/22 and a Radiant one 2/3 smaller, eight bodies:
// 18/22, 16/19, 14/16, 12/13, 10/10, 8/7, 6/4, 4/1".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::layers::keywords_of;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const VANILLA: &str = "core-008"; // Mr. Vanilla, 4/4, no text

    fn game(field: Value, hits: usize) -> Scenario {
        crate::scenario(json!({
            "p1": { "field": field, "hand": vec![HIT_JOB; hits], "mana": 30 },
        }))
    }

    fn hit(s: &mut Scenario, id: &str) {
        s.play(
            HIT_JOB,
            json!({ "targets": [{ "pick": "instance", "instanceId": id }] }),
        );
    }

    fn row_defs(s: &Scenario) -> Vec<Option<String>> {
        (1..=5)
            .map(|lane| s.unit(P1, lane).map(|unit| unit.def_id))
            .collect()
    }

    /// Kills the Shrinking Felinor in lane 1 and returns the body that stands there after.
    fn shrink(s: &mut Scenario) -> Option<CardInstance> {
        let id = s
            .unit(P1, 1)
            .unwrap_or_else(|| panic!("expected a p1 unit in lane 1"))
            .id;
        hit(s, &id);
        s.unit(P1, 1)
    }

    /// The bodies the chain runs through from `s`'s lane 1 on, as (attack, health, radiant), until the
    /// summon fails.
    fn run_chain(s: &mut Scenario) -> Vec<(i32, i32, bool)> {
        let mut bodies = Vec::new();
        while let Some(next) = shrink(s) {
            assert_eq!(next.def_id, ID);
            let view = s.stats(&next);
            bodies.push((view.attack, view.max_health, next.radiant));
        }
        bodies
    }

    mod m94_shrinking_felinor {
        use super::*;

        #[test]
        fn is_a_4_cost_9_11_felinor_and_radiant_18_22_with_its_shrinks_declared() {
            let def = crate::card_def(ID);
            assert_eq!(def.cost, CardCost::Fixed(4));
            assert!(!def.token);
            assert_eq!(def.rarity, Rarity::Rare);
            assert_eq!([def.base.attack, def.base.health], [Some(9), Some(11)]);
            assert_eq!([def.radiant.attack, def.radiant.health], [Some(18), Some(22)]);
            assert!(def.base.keywords.is_empty());
            assert!(def.radiant.keywords.is_empty());
            let params = def.params.unwrap_or_default();
            let shrink_attack = params.iter().find(|param| param.key == "shrinkAttack");
            let shrink_health = params.iter().find(|param| param.key == "shrinkHealth");
            assert_eq!(
                shrink_attack.map(|param| (param.base, param.radiant, param.min)),
                Some((3, 2, Some(1)))
            );
            assert_eq!(
                shrink_health.map(|param| (param.base, param.radiant, param.min)),
                Some((3, 3, Some(1)))
            );
        }

        mod base {
            use super::*;

            #[test]
            fn r1181_r78_the_chain_runs_9_11_6_8_3_5_0_2_and_then_ends() {
                let mut s = game(json!([ID]), 4);
                let start = s.unit(P1, 1).unwrap_or_else(|| panic!("the card"));
                s.expect_stats(&start, json!({ "attack": 9, "maxHealth": 11 }));
                // R78: each body is read off the one that died, its override included, not its printed face.
                assert_eq!(
                    run_chain(&mut s),
                    vec![(6, 8, false), (3, 5, false), (0, 2, false)]
                );
                assert_eq!(row_defs(&s), vec![None, None, None, None, None]);
            }

            #[test]
            fn r1181_r57_buffs_and_keywords_never_carry() {
                let mut s = game(json!([ID]), 1);
                let id = unit_id(&s);
                s.card_mut(id.as_str()).buffs = AttackHealth { attack: 3, health: 3 };
                s.card_mut(id.as_str()).granted_keywords = vec![Keyword::Taunt];
                let next = shrink(&mut s).unwrap_or_else(|| panic!("the next body"));
                // 9/11 less 3/3: the buff's +3/+3 is not a base stat, and Taunt stays on the dead card.
                s.expect_stats(&next, json!({ "attack": 6, "maxHealth": 8 }));
                assert!(keywords_of(s.state(), &next).is_empty());
                assert_eq!(next.buffs, AttackHealth::default());
            }

            #[test]
            fn r64_a_full_board_summons_none() {
                // The card tops a pile on lane 1; when it dies the Vanilla under it resumes there (§3.2).
                let mut s = game(
                    json!([VANILLA, { "def": ID, "stack": true }, VANILLA, VANILLA, VANILLA, VANILLA]),
                    1,
                );
                let id = unit_id(&s);
                hit(&mut s, &id);
                s.expect_in_zone(&id, "graveyard");
                assert!(!row_defs(&s).contains(&Some(ID.to_string())));
            }

            #[test]
            fn r386_a_buff_shrinks_the_attack_by_one_less() {
                let mut s = game(json!([ID]), 1);
                let id = unit_id(&s);
                assert_eq!(crate::upgrade_number(&mut s, &id, "shrinkAttack"), 2);
                let next = shrink(&mut s).unwrap_or_else(|| panic!("the next body"));
                s.expect_stats(&next, json!({ "attack": 7, "maxHealth": 8 }));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r1181_the_chain_runs_eight_radiant_bodies_down_to_4_1() {
                let mut s = game(json!([{ "def": ID, "radiant": true }]), 8);
                let start = s.unit(P1, 1).unwrap_or_else(|| panic!("the card"));
                s.expect_stats(&start, json!({ "attack": 18, "maxHealth": 22 }));
                assert_eq!(
                    run_chain(&mut s),
                    vec![
                        (16, 19, true),
                        (14, 16, true),
                        (12, 13, true),
                        (10, 10, true),
                        (8, 7, true),
                        (6, 4, true),
                        (4, 1, true),
                    ]
                );
                assert_eq!(row_defs(&s), vec![None, None, None, None, None]);
            }

            #[test]
            fn r1181_a_radiant_one_keeps_its_override_and_shrinks_by_the_radiant_numbers() {
                let mut s = game(
                    json!([{ "def": ID, "radiant": true, "statsOverride": { "attack": 6, "health": 8 } }]),
                    1,
                );
                let next = shrink(&mut s).unwrap_or_else(|| panic!("the next body"));
                assert!(next.radiant);
                // 6/8 less the Radiant 2/3: the printed 18/22 never enters into it.
                s.expect_stats(&next, json!({ "attack": 4, "maxHealth": 5 }));
            }

            #[test]
            fn r386_the_radiant_attack_shrink_is_never_tuned_below_1() {
                let mut s = game(json!([{ "def": ID, "radiant": true }]), 1);
                let id = unit_id(&s);
                assert_eq!(crate::upgrade_number(&mut s, &id, "shrinkAttack"), 1);
                assert!(!crate::can_upgrade_number(&s, &id, "shrinkAttack"));
            }
        }

        fn unit_id(s: &Scenario) -> String {
            s.unit(P1, 1)
                .unwrap_or_else(|| panic!("expected a p1 unit in lane 1"))
                .id
        }
    }
}
