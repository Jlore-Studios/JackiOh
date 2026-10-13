//! M #37 CN in a bottle (SPEC §8.8 row 37): (X) Spell, CN, Epic.
//!
//! Base:    "When this enters your hand: Replace it with a random Radiant card."
//! Radiant: "When this enters your hand: Replace it with a random Radiant card. It costs (0)."
//! Engine:
//! - **The hook:** the NEW Script hook `enters_hand`, run by `draw::run_arrival_hooks` on a hand
//!   arrival, beside R151's start-of-game clause (MD-B18, R925).
//! - **The replacement:** `transform_random` of the card itself in the hand (§6.3 Replace, R31;
//!   R671 keeps its place). The pool is the non-token cards of every open set except this one
//!   (R380, R387). The new card is Radiant and carries NEW riders: `chinese` (ME-CN) and, on the
//!   Radiant face, `costOverride: 0`.
//! - **What follows:** the replacement is a card generated into a hand, so R673's Glitch roll
//!   applies. It runs its own arrival hooks (R151, the `arrives` rider), so a Heroic Power rolls its
//!   power. Only the owner sees any of it (R97).
//! - **The (X)** is printed only (MD-B19, R926): the card is never played from a hand, it costs 0
//!   wherever a rule reads it out of play (R65), and a cast of it resolves with no effect. A full
//!   hand burns the card before it enters (§2.4, R4), so it reaches the graveyard unreplaced.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-037";

fn bottle(radiant: bool) -> Script {
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

pub fn script() -> CardScripts {
    // No cry on either face: a cast resolves with no effect (MD-B19, R926). The Radiant face's new
    // card costs (0).
    let base = bottle(false);
    let radiant = bottle(true);
    CardScripts { base, radiant }
}

// M #37 CN in a bottle — SPEC §8.8 row 37, BUILD M10 row M 37: "On every arrival in a hand (a draw,
// an add, a Discover, a bounce, a steal, the opening deal, a mulligan replacement; MD-B18) it is
// replaced in place by a random non-token Radiant card of any set but this one, carrying `chinese`,
// hidden from the opponent (R97); the new card is generated for R673 and runs its own arrival
// hooks (R151); a full hand burns it unreplaced; a cast or a random play of it does nothing, and
// out of play it costs 0 (MD-B19); radiant the new card costs (0)".
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

    /// A bottle added to p1's hand; the card that took its place.
    fn add_bottle(s: &mut Scenario, radiant: bool) -> CardInstance {
        let _ = apply_for_p1(
            s,
            &[jackioh_engine::effects::add_to_hand(json_as(
                json!({ "defId": BOTTLE, "radiant": radiant }),
            ))],
        );
        s.hand(P1).last().cloned().expect("the replacement")
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

    fn bottle_transformed(events: &[GameEvent]) -> bool {
        events
            .iter()
            .any(|event| matches!(event, GameEvent::Transformed { from_def_id, .. } if from_def_id == BOTTLE))
    }

    mod m37_cn_in_a_bottle {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r925_a_draw_replaces_it_in_place_radiant_chinese_hidden_from_the_opponent() {
                // Stockpile draws the bottle on top of the library, then a filler: the bottle is
                // replaced where it arrived.
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
                assert_eq!(hand.len(), 3);
                assert!(hand.iter().all(|card| card.def_id != BOTTLE));
                let new_card = hand[1].clone();
                assert_ne!(new_card.def_id, BOTTLE);
                assert!(new_card.radiant);
                assert_eq!(new_card.chinese, Some(true));
                assert_eq!(new_card.cost_override, None);
                // The filler drawn after it lands behind it: the replacement kept its place.
                assert_eq!(hand[2].def_id, FILLER);
                // The opponent sees a count, never the card (R97).
                let view: Value = serde_json::to_value(view_for(s.state(), P2)).expect("the view serialises");
                assert_eq!(view["opponent"]["hand"], json!({ "count": 3 }));
            }

            #[test]
            fn r925_a_full_hand_burns_it_unreplaced() {
                // Ten in hand and the bottle alone in the library: the turn draw burns it before it
                // enters (§2.4, R4), so it reaches the graveyard as itself.
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "bottle-burn",
                    "p1": {
                        "hand": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                        "library": [{ "def": BOTTLE }],
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.end_turn();
                s.end_turn();
                assert_eq!(s.hand(P1).len(), 10);
                assert!(s.hand(P1).iter().all(|card| card.def_id != BOTTLE));
                let burned: Vec<&CardInstance> = s
                    .state()
                    .players
                    .p1
                    .graveyard
                    .iter()
                    .filter(|card| card.def_id == BOTTLE)
                    .collect();
                assert_eq!(burned.len(), 1);
            }

            #[test]
            fn r926_out_of_play_it_costs_0_and_declares_no_cry() {
                // The (X) is printed only: out of play the card costs 0 (R65).
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
                assert_eq!(effective_cost(s.state(), &held, Default::default()), 0);
                // And neither face runs a cry: a cast resolves with no effect (MD-B19).
                assert!(script().base.cry.is_none());
                assert!(script().radiant.cry.is_none());
                assert!(script().base.enters_hand.is_some());
                assert!(script().radiant.enters_hand.is_some());
            }

            #[test]
            fn r925_an_add_replaces_it_in_place() {
                let mut s = quiet("bottle-add");
                let new_card = add_bottle(&mut s, false);
                assert_eq!(s.hand(P1).len(), 2);
                assert_ne!(new_card.def_id, BOTTLE);
                assert!(new_card.radiant);
                assert_eq!(new_card.chinese, Some(true));
            }

            #[test]
            fn r925_the_opening_deal_and_a_mulligan_replacement_replace_it() {
                crate::register_all();
                let (mut dealt, mut replaced) = (0, 0);
                for n in 0..200 {
                    let options: CreateGameOptions = json_as(json!({
                        "seed": format!("bottle-deal-{n}"),
                        "decks": [decks().0, decks().1],
                    }));
                    let begun = begin_game(&create_game(&options));
                    assert!(
                        begun
                            .state
                            .players
                            .p1
                            .hand
                            .iter()
                            .all(|card| card.def_id != BOTTLE)
                    );
                    if bottle_transformed(&begun.events) {
                        dealt += 1;
                        continue;
                    }
                    if replaced > 0 {
                        continue;
                    }
                    // Not dealt: p1 sends its whole hand back, so the bottle may come as a replacement.
                    let kept: Vec<String> = begun
                        .state
                        .players
                        .p2
                        .hand
                        .iter()
                        .map(|card| card.id.clone())
                        .collect();
                    let mut state = begun.state;
                    for (player, keep) in [(P1, Vec::new()), (P2, kept)] {
                        let action: Action = json_as(json!({
                            "type": "mulligan",
                            "keep": keep,
                            "playerId": player,
                            "nonce": format!("bottle-mulligan-{n}-{player}"),
                        }));
                        let result = reduce(&state, &action);
                        assert!(result.error.is_none(), "{:?}", result.error);
                        if bottle_transformed(&result.events) {
                            replaced += 1;
                        }
                        state = result.state;
                    }
                    assert!(state.players.p1.hand.iter().all(|card| card.def_id != BOTTLE));
                }
                assert!(dealt > 0, "some seed deals the bottle");
                assert!(replaced > 0, "some mulligan draws the bottle");
            }

            #[test]
            fn r387_the_new_card_is_a_non_token_card_of_any_set_but_never_itself() {
                let mut s = quiet("bottle-sweep");
                let mut sets: Vec<SetName> = Vec::new();
                for _ in 0..60 {
                    let new_card = add_bottle(&mut s, false);
                    let def = crate::card_def(&new_card.def_id);
                    assert_ne!(def.id, BOTTLE, "never itself (R387)");
                    assert!(!def.token, "{} is a token", def.id);
                    if !sets.contains(&def.set) {
                        sets.push(def.set);
                    }
                    s.state_mut().players.p1.hand.pop();
                }
                assert!(sets.len() > 1, "of any set: {sets:?}");
            }

            #[test]
            fn r97_the_new_card_is_hidden_from_the_opponent() {
                let mut s = quiet("bottle-hidden");
                let new_card = add_bottle(&mut s, false);
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
                // With Glitch's odds at certainty, the replacement is a Glitch.
                let mut s = quiet("bottle-glitch");
                s.state_mut().system_plays = Some(GLITCH_ODDS_DENOMINATOR / GLITCH_ODDS_PER_SYSTEM_PLAY);
                let new_card = add_bottle(&mut s, false);
                assert_eq!(new_card.def_id, GLITCH_DEF_ID);
            }

            #[test]
            fn r151_the_new_card_runs_its_own_arrival_hooks() {
                // A Heroic Power rolls its power as it arrives (R43): one that replaces a bottle has.
                const HEROIC_POWER: &str = "core-098";
                let mut s = quiet("bottle-arrives");
                let mut found = None;
                for _ in 0..3000 {
                    let new_card = add_bottle(&mut s, false);
                    if new_card.def_id == HEROIC_POWER {
                        found = Some(new_card);
                        break;
                    }
                    s.state_mut().players.p1.hand.pop();
                }
                let power = found.expect("some bottle becomes a Heroic Power");
                assert!(
                    power.memory.get("power").is_some_and(Value::is_string),
                    "{:?}",
                    power.memory
                );
            }

            #[test]
            fn r926_a_cast_or_a_random_play_of_it_does_nothing() {
                // Played from a hand it was put in, it resolves with no effect.
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "bottle-play",
                    "p1": { "hand": [{ "def": BOTTLE }, FILLER], "library": filler(4) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                let held = s
                    .hand(P1)
                    .into_iter()
                    .find(|card| card.def_id == BOTTLE)
                    .expect("the bottle");
                let before = s.state().players[P1].clone();
                s.play(&held.id, json!({ "x": 1 }));
                s.expect_in_zone(&held.id, "graveyard");
                let after = &s.state().players[P1];
                assert_eq!(after.hand.len(), before.hand.len() - 1, "nothing added");
                assert_eq!(after.library.len(), before.library.len(), "nothing drawn");
                assert_eq!(after.hero, before.hero);
                // Cast at random by an effect, it does nothing either.
                let mut s = quiet("bottle-random-cast");
                let before = s.state().players[P1].clone();
                let events = apply_for_p1(
                    &mut s,
                    &[cast_new(json_as(json!({ "def": BOTTLE, "random": true })))],
                );
                let after = &s.state().players[P1];
                assert_eq!(after.hand.len(), before.hand.len());
                assert_eq!(after.library.len(), before.library.len());
                assert_eq!(after.hero, before.hero);
                assert!(!bottle_transformed(&events));
            }
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
                assert_eq!(hand.len(), 3);
                assert!(hand.iter().all(|card| card.def_id != BOTTLE));
                let new_card = hand[1].clone();
                assert!(new_card.radiant);
                assert_eq!(new_card.chinese, Some(true));
                assert_eq!(new_card.cost_override, Some(0));
            }
        }
    }
}
