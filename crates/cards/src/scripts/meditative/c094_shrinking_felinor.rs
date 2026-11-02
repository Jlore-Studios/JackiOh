//! M #94 Shrinking Felinor (SPEC §8.8 row 94): (4) Unit, Felinor, Rare, 9/9 → 18/18.
//!
//! Base and Radiant: "Death: Summon a Shrinking Felinor whose base stats are this one's
//!                   −{shrinkAttack}/−{shrinkHealth}."
//! Engine:
//! - **What it reads:** `ctx.self_` is the card as it died, its `statsOverride` still on it (R78), and
//!   `base_stats_of` reads its §10.4 layer-1 stats off that: the override as its face wears it, else its
//!   printed stats (R1181). Buffs, tuning, auras and damage are not base stats. Reading the graveyard
//!   copy instead would restart every chain at 9/9, since R78 clears the override there.
//! - **What it summons:** a fresh base Shrinking Felinor, not a copy (R57), so no buff, granted keyword or
//!   tuning carries, into the leftmost open unit zone (R64), shrunk by the dying face's numbers. The
//!   Radiant face summons a base body too (R1828003, #1828), so its chain goes on on the base face.
//! - **The end:** a body whose health would be 0 or less is not summoned; each shrink is a declared
//!   number (R386) that never tunes below 1, so the chain always ends.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-094";

/// The base stats the Death summons the next Felinor with: this one's base stats less the shrink,
/// attack floored at 0; `None` when its health would be 0 or less (R1181), which ends the chain.
fn next_body(state: &GameState, this: &CardInstance, shrink_attack: i32, shrink_health: i32) -> Option<AttackHealth> {
    let base = base_stats_of(state, this);
    let health = base.health - shrink_health;
    (health > 0).then(|| AttackHealth { attack: (base.attack - shrink_attack).max(0), health })
}

/// R2476000: after "Summon a Shrinking Felinor", the base stats its Death would summon one with ("6/6"),
/// its own base stats down the chain less the shrink; the dash where the chain ends. Its value is that
/// Health, 0 for none. On the field, where a chain lowers its base stats; in hand the face's numbers
/// say it already (R78). Base stats are public.
fn preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if ctx.zone != ConditionZone::Field {
            return vec![];
        }
        let next = next_body(ctx.state, ctx.self_, param(&ctx, "shrinkAttack"), param(&ctx, "shrinkHealth"));
        vec![PreviewValue {
            label: "Summon a Shrinking Felinor".to_string(),
            value: next.map_or(0, |next| next.health),
            display: Some(next.map_or_else(
                || NOTHING_REMEMBERED.to_string(),
                |next| format!("{}/{}", next.attack, next.health),
            )),
            ids: None,
        }]
    })
}

fn shrinking(summons_radiant: bool) -> Script {
    Script {
        death: Some(hook(move |ctx| {
            let Some(this) = ctx.self_.clone() else {
                return vec![];
            };
            let Some(next) =
                next_body(ctx.state, &this, param(&*ctx, "shrinkAttack"), param(&*ctx, "shrinkHealth"))
            else {
                return vec![];
            };
            vec![summon(json_as(json!({
                "defId": ID,
                "player": "self",
                "radiant": summons_radiant,
                "statsOverride": { "attack": next.attack, "health": next.health },
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // R1828003: both faces summon a base body; the Radiant face still shrinks by its own numbers.
    // R2476000: the preview is the current faces' alone; the kept script below ran without one.
    CardScripts {
        base: Script { preview: Some(preview()), ..shrinking(false) },
        radiant: Script { preview: Some(preview()), ..shrinking(false) },
    }
}

/// R767000: through v0.4.9 the Radiant face summoned a Radiant body; `KEPT_SCRIPTS` keeps it.
pub fn script_v0_4_9() -> CardScripts {
    CardScripts {
        base: shrinking(false),
        radiant: shrinking(true),
    }
}

// M #94 Shrinking Felinor — SPEC §8.8 row 94, BUILD M10 row M 94 (MD-E20).
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

        mod base {
            use super::*;

            #[test]
            fn r1181_r78_r1828003_the_chain_runs_9_9_6_6_3_3_and_then_ends() {
                let mut s = game(json!([ID]), 3);
                let start = s.unit(P1, 1).unwrap_or_else(|| panic!("the card"));
                s.expect_stats(&start, json!({ "attack": 9, "maxHealth": 9 }));
                // R78: each body is read off the one that died, its override included, not its printed face.
                assert_eq!(run_chain(&mut s), vec![(6, 6, false), (3, 3, false)]);
                assert_eq!(row_defs(&s), vec![None, None, None, None, None]);
            }

            #[test]
            fn r1181_r57_buffs_and_keywords_never_carry() {
                let mut s = game(json!([ID]), 1);
                let id = unit_id(&s);
                s.card_mut(id.as_str()).buffs = AttackHealth { attack: 3, health: 3 };
                s.card_mut(id.as_str()).granted_keywords = vec![Keyword::Taunt];
                let next = shrink(&mut s).unwrap_or_else(|| panic!("the next body"));
                // 9/9 less 3/3: the buff's +3/+3 is not a base stat, and Taunt stays on the dead card.
                s.expect_stats(&next, json!({ "attack": 6, "maxHealth": 6 }));
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
                s.expect_stats(&next, json!({ "attack": 7, "maxHealth": 6 }));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r1181_r1828003_the_radiant_face_summons_base_bodies_15_15_down_to_3_3() {
                let mut s = game(json!([{ "def": ID, "radiant": true }]), 6);
                let start = s.unit(P1, 1).unwrap_or_else(|| panic!("the card"));
                s.expect_stats(&start, json!({ "attack": 18, "maxHealth": 18 }));
                assert_eq!(
                    run_chain(&mut s),
                    vec![
                        (15, 15, false),
                        (12, 12, false),
                        (9, 9, false),
                        (6, 6, false),
                        (3, 3, false),
                    ]
                );
                assert_eq!(row_defs(&s), vec![None, None, None, None, None]);
            }

            #[test]
            fn r1181_r1828003_a_radiant_one_keeps_its_override_and_shrinks_by_the_radiant_numbers_into_a_base_body() {
                let mut s = game(
                    json!([{ "def": ID, "radiant": true, "statsOverride": { "attack": 6, "health": 8 } }]),
                    1,
                );
                let next = shrink(&mut s).unwrap_or_else(|| panic!("the next body"));
                assert!(!next.radiant);
                // 6/8 less the Radiant 3/3: the printed 18/18 never enters into it.
                s.expect_stats(&next, json!({ "attack": 3, "maxHealth": 5 }));
            }

            #[test]
            fn r386_the_radiant_attack_shrink_is_never_tuned_below_1() {
                let mut s = game(json!([{ "def": ID, "radiant": true }]), 1);
                let id = unit_id(&s);
                assert_eq!(crate::upgrade_number(&mut s, &id, "shrinkAttack"), 2);
                assert_eq!(crate::upgrade_number(&mut s, &id, "shrinkAttack"), 1);
                assert!(!crate::can_upgrade_number(&s, &id, "shrinkAttack"));
            }
        }

        /// R767000: the script kept for the replays of v0.4.9 and before.
        mod v0_4_9 {
            use super::*;

            #[test]
            fn r767000_r1828003_a_game_on_v0_4_9_folds_under_the_kept_script_the_radiant_face_summons_a_radiant_16_19()
            {
                crate::register_all();
                jackioh_engine::with_shipped(crate::catalog_at("v0.4.9"), || {
                    let mut s = game(json!([{ "def": ID, "radiant": true }]), 1);
                    let start = s.unit(P1, 1).unwrap_or_else(|| panic!("the card"));
                    // v0.4.9's own numbers: 18/22, shrinking 2/3.
                    s.expect_stats(&start, json!({ "attack": 18, "maxHealth": 22 }));
                    let next = shrink(&mut s).unwrap_or_else(|| panic!("the next body"));
                    assert!(next.radiant);
                    s.expect_stats(&next, json!({ "attack": 16, "maxHealth": 19 }));
                });
            }
        }

        fn unit_id(s: &Scenario) -> String {
            s.unit(P1, 1)
                .unwrap_or_else(|| panic!("expected a p1 unit in lane 1"))
                .id
        }
    }

    /// R2476000: after "Summon a Shrinking Felinor", the stats its Death would summon, to both seats: 6/6
    /// on the 9/9, and 3/3 on the 6/6 that Death summons.
    #[test]
    fn r2476000_it_previews_the_stats_its_death_would_summon() {
        let mut s = game(json!([ID]), 1);
        let first = s.unit(P1, 1).expect("the Felinor").id;
        let label = "Summon a Shrinking Felinor".to_string();
        assert_eq!(crate::preview_seen(&s, P1, &first), vec![(label.clone(), 6, Some("6/6".to_string()))]);
        assert_eq!(crate::preview_seen(&s, PlayerId::P2, &first), vec![(label.clone(), 6, Some("6/6".to_string()))]);
        let next = shrink(&mut s).expect("a 6/6");
        assert_eq!(crate::preview_seen(&s, P1, &next.id), vec![(label, 3, Some("3/3".to_string()))]);
    }
}
