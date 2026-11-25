//! SPEC §8.13 IA #129 Signal Jammer ((2) Unit, Common, 2/5 → 4/10), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Aura: Enemy Spells cost ({surcharge}) more."
//!   Radiant: "Aura: Enemy Spells cost ({surcharge}) more."
//!
//! C #77 Anti-Magic Monkey's price rule laid on the opponent's Spells only (B5 E15, §6.3 Cost, R65):
//! a Spell the opponent plays from anywhere a play takes it pays the declared `surcharge` more; your
//! own Spells and enemy Units pay nothing. Radiant: the same rule, its declared 2.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-129";

fn jammer() -> Script {
    Script {
        cost_aura: Some(read_hook(|args| {
            vec![CostAura {
                rule: CostRule {
                    types: Some(vec![CardType::Spell]),
                    amount: Some(param(&args, "surcharge")),
                    ..CostRule::default()
                },
                whose: CostAuraWhose::Opponents,
                ban: None,
            }]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The Radiant face's 2 is its declared `surcharge`.
    let base = jammer();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #129 Signal Jammer — SPEC §8.13 row 129 (R65).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const JAMMER: &str = ID;
    const SPELL: &str = "stand-in (0) Spell";
    const UNIT: &str = "stand-in (3) 6/6 R12/12";

    fn cost_of(s: &Scenario, card: &CardInstance) -> i32 {
        jackioh_engine::mana::effective_cost(s.state(), card, Default::default())
    }

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "signal-jammer",
            "p1": { "field": [{ "def": JAMMER, "radiant": radiant }] },
            "p2": { "hand": [SPELL], "field": [UNIT], "library": crate::filler(3) },
        }))
    }

    fn enemy_costs(s: &Scenario) -> (i32, i32) {
        let spell = s.card(SPELL).clone();
        let unit = s.card(UNIT).clone();
        (cost_of(s, &spell), cost_of(s, &unit))
    }

    mod base {
        use super::*;

        #[test]
        fn enemy_spells_cost_1_more_enemy_units_are_unchanged() {
            let s = board(false);
            assert_eq!(enemy_costs(&s), (1, 3));
        }

        #[test]
        fn your_own_spells_pay_nothing_more() {
            let s = scenario(json!({
                "seed": "signal-jammer-own",
                "p1": { "field": [{ "def": JAMMER }], "hand": [SPELL] },
                "p2": { "hand": [UNIT] },
            }));
            let spell = s.card(SPELL).clone();
            assert_eq!(cost_of(&s, &spell), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn enemy_spells_cost_2_more_enemy_units_are_unchanged() {
            let s = board(true);
            assert_eq!(enemy_costs(&s), (2, 3));
        }
    }
}
