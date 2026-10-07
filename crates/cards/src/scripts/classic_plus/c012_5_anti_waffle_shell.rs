//! C+ #12.5 Anti-Waffle Shell (SPEC §8.7 row 12.5): (1) Field Spell, Pancake, Token (printed Legendary).
//!   Both faces: "Cry: Give your Units Divine Shield. Aura: Your Units have +{aura}/+{aura}." — 2, Radiant 4.
//! The Cry grants Divine Shield (a granted keyword, §10.4) to each Unit you control then; the aura covers
//! every Unit you control while this is on the field, later arrivals and a carried Unit included.

use indexmap::IndexSet;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-012-5";

/// `forEachCard`'s `cards`, typed (TS `(ctx) => readonly (CardInstance | string)[]`, ids here).
fn cards_of(f: impl Fn(&mut EffectContext<'_>) -> Vec<String> + Send + Sync + 'static) -> ForEachCardCards {
    Arc::new(f)
}

/// `forEachCard`'s `each`, typed (TS `(instanceId) => Effect`).
fn each_of(f: impl Fn(&str) -> Effect + Send + Sync + 'static) -> ForEachCardEach {
    Arc::new(f)
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| {
            vec![for_each_card(ForEachCardArgs {
                cards: cards_of(|ctx| {
                    active_units_of(ctx.state, ctx.controller).iter().map(|unit| unit.id.clone()).collect()
                }),
                each: each_of(|instance_id| {
                    grant_keyword(json_as(json!({
                        "target": { "of": "instance", "instanceId": instance_id },
                        "keyword": { "kind": "Divine Shield" },
                    })))
                }),
            })]
        })),
        aura: Some(aura_hook(|args| {
            let n = param(&args, "aura");
            let yours: IndexSet<String> =
                active_units_of(args.state, args.self_.controller).iter().map(|unit| unit.id.clone()).collect();
            vec![AuraEntry {
                applies: Box::new(move |unit: &CardInstance| yours.contains(&unit.id)),
                mod_: StatMod { attack: Some(n), max_health: Some(n), ..StatMod::default() },
            }]
        })),
        ..Script::default()
    };
    // The Radiant face is the same text at +4/+4, a catalog value.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C+ #12.5 Anti-Waffle Shell — SPEC §8.7 row 12.5, BUILD M9 Classic+ row C+ 12.5: "Field Spell: its Cry
// gives every Unit you control Divine Shield (later Units get none); Aura: your Units have +2/+2 while
// it is on the field, later ones included, gone when it leaves; the aura reads through `param()`;
// radiant +4/+4".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const SHELL: &str = "classicplus-012-5";
    const MENACE: &str = "core-019"; // (3) 9/9
    const POINTMASTER: &str = "core-020"; // (2) 7/1
    const MAGIC_JAMMED: &str = "core-036"; // destroy target backrow card
    const FILLER: &str = "core-005";

    fn shell(radiant: bool) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": SHELL, "radiant": radiant }, POINTMASTER, MAGIC_JAMMED, FILLER],
                "field": [MENACE],
                "mana": 8,
            },
            "p2": { "hand": [FILLER], "field": [MENACE] },
        }));
        s.play(SHELL, json!({}));
        s
    }

    fn has_shield(s: &Scenario, card: &str) -> bool {
        s.stats(card).keywords.iter().any(|k| k.kind() == KeywordKind::DivineShield)
    }

    /// TS `s.unit(player, 1)?.id ?? ""`.
    fn lane_1(s: &Scenario, player: PlayerId) -> String {
        s.unit(player, 1).map(|unit| unit.id).unwrap_or_default()
    }

    /// TS `toMatchObject({ attack, maxHealth })` on `s.stats(…)`.
    fn attack_and_max(s: &Scenario, card: &str) -> (i32, i32) {
        let view = s.stats(card);
        (view.attack, view.max_health)
    }

    mod base {
        use super::*;

        #[test]
        fn its_cry_gives_each_unit_you_control_divine_shield_the_opponent_s_get_none() {
            crate::register_all();
            let s = shell(false);
            assert!(has_shield(&s, &lane_1(&s, P1)));
            assert!(!has_shield(&s, &lane_1(&s, P2)));
        }

        #[test]
        fn aura_your_units_have_2_2_the_opponent_s_don_t() {
            crate::register_all();
            let s = shell(false);
            assert_eq!(attack_and_max(&s, &lane_1(&s, P1)), (11, 11));
            assert_eq!(attack_and_max(&s, &lane_1(&s, P2)), (9, 9));
        }

        #[test]
        fn a_later_unit_gets_the_aura_but_no_divine_shield() {
            crate::register_all();
            let mut s = shell(false);
            s.play(POINTMASTER, json!({}));
            s.expect_stats(POINTMASTER, json!({ "attack": 9, "health": 3 }));
            assert!(!has_shield(&s, POINTMASTER));
        }

        #[test]
        fn the_aura_is_gone_when_it_leaves_the_granted_divine_shield_stays() {
            crate::register_all();
            let mut s = shell(false);
            let shell_id = s.card(SHELL).id.clone();
            s.play(MAGIC_JAMMED, json!({ "targets": [{ "pick": "instance", "instanceId": shell_id }] }));
            s.expect_in_zone(SHELL, "graveyard");
            assert_eq!(attack_and_max(&s, &lane_1(&s, P1)), (9, 9));
            assert!(has_shield(&s, &lane_1(&s, P1)));
        }

        #[test]
        fn r386_the_aura_reads_through_param_an_upgrade_makes_it_3_3() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [SHELL, FILLER], "field": [MENACE] }, "p2": { "hand": [FILLER] } }));
            step_param(s.card_mut(SHELL), "aura", 1);
            s.play(SHELL, json!({}));
            s.expect_stats(MENACE, json!({ "attack": 12, "maxHealth": 12 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn t_4_4_and_the_cry_s_divine_shield() {
            crate::register_all();
            let s = shell(true);
            assert_eq!(attack_and_max(&s, &lane_1(&s, P1)), (13, 13));
            assert!(has_shield(&s, &lane_1(&s, P1)));
        }
    }
}
