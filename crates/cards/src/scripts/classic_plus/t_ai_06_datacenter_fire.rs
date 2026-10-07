//! T-AI-6 Datacenter Fire (SPEC §8.7 row T-AI-6, §7, B8). (2) Spell, AI, Token.
//!   Base:    "Destroy all Field Spells. Deal 1 damage to each hero for each one destroyed."
//!   Radiant: "Destroy all enemy Field Spells. Deal 2 damage to the enemy hero for each one destroyed."
//!   Engine:  "The Field Spells on both sides (Traps and Field Traps are not), an Ivory Tower included
//!            whatever it has fused (R418, R588); Indestructible ones (#98) survive and don't count. The
//!            damage is one instance per hero, 1 (Radiant 2) times the count, through §4.4, so Spell
//!            Damage raises it once (§4.4 step 0); none destroyed, no damage. Its preview (R280) is the
//!            damage each hero would take now. The base face burns your own Claude's Datacenter too
//!            while it stands in its backrow zone; animated, it is a Unit and stays (R588). Tunes: none."
//!
//! The sweep and the count are one engine verb (`destroy_field_spells_and_hit`, effects/datacenter.rs),
//! and the preview reads the same count (`field_spells_doomed`) off the public backrows, times the face's
//! number — the hit before Spell Damage, as every Core preview reads its own number (R280). An Animated
//! Field Spell standing in a unit zone is a Unit there (R383) and no Field Spell of the sweep's (R588).

use jackioh_engine::effects::{FieldSpellSide, SweepReader, destroy_field_spells_and_hit, field_spells_doomed};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-t-ai-06";

/// TS's `{ base, radiant } as const`: one number per face.
#[derive(Clone, Copy)]
struct ByFaceDamage {
    base: i32,
    radiant: i32,
}

/// §8.7: 1 damage per Field Spell on the base face, 2 on the Radiant. An AI card declares no params (B8).
const DAMAGE_PER: ByFaceDamage = ByFaceDamage { base: 1, radiant: 2 };

/// R280: the formula as each face prints it, read off the catalog text so it is always a substring.
///
/// TS matched `/Deal \d+ damage to [^.]+ for each one destroyed/` (no regex crate in a pure crate,
/// SURFACE §8): the leftmost "Deal <digits> damage to ", then the longest run of non-"." characters (one
/// at least) that ends in " for each one destroyed", as the greedy class backtracks to.
fn formula_in(text: &str) -> String {
    const HEAD: &str = "Deal ";
    const MIDDLE: &str = " damage to ";
    const TAIL: &str = " for each one destroyed";
    let mut from = 0;
    while let Some(offset) = text[from..].find(HEAD) {
        let start = from + offset;
        let mut at = start + HEAD.len();
        let digits = text[at..].bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 {
            at += digits;
            if text[at..].starts_with(MIDDLE) {
                let class_start = at + MIDDLE.len();
                let segment_end = text[class_start..].find('.').map_or(text.len(), |dot| class_start + dot);
                if let Some(tail_at) = text[class_start..segment_end].rfind(TAIL).filter(|&tail_at| tail_at >= 1) {
                    return text[start..class_start + tail_at + TAIL.len()].to_string();
                }
            }
        }
        from = start + 1;
    }
    panic!("T-AI-6's text names no damage formula: {text}");
}

/// The face's sweep (`"any"` or `"enemy"`, TS `FieldSpellSide`) and its preview.
fn fire(side: &'static str, damage_per: i32, label: String) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![destroy_field_spells_and_hit(json_as(json!({ "side": side, "damagePer": damage_per })))]
        })),
        preview: Some(condition_hook(move |ctx: ConditionContext<'_>| {
            let typed: FieldSpellSide = json_as(json!(side));
            let doomed = field_spells_doomed(&SweepReader::of_condition(ctx), typed);
            vec![PreviewValue {
                label: label.clone(),
                value: doomed.len() as i32 * damage_per,
                display: None,
                ids: None,
            }]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let def = crate::card_def(ID);
    CardScripts {
        base: fire("any", DAMAGE_PER.base, formula_in(&def.base.text)),
        radiant: fire("enemy", DAMAGE_PER.radiant, formula_in(&def.radiant.text)),
    }
}

// T-AI-6 Datacenter Fire — SPEC §8.7 row T-AI-6, BUILD M9 Classic+ row T-AI-6: "Destroys every Field
// Spell on both sides, yours included (a Claude's Datacenter too); Field Traps and Traps are no Field
// Spells; an Indestructible one (Heroic Power) stays and doesn't count; then each hero takes one hit of
// 1 per Field Spell destroyed, one instance of N as 'for each' in one sentence always is (so Spell Damage
// adds once and Anime Armor caps it at 1); none destroyed, no damage; a destroyed Field Spell that
// prints Death fires it (§4.5); its preview is the damage each hero would take now (R280); radiant only
// enemy Field Spells, and 2 damage to the enemy hero for each".
//
// The preview's own proofs (labels, values against the resolution, what the hook reads) are in
// `../preview.test.ts`. Each hero's hit is one `damage` event of N, so a per-hit cap (C+ #11 Anime Armor)
// caps it once, and an Ivory Tower holding a Unit (R418) is swept: both are proved through fixtures in
// `packages/engine/test/effects-datacenter.test.ts`, since neither card's script (C+ #11, #33) is part of
// this slice.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const FIRE: &str = "classicplus-t-ai-06";
    const DATACENTER: &str = "classicplus-078"; // (2) Field Spell
    const TWINSPELL: &str = "core-079"; // (2) Field Spell
    const FARM: &str = "core-058"; // Rush Token Farm, Field Spell
    const HEROIC_POWER: &str = "core-098"; // Field Spell, Indestructible
    const BEAR: &str = "core-060"; // (1) Trap
    const BREAD: &str = "core-018"; // Field Trap
    const SOLARIUS: &str = "classicplus-038"; // Unit printing Spell Damage +2
    const BAUBLE: &str = "classicplus-061"; // (1) Field Spell: "Death: Add 2 Stockpiles to your hand. Each costs (0)."
    const STOCKPILE: &str = "core-005";
    const VANILLA: &str = "core-008";

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialises")
    }

    /// TS's `{ ...base, ...extra }` on a side setup.
    fn spread(base: Value, extra: &Value) -> Value {
        let mut out = base;
        if let (Some(into), Some(from)) = (out.as_object_mut(), extra.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        out
    }

    fn fire(p1: Value, p2: Value, radiant_face: bool) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": spread(json!({ "hand": [{ "def": FIRE, "radiant": radiant_face }, VANILLA], "library": [VANILLA] }), &p1),
            "p2": spread(json!({ "hand": [VANILLA], "library": [VANILLA] }), &p2),
        }));
        s.play(FIRE, json!({}));
        s
    }

    fn hits_on(s: &Scenario, player: &str) -> Vec<i32> {
        let hero = format!("hero-{player}");
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } if *target_id == hero => Some(*amount),
                _ => None,
            })
            .collect()
    }

    fn count(events: &[GameEvent], kind: &str) -> usize {
        events.iter().filter(|event| js(*event)["type"] == kind).count()
    }

    fn up(def_id: &str, lane: i32) -> Value {
        json!({ "def": def_id, "faceUp": true, "lane": lane })
    }

    fn down(def_id: &str, lane: i32) -> Value {
        json!({ "def": def_id, "faceUp": false, "lane": lane })
    }

    mod t_ai_6_datacenter_fire {
        use super::*;

        #[test]
        fn is_a_2_ai_spell_token_with_a_preview_on_both_faces() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(js(&def.cost), json!(2));
            assert_eq!(def.type_, CardType::Spell);
            assert_eq!(js(&def.tags), json!(["AI", "Token"]));
            let scripts = super::super::script();
            assert!(scripts.base.preview.is_some());
            assert!(scripts.radiant.preview.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn r59_destroys_every_field_spell_on_both_sides_your_own_claudes_datacenter_too() {
                let mut s = fire(
                    json!({ "backrow": [up(DATACENTER, 1)] }),
                    json!({ "backrow": [up(TWINSPELL, 1), up(FARM, 3)] }),
                    false,
                );
                s.expect_in_zone(DATACENTER, "graveyard")
                    .expect_in_zone(TWINSPELL, "graveyard")
                    .expect_in_zone(FARM, "graveyard");
                assert_eq!(count(s.last_events(), "destroyed"), 3);
            }

            #[test]
            fn s4_4_each_hero_takes_one_hit_of_1_per_field_spell_destroyed() {
                let mut s = fire(
                    json!({ "backrow": [up(DATACENTER, 1)] }),
                    json!({ "backrow": [up(TWINSPELL, 1), up(FARM, 3)] }),
                    false,
                );
                assert_eq!(hits_on(&s, "p1"), vec![3]);
                assert_eq!(hits_on(&s, "p2"), vec![3]);
                s.expect_health(P1, 27).expect_health(P2, 27);
            }

            #[test]
            fn traps_and_field_traps_are_no_field_spells_they_stay_and_dont_count() {
                let mut s = fire(
                    json!({ "backrow": [down(BEAR, 2)] }),
                    json!({ "backrow": [down(BREAD, 1), up(TWINSPELL, 2)] }),
                    false,
                );
                s.expect_in_zone(BEAR, "field")
                    .expect_in_zone(BREAD, "field")
                    .expect_in_zone(TWINSPELL, "graveyard");
                assert_eq!(hits_on(&s, "p1"), vec![1]);
            }

            #[test]
            fn r46_an_indestructible_heroic_power_stays_and_doesnt_count() {
                let mut s = fire(json!({ "backrow": [up(HEROIC_POWER, 1)] }), json!({ "backrow": [up(TWINSPELL, 1)] }), false);
                s.expect_in_zone(HEROIC_POWER, "field");
                assert_eq!(hits_on(&s, "p2"), vec![1]);
            }

            #[test]
            fn s4_5_a_destroyed_field_spell_that_prints_death_fires_it_bauble_bubbles_stockpiles_reach_its_owner() {
                let mut s = fire(json!({}), json!({ "backrow": [up(BAUBLE, 1)] }), false);
                s.expect_in_zone(BAUBLE, "graveyard");
                let stockpiles: Vec<CardInstance> =
                    s.hand(P2).into_iter().filter(|card| card.def_id == STOCKPILE).collect();
                assert_eq!(stockpiles.len(), 2);
                assert!(stockpiles.iter().all(|card| card.cost_override == Some(0)));
                assert_eq!(hits_on(&s, "p2"), vec![1]);
            }

            #[test]
            fn r129_none_destroyed_no_damage() {
                let mut s = fire(json!({ "backrow": [up(HEROIC_POWER, 1), down(BEAR, 2)] }), json!({}), false);
                assert_eq!(count(s.last_events(), "damage"), 0);
                assert_eq!(count(s.last_events(), "destroyed"), 0);
                s.expect_health(P1, 30).expect_health(P2, 30);
            }

            #[test]
            fn r588_r383_an_animated_field_spell_standing_in_a_unit_zone_is_a_unit_there_neither_destroyed_nor_counted() {
                // C+ #12.8 Frostspatula, "Animated on your turn": p1's turn starts and it steps into a unit zone.
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [FIRE, VANILLA], "backrow": [up("classicplus-012-8", 1)], "library": [VANILLA] },
                    "p2": { "hand": [VANILLA], "backrow": [up(TWINSPELL, 1)], "library": [VANILLA] },
                }));
                s.end_turn();
                let spatula = s.unit(P1, 1);
                assert_eq!(
                    spatula.as_ref().map(|unit| unit.def_id.clone()),
                    Some("classicplus-012-8".to_string())
                );
                s.play(FIRE, json!({}));
                assert_eq!(s.unit(P1, 1).map(|unit| unit.id), spatula.map(|unit| unit.id));
                s.expect_in_zone(TWINSPELL, "graveyard");
                assert_eq!(hits_on(&s, "p1"), vec![1]);
            }

            #[test]
            fn s4_4_step_0_spell_damage_raises_each_heros_one_hit_once() {
                let s = fire(
                    json!({ "field": [SOLARIUS], "backrow": [up(DATACENTER, 1)] }),
                    json!({ "backrow": [up(TWINSPELL, 1)] }),
                    false,
                );
                assert_eq!(hits_on(&s, "p1"), vec![4]);
                assert_eq!(hits_on(&s, "p2"), vec![4]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn destroys_only_the_enemys_field_spells_yours_stay() {
                let mut s = fire(
                    json!({ "backrow": [up(DATACENTER, 1)] }),
                    json!({ "backrow": [up(TWINSPELL, 1), up(FARM, 2)] }),
                    true,
                );
                s.expect_in_zone(DATACENTER, "field")
                    .expect_in_zone(TWINSPELL, "graveyard")
                    .expect_in_zone(FARM, "graveyard");
            }

            #[test]
            fn s4_4_the_enemy_hero_takes_one_hit_of_2_per_field_spell_destroyed_and_your_hero_none() {
                let mut s = fire(
                    json!({ "backrow": [up(DATACENTER, 1)] }),
                    json!({ "backrow": [up(TWINSPELL, 1), up(FARM, 2)] }),
                    true,
                );
                assert_eq!(hits_on(&s, "p2"), vec![4]);
                assert!(hits_on(&s, "p1").is_empty());
                s.expect_health(P1, 30).expect_health(P2, 26);
            }

            #[test]
            fn r46_r129_an_enemy_heroic_power_stays_none_destroyed_no_damage() {
                let mut s = fire(json!({}), json!({ "backrow": [up(HEROIC_POWER, 1), down(BEAR, 2)] }), true);
                s.expect_in_zone(HEROIC_POWER, "field");
                assert_eq!(count(s.last_events(), "damage"), 0);
            }
        }
    }
}
