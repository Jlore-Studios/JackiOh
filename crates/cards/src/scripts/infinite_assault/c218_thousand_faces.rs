//! IA #218 Thousand Faces (SPEC §8.13 row 218), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Your face-down cards can't be destroyed. Once per turn, when you set a card
//!            face-down, draw a card."
//!   Radiant: "Your face-down cards can't be destroyed. Twice per turn, when you set a card
//!            face-down, draw a card."
//!
//! The guard is `StaticFlags.face_down_indestructible`: a destroy from either side is stopped at
//! §4.5 step 1, but reveal, Exile, bounce, steal and transform are not (R4594001). A card its
//! controller sets face-down arrives face-down in its controller's backrow — a face-down play and
//! a summon landing face-down both emit `summoned` with the backrow row — so the one arrival event
//! answers each set exactly once (R4586003); face-up arrivals never do (R61). The draw cap is per
//! turn on its controller's memory, on either player's turn; simultaneous sets count in order.
//!
//! Engine: `StaticFlags.face_down_indestructible`, `draw` on `summoned` (R4594001). Built by C10
//! (#4594).

use jackioh_engine::prelude::*;
use jackioh_engine::preview::is_face_down;

pub const ID: &str = "infiniteassault-218";

/// This turn's draws so far: the turn and the count, on the card's own memory.
const DRAWN: &str = "drawn";

/// One of its controller's cards arriving face-down in its controller's backrow.
fn set_face_down(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::Summoned {
        player,
        instance_id,
        row: Row::Backrow,
        ..
    } = event
    else {
        return false;
    };
    *player == ctx.controller
        && find_instance(&*ctx.state, instance_id)
            .is_some_and(|card| is_face_down(&*ctx.state, card))
}

fn faces(cap: i64) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            face_down_indestructible: Some(true),
            ..StaticFlags::default()
        }),
        triggers: vec![TriggerDef::new(
            "thousand-faces",
            &[GameEventType::Summoned],
            move |ctx, event| {
                if !set_face_down(ctx, event) {
                    return vec![];
                }
                let turn = i64::from(ctx.state.turn);
                let used = match recalled(&*ctx, DRAWN) {
                    Some(done)
                        if done.get("turn").and_then(Value::as_i64) == Some(turn) =>
                    {
                        done.get("count").and_then(Value::as_i64).unwrap_or(0)
                    }
                    _ => 0,
                };
                if used >= cap {
                    return vec![];
                }
                vec![
                    remember(json_as(json!({
                        "key": DRAWN,
                        "value": { "turn": turn, "count": used + 1 },
                    }))),
                    draw(json_as(json!({ "count": 1 }))),
                ]
            },
        )
        .with_when(|ctx, event| set_face_down(ctx, event))],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: faces(1), radiant: faces(2) }
}

// IA #218 Thousand Faces — SPEC §8.13 row 218 (R4594001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const TRAP: &str = "stand-in (1) Trap"; // A textless Trap, so no reveal fires mid-test.
    const JAMMED: &str = "core-036"; // Magic Jammed: destroy a backrow card, lock its zone.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn set_id(s: &Scenario) -> String {
        s.state().players[P1]
            .backrow
            .iter()
            .filter_map(|slot| slot.clone())
            .find(|card| card.def_id == TRAP)
            .expect("a set Trap")
            .id
            .clone()
    }

    fn standing(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": true, "lane": 1 }],
                "hand": [TRAP, TRAP, TRAP, STOCKPILE],
                "library": crate::filler(6),
            },
            "p2": { "hand": [JAMMED, TRAP, STOCKPILE], "library": crate::filler(4) },
        }))
    }

    fn set_traps(s: &mut Scenario, count: usize) {
        for zone in 2..2 + count as i32 {
            s.play(TRAP, json!({ "zone": zone }));
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r4594001_a_set_trap_survives_a_destroy() {
            let mut s = standing(false, "faces-guard");
            s.play(TRAP, json!({ "zone": 2 }));
            let id = set_id(&s);
            s.end_turn();
            // Magic Jammed destroys the set Trap — and fails.
            s.play(JAMMED, json!({ "targets": pick::id(&id) }));
            assert_eq!(set_id(&s), id, "the set Trap stands");
        }

        #[test]
        fn r4594001_a_face_up_card_dies() {
            let mut s = standing(false, "faces-faceup");
            let faces = s.state().players[P1]
                .backrow
                .iter()
                .filter_map(|slot| slot.clone())
                .find(|card| card.def_id == ID)
                .expect("Thousand Faces")
                .id
                .clone();
            s.end_turn();
            // Magic Jammed destroys Thousand Faces itself, face-up.
            s.play(JAMMED, json!({ "targets": pick::id(&faces) }));
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn r4594001_two_sets_draw_one() {
            let mut s = standing(false, "faces-cap");
            let hand = s.hand(P1).len();
            set_traps(&mut s, 2);
            assert_eq!(s.hand(P1).len(), hand - 2 + 1, "one draw for two sets");
        }

        #[test]
        fn r4594001_next_turn_draws_again() {
            let mut s = standing(false, "faces-again");
            set_traps(&mut s, 1);
            s.end_turn();
            s.end_turn();
            let hand = s.hand(P1).len();
            s.play(TRAP, json!({ "zone": 4 }));
            assert_eq!(s.hand(P1).len(), hand, "the new turn draws again");
        }

        #[test]
        fn r4594001_an_enemy_set_draws_nothing() {
            let mut s = standing(false, "faces-enemy");
            s.end_turn();
            let hand = s.hand(P1).len();
            s.play(TRAP, json!({ "zone": 2 }));
            assert_eq!(s.hand(P1).len(), hand, "no draw on the enemy's set");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4594001_three_sets_draw_two() {
            let mut s = standing(true, "faces-radiant-cap");
            let hand = s.hand(P1).len();
            set_traps(&mut s, 3);
            assert_eq!(s.hand(P1).len(), hand - 3 + 2, "two draws for three sets");
        }
    }
}
