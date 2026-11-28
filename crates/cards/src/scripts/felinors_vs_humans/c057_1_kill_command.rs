//! FvH #57.1 Kill Command (SPEC §8.14 row 57.1; §4.4, R81, R923, R1748001): (1) Spell, Other-Origins,
//! Token (printed Epic), a token FvH #57 Otherworldly Hunting adds.
//!   Base:    "Deal {damage} damage, or {felinorDamage} if you control a Felinor." — 3, or 5.
//!   Radiant: "Deal {damage} damage, or {felinorDamage} if you control a Felinor." — 6, or 10.
//!
//! Engine: one declared target, any Unit or hero on either side (§8 Conventions, R81); one hit of the
//! declared `felinorDamage` when its caster controls a Felinor-tagged Unit on the field as it resolves
//! (a granted tag counts, R923; an enemy's Felinor does not), else of the declared `damage`. Hearthstone's
//! Kill Command, its Beast a Felinor. The faces differ only in their declared numbers. Built by S05
//! (#5255).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-057-1";

/// "If you control a Felinor": a Felinor-tagged Unit on the caster's side of the field now (R923). Read
/// off the field itself, not a Spell's scope, which passes a Spell-immune Unit by.
fn controls_a_felinor(ctx: &EffectContext<'_>) -> bool {
    active_units_of(ctx.state, ctx.controller)
        .iter()
        .any(|unit| tags_of(ctx.state, unit).contains(&Tag::Felinor))
}

fn kill_command() -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))],
        cry: Some(hook(|ctx| {
            let amount = if controls_a_felinor(ctx) {
                param(&*ctx, "felinorDamage")
            } else {
                param(&*ctx, "damage")
            };
            vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = kill_command();
    // The same script: the Radiant face's 6 and 10 are its declared `damage` and `felinorDamage`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #57.1 Kill Command — SPEC §8.14 row 57.1 (R81, R923).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A Felinor of the set's own: FvH #57.4 Wide Felinor.
    const FELINOR: &str = "felinorsvshumans-057-4";
    const TARGET: &str = "stand-in (1) 1/20";
    const OTHER: &str = "stand-in (1) 2/2";
    /// A textless (0) Spell: something left to play, so no turn ends by itself.
    const FILLER: &str = "stand-in (0) Spell";

    /// p1 holds Kill Command with `mine` on its field; p2 holds the 1/20 target and `theirs`.
    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        let mut p2_field = vec![json!({ "def": TARGET, "lane": 1 })];
        if let Value::Array(more) = theirs {
            p2_field.extend(more);
        }
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": mine },
            "p2": { "hand": [FILLER], "field": p2_field },
        }))
    }

    /// Kill Command aimed at p2's 1/20 in lane 1; its Health after.
    fn health_after_the_hit(s: &mut Scenario) -> i32 {
        let target = s.unit_at(P2, 1);
        s.play(ID, pick::targets(pick::instance(&target)));
        s.stats(&target).health
    }

    mod base {
        use super::*;

        #[test]
        fn deals_3_with_no_felinor_of_yours() {
            let mut s = board(false, json!([OTHER]), json!([]));
            assert_eq!(health_after_the_hit(&mut s), 20 - 3);
        }

        #[test]
        fn deals_5_while_you_control_a_felinor() {
            let mut s = board(false, json!([OTHER, FELINOR]), json!([]));
            assert_eq!(health_after_the_hit(&mut s), 20 - 5);
        }

        #[test]
        fn an_enemy_felinor_is_none_of_yours() {
            let mut s = board(false, json!([OTHER]), json!([{ "def": FELINOR, "lane": 2 }]));
            assert_eq!(health_after_the_hit(&mut s), 20 - 3);
        }

        #[test]
        fn r923_a_unit_granted_the_felinor_tag_counts() {
            let mut s = board(false, json!([OTHER]), json!([]));
            let other = s.unit_at(P1, 1);
            s.card_mut(&other).granted_tags = Some(vec![Tag::Felinor]);
            assert_eq!(health_after_the_hit(&mut s), 20 - 5);
        }

        #[test]
        fn r81_it_may_hit_a_hero() {
            let mut s = board(false, json!([FELINOR]), json!([]));
            s.play(ID, pick::targets(pick::hero("p2")));
            s.expect_health(P2, HERO_HEALTH - 5);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn deals_6_with_no_felinor_of_yours() {
            let mut s = board(true, json!([OTHER]), json!([]));
            assert_eq!(health_after_the_hit(&mut s), 20 - 6);
        }

        #[test]
        fn deals_10_while_you_control_a_felinor() {
            let mut s = board(true, json!([FELINOR]), json!([]));
            assert_eq!(health_after_the_hit(&mut s), 20 - 10);
        }
    }
}
