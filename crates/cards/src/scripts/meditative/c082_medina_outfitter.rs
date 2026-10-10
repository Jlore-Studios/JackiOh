//! M #82 Medina Outfitter (SPEC §8.8 row 82): (1) Unit, Common, 1/1 → 2/2.
//!
//! Base:    "Cry: Buff every card in your hand."
//! Radiant: "Cry: Buff every card in your hand {times|time|times}."
//! Engine: `upgrade({ scope: { side: "self", zones: ["hand"] }, times: param("times") })`: each card
//! in the hand takes `times` separate Buffs (R386), each one draw (R442); an Immutable card is left
//! alone; a hand is a pile the opponent cannot read, so every application is cued and none counted
//! (R440).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-082";

fn outfitter() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![upgrade(json_as(json!({
                "scope": { "side": "self", "zones": ["hand"] },
                "times": param(&*ctx, "times"),
            })))]
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

// M #82 Medina Outfitter — SPEC §8.8 row 82, BUILD M10 row M 82: "Cry (played or cast): one Buff on
// each card in your hand, an Immutable one unchanged, each `upgraded` cued to the opponent with no
// card named (R440); an empty hand, nothing; radiant 2/2 and three Buffs each, times read through
// `param()`".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const OUTFITTER: &str = "meditative-082";
    const VANILLA: &str = "core-008"; // (1) Human Unit, no text.
    const STOCKPILE: &str = "core-005"; // (1) Spell: draw 2.
    const MENACE: &str = "core-019"; // Radiant: Immutable.
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
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

        #[test]
        fn is_a_1_1_1_common_unit_radiant_2_2_with_times_1_to_3_on_the_radiant_face() {
            let def = crate::card_def(ID);
            assert_eq!(def.id, OUTFITTER);
            assert_eq!(def.cost, CardCost::Fixed(1));
            assert_eq!(def.type_, CardType::Unit);
            assert_eq!(def.rarity, Rarity::Common);
            assert!(def.tags.is_empty());
            assert_eq!(
                [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
                [Some(1), Some(1), Some(2), Some(2)]
            );
            let params = def.params.expect("times");
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].key, "times");
            assert_eq!((params[0].base, params[0].radiant), (1, 3));
        }

        mod base {
            use super::*;

            #[test]
            fn r386_r440_its_cry_buffs_each_card_in_your_hand_once_an_immutable_one_cued_none() {
                crate::register_all();
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
                crate::register_all();
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
                crate::register_all();
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
            crate::register_all();
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
}
