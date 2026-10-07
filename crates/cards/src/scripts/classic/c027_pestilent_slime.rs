//! C #27 Pestilent Slime (SPEC §8.6 row 27). (0) Unit, Common, 1/1 → 2/2.
//!   Base:    "Plague Counters placed on this are multiplied by {multiplier}." — ×2
//!   Radiant: "Plague Counters placed on this are multiplied by {multiplier}." — ×3
//!   Engine:  "Plague Counters (§6.3): a placement multiplier (×2, Radiant ×3) on every placement onto it,
//!            whoever places them. Tunes: multiplier 2 ↑."
//!
//! B5 E19, R471: the multiplier is the card's `plagueMultiplier`, which `plague.placePlagueOn` reads on
//! every placement onto it — "Place N Plague Counters on this" puts N × multiplier, one placement of a
//! "Place N Plague Counters" split puts 1 × multiplier — so each placement is still ONE placement,
//! reported by one `counterChanged` carrying how many it put (`placed`), and a "whenever Plague Counters
//! are placed on this" answers it once. It is the card's text, so it holds whoever places the tokens,
//! and a Vanilla Slime multiplies by 1. Its tokens are counters, which R78 clears when it leaves the
//! field.
//!
//! The multiplier is the declared `multiplier` (R386), read through `param` on the face it wears; both
//! faces run this one script.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-027";

pub fn script() -> CardScripts {
    let base = Script {
        // TS `({ state, self, radiant }) => param({ state, self, radiant }, "multiplier")`: the hook's
        // own `{ state, self, radiant }` is what `param` reads.
        plague_multiplier: Some(read_hook(|args| param(&args, "multiplier"))),
        ..Script::default()
    };
    // The same script: the Radiant face's ×3 is its declared `multiplier`, which `param` reads off the face it wears.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C #27 Pestilent Slime — SPEC §8.6 row 27, BUILD M9 Classic row C 27: "Every placement onto it is
// doubled: "place N Plague Counters on this" puts 2N, and each one-token placement of a split puts 2,
// still one placement for "whenever Plague Counters are placed" triggers; `counterChanged` shows the
// count; its tokens reset when it leaves (R78); radiant 2/2: tripled; its tuned number (multiplier)
// reads through `param()` (R386)".
//
// The placements come from C #39 Outbreak ("Place {tokens} Plague Counters on a permanent", one
// placement) and C #70 Book of Plague ("Place {tokens} Plague Counters": one placement per token, all
// on the one permanent a single prompt names, R689).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const SLIME: &str = "classic-027";
    const OUTBREAK: &str = "classic-039"; // (1) Spell: place {tokens} on a permanent; own → draw per token.
    const BOOK_OF_PLAGUE: &str = "classic-070"; // (1) Spell: Place 5 Plague Counters (5 placements).
    const FLOOD: &str = "core-017"; // (4) Spell: Bounce all Units.
    const VANILLA: &str = "core-008";
    const POSTDOC: &str = "core-061"; // Radiant: Cry: choose any Unit on the field, summon a Vanilla copy of it.
    const ANCHOR: &str = "core-010";
    const X: &str = "core-020";

    /// TS `placements`: the `counterChanged` events of the plague counter on this card (TS `Placement`).
    fn placements(s: &Scenario, instance_id: &str) -> Vec<GameEvent> {
        s.events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    GameEvent::CounterChanged { instance_id: id, counter: CounterKind::Plague, .. } if id == instance_id
                )
            })
            .cloned()
            .collect()
    }

    fn placed_of(event: &GameEvent) -> Option<i32> {
        match event {
            GameEvent::CounterChanged { placed, .. } => *placed,
            _ => None,
        }
    }

    fn value_of(event: &GameEvent) -> Option<i32> {
        match event {
            GameEvent::CounterChanged { value, .. } => Some(*value),
            _ => None,
        }
    }

    fn outbreak_on<'a>(s: &'a mut Scenario, instance_id: &str) -> &'a mut Scenario {
        s.play(OUTBREAK, json!({ "targets": [{ "pick": "instance", "instanceId": instance_id }] }))
    }

    fn plague_of(s: &Scenario, card: &str) -> Option<i32> {
        s.card(card).counters.plague
    }

    /// TS `stepParam(s.card(ref), key, steps)`: on the live instance, found again by id.
    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        step_param(find_instance_mut(s.state_mut(), &id).expect("the card is in the state"), key, steps);
    }

    mod c_n27_pestilent_slime {
        use super::*;

        #[test]
        fn declares_its_multiplier_as_a_placement_hook_one_script_on_both_faces() {
            crate::register_all();
            let scripts = script();
            assert_eq!(crate::card_def(ID).id, SLIME);
            assert!(scripts.base.plague_multiplier.is_some());
            // TS `expect(radiant).toBe(base)`: the Radiant face is the base script itself.
            assert!(Arc::ptr_eq(
                scripts.radiant.plague_multiplier.as_ref().unwrap(),
                scripts.base.plague_multiplier.as_ref().unwrap()
            ));
        }

        mod base {
            use super::*;

            #[test]
            fn is_a_1_1() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "field": [SLIME], "hand": [ANCHOR] } }));
                s.expect_stats(SLIME, json!({ "attack": 1, "health": 1 }));
            }

            #[test]
            fn r471_a_placement_of_1_on_it_puts_2_as_one_placement_one_counterchanged_carrying_placed_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "field": [SLIME], "library": [X, X, X] },
                    "p2": { "hand": [ANCHOR] },
                }));
                let slime = s.card(SLIME).clone();

                outbreak_on(&mut s, &slime.id);

                assert_eq!(plague_of(&s, &slime.id), Some(2));
                assert_eq!(
                    serde_json::to_value(placements(&s, &slime.id)).unwrap(),
                    json!([{ "type": "counterChanged", "instanceId": slime.id, "counter": "plague", "value": 2, "placed": 2 }])
                );
            }

            #[test]
            fn a_placement_of_n_puts_2n_a_radiant_outbreak_s_2_put_4_on_top_of_what_it_had() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [{ "def": OUTBREAK, "radiant": true }, ANCHOR],
                        "field": [{ "def": SLIME, "counters": { "plague": 1 } }],
                        "library": [X, X, X, X, X, X],
                    },
                    "p2": { "hand": [ANCHOR] },
                }));
                let slime = s.card(SLIME).clone();

                outbreak_on(&mut s, &slime.id);

                assert_eq!(plague_of(&s, &slime.id), Some(5));
                let pairs: Vec<(Option<i32>, Option<i32>)> = placements(&s, &slime.id)
                    .iter()
                    .map(|event| (value_of(event), placed_of(event)))
                    .collect();
                assert_eq!(pairs, vec![(Some(5), Some(4))]);
            }

            #[test]
            fn whoever_places_them_the_opponent_s_placement_on_it_is_doubled_too() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [ANCHOR], "field": [SLIME] },
                    "p2": { "hand": [OUTBREAK, ANCHOR], "library": [X, X] },
                }));
                let slime = s.card(SLIME).clone();

                outbreak_on(&mut s, &slime.id);

                assert_eq!(placements(&s, &slime.id).first().and_then(placed_of), Some(2));
            }

            #[test]
            fn r471_r689_each_one_token_placement_on_it_puts_2_each_its_own_placement_one_answer_lands_all_five() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [BOOK_OF_PLAGUE, ANCHOR], "field": [SLIME, VANILLA] },
                    "p2": { "hand": [ANCHOR] },
                }));
                let slime = s.card(SLIME).clone();
                let vanilla = s.card(VANILLA).clone();

                s.play(BOOK_OF_PLAGUE, json!({}));
                s.answer(json!([{ "pick": "instance", "instanceId": slime.id }]));

                assert!(s.state().pending.is_none());
                assert_eq!(plague_of(&s, &slime.id), Some(10));
                let placed: Vec<Option<i32>> = placements(&s, &slime.id).iter().map(placed_of).collect();
                assert_eq!(placed, vec![Some(2), Some(2), Some(2), Some(2), Some(2)]);
                assert_eq!(plague_of(&s, &vanilla.id).unwrap_or(0), 0);
            }

            #[test]
            fn s6_3_vanilla_a_vanilla_copy_has_no_text_so_a_placement_of_1_on_it_puts_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": POSTDOC, "radiant": true }, OUTBREAK, ANCHOR], "field": [SLIME], "library": [X, X, X] },
                    "p2": { "hand": [ANCHOR] },
                }));
                let slime = s.card(SLIME).id.clone();
                s.play(POSTDOC, json!({ "zone": 3, "targets": [{ "pick": "instance", "instanceId": slime }] }));
                let copy = match s.unit(P1, 2) {
                    Some(copy) if copy.def_id == SLIME && copy.vanilla => copy,
                    _ => panic!("a Vanilla Slime in lane 2"),
                };

                outbreak_on(&mut s, &copy.id);

                assert_eq!(plague_of(&s, &copy.id), Some(1));
                let placed: Vec<Option<i32>> = placements(&s, &copy.id).iter().map(placed_of).collect();
                assert_eq!(placed, vec![Some(1)]);
            }

            #[test]
            fn r78_its_tokens_reset_when_it_leaves_the_field() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FLOOD, ANCHOR], "field": [{ "def": SLIME, "counters": { "plague": 4 } }] },
                    "p2": { "hand": [ANCHOR] },
                }));
                let slime = s.card(SLIME).clone();

                s.play(FLOOD, json!({}));

                s.expect_in_zone(&slime, "hand");
                assert_eq!(plague_of(&s, &slime.id).unwrap_or(0), 0);
            }

            #[test]
            fn r386_an_upgrade_makes_it_3_a_degrade_1() {
                crate::register_all();
                let mut up = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "field": [SLIME], "library": [X, X, X, X] },
                    "p2": { "hand": [ANCHOR] },
                }));
                step(&mut up, SLIME, "multiplier", 1);
                let slime = up.card(SLIME).id.clone();
                outbreak_on(&mut up, &slime);
                assert_eq!(plague_of(&up, SLIME), Some(3));

                let mut down = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "field": [SLIME], "library": [X, X, X, X] },
                    "p2": { "hand": [ANCHOR] },
                }));
                step(&mut down, SLIME, "multiplier", -1);
                let slime = down.card(SLIME).id.clone();
                outbreak_on(&mut down, &slime);
                assert_eq!(plague_of(&down, SLIME), Some(1));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn is_a_2_2_and_a_placement_of_1_on_it_puts_3() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "field": [{ "def": SLIME, "radiant": true }], "library": [X, X, X, X] },
                    "p2": { "hand": [ANCHOR] },
                }));
                let slime = s.card(SLIME).clone();
                s.expect_stats(&slime, json!({ "attack": 2, "health": 2 }));

                outbreak_on(&mut s, &slime.id);

                assert_eq!(plague_of(&s, &slime.id), Some(3));
                let placed: Vec<Option<i32>> = placements(&s, &slime.id).iter().map(placed_of).collect();
                assert_eq!(placed, vec![Some(3)]);
            }

            #[test]
            fn r386_an_upgrade_on_the_radiant_face_steps_3_to_4() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "field": [{ "def": SLIME, "radiant": true }], "library": [X, X, X, X, X] },
                    "p2": { "hand": [ANCHOR] },
                }));
                step(&mut s, SLIME, "multiplier", 1);

                let slime = s.card(SLIME).id.clone();
                outbreak_on(&mut s, &slime);

                assert_eq!(plague_of(&s, SLIME), Some(4));
            }
        }
    }
}
