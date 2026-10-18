//! M #37 CN in a Bottle (SPEC §8.8 row 37): (1) Spell, CN, Epic.
//!
//! Base:    "Cast on draw: Add a random Radiant card to your hand."
//! Radiant: "Cast on draw: Add a random Radiant card to your hand. It costs (0)."
//! Engine:
//! - **Cast on draw** is the static flag §2.4's draw reads (R58): drawn, it casts itself at once and
//!   the draw repeats, so its owner draws again. Setup never deals it (R635, R748).
//! - **The card:** `add_random_from_catalog` of one non-token card of every open set but this one
//!   (R380, R387), Radiant, carrying the NEW rider `chinese` (ME-CN) and, on the Radiant face,
//!   `costOverride: 0`. It is a card generated into a hand, so R673's Glitch roll applies, and it
//!   runs its own arrival hooks (R151), so a Heroic Power rolls its power. Only the owner sees it
//!   (R97). A full hand burns it (§2.4, R4).
//! - **The (1) and the play** are R1149001's: a bottle that reaches a hand any other way (an add, a
//!   bounce, a steal, a draw past R58's chain cap) stays there as a (1) Spell, and played from it, it
//!   does the same.
//!
//! Through patch v0.3.7b it cost (X) and was replaced in place on any arrival in a hand (the
//! `enters_hand` hook, R925, R926); `script_v0_3_7b` keeps that for those versions' replays
//! (R767000).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-037";

fn bottle(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |_ctx| {
            let mut args = json!({ "radiant": true, "chinese": true });
            if radiant {
                args["costOverride"] = json!(0);
            }
            vec![add_random_from_catalog(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: bottle(false),
        radiant: bottle(true),
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

// M #37 CN in a Bottle — SPEC §8.8 row 37, BUILD M10 row M 37: "Cast on draw (R58, R1149001): drawn,
// it is cast and adds a random non-token Radiant card of any set but this one, carrying `chinese`,
// hidden from the opponent (R97); the new card is generated for R673 and runs its own arrival hooks
// (R151); a full hand burns the new card; setup never deals it (R635); added to a hand without a draw
// it stays there at (1), and played it does the same; radiant the new card costs (0)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOTTLE: &str = "meditative-037";
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2.
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// `effects` applied for p1 on a sink over the scenario's state, then settled; the rng cursor
    /// written back; their events.
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
            "core-019", "core-020", "core-025",
        ];
        let mut mine = vec![BOTTLE.to_string()];
        mine.extend(rest.iter().map(|id| id.to_string()));
        let mut theirs = vec!["core-026".to_string()];
        theirs.extend(rest.iter().map(|id| id.to_string()));
        (mine, theirs)
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
                // The opponent sees a count, never the card (R97).
                let view: Value = serde_json::to_value(view_for(s.state(), P2)).expect("the view serialises");
                assert_eq!(view["opponent"]["hand"], json!({ "count": 4 }));
            }

            #[test]
            fn r1149001_a_full_hand_casts_it_and_burns_the_new_card() {
                // Ten in hand and the bottle on top of the library: the turn draw casts it anyway, and
                // the card it adds finds no room and is burned (§2.4, R4).
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
                // The printed-only (X) is gone: it costs (1) like any card in a hand.
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
                let theirs = serde_json::to_string(&s.view(P2)).expect("the view serialises");
                assert!(!theirs.contains(&new_card.id), "p2 never reads the new card's id");
                assert!(
                    !theirs.contains(&new_card.def_id),
                    "p2 never reads {}",
                    new_card.def_id
                );
                let mine = serde_json::to_string(&s.view(P1)).expect("the view serialises");
                assert!(mine.contains(&new_card.def_id), "p1 reads its own card");
            }

            #[test]
            fn r673_the_new_card_is_generated_for_glitch() {
                // With Glitch's odds at certainty, the new card is a Glitch.
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
            let shipped = crate::shipped_catalog("v0.3.7").expect("v0.3.7 is carried");
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
