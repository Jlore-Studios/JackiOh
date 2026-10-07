//! C #39 Outbreak (SPEC §8.6 row 39). (1) Spell, Epic.
//!   Base:    "Place {tokens|Plague Counter|Plague Counters} on a permanent. If it's an enemy permanent with
//!            at least as many Plague Counters as its cost, steal it. Otherwise, draw a card for each
//!            Plague Counter on it." — 1 token
//!   Radiant: the same text — 2 tokens
//!   Engine:  "One declared target (R81), either side, one placement of 1 (Radiant 2) (Plague Counters,
//!            §6.3; a C #27 Pestilent Slime multiplies it). Its cost is R65's on the field: an X-cost
//!            card on the field costs the X it was played for … (R396) … Steal per §6.3 and R15;
//!            otherwise draw N (the hand cap applies). Tunes: tokens 1 ↑."
//!
//! The target is any permanent on either side — the top of a unit pile or a backrow card, face-down
//! ones included — declared with the play (R81); a face-down card its chooser may not read is offered
//! by its id alone and the placement on it never names it to them (R177).
//!
//! One placement of {tokens} on it (`placePlague`, B5 E19, R471), multiplied by its own multiplier.
//! Then the two branches, read once the placement has landed:
//!   * an enemy permanent (its controller is not the caster) whose Plague Counters are at least its cost
//!     — R396's `costNow`: an X card on the field its X, 0 with none chosen; any other card R65's cost
//!     where it stands — is stolen (§6.3, R15: the same lane if free, else the first free zone of its
//!     row, an entry, R171); with no free zone it stays with them, and nothing is drawn, since the
//!     text's "otherwise" is the condition's, not the steal's. A (0) Cost enemy permanent is always
//!     stolen. A stolen face-down trap is read by its new controller from then on (R33).
//!   * otherwise — your own permanent always — one draw per Plague Counter on it (§2.4: the hand cap
//!     burns what does not fit).
//! A target that has left the field by then takes nothing and draws nothing.
//!
//! Both branches are read after the placement, as the list reaches them (`forEachCard`), because the
//! placement is what they count. The number is the declared `tokens` (R386), read through `param`.

use jackioh_engine::prelude::*;
use jackioh_engine::effects::{ForEachCardArgs, draw, for_each_card, instance_of, place_plague, steal};

pub const ID: &str = "classic-039";

/// "a permanent": the top of any unit pile or any backrow card, either side.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "backrow"] }))]
}

/// TS `outcome`'s answer, `{ id, steals, tokens }`.
struct Outcome {
    id: String,
    steals: bool,
    tokens: i32,
}

/// The chosen permanent if it is still on the field, and whether the steal branch holds for it: an
/// enemy permanent with at least its cost in Plague Counters — or one this run's steal has just taken,
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
    let steals = taken_now || (card.controller != ctx.controller && tokens >= cost_now(&ctx.state, &card));
    Some(Outcome {
        id: card.id.clone(),
        steals,
        tokens,
    })
}

/// The first `forEachCard`'s `cards`: the chosen permanent, when the steal branch holds.
fn stolen_ones(c: &mut EffectContext) -> Vec<String> {
    match outcome(c) {
        Some(read) if read.steals => vec![read.id],
        _ => vec![],
    }
}

/// The second `forEachCard`'s `cards`: one entry per token, each a draw of 1 — "draw a card for each
/// Plague Counter on it".
fn draws_owed(c: &mut EffectContext) -> Vec<String> {
    match outcome(c) {
        Some(read) if !read.steals => vec![read.id; usize::try_from(read.tokens).unwrap_or(0)],
        _ => vec![],
    }
}

fn steal_it(instance_id: &str) -> Effect {
    steal(json_as(json!({ "instanceId": instance_id })))
}

fn draw_one(_instance_id: &str) -> Effect {
    draw(json_as(json!({ "count": 1 })))
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            vec![
                place_plague(json_as(json!({ "target": { "of": "chosen" }, "amount": param(ctx, "tokens") }))),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(stolen_ones),
                    each: Arc::new(steal_it),
                }),
                for_each_card(ForEachCardArgs {
                    // One entry per token, each a draw of 1: "draw a card for each Plague Counter on it".
                    cards: Arc::new(draws_owed),
                    each: Arc::new(draw_one),
                }),
            ]
        })),
        ..Script::default()
    };

    // The same script: the Radiant face's 2 tokens are its declared `tokens`, which `param` reads off the running face.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// C #39 Outbreak — SPEC §8.6 row 39, BUILD M9 Classic row C 39: "A declared target, any permanent
// either side, face-down included; one placement of 1 (C #27 doubles it); then, if it is an enemy
// permanent with at least as many Plague Counters as its cost (R65 on the field; an X card its X, 0 with
// none chosen, R396), steal it (§6.3, R15), an entry (R171); no free zone → it stays with them and
// nothing is drawn; otherwise draw one card per token on it (hand cap); your own permanent always
// draws; a (0) Cost enemy permanent is always stolen; a stolen face-down trap is read by you alone
// from then on (R33); a face-down option, and the placement on it, never name it to you (R177);
// radiant: 2 tokens; its tuned number (tokens) reads through `param()` (R386)".
//
// The R396 cases read costs through the engine's `costNow`. A Plague Chalice played for X enters with
// X Plague Counters (SPEC §8.6 row 87), so any placement brings it to its X; the cases that need an X card
// played for 3 with fewer tokens than its X use C+ #69 Buff Billy, the other X permanent R396 names.
#[cfg(test)]
mod tests {
    use super::{script, ID};
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

    use crate::js;

    /// TS `at(s, ref): Selection[]`.
    fn at(s: &Scenario, card: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": s.card(card).id }])
    }

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

    mod c_39_outbreak {
        use super::*;

        #[test]
        fn declares_one_target_any_permanent_on_either_side_and_runs_one_script_on_both_faces() {
            crate::register_all();
            assert_eq!(crate::card_def(ID).id, OUTBREAK);
            let scripts = script();
            assert_eq!(
                js(&scripts.base.targets),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "backrow"] } }]),
            );
            // TS `expect(radiant).toBe(base)`: the same declaration and the same Cry.
            assert_eq!(js(&scripts.radiant.targets), js(&scripts.base.targets));
            assert!(scripts.base.cry.is_some() && scripts.radiant.cry.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn places_1_plague_counter_on_your_own_permanent_then_draws_one_card_per_token_on_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "field": [{ "def": VANILLA, "counters": { "plague": 2 } }], "library": lib(5) },
                    "p2": { "hand": [ANCHOR] },
                }));

                let targets = at(&s, VANILLA);
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

                let targets = at(&s, VANILLA);
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

                let targets = at(&s, MENACE);
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

                let targets = at(&s, MENACE);
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

                let targets = at(&s, SLIME);
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

                let targets = at(&s, SLIME);
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

                let targets = at(&s, VANILLA);
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

                let targets = at(&s, VANILLA);
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
                assert!(serde_json::to_string(&s.view(P1).you.backrow).expect("a view serialises").contains(PAWN));
                assert!(!serde_json::to_string(&s.view(P2)).expect("a view serialises").contains(PAWN));
            }

            #[test]
            fn r177_a_face_down_enemy_option_and_the_placement_on_it_never_name_it_to_you() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [OUTBREAK, ANCHOR], "library": lib(3) },
                    "p2": { "hand": [ANCHOR], "backrow": [{ "def": UNLICENSED, "faceUp": false }] },
                }));
                let trap = s.card(UNLICENSED).id.clone();

                assert!(!serde_json::to_string(&s.view(P1)).expect("a view serialises").contains(UNLICENSED));
                s.play(OUTBREAK, json!({ "targets": [{ "pick": "instance", "instanceId": trap }] }));

                // (2) Cost with 1 token: not stolen, so it stays theirs, face-down, and you draw 1.
                assert!(stolen_ids(&s).is_empty());
                assert_eq!(drawn(&s), 1);
                assert_eq!(s.card(&trap).counters.plague, Some(1));
                assert!(!serde_json::to_string(&s.view(P1)).expect("a view serialises").contains(UNLICENSED));
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

                let targets = at(&s, BILLY);
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

                let targets = at(&s, CHALICE);
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

                let targets = at(&s, CHALICE);
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

                let targets = at(&s, VANILLA);
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

                let targets = at(&s, MENACE);
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

                let targets = at(&s, BILLY);
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

                let targets = at(&s, BILLY);
                s.play(OUTBREAK, json!({ "targets": targets }));

                assert_eq!(stolen_ids(&s), vec![s.card(BILLY).id.clone()]);
            }
        }
    }
}
