//! SPEC §8.14 FvH #66 Fuzzyfruit ((1) Spell, Fruit, Common), a card of the held Felinors vs Humans set
//! (R5235000).
//!   Base:    "Summon a random (1) Cost Felinor and give it +{stats}/+{stats}." (stats 1)
//!   Radiant: the same words (stats 2, R5263001).
//!
//! `summon_random` of one non-token Unit printed at (1) whose catalog tags hold Felinor, of the sets that ship,
//! into the leftmost open unit zone (R64), running no Cry (R1); then a permanent `buff` of +stats/+stats on
//! what it summoned (`summoned_so_far`, M #58's shape). A full row summons nothing and nothing is buffed
//! (R129). Both faces print the same words and the summon is not Radiant, as the designer wrote; the
//! designer's two faces were alike, so the Radiant face's buff is +2/+2 (R276, R5263001). The (1) is a
//! price, no declared number. Built by S07 (#5257).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-066";

/// The printed price of the Felinor the Spell summons.
const FELINOR_COST: i32 = 1;

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            let stats = param(ctx, "stats");
            vec![
                summon_random(json_as(json!({
                    "query": { "type": "Unit", "cost": FELINOR_COST, "tags": ["Felinor"] },
                }))),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(|ctx: &mut EffectContext<'_>| summoned_so_far(ctx)),
                    each: Arc::new(move |instance_id: &str| {
                        buff(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                            "attack": stats,
                            "health": stats,
                        })))
                    }),
                }),
            ]
        })),
        ..Script::default()
    };
    // The same script: both faces print the same words, each face its own `stats`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #66 Fuzzyfruit — SPEC §8.14 row 66.
#[cfg(test)]
mod tests {
    use super::{FELINOR_COST, ID};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: a card to hold
    const VANILLA: &str = "stand-in (1) 4/4";

    fn board(radiant: bool, seed: &str, field: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": field, "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    /// The Units p1 summoned in the last step, by `summoned`.
    fn summoned(s: &Scenario) -> Vec<CardInstance> {
        s.last_events_of(GameEventType::Summoned)
            .into_iter()
            .filter_map(|event| match event {
                GameEvent::Summoned {
                    player: P1,
                    instance_id,
                    ..
                } => Some(s.card(instance_id.as_str()).clone()),
                _ => None,
            })
            .collect()
    }

    /// The face's `stats`: 1 on the base face, 2 on the Radiant (R5263001).
    fn summons_a_one_cost_felinor_with_its_buff(radiant: bool, stats: i32) {
        for seed in ["fuzzyfruit-a", "fuzzyfruit-b", "fuzzyfruit-c"] {
            let mut s = board(radiant, seed, json!([]));
            s.play(ID, json!({}));
            let units = nonempty(summoned(&s), "the Felinor it summoned");
            assert_eq!(units.len(), 1);
            for unit in &units {
                let def = def_of(Some(s.state()), &unit.def_id);
                assert_eq!(def.type_, CardType::Unit);
                assert_eq!(query_cost(def), FELINOR_COST, "{} is printed at (1)", def.id);
                assert!(
                    def.tags.contains(&Tag::Felinor),
                    "{} prints the Felinor tag",
                    def.id
                );
                assert!(!def.token, "{} is no token", def.id);
                assert!(!unit.radiant, "the summon is not Radiant on either face");
                let buffs = s.card(unit).buffs;
                assert_eq!(
                    (buffs.attack, buffs.health),
                    (stats, stats),
                    "a permanent +{stats}/+{stats}"
                );
            }
        }
    }

    mod base {
        use super::*;

        #[test]
        fn summons_a_random_1_cost_felinor_and_gives_it_plus_1_plus_1() {
            summons_a_one_cost_felinor_with_its_buff(false, 1);
        }

        #[test]
        fn a_full_unit_row_summons_and_buffs_nothing() {
            let field = json!([VANILLA, VANILLA, VANILLA, VANILLA, VANILLA]);
            let mut s = board(false, "fuzzyfruit-full", field);
            s.play(ID, json!({}));
            assert!(summoned(&s).is_empty());
            assert_eq!(s.last_count(GameEventType::Buffed), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5263001_summons_a_random_1_cost_felinor_not_radiant_and_gives_it_plus_2_plus_2() {
            summons_a_one_cost_felinor_with_its_buff(true, 2);
        }
    }
}
