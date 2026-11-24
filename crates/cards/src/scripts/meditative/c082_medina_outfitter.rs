//! M #82 Medina Outfitter (SPEC §8.8 row 82): (1) Unit, Common, 1/1 → 2/2.
//!
//! Base:    "Cry: Buff every card in your hand. You have {mana} less mana next turn."
//! Radiant: "Cry: Buff every card in your hand {times|time|times}. You have {mana} less mana next turn."
//! Engine: `upgrade({ scope: { side: "self", zones: ["hand"] }, times: param("times") })`: each card
//! in the hand takes `times` separate Buffs (R386), each one draw (R442); an Immutable card is left
//! alone; a hand is a pile the opponent cannot read, so every application is cued and none counted
//! (R440). Issue #4665 gave both faces' Cry "You have {mana} less mana next turn" (R4665005), as
//! Core #2 Speciesist's (R4353021): `next_turn_mana` of the declared `mana` (1 on both faces, R386),
//! read with `param_or` at 0, so a replay of a version that declared no `mana` loses none (R2657006)
//! and needs no kept script.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-082";

/// R4665005: the Cry's "You have {mana} less mana next turn"; 0 under a definition that declares no
/// `mana`, as every one through v0.6.12c (R2657006).
const NO_LOSS: i32 = 0;

fn outfitter() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let mut effects = vec![upgrade(json_as(json!({
                "scope": { "side": "self", "zones": ["hand"] },
                "times": param(&*ctx, "times"),
            })))];
            let loss = param_or(&*ctx, "mana", NO_LOSS);
            if loss != 0 {
                effects.push(next_turn_mana(json_as(json!({ "amount": -loss }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The Radiant face Buffs each card three times; the count reads through `param()`.
    let base = outfitter();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M #82 Medina Outfitter — SPEC §8.8 row 82, BUILD M10 row M 82.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const OUTFITTER: &str = "meditative-082";
    const VANILLA: &str = "core-008"; // Mr. Vanilla: Human Unit, no text
    const STOCKPILE: &str = "core-005"; // Stockpile: draw 2
    const MENACE: &str = "core-019"; // Midrange Menace: Radiant: Immutable
    /// (3) Field Spell, none of the hand cards above: p2's own hand shows in p2's view, so a filler
    /// that were Stockpile would put its id there and hide the leak the view check looks for.
    const FILLER: &str = "core-006";

    fn filler(count: usize) -> Value {
        json!(vec![FILLER; count])
    }

    fn upgraded_for(events: &[GameEvent], id: &str) -> Vec<GameEvent> {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Upgraded { instance_id, .. } if instance_id == id))
            .cloned()
            .collect()
    }

    fn change_of(event: &GameEvent) -> TuningChange {
        match event {
            GameEvent::Upgraded { change, .. } => change.clone(),
            other => panic!("not an upgrade: {other:?}"),
        }
    }

    mod m82_medina_outfitter {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r386_r440_its_cry_buffs_each_card_in_your_hand_once_an_immutable_one_cued_none() {
                let mut s = scenario(json!({
                    "seed": "outfitter",
                    "p1": {
                        "mana": 10,
                        "hand": [OUTFITTER, VANILLA, STOCKPILE, { "def": MENACE, "radiant": true }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                let vanilla = s.hand(P1).iter().find(|card| card.def_id == VANILLA).expect("vanilla").id.clone();
                let stockpile =
                    s.hand(P1).iter().find(|card| card.def_id == STOCKPILE).expect("stockpile").id.clone();
                let menace =
                    s.hand(P1).iter().find(|card| card.def_id == MENACE).expect("menace").id.clone();
                s.play(OUTFITTER, json!({}));
                // One Buff each: three `upgraded` events.
                assert_eq!(upgraded_for(s.events(), &vanilla).len(), 1);
                assert_eq!(upgraded_for(s.events(), &stockpile).len(), 1);
                let menace_events = upgraded_for(s.events(), &menace);
                assert_eq!(menace_events.len(), 1);
                // The Immutable one is unchanged, cued `none`.
                assert_eq!(change_of(&menace_events[0]), TuningChange::None);
                assert_ne!(change_of(&upgraded_for(s.events(), &vanilla)[0]), TuningChange::None);
                // P2's view names none of the hand cards.
                let view = serde_json::to_string(&s.view(P2)).expect("a view is JSON");
                assert!(!view.contains(VANILLA));
                assert!(!view.contains(STOCKPILE));
                assert!(!view.contains(MENACE));
            }

            #[test]
            fn an_empty_hand_nothing() {
                let mut s = scenario(json!({
                    "seed": "outfitter-empty",
                    "p1": { "mana": 10, "hand": [OUTFITTER], "library": filler(4) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(OUTFITTER, json!({}));
                assert!(
                    !s.events().iter().any(|event| matches!(event, GameEvent::Upgraded { .. })),
                    "no card, no application"
                );
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn radiant_three_buffs_each() {
                let mut s = scenario(json!({
                    "seed": "outfitter-radiant",
                    "p1": {
                        "mana": 10,
                        "hand": [{ "def": OUTFITTER, "radiant": true }, VANILLA, STOCKPILE],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                let vanilla = s.hand(P1).iter().find(|card| card.def_id == VANILLA).expect("vanilla").id.clone();
                let stockpile =
                    s.hand(P1).iter().find(|card| card.def_id == STOCKPILE).expect("stockpile").id.clone();
                s.play(OUTFITTER, json!({}));
                assert_eq!(upgraded_for(s.events(), &vanilla).len(), 3);
                assert_eq!(upgraded_for(s.events(), &stockpile).len(), 3);
            }
        }

        #[test]
        fn r749_times_is_tuned_on_the_radiant_face_only() {
            let base = scenario(json!({
                "seed": "outfitter-tuned",
                "p1": { "hand": [OUTFITTER], "library": filler(4) },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            assert!(!crate::can_upgrade_number(&base, OUTFITTER, "times"));
            let mut radiant = scenario(json!({
                "seed": "outfitter-tuned-radiant",
                "p1": { "mana": 10, "hand": [{ "def": OUTFITTER, "radiant": true }, VANILLA], "library": filler(4) },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            assert!(crate::can_upgrade_number(&radiant, OUTFITTER, "times"));
            // And it reads through `param()`: one Degrade makes it two Buffs each.
            crate::degrade_number(&mut radiant, OUTFITTER, "times");
            let vanilla =
                radiant.hand(P1).iter().find(|card| card.def_id == VANILLA).expect("vanilla").id.clone();
            radiant.play(OUTFITTER, json!({}));
            assert_eq!(upgraded_for(radiant.events(), &vanilla).len(), 2);
        }
    }

    /// R4665005: issue #4665's "You have {mana} less mana next turn", on both faces.
    mod m82_less_mana_next_turn_r4665005 {
        use super::*;

        /// The declared `mana`, 1 on both faces.
        const LOSS: i32 = 1;

        fn board(radiant: bool) -> Scenario {
            scenario(json!({
                "p1": {
                    "mana": 10,
                    "hand": [{ "def": OUTFITTER, "radiant": radiant }, VANILLA],
                    "library": filler(4),
                },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }))
        }

        #[test]
        fn r4665005_both_faces_cry_loses_1_mana_next_turn_and_the_turn_after_is_whole() {
            for radiant in [false, true] {
                let mut s = board(radiant);
                s.play(OUTFITTER, json!({}));
                assert_eq!(s.state().players.p1.mana.next_turn_mod, -LOSS, "radiant {radiant}");
                assert_eq!(s.state().players.p2.mana.next_turn_mod, 0, "radiant {radiant}: its own side");
                crate::next_turn_of(&mut s, P1);
                let side = &s.state().players.p1;
                assert_eq!(side.mana.current, side.mana.max - LOSS, "radiant {radiant}");
                crate::next_turn_of(&mut s, P1);
                let side = &s.state().players.p1;
                assert_eq!(side.mana.current, side.mana.max, "radiant {radiant}");
            }
        }

        /// R2657006: a definition that declares no `mana`, as v0.6.12c's, loses none.
        #[test]
        fn r4665005_r2657006_under_a_definition_with_no_mana_the_cry_loses_none() {
            let kept = crate::under_kept_as(
                OUTFITTER,
                script(),
                |def| {
                    if let Some(params) = def.params.as_mut() {
                        params.retain(|param| param.key != "mana");
                    }
                },
                || {
                    let mut s = board(false);
                    s.play(OUTFITTER, json!({}));
                    s.state().players.p1.mana.next_turn_mod
                },
            );
            assert_eq!(kept, 0);
        }
    }
}
