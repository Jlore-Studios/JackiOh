//! C #42 Transmutable Toxins (SPEC §8.6 row 42, §6.2 Activate, §6.3 Plague Counter, §10.4 layer 5; R60,
//! R69, R384). Field Spell, cost 2, Rare.
//!   Base:    "Aura: Your Units have +{stats}/+{stats} for each Plague Counter on them. Enemy Units have
//!            −{stats}/−{stats} for each Plague Counter on them.\nActivate: Place a Plague Counter on each
//!            of {tokens|random Unit|random Units}." (stats 1, tokens 2)
//!   Radiant: the same words with stats 2.
//!   Engine:  "An aura (§10.4 layer 5) reading each unit's `counters.plague`; −1/−1 lowers max health,
//!            so an enemy can die of it at the state check (#46 Suppressive Aura's rule). Activate
//!            (§6.2, R384, once per turn): one Plague Counter (§6.3) on each of two different random
//!            units on the field, either side (R60: a random pick of N picks N different cards; with
//!            one unit on the field, it gets one token); a C #27 Pestilent Slime multiplies its own.
//!            Tunes: tokens 2 ↑; stats per token 1 ↑."
//!
//! THE AURA is §10.4's layer 5, recomputed on every read: for each unit on the field carrying Plague
//! Tokens, one entry naming that unit with its stats per token (`param(ctx, "stats")`) times its
//! tokens — up for its controller's own Units, down for the enemy's. Lowering max health is what kills
//! an enemy at the state check when it reaches 0 (§4.5), an Indestructible one too (R69). A unit
//! dormant under a Stack pile is not on the field (R13), so it is not read. The hook is a pure read of
//! instance data: it never asks the layers for a stat, so it cannot recurse.
//!
//! THE ACTIVATE (B3.2, R384: once per turn, by its controller, while it acts on the field) is one
//! placement of a single token on each of `param(ctx, "tokens")` different random units on the field,
//! either side (`placePlagueRandom`, R60: fewer units, fewer placements; none, nothing). Each is a
//! placement, so a C #27 Pestilent Slime's multiplier doubles the one it receives (B5 E19). It is not a
//! play, so nothing that answers plays sees it.

use jackioh_engine::prelude::*;
use jackioh_engine::effects::place_plague_random;

pub const ID: &str = "classic-042";

/// TS `const aura: AuraHook = ({ state, self, radiant }) => …`.
fn aura<'a>(a: HookArgs<'a>) -> Vec<AuraEntry<'a>> {
    let per_token = param(&a, "stats");
    let mut entries = Vec::new();
    for player in PLAYER_IDS {
        for unit in active_units_of(a.state, player) {
            let tokens = unit.counters.plague.unwrap_or(0);
            if tokens <= 0 {
                continue;
            }
            let change = (if unit.controller == a.self_.controller { 1 } else { -1 }) * per_token * tokens;
            let unit_id = unit.id.clone();
            entries.push(AuraEntry {
                applies: Box::new(move |candidate: &CardInstance| candidate.id == unit_id),
                mod_: StatMod {
                    attack: Some(change),
                    max_health: Some(change),
                    ..StatMod::default()
                },
            });
        }
    }
    entries
}

pub fn script() -> CardScripts {
    let toxins = Script {
        aura: Some(aura_hook(aura)),
        activations: vec![ActivationDecl {
            id: "transmutable-toxins".into(),
            label: "Place a Plague Counter on each of several random Units".into(),
            uses: ActivationUses::Count(1),
            cost: None,
            targets: vec![],
            modes: vec![],
            can_activate: None,
            has: None,
            run: hook(|ctx| {
                vec![place_plague_random(json_as(json!({
                    "count": param(ctx, "tokens"),
                    "amount": 1,
                    "scope": { "side": "any", "rows": ["units"] },
                })))]
            }),
        }],
        ..Script::default()
    };

    let base = toxins.clone();

    // The same script: the Radiant face differs only in its declared stats per token (2), which `param`
    // reads off the running face.
    let radiant = toxins;

    CardScripts { base, radiant }
}

// C #42 Transmutable Toxins — SPEC §8.6 row 42, BUILD M9 Classic row C 42: "Aura (§10.4 layer 5):
// your Units have +1/+1 for each Plague Counter on them and enemy Units −1/−1, recomputed on every
// change, so a token placed later applies at once; the −1/−1 lowers max health and an enemy can die of
// it at the state check, an Indestructible one too once its max health reaches 0 (R69); gone when it
// leaves; Activate, once per turn (R384): one token on each of two different random Units on the field,
// either side (R60: one Unit → one token; none → nothing); C #27 doubles its share; not a play;
// radiant: +2/+2 and −2/−2; its tuned numbers (tokens, stats per token) read through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::{script, ID};
    use jackioh_engine::testkit::*;

    const TOXINS: &str = "classic-042";
    const SLIME: &str = "classic-027"; // (0) Unit 1/1: "Plague Counters placed on this are multiplied by {multiplier}." (2)
    const FAUCI: &str = "core-091"; // (2) Unit 1/6: "Whenever this takes damage, it gets a Plague Counter."
    const ECLIPSE: &str = "core-035"; // (1) Spell: "Deal 3 damage to a target."
    const COLLATERAL: &str = "core-034"; // (4) Spell: "Exile target permanent and a random card from your opponent's deck."
    const STATE_OF_GAME: &str = "classic-041"; // (1) Unit 3/3 Indestructible
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const MENACE: &str = "core-019"; // (3) Unit 9/9
    const TIMMY: &str = "core-011"; // (1) Unit 3/3
    const STOCKPILE: &str = "core-005"; // (1) Spell, a spare card (§2.5)

    /// An engine value as the JSON the TS test reads (SURFACE §5.1: the same keys and values).
    fn js<T: serde::Serialize + ?Sized>(value: &T) -> Value {
        serde_json::to_value(value).expect("an engine value serialises")
    }

    /// TS `stepParam(s.card(ref), key, steps)`: TS's `card()` handed back the live instance, so the
    /// step is written on the state's own copy, found again by id.
    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        step_param(find_instance_mut(s.state_mut(), &id).expect("the card is in the game"), key, steps);
    }

    fn tokens_on(s: &Scenario, player: PlayerId, lane: i32) -> i32 {
        s.unit(player, lane).and_then(|unit| unit.counters.plague).unwrap_or(0)
    }

    /// `mine` and `theirs`: the TS helper's field lists, as JSON arrays.
    fn with_toxins(radiant_face: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [ECLIPSE, STOCKPILE],
                "backrow": [{ "def": TOXINS, "radiant": radiant_face }],
                "field": mine,
                "library": [STOCKPILE],
            },
            "p2": { "hand": [COLLATERAL, STOCKPILE], "field": theirs, "library": [STOCKPILE, STOCKPILE] },
        }))
    }

    mod c_42_transmutable_toxins {
        use super::*;

        #[test]
        fn declares_its_two_numbers_r386_tokens_2_stats_per_token_1_radiant_2_and_one_activate() {
            crate::register_all();
            assert_eq!(
                js(&crate::card_def(ID).params),
                json!([
                    { "key": "tokens", "base": 2, "radiant": 2, "better": "up", "step": 1, "min": 1 },
                    { "key": "stats", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 },
                ]),
            );
            let scripts = script();
            assert_eq!(scripts.base.activations.iter().map(|ability| js(&ability.uses)).collect::<Vec<Value>>(), vec![json!(1)]);
            // TS `expect(radiant).toBe(base)`: the Radiant face is the same script — the same aura and
            // the same one ability.
            assert_eq!(scripts.radiant.aura.is_some(), scripts.base.aura.is_some());
            assert_eq!(
                scripts.radiant.activations.iter().map(|ability| (ability.id.clone(), ability.label.clone(), js(&ability.uses))).collect::<Vec<_>>(),
                scripts.base.activations.iter().map(|ability| (ability.id.clone(), ability.label.clone(), js(&ability.uses))).collect::<Vec<_>>(),
            );
        }

        mod base {
            use super::*;

            #[test]
            fn aura_your_units_have_1_1_for_each_plague_counter_on_them() {
                crate::register_all();
                let mut s = with_toxins(false, json!([{ "def": VANILLA, "counters": { "plague": 2 } }, TIMMY]), json!([]));
                s.expect_stats(VANILLA, json!({ "attack": 6, "health": 6, "maxHealth": 6 }));
                s.expect_stats(TIMMY, json!({ "attack": 3, "health": 3, "maxHealth": 3 }));
            }

            #[test]
            fn aura_enemy_units_have_1_1_for_each_plague_counter_on_them() {
                crate::register_all();
                let mut s = with_toxins(false, json!([]), json!([{ "def": MENACE, "counters": { "plague": 3 } }]));
                s.expect_stats(MENACE, json!({ "attack": 6, "health": 6, "maxHealth": 6 }));
            }

            #[test]
            fn recomputed_on_every_change_a_token_placed_later_applies_at_once() {
                crate::register_all();
                // A Fed Fauci (1/6) hit for 3 gets a token: the enemy's is −1/−1 at once (max health 5, 2 left),
                // yours +1/+1 (max health 7, 4 left).
                let mut s = scenario(json!({
                    "p1": { "hand": [ECLIPSE, ECLIPSE, STOCKPILE], "backrow": [TOXINS], "field": [FAUCI] },
                    "p2": { "hand": [STOCKPILE], "field": [FAUCI] },
                }));
                let theirs = s.unit(PlayerId::P2, 1).map(|unit| unit.id.clone());
                let mine = s.unit(PlayerId::P1, 1).map(|unit| unit.id.clone());
                let (Some(theirs), Some(mine)) = (theirs, mine) else {
                    panic!("setup");
                };
                let first = s.hand(PlayerId::P1).first().map(|card| card.id.clone()).unwrap_or_else(|| ECLIPSE.to_string());
                s.play(&first, json!({ "targets": [{ "pick": "instance", "instanceId": theirs }] }));
                assert_eq!(tokens_on(&s, PlayerId::P2, 1), 1);
                s.expect_stats(&theirs, json!({ "attack": 0, "maxHealth": 5, "health": 2 }));
                let first = s.hand(PlayerId::P1).first().map(|card| card.id.clone()).unwrap_or_else(|| ECLIPSE.to_string());
                s.play(&first, json!({ "targets": [{ "pick": "instance", "instanceId": mine }] }));
                assert_eq!(tokens_on(&s, PlayerId::P1, 1), 1);
                s.expect_stats(&mine, json!({ "attack": 2, "maxHealth": 7, "health": 4 }));
            }

            #[test]
            fn s4_5_the_1_1_lowers_max_health_an_enemy_it_brings_to_0_dies_at_the_state_check() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TOXINS, STOCKPILE] },
                    "p2": {
                        "hand": [STOCKPILE],
                        "field": [{ "def": TIMMY, "counters": { "plague": 3 } }, { "def": VANILLA, "counters": { "plague": 1 } }],
                    },
                }));
                let timmy = s.card(TIMMY).id.clone();
                s.play(TOXINS, json!({ "zone": 1 }));
                s.expect_in_zone(&timmy, "graveyard");
                s.expect_stats(VANILLA, json!({ "attack": 3, "maxHealth": 3 }));
            }

            #[test]
            fn r69_an_indestructible_enemy_dies_too_once_its_max_health_reaches_0() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TOXINS, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE], "field": [{ "def": STATE_OF_GAME, "counters": { "plague": 3 } }] },
                }));
                let state = s.card(STATE_OF_GAME).id.clone();
                s.play(TOXINS, json!({ "zone": 1 }));
                s.expect_in_zone(&state, "graveyard");
            }

            #[test]
            fn gone_when_it_leaves_the_stats_return() {
                crate::register_all();
                let mut s = with_toxins(false, json!([{ "def": VANILLA, "counters": { "plague": 2 } }]), json!([]));
                s.end_turn();
                let toxins = s.card(TOXINS).id.clone();
                s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": toxins }] }));
                s.expect_in_zone(TOXINS, "exile");
                s.expect_stats(VANILLA, json!({ "attack": 4, "health": 4, "maxHealth": 4 }));
            }

            #[test]
            fn a_unit_with_no_token_is_untouched() {
                crate::register_all();
                let mut s = with_toxins(false, json!([VANILLA]), json!([MENACE]));
                s.expect_stats(VANILLA, json!({ "attack": 4, "maxHealth": 4 }));
                s.expect_stats(MENACE, json!({ "attack": 9, "maxHealth": 9 }));
            }

            #[test]
            fn r386_an_upgrade_of_stats_per_token_makes_it_2_2_and_2_2() {
                crate::register_all();
                let mut s = with_toxins(
                    false,
                    json!([{ "def": VANILLA, "counters": { "plague": 1 } }]),
                    json!([{ "def": MENACE, "counters": { "plague": 1 } }]),
                );
                step(&mut s, TOXINS, "stats", 1);
                s.expect_stats(VANILLA, json!({ "attack": 6, "maxHealth": 6 }));
                s.expect_stats(MENACE, json!({ "attack": 7, "maxHealth": 7 }));
            }

            #[test]
            fn r384_activate_one_token_on_each_of_two_different_random_units_either_side() {
                crate::register_all();
                let mut s = with_toxins(false, json!([VANILLA, TIMMY]), json!([MENACE]));
                s.activate(TOXINS, json!({}));
                let tokens = [tokens_on(&s, PlayerId::P1, 1), tokens_on(&s, PlayerId::P1, 2), tokens_on(&s, PlayerId::P2, 1)];
                assert_eq!(tokens.iter().filter(|&&count| count == 1).count(), 2);
                assert_eq!(tokens.iter().filter(|&&count| count == 0).count(), 1);
            }

            #[test]
            fn r384_once_per_turn_a_second_activate_is_refused() {
                crate::register_all();
                let mut s = with_toxins(false, json!([VANILLA, TIMMY]), json!([MENACE]));
                s.activate(TOXINS, json!({}));
                s.expect_refused(|s| {
                    s.activate(TOXINS, json!({}));
                });
            }

            #[test]
            fn r60_with_one_unit_on_the_field_it_gets_one_token_with_none_nothing() {
                crate::register_all();
                let mut one = with_toxins(false, json!([]), json!([MENACE]));
                one.activate(TOXINS, json!({}));
                assert_eq!(tokens_on(&one, PlayerId::P2, 1), 1);
                let mut none = with_toxins(false, json!([]), json!([]));
                none.activate(TOXINS, json!({}));
                assert!(!none.events().iter().map(js).any(|event| event["type"] == "counterChanged"));
            }

            #[test]
            fn e19_a_c_27_pestilent_slime_doubles_its_share() {
                crate::register_all();
                let mut s = with_toxins(false, json!([SLIME]), json!([MENACE]));
                s.activate(TOXINS, json!({}));
                assert_eq!(tokens_on(&s, PlayerId::P1, 1), 2);
                assert_eq!(tokens_on(&s, PlayerId::P2, 1), 1);
            }

            #[test]
            fn an_activate_is_not_a_play_nothing_that_answers_plays_sees_it() {
                crate::register_all();
                let mut s = with_toxins(false, json!([VANILLA]), json!([MENACE]));
                s.activate(TOXINS, json!({}));
                assert!(!s.events().iter().map(js).any(|event| event["type"] == "cardPlayed"));
            }

            #[test]
            fn r386_an_upgrade_of_tokens_places_on_3_different_units() {
                crate::register_all();
                let mut s = with_toxins(false, json!([VANILLA, TIMMY]), json!([MENACE]));
                step(&mut s, TOXINS, "tokens", 1);
                s.activate(TOXINS, json!({}));
                assert_eq!(
                    [tokens_on(&s, PlayerId::P1, 1), tokens_on(&s, PlayerId::P1, 2), tokens_on(&s, PlayerId::P2, 1)],
                    [1, 1, 1],
                );
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn c2_2_for_each_token_on_your_units_and_2_2_on_the_enemys() {
                crate::register_all();
                let mut s = with_toxins(
                    true,
                    json!([{ "def": VANILLA, "counters": { "plague": 2 } }]),
                    json!([{ "def": MENACE, "counters": { "plague": 2 } }]),
                );
                s.expect_stats(VANILLA, json!({ "attack": 8, "health": 8, "maxHealth": 8 }));
                s.expect_stats(MENACE, json!({ "attack": 5, "health": 5, "maxHealth": 5 }));
            }

            #[test]
            fn an_enemy_it_brings_to_0_max_health_dies() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": TOXINS, "radiant": true }, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE], "field": [{ "def": VANILLA, "counters": { "plague": 2 } }] },
                }));
                let vanilla = s.card(VANILLA).id.clone();
                s.play(TOXINS, json!({ "zone": 1 }));
                s.expect_in_zone(&vanilla, "graveyard");
            }

            #[test]
            fn r384_activate_the_same_two_random_units_one_token_each() {
                crate::register_all();
                let mut s = with_toxins(true, json!([VANILLA]), json!([MENACE]));
                s.activate(TOXINS, json!({}));
                assert_eq!([tokens_on(&s, PlayerId::P1, 1), tokens_on(&s, PlayerId::P2, 1)], [1, 1]);
                s.expect_stats(VANILLA, json!({ "attack": 6, "maxHealth": 6 }));
                s.expect_stats(MENACE, json!({ "attack": 7, "maxHealth": 7 }));
            }
        }
    }
}
