//! SPEC §8.12 S+ #1 Starfruit, a card of the Stinkyscape+ set (R1420, R3535000). (1) Spell, Fruit, Common.
//!   Base:    "Cleave, Pierce. Deal {damage} damage to a Unit." — 1.
//!   Radiant: "Cleave, Pierce, Lifesteal. Deal {damage} damage to a Unit." — 2.
//!
//! R3566000: the declared target is any Unit (R81). A Spell's Cleave gives the same hit to the Units
//! beside the target in its row and side (R2198003), every Unit read before the first hit; Pierce is
//! `ignoreArmor` (R346) on every hit, and the Radiant face's Lifesteal is on every hit (R85). The
//! number is the declared `damage` (R386).

use jackioh_engine::effects::{
    BoardScope, DamageEffectArgs, DamageFlagArgs, TargetSpec, adjacent_to_aimed, damage,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-001";

/// "a Unit": a Unit of either side, never a hero.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

/// R3566000: the chosen Unit's hit, then the same hit on each Unit beside it, read before the first.
fn shot(ctx: &EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    let amount = param(ctx, "damage");
    let hit = |to: TargetSpec| {
        damage(DamageEffectArgs {
            to,
            amount,
            flags: DamageFlagArgs {
                ignore_armor: Some(true),
                lifesteal: radiant.then_some(true),
                ..DamageFlagArgs::default()
            },
        })
    };
    let chosen = TargetSpec::Chosen { index: None };
    let beside = adjacent_to_aimed(ctx, &chosen, &BoardScope::default(), TargetAim::Harm);
    let mut hits = vec![hit(chosen)];
    hits.extend(
        beside
            .into_iter()
            .map(|unit| hit(TargetSpec::Instance { instance_id: unit.id })),
    );
    hits
}

fn starfruit(radiant: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| shot(ctx, radiant))),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: starfruit(false),
        radiant: starfruit(true),
    }
}

// S+ #1 Starfruit — SPEC §8.12 row 1 (R3566000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A 4/4 with Armor 3: Pierce goes through it.
    const ARMORED: &str = "stand-in (1) 4/4 Armor 3";
    const FILLER: &str = "core-005";

    fn board(radiant: bool, hero_health: i32) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "health": hero_health },
            "p2": { "hand": [FILLER], "field": [ARMORED, ARMORED, ARMORED, ARMORED] },
        }))
    }

    #[test]
    fn r3566000_base_hits_the_pick_and_its_neighbours_for_1_through_armor() {
        let mut s = board(false, HERO_HEALTH);
        let (first, second, third, fourth) = (
            s.unit_at(P2, 1),
            s.unit_at(P2, 2),
            s.unit_at(P2, 3),
            s.unit_at(P2, 4),
        );

        s.play(ID, json!({ "targets": pick::instance(&second) }));

        for hit in [&first, &second, &third] {
            s.expect_stats(hit, json!({ "health": 3 }));
        }
        s.expect_stats(&fourth, json!({ "health": 4 }));
        s.expect_health(P1, HERO_HEALTH);
    }

    #[test]
    fn r3566000_radiant_hits_for_2_and_heals_the_hero_for_every_hit() {
        let mut s = board(true, 10);
        let (first, second, third, fourth) = (
            s.unit_at(P2, 1),
            s.unit_at(P2, 2),
            s.unit_at(P2, 3),
            s.unit_at(P2, 4),
        );

        s.play(ID, json!({ "targets": pick::instance(&second) }));

        for hit in [&first, &second, &third] {
            s.expect_stats(hit, json!({ "health": 2 }));
        }
        s.expect_stats(&fourth, json!({ "health": 4 }));
        s.expect_health(P1, 10 + 3 * 2);
    }

    #[test]
    fn r3566000_the_pick_at_the_edge_hits_one_neighbour() {
        let mut s = board(false, HERO_HEALTH);
        let (first, second, third) = (s.unit_at(P2, 1), s.unit_at(P2, 2), s.unit_at(P2, 3));

        s.play(ID, json!({ "targets": pick::instance(&first) }));

        s.expect_stats(&first, json!({ "health": 3 }));
        s.expect_stats(&second, json!({ "health": 3 }));
        s.expect_stats(&third, json!({ "health": 4 }));
    }
}
