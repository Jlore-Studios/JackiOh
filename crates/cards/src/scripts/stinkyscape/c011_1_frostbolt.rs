//! S #11.1 Frostbolt (SPEC §8.10 row S 11.1; §3.1, §4.4, R81, R1867004): (1) Spell,
//! Other-Origins, Token (printed Epic).
//!   Base:    "Deal {damage} damage and Freeze it." — 3.
//!   Radiant: "Deal {damage} damage and Freeze it. Deal {damage} damage to the Units adjacent to it."
//!            — 5.
//!
//! Engine: a declared target, any Unit or hero on either side (§8 Conventions, R81); one hit of the
//! declared damage (§4.4: Armor, Divine Shield and Spell Damage all apply), then `freeze` on the
//! target — a hero passed by, since heroes are never Frozen (§6.1), and a Unit the hit destroyed
//! passed by with it. The Radiant face also hits each Unit beside the target in its row on its side
//! (Cleave's adjacency, §3.1; none beside a hero) with its own instance of the declared damage,
//! all before one state check (R59); the neighbours are never Frozen (R1867004).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-011-1";

/// §8 Conventions: any Unit or hero, either side.
fn targets() -> Vec<TargetDecl> {
    vec![json_as(json!({
        "kind": "target",
        "min": 1,
        "max": 1,
        "filter": { "side": "any", "of": ["unit", "hero"] },
    }))]
}

/// The Units beside the chosen target, in its row on its side (§3.1); none beside a hero.
fn neighbours(ctx: &mut EffectContext<'_>) -> Vec<String> {
    adjacent_to(ctx, &json_as(json!({ "of": "chosen" })), &BoardScope::default())
        .into_iter()
        .map(|card| card.id)
        .collect()
}

fn frostbolt(radiant: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let amount = param(&*ctx, "damage");
            let mut effects = vec![
                damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount }))),
                freeze(json_as(json!({ "target": { "of": "chosen" } }))),
            ];
            if radiant {
                effects.push(for_each_card(ForEachCardArgs {
                    cards: Arc::new(neighbours),
                    each: Arc::new(move |instance_id: &str| {
                        damage(json_as(json!({
                            "to": { "of": "instance", "instanceId": instance_id },
                            "amount": amount,
                        })))
                    }),
                }));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: frostbolt(false),
        radiant: frostbolt(true),
    }
}

// S #11.1 Frostbolt — SPEC §8.10 row S 11.1, BUILD M12 row S 11.1 (R1867004).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P2: PlayerId = PlayerId::P2;

    const FROSTBOLT: &str = "stinkyscape-011-1";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9.
    const FILLER: &str = "core-005";

    fn frozen(s: &Scenario, card: &CardInstance) -> bool {
        s.card(card).frozen.is_some()
    }

    mod base {
        use super::*;

        #[test]
        fn deals_3_to_a_unit_which_is_frozen_if_it_lives() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FROSTBOLT, FILLER] },
                "p2": { "hand": [FILLER], "field": [MENACE] },
            }));
            let unit = s.unit_at(P2, 1);

            s.play(FROSTBOLT, json!({ "targets": pick::instance(&unit) }));

            s.expect_stats(&unit, json!({ "health": 6 }));
            assert!(frozen(&s, &unit));
        }

        #[test]
        fn deals_3_to_a_hero_and_nothing_is_frozen() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FROSTBOLT, FILLER] },
                "p2": { "hand": [FILLER] },
            }));

            s.play(FROSTBOLT, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            s.expect_health(P2, HERO_HEALTH - 3);
        }

        #[test]
        fn a_unit_the_hit_destroyed_is_not_frozen() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FROSTBOLT, FILLER] },
                "p2": { "hand": [FILLER], "field": [{ "def": VANILLA, "damage": 3 }] },
            }));
            let unit = s.unit_at(P2, 1);

            s.play(FROSTBOLT, json!({ "targets": pick::instance(&unit) }));

            s.expect_in_zone(&unit, "graveyard");
        }

        #[test]
        fn r386_an_upgrade_makes_it_deal_4() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FROSTBOLT, FILLER] },
                "p2": { "hand": [FILLER], "field": [MENACE] },
            }));
            step_param(s.card_mut(FROSTBOLT), "damage", 1);
            let unit = s.unit_at(P2, 1);

            s.play(FROSTBOLT, json!({ "targets": pick::instance(&unit) }));

            s.expect_stats(&unit, json!({ "health": 5 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1867004_deals_5_to_the_target_and_to_each_unit_beside_it_the_neighbours_not_frozen() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": FROSTBOLT, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER], "field": [MENACE, MENACE, MENACE, MENACE] },
            }));
            let left = s.unit_at(P2, 1);
            let target = s.unit_at(P2, 2);
            let right = s.unit_at(P2, 3);
            let far = s.unit_at(P2, 4);

            s.play(FROSTBOLT, json!({ "targets": pick::instance(&target) }));

            for unit in [&target, &left, &right] {
                s.expect_stats(unit, json!({ "health": 4 }));
            }
            s.expect_stats(&far, json!({ "health": 9 }));
            assert!(frozen(&s, &target));
            assert!(!frozen(&s, &left));
            assert!(!frozen(&s, &right));
        }
    }
}
