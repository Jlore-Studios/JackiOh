//! SPEC §8.13 IA #8 Firing Solution ((1) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Deal {damage} damage to a Unit. If it dies, add a random Infinite Assault Spell to your hand." (damage 3)
//!   Radiant: "Deal {damage} damage to a Unit. If it dies, add a random Infinite Assault Spell to your hand." (damage 6)
//!
//! The hit, then the state check (R59) and the question the Radiant Ping asks after it: whether the
//! Unit has left the field since the Spell began (`left_field_after` against the run's own mark,
//! R174), a Reborn body put back included. If it has, one pick from the set-named pool of Spells
//! (R1846001, R4577000; this card and the tokens left out, R387, §5.1).

use jackioh_engine::effects::{add_random_from_catalog, after_state_check};
use jackioh_engine::prelude::*;
use jackioh_engine::stays::{exit_mark, left_field_after};

pub const ID: &str = "infiniteassault-008";

/// The set every "Infinite Assault card" names (R4577000).
const SET: &str = "Infinite Assault";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Harm),
        ..TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))
    }]
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            let Some(unit) = instance_of(ctx, &TargetSpec::Chosen { index: None }) else {
                return vec![];
            };
            let mark = ctx.exits_from.unwrap_or_else(|| exit_mark(ctx.state));
            let hit = damage(json_as(json!({ "to": { "of": "chosen" }, "amount": param(&*ctx, "damage") })));
            vec![
                hit,
                after_state_check(move |after| {
                    if !left_field_after(after.state, mark, &unit.id) {
                        return vec![];
                    }
                    vec![add_random_from_catalog(json_as(json!({
                        "query": { "set": SET, "type": "Spell" },
                        "count": 1,
                    })))]
                }),
            ]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 6 is its declared `damage`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #8 Firing Solution — SPEC §8.13 row 8 (R59, R174).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::catalog::def_of;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn board(radiant: bool, enemy: &str, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }] },
            "p2": { "field": [enemy] },
        }))
    }

    fn fire(s: &mut Scenario) -> CardInstance {
        let enemy = s.unit_at(P2, 1);
        s.play(ID, pick::targets(pick::instance(&enemy)));
        enemy
    }

    /// The card the kill added, which must be an Infinite Assault Spell.
    fn expect_one_spell(s: &Scenario) {
        let hand = s.hand(P1);
        assert_eq!(hand.len(), 1);
        for card in nonempty(hand, "the added Spell") {
            let def = def_of(Some(s.state()), &card.def_id);
            assert_eq!(def.set, SetName::InfiniteAssault, "{}", def.id);
            assert_eq!(def.type_, CardType::Spell, "{}", def.id);
            assert!(!def.is_token(), "{}", def.id);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn a_kill_adds_a_random_infinite_assault_spell() {
            for seed in 0..8 {
                let mut s = board(false, "stand-in (1) 1/3", &format!("firing-solution-{seed}"));
                let enemy = fire(&mut s);
                s.expect_in_zone(&enemy, "graveyard");
                expect_one_spell(&s);
            }
        }

        #[test]
        fn a_unit_that_lives_gives_nothing() {
            let mut s = board(false, "stand-in (1) 1/4", "firing-solution");
            let enemy = fire(&mut s);
            s.expect_stats(&enemy, json!({ "health": 1 }));
            assert!(s.hand(P1).is_empty());
        }

        #[test]
        fn r174_a_reborn_unit_died_though_it_is_back() {
            let mut s = board(false, "stand-in (1) 1/3 Reborn", "firing-solution");
            fire(&mut s);
            assert_eq!(active_units_of(s.state(), P2).len(), 1, "Reborn put it back");
            expect_one_spell(&s);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn six_damage_kills_what_three_would_not_and_adds_a_spell() {
            let mut s = board(true, "stand-in (1) 1/6", "firing-solution");
            let enemy = fire(&mut s);
            s.expect_in_zone(&enemy, "graveyard");
            expect_one_spell(&s);
        }

        #[test]
        fn a_unit_that_lives_gives_nothing() {
            let mut s = board(true, "stand-in (1) 1/7", "firing-solution");
            fire(&mut s);
            assert!(s.hand(P1).is_empty());
        }
    }
}
