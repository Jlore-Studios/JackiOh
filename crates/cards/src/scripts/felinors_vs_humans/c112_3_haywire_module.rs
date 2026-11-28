//! FvH #112.3 Haywire Module (SPEC §8.14 row 112.3; R62): (1) Unit, Token (printed Mythic), 2/2 → 4/4, of
//! the held Felinors vs Humans set. A module FvH #112 Jilliax Deluxe offers and fuses (FV2 #112).
//!   Base and Radiant: "End of turn: Deal {damage} damage to your hero." — damage 3 on both faces.
//! Engine: an `end_of_turn` hook (its controller's turn, R62): `damage` of `damage` to its own hero, as S #43
//! Healing Capsule's end-of-turn heal. Fused, it is the fused card's, its number read off this ingredient.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-112-3";

/// The declared "Deal {damage} damage" (`params`).
const DAMAGE_KEY: &str = "damage";

pub fn script() -> CardScripts {
    let base = Script {
        end_of_turn: Some(hook(|ctx| {
            let amount = param(ctx, DAMAGE_KEY);
            vec![damage(json_as(json!({ "to": { "of": "selfHero" }, "amount": amount })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 3 is its declared `damage`; its 4/4 is catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #112.3 Haywire Module — SPEC §8.14 row 112.3 (R62).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const FILLER: &str = "core-005"; // Stockpile: a card in hand, so no turn ends by itself
    /// The printed damage on both faces.
    const DAMAGE: i32 = 3;

    fn on_field(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "radiant": radiant }], "library": filler(6) },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    /// At its controller's end of turn its own hero takes 3; the other player's end of turn does nothing.
    fn burns_its_own_hero(radiant: bool) {
        let mut s = on_field(radiant);
        s.end_turn();
        s.expect_health(P1, HERO_HEALTH - DAMAGE);
        s.expect_health(P2, HERO_HEALTH);
        s.end_turn();
        s.expect_health(P1, HERO_HEALTH - DAMAGE);
    }

    mod base {
        use super::*;

        #[test]
        fn r62_at_the_end_of_your_turn_your_hero_takes_3() {
            burns_its_own_hero(false);
        }

        #[test]
        fn r386_a_nerf_of_the_number_deals_less() {
            let mut s = on_field(false);
            step_param(s.card_mut(ID), "damage", -1);
            s.end_turn();
            s.expect_health(P1, HERO_HEALTH - (DAMAGE - 1));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r62_at_the_end_of_your_turn_your_hero_takes_3() {
            burns_its_own_hero(true);
        }
    }
}
