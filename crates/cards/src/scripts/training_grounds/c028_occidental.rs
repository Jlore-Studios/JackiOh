//! SPEC §8.11 Training Grounds 28 Occidental: (3) Unit, Human, Epic, 9/5 → 18/10, a card of the held
//! Training Grounds set (R1420, R2194000).
//!
//! Base:    "Cry: Destroy all permanents in the rightmost lane."
//! Radiant: "Cry: Destroy all enemy permanents in the rightmost lane."
//! Engine: C+ #24 Crushing Walls' sweep on one lane, each zone's top card (§3: lanes run left to right
//! from each owner's seat, so the rightmost is the board's last,
//! `UNIT_ZONES`), both sides and both rows (Radiant: the enemy's alone), passing a Unit standing on an
//! Ivory Tower as its play resolves (R446, R653). The base face destroys itself there too, as
//! Hearthstone's "destroy all minions" (R2196003). TG #29 Oriental mirrors it on lane 1.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-028";

/// The rightmost lane, the board's last (§3.1, R2196003).
const RIGHTMOST: i32 = UNIT_ZONES;

fn sweep(side: ScopeSide) -> Script {
    let in_the_lane = move |ctx: &mut EffectContext<'_>| -> Vec<String> {
        cards_in_scope(ctx, &json_as(json!({ "side": side, "rows": ["units", "backrow"] })))
            .into_iter()
            .filter(|card| !is_carried(&*ctx.state, card))
            .filter(|card| slot_of(&*ctx.state, card).is_some_and(|at| at.lane == RIGHTMOST))
            .map(|card| card.id.clone())
            .collect()
    };
    Script {
        cry: Some(hook(move |_ctx| {
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(in_the_lane),
                each: Arc::new(|instance_id: &str| {
                    destroy(json_as(json!({ "target": { "of": "instance", "instanceId": instance_id } })))
                }),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: sweep(ScopeSide::Any),
        radiant: sweep(ScopeSide::Enemy),
    }
}

// TG #28 Occidental — SPEC §8.11 row 28, BUILD M13 row TG 28.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const OCCIDENTAL: &str = super::ID;
    const LANE: i32 = super::RIGHTMOST;
    const BODY: &str = "core-008"; // Mr. Vanilla 4/4
    const ROCK: &str = "core-066"; // Indestructible
    const FIENDER: &str = "core-092"; // Stack
    const FIELD_SPELL: &str = "core-064";
    const TRAP: &str = "core-060";
    const FLAGBEARER: &str = "classicplus-046"; // Death: shuffle a Felinor Flagbearer Prime into your deck
    const PRIME: &str = "classicplus-046-1";
    const FILLER: &str = "core-005";

    /// Both sides hold Mr. Vanillas and backrow cards in lanes 2 and 5, p1's lane-5 one a face-down Trap.
    fn swept(radiant: bool, zone: i32) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": OCCIDENTAL, "radiant": radiant }, FILLER],
                "field": [{ "def": BODY, "lane": 2 }, { "def": BODY, "lane": LANE }],
                "backrow": [{ "def": FIELD_SPELL, "lane": 2 }, { "def": TRAP, "lane": LANE, "faceUp": false }],
            },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": BODY, "lane": 2 }, { "def": BODY, "lane": LANE }],
                "backrow": [{ "def": FIELD_SPELL, "lane": 2 }, { "def": FIELD_SPELL, "lane": LANE }],
            },
        }));
        s.play(OCCIDENTAL, json!({ "zone": zone }));
        s
    }

    fn holds(s: &Scenario, player: PlayerId, lane: i32) -> (bool, bool) {
        (s.unit(player, lane).is_some(), s.backrow(player, lane).is_some())
    }

    mod base {
        use super::*;

        #[test]
        fn r2196003_both_sides_lane_5_units_and_backrow_cards_are_destroyed_face_down_included() {
            let s = swept(false, 1);
            assert_eq!(holds(&s, P1, LANE), (false, false));
            assert_eq!(holds(&s, P2, LANE), (false, false));
        }

        #[test]
        fn r2196003_lanes_1_to_4_are_untouched() {
            let s = swept(false, 1);
            assert!(s.unit(P1, 1).is_some_and(|card| card.def_id == OCCIDENTAL));
            assert_eq!(holds(&s, P1, 2), (true, true));
            assert_eq!(holds(&s, P2, 2), (true, true));
        }

        #[test]
        fn s6_1_an_indestructible_one_stays() {
            let mut s = scenario(json!({
                "p1": { "hand": [OCCIDENTAL, FILLER] },
                "p2": { "hand": [FILLER], "field": [{ "def": ROCK, "lane": LANE }] },
            }));
            s.play(OCCIDENTAL, json!({ "zone": 1 }));
            assert!(s.unit(P2, LANE).is_some_and(|card| card.def_id == ROCK));
        }

        #[test]
        fn r13_a_card_beneath_a_stack_in_lane_5_resumes() {
            let mut s = scenario(json!({
                "p1": { "hand": [OCCIDENTAL, FILLER] },
                "p2": { "hand": [FILLER], "field": [{ "def": BODY, "lane": LANE }, { "def": FIENDER, "lane": LANE, "stack": true }] },
            }));
            s.play(OCCIDENTAL, json!({ "zone": 1 }));
            assert!(s.unit(P2, LANE).is_some_and(|card| card.def_id == BODY));
        }

        #[test]
        fn r2196003_played_into_lane_5_it_destroys_itself_as_well() {
            let mut s = scenario(json!({
                "p1": { "hand": [OCCIDENTAL, FILLER] },
                "p2": { "hand": [FILLER], "field": [{ "def": BODY, "lane": LANE }] },
            }));
            s.play(OCCIDENTAL, json!({ "zone": LANE }));
            s.expect_in_zone(OCCIDENTAL, "graveyard");
            assert!(s.unit(P2, LANE).is_none());
        }

        #[test]
        fn s4_5_a_destroyed_card_s_death_fires() {
            let mut s = scenario(json!({
                "p1": { "hand": [OCCIDENTAL, FILLER] },
                "p2": { "hand": [FILLER], "library": [FILLER], "field": [{ "def": FLAGBEARER, "lane": LANE }] },
            }));
            s.play(OCCIDENTAL, json!({ "zone": 1 }));
            assert!(s.pile(P2, "library").iter().any(|card| card.def_id == PRIME));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2196003_an_18_10_that_destroys_only_the_enemy_s_lane_5_cards() {
            let mut s = swept(true, 1);
            s.expect_stats(OCCIDENTAL, json!({ "attack": 18, "health": 10 }));
            assert_eq!(holds(&s, P2, LANE), (false, false));
            assert_eq!(holds(&s, P1, LANE), (true, true));
        }

        #[test]
        fn r2196003_played_into_lane_5_it_spares_itself() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": OCCIDENTAL, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER], "field": [{ "def": BODY, "lane": LANE }] },
            }));
            s.play(OCCIDENTAL, json!({ "zone": LANE }));
            s.expect_in_zone(OCCIDENTAL, "field");
            assert!(s.unit(P2, LANE).is_none());
        }
    }
}
