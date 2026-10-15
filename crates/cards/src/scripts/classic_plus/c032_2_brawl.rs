//! C+ #32.2 Brawl (SPEC §8.7 row 32.2; §6.1, §6.3 Destroy, R46, R59, R60, R81, R129). (2) Spell token,
//! printed Epic.
//!   Base:    "Destroy all Units but one chosen at random."
//!   Radiant: "Destroy all Units but one of your choice."
//!
//! The survivor is one of the Units on the field a Spell may affect (either side, the tops of piles; an
//! Immune to Spells Unit is not among them and stays anyway, §6.1): drawn by the match rng on the base
//! face (R60), and on the Radiant face a Unit target declared with the play (R81). Every other such Unit
//! is marked at once, so one state check collects them all (R59); an Indestructible one survives
//! anyway (R46) and a Reborn one comes back. With no Unit there is nothing to destroy and nothing is
//! drawn (R129).

use jackioh_engine::effects::{
    ForEachCardArgs, cards_in_scope, destroy, for_each_card, instance_of, remember_random,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-032-2";

/// Where the base face keeps the survivor it drew, until the destroy reads it.
const SURVIVOR: &str = "survivor";

/// R81: the Radiant face's survivor, one Unit of either side.
fn survivor_pick() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

/// "All Units": both sides' Units a Spell may affect, the tops of piles (§3.2), in R68's order.
fn unit_ids(ctx: &EffectContext<'_>) -> Vec<String> {
    cards_in_scope(ctx, &json_as(json!({ "side": "any" })))
        .iter()
        .map(|unit| unit.id.clone())
        .collect()
}

/// Every such Unit but `survivor`, each marked destroyed; read as the list reaches it.
fn destroy_all_but(survivor: impl Fn(&EffectContext<'_>) -> Option<String> + Send + Sync + 'static) -> Effect {
    for_each_card(ForEachCardArgs {
        cards: Arc::new(move |ctx: &mut EffectContext<'_>| {
            let spared = survivor(&*ctx);
            unit_ids(&*ctx)
                .into_iter()
                .filter(|id| Some(id) != spared.as_ref())
                .collect()
        }),
        each: Arc::new(|instance_id: &str| {
            destroy(json_as(json!({ "target": { "of": "instance", "instanceId": instance_id } })))
        }),
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![
                // R60: one draw of the match rng over the Units it would destroy; none, no draw (R129).
                remember_random(json_as(json!({ "key": SURVIVOR, "options": unit_ids(&*ctx) }))),
                destroy_all_but(|at| recalled(at, SURVIVOR).and_then(|drawn| drawn.as_str().map(str::to_string))),
            ]
        })),
        ..Script::default()
    };
    let radiant = Script {
        targets: survivor_pick(),
        cry: Some(hook(|_ctx| {
            vec![destroy_all_but(|at| {
                instance_of(at, &json_as(json!({ "of": "chosen" }))).map(|unit| unit.id.clone())
            })]
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// C+ #32.2 Brawl — SPEC §8.7 row 32.2, BUILD M9 Classic+ row C+ 32.2: "Destroys every Unit on both
// sides but one survivor drawn at random (R60); Indestructible and Immune to Spells Units stay as well;
// Reborn units come back; no Units, nothing; radiant the survivor is a Unit you choose with the play
// (R81)".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BRAWL: &str = "classicplus-032-2";
    const MENACE: &str = "core-019";
    const TIMMY: &str = "core-011";
    /// #56 Jilliax: Divine Shield, which a destroy ignores.
    const JILLIAX: &str = "core-056";
    /// C #41 State of the Game, Indestructible.
    const UNBREAKABLE: &str = "classic-041";
    /// #3 Right-house defender: Taunt, Divine Shield, Reborn.
    const DEFENDER: &str = "core-003";
    const FILLER: &str = "core-005";
    /// C+ #19.1 Top Loser, whose Radiant face is Immune to Spells.
    const TOP_LOSER: &str = "classicplus-019-1";

    fn units(s: &Scenario) -> Vec<String> {
        [P1, P2]
            .into_iter()
            .flat_map(|player| (1..=5).filter_map(move |lane| s.unit(player, lane).map(|unit| unit.id)))
            .collect()
    }

    fn destroyed(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Destroyed { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn defs_of(s: &Scenario, ids: &[String]) -> Vec<String> {
        ids.iter().map(|id| s.card(id).def_id.clone()).collect()
    }

    use crate::js;

    fn brawl(seed: &str, radiant_face: bool, targets: Option<fn(&Scenario) -> Value>) -> (Scenario, Vec<String>) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": BRAWL, "radiant": radiant_face }, FILLER], "field": [TIMMY, MENACE] },
            "p2": { "field": [TIMMY, JILLIAX] },
        }));
        let before = units(&s);
        let options = match targets {
            None => json!({}),
            Some(targets) => json!({ "targets": targets(&s) }),
        };
        s.play(BRAWL, options);
        (s, before)
    }

    mod c_n32_2_brawl {
        use super::*;

        #[test]
        fn declares_nothing_on_the_base_face_and_one_unit_of_either_side_on_the_radiant_face_r81() {
            crate::register_all();
            assert_eq!(ID, BRAWL);
            assert_eq!(crate::card_def(ID).id, BRAWL);
            let scripts = script();
            assert!(scripts.base.targets.is_empty());
            assert_eq!(
                js(&scripts.radiant.targets),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit"] } }])
            );
        }

        mod base {
            use super::*;

            #[test]
            fn r60_destroys_every_unit_on_both_sides_but_one_survivor_drawn_at_random_all_in_one_check_r59() {
                crate::register_all();
                let (mut s, before) = brawl("brawl-one", false, None);
                let after = units(&s);
                assert_eq!(after.len(), 1);
                assert!(before.contains(&after[0]));
                let mut gone = destroyed(&s);
                gone.sort();
                let mut want: Vec<String> = before.iter().filter(|id| **id != after[0]).cloned().collect();
                want.sort();
                assert_eq!(gone, want);
                s.expect_in_zone(BRAWL, "graveyard");
            }

            #[test]
            fn r60_the_survivor_is_the_match_rng_s_the_same_seed_spares_the_same_unit_other_seeds_others() {
                crate::register_all();
                assert_eq!(units(&brawl("brawl-seed", false, None).0), units(&brawl("brawl-seed", false, None).0));
                let spared: IndexSet<i64> = ["a", "b", "c", "d", "e", "f", "g", "h"]
                    .iter()
                    .map(|seed| {
                        let (s, before) = brawl(&format!("brawl-{seed}"), false, None);
                        let survivor = units(&s).first().cloned().unwrap_or_default();
                        before.iter().position(|id| *id == survivor).map(|at| at as i64).unwrap_or(-1)
                    })
                    .collect();
                assert!(spared.len() > 1);
            }

            #[test]
            fn s9_3_the_random_survivor_replays_from_a_json_copy_to_the_same_hash() {
                crate::register_all();
                let s = scenario(json!({
                    "seed": "brawl-replay",
                    "p1": { "hand": [BRAWL, FILLER], "field": [TIMMY, MENACE] },
                    "p2": { "field": [TIMMY, JILLIAX] },
                }));
                let action: Action = json_as(json!({
                    "type": "play",
                    "instanceId": s.card(BRAWL).id,
                    "playerId": "p1",
                    "nonce": "brawl-replay",
                }));
                let thawed: GameState = serde_json::from_str(&serde_json::to_string(s.state()).unwrap()).unwrap();
                let live = reduce(s.state(), &action);
                assert!(live.error.is_none());
                assert_eq!(hash_state(&reduce(&thawed, &action).state), hash_state(&live.state));
            }

            #[test]
            fn r46_an_indestructible_unit_stays_as_well_a_reborn_unit_comes_back() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "brawl-keywords",
                    "p1": { "hand": [BRAWL, FILLER], "field": [UNBREAKABLE, MENACE] },
                    "p2": { "field": [DEFENDER, TIMMY] },
                }));
                s.play(BRAWL, json!({}));
                s.expect_in_zone(UNBREAKABLE, "field");
                s.expect_in_zone(DEFENDER, "field");
                let left = defs_of(&s, &units(&s));
                assert!(left.iter().any(|id| id == UNBREAKABLE));
                assert!(left.iter().any(|id| id == DEFENDER));
                assert!(left.len() <= 3);
            }

            #[test]
            fn s6_1_an_immune_to_spells_unit_is_not_among_the_candidates_and_stays_anyway() {
                crate::register_all();
                for seed in ["immune-a", "immune-b", "immune-c", "immune-d"] {
                    let mut s = scenario(json!({
                        "seed": seed,
                        "p1": { "hand": [BRAWL, FILLER], "field": [TIMMY, MENACE] },
                        "p2": { "field": [{ "def": TOP_LOSER, "radiant": true }] },
                    }));
                    s.play(BRAWL, json!({}));
                    let left = defs_of(&s, &units(&s));
                    assert!(left.iter().any(|id| id == TOP_LOSER));
                    assert_eq!(left.len(), 2);
                    assert_eq!(destroyed(&s).len(), 1);
                }
            }

            #[test]
            fn r129_with_no_unit_there_is_nothing_to_destroy_and_nothing_is_drawn() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [BRAWL, FILLER] } }));
                let cursor = s.state().rng_cursor;
                s.play(BRAWL, json!({}));
                assert!(destroyed(&s).is_empty());
                assert_eq!(s.state().rng_cursor, cursor);
                s.expect_in_zone(BRAWL, "graveyard");
            }

            #[test]
            fn s8_7_a_lone_unit_is_the_survivor() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [BRAWL, FILLER] }, "p2": { "field": [MENACE] } }));
                s.play(BRAWL, json!({}));
                s.expect_in_zone(MENACE, "field");
                assert!(destroyed(&s).is_empty());
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r81_the_survivor_is_the_unit_you_choose_with_the_play_an_enemy_one_included() {
                crate::register_all();
                let (s, before) = brawl(
                    "brawl-radiant",
                    true,
                    Some(|at| {
                        json!([{ "pick": "instance", "instanceId": at.unit(PlayerId::P2, 2).map(|unit| unit.id).unwrap_or_default() }])
                    }),
                );
                assert_eq!(defs_of(&s, &units(&s)), [JILLIAX]);
                assert_eq!(destroyed(&s).len(), before.len() - 1);
            }

            #[test]
            fn r81_an_immune_to_spells_unit_can_t_be_the_chosen_survivor_and_stays_all_the_same() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": BRAWL, "radiant": true }, FILLER], "field": [TIMMY] },
                    "p2": { "field": [{ "def": TOP_LOSER, "radiant": true }, MENACE] },
                }));
                let loser = s.card(TOP_LOSER).id.clone();
                s.expect_refused_with(
                    |s| s.play(BRAWL, json!({ "targets": [{ "pick": "instance", "instanceId": loser }] })),
                    "not a legal target",
                );
                let timmy = s.card(TIMMY).id.clone();
                s.play(BRAWL, json!({ "targets": [{ "pick": "instance", "instanceId": timmy }] }));
                let mut left = defs_of(&s, &units(&s));
                left.sort();
                let mut want = vec![TIMMY.to_string(), TOP_LOSER.to_string()];
                want.sort();
                assert_eq!(left, want);
            }

            #[test]
            fn r81_or_one_of_your_own() {
                crate::register_all();
                let (s, _) = brawl(
                    "brawl-radiant-own",
                    true,
                    Some(|at| {
                        json!([{ "pick": "instance", "instanceId": at.unit(PlayerId::P1, 2).map(|unit| unit.id).unwrap_or_default() }])
                    }),
                );
                assert_eq!(defs_of(&s, &units(&s)), [MENACE]);
            }
        }
    }
}
