//! SPEC §8.13 IA #62 Burnout ((1) Trap, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reveals when your opponent ends their turn with unspent mana: Deal twice that much damage to their hero."
//!   Radiant: "Reveals when your opponent ends their turn with unspent mana: Deal four times that much damage to their hero."
//!
//! Core #18 Bread and Butter's end-of-turn trap window (§4.2 step 4, R62, R68): the Float straight
//! off the `turnEnded` event (`float_of`), but only when the turn that ended is the opponent's.
//! The hit is the Float times the face's multiplier, to the opponent's hero. Any other event, a
//! turn that ends with nothing unspent, and its controller's own turn leave it set (R61).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-062";

/// The base face's "twice that much".
const BASE_MULTIPLIER: i32 = 2;

/// The Radiant face's "four times that much".
const RADIANT_MULTIPLIER: i32 = 4;

/// The opponent's unspent mana on this event, else 0: Core #18's reader, narrowed to the turn its
/// controller did not take.
fn opponents_float(ctx: &EffectContext<'_>, event: &GameEvent) -> i32 {
    float_of(event).map_or(0, |(player, float)| {
        if player == opponent_of(ctx.controller) {
            float
        } else {
            0
        }
    })
}

fn burnout(multiplier: i32) -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("burnout", &[GameEventType::TurnEnded], move |ctx, event| {
                let amount = opponents_float(ctx, event) * multiplier;
                if amount <= 0 {
                    return vec![];
                }
                vec![damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": amount }),
                ))]
            })
            .with_when(|ctx, event| opponents_float(ctx, event) > 0),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: burnout(BASE_MULTIPLIER),
        radiant: burnout(RADIANT_MULTIPLIER),
    }
}

// IA #62 Burnout — SPEC §8.13 row 62.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: a card in hand, so no turn ends by itself

    fn armed(radiant: bool, active: &str, mana: i32) -> Scenario {
        scenario(json!({
            "active": active,
            "p1": { "backrow": [{ "def": ID, "radiant": radiant }], "library": crate::filler(2) },
            "p2": { "hand": [FILLER], "mana": mana, "library": crate::filler(2) },
        }))
    }

    fn trap(s: &Scenario) -> CardInstance {
        match s.backrow(P1, 1) {
            Some(found) => found,
            None => panic!("no trap in p1 backrow lane 1"),
        }
    }

    mod base {
        use super::*;

        #[test]
        fn a_float_of_3_deals_6_to_their_hero() {
            let mut s = armed(false, "p2", 3);
            s.end_turn();
            s.expect_health(P2, HERO_HEALTH - 6);
            s.expect_health(P1, HERO_HEALTH);
        }

        #[test]
        fn a_turn_that_spends_everything_leaves_it_set() {
            let mut s = armed(false, "p2", 0);
            let before = trap(&s);
            s.end_turn();
            s.expect_health(P2, HERO_HEALTH);
            s.expect_in_zone(&before, "field");
        }

        #[test]
        fn your_own_float_does_not_reveal_it() {
            let mut s = scenario(json!({
                "p1": { "backrow": [ID], "mana": 3, "library": crate::filler(2) },
                "p2": { "hand": [FILLER], "library": crate::filler(2) },
            }));
            let before = trap(&s);
            s.end_turn();
            s.expect_health(P1, HERO_HEALTH);
            s.expect_in_zone(&before, "field");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_float_of_3_deals_12_to_their_hero() {
            let mut s = armed(true, "p2", 3);
            s.end_turn();
            s.expect_health(P2, HERO_HEALTH - 12);
        }
    }
}
