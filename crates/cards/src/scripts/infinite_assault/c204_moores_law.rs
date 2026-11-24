//! IA #204 Moore's Law (SPEC §8.13 row 204), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "The Units in your hand cost ({discount}) less."
//!   Radiant: "The cards in your hand cost ({discount}) less."
//!
//! Engine: a Cry (R1): `for_each_card` over the hand — Units only on the base face, every card
//! on the Radiant face — each taking a lasting `costMod` of the declared `discount` (R78: kept
//! until a graveyard takes it off, R766). Built by C9 (#4593).

use jackioh_engine::effects::{ForEachCardArgs, for_each_card, set_cost_mod};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-204";

fn moores(units_only: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let discount = param(&*ctx, "discount");
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(move |each: &mut EffectContext<'_>| {
                    zone_cards(each.state, each.controller, OffFieldZone::Hand)
                        .into_iter()
                        .filter(|card| {
                            !units_only || card_type_of(each.state, card) == CardType::Unit
                        })
                        .map(|card| card.id)
                        .collect()
                }),
                each: Arc::new(move |instance_id: &str| {
                    set_cost_mod(json_as(json!({
                        "target": { "of": "instance", "instanceId": instance_id },
                        "amount": -discount,
                    })))
                }),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: moores(true),
        radiant: moores(false),
    }
}

// IA #204 Moore's Law — SPEC §8.13 row 204.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const UNIT: &str = "stand-in (2) 3/3";
    const SPELL: &str = "stand-in (2) Spell";

    fn cast(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, UNIT, SPELL],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": {},
        }));
        s.play(ID, json!({}));
        s
    }

    fn cost_of(s: &Scenario, def_id: &str) -> i32 {
        let card = s
            .hand(P1)
            .into_iter()
            .find(|held| held.def_id == def_id)
            .unwrap_or_else(|| panic!("no {def_id} in hand"));
        effective_cost(s.state(), &card, Default::default())
    }

    mod base {
        use super::*;

        #[test]
        fn discounts_units_only() {
            let s = cast(false, "moores-base");
            assert_eq!(cost_of(&s, UNIT), 1);
            assert_eq!(cost_of(&s, SPELL), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn discounts_every_card() {
            let s = cast(true, "moores-radiant");
            assert_eq!(cost_of(&s, UNIT), 1);
            assert_eq!(cost_of(&s, SPELL), 1);
        }
    }
}
