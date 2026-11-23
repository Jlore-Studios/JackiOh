//! SPEC §8.13 IA #11 Quickgen Turret ((2) Unit, Rare, 2/5 → 4/10), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Ranged\nStart of turn: Deal {damage} damage to a random enemy for each Created card in your hand." (damage 1)
//!   Radiant: "Ranged\nStart of turn: Deal {damage} damage to a random enemy for each Created card in your hand." (damage 2)
//!
//! C+ #3 Second Amendment Snake's `damageSplit`: one hit of `damage` per Created card (R943) in its
//! controller's hand as its turn starts, each at a random enemy still standing, the hero included,
//! drawn again for every hit. Ranged is the catalog's.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-011";

/// The Created cards in the controller's hand.
fn created_in_hand(ctx: &EffectContext<'_>) -> i32 {
    let count = zone_cards(ctx.state, ctx.controller, OffFieldZone::Hand)
        .iter()
        .filter(|card| is_created(card))
        .count();
    i32::try_from(count).unwrap_or(i32::MAX)
}

pub fn script() -> CardScripts {
    let base = Script {
        start_of_turn: Some(hook(|ctx| {
            let hits = created_in_hand(ctx);
            if hits == 0 {
                return vec![];
            }
            let per_hit = param(&*ctx, "damage");
            vec![damage_split(json_as(json!({
                "amount": hits * per_hit,
                "perHit": per_hit,
                "among": "enemies",
            })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 2 is its declared `damage`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #11 Quickgen Turret — SPEC §8.13 row 11.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SPELL: &str = "stand-in (0) Spell";

    /// The Turret on p1's field, `created` Created cards and one dealt card in hand, and an empty enemy
    /// board, so every hit lands on the enemy hero.
    fn turn_starts(radiant: bool, created: usize) -> Scenario {
        let mut hand = vec![json!(SPELL)];
        hand.extend((0..created).map(|_| json!({ "def": SPELL, "created": true })));
        let mut s = scenario(json!({
            "p1": { "hand": hand, "field": [{ "def": ID, "radiant": radiant }], "library": crate::filler(2) },
            "p2": { "library": crate::filler(2) },
        }));
        s.start_turn();
        s
    }

    mod base {
        use super::*;

        #[test]
        fn one_damage_for_each_created_card_in_hand() {
            let mut s = turn_starts(false, 2);
            s.expect_health(P2, HERO_HEALTH - 2);
            let turret = s.unit_at(P1, 1);
            assert!(has_keyword(&s.stats(&turret).keywords, KeywordKind::Ranged));
        }

        #[test]
        fn no_created_card_no_hit() {
            let mut s = turn_starts(false, 0);
            s.expect_health(P2, HERO_HEALTH);
        }

        #[test]
        fn r68_none_at_the_opponents_start_of_turn() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [{ "def": SPELL, "created": true }], "field": [ID] },
                "p2": { "library": crate::filler(2) },
            }));
            s.start_turn();
            s.expect_health(P2, HERO_HEALTH);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn two_damage_for_each_created_card_in_hand() {
            let mut s = turn_starts(true, 3);
            s.expect_health(P2, HERO_HEALTH - 6);
        }
    }
}
