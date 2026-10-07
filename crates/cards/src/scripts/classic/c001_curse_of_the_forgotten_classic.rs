//! C #1 Curse of the Forgotten Classic (SPEC §8.6 row 1, §6.3 Damage, Draw, Recruit, Forced attack;
//! R12, R33, R53, R63, R280). Spell, cost 1, Rare.
//!   Base:    "Deal {damage} damage to the enemy hero for each card in their exile. Draw {draw}."
//!   Radiant: "Deal {damage} damage to the enemy hero for each card in their exile. Draw {draw}.
//!            Recruit a card from their exile. If it's a Unit, it attacks the enemy hero at once."
//!   Engine:  "One hit of N on the enemy hero, N = the opponent's exile size as it resolves: "for each
//!            card" in one sentence is one hit, so Armor applies once, as Hearthstone reads it, and
//!            N = 0 is no hit (R63); then the draw. The Radiant face keeps the draw. Recruit (§6.3) from
//!            the opponent's exile: exile is chronological (§3), so Recruit's top-down scan is newest
//!            first, and the most recently exiled permanent card there is summoned under your control,
//!            no Cry (R1), its owner unchanged, so it goes to its owner's piles when it leaves the field
//!            (§3.2); a Unit then makes one forced attack on the enemy hero (R53), summoning sickness
//!            ignored; with no permanent in their exile nothing is recruited. Tunes: damage per card 1
//!            ↑; draw 1 ↑."
//!
//! ONE HIT: the damage per card (`param(ctx, "damage")`) times the opponent's exile size, read as the
//! Spell resolves, is a single `damage` on the enemy hero, so Armor and a hit cap meet it once. A total
//! of 0 is no damage instance at all (R63), so nothing is dealt. Then the draw (`param(ctx, "draw")`).
//!
//! THE PREVIEW (R280) is that total, computed by the same function the Cry deals with, under the label
//! "for each card in their exile" (the formula as both faces print it). It reads the opponent's exile
//! size and the card's own number, both public.
//!
//! THE RADIANT RECRUIT is the engine's E25 `recruit({ from: "exile", whose: "enemy" })`: their exile
//! scanned newest first for a permanent (never a Spell), summoned on your side under your control with
//! its owner unchanged (a Unit to your leftmost open unit zone, a Trap face-down to your backrow, read by
//! you alone, R33); with no open zone for it, or no permanent there, nothing is recruited. A Unit it
//! recruited then makes one forced attack on the enemy hero (`forcedAttacks` over the units of its
//! definition this list summoned, R53): no Taunt, position or summoning sickness stops it, and it spends
//! no exertion. Which card the Recruit takes is known before it happens (the newest permanent in their
//! exile), so the attacker is named by that definition as well as by "summoned by this list": a Unit a
//! card cast on the draw summoned (R58) is this list's too, and it does not attack.

use jackioh_engine::effects::{damage, draw, forced_attacks, recruit};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-001";

/// The label both faces print the formula under (R280).
const FORMULA: &str = "for each card in their exile";

/// N: the damage per card times the opponent's exile size, now. (TS took an `EffectContext |
/// ConditionContext`; both are a `ParamContext`, and the state they read is handed beside it.)
fn curse_damage(ctx: &impl ParamContext, state: &GameState, controller: PlayerId) -> i32 {
    param(ctx, "damage") * zone_count(state, opponent_of(controller), OffFieldZone::Exile)
}

fn hit_and_draw(ctx: &EffectContext<'_>) -> Vec<Effect> {
    vec![
        damage(json_as(json!({
            "to": { "of": "enemyHero" },
            "amount": curse_damage(ctx, &*ctx.state, ctx.controller),
        }))),
        draw(json_as(json!({ "count": param(ctx, "draw") }))),
    ]
}

/// TS `const preview: Script["preview"]`: the total under the formula's label (R280).
fn preview() -> PreviewHook {
    condition_hook(|c| {
        vec![PreviewValue {
            label: FORMULA.to_string(),
            value: curse_damage(&c, c.state, c.controller),
            display: None,
            ids: None,
        }]
    })
}

/// The card the Recruit will take: the newest permanent in the opponent's exile (a Spell is never
/// recruited, and a unit token is never in an exile, R11), read as the Spell resolves.
fn newest_permanent_of_theirs(ctx: &EffectContext<'_>) -> Option<CardInstance> {
    let state: &GameState = &*ctx.state;
    let exile = zone_cards(state, opponent_of(ctx.controller), OffFieldZone::Exile);
    exile
        .iter()
        .rev()
        .find(|card| def_of(Some(state), &card.def_id).type_ != CardType::Spell)
        .cloned()
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| hit_and_draw(ctx))),
        preview: Some(preview()),
        ..Script::default()
    };

    let radiant = Script {
        cry: Some(hook(|ctx| {
            let recruited = newest_permanent_of_theirs(ctx);
            let mut effects = hit_and_draw(ctx);
            effects.push(recruit(json_as(json!({ "from": "exile", "whose": "enemy" }))));
            // "If it's a Unit, it attacks": the unit this list summoned of that definition — not a Unit a
            // card cast on the draw summoned (C+ #26 Tommy Tempo), which is summoned by this list too.
            if let Some(card) = recruited
                && def_of(Some(&*ctx.state), &card.def_id).type_ == CardType::Unit
            {
                effects.push(forced_attacks(json_as(json!({
                    "attackers": { "side": "self", "defId": card.def_id, "summonedThisScript": true },
                    "target": { "spec": { "of": "enemyHero" } },
                }))));
            }
            effects
        })),
        preview: Some(preview()),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #1 Curse of the Forgotten Classic — SPEC §8.6 row 1, BUILD M9 Classic row C 1: "One hit of N on
// the enemy hero, N = the size of their exile as it resolves, so Armor applies once; an empty exile
// deals no hit (R63); then draw 1; its preview is N (R280); radiant: the same hit and draw (the
// Radiant keeps "Draw 1"), then Recruit from their exile its most recently exiled permanent card, under
// your control with its owner unchanged, back to their piles when it leaves the field (§3.2); a Unit
// recruited that way makes one forced attack on the enemy hero at once, summoning sickness ignored
// (R53); no permanent in their exile, or no open zone, → nothing recruited; a recruited trap is set
// face-down and read by you alone (R33); its tuned numbers (damage per card, draw) read through
// `param()` (R386)".
//
// The preview (R280) is proved in `test/preview.test.ts`, with the other cards that declare one.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CURSE: &str = "classic-001";
    const STOCKPILE: &str = "core-005"; // (1) Spell
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt
    const MR_TOKEN: &str = "core-015"; // (1) Unit 1/1: "Cry: Summon a Rush Token."
    const SHEEPISH: &str = "core-041"; // (1) Trap
    const MANA_WELL: &str = "core-006"; // (3) Field Spell
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy target Unit
    const RUSH_TOKEN: &str = "core-t-rush";

    use crate::js;

    use crate::matches_object;

    fn hits_on(s: &Scenario, player: PlayerId) -> Vec<i32> {
        let hero = format!("hero-{}", player.as_str());
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } if *target_id == hero => Some(*amount),
                _ => None,
            })
            .collect()
    }

    fn damage_events(s: &Scenario) -> Vec<Value> {
        s.events().iter().map(js).filter(|event| event["type"] == "damage").collect()
    }

    fn hand_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player).into_iter().map(|card| card.def_id).collect()
    }

    /// p2's exiled card of this definition (p1's library holds some of the same definitions).
    fn their_exiled(s: &Scenario, def_id: &str) -> CardInstance {
        match s.pile(P2, "exile").into_iter().find(|candidate| candidate.def_id == def_id) {
            Some(card) => card,
            None => panic!("p2's exile holds no {def_id}"),
        }
    }

    #[derive(Default)]
    struct CurseOpts {
        their_exile: Option<Vec<&'static str>>,
        my_exile: Vec<&'static str>,
        armor: Option<i32>,
        my_field: Vec<&'static str>,
        their_field: Vec<&'static str>,
    }

    fn curse(radiant_face: bool, opts: CurseOpts) -> Scenario {
        let mut p2 = json!({
            "hand": [STOCKPILE, HIT_JOB],
            "exile": opts.their_exile.unwrap_or_else(|| vec![STOCKPILE, STOCKPILE, STOCKPILE]),
            "field": opts.their_field,
            "library": [STOCKPILE, STOCKPILE],
        });
        if let Some(armor) = opts.armor {
            p2["armor"] = json!(armor);
        }
        scenario(json!({
            "p1": {
                "hand": [{ "def": CURSE, "radiant": radiant_face }, STOCKPILE],
                "library": [VANILLA, MENACE, STOCKPILE],
                "exile": opts.my_exile,
                "field": opts.my_field,
            },
            "p2": p2,
        }))
    }

    mod c_n1_curse_of_the_forgotten_classic {
        use super::*;

        #[test]
        fn declares_its_two_numbers_r386_and_a_preview_on_both_faces() {
            crate::register_all();
            assert_eq!(
                js(&crate::card_def(CURSE).params),
                json!([
                    { "key": "damage", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1 },
                    { "key": "draw", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1 },
                ])
            );
            let scripts = script();
            assert!(scripts.base.preview.is_some());
            assert!(scripts.radiant.preview.is_some());
            assert!(scripts.base.targets.is_empty());
        }

        mod base {
            use super::*;

            #[test]
            fn one_hit_of_n_on_the_enemy_hero_n_the_size_of_their_exile_as_it_resolves() {
                crate::register_all();
                let mut s = curse(false, CurseOpts::default());
                s.play(CURSE, json!({}));
                assert_eq!(hits_on(&s, P2), vec![3]);
                s.expect_health(P2, 27);
            }

            #[test]
            fn r63_one_hit_so_armor_applies_once() {
                crate::register_all();
                let mut s = curse(
                    false,
                    CurseOpts {
                        armor: Some(2),
                        their_exile: Some(vec![STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE]),
                        ..CurseOpts::default()
                    },
                );
                s.play(CURSE, json!({}));
                s.expect_health(P2, 28);
                assert_eq!(damage_events(&s).len(), 1);
            }

            #[test]
            fn r63_an_empty_exile_deals_no_hit_and_the_draw_still_happens() {
                crate::register_all();
                let mut s = curse(false, CurseOpts { their_exile: Some(vec![]), ..CurseOpts::default() });
                s.play(CURSE, json!({}));
                assert_eq!(damage_events(&s), Vec::<Value>::new());
                s.expect_health(P2, 30);
                assert_eq!(hand_defs(&s, P1), vec![STOCKPILE, VANILLA]);
            }

            #[test]
            fn then_draws_1() {
                crate::register_all();
                let mut s = curse(false, CurseOpts::default());
                s.play(CURSE, json!({}));
                assert_eq!(hand_defs(&s, P1), vec![STOCKPILE, VANILLA]);
                s.expect_events(json!(["damage", "drawn"]));
            }

            #[test]
            fn only_their_exile_counts_not_yours() {
                crate::register_all();
                let mut s = curse(
                    false,
                    CurseOpts {
                        their_exile: Some(vec![STOCKPILE]),
                        my_exile: vec![STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE],
                        ..CurseOpts::default()
                    },
                );
                s.play(CURSE, json!({}));
                assert_eq!(hits_on(&s, P2), vec![1]);
            }

            #[test]
            fn the_base_face_recruits_nothing() {
                crate::register_all();
                let mut s = curse(false, CurseOpts { their_exile: Some(vec![VANILLA]), ..CurseOpts::default() });
                let vanilla = their_exiled(&s, VANILLA);
                s.play(CURSE, json!({}));
                assert!(s.unit(P1, 1).is_none());
                s.expect_in_zone(&vanilla, "exile");
            }

            #[test]
            fn r386_an_upgrade_of_damage_per_card_deals_2_for_each() {
                crate::register_all();
                let mut s = curse(false, CurseOpts::default());
                step_param(s.card_mut(CURSE), "damage", 1);
                s.play(CURSE, json!({}));
                assert_eq!(hits_on(&s, P2), vec![6]);
            }

            #[test]
            fn r386_an_upgrade_of_draw_draws_2() {
                crate::register_all();
                let mut s = curse(false, CurseOpts::default());
                step_param(s.card_mut(CURSE), "draw", 1);
                s.play(CURSE, json!({}));
                assert_eq!(hand_defs(&s, P1), vec![STOCKPILE, VANILLA, MENACE]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn keeps_the_same_hit_and_the_draw() {
                crate::register_all();
                let mut s = curse(true, CurseOpts::default());
                s.play(CURSE, json!({}));
                assert_eq!(hits_on(&s, P2), vec![3]);
                assert_eq!(hand_defs(&s, P1), vec![STOCKPILE, VANILLA]);
            }

            #[test]
            fn e25_recruits_the_most_recently_exiled_permanent_of_theirs_under_your_control_its_owner_unchanged() {
                crate::register_all();
                let mut s = curse(
                    true,
                    CurseOpts { their_exile: Some(vec![MENACE, VANILLA, STOCKPILE]), ..CurseOpts::default() },
                );
                let menace = their_exiled(&s, MENACE);
                let vanilla = their_exiled(&s, VANILLA);
                s.play(CURSE, json!({}));
                assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(vanilla.id.clone()));
                let recruited = s.card(&vanilla).clone();
                assert_eq!(recruited.controller, P1);
                assert_eq!(recruited.owner, P2);
                s.expect_in_zone(&menace, "exile");
            }

            #[test]
            fn r53_a_recruited_unit_attacks_the_enemy_hero_at_once_summoning_sickness_ignored() {
                crate::register_all();
                let mut s = curse(
                    true,
                    CurseOpts { their_exile: Some(vec![MENACE, VANILLA, STOCKPILE]), ..CurseOpts::default() },
                );
                s.play(CURSE, json!({}));
                // 3 for the exile of three, then the Vanilla's 4.
                assert_eq!(hits_on(&s, P2), vec![3, 4]);
                s.expect_health(P2, 23);
                s.expect_events(json!(["damage", "drawn", "summoned", "attackDeclared"]));
            }

            #[test]
            fn r53_the_forced_attack_skips_taunt_it_hits_the_hero_past_their_taunt_unit() {
                crate::register_all();
                let mut s = curse(
                    true,
                    CurseOpts {
                        their_exile: Some(vec![VANILLA]),
                        their_field: vec![MENACE],
                        ..CurseOpts::default()
                    },
                );
                s.play(CURSE, json!({}));
                assert_eq!(hits_on(&s, P2), vec![1, 4]);
            }

            #[test]
            fn r1_a_recruit_fires_no_cry() {
                crate::register_all();
                let mut s = curse(true, CurseOpts { their_exile: Some(vec![MR_TOKEN]), ..CurseOpts::default() });
                s.play(CURSE, json!({}));
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(MR_TOKEN.to_string()));
                assert!(s.unit(P1, 2).is_none());
                assert!(!s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::Summoned { def_id, .. } if def_id == RUSH_TOKEN
                )));
            }

            #[test]
            fn s3_2_when_it_leaves_the_field_it_goes_to_its_owner_s_piles() {
                crate::register_all();
                let mut s = curse(true, CurseOpts { their_exile: Some(vec![VANILLA]), ..CurseOpts::default() });
                let vanilla = their_exiled(&s, VANILLA);
                s.play(CURSE, json!({}));
                s.end_turn();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla.id }] }));
                s.expect_in_zone(&vanilla, "graveyard");
                let theirs: Vec<String> = s.pile(P2, "graveyard").into_iter().map(|card| card.id).collect();
                assert!(theirs.contains(&vanilla.id));
                let mine: Vec<String> = s.pile(P1, "graveyard").into_iter().map(|card| card.id).collect();
                assert!(!mine.contains(&vanilla.id));
            }

            #[test]
            fn no_permanent_in_their_exile_nothing_is_recruited() {
                crate::register_all();
                let mut s = curse(true, CurseOpts { their_exile: Some(vec![STOCKPILE, STOCKPILE]), ..CurseOpts::default() });
                s.play(CURSE, json!({}));
                assert!(s.unit(P1, 1).is_none());
                assert!(s.backrow(P1, 1).is_none());
                assert_eq!(hits_on(&s, P2), vec![2]);
            }

            #[test]
            fn no_open_zone_nothing_is_recruited_and_nothing_attacks() {
                crate::register_all();
                let mut s = curse(
                    true,
                    CurseOpts {
                        their_exile: Some(vec![VANILLA]),
                        my_field: vec![MENACE, MENACE, MENACE, MENACE, MENACE],
                        ..CurseOpts::default()
                    },
                );
                let vanilla = their_exiled(&s, VANILLA);
                s.play(CURSE, json!({}));
                s.expect_in_zone(&vanilla, "exile");
                assert_eq!(hits_on(&s, P2), vec![1]);
            }

            #[test]
            fn r33_a_recruited_trap_is_set_face_down_and_read_by_you_alone_it_attacks_nothing() {
                crate::register_all();
                let mut s = curse(true, CurseOpts { their_exile: Some(vec![VANILLA, SHEEPISH]), ..CurseOpts::default() });
                s.play(CURSE, json!({}));
                let trap = s.backrow(P1, 1);
                assert_eq!(trap.as_ref().map(|card| card.def_id.as_str()), Some(SHEEPISH));
                assert_ne!(trap.as_ref().and_then(|card| card.face_up), Some(true));
                assert_eq!(trap.as_ref().map(|card| card.owner), Some(P2));
                assert!(s.unit(P1, 1).is_none());
                assert_eq!(js(&s.view(P2))["opponent"]["backrow"][0], json!({ "faceDown": true, "cost": 1 }));
                assert!(matches_object(&js(&s.view(P1))["you"]["backrow"][0], &json!({ "defId": SHEEPISH })));
                assert_eq!(hits_on(&s, P2), vec![2]);
                // Its owner's event stream names it nowhere once it is set (R97: judged where it is now).
                let theirs = serde_json::to_string(&s.view(P2)).expect("a view serialises");
                let trap_id = trap.as_ref().map(|card| card.id.clone()).unwrap_or_else(|| "missing".to_string());
                assert!(!theirs.contains(&format!("\"{trap_id}\"")));
                assert!(!theirs.contains(SHEEPISH));
            }

            #[test]
            fn the_newest_permanent_with_no_open_zone_for_it_nothing_is_recruited_not_an_older_one() {
                crate::register_all();
                // The Trap is the newest permanent and your backrow is full, so nothing comes, though the older
                // Vanilla would fit a unit zone; and nothing attacks.
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [{ "def": CURSE, "radiant": true }, STOCKPILE],
                        "library": [VANILLA],
                        "backrow": [MANA_WELL, MANA_WELL, MANA_WELL, MANA_WELL, MANA_WELL],
                    },
                    "p2": { "hand": [STOCKPILE], "exile": [VANILLA, SHEEPISH], "library": [STOCKPILE] },
                }));
                s.play(CURSE, json!({}));
                let units: Vec<Option<CardInstance>> = (1..=5).map(|lane| s.unit(P1, lane)).collect();
                assert_eq!(units, vec![None, None, None, None, None]);
                let exiled: Vec<String> = s.pile(P2, "exile").into_iter().map(|card| card.def_id).collect();
                assert_eq!(exiled, vec![VANILLA, SHEEPISH]);
                assert_eq!(hits_on(&s, P2), vec![2]);
            }

            #[test]
            fn a_recruited_field_spell_comes_face_up_to_your_backrow() {
                crate::register_all();
                let mut s = curse(true, CurseOpts { their_exile: Some(vec![MANA_WELL]), ..CurseOpts::default() });
                s.play(CURSE, json!({}));
                assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some(MANA_WELL.to_string()));
                assert!(matches_object(
                    &js(&s.view(P2))["opponent"]["backrow"][0],
                    &json!({ "defId": MANA_WELL })
                ));
            }

            #[test]
            fn r386_an_upgrade_of_damage_per_card_deals_2_for_each_and_the_recruit_still_happens() {
                crate::register_all();
                let mut s = curse(true, CurseOpts { their_exile: Some(vec![VANILLA, STOCKPILE]), ..CurseOpts::default() });
                step_param(s.card_mut(CURSE), "damage", 1);
                s.play(CURSE, json!({}));
                assert_eq!(hits_on(&s, P2), vec![4, 4]);
            }
        }
    }
}
