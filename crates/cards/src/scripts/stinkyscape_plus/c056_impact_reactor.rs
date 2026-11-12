//! SPEC §8.12 S+ #56 Impact Reactor (R3546000, R3556000, R3556006; R62, R1420):
//! (3) Field Spell, Mindustrial, Legendary, a card of the Stinkyscape+ set.
//!   Base:    "GIGA
//!            Cry: Lose {mana} mana next turn, and {mana2} mana the turn after.
//!            Dormant for {dormant|turn|turns}.
//!            When awoken: Gain infinite mana."
//!   Radiant: "GIGA
//!            Cry: Lose {mana} mana next turn, and {mana2} mana the turn after.
//!            Dormant for {dormant|turn|turns}.
//!            When awoken: Gain infinite mana."
//! (4 and 2; 2 and 1.)
//!
//! GIGA is catalog data (R3546000): the Field Spell stands in four zones. The Cry (run as it is
//! played) queues two stacking next-turn losses (R3556006): `next_turn_mana` of minus `mana` for the
//! next refresh, and a `delay` of minus `mana2` to the start of the controller's next turn — queued
//! unwatched, so it stands if the Reactor leaves. It enters Dormant for `dormant` turns (R3556000),
//! and while it acts awake its `infinite_mana` flag spends no mana (R3548004).

use jackioh_engine::prelude::*;
use jackioh_engine::prompts::RESUME_HOOK;

pub const ID: &str = "stinkyscapeplus-056";

/// "Dormant for 2 turns".
const DORMANT_TURNS: i32 = 2;
/// The delayed second loss's resume step.
const THE_TURN_AFTER: &str = "the-turn-after";

/// A number the Cry carried into the delayed entry's data.
fn stored(ctx: &EffectContext<'_>, key: &str) -> Option<i32> {
    ctx.data
        .get(key)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}

fn impact_reactor() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            slumber: Some(SlumberText {
                turns: Some(DORMANT_TURNS),
                wakes_by: None,
            }),
            infinite_mana: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| {
            let mana2 = param(&*ctx, "mana2");
            let name = def_of(Some(&*ctx.state), ID).name.clone();
            vec![
                next_turn_mana(json_as(json!({ "amount": -param(&*ctx, "mana") }))),
                delay(json_as(json!({
                    "at": { "phase": "start", "player": "self" },
                    "turns": 1,
                    "step": THE_TURN_AFTER,
                    "hook": RESUME_HOOK,
                    "data": { "amount": mana2 },
                    "label": format!("{name}: lose {mana2} mana next turn"),
                }))),
            ]
        })),
        resume: IndexMap::from([(
            THE_TURN_AFTER,
            hook(|ctx| {
                let Some(amount) = stored(ctx, "amount") else {
                    return vec![];
                };
                vec![next_turn_mana(json_as(json!({ "amount": -amount })))]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: impact_reactor(),
        radiant: impact_reactor(),
    }
}

// S+ #56 Impact Reactor — SPEC §8.12 row S+ 56 (R3546000, R3556000, R3556006; R62).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, next_turn_of, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005";
    /// A dearer card, playable only past the early mana.
    const DEAR: &str = "core-025";

    /// p1 holds the Reactor; `"mana": 10` plays it (and the GIGA anchor) from hand.
    fn reactor_board(seed: &str) -> Scenario {
        let mut s = weapon_board(
            seed,
            json!({
                "mana": 10,
                "hand": [ID, FILLER, DEAR],
                "library": filler(6),
            }),
            json!({
                "hand": [FILLER],
                "library": filler(6),
            }),
        );
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn mana(s: &Scenario, player: PlayerId) -> (i32, i32) {
        let mana = &s.state().players[player].mana;
        (mana.current, mana.max)
    }

    mod base {
        use super::*;

        #[test]
        fn r3546000_it_is_giga() {
            let s = reactor_board("s56-giga");
            let card = s.card(ID);
            assert!(is_giga(card), "the Reactor stands GIGA");
            assert_eq!(lanes_of(card).len(), 2, "in both lanes");
        }

        #[test]
        fn r3556006_the_cry_costs_4_next_turn_and_2_the_turn_after() {
            let mut s = reactor_board("s56-cry");
            // Next refresh loses 4.
            next_turn_of(&mut s, P2);
            let (current, max) = mana(&s, P2);
            assert_eq!((current, max), (max, max), "p2 untouched");
            next_turn_of(&mut s, P1);
            let (current, max) = mana(&s, P1);
            assert_eq!(current, max - 4, "4 lost next turn");
            // The turn after loses 2 more.
            next_turn_of(&mut s, P2);
            next_turn_of(&mut s, P1);
            let (current, max) = mana(&s, P1);
            assert_eq!(current, max - 2, "2 lost the turn after");
        }

        #[test]
        fn r3556000_it_enters_dormant_and_awakens_on_the_second_own_turn_with_infinite_mana() {
            let mut s = reactor_board("s56-wake");
            let id = s.card(ID).id.clone();
            assert_eq!(slumber_turns_of(s.card(id.as_str())), Some(2));
            assert_eq!(s.view(P1).you.infinite_mana, None, "no infinite mana while Dormant");
            // A dearer card is not playable yet.
            next_turn_of(&mut s, P1);
            assert_eq!(slumber_turns_of(s.card(id.as_str())), Some(1), "counting down");
            assert_eq!(s.view(P1).you.infinite_mana, None);
            next_turn_of(&mut s, P1);
            assert_eq!(slumber_turns_of(s.card(id.as_str())), None, "awake on the second own turn");
            assert_eq!(s.view(P1).you.infinite_mana, Some(true), "infinite mana once awake");
            // A dearer card plays into a free lane though the tray is empty: mana is not spent.
            let before = mana(&s, P1);
            s.play(DEAR, json!({ "zone": 3 }));
            assert_eq!(mana(&s, P1), before, "nothing spent");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3556006_radiant_costs_2_then_1() {
            let mut s = weapon_board(
                "s56-radiant",
                json!({
                    "mana": 10,
                    "hand": [{ "def": ID, "radiant": true }, FILLER],
                    "library": filler(6),
                }),
                json!({
                    "hand": [FILLER],
                    "library": filler(6),
                }),
            );
            s.play(ID, json!({ "zone": 1 }));
            next_turn_of(&mut s, P1);
            let (current, max) = mana(&s, P1);
            assert_eq!(current, max - 2, "2 lost next turn");
            next_turn_of(&mut s, P2);
            next_turn_of(&mut s, P1);
            let (current, max) = mana(&s, P1);
            assert_eq!(current, max - 1, "1 lost the turn after");
        }
    }
}
