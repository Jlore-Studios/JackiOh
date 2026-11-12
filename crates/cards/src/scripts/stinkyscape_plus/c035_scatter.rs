//! SPEC §8.12 S+ #35 Scatter (Unit 0/6 → 0/12, Mindustrial, Common, cost 2), a card of the Stinkyscape+ set (R1420).
//!   Base:    "Flexible, Can't attack, Can't Defend
//!            Activate 1: Deal 0~2 damage to all enemies, if the Unit is Flying deal {damage2} damage instead" — damage2 8
//!   Radiant: "Flexible, Can't attack, Can't Defend
//!            Activate 2: Deal 1~3 damage to all enemies, if the Unit is Flying deal {damage2} damage instead" — damage2 10
//!
//! Engine: Flexible is a catalog keyword (R3543000): played into the backrow Scatter is a Unit there and still
//! takes its Activate. The Activate is 1 use a turn on the base face and 2 on the Radiant face (R384,
//! R1857003; the `Activate:` and `Activate 2:` labels, no param, R482). R3559000: "all enemies" is every enemy Unit, then the enemy hero, as S #13
//! and S #40 read it (`damage_all`, R59); each target rolls its own uniform amount just before its hit, as
//! Flak's targets do (R2521001), Lucky and Luck applying (R987); a Unit that has Flying when its hit comes
//! takes `damage2` instead and rolls nothing (`keywordInstead`); a hero is never Flying, and a 0 is no hit
//! (R63).

use jackioh_engine::effects::{DamageOdds, damage_all};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-035";

/// "Activate": 1 use a turn on the base face, a label and no param (R384, R482, R1857003).
const USES: i32 = 1;
/// "Activate 2": 2 uses a turn on the Radiant face.
const RADIANT_USES: i32 = 2;

/// "0~2" on the base face, "1~3" on the Radiant face.
const LEAST: i32 = 0;
const MOST: i32 = 2;
const RADIANT_LEAST: i32 = 1;
const RADIANT_MOST: i32 = 3;

/// R3559000: every amount in `least..=most` is as likely as any other.
const WEIGHT: i32 = 1;

/// R3559000: `WEIGHT` on each amount in `least..=most`.
fn odds(least: i32, most: i32) -> Vec<DamageOdds> {
    (least..=most)
        .map(|amount| DamageOdds {
            amount,
            weight: WEIGHT,
        })
        .collect()
}

/// R3559000: "deal `least`~`most` damage to all enemies, if the Unit is Flying deal `damage2` instead".
fn scatter(uses: i32, least: i32, most: i32) -> Script {
    let activation = ActivationDecl {
        id: "scatter".into(),
        label: "Deal damage to all enemies".into(),
        uses: ActivationUses::Count(uses),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |ctx| {
            // R987: the card's Lucky now plus its controller's Luck, as Flak reads it.
            let lucky = ctx.live_self().map_or(0, |me| lucky_on(&*ctx.state, me))
                + luck_of(&*ctx.state, ctx.controller);
            vec![damage_all(json_as(json!({
                "amount": least,
                "odds": odds(least, most),
                "lucky": lucky,
                "side": "enemy",
                "heroes": true,
                "keywordInstead": { "keyword": KeywordKind::Flying, "amount": param(&*ctx, "damage2") },
            })))]
        }),
    };
    Script {
        activations: vec![activation],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: scatter(USES, LEAST, MOST),
        radiant: scatter(RADIANT_USES, RADIANT_LEAST, RADIANT_MOST),
    }
}

// S+ #35 Scatter — SPEC §8.12 row 35 (R3559000; R59, R63, R384, R987, R2521001, R3543000).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{next_turn_of, weapon_board};
    use jackioh_engine::subsystems::activate::activate_actions_for;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// An enemy no roll can bring down.
    const TOUGH: &str = "stand-in (1) 0/40";
    const FLYING: &str = "stand-in (1) 0/40 Flying";

    /// What a Flying Unit takes: the catalog's `damage2`, base and Radiant.
    const FLYER_DAMAGE: i32 = 8;
    const RADIANT_FLYER_DAMAGE: i32 = 10;
    const RADIANT_HEALTH: i32 = 12;
    const SEEDS: usize = 40;

    /// Scatter and a Flying Unit of its own for p1, p2's board as given.
    fn board(radiant: bool, seed: &str, p2: Value) -> Scenario {
        weapon_board(
            seed,
            json!({ "field": [{ "def": ID, "radiant": radiant }, FLYING] }),
            json!({ "field": p2 }),
        )
    }

    fn offers(s: &Scenario) -> usize {
        activate_actions_for(s.state(), P1, s.card(ID)).len()
    }

    /// Activates Scatter against p2's TOUGH, TOUGH and FLYING in lanes 1 to 3 and holds every hit to
    /// the rolls a copy of the match rng gives: lanes 1 and 2 then the hero each `least + rng.int(n)`,
    /// the flyer none; p1's own flyer is untouched.
    fn scatter_as_foreseen(radiant: bool, seed: &str, least: i32, most: i32, flyer: i32) -> Vec<i32> {
        let mut s = board(radiant, seed, json!([TOUGH, TOUGH, FLYING]));
        let (a, b, flying) = (s.unit_at(P2, 1).id, s.unit_at(P2, 2).id, s.unit_at(P2, 3).id);
        let own = s.unit_at(P1, 2).id;
        let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
        let span = most - least + 1;
        let rolls: Vec<i32> = (0..3).map(|_| least + rng.int(span)).collect();
        s.activate(ID, json!({}));
        assert_eq!(s.card(a.as_str()).damage, rolls[0], "lane 1 takes its own roll");
        assert_eq!(s.card(b.as_str()).damage, rolls[1], "lane 2 takes its own roll");
        assert_eq!(s.card(flying.as_str()).damage, flyer, "a Flying Unit takes the stated amount");
        assert_eq!(s.card(own.as_str()).damage, 0, "its own side is not an enemy");
        s.expect_health(P2, HERO_HEALTH - rolls[2]);
        s.expect_health(P1, HERO_HEALTH);
        assert_eq!(
            s.state().rng_cursor,
            rng.cursor(),
            "one draw for each Unit that is not Flying and one for the hero, no other"
        );
        rolls
    }

    mod base {
        use super::*;

        #[test]
        fn r3559000_rolls_0_to_2_for_each_enemy_and_deals_8_to_a_flyer() {
            let mut seen: IndexSet<i32> = IndexSet::new();
            let mut differed = false;
            for seed in 0..SEEDS {
                let rolls = scatter_as_foreseen(false, &format!("s035-base-{seed}"), LEAST, MOST, FLYER_DAMAGE);
                differed |= rolls[0] != rolls[1];
                seen.extend(rolls);
            }
            assert!(differed, "two enemies can take different amounts in one Activate");
            assert_eq!(seen, IndexSet::from([0, 1, 2]), "every amount from 0 to 2 comes up, none other");
        }

        #[test]
        fn r63_a_roll_of_0_is_no_hit() {
            for seed in 0..SEEDS {
                let mut s = board(false, &format!("s035-zero-{seed}"), json!([TOUGH]));
                let enemy = s.unit_at(P2, 1).id;
                s.activate(ID, json!({}));
                let dealt = s.card(enemy.as_str()).damage;
                let hits: Vec<(String, i32)> = crate::hits(s.last_events())
                    .into_iter()
                    .filter(|(target, _)| *target == enemy)
                    .collect();
                assert_eq!(hits.len(), usize::from(dealt > 0), "a hit only for an amount above 0");
            }
        }

        #[test]
        fn r384_it_activates_once_a_turn() {
            let mut s = board(false, "s035-once", json!([TOUGH]));
            assert_eq!(offers(&s), 1);
            s.activate(ID, json!({}));
            assert_eq!(offers(&s), 0);
            s.expect_refused(|s| s.activate(ID, json!({})));
            next_turn_of(&mut s, P1);
            assert_eq!(offers(&s), 1);
        }

        #[test]
        fn r3543000_played_into_the_backrow_it_still_activates() {
            let mut s = weapon_board(
                "s035-backrow",
                json!({ "hand": [ID], "mana": 10 }),
                json!({ "field": [TOUGH] }),
            );
            let enemy = s.unit_at(P2, 1).id;
            s.play(ID, json!({ "zone": 2, "row": "backrow" }));
            assert!(s.backrow(P1, 2).is_some_and(|card| card.def_id == ID), "it stands in the backrow");
            next_turn_of(&mut s, P1);
            s.activate(ID, json!({}));
            assert!(s.card(enemy.as_str()).damage <= MOST, "its roll landed on the enemy");
            assert_eq!(offers(&s), 0, "the one use is spent");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3559000_rolls_1_to_3_for_each_enemy_and_deals_10_to_a_flyer() {
            let mut seen: IndexSet<i32> = IndexSet::new();
            for seed in 0..SEEDS {
                let rolls = scatter_as_foreseen(
                    true,
                    &format!("s035-radiant-{seed}"),
                    RADIANT_LEAST,
                    RADIANT_MOST,
                    RADIANT_FLYER_DAMAGE,
                );
                seen.extend(rolls);
            }
            assert_eq!(seen, IndexSet::from([1, 2, 3]), "every amount from 1 to 3 comes up, none other");
        }

        #[test]
        fn r384_it_activates_twice_a_turn_and_no_third_time() {
            let mut s = board(true, "s035-twice", json!([TOUGH]));
            assert_eq!(offers(&s), 1);
            s.activate(ID, json!({}));
            assert_eq!(offers(&s), 1, "one use left");
            s.activate(ID, json!({}));
            assert_eq!(offers(&s), 0);
            s.expect_refused(|s| s.activate(ID, json!({})));
        }

        #[test]
        fn its_health_is_doubled() {
            let s = board(true, "s035-health", json!([]));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (0, RADIANT_HEALTH));
        }
    }
}
