//! M #57 Clip-Farming Critikal (SPEC §8.8 row 57, R1081): (1) Unit, Common, 5/1 → 10/2.
//!
//! Base:    "Combo: Return the card you played before this to hand."
//! Radiant: "Combo: Return the card you played before this to hand. Make it Radiant."
//! Engine: Combo 1 as its Cry. "The previous card" is `played_ids_this_turn` at the index just
//! before this play (`played_earlier`), casts counted (R70), found wherever it is (R98): from your
//! graveyard or exile it moves to your hand (R746); a permanent you control is Bounced (R692,
//! R747), a unit token vanishing; anywhere else nothing happens. A full hand burns it. Radiant:
//! then `set_radiant` on it, in hand — before the Bounce, so the Bounce carries it home.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-057";

fn critikal(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // Combo 1 (§6.2): one or more cards played earlier this turn, read at play time.
            let at = played_earlier(ctx.state, ctx.controller, ctx.live_self());
            if at < 1 {
                return vec![];
            }
            let ids = played_ids_this_turn(ctx.state, ctx.controller);
            let Some(prev) = ids.get((at - 1) as usize).cloned() else {
                return vec![];
            };
            let Some(card) = find_instance(ctx.state, &prev).cloned() else {
                return vec![];
            };
            // From your graveyard or exile it moves to your hand ("Return … to hand", R746).
            if matches!(card.zone, Zone::Graveyard { player } | Zone::Exile { player } if player == ctx.controller)
            {
                return vec![add_to_hand(json_as(json!({
                    "instance": { "of": "instance", "instanceId": card.id },
                    "radiant": radiant,
                })))];
            }
            // A permanent you control is Bounced (R692, R747); the Radiant face makes it Radiant
            // first, so the Bounce carries it home. Anywhere else nothing happens.
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
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: critikal(false),
        radiant: critikal(true),
    }
}

// M #57 Clip-Farming Critikal — SPEC §8.8 row 57, BUILD M10 row M 57: "Combo: played after another
// card this turn, that previous play (a cast counting, R70) returns from your graveyard or exile
// to your hand, or is Bounced from your side of the field (R1081); a unit token vanishes; a card
// in a deck or a hand, one that has ceased to exist, or one on the opponent's side, nothing;
// played first, nothing; the hand cap burns it; radiant 10/2 and the returned card is made
// Radiant".
#[cfg(test)]
mod tests {
    use super::*;
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
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
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
            fn r1081_r70_a_cast_counts_as_the_previous_play() {
                crate::register_all();
                // The Love Bomb on top of the library casts on the opening draw: a cast is a play
                // (R70), so Critikal returns the Bomb from the graveyard.
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

                assert!(def_ids(&s.hand(P1)).contains(&BOMB.to_string()));
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
                s.play(CRITIKAL, json!({}));

                assert_eq!(def_ids(&s.hand(P1)), vec![FILLER.to_string()]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn stats_are_10_2() {
                crate::register_all();
                let def = crate::card_def(CRITIKAL);
                assert_eq!((def.base.attack, def.base.health), (Some(5), Some(1)));
                assert_eq!((def.radiant.attack, def.radiant.health), (Some(10), Some(2)));
            }

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
}
