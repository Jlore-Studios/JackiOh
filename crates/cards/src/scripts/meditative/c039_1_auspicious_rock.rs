//! Meditative #39.1 Auspicious Rock (SPEC §8.8 row 39.1). (0) Spell, CN, Token (printed Rare).
//!
//!   Base:    "Lose {life} health. Add one of these to your hand at random: Dud (20%), Jade (70%)
//!            or Red Jade (10%)." — life 2
//!   Radiant: "Lucky 2
//!            Lose {life} health. Add one of these to your hand at random: Dud (20%), Jade (70%)
//!            or Red Jade (10%)." — life 2, Lucky 2
//!
//! Engine: `lose_health` on your hero (R18: no pipeline, no Armor, so nothing stops it), then one
//! weighted roll (ME-WEIGHTED-ROLL: `add_rolled` over `AUSPICIOUS_ROCK_ODDS`, R960, MD-C1). The
//! token rolled (M #39.2 to #39.4) lands in your hand uncast, on its base face; a burned roll is
//! lost (§2.4). Lucky is the Rock's printed Lucky — 2 on the Radiant face — each extra roll kept
//! if it is better, ranked Dud < Jade < Red Jade. A player's Luck (ME-LUCK, M #40) joins in MB11,
//! not here. "When played" on a Spell is its resolution.
//! Tunes: life 2 ↓; Lucky 2 ↑, on the Radiant face only.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-039-1";

/// The table the Rock rolls on (R960): Dud < Jade < Red Jade, worst to best.
pub const ROCK_TABLE: &str = "auspiciousRock";

fn rock_cry() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![
                lose_health(json_as(
                    json!({ "player": "self", "amount": param(&*ctx, "life") }),
                )),
                add_rolled(json_as(json!({ "table": ROCK_TABLE }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // Both faces run this one script: the Radiant face's Lucky 2 is its printed Lucky, which
    // `add_rolled` reads off the running face, and `life` is 2 on both.
    let base = rock_cry();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Meditative #39.1 Auspicious Rock — SPEC §8.8 row 39.1, BUILD M10 row M 39.1: "Your hero loses 2
// health (R18: Armor and a hero immunity do not stop it), then one roll by `AUSPICIOUS_ROCK_ODDS`
// adds a base Dud, Jade or Red Jade to your hand uncast (MD-C1), the hand cap burning it; over many
// seeds the odds hold at 20, 70 and 10; M 40's Luck joins in MB11; life reads through `param();
// radiant Lucky 2: three rolls keeping the best (Dud < Jade < Red Jade)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const DUD: &str = "meditative-039-3";
    const JADE: &str = "meditative-039-2";
    const RED_JADE: &str = "meditative-039-4";

    const FILLER: &str = "core-008";

    /// The rolled token's rank, worst to best (R960): Dud < Jade < Red Jade.
    fn rank(def_id: &str) -> i64 {
        [DUD, JADE, RED_JADE]
            .iter()
            .position(|id| *id == def_id)
            .map_or(-1, |i| i as i64)
    }

    fn rolled(s: &Scenario) -> String {
        let mut hand: Vec<String> = s.hand(P1).iter().map(|card| card.def_id.clone()).collect();
        hand.retain(|def| def != FILLER);
        assert_eq!(hand.len(), 1, "one rolled token");
        hand[0].clone()
    }

    #[test]
    fn r18_loses_2_health_through_armor() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [ID, FILLER], "armor": 5 },
            "p2": { "hand": [FILLER] },
        }));
        s.play(ID, json!({}));

        // R18: lose health is no pipeline, so Armor never stops it — and no `damage` event fires.
        s.expect_health(P1, 28);
        assert_eq!(s.view(P1).you.hero.armor, 5, "the Armor is untouched");
    }

    #[test]
    fn r960_adds_one_base_dud_jade_or_red_jade() {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": "rock-roll",
            "p1": { "hand": [ID, FILLER] },
            "p2": { "hand": [FILLER] },
        }));

        s.play(ID, json!({}));

        let def = rolled(&s);
        assert!(rank(&def) >= 0, "{def} is a Dud, a Jade or a Red Jade");
        let card = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == def)
            .expect("the roll");
        assert!(!card.radiant, "the roll lands on its base face");
        // Uncast: it sits in the hand, and no Jade Counter moved for a Jade.
        assert!(rank(&def) != 1 || s.view(P1).you.jade.unwrap_or(0) == 0);
    }

    #[test]
    fn a_full_hand_burns_the_roll() {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": "rock-burn",
            "p1": { "hand": [ID, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER] },
        }));

        s.play(ID, json!({}));

        assert_eq!(s.hand(P1).len(), 10);
        let burned: Vec<String> = s
            .pile(P1, "graveyard")
            .into_iter()
            .map(|card| card.def_id)
            .collect();
        assert!(
            burned.iter().any(|def| rank(def) >= 0),
            "the roll burned: {burned:?}"
        );
    }

    #[test]
    fn r960_the_radiant_rock_never_rolls_worse_than_the_base_on_one_seed_and_sometimes_better() {
        crate::register_all();
        let mut worse_never = true;
        let mut better_once = false;
        for n in 0..30 {
            let seed = format!("rock-lucky-{n}");
            let mut base =
                scenario(json!({ "seed": seed, "p1": { "hand": [ID, FILLER] }, "p2": { "hand": [FILLER] } }));
            base.play(ID, json!({}));
            let mut lucky = scenario(
                json!({ "seed": seed, "p1": { "hand": [{ "def": ID, "radiant": true }, FILLER] }, "p2": { "hand": [FILLER] } }),
            );
            lucky.play(ID, json!({}));
            let (plain, gifted) = (rank(&rolled(&base)), rank(&rolled(&lucky)));
            assert!(plain >= 0 && gifted >= 0);
            worse_never &= gifted >= plain;
            better_once |= gifted > plain;
        }
        assert!(worse_never, "Lucky 2 keeps the best of three");
        assert!(better_once, "over 30 seeds some roll improves");
    }

    #[test]
    fn r18_a_hero_immunity_does_not_stop_it() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [ID, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        // MD-C21's hero immunity stops damage, and lose health is no damage (R18).
        s.state_mut().players[P1].mods.push(PlayerModifier {
            id: "immune".to_string(),
            expiry: ModifierExpiry::Never,
            kind: ModifierKind::HeroImmune,
        });
        s.play(ID, json!({}));
        s.expect_health(P1, 28);
    }

    #[test]
    fn r960_over_many_seeds_the_odds_hold_at_20_70_and_10() {
        crate::register_all();
        let odds: Vec<(&str, i32)> = AUSPICIOUS_ROCK_ODDS
            .iter()
            .map(|row| (row.def_id, row.percent))
            .collect();
        assert_eq!(odds, vec![(DUD, 20), (JADE, 70), (RED_JADE, 10)]);
        const ROLLS: usize = 600;
        let mut counts = [0usize; 3];
        for n in 0..ROLLS {
            let mut s = scenario(json!({
                "seed": format!("rock-odds-{n}"),
                "p1": { "hand": [ID, FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            s.play(ID, json!({}));
            counts[rank(&rolled(&s)) as usize] += 1;
        }
        // Each share within six points of its odds (over four standard deviations at 600 rolls).
        for (count, (def, percent)) in counts.iter().zip(odds) {
            let share = (*count * 100) as f64 / ROLLS as f64;
            assert!(
                (share - percent as f64).abs() <= 6.0,
                "{def}: {share}% against {percent}%"
            );
        }
    }

    #[test]
    fn life_reads_through_param() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [ID, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        set_param(s.card_mut(ID), "life", 5);
        s.play(ID, json!({}));
        s.expect_health(P1, 25);
    }

    #[test]
    fn r987_feng_shuis_luck_joins_the_roll() {
        // M #40's Luck 1 gives the base Rock a second roll, keeping the better.
        crate::register_all();
        const FENG_SHUI: &str = "meditative-040";
        let mut worse_never = true;
        let mut better_once = false;
        for n in 0..30 {
            let seed = format!("rock-luck-{n}");
            let mut plain =
                scenario(json!({ "seed": seed, "p1": { "hand": [ID, FILLER] }, "p2": { "hand": [FILLER] } }));
            plain.play(ID, json!({}));
            let mut lucky = scenario(json!({
                "seed": seed,
                "p1": { "hand": [ID, FILLER], "backrow": [FENG_SHUI] },
                "p2": { "hand": [FILLER] },
            }));
            assert_eq!(luck_of(lucky.state(), P1), 1);
            lucky.play(ID, json!({}));
            let (without, with) = (rank(&rolled(&plain)), rank(&rolled(&lucky)));
            worse_never &= with >= without;
            better_once |= with > without;
        }
        assert!(worse_never, "Luck keeps the best of the rolls");
        assert!(better_once, "over 30 seeds some roll improves");
    }
}
