//! SPEC §8.12 S+ #40 Mender (R3556002; R62, R1420, R3543001):
//! (1) Field Spell, Mindustrial, Common, a card of the Stinkyscape+ set.
//!   Base:    "Flexible
//!            End of turn: Heal adjacent units to full."
//!   Radiant: "Flexible
//!            End of turn: Heal adjacent units to full & give them Divine Shield."
//!
//! Flexible with no stats keeps to its backrow (R3543001). At the end of its controller's turn (R62)
//! it heals the adjacent Units — its controller's Units standing in its lane or a lane beside it
//! (R3556002) — to full (`heal` with `toFull`); the Radiant face then grants each of them Divine
//! Shield (as S+ #13 Shield Repairer's).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-040";

/// R3556002: how far beside its lane the Mender reaches.
const REACH: i32 = 1;

/// The controller's Units adjacent to `self_`: standing in its lane or a lane beside it.
fn adjacent(ctx: &EffectContext<'_>, self_: &CardInstance) -> Vec<String> {
    let lanes = lanes_of(self_);
    active_units_of(ctx.state, ctx.controller)
        .into_iter()
        .filter(|unit| {
            lanes_of(unit)
                .iter()
                .any(|lane| lanes.iter().any(|mine| (lane - mine).abs() <= REACH))
        })
        .map(|unit| unit.id.clone())
        .collect()
}

fn mender(radiant: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            let Some(self_) = ctx.self_.clone() else {
                return vec![];
            };
            let mut effects = Vec::new();
            for id in adjacent(ctx, &self_) {
                effects.push(heal(json_as(
                    json!({ "target": { "of": "instance", "instanceId": id }, "toFull": true }),
                )));
                if radiant {
                    effects.push(grant_keyword(json_as(json!({
                        "target": { "of": "instance", "instanceId": id },
                        "keyword": { "kind": "Divine Shield" },
                    }))));
                }
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: mender(false),
        radiant: mender(true),
    }
}

// S+ #40 Mender — SPEC §8.12 row S+ 40 (R3556002; R62, R3543001).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, next_turn_of, run_as, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A durable Unit to wound and heal.
    const BODY: &str = "stand-in (1) 4/4 R12/12";

    /// p1's Mender in backrow lane 2 with Units around it; `"mana": 10` plays it from hand.
    fn mended(radiant: bool, seed: &str) -> Scenario {
        let mut s = weapon_board(
            seed,
            json!({
                "mana": 10,
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [BODY, BODY, BODY, BODY],
                "library": filler(6),
            }),
            json!({
                "field": [BODY],
                "library": filler(6),
            }),
        );
        s.play(ID, json!({ "zone": 2 }));
        s
    }

    /// Wound every Unit on the board by 2, as p1 and outside any turn boundary.
    fn wound_all(s: &mut Scenario) {
        for player in [P1, P2] {
            for lane in 1..=4 {
                if let Some(unit) = s.unit(player, lane) {
                    let id = unit.id.clone();
                    run_as(
                        s,
                        jackioh_engine::effects::damage(json_as(json!({
                            "to": { "of": "instance", "instanceId": id },
                            "amount": 2,
                        }))),
                        P1,
                    );
                }
            }
        }
    }

    fn damaged(s: &Scenario, player: PlayerId, lane: i32) -> i32 {
        let id = s.unit(player, lane).expect("a Unit there").id;
        let view = s.stats(id.as_str());
        view.max_health - view.health
    }

    fn shielded(s: &Scenario, id: &str) -> bool {
        s.stats(id).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::DivineShield)
    }

    mod base {
        use super::*;

        #[test]
        fn r3556002_end_of_turn_heals_its_controllers_units_in_its_lane_and_beside_it() {
            let mut s = mended(false, "s40-heal");
            wound_all(&mut s);
            // Lane 2's Mender: its controller's lanes 1-3 heal, lane 4 and the enemy across do not.
            s.end_turn();
            for lane in 1..=3 {
                assert_eq!(damaged(&s, P1, lane), 0, "p1's lane {lane} healed to full");
            }
            assert_eq!(damaged(&s, P1, 4), 2, "p1's lane 4 is no adjacent Unit");
            assert_eq!(damaged(&s, P2, 1), 2, "the enemy across is no adjacent Unit");
        }

        #[test]
        fn r3556002_not_on_the_opponents_end_of_turn() {
            let mut s = mended(false, "s40-turns");
            // Past its controller's end of turn, so its heal already fired; wound afresh.
            next_turn_of(&mut s, P2);
            wound_all(&mut s);
            s.end_turn();
            assert_eq!(damaged(&s, P1, 1), 2, "the opponent's end of turn heals nothing");
            assert_eq!(damaged(&s, P1, 2), 2, "the opponent's end of turn heals nothing");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3556002_radiant_also_gives_them_divine_shield() {
            let mut s = mended(true, "s40-radiant");
            wound_all(&mut s);
            s.end_turn();
            for lane in 1..=3 {
                assert_eq!(damaged(&s, P1, lane), 0, "p1's lane {lane} healed to full");
                let id = s.unit(P1, lane).expect("a Unit there").id;
                assert!(shielded(&s, id.as_str()), "p1's lane {lane} has Divine Shield");
            }
            let far = s.unit(P1, 4).expect("a Unit there").id;
            assert!(!shielded(&s, far.as_str()), "p1's lane 4 has no Divine Shield");
        }
    }
}
