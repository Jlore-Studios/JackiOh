// R2521001 (issue #2521): `damage_all`'s weighted roll, `odds` and `lucky` (Stinkyscape #53, #57, #61
// Flak). With `odds`, each target rolls its own amount just before its hit, the units in `cardsInScope`
// order and then the heroes, as Hearthstone's Lightning Storm rolls each minion apart; Lucky X rolls each
// X more times and keeps the highest (§6.1). Without `odds` the sweep draws nothing. The fixture ids
// (`odds-`, indexed from 252100) collide with no other file's.

use std::collections::BTreeSet;

use jackioh_engine::effects::{DamageAllArgs, DamageOdds, damage_all};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{both, json_of, new_game, put, round_trip, run_effect, slot};

const ODDS: [DamageOdds; 3] = [
    DamageOdds {
        amount: 1,
        weight: 60,
    },
    DamageOdds {
        amount: 2,
        weight: 30,
    },
    DamageOdds {
        amount: 3,
        weight: 10,
    },
];
/// The test's own reading of those odds, independent of the engine's walk.
const ODDS_TOTAL: i32 = 100;
const ONE_BELOW: i32 = 60;
const TWO_BELOW: i32 = 90;
const RADIANT_LUCKY: i32 = 1;
/// What the plain sweep (no odds) deals.
const PLAIN_AMOUNT: i32 = 2;
const SEEDS: usize = 60;

/// A 0/4 Unit: "Activate: Deal a random 1 to 3 damage to each enemy (60%, 30%, 10%, lowest first)."
/// Radiant: the same with Lucky 1.
const FLAK: &str = "odds-flak";
/// A 0/40 Unit with no text: nothing here brings it down, so every sweep hits it.
const WALL: &str = "odds-wall";
/// A 1/1 Unit with no text, to draw.
const PLAIN: &str = "odds-plain";

fn card(index: u32, id: &str, stats: (i32, i32), radiant_keywords: Value) -> CardDef {
    let (attack, health) = stats;
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (damage odds fixture)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "attack": attack, "health": health, "keywords": [], "text": id },
        "radiant": { "attack": 2 * attack, "health": 2 * health, "keywords": radiant_keywords, "text": id },
    }))
}

/// The flak's Activate, written as a card script writes it: the odds and the card's Lucky stated.
fn volley() -> ActivationDecl {
    ActivationDecl {
        id: "volley".to_string(),
        label: "Deal a random 1 to 3 damage to each enemy".to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| {
            let lucky = ctx.live_self().map_or(0, |me| lucky_on(&*ctx.state, me));
            vec![damage_all(json_as(json!({
                "amount": ODDS[0].amount,
                "odds": ODDS,
                "lucky": lucky,
                "side": "enemy",
                "heroes": true,
            })))]
        }),
    }
}

fn register_odds_fixtures() {
    crate::rules::fixtures::harness::setup_catalog();
    let mut catalog: CardDefs = registered_catalog().clone();
    for def in [
        card(
            252100,
            FLAK,
            (0, 4),
            json!([{ "kind": "Lucky", "n": RADIANT_LUCKY }]),
        ),
        card(252101, WALL, (0, 40), json!([])),
        card(252102, PLAIN, (1, 1), json!([])),
    ] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts: IndexMap<String, CardScripts> = registered_scripts().clone();
    scripts.insert(
        FLAK.to_string(),
        both(Script {
            activations: vec![volley()],
            ..Script::default()
        }),
    );
    register_scripts(scripts);
}

/// `p1`'s turn 9: the flak (Radiant or not) against two walls, plain cards to draw, and no turn that
/// ends by itself (R345).
fn board(seed: &str, radiant: bool) -> Scenario {
    register_odds_fixtures();
    let mut s = scenario(json!({
        "seed": seed,
        "p1": { "field": [{ "def": FLAK, "radiant": radiant }], "library": [PLAIN, PLAIN, PLAIN] },
        "p2": { "field": [WALL, WALL], "library": [PLAIN, PLAIN, PLAIN] },
    }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn damage_hits(events: &[GameEvent]) -> Vec<(String, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage {
                target_id, amount, ..
            } => Some((target_id.clone(), *amount)),
            _ => None,
        })
        .collect()
}

fn read_roll(rng: &mut Rng) -> i32 {
    let at = rng.int(ODDS_TOTAL);
    if at < ONE_BELOW {
        1
    } else if at < TWO_BELOW {
        2
    } else {
        3
    }
}

/// What the flak's Activate deals on `s`, read off a copy of the match rng: p2's walls in lane order,
/// then p2's hero, each its own roll with `lucky` more kept at the highest; and the cursor left.
fn foreseen(s: &Scenario, lucky: i32) -> (Vec<(String, i32)>, u32) {
    let mut targets: Vec<String> = (1..=UNIT_ZONES)
        .filter_map(|lane| s.unit(P2, lane))
        .map(|unit| unit.id)
        .collect();
    targets.push("hero-p2".to_string());
    let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
    let hits: Vec<(String, i32)> = targets
        .into_iter()
        .map(|target| (target, rng.lucky(lucky, read_roll, Ord::max)))
        .collect();
    (hits, rng.cursor())
}

/// Activates the flak on `s` and holds every hit to what `foreseen` read; the hits.
fn volley_as_foreseen(s: &mut Scenario, lucky: i32) -> Vec<(String, i32)> {
    let (expected, cursor) = foreseen(s, lucky);
    let health = s.state().players[P2].hero.health;
    s.activate(FLAK, json!({}));
    let hits = damage_hits(s.last_events());
    assert_eq!(hits, expected, "one roll per target, in the order they are hit");
    assert_eq!(
        s.state().rng_cursor,
        cursor,
        "1 + Lucky draws per target, and no other"
    );
    let to_hero: i32 = hits
        .iter()
        .filter(|(target, _)| target == "hero-p2")
        .map(|(_, amount)| amount)
        .sum();
    s.expect_health(P2, health - to_hero);
    hits
}

/// p2's two walls and nothing else, in a game in its setup: `run_effect` runs a verb straight on it.
fn wall_game(seed: &str) -> (GameState, [CardInstance; 2]) {
    let mut state = new_game(seed, None);
    register_odds_fixtures();
    let walls = [1, 2].map(|lane| put(&mut state, WALL, slot(P2, Row::Units, lane), json!({})));
    (state, walls)
}

mod r2521001_each_target_rolls_its_own_amount {
    use super::*;

    #[test]
    fn r2521001_each_target_rolls_just_before_its_hit_the_units_in_order_then_the_hero() {
        let mut seen: BTreeSet<i32> = BTreeSet::new();
        let mut walls_differed = false;
        let mut hero_differed = false;
        for seed in 0..SEEDS {
            let mut s = board(&format!("r2521001-own-{seed}"), false);
            let hits = volley_as_foreseen(&mut s, 0);
            assert_eq!(hits.len(), 3, "two walls and the hero");
            walls_differed |= hits[0].1 != hits[1].1;
            hero_differed |= hits[2].1 != hits[0].1;
            seen.extend(hits.iter().map(|(_, amount)| *amount));
        }
        assert!(
            walls_differed,
            "two enemies can take different amounts in one sweep"
        );
        assert!(hero_differed, "the hero rolls apart from the units");
        assert_eq!(
            seen,
            BTreeSet::from([1, 2, 3]),
            "every face of the odds comes up, none other"
        );
    }

    #[test]
    fn r2521001_over_the_seeds_the_low_end_comes_up_most_and_the_high_end_least() {
        let mut counts = [0_usize; 3];
        for seed in 0..SEEDS {
            let mut s = board(&format!("r2521001-weights-{seed}"), false);
            s.activate(FLAK, json!({}));
            for (_, amount) in damage_hits(s.last_events()) {
                counts[(amount - 1) as usize] += 1;
            }
        }
        assert!(
            counts[0] > counts[1] && counts[1] > counts[2] && counts[2] > 0,
            "60/30/10: {counts:?}"
        );
    }

    #[test]
    fn r2521001_a_second_sweep_rolls_every_target_anew() {
        let mut rerolled = false;
        for seed in 0..SEEDS {
            let (mut state, walls) = wall_game(&format!("r2521001-anew-{seed}"));
            let sweep = || {
                damage_all(json_as(json!({
                    "amount": ODDS[0].amount,
                    "odds": ODDS,
                    "side": "enemy",
                })))
            };
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let first = damage_hits(&run_effect(&mut state, sweep(), P1));
            let second = damage_hits(&run_effect(&mut state, sweep(), P1));
            let ids: Vec<String> = walls.iter().map(|wall| wall.id.clone()).collect();
            let expected = |rng: &mut Rng| -> Vec<(String, i32)> {
                ids.iter().map(|id| (id.clone(), read_roll(rng))).collect()
            };
            assert_eq!(first, expected(&mut rng));
            assert_eq!(second, expected(&mut rng));
            assert_eq!(state.rng_cursor, rng.cursor());
            rerolled |= first[0].1 != second[0].1;
        }
        assert!(rerolled, "a wall's second hit is not its first one again");
    }
}

mod r2521001_lucky_rolls_each_target_again {
    use super::*;

    #[test]
    fn r2521001_the_radiant_faces_lucky_1_rolls_each_target_twice_keeping_the_higher() {
        let mut seen: BTreeSet<i32> = BTreeSet::new();
        for seed in 0..SEEDS {
            let mut s = board(&format!("r2521001-lucky-{seed}"), true);
            let hits = volley_as_foreseen(&mut s, RADIANT_LUCKY);
            seen.extend(hits.iter().map(|(_, amount)| *amount));
        }
        assert_eq!(seen, BTreeSet::from([1, 2, 3]));
    }

    #[test]
    fn r2521001_r1438_given_lucky_adds_to_each_targets_rolls() {
        for seed in 0..SEEDS {
            let mut s = board(&format!("r2521001-given-{seed}"), true);
            s.card_mut(FLAK).granted_keywords.push(Keyword::Lucky { n: 1 });
            volley_as_foreseen(&mut s, RADIANT_LUCKY + 1);
        }
    }

    #[test]
    fn r2521001_a_lucky_roll_is_never_below_the_plain_one_on_the_same_seed() {
        for seed in 0..SEEDS {
            let seed = format!("r2521001-paired-{seed}");
            let mut plain = board(&seed, false);
            let mut lucky = board(&seed, true);
            plain.activate(FLAK, json!({}));
            lucky.activate(FLAK, json!({}));
            // The first target's first roll is the same draw on both; the lucky one keeps the higher.
            let (plain_first, lucky_first) = (
                damage_hits(plain.last_events())[0].1,
                damage_hits(lucky.last_events())[0].1,
            );
            assert!(lucky_first >= plain_first, "{lucky_first} against {plain_first}");
        }
    }
}

mod r2521001_without_odds_nothing_changes {
    use super::*;

    #[test]
    fn r2521001_a_sweep_without_odds_deals_its_amount_and_draws_nothing() {
        for args in [
            json!({ "amount": PLAIN_AMOUNT, "side": "enemy", "heroes": true }),
            // Lucky without odds has no roll to repeat.
            json!({ "amount": PLAIN_AMOUNT, "side": "enemy", "heroes": true, "lucky": 3 }),
            // Odds with nothing to weigh draw nothing and fall back on the amount.
            json!({ "amount": PLAIN_AMOUNT, "side": "enemy", "heroes": true, "odds": [] }),
        ] {
            let (mut state, walls) = wall_game("r2521001-plain");
            let cursor = state.rng_cursor;
            let hits = damage_hits(&run_effect(&mut state, damage_all(json_as(args.clone())), P1));
            assert_eq!(
                hits,
                vec![
                    (walls[0].id.clone(), PLAIN_AMOUNT),
                    (walls[1].id.clone(), PLAIN_AMOUNT),
                    ("hero-p2".to_string(), PLAIN_AMOUNT),
                ],
                "{args}"
            );
            assert_eq!(state.rng_cursor, cursor, "{args}: no draw");
        }
    }

    #[test]
    fn r2521001_the_args_without_odds_write_no_new_key_so_old_states_read_and_write_the_same() {
        let plain: DamageAllArgs = json_as(json!({ "amount": PLAIN_AMOUNT, "side": "enemy" }));
        assert!(plain.odds.is_none() && plain.lucky.is_none());
        assert_eq!(
            json_of(&plain),
            json!({ "amount": PLAIN_AMOUNT, "side": "enemy" })
        );
    }

    #[test]
    fn r2521001_the_odds_and_lucky_read_as_a_script_writes_them_and_round_trip() {
        let args: DamageAllArgs = json_as(json!({
            "amount": 1,
            "odds": [{ "amount": 1, "weight": 60 }, { "amount": 2, "weight": 30 }, { "amount": 3, "weight": 10 }],
            "lucky": RADIANT_LUCKY,
            "side": "enemy",
            "heroes": true,
        }));
        assert_eq!(args.odds.as_deref(), Some(&ODDS[..]));
        assert_eq!(args.lucky, Some(RADIANT_LUCKY));
        assert_eq!(round_trip(&args), args);
        assert_eq!(json_of(&args)["odds"][2], json!({ "amount": 3, "weight": 10 }));
    }
}
