//! C #27 Pestilent Slime (SPEC §8.6 row 27; R471, R2521005). (0) Unit, Common, 1/1 → 2/2.
//!   Base:    "Plague Counters placed on this are multiplied by {multiplier}.\nDeath: Spread its Plague
//!            Counters among your permanents." — ×2
//!   Radiant: the same words — ×3
//!   Engine:  "Plague Counters (§6.3): a placement multiplier (×2, Radiant ×3) on every placement onto it,
//!            whoever places them. Tunes: multiplier 2 ↑."
//!
//! B5 E19, R471: the multiplier is the card's `plagueMultiplier`, which `plague.placePlagueOn` reads on
//! every placement onto it, so "Place N" puts N × multiplier and each placement of a split puts 1 ×
//! multiplier. Each is still ONE placement, one `counterChanged` carrying `placed`, so a "whenever
//! Plague Counters are placed on this" answers once. A Vanilla Slime multiplies by 1; R78 clears its tokens.
//! The multiplier is the declared `multiplier` (R386), read through `param`; both faces run this script.
//!
//! THE DEATH (#2521, R2521005): the Plague Counters it held as it died, read off the Death's snapshot
//! (taken before R78's reset, R89), each become one placement of one counter (`placePlagueRandom` of
//! one card) on a permanent its controller controls on the field, units and backrow, face-up or down,
//! each drawn on its own from the match rng as it resolves, so one permanent may take several. A
//! placement onto another Pestilent Slime is multiplied by that Slime's multiplier (R471). With no
//! permanent there, the counters are lost; with none held, nothing happens and nothing is drawn.
//! Through v0.5.11 it had no Death, which `script_v0_5_11` keeps (R767000).

use jackioh_engine::effects::place_plague_random;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-027";

/// R2521005: each counter it died with is one placement of this many counters…
const COUNTERS_PER_PLACEMENT: i32 = 1;
/// …on this many random permanents.
const PERMANENTS_PER_PLACEMENT: i32 = 1;

/// R2521005: "Death: Spread its Plague Counters among your permanents." One placement per counter on
/// the snapshot, each on a random permanent of its controller's (units and backrow, face-down included).
fn spread(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let held = ctx.self_.as_ref().map_or(0, plague_on);
    (0..held)
        .map(|_| {
            place_plague_random(json_as(json!({
                "count": PERMANENTS_PER_PLACEMENT,
                "amount": COUNTERS_PER_PLACEMENT,
                "scope": { "side": "self", "rows": ["units", "backrow"] },
            })))
        })
        .collect()
}

pub fn script() -> CardScripts {
    let base = Script {
        plague_multiplier: Some(read_hook(|args| param(&args, "multiplier"))),
        death: Some(hook(spread)),
        ..Script::default()
    };
    // The same script: the Radiant face's ×3 is its declared `multiplier`, which `param` reads off the face it wears.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// R767000: through v0.5.11 it had no Death, so the counters it held left with it (R78); `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_5_11() -> CardScripts {
    let base = Script {
        plague_multiplier: Some(read_hook(|args| param(&args, "multiplier"))),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C #27 Pestilent Slime — SPEC §8.6 row 27, BUILD M9 Classic row C 27 (R78, R386, R2521005). Placements come
// from C #39 Outbreak (one) and C #70 Book of Plague (one per token, R689); the Slime dies to Core #35 Lunar
// Eclipse.
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

    const LUNAR_ECLIPSE: &str = "core-035"; // (1) Spell: deal {damage} damage, enough for either face.
    const SHEEPISH: &str = "core-041"; // (1) Trap, set face-down.

    /// p1's Slime (Radiant when `radiant`) holding `held` Plague Counters in lane 1, `field` beside it
    /// from lane 2 and `backrow` behind; p2's Vanilla and Sheepish, which no placement may reach.
    fn dying(seed: &str, radiant: bool, held: i32, field: Value, backrow: Value) -> Scenario {
        crate::register_all();
        let slime = json!({ "def": SLIME, "radiant": radiant, "counters": { "plague": held }, "lane": 1 });
        let mut units = vec![slime];
        units.extend(field.as_array().cloned().unwrap_or_default());
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [LUNAR_ECLIPSE, ANCHOR], "field": units, "backrow": backrow, "library": [X, X, X] },
            "p2": { "hand": [ANCHOR], "field": [VANILLA], "backrow": [SHEEPISH], "library": [X, X, X] },
        }))
    }

    /// p1 casts Lunar Eclipse at the Slime in lane 1, which dies; its placements, by the card each went
    /// on and how many it put there.
    fn kill_slime(s: &mut Scenario) -> Vec<(String, i32)> {
        let slime = s.unit_at(P1, 1);
        let from = s.events().len();
        s.play(LUNAR_ECLIPSE, json!({ "targets": pick::instance(&slime) }));
        s.expect_in_zone(&slime, "graveyard");
        s.events()[from..]
            .iter()
            .filter_map(|event| match event {
                GameEvent::CounterChanged {
                    instance_id,
                    counter: CounterKind::Plague,
                    placed: Some(placed),
                    ..
                } => Some((instance_id.clone(), *placed)),
                _ => None,
            })
            .collect()
    }

    /// Every Plague Counter on `player`'s permanents.
    fn plague_on_side(s: &Scenario, player: PlayerId) -> i32 {
        plague_on_field(s.state(), player)
    }

    mod c_n27_pestilent_slime {
        use super::*;

        mod base {
            use super::*;

            /// R767000: the script kept for the replays of v0.5.11 and before multiplies and has no Death.
            #[test]
            fn r767000_the_script_kept_for_v0_5_11_has_no_death() {
                let kept = script_v0_5_11();
                for face in [&kept.base, &kept.radiant] {
                    assert!(face.plague_multiplier.is_some());
                    assert!(face.death.is_none());
                }
                assert!(script().base.death.is_some(), "since #2521 it has one");
            }

            #[test]
            fn r2521005_each_counter_it_died_with_is_one_placement_of_1_on_its_controllers_permanents() {
                let mut s = dying(
                    "slime-death",
                    false,
                    3,
                    json!([{ "def": VANILLA, "lane": 2 }, { "def": X, "lane": 3 }]),
                    json!([SHEEPISH]),
                );
                let mine: Vec<String> = [s.unit_at(P1, 2), s.unit_at(P1, 3)]
                    .iter()
                    .map(|card| card.id.clone())
                    .chain(s.backrow(P1, 1).map(|card| card.id))
                    .collect();

                let placements = kill_slime(&mut s);

                assert_eq!(placements.len(), 3, "one placement per counter: {placements:?}");
                for (on, placed) in &placements {
                    assert_eq!(*placed, 1, "one counter each");
                    assert!(mine.contains(on), "on a permanent of its controller's: {on}");
                }
                assert_eq!(plague_on_side(&s, P1), 3);
                assert_eq!(plague_on_side(&s, PlayerId::P2), 0, "never the enemy's");
            }

            #[test]
            fn r2521005_each_placement_is_drawn_on_its_own_so_the_counters_spread() {
                let board = |seed: &str| {
                    dying(
                        seed,
                        false,
                        2,
                        json!([{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 3 }]),
                        json!([]),
                    )
                };
                // Over the seeds, two placements land on two different Vanillas…
                let seed = seed_where("slime-spread", |seed| {
                    let placements = kill_slime(&mut board(seed));
                    placements.len() == 2 && placements[0].0 != placements[1].0
                });
                let mut s = board(&seed);
                let (two, three) = (s.unit_at(P1, 2), s.unit_at(P1, 3));
                kill_slime(&mut s);
                assert_eq!(s.card(&two).counters.plague, Some(1));
                assert_eq!(s.card(&three).counters.plague, Some(1));
                // …and on others both on one, as the draws are each their own.
                let seed = seed_where("slime-stack", |seed| {
                    let placements = kill_slime(&mut board(seed));
                    placements.len() == 2 && placements[0].0 == placements[1].0
                });
                let mut s = board(&seed);
                kill_slime(&mut s);
                assert_eq!(plague_on_side(&s, P1), 2);
            }

            #[test]
            fn r2521005_a_face_down_backrow_card_is_a_permanent_it_may_take_them() {
                let mut s = dying("slime-backrow", false, 2, json!([]), json!([SHEEPISH]));
                let trap = s.backrow(P1, 1).expect("p1's Sheepish");
                assert_ne!(trap.face_up, Some(true), "set face-down");
                let placements = kill_slime(&mut s);
                assert_eq!(placements, vec![(trap.id.clone(), 1), (trap.id.clone(), 1)]);
                assert_eq!(s.card(&trap).counters.plague, Some(2));
            }

            #[test]
            fn r2521005_with_no_permanent_of_its_controllers_left_the_counters_are_lost() {
                let mut s = dying("slime-lost", false, 3, json!([]), json!([]));
                let placements = kill_slime(&mut s);
                assert!(placements.is_empty(), "{placements:?}");
                assert_eq!(plague_on_side(&s, P1), 0);
                assert_eq!(plague_on_side(&s, PlayerId::P2), 0, "never the enemy's");
            }

            #[test]
            fn r2521005_with_no_counters_its_death_does_nothing() {
                let mut s = dying("slime-none", false, 0, json!([{ "def": VANILLA, "lane": 2 }]), json!([]));
                let placements = kill_slime(&mut s);
                assert!(placements.is_empty(), "{placements:?}");
                assert_eq!(plague_on_side(&s, P1), 0);
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
                step_param(up.card_mut(SLIME), "multiplier", 1);
                let slime = up.card(SLIME).id.clone();
                outbreak_on(&mut up, &slime);
                assert_eq!(plague_of(&up, SLIME), Some(3));

                let mut down = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "field": [SLIME], "library": [X, X, X, X] },
                    "p2": { "hand": [ANCHOR] },
                }));
                step_param(down.card_mut(SLIME), "multiplier", -1);
                let slime = down.card(SLIME).id.clone();
                outbreak_on(&mut down, &slime);
                assert_eq!(plague_of(&down, SLIME), Some(1));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r2521005_its_counters_spread_and_a_placement_onto_another_slime_is_multiplied() {
                // The dying Radiant Slime holds 2; the only permanent left is a base Slime (×2).
                let mut s = dying("slime-radiant", true, 2, json!([{ "def": SLIME, "lane": 2 }]), json!([]));
                let other = s.unit_at(P1, 2);
                let placements = kill_slime(&mut s);
                assert_eq!(placements, vec![(other.id.clone(), 2), (other.id.clone(), 2)]);
                assert_eq!(s.card(&other).counters.plague, Some(4));
            }

            #[test]
            fn r2521005_onto_a_radiant_slime_each_placement_triples() {
                let mut s = dying(
                    "slime-radiant-onto",
                    true,
                    3,
                    json!([{ "def": SLIME, "radiant": true, "lane": 2 }]),
                    json!([]),
                );
                let other = s.unit_at(P1, 2);
                let placements = kill_slime(&mut s);
                assert_eq!(placements.len(), 3);
                assert!(placements.iter().all(|(on, placed)| *on == other.id && *placed == 3));
                assert_eq!(s.card(&other).counters.plague, Some(9));
            }

            #[test]
            fn r2521005_with_no_permanent_left_the_radiant_slimes_counters_are_lost() {
                let mut s = dying("slime-radiant-lost", true, 2, json!([]), json!([]));
                assert!(kill_slime(&mut s).is_empty());
                assert_eq!(plague_on_side(&s, P1), 0);
            }

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
                step_param(s.card_mut(SLIME), "multiplier", 1);

                let slime = s.card(SLIME).id.clone();
                outbreak_on(&mut s, &slime);

                assert_eq!(plague_of(&s, SLIME), Some(4));
            }
        }
    }
}
