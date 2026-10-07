//! C #47 Recurring Felinor (SPEC §8.6 row 47; §6.3 Cast; R4, R68, R70, R78, R87, R386). Unit, Felinor,
//! 3/2 → 6/4, cost 2, Rare.
//!   Base:    "Cry: Cast Ancient Acquisition.\nWhile this is in your graveyard: When one of your Traps
//!            reveals, return this to hand."
//!   Radiant: "… return this to hand. It costs ({returnCost})." (0)
//!   Engine:  the Cry casts a generated C #34 on its base face, free, returning 2 random cards (R684),
//!            then to your graveyard (R70, R87). The return is a graveyard trigger on your `trapFired`, a
//!            Field Trap's firing included, live only while this card is in your graveyard; the hand cap
//!            applies (R4). Radiant: it returns with `costOverride` 0, which persists in every zone (R78).
//!
//! A graveyard trigger's `when` is not consulted outside the trap window, so "one of your Traps" is
//! checked in `run`, which answers anything else with no effects.

use jackioh_engine::effects::{CastDef, CastHow, CastNewArgs, CastNewDef, add_to_hand, cast_new};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-047";

const ANCIENT_ACQUISITION: &str = "classic-034";

/// TS `costOverride?: (ctx: EffectContext) => number`: the cost the returned card takes, read as the
/// trigger runs.
type ReturnCost = Arc<dyn Fn(&EffectContext<'_>) -> i32 + Send + Sync>;

fn recur(cost_override: Option<ReturnCost>) -> TriggerDef {
    TriggerDef::new("recurring-felinor", &[GameEventType::TrapFired], move |ctx, event| {
        let GameEvent::TrapFired { controller, .. } = event else {
            return vec![];
        };
        if *controller != ctx.controller {
            return vec![];
        }
        let mut args = json!({ "instance": { "of": "self" } });
        if let Some(cost) = &cost_override {
            args["costOverride"] = json!(cost(&*ctx));
        }
        vec![add_to_hand(json_as(args))]
    })
}

fn cry(_ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    vec![cast_new(CastNewArgs {
        def: CastNewDef::Def(CastDef {
            def_id: ANCIENT_ACQUISITION.to_string(),
            radiant: Some(false),
        }),
        radiant: None,
        how: CastHow::default(),
    })]
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(cry)),
        graveyard_triggers: vec![recur(None)],
        ..Script::default()
    };

    let radiant = Script {
        cry: Some(hook(cry)),
        graveyard_triggers: vec![recur(Some(Arc::new(|ctx: &EffectContext<'_>| -> i32 {
            param(ctx, "returnCost")
        })))],
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #47 Recurring Felinor — SPEC §8.6 row 47, BUILD M9 Classic row C 47: "Cry: cast a generated Ancient
// Acquisition (C #34, base face), free and counted as played (R70), returning 2 at random (R684), the
// Spell going to your graveyard afterwards (R87); while this card is in your graveyard, whenever one
// of your Traps or Field Traps fires (`trapFired`), return this to hand (a graveyard trigger, R68); an
// opponent's trap doesn't, and in a hand or on the field it doesn't; the returned card follows R97 in
// the opponent's view; radiant 6/4: it returns and costs (0) (`costOverride`); its tuned number
// (radiant cost) reads through `param()` (R386)".
//
// C #34 Ancient Acquisition ("Return 2 random cards from your graveyard to hand") has its
// own tests. The traps that fire are Core's Sheepish (a Trap answering a played Unit) and Bread and
// Butter (a Field Trap answering a turn's end with mana unspent), and C #52 Final Gambit (a Trap that
// fires as it replaces a lethal hit).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FELINOR: &str = "classic-047";
    const ACQUISITION: &str = "classic-034";
    const GAMBIT: &str = "classic-052";
    const SHEEPISH: &str = "core-041"; // Trap: when your opponent plays a Unit, after its Cry: transform it into a Sheep.
    const BREAD: &str = "core-018"; // Field Trap: when a turn ends with mana unspent, summon a Bread Token.
    const VANILLA: &str = "core-008";
    const GARY: &str = "core-004";
    const LUNAR: &str = "core-035";
    const FILLER: &str = "core-005";

    /// An engine value as the JSON the TS test reads (SURFACE §5.1: the same keys and values).
    fn js<T: serde::Serialize + ?Sized>(value: &T) -> Value {
        serde_json::to_value(value).expect("an engine value serialises")
    }

    /// p2 plays Mr. Vanilla into p1's face-down Sheepish, with p1's Recurring Felinor in its graveyard.
    /// TS defaults: `felinor` `{ def: FELINOR }`, `p1Hand` `[FILLER]`; every caller passes both.
    fn sheepish_fires(felinor: Value, p1_hand: Vec<&str>) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": p1_hand, "backrow": [{ "def": SHEEPISH, "faceUp": false }], "graveyard": [felinor] },
            "p2": { "hand": [VANILLA, FILLER] },
            "active": "p2",
        }));
        s.play(VANILLA, json!({}));
        s
    }

    /// TS `stepParam(s.card(ref), key, steps)`: the step written on the card as it stands in the state.
    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        let instance = find_instance_mut(s.state_mut(), &id).expect("the card to step is in the state");
        step_param(instance, key, steps);
    }

    fn graveyard(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.pile(player, "graveyard")
    }

    mod c47_recurring_felinor {
        use super::*;

        #[test]
        fn names_c_34_in_its_refs_a_cry_and_one_graveyard_trigger_on_trap_fired_on_each_face() {
            crate::register_all();
            let def = js(&registered_catalog()[ID]);
            assert_eq!(def["id"], FELINOR);
            assert_eq!(def["refs"], json!([ACQUISITION]));
            assert_eq!(
                def["params"],
                json!([{ "key": "returnCost", "base": 0, "radiant": 0, "better": "down", "step": 1, "min": 0 }]),
            );
            let scripts = script();
            for face in [&scripts.base, &scripts.radiant] {
                let on: Vec<Vec<GameEventType>> = face.graveyard_triggers.iter().map(|trigger| trigger.on.clone()).collect();
                assert_eq!(js(&on), json!([["trapFired"]]));
            }
        }

        mod base {
            use super::*;

            #[test]
            fn r70_cry_casts_a_generated_ancient_acquisition_free_and_counted_as_played_returning_2_at_random_r87_it_then_lands_in_your_graveyard() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FELINOR, FILLER], "graveyard": [LUNAR, GARY, VANILLA] },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(FELINOR, json!({}));
                s.expect_mana(P1, 2);
                assert_eq!(cards_played_this_turn(s.state(), P1), 2);
                let played: Vec<String> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::CardPlayed { def_id, .. } => Some(def_id.clone()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(played, vec![FELINOR, ACQUISITION]);

                // R684: no prompt — two random cards return, one stays.
                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(P1).len(), 3);
                assert_eq!(graveyard(&s, P1).len(), 2);

                let acquisition = graveyard(&s, P1).into_iter().find(|card| card.def_id == ACQUISITION);
                assert_eq!(acquisition.as_ref().map(|card| card.radiant), Some(false));
                assert_eq!(acquisition.as_ref().map(|card| card.owner), Some(P1));
                s.expect_in_zone(FELINOR, "field");
            }

            #[test]
            fn cry_with_an_empty_graveyard_the_cast_asks_nothing_and_still_lands_in_your_graveyard() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [FELINOR, FILLER] }, "p2": { "hand": [FILLER] } }));
                s.play(FELINOR, json!({}));
                assert!(s.state().pending.is_none());
                let defs: Vec<String> = graveyard(&s, P1).into_iter().map(|card| card.def_id).collect();
                assert_eq!(defs, vec![ACQUISITION]);
            }

            #[test]
            fn r684_the_generated_cast_returns_at_random_the_same_game_returns_the_same_cards() {
                crate::register_all();
                let mk = || {
                    scenario(json!({
                        "p1": { "hand": [FELINOR, FILLER], "graveyard": [LUNAR, GARY, VANILLA] },
                        "p2": { "hand": [FILLER] },
                    }))
                };
                let mut first = mk();
                first.play(FELINOR, json!({}));
                let mut second = mk();
                second.play(FELINOR, json!({}));
                let ids = |s: &Scenario| -> Vec<Value> {
                    s.events()
                        .iter()
                        .map(js)
                        .filter(|event| event["type"] == "addedToHand" && event["player"] == "p1")
                        .map(|event| event["instanceId"].clone())
                        .collect()
                };
                assert_eq!(ids(&first), ids(&second));
            }

            #[test]
            fn r68_in_your_graveyard_when_one_of_your_traps_fires_it_returns_to_your_hand() {
                crate::register_all();
                let mut s = sheepish_fires(json!({ "def": FELINOR }), vec![FILLER]);
                s.expect_events(json!(["trapFired"]));
                s.expect_in_zone(FELINOR, "hand");
                assert!(s.card(FELINOR).cost_override.is_none());
            }

            #[test]
            fn a_field_traps_firing_counts_bread_and_butter_at_your_turns_end_returns_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "backrow": [BREAD], "graveyard": [FELINOR] },
                    "p2": { "hand": [FILLER], "library": [FILLER] },
                }));
                s.end_turn();
                s.expect_events(json!(["turnEnded", "trapFired"]));
                s.expect_in_zone(FELINOR, "hand");
            }

            #[test]
            fn a_trap_that_fires_as_it_replaces_counts_final_gambit_re_aiming_a_lethal_hit_returns_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [FILLER],
                        "backrow": [{ "def": GAMBIT, "faceUp": false }],
                        "graveyard": [FELINOR],
                        "library": [FILLER, FILLER, FILLER],
                        "health": 4,
                    },
                    "p2": { "hand": [FILLER], "field": [VANILLA] },
                    "active": "p2",
                }));
                s.attack(VANILLA, "hero");
                s.expect_in_zone(FELINOR, "hand");
            }

            #[test]
            fn your_opponents_trap_does_not_return_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [VANILLA, FILLER], "graveyard": [FELINOR] },
                    "p2": { "hand": [FILLER], "backrow": [{ "def": SHEEPISH, "faceUp": false }] },
                }));
                s.play(VANILLA, json!({}));
                s.expect_events(json!(["trapFired"]));
                s.expect_in_zone(FELINOR, "graveyard");
            }

            #[test]
            fn on_the_field_or_in_a_hand_it_does_nothing_when_your_trap_fires() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [FELINOR, FILLER],
                        "field": [{ "def": FELINOR }],
                        "backrow": [{ "def": SHEEPISH, "faceUp": false }],
                    },
                    "p2": { "hand": [VANILLA, FILLER] },
                    "active": "p2",
                }));
                let on_field = s.unit(P1, 1);
                let in_hand = s.hand(P1).into_iter().find(|card| card.def_id == FELINOR);
                s.play(VANILLA, json!({}));
                s.expect_events(json!(["trapFired"]));
                let (Some(on_field), Some(in_hand)) = (on_field, in_hand) else {
                    panic!("both Felinors should be in place");
                };
                s.expect_in_zone(&on_field, "field").expect_in_zone(&in_hand, "hand");
                assert_eq!(s.hand(P1).len(), 2);
            }

            #[test]
            fn r4_the_hand_cap_applies_with_a_full_hand_it_is_burned_back_into_your_graveyard() {
                crate::register_all();
                let ten: Vec<&str> = (0..10).map(|_| FILLER).collect();
                let mut s = sheepish_fires(json!({ "def": FELINOR }), ten);
                assert!(s.events().iter().any(|event| matches!(event, GameEvent::Burned { .. })));
                s.expect_in_zone(FELINOR, "graveyard");
                assert_eq!(s.hand(P1).len(), 10);
            }

            #[test]
            fn r97_once_back_in_your_hand_the_opponent_no_longer_reads_it() {
                crate::register_all();
                let s = sheepish_fires(json!({ "def": FELINOR }), vec![FILLER]);
                assert!(js(&s.view(P1)).to_string().contains(FELINOR));
                let theirs = js(&s.view(P2));
                assert!(!theirs.to_string().contains(FELINOR));
                let graveyard_defs: Vec<Value> = theirs["opponent"]["graveyard"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .map(|card| card["defId"].clone())
                    .collect();
                assert!(!graveyard_defs.contains(&json!(FELINOR)));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r70_cry_casts_ancient_acquisition_on_its_base_face_2_random_returns_not_the_radiants_4() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": FELINOR, "radiant": true }, FILLER], "graveyard": [LUNAR, GARY, VANILLA] },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(FELINOR, json!({}));
                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(P1).len(), 3);
                assert_eq!(
                    graveyard(&s, P1).into_iter().find(|card| card.def_id == ACQUISITION).map(|card| card.radiant),
                    Some(false),
                );
                s.expect_stats(FELINOR, json!({ "attack": 6, "health": 4 }));
            }

            #[test]
            fn returns_when_one_of_your_traps_fires_and_costs_0() {
                crate::register_all();
                let mut s = sheepish_fires(json!({ "def": FELINOR, "radiant": true }), vec![FILLER]);
                let felinor = s.card(FELINOR).clone();
                s.expect_in_zone(&felinor, "hand");
                assert_eq!(felinor.cost_override, Some(0));
                assert_eq!(effective_cost(s.state(), &felinor, Default::default()), 0);
            }

            #[test]
            fn r97_r177_back_in_your_hand_at_0_the_opponent_reads_neither_the_card_nor_its_new_cost() {
                crate::register_all();
                let s = sheepish_fires(json!({ "def": FELINOR, "radiant": true }), vec![FILLER]);
                let theirs = js(&s.view(P2)).to_string();
                assert!(!theirs.contains(FELINOR));
                assert!(!theirs.contains(s.card(FELINOR).id.as_str()));
            }

            #[test]
            fn r78_the_0_persists_it_is_played_for_nothing_and_keeps_its_0_on_the_field() {
                crate::register_all();
                let mut s = sheepish_fires(json!({ "def": FELINOR, "radiant": true }), vec![FILLER]);
                let felinor = s.card(FELINOR).clone();
                s.end_turn(); // p2's turn ends; p1's begins.
                let mana = s.state().players.p1.mana.current;
                s.play(&felinor, json!({}));
                s.expect_mana(P1, mana);
                assert_eq!(s.card(&felinor).cost_override, Some(0));
            }

            #[test]
            fn your_opponents_trap_does_not_return_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [VANILLA, FILLER], "graveyard": [{ "def": FELINOR, "radiant": true }] },
                    "p2": { "hand": [FILLER], "backrow": [{ "def": SHEEPISH, "faceUp": false }] },
                }));
                s.play(VANILLA, json!({}));
                s.expect_in_zone(FELINOR, "graveyard");
            }

            #[test]
            fn r386_a_degrade_makes_it_return_costing_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [FILLER],
                        "backrow": [{ "def": SHEEPISH, "faceUp": false }],
                        "graveyard": [{ "def": FELINOR, "radiant": true }],
                    },
                    "p2": { "hand": [VANILLA, FILLER] },
                    "active": "p2",
                }));
                step(&mut s, FELINOR, "returnCost", 1);
                s.play(VANILLA, json!({}));
                let felinor = s.card(FELINOR).clone();
                s.expect_in_zone(&felinor, "hand");
                assert_eq!(effective_cost(s.state(), &felinor, Default::default()), 1);
            }
        }
    }
}
