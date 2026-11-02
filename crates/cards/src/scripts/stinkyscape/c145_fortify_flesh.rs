//! S #145 Fortify Flesh (SPEC §8.10 row S 145; §4.4, §6.1, R124, R1438, R1867008): (4) Spell, Rare.
//!   Base:    "Your hero gains a random {least} to {most} Armor." — 3 to 5.
//!   Radiant: "Lucky 1 / Your hero gains a random {least} to {most} Armor." — 3 to 7.
//!
//! Engine: one luck-based roll from the match rng, uniform from the declared least to the declared
//! most, both ends included (a Buff that lifts the low end past the high one rolls between the
//! two), then `gain_hero_armor`: the hero's Armor, §4.4 step 2's reduction of every hit kept for
//! the rest of the game, stacking with any Armor already there (C+ #46's, R124). Lucky is the
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
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005";

    fn armor_of(s: &Scenario, player: PlayerId) -> i32 {
        s.state().players[player].hero.armor
    }

    fn gained(seed: &str, hand: Value) -> (i32, u32) {
        crate::register_all();
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
        fn r1867008_armor_rises_by_3_4_or_5_each_seen_over_the_seeds() {
            let mut seen: IndexSet<i32> = IndexSet::new();
            for seed in 0..120 {
                let (gained, draws) = gained(&format!("fortify-{seed}"), json!([FORTIFY, FILLER]));
                assert!((3..=5).contains(&gained), "{gained} in 3 to 5");
                assert_eq!(draws, 1, "one roll");
                seen.insert(gained);
            }
            let expected: IndexSet<i32> = [3, 4, 5].into_iter().collect();
            assert_eq!(seen, expected);
        }

        #[test]
        fn armor_persists_on_later_turns_and_stacks_with_armor_already_there() {
            crate::register_all();
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
            assert!((5..=7).contains(&armor), "2 stacked with a roll of 3 to 5: {armor}");

            let before = s.state().players.p1.hero.health;
            s.attack(&hammer.id, "hero");
            let dealt = before - s.state().players.p1.hero.health;
            assert_eq!(dealt, (6 - armor).max(0), "a 6-hit through {armor} Armor");
            assert_eq!(armor_of(&s, P1), armor, "Armor stays");
        }

        #[test]
        fn r1867008_a_lift_of_the_low_end_past_the_high_one_rolls_between_the_two() {
            for seed in 0..40 {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": format!("fortify-inverted-{seed}"),
                    "p1": { "hand": [FORTIFY, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                step_param(s.card_mut(FORTIFY), "least", 3);
                s.play(FORTIFY, json!({}));
                let armor = armor_of(&s, P1);
                assert!((5..=6).contains(&armor), "between 5 and 6: {armor}");
            }
        }

        #[test]
        fn r386_an_upgrade_of_most_to_6_rolls_up_to_6() {
            let mut seen: IndexSet<i32> = IndexSet::new();
            for seed in 0..60 {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": format!("fortify-most-{seed}"),
                    "p1": { "hand": [FORTIFY, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                step_param(s.card_mut(FORTIFY), "most", 1);
                s.play(FORTIFY, json!({}));
                seen.insert(armor_of(&s, P1));
            }
            assert!(seen.contains(&6), "{seen:?}");
            assert!(seen.iter().all(|armor| (3..=6).contains(armor)), "{seen:?}");
        }
    }

    mod radiant {
        use super::*;

        fn radiant_gained(seed: &str) -> (i32, u32) {
            gained(seed, json!([{ "def": FORTIFY, "radiant": true }, FILLER]))
        }

        #[test]
        fn r1867008_radiant_rolls_3_to_7_twice_keeping_the_higher() {
            let mut seen: IndexSet<i32> = IndexSet::new();
            for seed in 0..200 {
                let (gained, draws) = radiant_gained(&format!("fortify-radiant-{seed}"));
                assert!((3..=7).contains(&gained), "{gained} in 3 to 7");
                assert_eq!(draws, 2, "Lucky 1 rolls twice");
                seen.insert(gained);
            }
            assert!(seen.contains(&3) && seen.contains(&7), "{seen:?}");
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
