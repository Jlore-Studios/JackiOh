//! S #153 Stimmycheck (SPEC §8.10 row S 153; §6.1, §6.3 Mana, R62, R78, R350, R440, R1440, R1848006):
//! (0) Spell, Legendary.
//!   Base:    "Gain {mana} mana. Inflation: each card in your hand and deck has a {chance}% chance to
//!            cost (1) more. / At the end of this turn, add a Stimmycheck to your hand. Its Inflation
//!            chance goes up by 10%." — 1, 15.
//!   Radiant: "Lucky 1" and the same text, the copy a Radiant Stimmycheck.
//!
//! Engine: `gain_mana` (this turn's mana, §6.3), then `inflate` (R1848006): one luck-based roll per
//! card of its caster's hand, in order, then deck, from the top, a hit adding `INFLATION_COST` to its
//! `costMod`, an X card and the resolving Stimmycheck passed by, silent (R440); a Stimmycheck carrying
//! its own chance (`INFLATION_CHANCE_KEY`) rolls at that instead of `chance`. "At the end of this
//! turn" is a delayed add at the end of the turn it is cast on, whoever's (R350, ST-S14-11), not an
//! End of turn trigger, the card being in the graveyard by then: `delay` re-enters this script's
//! `resume` step (R62), whose data carries the copy's chance, 10 higher and never past 100
//! (`next_inflation_chance`), written into the copy's memory as `add_to_hand` makes it.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-153";

/// R62: the step the end-of-turn delayed add re-enters (§10.6: `script.resume[step]`).
const COPY_STEP: &str = "copy";

/// R2476000: in its owner's hand, after "15% chance" (its `chance` as it stands, R386), the chance its
/// Inflation rolls at: the one it carries from the Stimmycheck that added it (`inflation_chance_of`,
/// the cry's own read), else the printed one. Hand only, so its owner alone reads it.
fn preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if ctx.zone != ConditionZone::Hand {
            return vec![];
        }
        let printed = param(&ctx, "chance");
        vec![count_preview(&format!("{printed}% chance"), inflation_chance_of(Some(ctx.self_), printed))]
    })
}

fn stimmycheck(radiant: bool) -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let printed = param(&*ctx, "chance");
            let chance = inflation_chance_of(ctx.self_.as_ref(), printed);
            vec![
                gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") }))),
                inflate(json_as(json!({ "chance": printed }))),
                delay(json_as(json!({
                    "at": { "phase": "end", "player": THIS_TURN },
                    "step": COPY_STEP,
                    "hook": RESUME_HOOK,
                    "data": { INFLATION_CHANCE_KEY: next_inflation_chance(chance) },
                }))),
            ]
        })),
        resume: IndexMap::from([(
            COPY_STEP,
            hook(move |ctx| {
                let chance = ctx.data.get(INFLATION_CHANCE_KEY).cloned().unwrap_or(Value::Null);
                vec![add_to_hand(json_as(json!({
                    "defId": ID,
                    "radiant": radiant,
                    "memory": { INFLATION_CHANCE_KEY: chance },
                })))]
            }),
        )]),
        preview: Some(preview()),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: stimmycheck(false),
        radiant: stimmycheck(true),
    }
}

// S #153 Stimmycheck — SPEC §8.10 row S 153, BUILD M12 row S 153 (R1848006).
#[cfg(test)]
mod tests {
    use jackioh_engine::prelude::INFLATION_CHANCE_KEY;
    use jackioh_engine::testkit::*;

    use crate::{degrade_number, filler, scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;

    const STIMMY: &str = "stinkyscape-153";
    const NONO: &str = "stinkyscape-099"; // (X) Spell.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005";

    const SEEDS: usize = 20;

    fn stimmy_board(seed: &str, radiant: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": STIMMY, "radiant": radiant }, VANILLA, VANILLA, VANILLA, NONO, FILLER],
                "library": filler(6),
            },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }))
    }

    fn cost_mods(s: &Scenario) -> Vec<i32> {
        let side = &s.state().players[P1];
        side.hand
            .iter()
            .chain(side.library.iter())
            .filter(|card| card.def_id != NONO)
            .map(|card| card.cost_mod)
            .collect()
    }

    fn copy_chance(s: &Scenario) -> Option<i64> {
        s.hand(P1)
            .into_iter()
            .find(|card| card.def_id == STIMMY)
            .and_then(|card| card.memory.get(INFLATION_CHANCE_KEY).and_then(Value::as_i64))
    }

    fn hits_over_seeds(radiant: bool) -> usize {
        (0..SEEDS)
            .map(|i| {
                let mut s = stimmy_board(&format!("stimmy-{i}"), radiant);
                s.play(STIMMY, json!({}));
                cost_mods(&s).into_iter().filter(|cost_mod| *cost_mod == INFLATION_COST).count()
            })
            .sum()
    }

    /// R2476000: in hand, after "15% chance", the chance it rolls at: 15 printed, 25 once it carries 25,
    /// and the copy its play adds carries 35.
    #[test]
    fn r2476000_in_hand_it_previews_the_inflation_chance_it_carries() {
        for radiant in [false, true] {
            let mut s = stimmy_board("stimmy-preview", radiant);
            let stimmy = s.card(STIMMY).id.clone();
            assert_eq!(crate::preview_seen(&s, P1, &stimmy), vec![("15% chance".to_string(), 15, None)]);
            s.card_mut(STIMMY).memory.insert(INFLATION_CHANCE_KEY.to_string(), json!(25));
            assert_eq!(crate::preview_seen(&s, P1, &stimmy), vec![("15% chance".to_string(), 25, None)]);
            assert!(crate::preview_seen(&s, PlayerId::P2, &stimmy).is_empty(), "radiant {radiant}");
            s.play(STIMMY, json!({}));
            s.end_turn();
            assert_eq!(copy_chance(&s), Some(35), "radiant {radiant}");
        }
    }

    mod base {
        use super::*;

        #[test]
        fn at_0_it_gains_1_mana() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [STIMMY, FILLER], "mana": 0 },
                "p2": { "hand": [FILLER] },
            }));

            s.play(STIMMY, json!({}));

            assert_eq!(s.state().players[P1].mana.current, 1);
        }

        #[test]
        fn r1848006_hits_and_misses_over_the_seeds_and_never_an_x_card() {
            let mut seen: Vec<i32> = Vec::new();
            for i in 0..SEEDS {
                let mut s = stimmy_board(&format!("stimmy-{i}"), false);
                s.play(STIMMY, json!({}));
                for cost_mod in cost_mods(&s) {
                    assert!(cost_mod == 0 || cost_mod == INFLATION_COST, "seed {i}: {cost_mod}");
                    seen.push(cost_mod);
                }
                let nono = s.card(NONO).clone();
                assert_eq!(nono.cost_mod, 0, "R65: an X card is passed by");
            }
            assert!(seen.contains(&0) && seen.contains(&INFLATION_COST), "hits and misses");
        }

        #[test]
        fn r1848006_nothing_is_announced() {
            let seed = seed_where("stimmy-silent", |seed| {
                let mut s = stimmy_board(seed, false);
                s.play(STIMMY, json!({}));
                cost_mods(&s).contains(&INFLATION_COST)
            });
            let mut s = stimmy_board(&seed, false);

            s.play(STIMMY, json!({}));

            assert!(cost_mods(&s).contains(&INFLATION_COST), "a card was hit");
            assert!(
                !s.last_events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::CostChanged { .. }))
            );
        }

        #[test]
        fn r1848006_the_copy_rolls_at_25_and_its_copy_at_35() {
            let mut s = stimmy_board("stimmy-copies", false);
            s.play(STIMMY, json!({}));
            assert_eq!(copy_chance(&s), None, "the copy waits for the turn's end");

            s.end_turn();
            assert_eq!(copy_chance(&s), Some(25));
            s.end_turn();
            let copy = s.hand(P1).into_iter().find(|card| card.def_id == STIMMY).unwrap();
            assert!(!copy.radiant);
            s.play(&copy, json!({}));
            s.end_turn();

            assert_eq!(copy_chance(&s), Some(35));
        }

        #[test]
        fn mana_and_chance_read_through_param() {
            let mut s = stimmy_board("stimmy-param", false);
            assert_eq!(upgrade_number(&mut s, STIMMY, "mana"), 2);
            assert_eq!(degrade_number(&mut s, STIMMY, "chance"), 19);
            let before = s.state().players[P1].mana.current;

            s.play(STIMMY, json!({}));
            assert_eq!(s.state().players[P1].mana.current, before + 2);
            s.end_turn();

            assert_eq!(copy_chance(&s), Some(29), "the chance it rolled at, 10 higher");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn lucky_1_spares_more_and_the_copy_is_radiant() {
            assert!(hits_over_seeds(true) < hits_over_seeds(false));

            let mut s = stimmy_board("stimmy-radiant", true);
            s.play(STIMMY, json!({}));
            s.end_turn();
            let copy = s.hand(P1).into_iter().find(|card| card.def_id == STIMMY).unwrap();
            assert!(copy.radiant);
            assert_eq!(copy_chance(&s), Some(25));
        }
    }
}
