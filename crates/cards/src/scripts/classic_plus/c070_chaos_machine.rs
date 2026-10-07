//! C+ #70 Chaos Machine (SPEC §8.7 row 70, R386; BUILD M9 row C+ 70). (2) Field Spell, Rare.
//!   Both faces: "Start of turn and end of turn: Upgrade {cards} random other card(s) in your hand or
//!   on your side of the field. Degrade {cards} random card(s) in your opponent's hand or on their
//!   side of the field." — cards 1, Radiant 2
//!
//! At its controller's start and end of turn (R62). Each pick is uniform over the named hand and field
//! cards but Chaos Machine itself (the tops of piles, face-down ones included, R585), different cards (R60);
//! the engine draws it over the piles' sizes alone and reports in R242's order, public cards first, and
//! a hidden card's change reaches the other seat as a bare cue (R177, R440). Empty zones draw nothing
//! (R129). The Radiant face's two is its declared `cards`, so both faces run this one script.

use jackioh_engine::effects::{degrade, upgrade};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-070";

fn tick(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    vec![
        upgrade(json_as(json!({
            "scope": { "side": "self", "zones": ["hand", "field"], "excludeSelf": true },
            "random": param(ctx, "cards")
        }))),
        degrade(json_as(json!({ "scope": { "side": "enemy", "zones": ["hand", "field"] }, "random": param(ctx, "cards") }))),
    ]
}

pub fn script() -> CardScripts {
    let tick: Hook = hook(tick);
    let base = Script {
        start_of_turn: Some(tick.clone()),
        end_of_turn: Some(tick),
        ..Script::default()
    };

    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C+ #70 Chaos Machine — SPEC §8.7 row 70, BUILD M9 row C+ 70: at the start and at the end of its
// controller's turn it Upgrades one random other card among their hand and side of the field and
// Degrades one random card among the opponent's (R386: one draw each; a pick on an Immutable card or
// one nothing fits changes nothing); a pick over a hand and a field is split by the piles' sizes
// (R242), and a hidden card's change reaches the other seat under the sentinel (R177); empty zones,
// nothing and no draw (R129); the count reads through `param()`; radiant two different cards each
// way. R585: the Upgrade never picks Chaos Machine itself.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MACHINE: &str = "classicplus-070";
    const UNIT: &str = "core-008"; // Mr. Vanilla 4/4.
    const MENACE: &str = "core-019"; // Midrange Menace; Radiant it is Immutable.
    const FILLER: &str = "core-005";
    const NETHER: &str = "core-088"; // Twisting Nether: a (4) Spell with no keywords and no declared numbers.

    use crate::js;

    /// TS `Tune[]`: the `upgraded` or `degraded` events, as their JSON (`instanceId`, `defId`, `change`).
    fn tunes(events: &[GameEvent], kind: &str) -> Vec<Value> {
        events.iter().filter(|event| event.event_type().as_str() == kind).map(js).collect()
    }

    fn instance_id_of(tune: &Value) -> String {
        tune["instanceId"].as_str().unwrap_or("").to_string()
    }

    fn owner_of(s: &Scenario, id: &str) -> Option<PlayerId> {
        for player in [P1, P2] {
            let side = &s.state().players[player];
            if side.hand.iter().any(|card| card.id == id) {
                return Some(player);
            }
            if side.units.iter().any(|pile| pile.as_ref().and_then(|pile| pile.first()).is_some_and(|top| top.id == id)) {
                return Some(player);
            }
            if side.backrow.iter().any(|card| card.as_ref().is_some_and(|card| card.id == id)) {
                return Some(player);
            }
        }
        None
    }

    /// TS `machine(p1, p2, { radiant?, seed? })`: Chaos Machine first in p1's backrow.
    fn machine(p1: Value, p2: Value, radiant: bool, seed: Option<&str>) -> Scenario {
        let mut backrow = vec![json!({ "def": MACHINE, "radiant": radiant })];
        if let Some(rest) = p1.get("backrow").and_then(Value::as_array) {
            backrow.extend(rest.iter().cloned());
        }
        let mut p1 = p1;
        p1["backrow"] = json!(backrow);
        let mut options = json!({ "p1": p1, "p2": p2 });
        if let Some(seed) = seed {
            options["seed"] = json!(seed);
        }
        scenario(options)
    }

    fn machine_id(s: &Scenario) -> String {
        s.backrow(P1, 1).expect("Chaos Machine in lane 1").id
    }

    #[test]
    fn is_a_2_field_spell_whose_two_hooks_run_one_tick_on_both_faces() {
        crate::register_all();
        let def = js(&crate::card_def(ID));
        assert_eq!(def["cost"], json!(2));
        assert_eq!(def["type"], json!("Field Spell"));
        let CardScripts { base, radiant } = script();
        let start = base.start_of_turn.as_ref().unwrap();
        assert!(Arc::ptr_eq(start, base.end_of_turn.as_ref().unwrap()));
        assert!(Arc::ptr_eq(start, radiant.start_of_turn.as_ref().unwrap()));
        assert!(Arc::ptr_eq(base.end_of_turn.as_ref().unwrap(), radiant.end_of_turn.as_ref().unwrap()));
    }

    mod base {
        use super::*;

        #[test]
        fn r62_r386_at_the_end_of_your_turn_one_upgrade_among_your_cards_one_degrade_among_theirs_each_its_own_draw() {
            crate::register_all();
            let mut s = machine(json!({ "hand": [FILLER], "field": [UNIT] }), json!({ "hand": [FILLER], "field": [UNIT] }), false, None);
            s.end_turn();
            let up = tunes(s.last_events(), "upgraded");
            let down = tunes(s.last_events(), "degraded");
            assert_eq!(up.len(), 1);
            assert_eq!(down.len(), 1);
            assert_eq!(owner_of(&s, &instance_id_of(&up[0])), Some(P1));
            assert_eq!(owner_of(&s, &instance_id_of(&down[0])), Some(P2));
        }

        #[test]
        fn r62_at_the_start_of_your_turn_too_and_never_on_the_opponent_s_turn() {
            crate::register_all();
            let mut s = machine(
                json!({ "hand": [FILLER, FILLER], "field": [UNIT] }),
                json!({ "hand": [FILLER, FILLER], "field": [UNIT] }),
                false,
                None,
            );
            s.end_turn();
            assert_eq!(s.state().active, P2);
            // p1's end of turn ticked once; p2's start of turn did not.
            assert_eq!(tunes(s.last_events(), "upgraded").len(), 1);
            s.end_turn();
            assert_eq!(s.state().active, P1);
            // p2's end of turn: nothing. p1's start of turn: one pair.
            assert_eq!(tunes(s.last_events(), "upgraded").len(), 1);
            assert_eq!(tunes(s.last_events(), "degraded").len(), 1);
            let down = tunes(s.last_events(), "degraded");
            assert_eq!(owner_of(&s, &instance_id_of(&down[0])), Some(P2));
        }

        #[test]
        fn r585_never_upgrades_itself_alone_on_its_side_with_an_empty_hand_nothing_changes() {
            crate::register_all();
            let mut s = machine(json!({}), json!({ "hand": [FILLER] }), false, None);
            let own = machine_id(&s);
            s.start_turn();
            assert!(tunes(s.last_events(), "upgraded").is_empty());
            assert!(s.card(&own).tuning.is_none());
            // The Degrade still resolves.
            assert_eq!(tunes(s.last_events(), "degraded").len(), 1);
        }

        #[test]
        fn r585_the_upgrade_picks_among_the_other_cards_never_itself_over_several_seeds() {
            crate::register_all();
            for n in 0..20 {
                let seed = format!("chaos-noself-{n}");
                let mut s = machine(json!({ "hand": [FILLER, FILLER], "field": [UNIT] }), json!({ "hand": [FILLER] }), false, Some(&seed));
                let own = machine_id(&s);
                s.start_turn();
                let up = tunes(s.last_events(), "upgraded");
                assert_eq!(up.len(), 1);
                assert_ne!(instance_id_of(&up[0]), own);
            }
        }

        #[test]
        fn r386_a_pick_on_an_immutable_card_changes_nothing_and_reports_nothing() {
            crate::register_all();
            let mut s = machine(json!({ "hand": [FILLER] }), json!({ "hand": [], "field": [{ "def": MENACE, "radiant": true }] }), false, None);
            let menace = s.unit(P2, 1).expect("Midrange Menace").id;
            s.start_turn();
            assert!(tunes(s.last_events(), "degraded").is_empty());
            assert!(s.card(&menace).tuning.is_none());
            assert_eq!(s.card(&menace).cost_mod, 0);
        }

        #[test]
        fn r386_r440_a_pick_on_a_card_nothing_fits_changes_nothing_their_4_spell_with_no_numbers_is_cued_none_unchanged() {
            crate::register_all();
            let mut s = machine(json!({ "hand": [FILLER], "library": [FILLER] }), json!({ "hand": [NETHER] }), false, None);
            let nether = s.hand(P2)[0].id.clone();
            s.start_turn();
            let down = tunes(s.last_events(), "degraded");
            assert_eq!(down.len(), 1);
            assert_eq!(down[0]["instanceId"], json!(nether));
            assert_eq!(down[0]["change"]["kind"], json!("none"));
            assert_eq!(s.card(&nether).cost_mod, 0);
            assert!(s.card(&nether).tuning.is_none());
        }

        #[test]
        fn s3_2_r13_a_stack_pile_offers_only_its_top_the_card_dormant_beneath_is_never_picked() {
            crate::register_all();
            for n in 0..12 {
                let seed = format!("chaos-stack-{n}");
                let mut s = machine(
                    json!({ "hand": [FILLER], "library": [FILLER] }),
                    json!({
                        "hand": [],
                        "field": [{ "def": "core-043", "lane": 1 }, { "def": "core-092", "stack": true }, { "def": UNIT, "lane": 2 }]
                    }),
                    false,
                    Some(&seed),
                );
                let Some(dormant) = s.state().players.p2.units[0].as_ref().and_then(|pile| pile.get(1)).map(|card| card.id.clone())
                else {
                    panic!("a dormant card under the pile");
                };
                s.start_turn();
                let down = tunes(s.last_events(), "degraded");
                let picked = down.first().map(instance_id_of);
                let tops = [s.unit(P2, 1).expect("lane 1").id, s.unit(P2, 2).expect("lane 2").id];
                assert!(picked.is_some_and(|id| tops.contains(&id)));
                assert!(s.card(&dormant).tuning.is_none());
                assert_eq!(s.card(&dormant).cost_mod, 0);
            }
        }

        #[test]
        fn r242_a_pick_over_a_hand_and_a_field_takes_either_by_the_piles_sizes_alone() {
            crate::register_all();
            let (mut hand, mut field) = (0, 0);
            for n in 0..30 {
                let seed = format!("chaos-split-{n}");
                let mut s = machine(json!({ "hand": [FILLER] }), json!({ "hand": [FILLER], "field": [UNIT] }), false, Some(&seed));
                let unit = s.unit(P2, 1).expect("Mr. Vanilla").id;
                s.start_turn();
                let down = tunes(s.last_events(), "degraded");
                if down.first().map(instance_id_of) == Some(unit) {
                    field += 1;
                } else {
                    hand += 1;
                }
            }
            assert!(field > 5);
            assert!(hand > 5);
        }

        #[test]
        fn r177_the_degrade_of_an_opponent_s_hand_card_reaches_you_under_the_sentinel_your_hand_s_upgrade_reaches_them_so() {
            crate::register_all();
            let hidden_id = jackioh_engine::view_for::HIDDEN_ID;
            let mut s = machine(json!({ "hand": [FILLER, FILLER] }), json!({ "hand": [FILLER, FILLER] }), false, None);
            s.end_turn();
            let down = tunes(&s.view(P1).events, "degraded");
            assert_eq!(down.len(), 1);
            assert_eq!(down[0]["instanceId"], json!(hidden_id));
            assert_eq!(down[0]["defId"], json!(hidden_id));
            let theirs = tunes(&s.view(P2).events, "degraded");
            assert_ne!(theirs.first().map(|tune| tune["instanceId"].clone()), Some(json!(hidden_id)));
            let mut hidden = false;
            let mut n = 0;
            while n < 20 && !hidden {
                let seed = format!("chaos-view-{n}");
                n += 1;
                let mut t = machine(json!({ "hand": [FILLER, FILLER] }), json!({ "hand": [FILLER, FILLER] }), false, Some(&seed));
                let own = machine_id(&t);
                t.end_turn();
                let up = tunes(t.last_events(), "upgraded");
                let Some(up) = up.first() else {
                    continue;
                };
                if instance_id_of(up) == own {
                    continue;
                }
                let seen_by_them = tunes(&t.view(P2).events, "upgraded");
                assert_eq!(seen_by_them[0]["instanceId"], json!(hidden_id));
                assert_eq!(seen_by_them[0]["defId"], json!(hidden_id));
                let seen_by_you = tunes(&t.view(P1).events, "upgraded");
                assert_eq!(seen_by_you.first().map(instance_id_of), Some(instance_id_of(up)));
                hidden = true;
            }
            assert!(hidden);
        }

        #[test]
        fn r129_an_opponent_with_no_hand_and_no_field_takes_no_degrade_and_no_draw() {
            crate::register_all();
            let mut s = machine(json!({ "hand": [FILLER] }), json!({ "hand": [] }), false, None);
            let own = s.backrow(P1, 1).expect("Chaos Machine in lane 1");
            let end_of_turn = script().base.end_of_turn.expect("the tick");
            let mut events: Vec<GameEvent> = Vec::new();
            let state = s.state_mut();
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            {
                let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
                let mut ctx = jackioh_engine::resolve::make_context(
                    &mut sink,
                    Some(&own),
                    jackioh_engine::resolve::HookOptions { controller: Some(P1), ..Default::default() },
                );
                let mut effects = end_of_turn(&mut ctx);
                let degrade_theirs = effects.remove(1);
                let cursor = ctx.rng.cursor();
                jackioh_engine::resolve::apply_effects(&[degrade_theirs], &mut ctx);
                assert_eq!(ctx.rng.cursor(), cursor);
            }
            assert!(events.is_empty());
        }

        #[test]
        fn r386_the_count_reads_through_param_an_upgrade_of_cards_makes_it_two_each_way() {
            crate::register_all();
            let mut s = machine(
                json!({ "hand": [FILLER, FILLER], "field": [UNIT] }),
                json!({ "hand": [FILLER, FILLER], "field": [UNIT] }),
                false,
                None,
            );
            let own = machine_id(&s);
            step_param(s.card_mut(&own), "cards", 1);
            s.end_turn();
            assert_eq!(tunes(s.last_events(), "upgraded").len(), 2);
            assert_eq!(tunes(s.last_events(), "degraded").len(), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r60_two_different_cards_each_way_at_the_end_and_at_the_start_of_your_turn() {
            crate::register_all();
            let mut s = machine(
                json!({ "hand": [FILLER, FILLER], "field": [UNIT] }),
                json!({ "hand": [FILLER, FILLER], "field": [UNIT, UNIT] }),
                true,
                None,
            );
            s.end_turn();
            for kind in ["upgraded", "degraded"] {
                let ids: Vec<String> = tunes(s.last_events(), kind).iter().map(instance_id_of).collect();
                assert_eq!(ids.len(), 2);
                let distinct: IndexSet<&String> = ids.iter().collect();
                assert_eq!(distinct.len(), 2);
            }
            s.end_turn();
            assert_eq!(tunes(s.last_events(), "upgraded").len(), 2);
            assert_eq!(tunes(s.last_events(), "degraded").len(), 2);
        }

        #[test]
        fn r129_r60_r585_with_one_other_card_on_your_side_that_one_card_only_never_itself() {
            crate::register_all();
            let mut s = machine(json!({ "hand": [FILLER] }), json!({ "hand": [FILLER] }), true, None);
            let own = machine_id(&s);
            s.start_turn();
            assert_eq!(tunes(s.last_events(), "degraded").len(), 1);
            let up = tunes(s.last_events(), "upgraded");
            assert_eq!(up.len(), 1);
            assert_ne!(instance_id_of(&up[0]), own);
        }
    }
}
