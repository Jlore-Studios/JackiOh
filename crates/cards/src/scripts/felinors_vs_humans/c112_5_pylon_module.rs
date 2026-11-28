//! FvH #112.5 Pylon Module (SPEC §8.14 row 112.5): (2) Unit, Token (printed Mythic), 2/2 → 4/4, of the held
//! Felinors vs Humans set. A module FvH #112 Jilliax Deluxe offers and fuses (FV2 #112).
//!   Base and Radiant: "Aura: Your other Units have +{buff}/+{buff}." — buff 1, Radiant 2.
//! Engine: Aura (§10.4): every other Unit its controller has on the field (one standing in the backrow too,
//! R1081000) has +`buff`/+`buff`, TG #3 Group Leader's aura with Health. Fused, it runs as the fused card
//! (`fused_aura`), so "other" is every Unit but the fused card.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-112-5";

/// The declared "+{buff}/+{buff}" (`params`).
const BUFF_KEY: &str = "buff";

fn pylon() -> Script {
    Script {
        aura: Some(aura_hook(|ctx| {
            let amount = param(&ctx, BUFF_KEY);
            vec![AuraEntry {
                applies: Box::new(move |unit: &CardInstance| {
                    unit.controller == ctx.self_.controller
                        && in_units_row_or_standing(ctx.state, unit)
                        && unit.id != ctx.self_.id
                }),
                mod_: StatMod {
                    attack: Some(amount),
                    max_health: Some(amount),
                    ..StatMod::default()
                },
            }]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's 2 is its declared `buff`, which `param` reads.
    let base = pylon();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #112.5 Pylon Module — SPEC §8.14 row 112.5.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const BODY: &str = "stand-in (1) 3/3";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": ["core-005"], "field": [{ "def": ID, "lane": 1, "radiant": radiant }, { "def": BODY, "lane": 2 }] },
            "p2": { "hand": ["core-005"], "field": [{ "def": BODY, "lane": 1 }] },
        }))
    }

    fn stats_at(s: &Scenario, player: PlayerId, lane: i32) -> (i32, i32) {
        let view = s.stats(s.unit_at(player, lane).id.as_str());
        (view.attack, view.health)
    }

    /// Your other Units have +`buff`/+`buff`; the Pylon itself and the enemy's do not.
    fn powers(radiant: bool, buff: i32, own: (i32, i32)) {
        let s = board(radiant);
        assert_eq!(stats_at(&s, P1, 2), (3 + buff, 3 + buff), "your other Unit");
        assert_eq!(stats_at(&s, P2, 1), (3, 3), "an enemy Unit");
        assert_eq!(stats_at(&s, P1, 1), own, "the Pylon itself");
    }

    mod base {
        use super::*;

        #[test]
        fn s10_4_your_other_units_have_plus_1_plus_1() {
            powers(false, 1, (2, 2));
        }

        #[test]
        fn r386_a_buff_of_the_number_raises_the_aura() {
            let mut s = board(false);
            step_param(s.card_mut(ID), "buff", 1);
            assert_eq!(stats_at(&s, P1, 2), (5, 5));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s10_4_your_other_units_have_plus_2_plus_2() {
            powers(true, 2, (4, 4));
        }
    }
}
