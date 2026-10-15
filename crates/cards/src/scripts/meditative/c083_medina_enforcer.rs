//! M #83 Medina Enforcer (SPEC §8.8 row 83): (2) Unit, Rare, 4/4 → 8/8.
//!
//! Base:    "End of turn: Buff every card in your hand."
//! Radiant: "End of turn: Buff every card in your hand {times|time|times}."
//! Engine: M #82's effect at its controller's end of turn (R62's end-of-turn triggers). The
//! designer's Radiant face kept 4/4; R275's stat half makes it 8/8 (⚠ designer).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-083";

fn enforcer() -> Script {
    Script {
        end_of_turn: Some(hook(|ctx| {
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
    let base = enforcer();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M #83 Medina Enforcer — SPEC §8.8 row 83, BUILD M10 row M 83: "At your end of turn only, one Buff
// on each card in your hand (R386), cued per R440; nothing at the opponent's end; radiant 8/8 and
// three Buffs each".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const ENFORCER: &str = "meditative-083";
    const VANILLA: &str = "core-008";
    const STOCKPILE: &str = "core-005";
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

    fn upgraded_count(events: &[GameEvent]) -> usize {
        events.iter().filter(|event| matches!(event, GameEvent::Upgraded { .. })).count()
    }

    mod m83_medina_enforcer {
        use super::*;

        #[test]
        fn is_a_2_4_4_rare_unit_radiant_8_8_with_times_1_to_3_on_the_radiant_face() {
            let def = crate::card_def(ID);
            assert_eq!(def.id, ENFORCER);
            assert_eq!(def.cost, CardCost::Fixed(2));
            assert_eq!(def.type_, CardType::Unit);
            assert_eq!(def.rarity, Rarity::Rare);
            assert!(def.tags.is_empty());
            assert_eq!(
                [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
                [Some(4), Some(4), Some(8), Some(8)]
            );
            let params = def.params.expect("times");
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].key, "times");
            assert_eq!((params[0].base, params[0].radiant), (1, 3));
        }

        mod base {
            use super::*;

            #[test]
            fn r62_at_your_end_of_turn_each_hand_card_gets_one_buff() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "enforcer",
                    "p1": {
                        "mana": 10,
                        "hand": [VANILLA, STOCKPILE],
                        "field": [{ "def": ENFORCER, "lane": 1 }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                let vanilla = s.hand(P1).iter().find(|card| card.def_id == VANILLA).expect("vanilla").id.clone();
                let stockpile =
                    s.hand(P1).iter().find(|card| card.def_id == STOCKPILE).expect("stockpile").id.clone();
                s.end_turn();
                assert_eq!(upgraded_for(s.events(), &vanilla).len(), 1);
                assert_eq!(upgraded_for(s.events(), &stockpile).len(), 1);
            }

            #[test]
            fn nothing_at_the_opponents_end_of_turn() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "enforcer-foe",
                    "p1": {
                        "mana": 10,
                        "hand": [VANILLA, STOCKPILE],
                        "field": [{ "def": ENFORCER, "lane": 1 }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.end_turn(); // p1's end: two Buffs.
                assert_eq!(upgraded_count(s.events()), 2);
                s.end_turn(); // p2's end: nothing more.
                assert_eq!(upgraded_count(s.events()), 2);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn radiant_three_buffs_each() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "enforcer-radiant",
                    "p1": {
                        "mana": 10,
                        "hand": [VANILLA, STOCKPILE],
                        "field": [{ "def": ENFORCER, "radiant": true, "lane": 1 }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                let vanilla = s.hand(P1).iter().find(|card| card.def_id == VANILLA).expect("vanilla").id.clone();
                let stockpile =
                    s.hand(P1).iter().find(|card| card.def_id == STOCKPILE).expect("stockpile").id.clone();
                s.end_turn();
                assert_eq!(upgraded_for(s.events(), &vanilla).len(), 3);
                assert_eq!(upgraded_for(s.events(), &stockpile).len(), 3);
            }
        }
    }
}
