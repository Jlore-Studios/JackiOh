//! M #37 CN in a Bottle (SPEC §8.8 row 37): (1) Spell, CN, Epic.
//!
//! Base:    "Cast on draw: Add a random Radiant card to your hand. It costs (1) more (up to (4))."
//! Radiant: "Cast on draw: Add a random Radiant card to your hand. It costs (0)."
//! Engine:
//! - **Cast on draw** is the static flag §2.4's draw reads (R58): drawn, it casts itself at once and
//!   the draw repeats, so its owner draws again. Setup never deals it (R635, R748).
//! - **The card:** `add_random_from_catalog` of one non-token card of every open set but this one
//!   (R380, R387), Radiant, `chinese` (ME-CN), on the base face `costMod: 1` up to (4) (R1704003)
//!   and on the Radiant face `costOverride: 0`. R673's Glitch roll and its own arrival hooks (R151)
//!   apply. Only the owner sees it (R97). A full hand burns it (§2.4, R4).
//! - **The (1) and the play** are R1149001's: a bottle that reaches a hand any other way (an add, a
//!   bounce, a steal, a draw past R58's chain cap) stays there as a (1) Spell, and played from it, it
//!   does the same.
//!
//! Tunes: surcharge 1 ↓, on the base face only; setCost 0 ↓, on the Radiant face only (R1431, R824000,
//! R1748003). The cap of (4) stays printed (R1704003).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-037";

/// R1704003: "It costs (1) more (up to (4))", the printed raise `script_v0_4_4b` keeps (R1748003).
const RAISE: i32 = 1;
const RAISE_UP_TO: i32 = 4;

/// The Radiant face's printed "It costs (0)", which `script_v0_4_4b` keeps.
const FREE: i32 = 0;

/// The bottle; `raised`, the base face's card costs more, up to (4); `declared`, its numbers are the
/// declared `surcharge` and `setCost`, else the printed (1) and (0).
fn bottle(radiant: bool, raised: bool, declared: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |ctx| {
            let mut args = json!({ "radiant": true, "chinese": true });
            if radiant {
                args["costOverride"] = json!(if declared { param(&*ctx, "setCost") } else { FREE });
            } else if raised {
                args["costMod"] = json!(if declared { param(&*ctx, "surcharge") } else { RAISE });
                args["costModUpTo"] = json!(RAISE_UP_TO);
            }
            vec![add_random_from_catalog(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: bottle(false, true, true),
        radiant: bottle(true, true, true),
    }
}

/// R767000: through v0.4.4b its raise and its Radiant face's (0) were printed; `KEPT_SCRIPTS` folds
/// those replays, back to v0.4.0f's, under it.
pub fn script_v0_4_4b() -> CardScripts {
    CardScripts {
        base: bottle(false, true, false),
        radiant: bottle(true, true, false),
    }
}

/// R767000: the scripts v0.4.0f and the versions before it back to `script_v0_3_7b`'s ran, the base
/// face's card at its own cost; `KEPT_SCRIPTS` folds those versions' replays under them.
pub fn script_v0_4_0f() -> CardScripts {
    CardScripts {
        base: bottle(false, false, false),
        radiant: bottle(true, false, false),
    }
}

/// v0.3.7b's bottle: on every arrival in a hand it was replaced in place (R925), and a cast of it did
/// nothing (R926).
fn bottle_v0_3_7b(radiant: bool) -> Script {
    Script {
        enters_hand: Some(hook(move |_ctx| {
            let mut args = json!({
                "target": { "of": "self" },
                "radiant": true,
                "chinese": true,
                "arrives": true,
            });
            if radiant {
                args["costOverride"] = json!(0);
            }
            vec![transform_random(json_as(args))]
        })),
        ..Script::default()
    }
}

/// R767000: the scripts v0.3.7b and every version before it ran, the replacement on arrival in a hand
/// (R925); `KEPT_SCRIPTS` folds those versions' replays under them.
pub fn script_v0_3_7b() -> CardScripts {
    CardScripts {
        base: bottle_v0_3_7b(false),
        radiant: bottle_v0_3_7b(true),
    }
}

// M #37 CN in a Bottle — SPEC §8.8 row 37, BUILD M10 row M 37.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::filler;
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOTTLE: &str = "meditative-037";
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2.
    const FILLER: &str = "core-005";

    /// `effects` applied for p1 on a sink over the scenario's state, then settled; their events.
    fn apply_for_p1(s: &mut Scenario, effects: &[Effect]) -> Vec<GameEvent> {
        let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            {
                let mut ctx = make_context(
                    &mut sink,
                    None,
                    HookOptions {
                        controller: Some(P1),
                        ..HookOptions::default()
                    },
                );
                apply_effects(effects, &mut ctx);
            }
            settle(&mut sink, SettleOptions::default());
        }
        s.state_mut().rng_cursor = rng.cursor();
        events
    }

    /// p1 with a filler in hand and room for more; p2 holds a filler.
    fn quiet(seed: &str) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// A bottle cast for p1 (as its draw would cast it); the card it added, which it then takes back
    /// out of the hand and the bottle out of the graveyard, so the next cast starts from the same piles.
    fn cast_bottle(s: &mut Scenario, radiant: bool) -> CardInstance {
        let _ = apply_for_p1(
            s,
            &[cast_new(json_as(json!({ "def": BOTTLE, "radiant": radiant })))],
        );
        let new_card = s.hand(P1).last().cloned().expect("the card it added");
        s.state_mut().players.p1.hand.pop();
        s.state_mut()
            .players
            .p1
            .graveyard
            .retain(|card| card.def_id != BOTTLE);
        new_card
    }

    /// The decks of a dealt game: p1's holds one bottle.
    fn decks() -> (Vec<String>, Vec<String>) {
        let rest = [
            "core-001", "core-002", "core-003", "core-004", "core-005", "core-006", "core-007", "core-008",
            "core-009", "core-010", "core-011", "core-012", "core-013", "core-014", "core-016", "core-017",
            "core-019", "core-020", "core-025", "core-032", "core-036", "core-043", "core-044", "core-053",
        ];
        let mut mine = vec![BOTTLE.to_string()];
        mine.extend(rest.iter().map(|id| id.to_string()));
        let mut theirs = vec!["core-026".to_string()];
        theirs.extend(rest.iter().map(|id| id.to_string()));
        (mine, theirs)
    }

    /// R1704003: the `costMod` a raise of `by` leaves on a card the bottle added: what takes its price
    /// up to (4) at most, and nothing on an X-cost card.
    fn raise_on(card: &CardInstance, by: i32) -> i32 {
        let def = crate::card_def(&card.def_id);
        if def.cost == CardCost::X {
            return 0;
        }
        by.min((RAISE_UP_TO - query_cost(&def)).max(0))
    }

    fn in_graveyard(s: &Scenario, def_id: &str) -> usize {
        s.state()
            .players
            .p1
            .graveyard
            .iter()
            .filter(|card| card.def_id == def_id)
            .count()
    }

    mod m37_cn_in_a_bottle {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r1149001_drawing_it_casts_it_and_adds_a_random_radiant_chinese_card() {
                // Stockpile draws the bottle on top of the library: it is cast, adds its card, and the
                // draw repeats (R58), so Stockpile's two draws bring two fillers besides.
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "bottle-draw",
                    "p1": {
                        "hand": [{ "def": STOCKPILE }, FILLER],
                        "library": [{ "def": BOTTLE }, FILLER, FILLER, FILLER],
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(STOCKPILE, json!({}));
                let hand = s.hand(P1);
                assert_eq!(hand.len(), 4);
                assert!(hand.iter().all(|card| card.def_id != BOTTLE));
                assert_eq!(in_graveyard(&s, BOTTLE), 1, "cast, it resolved into the graveyard");
                assert_eq!(s.pile(P1, "library").len(), 1);
                let new_cards: Vec<&CardInstance> = hand.iter().filter(|card| card.def_id != FILLER).collect();
                assert_eq!(new_cards.len(), 1);
                let new_card = new_cards[0];
                assert!(new_card.radiant);
                assert_eq!(new_card.chinese, Some(true));
                assert_eq!(new_card.cost_override, None);
                let view: Value = js(&view_for(s.state(), P2));
                assert_eq!(view["opponent"]["hand"], json!({ "count": 4 }));
            }

            #[test]
            fn r1149001_a_full_hand_casts_it_and_burns_the_new_card() {
                // Ten in hand: the turn draw casts it anyway and burns the card it adds (§2.4, R4).
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "bottle-burn",
                    "p1": {
                        "hand": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                        "library": [{ "def": BOTTLE }, FILLER],
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.end_turn();
                s.end_turn();
                assert_eq!(s.hand(P1).len(), 10);
                assert!(s.hand(P1).iter().all(|card| card.def_id == FILLER));
                assert_eq!(in_graveyard(&s, BOTTLE), 1);
                let burned: Vec<String> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::Burned { def_id, .. } if def_id != FILLER => Some(def_id.clone()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(burned.len(), 1, "{burned:?}");
                assert_ne!(burned[0], BOTTLE, "the bottle was cast, not burned");
            }

            #[test]
            fn r1149001_an_add_leaves_it_in_hand_and_playing_it_adds_the_card() {
                let mut s = quiet("bottle-add");
                let _ = apply_for_p1(
                    &mut s,
                    &[jackioh_engine::effects::add_to_hand(json_as(json!({ "defId": BOTTLE })))],
                );
                let held = s
                    .hand(P1)
                    .into_iter()
                    .find(|card| card.def_id == BOTTLE)
                    .expect("the bottle stays in the hand");
                assert_eq!(s.hand(P1).len(), 2);
                assert_eq!(effective_cost(s.state(), &held, Default::default()), 1);

                let mana = s.state().players[P1].mana.current;
                s.play(&held.id, json!({}));
                s.expect_in_zone(&held.id, "graveyard");
                assert_eq!(s.state().players[P1].mana.current, mana - 1);
                let hand = s.hand(P1);
                assert_eq!(hand.len(), 2);
                let new_card = hand.last().cloned().expect("the card it added");
                assert_ne!(new_card.def_id, BOTTLE);
                assert!(new_card.radiant);
                assert_eq!(new_card.chinese, Some(true));
            }

            #[test]
            fn r926_r1149001_it_costs_1_and_declares_a_cry() {
                crate::register_all();
                let s = scenario(json!({
                    "seed": "bottle-cost",
                    "p1": {
                        "hand": [{ "def": BOTTLE }, FILLER],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                let held = s
                    .hand(P1)
                    .into_iter()
                    .find(|card| card.def_id == BOTTLE)
                    .expect("the bottle");
                assert_eq!(effective_cost(s.state(), &held, Default::default()), 1);
                for face in [script().base, script().radiant] {
                    assert!(face.cry.is_some());
                    assert!(face.enters_hand.is_none());
                    assert_eq!(face.static_flags.and_then(|flags| flags.cast_on_draw), Some(true));
                }
            }

            #[test]
            fn r635_r1149001_setup_never_deals_it() {
                crate::register_all();
                for n in 0..200 {
                    let options: CreateGameOptions = json_as(json!({
                        "seed": format!("bottle-deal-{n}"),
                        "decks": [decks().0, decks().1],
                    }));
                    let begun = begin_game(&create_game(&options));
                    assert!(begun.state.players.p1.hand.iter().all(|card| card.def_id != BOTTLE));
                    assert!(!begun.events.iter().any(|event| matches!(
                        event,
                        GameEvent::Transformed { from_def_id, .. } if from_def_id == BOTTLE
                    )));
                    // A mulligan of the whole hand never draws it either (R635).
                    let mut state = begun.state;
                    for player in [P1, P2] {
                        let action: Action = json_as(json!({
                            "type": "mulligan",
                            "keep": [],
                            "playerId": player,
                            "nonce": format!("bottle-mulligan-{n}-{player}"),
                        }));
                        let result = reduce(&state, &action);
                        assert!(result.error.is_none(), "{:?}", result.error);
                        state = result.state;
                    }
                    assert!(state.players.p1.hand.iter().all(|card| card.def_id != BOTTLE));
                }
            }

            /// The cards `casts` bottles add, each with its price out of play (R65: an embiggen card's
            /// base; `None` for an X-cost card) and the `costMod` it reached the hand with.
            fn raised(seed: &str, casts: usize) -> Vec<(Option<i32>, i32)> {
                let mut s = quiet(seed);
                (0..casts)
                    .map(|_| {
                        let new_card = cast_bottle(&mut s, false);
                        let def = crate::card_def(&new_card.def_id);
                        let printed = (def.cost != CardCost::X).then(|| query_cost(&def));
                        (printed, new_card.cost_mod)
                    })
                    .collect()
            }

            #[test]
            fn r1704003_at_3_or_less_costs_1_more() {
                let seen: Vec<(Option<i32>, i32)> = raised("bottle-raise-low", 80)
                    .into_iter()
                    .filter(|(printed, _)| printed.is_some_and(|cost| cost <= 3))
                    .collect();
                assert!(seen.iter().any(|(printed, _)| *printed == Some(3)), "{seen:?}");
                assert!(seen.iter().all(|(_, cost_mod)| *cost_mod == 1), "{seen:?}");
            }

            #[test]
            fn r1704003_at_4_or_more_unchanged() {
                let seen: Vec<(Option<i32>, i32)> = raised("bottle-raise-high", 80)
                    .into_iter()
                    .filter(|(printed, _)| printed.is_none_or(|cost| cost >= 4))
                    .collect();
                assert!(seen.iter().any(|(printed, _)| *printed == Some(4)), "{seen:?}");
                assert!(seen.iter().all(|(_, cost_mod)| *cost_mod == 0), "{seen:?}");
            }

            #[test]
            fn r387_the_new_card_is_a_non_token_card_of_any_set_but_never_itself() {
                let mut s = quiet("bottle-sweep");
                let mut sets: Vec<SetName> = Vec::new();
                for _ in 0..60 {
                    let new_card = cast_bottle(&mut s, false);
                    let def = crate::card_def(&new_card.def_id);
                    assert_ne!(def.id, BOTTLE, "never itself (R387)");
                    assert!(!def.token, "{} is a token", def.id);
                    if !sets.contains(&def.set) {
                        sets.push(def.set);
                    }
                }
                assert!(sets.len() > 1, "of any set: {sets:?}");
            }

            #[test]
            fn r97_the_new_card_is_hidden_from_the_opponent() {
                let mut s = quiet("bottle-hidden");
                let _ = apply_for_p1(&mut s, &[cast_new(json_as(json!({ "def": BOTTLE })))]);
                let new_card = s.hand(P1).last().cloned().expect("the card it added");
                assert_ne!(new_card.def_id, FILLER);
                let theirs = serde_json::to_string(&s.view(P2)).expect("the view serializes");
                assert!(!theirs.contains(&new_card.id), "p2 never reads the new card's id");
                assert!(
                    !theirs.contains(&new_card.def_id),
                    "p2 never reads {}",
                    new_card.def_id
                );
                let mine = serde_json::to_string(&s.view(P1)).expect("the view serializes");
                assert!(mine.contains(&new_card.def_id), "p1 reads its own card");
            }

            #[test]
            fn r673_the_new_card_is_generated_for_glitch() {
                let mut s = quiet("bottle-glitch");
                s.state_mut().system_plays = Some(GLITCH_ODDS_DENOMINATOR / GLITCH_ODDS_PER_SYSTEM_PLAY);
                let new_card = cast_bottle(&mut s, false);
                assert_eq!(new_card.def_id, GLITCH_DEF_ID);
            }

            #[test]
            fn r151_the_new_card_runs_its_own_arrival_hooks() {
                // A Heroic Power rolls its power as it arrives (R43): one a bottle adds has.
                const HEROIC_POWER: &str = "core-098";
                let mut s = quiet("bottle-arrives");
                let mut found = None;
                for _ in 0..3000 {
                    let new_card = cast_bottle(&mut s, false);
                    if new_card.def_id == HEROIC_POWER {
                        found = Some(new_card);
                        break;
                    }
                }
                let power = found.expect("some bottle adds a Heroic Power");
                assert!(
                    power.memory.get("power").is_some_and(Value::is_string),
                    "{:?}",
                    power.memory
                );
            }
        }

        #[test]
        fn r1149001_r767000_a_game_on_v0_3_7_folds_under_the_kept_script_a_draw_replaces_it_in_place() {
            crate::register_all();
            let shipped = crate::shipped_catalog("v0.3.0c").expect("v0.3.0c is carried");
            jackioh_engine::with_shipped(Some(shipped), || {
                let mut s = scenario(json!({
                    "seed": "bottle-v0-3-7",
                    "p1": {
                        "hand": [{ "def": STOCKPILE }, FILLER],
                        "library": [{ "def": BOTTLE }, FILLER, FILLER, FILLER],
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(STOCKPILE, json!({}));
                // Replaced where it arrived (R925), not cast: two draws, no repeat.
                assert_eq!(s.hand(P1).len(), 3);
                assert_eq!(in_graveyard(&s, BOTTLE), 0);
                assert!(s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::Transformed { from_def_id, .. } if from_def_id == BOTTLE
                )));
            });
        }

        #[test]
        fn r1748003_surcharge_moves_with_a_buff_or_a_nerf() {
            // A Buff finds it at its floor of 1 (an amount never drops below 1, R386); a Nerf in the
            // hand makes the card it adds cost (2) more, still up to (4).
            let mut s = quiet("bottle-surcharge");
            let _ = apply_for_p1(
                &mut s,
                &[jackioh_engine::effects::add_to_hand(json_as(json!({ "defId": BOTTLE })))],
            );
            let held = s
                .hand(P1)
                .into_iter()
                .find(|card| card.def_id == BOTTLE)
                .expect("the bottle stays in the hand");
            assert!(!crate::can_upgrade_number(&s, &held.id, "surcharge"));
            assert_eq!(crate::degrade_number(&mut s, &held.id, "surcharge"), 2);
            s.play(&held.id, json!({}));
            let new_card = s.hand(P1).last().cloned().expect("the card it added");
            assert_ne!(new_card.def_id, BOTTLE);
            assert_eq!(new_card.cost_mod, raise_on(&new_card, 2));
        }

        #[test]
        fn r1748003_r767000_the_v0_4_4b_kept_script_keeps_its_printed_surcharge_and_set_cost() {
            crate::under_kept(BOTTLE, super::super::script_v0_4_4b(), || {
                for radiant in [false, true] {
                    let mut s = quiet("bottle-kept-4-4b");
                    let _ = apply_for_p1(
                        &mut s,
                        &[jackioh_engine::effects::add_to_hand(json_as(
                            json!({ "defId": BOTTLE, "radiant": radiant }),
                        ))],
                    );
                    let held = s
                        .hand(P1)
                        .into_iter()
                        .find(|card| card.def_id == BOTTLE)
                        .expect("the bottle stays in the hand");
                    let key = if radiant { "setCost" } else { "surcharge" };
                    crate::degrade_number(&mut s, &held.id, key);
                    s.play(&held.id, json!({}));
                    let new_card = s.hand(P1).last().cloned().expect("the card it added");
                    if radiant {
                        assert_eq!(new_card.cost_override, Some(0), "the kept script prints (0)");
                    } else {
                        assert_eq!(new_card.cost_mod, raise_on(&new_card, RAISE), "its printed (1)");
                    }
                }
            });
        }

        #[test]
        fn r1704003_r767000_through_v0_4_0f_the_kept_script_adds_its_card_at_its_own_cost() {
            crate::under_kept(BOTTLE, super::super::script_v0_4_0f(), || {
                let mut s = quiet("bottle-kept");
                for _ in 0..20 {
                    assert_eq!(cast_bottle(&mut s, false).cost_mod, 0);
                }
            });
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_new_card_costs_0() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "bottle-radiant",
                    "p1": {
                        "hand": [{ "def": STOCKPILE }, FILLER],
                        "library": [{ "def": BOTTLE, "radiant": true }, FILLER, FILLER, FILLER],
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(STOCKPILE, json!({}));
                let hand = s.hand(P1);
                assert_eq!(hand.len(), 4);
                assert!(hand.iter().all(|card| card.def_id != BOTTLE));
                let new_cards: Vec<&CardInstance> = hand.iter().filter(|card| card.def_id != FILLER).collect();
                assert_eq!(new_cards.len(), 1);
                assert!(new_cards[0].radiant);
                assert_eq!(new_cards[0].chinese, Some(true));
                assert_eq!(new_cards[0].cost_override, Some(0));
            }

            #[test]
            fn r1704003_costs_0_no_raise() {
                let mut s = quiet("bottle-radiant-raise");
                for _ in 0..20 {
                    let new_card = cast_bottle(&mut s, true);
                    assert_eq!(new_card.cost_override, Some(0));
                    assert_eq!(new_card.cost_mod, 0);
                }
            }

            #[test]
            fn r1748003_set_cost_moves_with_a_buff_or_a_nerf() {
                // A Buff finds it at its floor of (0); a Nerf in the hand makes the card it adds cost (1).
                let mut s = quiet("bottle-radiant-set-cost");
                let _ = apply_for_p1(
                    &mut s,
                    &[jackioh_engine::effects::add_to_hand(json_as(
                        json!({ "defId": BOTTLE, "radiant": true }),
                    ))],
                );
                let held = s
                    .hand(P1)
                    .into_iter()
                    .find(|card| card.def_id == BOTTLE)
                    .expect("the bottle stays in the hand");
                assert!(!crate::can_upgrade_number(&s, &held.id, "setCost"));
                assert_eq!(crate::degrade_number(&mut s, &held.id, "setCost"), 1);
                s.play(&held.id, json!({}));
                let new_card = s.hand(P1).last().cloned().expect("the card it added");
                assert_ne!(new_card.def_id, BOTTLE);
                assert_eq!(new_card.cost_override, Some(1));
            }

            #[test]
            fn r1149001_played_from_a_hand_the_new_card_costs_0_too() {
                let mut s = quiet("bottle-radiant-play");
                let _ = apply_for_p1(
                    &mut s,
                    &[jackioh_engine::effects::add_to_hand(json_as(
                        json!({ "defId": BOTTLE, "radiant": true }),
                    ))],
                );
                let held = s
                    .hand(P1)
                    .into_iter()
                    .find(|card| card.def_id == BOTTLE)
                    .expect("the bottle stays in the hand");
                s.play(&held.id, json!({}));
                let new_card = s.hand(P1).last().cloned().expect("the card it added");
                assert_ne!(new_card.def_id, BOTTLE);
                assert!(new_card.radiant);
                assert_eq!(new_card.cost_override, Some(0));
            }
        }
    }
}
