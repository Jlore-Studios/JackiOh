//! S #145 Fortify Flesh (SPEC §8.10 row S 145; §4.4, §6.1, R124, R1438, R1867008): (4) Spell, Rare.
//!   Base:    "Your hero gains a random {least} to {most} Armor." — 5 to 8 since #5183 (R5183004; 3 to 5
//!            before).
//!   Radiant: "Lucky 1 / Your hero gains a random {least} to {most} Armor." — the base range since issue
//!            #4292 (3 to 7 before).
//!
//! Engine: one luck-based roll from the match rng, uniform from the declared least to the declared
//! most, both ends included (a Buff that lifts the low end past the high one rolls between the
//! two), then `gain_hero_armor`: the hero's Armor, §4.4 step 2's reduction of every hit, which each
//! hit it meets wears by 1 (R5183000), stacking with any Armor already there (C+ #46's, R124). Lucky is the
//! card's Lucky now plus its controller's Luck (C+ #25 Soul Shot's shape): the Radiant face prints
//! Lucky 1, so it rolls once more and keeps the higher (§6.1 Lucky X, R1438, R987).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-145";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            let least = param(&*ctx, "least");
            let most = param(&*ctx, "most");
            let (lo, hi) = if least <= most { (least, most) } else { (most, least) };
            let span = hi - lo + 1;
            let roll = |rng: &mut Rng| lo + rng.int(span);
            let total = ctx.live_self().map_or(0, |me| lucky_on(&*ctx.state, me))
                + luck_of(&*ctx.state, ctx.controller);
            let amount = if total > 0 {
                ctx.rng.lucky(total, roll, Ord::max)
            } else {
                roll(&mut *ctx.rng)
            };
            vec![gain_hero_armor(json_as(json!({ "amount": amount })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's range and its printed Lucky 1 are its catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #145 Fortify Flesh — SPEC §8.10 row S 145, BUILD M12 row S 145 (R1867008).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FORTIFY: &str = "stinkyscape-145";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005";

    fn armor_of(s: &Scenario, player: PlayerId) -> i32 {
        s.state().players[player].hero.armor
    }

    fn gained(seed: &str, hand: Value) -> (i32, u32) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": hand },
            "p2": { "hand": [FILLER] },
        }));
        let before = s.state().rng_cursor;
        s.play(FORTIFY, json!({}));
        (armor_of(&s, P1), s.state().rng_cursor - before)
    }

    mod base {
        use super::*;

        #[test]
        fn r1867008_r5183004_armor_rises_by_5_to_8_each_seen_over_the_seeds() {
            let mut seen: IndexSet<i32> = IndexSet::new();
            for seed in 0..120 {
                let (gained, draws) = gained(&format!("fortify-{seed}"), json!([FORTIFY, FILLER]));
                assert!((5..=8).contains(&gained), "{gained} in 5 to 8");
                assert_eq!(draws, 1, "one roll");
                seen.insert(gained);
            }
            let expected: IndexSet<i32> = [5, 6, 7, 8].into_iter().collect();
            assert_eq!(seen, expected);
        }

        #[test]
        fn r5183000_armor_persists_on_later_turns_stacks_with_armor_already_there_and_a_hit_wears_it() {
            let mut s = scenario(json!({
                "seed": "fortify-persist",
                "p1": { "hand": [FORTIFY, FILLER], "armor": 2 },
                "p2": {
                    "hand": [FILLER],
                    "field": [{ "def": VANILLA, "statsOverride": { "attack": 6, "health": 6 } }],
                },
            }));
            let hammer = s.unit_at(P2, 1);

            s.play(FORTIFY, json!({}));
            let armor = armor_of(&s, P1);
            assert!((7..=10).contains(&armor), "2 stacked with a roll of 5 to 8: {armor}");

            let before = s.state().players.p1.hero.health;
            s.attack(&hammer.id, "hero");
            let dealt = before - s.state().players.p1.hero.health;
            assert_eq!(dealt, (6 - armor).max(0), "a 6-hit through {armor} Armor");
            assert_eq!(armor_of(&s, P1), armor - 1, "the hit wore 1 off it");
        }

        /// R3857000: the gain is reported on the caster's hero, its roll and the Armor after it, to
        /// both seats.
        #[test]
        fn r3857000_the_gain_is_reported_on_the_casters_hero() {
            let mut s = scenario(json!({
                "seed": "fortify-reported",
                "p1": { "hand": [FORTIFY, FILLER], "armor": 2 },
                "p2": { "hand": [FILLER] },
            }));
            s.play(FORTIFY, json!({}));
            let armor = armor_of(&s, P1);
            let gained = GameEvent::ArmorGained {
                player: P1,
                amount: armor - 2,
                armor,
            };
            assert!((5..=8).contains(&(armor - 2)), "a roll of 5 to 8: {armor}");
            assert_eq!(s.events_of(GameEventType::ArmorGained), vec![gained.clone()]);
            for viewer in [P1, P2] {
                assert!(s.view(viewer).events.contains(&gained), "{viewer:?} sees the gain");
            }
        }

        #[test]
        fn r1867008_a_lift_of_the_low_end_past_the_high_one_rolls_between_the_two() {
            for seed in 0..40 {
                let mut s = scenario(json!({
                    "seed": format!("fortify-inverted-{seed}"),
                    "p1": { "hand": [FORTIFY, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                step_param(s.card_mut(FORTIFY), "least", 4);
                s.play(FORTIFY, json!({}));
                let armor = armor_of(&s, P1);
                assert!((8..=9).contains(&armor), "between 8 and 9: {armor}");
            }
        }

        #[test]
        fn r386_an_upgrade_of_most_to_9_rolls_up_to_9() {
            let mut seen: IndexSet<i32> = IndexSet::new();
            for seed in 0..60 {
                let mut s = scenario(json!({
                    "seed": format!("fortify-most-{seed}"),
                    "p1": { "hand": [FORTIFY, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                step_param(s.card_mut(FORTIFY), "most", 1);
                s.play(FORTIFY, json!({}));
                seen.insert(armor_of(&s, P1));
            }
            assert!(seen.contains(&9), "{seen:?}");
            assert!(seen.iter().all(|armor| (5..=9).contains(armor)), "{seen:?}");
        }
    }

    mod radiant {
        use super::*;

        fn radiant_gained(seed: &str) -> (i32, u32) {
            gained(seed, json!([{ "def": FORTIFY, "radiant": true }, FILLER]))
        }

        #[test]
        fn r1867008_r5183004_radiant_rolls_5_to_8_twice_keeping_the_higher() {
            let mut seen: IndexSet<i32> = IndexSet::new();
            for seed in 0..200 {
                let (gained, draws) = radiant_gained(&format!("fortify-radiant-{seed}"));
                assert!((5..=8).contains(&gained), "{gained} in 5 to 8");
                assert_eq!(draws, 2, "Lucky 1 rolls twice");
                seen.insert(gained);
            }
            assert!(seen.contains(&5) && seen.contains(&8), "{seen:?}");
        }

        #[test]
        fn r1867008_a_radiant_roll_is_never_below_the_base_faces_on_the_same_seed() {
            for seed in 0..20 {
                let (plain, _) =
                    gained(&format!("fortify-paired-{seed}"), json!([FORTIFY, FILLER]));
                let (lucky, _) = radiant_gained(&format!("fortify-paired-{seed}"));
                assert!(lucky >= plain, "radiant {lucky} against base {plain}");
            }
        }
    }
}
