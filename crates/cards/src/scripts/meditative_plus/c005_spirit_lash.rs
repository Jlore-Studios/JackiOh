//! M+ #5 Spirit Lash (SPEC §8.9 row 5, BUILD M11 row M+ 5): (0, Embiggened 1) Spell, Rare.
//!   Base:    "Deal {damage} damage to all Units. Embiggened (1): With Lifesteal and Pierce." (1)
//!   Radiant: "Echo 1" plus the same: it resolves a second time.
//!
//! One `damage_all` over every Unit on both sides ("any", no hero), one hit each, all landing
//! before the state check (R59, R1179000). Played Embiggened, each hit carries Lifesteal (R85)
//! and Pierce (R346), stated on the effect, so the card prints neither keyword. The Radiant face
//! prints Echo 1 (`StaticFlags.echo`, §6.2): the repeat is Embiggened if the play was (the
//! instance keeps the flag, R81). The damage is declared and read through `param` (R386).

use jackioh_engine::effects::damage_all;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-005";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            let embiggened = ctx.embiggened;
            vec![damage_all(json_as(json!({
                "amount": param(&*ctx, "damage"),
                "side": "any",
                "lifesteal": embiggened,
                "ignoreArmor": embiggened,
            })))]
        })),
        ..Script::default()
    };

    // The Radiant face adds Echo 1 and keeps the base script, so the repeat is Embiggened when
    // the play was (C+ #36.1's shape).
    let radiant = Script {
        static_flags: Some(StaticFlags {
            echo: Some(1),
            ..StaticFlags::default()
        }),
        ..base.clone()
    };

    CardScripts { base, radiant }
}

// M+ #5 Spirit Lash — SPEC §8.9 row 5, BUILD M11 row M+ 5: "at (0) one hit of 1 on every Unit of
// both sides, a Divine Shield popped, no hero hit; Embiggened at (1) each hit ignores Armor and
// heals your hero by the damage dealt; damage reads through `param()`; radiant Echo 1: it resolves
// twice, an Embiggened play twice Embiggened".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const LASH: &str = "meditativeplus-005";
    const ARMORED: &str = "core-025"; // 7/7, Armor 7
    const SHIELDED: &str = "core-056"; // 3/2, Divine Shield
    const BODY: &str = "core-019"; // 9/9
    const FILLER: &str = "core-005";

    use crate::scenario;
    use crate::unit_or_blank;

    fn hits(s: &Scenario) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { amount, .. } => Some(*amount),
                _ => None,
            })
            .collect()
    }

    fn has_keyword(s: &Scenario, card: &str, kind: KeywordKind) -> bool {
        s.stats(card).keywords.iter().any(|keyword| keyword.kind() == kind)
    }

    #[test]
    fn r1179000_one_hit_on_every_unit_of_both_sides_and_none_on_a_hero() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [LASH, FILLER], "field": [{ "def": BODY, "lane": 2 }] },
            "p2": { "hand": [FILLER], "field": [{ "def": BODY, "lane": 1 }, { "def": BODY, "lane": 3 }] },
        }));
        s.play(LASH, json!({}));
        let own = unit_or_blank(&s, P1, 2);
        s.expect_stats(&own, json!({ "health": 8 }));
        for lane in [1, 3] {
            let enemy = unit_or_blank(&s, P2, lane);
            s.expect_stats(&enemy, json!({ "health": 8 }));
        }
        assert_eq!(hits(&s), vec![1, 1, 1]);
        s.expect_health(P1, 30).expect_health(P2, 30);
    }

    #[test]
    fn a_divine_shield_is_popped() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [LASH, FILLER] },
            "p2": { "hand": [FILLER], "field": [SHIELDED] },
        }));
        s.play(LASH, json!({}));
        let shielded = s.unit(P2, 1).expect("gone");
        s.expect_stats(&shielded, json!({ "health": 2 }));
        assert!(!has_keyword(&s, &shielded.id, KeywordKind::DivineShield));
    }

    #[test]
    fn the_normal_version_does_not_pierce_armor() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [LASH, FILLER] },
            "p2": { "hand": [FILLER], "field": [ARMORED] },
        }));
        s.play(LASH, json!({}));
        let armored = unit_or_blank(&s, P2, 1);
        s.expect_stats(&armored, json!({ "health": 7 }));
    }

    #[test]
    fn r1179000_embiggened_each_hit_ignores_armor_and_heals_your_hero_by_the_damage_dealt() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "health": 20, "hand": [LASH, FILLER], "field": [BODY] },
            "p2": { "hand": [FILLER], "field": [ARMORED] },
        }));
        s.play(LASH, json!({ "embiggen": true }));
        s.expect_mana(P1, 3);
        // Two hits of 1, both through Armor: the 7/7 at 6, your own 9/9 at 8.
        let armored = unit_or_blank(&s, P2, 1);
        s.expect_stats(&armored, json!({ "health": 6 }));
        let own = unit_or_blank(&s, P1, 1);
        s.expect_stats(&own, json!({ "health": 8 }));
        assert_eq!(hits(&s), vec![1, 1]);
        // Lifesteal heals your hero by the damage dealt, once per hit.
        s.expect_health(P1, 22);
    }

    #[test]
    fn r386_damage_reads_through_param() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [LASH, FILLER] },
            "p2": { "hand": [FILLER], "field": [BODY] },
        }));
        step_param(s.card_mut(LASH), "damage", 1);
        s.play(LASH, json!({}));
        let body = unit_or_blank(&s, P2, 1);
        s.expect_stats(&body, json!({ "health": 7 }));
    }

    #[test]
    fn radiant_echo_1_hits_every_unit_twice() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": LASH, "radiant": true }, FILLER], "field": [BODY] },
            "p2": { "hand": [FILLER], "field": [BODY] },
        }));
        s.play(LASH, json!({}));
        let own = unit_or_blank(&s, P1, 1);
        s.expect_stats(&own, json!({ "health": 7 }));
        let enemy = unit_or_blank(&s, P2, 1);
        s.expect_stats(&enemy, json!({ "health": 7 }));
        assert_eq!(hits(&s), vec![1, 1, 1, 1]);
        s.expect_health(P1, 30).expect_health(P2, 30);
    }

    #[test]
    fn r1179000_radiant_an_embiggened_play_repeats_embiggened() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "health": 20, "hand": [{ "def": LASH, "radiant": true }, FILLER], "field": [BODY] },
            "p2": { "hand": [FILLER], "field": [ARMORED] },
        }));
        s.play(LASH, json!({ "embiggen": true }));
        // Both resolutions pierce: the 7/7 takes 1 twice.
        let armored = unit_or_blank(&s, P2, 1);
        s.expect_stats(&armored, json!({ "health": 5 }));
        assert_eq!(hits(&s), vec![1, 1, 1, 1]);
        // Four hits of 1 with Lifesteal.
        s.expect_health(P1, 24);
    }
}
