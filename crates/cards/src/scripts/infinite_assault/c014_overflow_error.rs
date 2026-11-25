//! SPEC §8.13 IA #14 Overflow Error ((1) Trap, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reveals when your opponent plays their third card in a turn: Deal {damage} damage to their
//!             hero. They discard a random card." (damage 3)
//!   Radiant: "Reveals when your opponent plays their third card in a turn: Deal {damage} damage to their
//!             hero. They discard two random cards." (damage 6)
//!
//! C+ T-AI-8 Rate Limit's condition (R99, R456: it fires as their third play is played, before that
//! play resolves) and its preview (R2476000), with a hit on the enemy hero and a random discard from
//! their hand (§6.3) for its effect. The third card is already out of their hand, so it is never the
//! one discarded.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-014";

/// "their third card in a turn".
const NTH_PLAY: i32 = 3;

/// "a random card", Radiant "two random cards".
const DISCARDS: i32 = 1;
const RADIANT_DISCARDS: i32 = 2;

/// The trap's `TriggerDef`: its `when` is the condition (R99).
fn overflow(discards: i32) -> TriggerDef {
    TriggerDef::new("overflow-error", &[GameEventType::CardPlayed], move |ctx, _event| {
        vec![
            damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": param(&*ctx, "damage") }))),
            discard_random(json_as(json!({ "count": discards, "player": "enemy" }))),
        ]
    })
    .with_when(|ctx, event| match event {
        GameEvent::CardPlayed { player, .. } => {
            *player != ctx.controller
                && ctx.state.active == *player
                && cards_played_this_turn(&*ctx.state, *player) == NTH_PLAY
        }
        _ => false,
    })
}

/// R2476000: the opponent's plays this turn, after "card in a turn", as Rate Limit shows them: on their
/// turn, and 0 on its controller's. Face-down, so its controller alone reads it (R33).
fn preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        let played = if ctx.your_turn { 0 } else { cards_played_this_turn(ctx.state, opponent_of(ctx.controller)) };
        vec![count_preview("card in a turn", played)]
    })
}

fn face(discards: i32) -> Script {
    Script {
        triggers: vec![overflow(discards)],
        preview: Some(preview()),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(DISCARDS),
        radiant: face(RADIANT_DISCARDS),
    }
}

// IA #14 Overflow Error — SPEC §8.13 row 14 (R99, R456).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SPELL: &str = "stand-in (0) Spell";

    /// p2's turn, p1's Overflow Error face-down, p2 holding six textless Spells.
    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": false, "lane": 2 }],
                "library": crate::filler(2),
            },
            "p2": { "hand": [SPELL, SPELL, SPELL, SPELL, SPELL, SPELL], "library": crate::filler(2) },
        }))
    }

    fn play_spells(s: &mut Scenario, count: usize) {
        for _ in 0..count {
            s.play(SPELL, json!({}));
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r99_their_first_two_plays_leave_it_armed() {
            let mut s = board(false);
            play_spells(&mut s, 2);
            assert_eq!(s.card(ID).face_up, Some(false));
            s.expect_health(P2, HERO_HEALTH);
            assert_eq!(s.hand(P2).len(), 4);
        }

        #[test]
        fn r456_their_third_play_reveals_it_3_damage_and_one_random_discard() {
            let mut s = board(false);
            play_spells(&mut s, 3);
            s.expect_in_zone(ID, "graveyard");
            s.expect_health(P2, HERO_HEALTH - 3);
            assert_eq!(s.hand(P2).len(), 2, "six, less three plays and one discard");
            s.expect_health(P1, HERO_HEALTH);
        }

        #[test]
        fn your_own_plays_never_set_it_off() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [SPELL, SPELL, SPELL],
                    "backrow": [{ "def": ID, "faceUp": false, "lane": 2 }],
                    "library": crate::filler(2),
                },
                "p2": { "hand": [SPELL], "library": crate::filler(2) },
            }));
            play_spells(&mut s, 3);
            assert_eq!(s.card(ID).face_up, Some(false));
            s.expect_health(P2, HERO_HEALTH);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r456_their_third_play_reveals_it_6_damage_and_two_random_discards() {
            let mut s = board(true);
            play_spells(&mut s, 3);
            s.expect_in_zone(ID, "graveyard");
            s.expect_health(P2, HERO_HEALTH - 6);
            assert_eq!(s.hand(P2).len(), 1, "six, less three plays and two discards");
        }
    }
}
