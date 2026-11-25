//! SPEC §8.13 IA #58 Overclocked Core ((2) Unit, Rare, 1/5 → 2/10), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Aura: Your Spells cost ({discount}) less. End of turn: Deal {damage} damage to your hero." (discount 1, damage 1)
//!   Radiant: "Aura: Your Spells cost ({discount}) less and your Units have +{attack} Attack. End of turn: Deal {damage} damage to your hero." (discount 1, attack 1, damage 1)
//!
//! The cost aura is IA #18 Prompt Engineer's shape narrowed to Spells (C #77 Anti-Magic Monkey's
//! `costAura`, R4577003's floor left off: no floor is printed). The Radiant face adds IA #22
//! Assault Mainframe's aura over its controller's Units on the field, itself included (as IA #18's
//! Radiant face includes itself). Its end of turn deals the declared `damage` to its controller's
//! hero (§2.2, R68).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-058";

fn spell_discount() -> CostAuraHook {
    read_hook(|args| {
        vec![CostAura {
            rule: CostRule {
                amount: Some(-param(&args, "discount")),
                types: Some(vec![CardType::Spell]),
                ..CostRule::default()
            },
            whose: CostAuraWhose::Yours,
            ban: None,
        }]
    })
}

/// "your Units have +N Attack": its controller's Units on the field, itself included.
fn units_attack() -> AuraHook {
    aura_hook(|ctx| {
        let amount = param(&ctx, "attack");
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == ctx.self_.controller && in_units_row_or_standing(ctx.state, unit)
            }),
            mod_: StatMod {
                attack: Some(amount),
                ..StatMod::default()
            },
        }]
    })
}

fn core(radiant: bool) -> Script {
    Script {
        cost_aura: Some(spell_discount()),
        aura: radiant.then(units_attack),
        end_of_turn: Some(hook(|ctx| {
            vec![damage(json_as(
                json!({ "to": { "of": "selfHero" }, "amount": param(&*ctx, "damage") }),
            ))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: core(false),
        radiant: core(true),
    }
}

// IA #58 Overclocked Core — SPEC §8.13 row 58.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SPELL: &str = "stand-in (2) Spell";
    const UNIT: &str = "stand-in (1) 2/2";

    mod base {
        use super::*;

        #[test]
        fn your_spells_cost_1_less() {
            let mut s = scenario(json!({
                "p1": { "hand": [SPELL], "field": [ID] },
            }));
            s.play(SPELL, json!({}));
            // The (2) Spell paid (1).
            s.expect_mana(P1, 3);
        }

        #[test]
        fn your_units_get_no_attack_and_its_end_of_turn_deals_1_to_your_hero() {
            let mut s = scenario(json!({
                "p1": { "hand": [SPELL], "field": [ID, UNIT], "library": crate::filler(2) },
                "p2": { "hand": [SPELL], "library": crate::filler(2) },
            }));
            let (core, unit) = (s.unit_at(P1, 1), s.unit_at(P1, 2));
            s.expect_stats(&core, json!({ "attack": 1, "health": 5 }));
            s.expect_stats(&unit, json!({ "attack": 2, "health": 2 }));
            s.end_turn();
            s.expect_health(P1, HERO_HEALTH - 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn your_spells_still_cost_1_less() {
            let mut s = scenario(json!({
                "p1": { "hand": [SPELL], "field": [{ "def": ID, "radiant": true }] },
            }));
            s.play(SPELL, json!({}));
            s.expect_mana(P1, 3);
        }

        #[test]
        fn your_units_have_1_attack_itself_included_and_its_end_of_turn_deals_1_to_your_hero() {
            let mut s = scenario(json!({
                "p1": { "hand": [SPELL], "field": [{ "def": ID, "radiant": true }, UNIT], "library": crate::filler(2) },
                "p2": { "hand": [SPELL], "library": crate::filler(2) },
            }));
            let (core, unit) = (s.unit_at(P1, 1), s.unit_at(P1, 2));
            s.expect_stats(&core, json!({ "attack": 3, "health": 10 }));
            s.expect_stats(&unit, json!({ "attack": 3, "health": 2 }));
            s.end_turn();
            s.expect_health(P1, HERO_HEALTH - 1);
        }
    }
}
