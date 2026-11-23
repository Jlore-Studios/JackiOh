//! M #83 Medina Enforcer (SPEC §8.8 row 83): (3) Unit, Rare, 4/7 → 8/14 ((2) 4/4 → 8/8 until issue #4353).
//!
//! Base:    "Activate: Buff every card in your hand."
//! Radiant: "Activate: Buff every card in your hand {times|time|times}."
//! Engine: M #82's effect as an Activate, once a turn on its controller's own turn (R384, §6.2), with no
//! cost, target or mode; summoning sickness does not stop it. The count is the declared `times` (R386),
//! 1, and 3 on the Radiant face. The designer's Radiant face kept 4/4 until issue #4353; R275's stat half
//! made it 8/8 (⚠ designer). Through v0.6.8b it Buffed the hand at its controller's end of turn (R62),
//! which `script_v0_6_8b` keeps (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-083";

/// The Activate's ability id.
const ABILITY: &str = "enforce";

/// M #82's effect: Buff every card in its controller's hand `times` times.
fn buff_the_hand(ctx: &EffectContext<'_>) -> Vec<Effect> {
    vec![upgrade(json_as(json!({
        "scope": { "side": "self", "zones": ["hand"] },
        "times": param(ctx, "times"),
    })))]
}

/// "Activate: Buff every card in your hand", once a turn (R384), since issue #4353.
fn enforce() -> ActivationDecl {
    ActivationDecl {
        id: ABILITY.into(),
        label: "Buff every card in your hand".into(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| buff_the_hand(ctx)),
    }
}

pub fn script() -> CardScripts {
    // The Radiant face Buffs each card three times; the count reads through `param()`.
    let base = Script {
        activations: vec![enforce()],
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// R767000: through v0.6.8b it Buffed the hand at its controller's end of turn (R62), with no Activate
/// (issue #4353); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_8b() -> CardScripts {
    let base = Script {
        end_of_turn: Some(hook(|ctx| buff_the_hand(ctx))),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M #83 Medina Enforcer — SPEC §8.8 row 83, BUILD M10 row M 83 (R384, R386, R440).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::filler;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const ENFORCER: &str = "meditative-083";
    const VANILLA: &str = "core-008";
    const STOCKPILE: &str = "core-005";
    const FILLER: &str = "core-005";

    fn upgraded_for(events: &[GameEvent], id: &str) -> Vec<GameEvent> {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Upgraded { instance_id, .. } if instance_id == id))
            .cloned()
            .collect()
    }

    fn upgraded_count(events: &[GameEvent]) -> usize {
        events.iter().filter(|event| matches!(event, GameEvent::Upgraded { .. })).count()
    }

    /// The Enforcer on p1's field, `radiant` or not, over a hand of a Vanilla and a Stockpile: the
    /// scenario and the two hand cards' ids.
    fn enforcing(radiant: bool, seed: &str) -> (Scenario, String, String) {
        let s = scenario(json!({
            "seed": seed,
            "p1": {
                "mana": 10,
                "hand": [VANILLA, STOCKPILE],
                "field": [{ "def": ENFORCER, "radiant": radiant, "lane": 1 }],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        let vanilla = s.hand(P1).iter().find(|card| card.def_id == VANILLA).expect("vanilla").id.clone();
        let stockpile = s.hand(P1).iter().find(|card| card.def_id == STOCKPILE).expect("stockpile").id.clone();
        (s, vanilla, stockpile)
    }

    /// p1 uses the Enforcer's Activate.
    fn activate(s: &mut Scenario) -> &mut Scenario {
        s.activate(ENFORCER, json!({ "ability": ABILITY }))
    }

    mod m83_medina_enforcer {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r384_its_activate_gives_each_hand_card_one_buff() {
                let (mut s, vanilla, stockpile) = enforcing(false, "enforcer");
                activate(&mut s);
                assert_eq!(upgraded_for(s.last_events(), &vanilla).len(), 1);
                assert_eq!(upgraded_for(s.last_events(), &stockpile).len(), 1);
            }

            #[test]
            fn r384_once_a_turn_and_back_on_the_next() {
                let (mut s, _, _) = enforcing(false, "enforcer-once");
                activate(&mut s);
                s.expect_refused(activate);
                assert_eq!(upgraded_count(s.events()), 2);
                crate::next_turn_of(&mut s, P1);
                let held = s.hand(P1).len();
                activate(&mut s);
                assert_eq!(upgraded_count(s.last_events()), held, "one Buff for each card held now");
            }

            #[test]
            fn r384_summoning_sickness_does_not_stop_it() {
                let mut s = scenario(json!({
                    "seed": "enforcer-played",
                    "p1": { "mana": 10, "hand": [ENFORCER, VANILLA, STOCKPILE], "library": filler(4) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(ENFORCER, json!({ "zone": 1 }));
                activate(&mut s);
                assert_eq!(upgraded_count(s.last_events()), 2);
            }

            #[test]
            fn issue_4353_nothing_at_either_end_of_turn() {
                let (mut s, _, _) = enforcing(false, "enforcer-ends");
                s.end_turn(); // p1's end: no Buff since issue #4353.
                s.end_turn(); // p2's end: none either.
                assert_eq!(upgraded_count(s.events()), 0);
            }

            /// R767000: through v0.6.8b it Buffed the hand at its controller's end of turn, never the
            /// opponent's, and had no Activate (issue #4353).
            #[test]
            fn r767000_the_script_kept_for_v0_6_8b_buffs_at_its_controllers_end_of_turn() {
                crate::under_kept(ID, script_v0_6_8b(), || {
                    let (mut s, vanilla, stockpile) = enforcing(false, "enforcer-kept");
                    s.expect_refused(activate);
                    s.end_turn(); // p1's end: two Buffs.
                    assert_eq!(upgraded_for(s.events(), &vanilla).len(), 1);
                    assert_eq!(upgraded_for(s.events(), &stockpile).len(), 1);
                    s.end_turn(); // p2's end: nothing more.
                    assert_eq!(upgraded_count(s.events()), 2);
                });
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r384_radiant_three_buffs_each() {
                let (mut s, vanilla, stockpile) = enforcing(true, "enforcer-radiant");
                activate(&mut s);
                assert_eq!(upgraded_for(s.last_events(), &vanilla).len(), 3);
                assert_eq!(upgraded_for(s.last_events(), &stockpile).len(), 3);
            }

            /// R386: the count is the declared `times`, the Radiant face's alone (`tunedOn`).
            #[test]
            fn r386_the_count_reads_through_param() {
                let (mut s, vanilla, _) = enforcing(true, "enforcer-tuned");
                let times = crate::upgrade_number(&mut s, ENFORCER, "times");
                activate(&mut s);
                assert_eq!(upgraded_for(s.last_events(), &vanilla).len(), times as usize);
            }
        }
    }
}
