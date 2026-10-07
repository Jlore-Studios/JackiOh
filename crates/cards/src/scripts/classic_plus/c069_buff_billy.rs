//! C+ #69 Buff Billy (SPEC §8.7 row 69, R348, R386, R396; BUILD M9 row C+ 69). (X) Unit, Human, Rare.
//!   Base:    "This is a 3X/3X. / Cry: Upgrade this X times."
//!   Radiant: "This is a 7X/7X. / Cry: Upgrade this 2X times."
//!
//! The 3X/3X (7X/7X) is the catalog's `xStats` (E40), read by the engine's layer 1 off the X the card
//! was played for (at least 1, R348); a Recruit or a summon has no X and arrives 0/0 to die at the
//! state check. The Cry is X (2X) separate Upgrades of itself (R386), each drawn from what fits it on
//! the field: its X has resolved there, so only the stats and keyword rows remain (R396).

use jackioh_engine::effects::upgrade;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-069";

/// The Radiant face's "2X times".
const RADIANT_UPGRADES_PER_X: i32 = 2;

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| vec![upgrade(json_as(json!({ "target": { "of": "self" }, "times": ctx.x })))])),
        ..Script::default()
    };

    let radiant = Script {
        cry: Some(hook(|ctx| {
            vec![upgrade(json_as(json!({ "target": { "of": "self" }, "times": RADIANT_UPGRADES_PER_X * ctx.x })))]
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C+ #69 Buff Billy — SPEC §8.7 row 69, BUILD M9 row C+ 69: X at least 1 (R348), chosen with the play;
// a 3X/3X from its `xStats`; its Cry Upgrades it X times, each its own draw from what fits it now
// (R386) — the stats split or an R21 keyword it lacks, never cost or X (it is an X-cost card, and on the
// field it costs its X, R396); the upgrades are `tuning`, kept by a copy (R57); summoned outside a play
// (a Recruit) it has X 0, arrives 0/0 and dies at the state check; radiant 7X/7X and 2X Upgrades.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const BILLY: &str = "classicplus-069";
    const FILLER: &str = "core-005";

    use crate::js;

    /// TS `run(s, effects)`: the effects applied through a context of p1's, then a state check.
    fn run(s: &mut Scenario, effects: Vec<Effect>) {
        let mut events: Vec<GameEvent> = Vec::new();
        let state = s.state_mut();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
            {
                let mut ctx = jackioh_engine::resolve::make_context(
                    &mut sink,
                    None,
                    jackioh_engine::resolve::HookOptions { controller: Some(P1), ..Default::default() },
                );
                jackioh_engine::resolve::apply_effects(&effects, &mut ctx);
            }
            jackioh_engine::state_check::state_check(&mut sink);
        }
        state.rng_cursor = rng.cursor();
    }

    /// The `upgraded` events, each as TS's `Extract<GameEvent, { type: "upgraded" }>` reads it: the
    /// card's id and the change's JSON.
    fn upgrades(events: &[GameEvent]) -> Vec<(String, Value)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Upgraded { instance_id, change, .. } => Some((instance_id.clone(), js(change))),
                _ => None,
            })
            .collect()
    }

    /// TS `billy(x, { radiant?, seed? })`: Buff Billy played into lane 1 for `x`.
    fn billy(x: i32, radiant: bool, seed: Option<&str>) -> (Scenario, CardInstance) {
        let mut options = json!({
            "p1": { "hand": [{ "def": BILLY, "radiant": radiant }, FILLER], "mana": 10 },
            "p2": { "hand": [FILLER] }
        });
        if let Some(seed) = seed {
            options["seed"] = json!(seed);
        }
        let mut s = scenario(options);
        s.play(BILLY, json!({ "zone": 1, "x": x }));
        let Some(unit) = s.unit(P1, 1) else {
            panic!("Buff Billy on the field");
        };
        (s, unit)
    }

    /// The stats the Upgrades added, summed from the events.
    fn stats_added(events: &[GameEvent]) -> (i64, i64) {
        upgrades(events).iter().fold((0, 0), |(attack, health), (_, change)| {
            if change["kind"] == json!("stats") {
                (
                    attack + change["attack"].as_i64().unwrap_or(0),
                    health + change["health"].as_i64().unwrap_or(0),
                )
            } else {
                (attack, health)
            }
        })
    }

    #[test]
    fn prints_an_x_unit_human_3x_3x_and_radiant_7x_7x() {
        crate::register_all();
        let def = crate::card_def(ID);
        let json = js(&def);
        assert_eq!(json["cost"], json!("X"));
        assert_eq!(json["type"], json!("Unit"));
        assert_eq!(json["tags"], json!(["Human"]));
        assert_eq!(js(&def.base.x_stats), json!({ "attack": 3, "health": 3 }));
        assert_eq!(js(&def.radiant.x_stats), json!({ "attack": 7, "health": 7 }));
        let CardScripts { base, radiant } = script();
        assert!(base.cry.is_some());
        assert!(radiant.cry.is_some());
    }

    mod base {
        use super::*;

        #[test]
        fn r348_x_is_chosen_with_the_play_1_up_to_your_mana_and_x_0_is_refused() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [BILLY, FILLER] }, "p2": { "hand": [FILLER] } }));
            let card = s.card(BILLY).id.clone();
            let mut xs: Vec<i64> = legal_actions(s.state(), P1)
                .iter()
                .map(js)
                .filter(|action| {
                    action["type"] == json!("play") && action["instanceId"] == json!(card) && action["zone"]["lane"] == json!(1)
                })
                .filter_map(|action| action["x"].as_i64())
                .collect();
            xs.sort();
            assert_eq!(xs, vec![1, 2, 3, 4]);
            s.expect_refused_with(|s| s.play(BILLY, json!({ "zone": 1, "x": 0 })), "X must be at least 1");
        }

        #[test]
        fn r386_played_for_x_2_it_is_a_6_6_and_its_cry_upgrades_it_twice_each_its_own_draw_landing_on_its_stats() {
            crate::register_all();
            let (mut s, unit) = billy(2, false, None);
            let events = upgrades(s.events());
            assert_eq!(events.len(), 2);
            assert!(events.iter().all(|(id, _)| *id == unit.id));
            let (attack, health) = stats_added(s.events());
            s.expect_stats(&unit.id, json!({ "attack": 6 + attack, "maxHealth": 6 + health }));
            assert_eq!(s.card(&unit.id).x, Some(2));
        }

        #[test]
        fn r386_r396_every_upgrade_is_the_stats_split_or_a_keyword_it_lacks_never_cost_or_x_on_the_field_it_costs_its_x() {
            crate::register_all();
            for n in 0..12 {
                let seed = format!("billy-{n}");
                let (s, unit) = billy(4, false, Some(&seed));
                let changes: Vec<Value> = upgrades(s.events()).into_iter().map(|(_, change)| change).collect();
                assert_eq!(changes.len(), 4);
                for change in &changes {
                    assert!(change["kind"] == json!("stats") || change["kind"] == json!("keyword"));
                    if change["kind"] == json!("stats") {
                        assert_eq!(change["attack"].as_i64().unwrap_or(0) + change["health"].as_i64().unwrap_or(0), 4);
                    }
                    if change["kind"] == json!("keyword") {
                        assert_eq!(change["added"], json!(true));
                    }
                }
                let live = s.card(&unit.id);
                assert!(js(&live.tuning)["x"].is_null());
                assert_eq!(live.cost_mod, 0);
                assert_eq!(cost_now(s.state(), live), 4);
            }
        }

        #[test]
        fn r21_a_keyword_it_gains_is_one_it_lacks_from_r21_s_pool_and_it_keeps_it() {
            crate::register_all();
            let mut checked = false;
            let mut n = 0;
            while n < 20 && !checked {
                let seed = format!("billy-kw-{n}");
                n += 1;
                let (s, unit) = billy(3, false, Some(&seed));
                let gained: Vec<Value> = upgrades(s.events())
                    .into_iter()
                    .filter(|(_, change)| change["kind"] == json!("keyword"))
                    .map(|(_, change)| change["keyword"]["kind"].clone())
                    .collect();
                if gained.is_empty() {
                    continue;
                }
                let distinct: IndexSet<String> = gained.iter().map(|kind| kind.to_string()).collect();
                assert_eq!(distinct.len(), gained.len());
                let has: Vec<Value> = s.stats(&unit.id).keywords.iter().map(|keyword| js(keyword)["kind"].clone()).collect();
                assert!(gained.iter().all(|kind| has.contains(kind)));
                checked = true;
            }
            assert!(checked);
        }

        #[test]
        fn r57_the_upgrades_are_its_tuning_and_a_copy_keeps_them() {
            crate::register_all();
            let (mut s, unit) = billy(3, false, None);
            let tuning = s.card(&unit.id).tuning.clone();
            assert!(tuning.is_some());
            run(
                &mut s,
                vec![jackioh_engine::effects::summon_copy(json_as(json!({
                    "of": { "of": "instance", "instanceId": unit.id },
                    "lane": 2
                })))],
            );
            let copy = s
                .unit(P1, 2)
                .or_else(|| s.pile(P1, "graveyard").into_iter().find(|card| card.def_id == BILLY));
            assert_eq!(copy.as_ref().map(|card| card.def_id.as_str()), Some(BILLY));
            assert_eq!(copy.as_ref().map(|card| js(&card.tuning)), Some(js(&tuning)));
        }

        #[test]
        fn b2_7_a_recruit_has_no_x_it_arrives_0_0_and_dies_at_the_state_check() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [FILLER], "library": [BILLY] }, "p2": { "hand": [FILLER] } }));
            run(&mut s, vec![jackioh_engine::effects::recruit(json_as(json!({ "filter": { "defId": BILLY } })))]);
            s.expect_in_zone(BILLY, "graveyard");
            assert!(s.unit(P1, 1).is_none());
        }

        #[test]
        fn r396_a_buff_billy_played_for_1_is_a_3_3_with_one_upgrade_costing_1_on_the_field() {
            crate::register_all();
            let (mut s, unit) = billy(1, false, None);
            assert_eq!(upgrades(s.events()).len(), 1);
            let (attack, health) = stats_added(s.events());
            s.expect_stats(&unit.id, json!({ "attack": 3 + attack, "maxHealth": 3 + health }));
            assert_eq!(cost_now(s.state(), s.card(&unit.id)), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn played_for_x_2_it_is_a_14_14_and_its_cry_upgrades_it_2x_4_times() {
            crate::register_all();
            let (mut s, unit) = billy(2, true, None);
            assert_eq!(upgrades(s.events()).len(), 4);
            let (attack, health) = stats_added(s.events());
            s.expect_stats(&unit.id, json!({ "attack": 14 + attack, "maxHealth": 14 + health }));
        }

        #[test]
        fn r386_its_upgrades_are_the_stats_split_or_a_keyword_never_cost_or_x() {
            crate::register_all();
            let (s, unit) = billy(1, true, Some("billy-radiant"));
            let changes: Vec<Value> = upgrades(s.events()).into_iter().map(|(_, change)| change["kind"].clone()).collect();
            assert_eq!(changes.len(), 2);
            assert!(changes.iter().all(|kind| *kind == json!("stats") || *kind == json!("keyword")));
            assert!(js(&s.card(&unit.id).tuning)["x"].is_null());
        }
    }
}
