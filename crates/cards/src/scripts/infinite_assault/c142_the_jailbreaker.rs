//! SPEC §8.13 IA #142 The Jailbreaker ((3) Unit, Legendary, 5/7 → 10/14), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Aura: Your opponent's cards cost (1) more, and their Spells have their targets chosen randomly."
//!            …
//!   Radiant: "Aura: Your opponent's cards cost (1) more, and their Spells have their targets chosen randomly."
//!            "Cry: Nerf every card in your opponent's hand."
//!
//! One acting card, two rules on the opponent (R4590001): #129 Signal Jammer's price rule without a
//! type filter, and the random-targets static flag #130 Prompt Injection spends one modifier for.
//! The base face already carries both; the Radiant face also Nerfs the enemy hand (§6.3 Nerf).

use jackioh_engine::effects::degrade;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-142";

/// "Your opponent's cards cost (1) more" on both faces.
const HIKE: i32 = 1;

fn jailbreaker(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            random_opponent_spell_targets: Some(true),
            ..StaticFlags::default()
        }),
        cost_aura: Some(read_hook(|_args| {
            vec![CostAura {
                rule: CostRule {
                    amount: Some(HIKE),
                    ..CostRule::default()
                },
                whose: CostAuraWhose::Opponents,
                ban: None,
            }]
        })),
        cry: if radiant {
            Some(hook(|_ctx| {
                vec![degrade(json_as(json!({
                    "scope": { "side": "enemy", "zones": ["hand"] },
                })))]
            }))
        } else {
            None
        },
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: jailbreaker(false),
        radiant: jailbreaker(true),
    }
}

// IA #142 The Jailbreaker — SPEC §8.13 row 142 (R4590001).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const JAILBREAKER: &str = ID;
    const BURN: &str = "classic-036"; // Burn: deal damage to any unit or hero
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    fn cost_of(s: &Scenario, card: &CardInstance) -> i32 {
        jackioh_engine::mana::effective_cost(s.state(), card, Default::default())
    }

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "the-jailbreaker",
            "p1": { "field": [{ "def": JAILBREAKER, "radiant": radiant }] },
            "p2": { "hand": [BURN, VANILLA], "mana": 10, "library": crate::filler(3) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r4590001_opponent_cards_cost_1_more_and_their_spells_choose_at_random() {
            let s = board(false);
            let burn = s.card(BURN).clone();
            let unit = s.card(VANILLA).clone();
            assert_eq!(cost_of(&s, &burn), 1, "Burn costs 1 under the hike");
            assert_eq!(cost_of(&s, &unit), 2, "Units cost 1 more too");
            let plays = s.plays_of(BURN);
            assert_eq!(plays.len(), 1, "one play, not one per target");
            assert!(
                matches!(plays[0], ActionBody::Play { targets: None, .. }),
                "no targets while the aura acts"
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4590001_the_radiant_aura_hikes_and_draws_and_its_cry_nerfs_the_hand() {
            let mut s = scenario(json!({
                "seed": "the-jailbreaker-radiant",
                "p1": { "hand": [{ "def": JAILBREAKER, "radiant": true }], "mana": 10 },
                "p2": { "hand": [BURN, VANILLA], "mana": 10, "library": crate::filler(3) },
            }));
            s.play(ID, json!({ "zone": 1 }));
            let burn = s.card(BURN).clone();
            assert_eq!(cost_of(&s, &burn), 1, "the hike holds");
            let plays = s.plays_of(BURN);
            assert_eq!(plays.len(), 1, "targets drawn at random");
            let nerfs = s
                .events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Degraded { .. }))
                .count();
            assert_eq!(nerfs, 2, "the Cry Nerfed every card of the hand");
        }
    }
}
