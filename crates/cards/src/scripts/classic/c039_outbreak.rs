//! C #39 Outbreak (SPEC §8.6 row 39). (1) Spell, Epic.
//!   Base:    "Place {tokens|Plague Counter|Plague Counters} on a permanent. If it's an enemy's with at
//!            least its cost in Plague Counters, steal it. Otherwise, draw 1 per Plague Counter on it."
//!            — 1 token
//!   Radiant: the same text — 2 tokens
//!
//! The target is any permanent on either side, face-down ones included, declared with the play (R81); a face-down
//! card its chooser may not read is offered by id alone and the placement never names it to them (R177).
//! One placement of {tokens} (`placePlague`, B5 E19, R471; `param`, R386; Pestilent Slime multiplies it), then, read
//! after it lands: an enemy permanent with counters at least its cost (R396's `costNow`: an X card on the field its X,
//! 0 with none chosen; any other card R65's cost) is stolen (§6.3, R15, an entry, R171); with no free zone it stays
//! with them and nothing is drawn, since "otherwise" is the condition's, not the steal's. Otherwise draw one per counter (§2.4). A target gone from the field takes nothing.
//! Tunes: tokens 1 ↑ (Radiant 2); drawPer 1 ↑, the cards drawn per counter (R1748003). Through v0.4.4b that 1 was
//! printed, not declared; `script_v0_4_4b` keeps that (R767000).

use jackioh_engine::prelude::*;
use jackioh_engine::effects::{ForEachCardArgs, draw, for_each_card, instance_of, place_plague, steal};

pub const ID: &str = "classic-039";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "backrow"] }))]
}

struct Outcome {
    id: String,
    steals: bool,
    tokens: i32,
}

/// which the draw half reads as the steal branch too (R136: this script's own `controlChanged`).
fn outcome(ctx: &EffectContext) -> Option<Outcome> {
    let card = instance_of(ctx, &TargetSpec::Chosen { index: None })?;
    if card.zone.z() != ZoneName::Field {
        return None;
    }
    let tokens = plague_on(&card);
    let taken_now = ctx.events[ctx.events_from..].iter().any(|event| {
        matches!(event, GameEvent::ControlChanged { instance_id, .. } if *instance_id == card.id)
    });
    let steals = taken_now || (card.controller != ctx.controller && tokens >= cost_now(ctx.state, &card));
    Some(Outcome {
        id: card.id.clone(),
        steals,
        tokens,
    })
}

fn stolen_ones(c: &mut EffectContext) -> Vec<String> {
    match outcome(c) {
        Some(read) if read.steals => vec![read.id],
        _ => vec![],
    }
}

fn draws_owed(c: &mut EffectContext) -> Vec<String> {
    match outcome(c) {
        Some(read) if !read.steals => vec![read.id; usize::try_from(read.tokens).unwrap_or(0)],
        _ => vec![],
    }
}

fn steal_it(instance_id: &str) -> Effect {
    steal(json_as(json!({ "instanceId": instance_id })))
}

/// "draw 1 per Plague Counter" as v0.4.4b printed it, before it was the declared `drawPer` (R1748003).
const PRINTED_DRAW_PER: i32 = 1;

fn outbreak(declared: bool) -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let per = if declared {
                param(ctx, "drawPer")
            } else {
                PRINTED_DRAW_PER
            };
            vec![
                place_plague(json_as(json!({ "target": { "of": "chosen" }, "amount": param(ctx, "tokens") }))),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(stolen_ones),
                    each: Arc::new(steal_it),
                }),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(draws_owed),
                    each: Arc::new(move |_instance_id: &str| draw(json_as(json!({ "count": per })))),
                }),
            ]
        })),
        ..Script::default()
    };

    CardScripts {
        radiant: base.clone(),
        base,
    }
}

pub fn script() -> CardScripts {
    outbreak(true)
}

/// R767000: through v0.4.4b its draws per counter were printed; `KEPT_SCRIPTS` folds those replays
pub fn script_v0_4_4b() -> CardScripts {
    outbreak(false)
}

// C #39 Outbreak — SPEC §8.6 row 39, BUILD M9 Classic row C 39 (#27, R65, R396, §6.3, R15, R171, R33, R177,
// R386).
// The R396 cases read costs through the engine's `costNow`. A Plague Chalice played for X enters with
// X Plague Counters (SPEC §8.6 row 87), so any placement brings it to its X; the cases that need an X card
// played for 3 with fewer tokens than its X use C+ #69 Buff Billy, the other X permanent R396 names.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const OUTBREAK: &str = "classic-039";
    const SLIME: &str = "classic-027"; // (0) Unit 1/1: placements on it doubled.
    const CHALICE: &str = "classic-087"; // (X) Field Spell.
    const BILLY: &str = "classicplus-069"; // (X) Unit 3X/3X; its Cry Upgrades it, never its cost (R396).
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9.
    const PAWN: &str = "core-096"; // (1) Trap, answers only a lethal attack.
    const UNLICENSED: &str = "core-085"; // (2) Trap; answers a permanent of a type its controller controls.
    const FILLER: &str = "core-005";
    const ANCHOR: &str = "core-010";
    const X: &str = "core-020";

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    fn stolen_ids(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::ControlChanged { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn drawn(s: &Scenario) -> usize {
        s.events().iter().filter(|event| matches!(event, GameEvent::Drawn { .. })).count()
    }

    fn drawn_in(events: &[GameEvent]) -> usize {
        events.iter().filter(|event| matches!(event, GameEvent::Drawn { .. })).count()
    }

    fn lib(n: usize) -> Vec<&'static str> {
        vec![X; n]
    }

    #[test]
    fn r1748003_draw_per_moves_with_a_buff_or_a_nerf() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [OUTBREAK, ANCHOR], "field": [{ "def": VANILLA, "counters": { "plague": 1 } }], "library": lib(6) },
            "p2": { "hand": [ANCHOR] },
        }));
        // A Nerf finds it at its floor of 1 (an amount never drops below 1, R386); a Buff draws 2 a counter.
        assert!(!crate::can_degrade_number(&s, OUTBREAK, "drawPer"));
        assert_eq!(crate::upgrade_number(&mut s, OUTBREAK, "drawPer"), 2);
        let targets = pick::card(&s, VANILLA);
        s.play(OUTBREAK, json!({ "targets": targets }));
        assert_eq!(s.card(VANILLA).counters.plague, Some(2));
        assert_eq!(drawn(&s), 4);
    }

    #[test]
    fn r1748003_r767000_the_v0_4_4b_kept_script_keeps_its_printed_draw_per() {
        crate::under_kept(OUTBREAK, super::script_v0_4_4b(), || {
            let mut s = scenario(json!({
                "p1": { "hand": [OUTBREAK, ANCHOR], "field": [{ "def": VANILLA, "counters": { "plague": 1 } }], "library": lib(6) },
                "p2": { "hand": [ANCHOR] },
            }));
            crate::upgrade_number(&mut s, OUTBREAK, "drawPer");
            let targets = pick::card(&s, VANILLA);
            s.play(OUTBREAK, json!({ "targets": targets }));
            assert_eq!(drawn(&s), 2, "the kept script draws its printed 1 a counter");
        });
    }

    mod c_39_outbreak {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn places_1_plague_counter_on_your_own_permanent_then_draws_one_card_per_token_on_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "field": [{ "def": VANILLA, "counters": { "plague": 2 } }], "library": lib(5) },
                    "p2": { "hand": [ANCHOR] },
                }));

                let targets = pick::card(&s, VANILLA);
                s.play(OUTBREAK, json!({ "targets": targets }));

                assert_eq!(s.card(VANILLA).counters.plague, Some(3));
                assert_eq!(drawn(&s), 3);
                assert!(stolen_ids(&s).is_empty());
            }

            #[test]
            fn your_own_permanent_always_draws_even_at_or_above_its_cost() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "field": [VANILLA], "library": lib(3) },
                    "p2": { "hand": [ANCHOR] },
                }));

                let targets = pick::card(&s, VANILLA);
                s.play(OUTBREAK, json!({ "targets": targets }));

                assert_eq!(drawn(&s), 1);
                assert_eq!(s.card(VANILLA).controller, P1);
            }

            #[test]
            fn an_enemy_permanent_with_fewer_tokens_than_its_cost_it_draws_one_per_token() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "library": lib(3) },
                    "p2": { "hand": [ANCHOR], "field": [MENACE] },
                }));

                let targets = pick::card(&s, MENACE);
                s.play(OUTBREAK, json!({ "targets": targets }));

                assert_eq!(s.card(MENACE).counters.plague, Some(1));
                assert_eq!(drawn(&s), 1);
                assert!(stolen_ids(&s).is_empty());
            }

            #[test]
            fn r15_r171_an_enemy_permanent_reaching_its_cost_is_stolen_into_the_same_lane_an_entry_and_nothing_is_drawn() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "library": lib(5) },
                    "p2": { "hand": [ANCHOR], "field": [{ "def": MENACE, "lane": 2, "counters": { "plague": 2 } }] },
                }));
                let menace = s.card(MENACE).id.clone();

                let targets = pick::card(&s, MENACE);
                s.play(OUTBREAK, json!({ "targets": targets }));

                assert_eq!(stolen_ids(&s), vec![menace.clone()]);
                assert_eq!(s.unit(P1, 2).map(|unit| unit.id.clone()), Some(menace.clone()));
                assert_eq!(s.card(&menace).counters.plague, Some(3));
                assert_eq!(drawn(&s), 0);
                s.expect_refused(|s| s.attack(&menace, "hero"));
            }

            #[test]
            fn a_0_cost_enemy_permanent_is_always_stolen() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "library": lib(3) },
                    "p2": { "hand": [ANCHOR], "field": [SLIME] },
                }));

                let targets = pick::card(&s, SLIME);
                s.play(OUTBREAK, json!({ "targets": targets }));

                assert_eq!(stolen_ids(&s), vec![s.card(SLIME).id.clone()]);
            }

            #[test]
            fn c_27_a_pestilent_slime_doubles_the_placement_your_own_slime_takes_2_and_you_draw_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "field": [SLIME], "library": lib(3) },
                    "p2": { "hand": [ANCHOR] },
                }));

                let targets = pick::card(&s, SLIME);
                s.play(OUTBREAK, json!({ "targets": targets }));

                assert_eq!(s.card(SLIME).counters.plague, Some(2));
                assert_eq!(drawn(&s), 2);
            }

            #[test]
            fn r15_with_no_free_zone_in_your_row_the_stolen_card_stays_with_them_and_nothing_is_drawn() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "field": [MENACE, MENACE, MENACE, MENACE, MENACE], "library": lib(3) },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA] },
                }));

                let targets = pick::card(&s, VANILLA);
                s.play(OUTBREAK, json!({ "targets": targets }));

                assert!(stolen_ids(&s).is_empty());
                assert_eq!(s.card(VANILLA).controller, P2);
                assert_eq!(drawn(&s), 0);
            }

            #[test]
            fn s2_4_the_hand_cap_burns_the_draws_that_do_not_fit() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [OUTBREAK, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                        "field": [{ "def": VANILLA, "counters": { "plague": 1 } }],
                        "library": lib(3),
                    },
                    "p2": { "hand": [ANCHOR] },
                }));

                let targets = pick::card(&s, VANILLA);
                s.play(OUTBREAK, json!({ "targets": targets }));

                assert_eq!(s.hand(P1).len(), 10);
                assert_eq!(s.events().iter().filter(|event| matches!(event, GameEvent::Burned { .. })).count(), 1);
            }

            #[test]
            fn r33_a_stolen_face_down_trap_is_read_by_you_alone_from_then_on() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "library": lib(3) },
                    "p2": { "hand": [ANCHOR], "backrow": [{ "def": PAWN, "faceUp": false }] },
                }));
                let pawn = s.card(PAWN).id.clone();

                s.play(OUTBREAK, json!({ "targets": [{ "pick": "instance", "instanceId": pawn }] }));

                assert_eq!(stolen_ids(&s), vec![pawn.clone()]);
                assert_ne!(s.card(&pawn).face_up, Some(true));
                assert!(serde_json::to_string(&s.view(P1).you.backrow).expect("a view serializes").contains(PAWN));
                assert!(!serde_json::to_string(&s.view(P2)).expect("a view serializes").contains(PAWN));
            }

            #[test]
            fn r177_a_face_down_enemy_option_and_the_placement_on_it_never_name_it_to_you() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "library": lib(3) },
                    "p2": { "hand": [ANCHOR], "backrow": [{ "def": UNLICENSED, "faceUp": false }] },
                }));
                let trap = s.card(UNLICENSED).id.clone();

                assert!(!serde_json::to_string(&s.view(P1)).expect("a view serializes").contains(UNLICENSED));
                s.play(OUTBREAK, json!({ "targets": [{ "pick": "instance", "instanceId": trap }] }));

                assert!(stolen_ids(&s).is_empty());
                assert_eq!(drawn(&s), 1);
                assert_eq!(s.card(&trap).counters.plague, Some(1));
                assert!(!serde_json::to_string(&s.view(P1)).expect("a view serializes").contains(UNLICENSED));
            }

            #[test]
            fn r396_an_x_card_on_the_field_costs_the_x_it_was_played_for_a_buff_billy_played_for_3_is_not_stolen_by_1_token() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [OUTBREAK, ANCHOR], "library": lib(4) },
                    "p2": { "hand": [BILLY, ANCHOR], "library": lib(2) },
                }));
                s.play(BILLY, json!({ "x": 3 }));
                s.end_turn();
                assert_eq!(s.state().active, P1);

                let targets = pick::card(&s, BILLY);
                s.play(OUTBREAK, json!({ "targets": targets }));

                assert_eq!(s.card(BILLY).counters.plague, Some(1));
                assert!(stolen_ids(&s).is_empty());
                assert_eq!(s.card(BILLY).controller, P2);
                assert_eq!(drawn_in(s.last_events()), 1);
            }

            #[test]
            fn r396_c_87_a_plague_chalice_played_for_3_enters_with_3_plague_counters_so_1_more_reaches_its_x_and_steals_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [OUTBREAK, ANCHOR], "library": lib(4) },
                    "p2": { "hand": [CHALICE, ANCHOR], "library": lib(2) },
                }));
                s.play(CHALICE, json!({ "x": 3 }));
                s.end_turn();

                let targets = pick::card(&s, CHALICE);
                s.play(OUTBREAK, json!({ "targets": targets }));

                assert_eq!(s.card(CHALICE).counters.plague, Some(4));
                assert_eq!(stolen_ids(&s), vec![s.card(CHALICE).id.clone()]);
                assert_eq!(drawn_in(s.last_events()), 0);
            }

            #[test]
            fn r396_an_x_card_that_arrived_with_no_x_chosen_costs_0_on_the_field_so_it_is_stolen() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "library": lib(3) },
                    "p2": { "hand": [ANCHOR], "backrow": [CHALICE] },
                }));

                let targets = pick::card(&s, CHALICE);
                s.play(OUTBREAK, json!({ "targets": targets }));

                assert_eq!(stolen_ids(&s), vec![s.card(CHALICE).id.clone()]);
            }

            #[test]
            fn r386_an_upgrade_places_2_an_enemy_2_cost_permanent_is_stolen() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "library": lib(3) },
                    "p2": { "hand": [ANCHOR], "backrow": [{ "def": UNLICENSED, "faceUp": false }] },
                }));
                step_param(s.card_mut(OUTBREAK), "tokens", 1);
                let trap = s.card(UNLICENSED).id.clone();

                s.play(OUTBREAK, json!({ "targets": [{ "pick": "instance", "instanceId": trap }] }));

                assert_eq!(s.card(&trap).counters.plague, Some(2));
                assert_eq!(stolen_ids(&s), vec![trap.clone()]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn places_2_your_own_permanent_draws_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": OUTBREAK, "radiant": true }, ANCHOR], "field": [VANILLA], "library": lib(3) },
                    "p2": { "hand": [ANCHOR] },
                }));

                let targets = pick::card(&s, VANILLA);
                s.play(OUTBREAK, json!({ "targets": targets }));

                assert_eq!(s.card(VANILLA).counters.plague, Some(2));
                assert_eq!(drawn(&s), 2);
            }

            #[test]
            fn places_2_an_enemy_3_cost_permanent_with_1_already_reaches_3_and_is_stolen() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": OUTBREAK, "radiant": true }, ANCHOR], "library": lib(3) },
                    "p2": { "hand": [ANCHOR], "field": [{ "def": MENACE, "counters": { "plague": 1 } }] },
                }));

                let targets = pick::card(&s, MENACE);
                s.play(OUTBREAK, json!({ "targets": targets }));

                assert_eq!(stolen_ids(&s), vec![s.card(MENACE).id.clone()]);
                assert_eq!(drawn(&s), 0);
            }

            #[test]
            fn r396_a_buff_billy_played_for_3_is_not_stolen_by_2_tokens_it_draws_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [{ "def": OUTBREAK, "radiant": true }, ANCHOR], "library": lib(4) },
                    "p2": { "hand": [BILLY, ANCHOR], "library": lib(2) },
                }));
                s.play(BILLY, json!({ "x": 3 }));
                s.end_turn();

                let targets = pick::card(&s, BILLY);
                s.play(OUTBREAK, json!({ "targets": targets }));

                assert!(stolen_ids(&s).is_empty());
                assert_eq!(drawn_in(s.last_events()), 2);
            }

            #[test]
            fn r386_an_upgrade_places_3_a_buff_billy_played_for_3_reaches_its_x_and_is_stolen() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [{ "def": OUTBREAK, "radiant": true }, ANCHOR], "library": lib(4) },
                    "p2": { "hand": [BILLY, ANCHOR], "library": lib(2) },
                }));
                s.play(BILLY, json!({ "x": 3 }));
                s.end_turn();
                step_param(s.card_mut(OUTBREAK), "tokens", 1);

                let targets = pick::card(&s, BILLY);
                s.play(OUTBREAK, json!({ "targets": targets }));

                assert_eq!(stolen_ids(&s), vec![s.card(BILLY).id.clone()]);
            }
        }
    }
}
