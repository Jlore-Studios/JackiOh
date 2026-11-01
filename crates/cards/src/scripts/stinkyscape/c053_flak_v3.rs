//! Stinkyscape #53 Flak v3 (SPEC §8.10 row 53; R2521001). Unit 0/4 → 0/8, Weapon, cost 2, Rare.
//!   Base:    "Starship Piece, Equip
//!            Activate: Deal a random {least} to {most} damage to each enemy Unit." — 1 to 3
//!   Radiant: "Starship Piece, Equip, Lucky 1
//!            Activate: …the same…" — 2 to 4
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses its base version into its owner's Dormant Starship
//! (R1852000, R2521000).
//! #2657: "each enemy Unit" is every enemy Unit and no hero (every enemy Unit and then the enemy hero,
//! R1862001's "each enemy", through v0.5.9f, `script_v0_5_9f`), one hit each before one state
//! check (`damage_all`, R59). R2521001: each of those hits is
//! its own roll, made just before it lands: `ODDS` laid over `least..=most` from the low end (a value
//! past the third weighted as the third, the two swapped if a Nerf crossed them). Lucky is the card's
//! Lucky now plus its controller's Luck, as S #145 Fortify Flesh reads it (R1438, R987): the Radiant
//! face prints Lucky 1, so each target rolls twice and takes the higher. Once a turn (R384). Both ends
//! are declared numbers (R386), read through `param`.
//! Through v0.5.11 it dealt its declared `damage` to all enemies, no roll, which `script_v0_5_11`
//! keeps (R767000).

use jackioh_engine::effects::{DamageOdds, damage_all};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-053";

/// R2521001: "60%, 30%, 10%, lowest first".
const ODDS: [i32; 3] = [60, 30, 10];

/// R2521001: `ODDS` over `least..=most`, the low end first; a value past the third is weighted as
/// the third, and a Nerf that crossed the two ends rolls between them.
fn odds(least: i32, most: i32) -> Vec<DamageOdds> {
    let (lo, hi) = if least <= most { (least, most) } else { (most, least) };
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

// Stinkyscape #53 Flak v3 — SPEC §8.10 row 53, BUILD M12 row S 53: its Activate deals each enemy Unit
// its own random 1 to 3 (60%, 30%, 10%), and the enemy hero nothing (#2657); two enemies can
// take different amounts; once a turn, a second use refused; equipped onto a Unit, the host takes the
// Activate; least and most read through `param()`, a value past the third weighted as the third;
// given Lucky keeps the higher roll; radiant 0/8, 2 to 4 with Lucky 1: two rolls per target, the
// higher kept (R2521001).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{give_lucky, hits, next_turn_of, weapon_board};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const VANILLA: &str = "core-008"; // (1) 4/4.

    /// The printed ranges, base and Radiant, the Radiant face's Lucky and its health.
    const BASE_RANGE: (i32, i32) = (1, 3);
    const RADIANT_RANGE: (i32, i32) = (2, 4);
    const RADIANT_LUCKY: i32 = 1;
    const RADIANT_HEALTH: i32 = 8;
    const ROUNDS: i32 = 1;
    /// The printed odds as the test reads them, lowest first.
    const PRINTED: [i32; 3] = [60, 30, 10];
    /// How many seeds a sweep of seeds walks.
    const SEEDS: usize = 100;

    /// A Menace no activation can bring down, so every round hits it.
    fn wall() -> Value {
        json!({ "def": MENACE, "statsOverride": { "attack": 9, "health": 40 } })
    }

    fn flak_on(radiant: bool, seed: &str, p2: Value) -> Scenario {
        weapon_board(
            seed,
            json!({ "field": [{ "def": ID, "radiant": radiant }, VANILLA] }),
            p2,
        )
    }

    /// Two walls for p2.
    fn walls() -> Value {
        json!({ "field": [wall(), wall()] })
    }

    /// One roll over `lo..=hi` as the text reads: 60, 30 and 10 from the low end, a value past the
    /// third weighted as the third.
    fn printed_roll(rng: &mut Rng, (lo, hi): (i32, i32)) -> i32 {
        let weight = |amount: i32| PRINTED[((amount - lo) as usize).min(PRINTED.len() - 1)];
        let mut at = rng.int((lo..=hi).map(weight).sum());
        for amount in lo..=hi {
            if at < weight(amount) {
                return amount;
            }
            at -= weight(amount);
        }
        hi
    }

    /// Activates `card` and holds its hits to the rolls a copy of the match rng gives: each enemy Unit
    /// in lane order, round after round, each its own roll over `range` with `lucky` more kept at the
    /// higher, and no other draw; the enemy hero takes nothing (#2657).
    fn fire(s: &mut Scenario, card: &str, range: (i32, i32), lucky: i32) -> Vec<(String, i32)> {
        let targets: Vec<String> = (1..=UNIT_ZONES)
            .filter_map(|lane| s.unit(P2, lane))
            .map(|unit| unit.id)
            .collect();
        let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
        let health = s.state().players.p2.hero.health;
        s.activate(card, json!({}));
        let mut expected: Vec<(String, i32)> = Vec::new();
        for _ in 0..ROUNDS {
            for target in &targets {
                let amount = rng.lucky(lucky, |rng| printed_roll(rng, range), Ord::max);
                expected.push((target.clone(), amount));
            }
        }
        let got = hits(s.last_events());
        assert_eq!(got, expected, "one roll per target and round");
        assert_eq!(s.state().rng_cursor, rng.cursor(), "1 + Lucky draws per hit, no other");
        assert!(got.iter().all(|(target, _)| target != "hero-p2"), "no hero hit since #2657");
        s.expect_health(P2, health);
        s.expect_health(P1, HERO_HEALTH);
        got
    }

    /// Activations over `SEEDS` seeds of the given face against two walls; how often each amount came
    /// up, and whether the two walls ever took different amounts in one round.
    fn over_the_seeds(radiant: bool, range: (i32, i32), lucky: i32) -> (IndexMap<i32, usize>, bool) {
        let mut counts: IndexMap<i32, usize> = IndexMap::new();
        let mut differed = false;
        for seed in 0..SEEDS {
            let mut s = flak_on(radiant, &format!("s053-seeds-{radiant}-{seed}"), walls());
            let got = fire(&mut s, ID, range, lucky);
            differed |= got[0].1 != got[1].1;
            for (_, amount) in got {
                *counts.entry(amount).or_default() += 1;
            }
        }
        (counts, differed)
    }

    mod base {
        use super::*;

        /// R767000: the script kept for the replays of v0.5.9f and before rolls for the enemy hero too,
        /// after each enemy Unit.
        #[test]
        fn r767000_the_script_kept_for_v0_5_9f_hits_the_enemy_hero_too() {
            crate::under_kept(ID, script_v0_5_9f(), || {
                let mut s = flak_on(false, "s005-kept-9e", json!({ "field": [wall()] }));
                let target = s.unit_at(P2, 1).id;
                s.activate(ID, json!({}));
                let got = hits(s.last_events());
                let round: Vec<String> = vec![target, "hero-p2".to_string()];
                let expected: Vec<String> = (0..ROUNDS).flat_map(|_| round.clone()).collect();
                let targets: Vec<String> = got.into_iter().map(|(target, _)| target).collect();
                assert_eq!(targets, expected);
            });
        }

        /// R767000: the script kept for the replays of v0.5.11 and before, under that version's declared
        /// `damage`, dealt it to each enemy Unit and then the enemy hero, round after round, rolling
        /// nothing.
        #[test]
        fn r767000_the_script_kept_for_v0_5_11_deals_its_declared_damage_and_rolls_nothing() {
            // v0.5.11's number, which its definition declared and the replay carries.
            const DAMAGE: i32 = 2;
            let old = |def: &mut CardDef| {
                let declared: Vec<Param> = serde_json::from_value(json!([{
                    "key": "damage", "base": DAMAGE, "radiant": 4, "better": "up", "step": 1, "min": 1
                }]))
                .expect("v0.5.11's damage");
                def.params.get_or_insert_with(Vec::new).extend(declared);
            };
            crate::under_kept_as(ID, script_v0_5_11(), old, || {
                let mut s = flak_on(false, "s053-kept", json!({ "field": [wall()] }));
                let target = s.unit_at(P2, 1).id;
                let cursor = s.state().rng_cursor;
                s.activate(ID, json!({}));
                let round = [(target, DAMAGE), ("hero-p2".to_string(), DAMAGE)];
                let expected: Vec<(String, i32)> = (0..ROUNDS).flat_map(|_| round.clone()).collect();
                assert_eq!(hits(s.last_events()), expected);
                assert_eq!(s.state().rng_cursor, cursor, "no roll drawn");
            });
        }

        #[test]
        fn r1862001_r2521001_each_enemy_unit_takes_its_own_1_to_3_and_the_hero_none() {
            let (counts, differed) = over_the_seeds(false, BASE_RANGE, 0);
            assert!(differed, "two enemies can take different amounts");
            let (lo, hi) = BASE_RANGE;
            let [one, two, three] = [lo, lo + 1, hi].map(|amount| counts.get(&amount).copied().unwrap_or(0));
            assert_eq!(one + two + three, counts.values().sum::<usize>(), "1 to 3 only: {counts:?}");
            assert!(one > two && two > three && three > 0, "60/30/10: {counts:?}");
        }

        #[test]
        fn r2521001_r1438_given_lucky_rolls_each_target_again_keeping_the_higher() {
            for seed in 0..SEEDS {
                let mut s = flak_on(false, &format!("s053-given-{seed}"), walls());
                give_lucky(&mut s, ID, 1);
                fire(&mut s, ID, BASE_RANGE, 1);
            }
        }

        #[test]
        fn r384_once_a_turn_a_second_use_refused() {
            let mut s = flak_on(false, "s053-once", json!({}));
            fire(&mut s, ID, BASE_RANGE, 0);
            s.expect_refused_with(|s| s.activate(ID, json!({})), "already been used");
            next_turn_of(&mut s, P1);
            fire(&mut s, ID, BASE_RANGE, 0);
        }

        #[test]
        fn r1086_r1852001_an_equipped_host_takes_the_activate_and_is_no_starship_piece() {
            let mut s = weapon_board(
                "s053-equip",
                json!({ "hand": [ID], "field": [VANILLA] }),
                walls(),
            );
            let host = s.unit_at(P1, 1).id;
            s.play(ID, json!({ "zone": 1, "equip": true }));
            let fused = s.card(host.as_str()).clone();
            assert!(!unit_has(s.state(), &fused, KeywordKind::StarshipPiece));
            fire(&mut s, host.as_str(), BASE_RANGE, 0);
        }

        #[test]
        fn r386_r2521001_an_upgrade_of_most_rolls_past_the_third_at_the_thirds_weight() {
            let mut seen: IndexSet<i32> = IndexSet::new();
            for seed in 0..SEEDS {
                let mut s = flak_on(false, &format!("s053-most-{seed}"), walls());
                let most = crate::upgrade_number(&mut s, ID, "most");
                seen.extend(fire(&mut s, ID, (BASE_RANGE.0, most), 0).into_iter().map(|(_, a)| a));
            }
            assert!(seen.contains(&(BASE_RANGE.1 + 1)), "{seen:?}");
        }

        #[test]
        fn r386_r2521001_an_upgrade_of_least_lifts_the_low_end() {
            let mut s = flak_on(false, "s053-least", walls());
            let least = crate::upgrade_number(&mut s, ID, "least");
            let got = fire(&mut s, ID, (least, BASE_RANGE.1), 0);
            assert!(got.iter().all(|(_, amount)| *amount >= least), "{got:?}");
        }

        #[test]
        fn r2521001_a_lift_of_least_past_most_rolls_between_the_two() {
            for seed in 0..SEEDS {
                let mut s = flak_on(false, &format!("s053-crossed-{seed}"), walls());
                step_param(s.card_mut(ID), "least", BASE_RANGE.1);
                // least 4 and most 3: between 3 and 4, 3 first.
                fire(&mut s, ID, (BASE_RANGE.1, BASE_RANGE.1 + 1), 0);
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1862001_r2521001_each_enemy_rolls_2_to_4_twice_keeping_the_higher() {
            let (counts, differed) = over_the_seeds(true, RADIANT_RANGE, RADIANT_LUCKY);
            assert!(differed, "two enemies can take different amounts");
            let total: usize = counts.values().sum();
            let (lo, hi) = RADIANT_RANGE;
            assert!(counts.keys().all(|amount| (lo..=hi).contains(amount)), "{counts:?}");
            assert!((lo..=hi).all(|amount| counts.contains_key(&amount)), "{counts:?}");
            // Lucky 1 lifts the odds off the low end: a 2 needs both rolls to be 2s (36%, not 60%).
            let low = counts.get(&RADIANT_RANGE.0).copied().unwrap_or(0);
            assert!(2 * low < total, "{counts:?}");
        }

        #[test]
        fn r2521001_a_radiant_first_hit_is_the_higher_of_its_two_rolls() {
            for seed in 0..SEEDS {
                let mut s = flak_on(true, &format!("s053-radiant-pair-{seed}"), walls());
                let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
                let first = printed_roll(&mut rng, RADIANT_RANGE);
                let second = printed_roll(&mut rng, RADIANT_RANGE);
                let got = fire(&mut s, ID, RADIANT_RANGE, RADIANT_LUCKY);
                assert_eq!(got[0].1, first.max(second));
            }
        }

        #[test]
        fn its_health_is_doubled() {
            let s = flak_on(true, "s053-health", json!({}));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (0, RADIANT_HEALTH));
        }
    }
}
