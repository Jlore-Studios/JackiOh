//! SPEC §8.12 S+ #56 Impact Reactor (R3546000, R3556000, R4783020; R62, R1420):
//! (3) Field Spell, Mindustrial, Legendary, a card of the Stinkyscape+ set.
//!   Base:    "GIGA
//!            Cry: Overload ({mana}).
//!            Dormant for {dormant|turn|turns}.
//!            When this awakens: Gain infinite mana."
//!   Radiant: "GIGA
//!            Cry: Overload ({mana}).
//!            Dormant for {dormant|turn|turns}.
//!            When this awakens: Gain infinite mana."
//! (2; 1.)
//!
//! GIGA is catalog data (R3546000): the Field Spell stands in four zones. The Cry (run as it is
//! played) Overloads its controller by `mana` (§6.3 Overload, R1848004, R4783020): a `next_turn_mana`
//! of minus `mana` at their next refresh, and nothing the turn after. It enters Dormant for `dormant`
//! turns (R3556000), and while it acts awake its `infinite_mana` flag spends no mana (R3548004).
//!
//! Issue #4783 (Stinkyscape+ Balance Changes VIII) made the Cry one Overload of 2 (Radiant 1). Through
//! v0.6.13h (`script_v0_6_13h`, R767000) it queued two stacking losses (R3556006): minus `mana`, 4
//! (Radiant 2), at the next refresh, and a `delay` of minus `mana2`, 2 (Radiant 1), to the start of
//! its controller's next turn, queued unwatched, so it stood if the Reactor left.

use jackioh_engine::prelude::*;
use jackioh_engine::prompts::RESUME_HOOK;

pub const ID: &str = "stinkyscapeplus-056";

/// "Dormant for 2 turns".
const DORMANT_TURNS: i32 = 2;
/// The delayed second loss's resume step (through v0.6.13h).
const THE_TURN_AFTER: &str = "the-turn-after";

/// A number the v0.6.13h Cry carried into the delayed entry's data.
fn stored(ctx: &EffectContext<'_>, key: &str) -> Option<i32> {
    ctx.data
        .get(key)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}

/// The face: today's when `through_v0_6_13h` is false, else the one v0.6.13h and before ran.
fn impact_reactor(through_v0_6_13h: bool) -> Script {
    let mut script = Script {
        static_flags: Some(StaticFlags {
            slumber: Some(SlumberText {
                turns: Some(DORMANT_TURNS),
                wakes_by: None,
            }),
            infinite_mana: Some(true),
            ..StaticFlags::default()
        }),
        // R4783020: "Overload ({mana})", taken off its controller's next refresh.
        cry: Some(hook(|ctx| {
            vec![next_turn_mana(json_as(json!({ "amount": -param(&*ctx, "mana") })))]
        })),
        ..Script::default()
    };
    if !through_v0_6_13h {
        return script;
    }
    // R3556006 (through v0.6.13h): minus `mana` next refresh, then minus `mana2` the turn after.
    script.cry = Some(hook(|ctx| {
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
    }));
    script.resume = IndexMap::from([(
        THE_TURN_AFTER,
        hook(|ctx| {
            let Some(amount) = stored(ctx, "amount") else {
                return vec![];
            };
            vec![next_turn_mana(json_as(json!({ "amount": -amount })))]
        }),
    )]);
    script
}

pub fn script() -> CardScripts {
    CardScripts {
        base: impact_reactor(false),
        radiant: impact_reactor(false),
    }
}

/// R767000: what it ran through v0.6.13h, before issue #4783: the Cry's second loss, `mana2`, the
/// turn after (R3556006).
pub fn script_v0_6_13h() -> CardScripts {
    CardScripts {
        base: impact_reactor(true),
        radiant: impact_reactor(true),
    }
}

// S+ #56 Impact Reactor — SPEC §8.12 row S+ 56 (R3546000, R3556000, R4783020; R62).
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

    /// p1 holds the Reactor of `radiant`'s face; `"mana": 10` plays it (and the GIGA anchor) from hand.
    fn reactor_board(seed: &str, radiant: bool) -> Scenario {
        let mut s = weapon_board(
            seed,
            json!({
                "mana": 10,
                "hand": [{ "def": ID, "radiant": radiant }, FILLER, DEAR],
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

    /// What p1 lacks of a full refresh at their next turn and at the one after it.
    fn losses(s: &mut Scenario) -> (i32, i32) {
        next_turn_of(s, P2);
        let (current, max) = mana(s, P2);
        assert_eq!(current, max, "p2 untouched");
        next_turn_of(s, P1);
        let (current, max) = mana(s, P1);
        let next = max - current;
        next_turn_of(s, P2);
        next_turn_of(s, P1);
        let (current, max) = mana(s, P1);
        (next, max - current)
    }

    mod base {
        use super::*;

        #[test]
        fn r3546000_it_is_giga() {
            let s = reactor_board("s56-giga", false);
            let card = s.card(ID);
            assert!(is_giga(card), "the Reactor stands GIGA");
            assert_eq!(lanes_of(card).len(), 2, "in both lanes");
        }

        #[test]
        fn r4783020_the_cry_overloads_2_next_turn_and_nothing_the_turn_after() {
            let mut s = reactor_board("s56-cry", false);
            assert_eq!(losses(&mut s), (2, 0));
        }

        #[test]
        fn r4783020_the_overload_is_its_declared_number() {
            let s = reactor_board("s56-declared", false);
            let mana = param_value(s.state(), Some(s.card(ID)), "mana", ParamValueOptions::default());
            assert_eq!(mana, 2);
        }

        #[test]
        fn r3556000_it_enters_dormant_and_awakens_on_the_second_own_turn_with_infinite_mana() {
            let mut s = reactor_board("s56-wake", false);
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
        fn r4783020_radiant_overloads_1_next_turn_and_nothing_the_turn_after() {
            let mut s = reactor_board("s56-radiant", true);
            assert_eq!(losses(&mut s), (1, 0));
        }
    }

    /// R767000: the script kept for the replays of v0.6.13h and before (issue #4783), under its v0.6.13h
    /// numbers: `mana` 4 (Radiant 2) and `mana2` 2 (Radiant 1).
    mod kept_v0_6_13h {
        use super::*;

        fn v0_6_13h(def: &mut CardDef) {
            def.params = serde_json::from_value(json!([
                { "key": "mana", "base": 4, "radiant": 2, "better": "down", "step": 1, "min": 1, "read": "cry" },
                { "key": "mana2", "base": 2, "radiant": 1, "better": "down", "step": 1, "min": 1, "read": "cry" },
                { "key": "dormant", "base": 2, "radiant": 2, "better": "down", "step": 1, "min": 1 }
            ]))
            .expect("the v0.6.13h params");
        }

        #[test]
        fn r767000_r3556006_the_cry_cost_4_next_turn_and_2_the_turn_after() {
            crate::under_kept_as(ID, script_v0_6_13h(), v0_6_13h, || {
                let mut s = reactor_board("s56-kept", false);
                assert_eq!(losses(&mut s), (4, 2));
            });
        }

        #[test]
        fn r767000_r3556006_radiant_cost_2_then_1() {
            crate::under_kept_as(ID, script_v0_6_13h(), v0_6_13h, || {
                let mut s = reactor_board("s56-kept-radiant", true);
                assert_eq!(losses(&mut s), (2, 1));
            });
        }
    }
}
