//! C+ #12.2 Death Boil (SPEC §8.7 row 12.2): (1) Spell, Pancake, Token (printed Legendary).
//!   Both faces: "Choose a Unit or hero. If it's an enemy, deal {amount} damage to it. If it's yours,
//!   heal it {amount}." — 6, Radiant 12.
//! Which clause applies is read at resolution from who controls the target; the damage is one §4.4
//! instance (Spell Damage raises it), the heal a Heal (a hero past 30, a unit up to its max, R19).

use jackioh_engine::damage::DamageTarget;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-012-2";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))]
}

/// Whether the chosen target is the caster's own (TS `boolean | null`: `None` when it is gone).
fn is_yours(ctx: &EffectContext<'_>) -> Option<bool> {
    let target = resolve_target(ctx, &json_as::<TargetSpec>(json!({ "of": "chosen" })))?;
    let controller = match target {
        DamageTarget::Hero { player } => player,
        DamageTarget::Unit { instance } => instance.controller,
    };
    Some(controller == ctx.controller)
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            let amount = param(&*ctx, "amount");
            let Some(yours) = is_yours(ctx) else {
                return vec![];
            };
            vec![if yours {
                heal(json_as(json!({ "target": { "of": "chosen" }, "amount": amount })))
            } else {
                damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))
            }]
        })),
        ..Script::default()
    };
    // The Radiant face is the same text at 12, a catalog value.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C+ #12.2 Death Boil — SPEC §8.7 row 12.2, BUILD M9 Classic+ row C+ 12.2: "A target Unit or hero: an
// enemy takes 6 damage (Spell Damage raises it), one of yours is healed 6 (a hero past 30, a unit up to
// its max health); the amount reads through `param()` (step 2); radiant 12 and 12".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const BOIL: &str = "classicplus-012-2";
    const MENACE: &str = "core-019"; // 9/9
    const SOLARIUS: &str = "classicplus-038"; // Spell Damage +2
    const FILLER: &str = "core-005";

    use crate::merged;

    fn boil(radiant: bool, p1: Value, p2: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": merged(json!({ "hand": [{ "def": BOIL, "radiant": radiant }, FILLER] }), p1),
            "p2": merged(json!({ "hand": [FILLER], "field": [MENACE] }), p2),
        }))
    }

    fn unit_at(s: &Scenario, player: PlayerId) -> Value {
        json!([{ "pick": "instance", "instanceId": s.unit(player, 1).map(|unit| unit.id).unwrap_or_default() }])
    }

    mod base {
        use super::*;

        #[test]
        fn an_enemy_unit_takes_6_damage() {
            crate::register_all();
            let mut s = boil(false, json!({}), json!({}));
            let targets = unit_at(&s, P2);
            s.play(BOIL, json!({ "targets": targets }));
            s.expect_stats(MENACE, json!({ "health": 3 }));
        }

        #[test]
        fn the_enemy_hero_takes_6_spell_damage_raises_it() {
            crate::register_all();
            let mut s = boil(false, json!({ "field": [SOLARIUS] }), json!({}));
            s.play(BOIL, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            s.expect_health(P2, HERO_HEALTH - 8);
        }

        #[test]
        fn your_own_unit_is_healed_6_never_past_its_max_health() {
            crate::register_all();
            let mut s = boil(false, json!({ "field": [{ "def": MENACE, "damage": 8 }] }), json!({ "field": [] }));
            let targets = unit_at(&s, P1);
            s.play(BOIL, json!({ "targets": targets }));
            s.expect_stats(MENACE, json!({ "health": 7, "maxHealth": 9 }));
            let mut full = boil(false, json!({ "field": [{ "def": MENACE, "damage": 2 }] }), json!({ "field": [] }));
            let targets = unit_at(&full, P1);
            full.play(BOIL, json!({ "targets": targets }));
            full.expect_stats(MENACE, json!({ "health": 9 }));
        }

        #[test]
        fn r19_your_hero_is_healed_6_past_30() {
            crate::register_all();
            let mut s = boil(false, json!({}), json!({}));
            s.play(BOIL, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            s.expect_health(P1, HERO_HEALTH + 6);
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Damage));
        }

        #[test]
        fn r386_the_amount_reads_through_param_an_upgrade_steps_it_by_2() {
            crate::register_all();
            let mut s = boil(false, json!({}), json!({}));
            step_param(s.card_mut(BOIL), "amount", 1);
            s.play(BOIL, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            s.expect_health(P2, HERO_HEALTH - 8);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn t_12_damage_to_an_enemy() {
            crate::register_all();
            let mut s = boil(true, json!({}), json!({}));
            s.play(BOIL, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            s.expect_health(P2, HERO_HEALTH - 12);
        }

        #[test]
        fn t_12_healing_to_one_of_yours() {
            crate::register_all();
            let mut s = boil(true, json!({ "health": 10 }), json!({}));
            s.play(BOIL, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            s.expect_health(P1, 22);
        }
    }
}
