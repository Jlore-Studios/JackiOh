//! Stinkyscape #53 Flak v3 (SPEC §8.10 row 53; R2521001). Unit 0/2 → 0/4, Weapon, cost 2, Rare.
//!   Base:    "Starship Piece, Equip
//!            Activate: Deal a random {least} to {most} damage to each enemy Unit." — 1 to 3
//!   Radiant: "Starship Piece, Equip, Lucky 1
//!            Activate: …the same…" — 2 to 4
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses its base version into its owner's Dormant Starship
//! (R1852000, R2521000).
//! #2657: "each enemy Unit" is every enemy Unit and no hero, one hit each before one state check
//! (`damage_all`, R59). R2521001: each hit is its own roll, made just before it lands: `ODDS` laid
//! over `least..=most` from the low end (a value past the third weighted as the third, the two
//! swapped if a Nerf crossed them). Lucky is the card's Lucky now plus its controller's Luck, as
//! S #145 Fortify Flesh reads it (R1438, R987): the Radiant face prints Lucky 1, so each target rolls
//! twice and takes the higher. Once a turn (R384). Both ends are declared numbers (R386), read
//! through `param`. `script_v0_5_9f` and `script_v0_5_11` keep the older behavior for replays (R767000).

use jackioh_engine::effects::{DamageOdds, damage_all};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-053";

/// R2521001: "60%, 30%, 10%, lowest first".
const ODDS: [i32; 3] = [60, 30, 10];

/// R2521001: `ODDS` over `least..=most`, the low end first; a value past the third is weighted as
/// the third, and a Nerf that crossed the two ends rolls between them.
fn odds(least: i32, most: i32) -> Vec<DamageOdds> {
    let (lo, hi) = if least <= most {
        (least, most)
    } else {
        (most, least)
    };
    (lo..=hi)
        .enumerate()
        .map(|(at, amount)| DamageOdds {
            amount,
            weight: ODDS[at.min(ODDS.len() - 1)],
        })
        .collect()
}

/// R1862001, R2521001: "deal a random `least` to `most` damage to each enemy", each target rolling
/// its own, `lucky` more times kept at the highest.
/// `heroes`: the enemy hero too, as through v0.5.9f; each enemy Unit alone since #2657.
fn round(least: i32, most: i32, lucky: i32, heroes: bool) -> Vec<Effect> {
    vec![damage_all(json_as(json!({
        "amount": least.min(most),
        "odds": odds(least, most),
        "lucky": lucky,
        "side": "enemy",
        "heroes": heroes,
    })))]
}

fn flak(heroes: bool) -> ActivationDecl {
    ActivationDecl {
        id: "flak".into(),
        label: if heroes {
            "Deal random damage to each enemy".into()
        } else {
            "Deal random damage to each enemy Unit".into()
        },
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |ctx| {
            let lucky = ctx.live_self().map_or(0, |me| lucky_on(&*ctx.state, me))
                + luck_of(&*ctx.state, ctx.controller);
            round(param(&*ctx, "least"), param(&*ctx, "most"), lucky, heroes)
        }),
    }
}

fn scripts(heroes: bool) -> CardScripts {
    let base = Script {
        activations: vec![flak(heroes)],
        ..Script::default()
    };
    // The same script: the Radiant face's range and its printed Lucky 1 are its catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

pub fn script() -> CardScripts {
    scripts(false)
}

/// R767000: through v0.5.9f its rolls hit the enemy hero too, after each enemy Unit; `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_5_9f() -> CardScripts {
    scripts(true)
}

/// R767000: "deal `amount` damage to all enemies" as v0.5.11 ran it: its declared `damage`, no roll.
fn round_v0_5_11(amount: i32) -> Vec<Effect> {
    vec![damage_all(json_as(
        json!({ "amount": amount, "side": "enemy", "heroes": true }),
    ))]
}

/// R767000: the Activate as v0.5.11 ran it.
fn flak_v0_5_11() -> ActivationDecl {
    ActivationDecl {
        id: "flak".into(),
        label: "Deal damage to all enemies".into(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| round_v0_5_11(param(&*ctx, "damage"))),
    }
}

/// R767000: through v0.5.11 its Activate dealt its declared `damage` to all enemies, with no roll and
/// no Lucky; `KEPT_SCRIPTS` folds those replays under it, the definitions they carry declaring
/// `damage`.
pub fn script_v0_5_11() -> CardScripts {
    let base = Script {
        activations: vec![flak_v0_5_11()],
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #53 Flak v3 — SPEC §8.10 row 53, BUILD M12 row S 53 (#2657, R2521001).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::stinkyscape_family as fam;

    /// How many seeds a sweep of seeds walks.
    const SEEDS: usize = 100;
    const ROUNDS: i32 = 1;

    mod base {
        use super::*;

        #[test]
        fn r767000_the_script_kept_for_v0_5_9f_hits_the_enemy_hero_too() {
            fam::flak_kept_v9(ID, "s005-kept-9e", ROUNDS, script_v0_5_9f);
        }

        #[test]
        fn r767000_the_script_kept_for_v0_5_11_deals_its_declared_damage_and_rolls_nothing() {
            fam::flak_kept_v11(ID, "s053-kept", ROUNDS, script_v0_5_11);
        }

        #[test]
        fn r1862001_r2521001_each_enemy_unit_takes_its_own_1_to_3_and_the_hero_none() {
            let seen = fam::flak_over_the_seeds(ID, "s053", false, fam::FLAK_BASE_RANGE, 0, ROUNDS, SEEDS);
            let counts = &seen.counts;
            assert!(seen.differed, "two enemies can take different amounts");
            let (lo, hi) = fam::FLAK_BASE_RANGE;
            let [one, two, three] = [lo, lo + 1, hi].map(|amount| counts.get(&amount).copied().unwrap_or(0));
            assert_eq!(
                one + two + three,
                counts.values().sum::<usize>(),
                "1 to 3 only: {counts:?}"
            );
            assert!(one > two && two > three && three > 0, "60/30/10: {counts:?}");
        }

        #[test]
        fn r2521001_r1438_given_lucky_rolls_each_target_again_keeping_the_higher() {
            fam::flak_lucky_again(ID, "s053", fam::FLAK_BASE_RANGE, SEEDS, ROUNDS);
        }

        #[test]
        fn r384_once_a_turn_a_second_use_refused() {
            fam::flak_once_a_turn(ID, "s053", fam::FLAK_BASE_RANGE, ROUNDS);
        }

        #[test]
        fn r1086_r1852001_an_equipped_host_takes_the_activate_and_is_no_starship_piece() {
            fam::flak_equip_takes_activate(ID, "s053", fam::FLAK_BASE_RANGE, ROUNDS);
        }

        #[test]
        fn r386_r2521001_an_upgrade_of_most_rolls_past_the_third_at_the_thirds_weight() {
            fam::flak_upgrade_most(ID, "s053", fam::FLAK_BASE_RANGE, SEEDS, ROUNDS);
        }

        #[test]
        fn r386_r2521001_an_upgrade_of_least_lifts_the_low_end() {
            fam::flak_upgrade_least(ID, "s053", fam::FLAK_BASE_RANGE, ROUNDS);
        }

        #[test]
        fn r2521001_a_lift_of_least_past_most_rolls_between_the_two() {
            fam::flak_roll_between(ID, "s053", fam::FLAK_BASE_RANGE, SEEDS, ROUNDS);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1862001_r2521001_each_enemy_rolls_2_to_4_twice_keeping_the_higher() {
            let seen = fam::flak_over_the_seeds(
                ID,
                "s053",
                true,
                fam::FLAK_RADIANT_RANGE,
                fam::FLAK_RADIANT_LUCKY,
                ROUNDS,
                SEEDS,
            );
            let counts = &seen.counts;
            assert!(seen.differed, "two enemies can take different amounts");
            let total: usize = counts.values().sum();
            let (lo, hi) = fam::FLAK_RADIANT_RANGE;
            assert!(
                counts.keys().all(|amount| (lo..=hi).contains(amount)),
                "{counts:?}"
            );
            assert!((lo..=hi).all(|amount| counts.contains_key(&amount)), "{counts:?}");
            // Lucky 1 lifts the odds off the low end: a 2 needs both rolls to be 2s (36%, not 60%).
            let low = counts.get(&fam::FLAK_RADIANT_RANGE.0).copied().unwrap_or(0);
            assert!(2 * low < total, "{counts:?}");
        }

        #[test]
        fn r2521001_a_radiant_first_hit_is_the_higher_of_its_two_rolls() {
            fam::flak_radiant_pair(
                ID,
                "s053",
                fam::FLAK_RADIANT_RANGE,
                fam::FLAK_RADIANT_LUCKY,
                SEEDS,
                ROUNDS,
            );
        }
    }
}
