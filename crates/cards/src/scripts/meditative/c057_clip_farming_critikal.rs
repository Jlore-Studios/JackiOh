//! M #57 Clip-Farming Critikal (SPEC §8.8 row 57, R1081): (1) Unit, Common, 4/1 → 8/2.
//!
//! Base:    "Combo: Return the card you played before this to hand."
//! Radiant: "Combo: Return the card you played before this to hand. Make it Radiant."
//! Engine: Combo 1 as its Cry. "The previous card" is `played_ids_this_turn` at the index just
//! before this play (`played_earlier`), casts counted (R70) but not a cast on draw (R2179002), found wherever it is (R98): from your
//! graveyard or exile it moves to your hand (R746); a permanent you control is Bounced (R692,
//! R747), a unit token vanishing; anywhere else nothing happens. A full hand burns it. Radiant:
//! then `set_radiant` on it, in hand — before the Bounce, so the Bounce carries it home.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-057";

/// Combo 1 (§6.2): the card played just before `me` this turn (a cast counting, R70), read at play
/// time, if it still exists; `None` when `me` was played first.
fn previous_play(state: &GameState, controller: PlayerId, me: Option<&CardInstance>) -> Option<CardInstance> {
    let at = played_earlier(state, controller, me);
    if at < 1 {
        return None;
    }
    let prev = played_ids_this_turn(state, controller).get((at - 1) as usize).cloned()?;
    find_instance(state, &prev).cloned()
}

/// Whether the Cry returns `card`: from its controller's graveyard or exile, or Bounced as a permanent
/// it controls (R746, R692, R747). Anywhere else nothing happens.
fn would_return(card: &CardInstance, controller: PlayerId) -> bool {
    matches!(card.zone, Zone::Graveyard { player } | Zone::Exile { player } if player == controller)
        || (matches!(card.zone, Zone::Field { .. }) && card.controller == controller)
}

/// R2476000: in its owner's hand, the card a play now would return, after "the card you played before
/// this", by name, or the dash when none would come back (played first, or that card is beyond its
/// reach). Hand only, so only its owner, who made that play, reads it: on the field the Combo is spent,
/// and a public value could name a face-down card set before it.
fn preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if ctx.zone != ConditionZone::Hand {
            return vec![];
        }
        let back = previous_play(ctx.state, ctx.controller, Some(ctx.self_))
            .filter(|card| would_return(card, ctx.controller));
        vec![card_preview(
            ctx.state,
            "the card you played before this",
            back.as_ref().map(|card| (card.def_id.as_str(), card.radiant)),
        )]
    })
}

fn critikal(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let Some(card) = previous_play(ctx.state, ctx.controller, ctx.live_self()) else {
                return vec![];
            };
            if matches!(card.zone, Zone::Graveyard { player } | Zone::Exile { player } if player == ctx.controller)
            {
                return vec![add_to_hand(json_as(json!({
                    "instance": { "of": "instance", "instanceId": card.id },
                    "radiant": radiant,
                })))];
            }
            if matches!(card.zone, Zone::Field { .. }) && card.controller == ctx.controller {
                if radiant {
                    return vec![
                        set_radiant(json_as(json!({ "instanceId": card.id }))),
                        bounce(json_as(json!({
                            "target": { "of": "instance", "instanceId": card.id },
                        }))),
                    ];
                }
                return vec![bounce(json_as(json!({
                    "target": { "of": "instance", "instanceId": card.id },
                })))];
            }
            vec![]
        })),
        // R195: hand only, the same read as the Cry, so the glow and the return cannot disagree.
        condition_met: Some(condition_hook(|ctx: ConditionContext<'_>| {
            ctx.zone == ConditionZone::Hand
                && played_earlier(ctx.state, ctx.controller, ctx.self_) >= 1
        })),
        preview: Some(preview()),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: critikal(false),
        radiant: critikal(true),
    }
}

// M #57 Clip-Farming Critikal — SPEC §8.8 row 57, BUILD M10 row M 57.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::def_ids;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const CRITIKAL: &str = "meditative-057";
    /// (1) Spell: draws and heals, then waits in the graveyard.
    const STOCKPILE: &str = "core-005";
    /// (1) 4/4 Unit with no hooks: the Bounce target.
    const VANILLA: &str = "core-008";
    /// (1) 1/1 Unit, Divine Shield, Reborn: the ceased-to-exist target.
    const DEFENDER: &str = "core-003";
    /// (1) Spell, deals 3 to a target.
    const ECLIPSE: &str = "core-035";
    /// Cast on draw: heals 7, then waits in the graveyard.
    const BOMB: &str = "meditative-030-1";
    /// (3) Field Spell, inert in hand and library. Never Stockpile: the returned Stockpile must be
    /// the only one in hand, and Stockpile's own draws must not look like it.
    const FILLER: &str = "core-006";

    fn filler(count: usize) -> Value {
        json!(vec![FILLER; count])
    }

    mod m57_clip_farming_critikal {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn played_first_nothing_returns() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "critikal-first",
                    "p1": { "hand": [CRITIKAL, FILLER], "library": filler(3) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(CRITIKAL, json!({}));

                assert_eq!(def_ids(&s.hand(P1)), vec![FILLER.to_string()]);
            }

            #[test]
            fn previous_spell_returns_from_graveyard() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "critikal-graveyard",
                    "p1": { "hand": [STOCKPILE, CRITIKAL, FILLER], "library": filler(4) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(STOCKPILE, json!({}));
                assert!(!def_ids(&s.hand(P1)).contains(&STOCKPILE.to_string()));
                s.play(CRITIKAL, json!({}));

                assert!(def_ids(&s.hand(P1)).contains(&STOCKPILE.to_string()));
            }

            #[test]
            fn r1081_r2179002_a_cast_on_draw_is_no_previous_play() {
                crate::register_all();
                // The Love Bomb on top of the library casts on the opening draw: a cast on draw is no
                // play (R2179002), so Critikal is the turn's first card and returns nothing.
                let mut s = scenario(json!({
                    "seed": "critikal-cast",
                    "p1": {
                        "hand": [CRITIKAL, FILLER],
                        "library": [BOMB, FILLER, FILLER, FILLER],
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.start_turn();
                s.play(CRITIKAL, json!({}));

                assert!(!def_ids(&s.hand(P1)).contains(&BOMB.to_string()));
                assert!(def_ids(&s.pile(P1, "graveyard")).contains(&BOMB.to_string()));
            }

            #[test]
            fn previous_unit_is_bounced_from_your_field() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "critikal-bounce",
                    "p1": { "hand": [VANILLA, CRITIKAL, FILLER], "library": filler(4) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(VANILLA, json!({}));
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(VANILLA.to_string()));
                s.play(CRITIKAL, json!({}));

                assert!(s.unit(P1, 1).is_none() || s.unit(P1, 1).unwrap().def_id == CRITIKAL);
                assert!(def_ids(&s.hand(P1)).contains(&VANILLA.to_string()));
            }

            #[test]
            fn a_previous_play_that_ceased_to_exist_returns_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "critikal-ceased",
                    "p1": { "hand": [DEFENDER, CRITIKAL, FILLER], "library": filler(4) },
                    "p2": {
                        "mana": 10,
                        "hand": [ECLIPSE, ECLIPSE, FILLER],
                        "library": filler(4),
                    },
                }));
                s.play(DEFENDER, json!({}));
                // Two hits: the first pops the shield, the second destroys it, and Reborn puts a
                // new body back under a new id — the played id has ceased to exist.
                s.end_turn();
                let defender = s.card(DEFENDER).id.clone();
                s.play(ECLIPSE, json!({ "targets": [{ "pick": "instance", "instanceId": defender }] }));
                let defender = s.card(DEFENDER).id.clone();
                s.play(ECLIPSE, json!({ "targets": [{ "pick": "instance", "instanceId": defender }] }));
                s.end_turn();
                // p1's hand is Critikal, the filler and the turn's draw: Critikal leaves it and
                // nothing comes back.
                let mut before = def_ids(&s.hand(P1));
                before.retain(|id| id != CRITIKAL);
                s.play(CRITIKAL, json!({}));

                assert_eq!(def_ids(&s.hand(P1)), before);
                assert!(!def_ids(&s.hand(P1)).contains(&DEFENDER.to_string()));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_returned_graveyard_card_is_made_radiant() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "critikal-radiant-graveyard",
                    "p1": {
                        "hand": [STOCKPILE, { "def": CRITIKAL, "radiant": true }, FILLER],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(STOCKPILE, json!({}));
                s.play(CRITIKAL, json!({}));

                let back = s.hand(P1).into_iter().find(|card| card.def_id == STOCKPILE);
                assert_eq!(back.map(|card| card.radiant), Some(true));
            }

            #[test]
            fn the_bounced_unit_is_made_radiant_first() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "critikal-radiant-bounce",
                    "p1": {
                        "hand": [VANILLA, { "def": CRITIKAL, "radiant": true }, FILLER],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(VANILLA, json!({}));
                s.play(CRITIKAL, json!({}));

                let back = s.hand(P1).into_iter().find(|card| card.def_id == VANILLA);
                assert_eq!(back.map(|card| card.radiant), Some(true));
            }
        }
    }

    /// R2476000: in hand, after "the card you played before this", the card its play would return: the
    /// dash before any play, then Stockpile once played, and the play returns it to hand.
    #[test]
    fn r2476000_in_hand_it_names_the_card_its_play_would_return() {
        crate::register_all();
        for radiant in [false, true] {
            let mut s = crate::scenario(json!({
                "p1": { "hand": [STOCKPILE, { "def": CRITIKAL, "radiant": radiant }, VANILLA], "library": [VANILLA, VANILLA, VANILLA], "mana": 10 },
                "p2": { "hand": [STOCKPILE], "library": [VANILLA, VANILLA] },
            }));
            let critikal = s.card(CRITIKAL).id.clone();
            let label = "the card you played before this".to_string();
            assert_eq!(crate::preview_seen(&s, P1, &critikal), vec![(label.clone(), 0, Some("—".to_string()))]);
            let stockpile = s.hand(P1).into_iter().find(|card| card.def_id == STOCKPILE).expect("p1's Stockpile").id;
            s.play(stockpile.as_str(), json!({}));
            assert_eq!(crate::preview_seen(&s, P1, &critikal), vec![(label, 1, Some("Stockpile".to_string()))]);
            s.play(critikal.as_str(), json!({ "zone": 1 }));
            s.expect_in_zone(stockpile.as_str(), "hand");
        }
    }
}
