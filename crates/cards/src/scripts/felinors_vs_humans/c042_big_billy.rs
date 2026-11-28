//! SPEC §8.14 FvH #42 Big Billy ((1) Unit, Human, Rare, 6/6 → 12/12), a card of the held Felinors vs Humans
//! set (R5235000). Built by S09 (#5259).
//!   Base:    "Stack. Can be played on either side. Cry: If you played this on your side, it becomes a
//!            {stats}/{stats}. Destroy the Units beneath this." (3)
//!   Radiant: "Stack. Can be played on either side. Cry: If you played this on your opponent's side, it
//!            becomes a {shrink}/{shrink}. Destroy the Units beneath this." (3)
//!
//! Engine: `StaticFlags.plays_on_either_side` with `ZoneChoice.player` (R5238002): played into an open zone
//! or, by Stack (§3.2), onto a pile on either side, it lands in that side's control, its owner unchanged, as
//! Silly Silas's summon for the opponent lands (R3837000); the play and its Cry stay its player's (R3837002).
//! The Cry (R1), whichever side it stands on: first `set_stats` of itself to the declared number when its
//! controller is its owner (base) or is not (Radiant), R5238002's reading of which side it went to; then
//! `crush` with `beneath_only` of the zone it stands in (R5238003, R5048000), named from its player's seat,
//! destroying only the Units beneath it in its pile. A Cry that does not run leaves it 6/6 (12/12). Tunes:
//! stats 3 ↑ on the base face, shrink 3 ↓ on the Radiant, each read by its Cry (R1748000).

use jackioh_engine::effects::{crush, set_stats};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-042";

fn billy(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            plays_on_either_side: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |ctx| {
            let Some(me) = ctx.self_.clone() else {
                return vec![];
            };
            let Zone::Field { player: side, row, lane } = me.zone else {
                return vec![];
            };
            let mut effects = Vec::new();
            // R5238002: on its own side it is its owner's; on the other side the opponent controls it.
            let on_own_side = me.controller == me.owner;
            if on_own_side != radiant {
                let stats = param(ctx, if radiant { "shrink" } else { "stats" });
                effects.push(set_stats(json_as(json!({
                    "target": { "of": "self" },
                    "attack": stats,
                    "health": stats,
                }))));
            }
            // The crush names a side from its player's seat, which on the other side is "enemy".
            let seat = if side == ctx.controller { "self" } else { "enemy" };
            let mut beneath: CrushArgs = json_as(json!({ "player": seat, "row": row, "lane": lane }));
            beneath.beneath_only = Some(true);
            effects.push(crush(beneath));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: billy(false),
        radiant: billy(true),
    }
}

// FvH #42 Big Billy — SPEC §8.14 row 42 (R5238002, R5238003, R5048000, R3837000, R1748000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const BODY: &str = "stand-in (1) 4/4";
    const FILLER: &str = "core-005"; // Stockpile: a card kept in hand, so no turn ends by itself

    fn board(radiant: bool, own: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "seed": "big-billy",
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": own,
                "library": crate::filler(3),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "field": theirs, "library": crate::filler(3) },
        }))
    }

    /// Plays Billy into `lane` of `side`'s unit row; returns its instance id.
    fn played(s: &mut Scenario, side: PlayerId, lane: i32) -> String {
        let billy = s.card(ID).id.clone();
        s.play(ID, json!({ "zone": lane, "player": side }));
        billy
    }

    #[test]
    fn r5238002_both_sides_zones_are_offered() {
        for radiant in [false, true] {
            let s = board(radiant, json!([]), json!([]));
            let sides: Vec<Option<PlayerId>> = s
                .plays_of(ID)
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Play { zone: Some(zone), .. } => Some(zone.player),
                    _ => None,
                })
                .collect();
            assert!(sides.contains(&Some(P2)), "radiant {radiant}: the opponent's zones, {sides:?}");
            assert!(sides.iter().any(|side| *side != Some(P2)), "radiant {radiant}: and its own");
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r5238002_played_on_your_side_it_becomes_a_3_3() {
            let mut s = board(false, json!([]), json!([]));
            let billy = played(&mut s, P1, 3);
            assert_eq!(s.unit_at(P1, 3).id, billy);
            s.expect_stats(billy.as_str(), json!({ "attack": 3, "health": 3, "maxHealth": 3 }));
        }

        #[test]
        fn r5238002_played_on_the_opponents_side_it_stays_6_6_in_their_control() {
            let mut s = board(false, json!([]), json!([]));
            let billy = played(&mut s, P2, 3);
            assert_eq!(s.unit_at(P2, 3).id, billy);
            assert!(s.unit(P1, 3).is_none());
            let card = s.card(billy.as_str()).clone();
            assert_eq!((card.controller, card.owner), (P2, P1));
            s.expect_stats(billy.as_str(), json!({ "attack": 6, "health": 6 }));
        }

        #[test]
        fn r5238003_stacked_on_your_own_unit_it_destroys_the_unit_beneath() {
            let mut s = board(false, json!([{ "def": BODY, "lane": 2 }]), json!([]));
            let beneath = s.unit_at(P1, 2);
            let billy = played(&mut s, P1, 2);
            s.expect_in_zone(&beneath, "graveyard");
            assert_eq!(s.unit_at(P1, 2).id, billy, "it stands");
            s.expect_stats(billy.as_str(), json!({ "attack": 3, "health": 3 }));
        }

        #[test]
        fn r5238003_stacked_on_an_enemy_unit_it_destroys_it_and_stands_on_their_side() {
            let pile = json!([{ "def": BODY, "lane": 2 }]);
            let mut s = board(false, pile.clone(), pile);
            let (own, foe) = (s.unit_at(P1, 2), s.unit_at(P2, 2));
            let billy = played(&mut s, P2, 2);
            s.expect_in_zone(&foe, "graveyard");
            s.expect_in_zone(&own, "field");
            assert_eq!(s.unit_at(P2, 2).id, billy);
            assert_eq!(s.card(billy.as_str()).controller, P2);
            s.expect_stats(billy.as_str(), json!({ "attack": 6, "health": 6 }));
        }

        #[test]
        fn r1748000_the_stats_read_through_param() {
            let mut s = board(false, json!([]), json!([]));
            let stats = crate::upgrade_number(&mut s, ID, "stats");
            assert_eq!(stats, 4);
            let billy = played(&mut s, P1, 1);
            s.expect_stats(billy.as_str(), json!({ "attack": stats, "health": stats }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5238002_played_on_your_side_it_stays_12_12() {
            let mut s = board(true, json!([]), json!([]));
            let billy = played(&mut s, P1, 3);
            s.expect_stats(billy.as_str(), json!({ "attack": 12, "health": 12 }));
        }

        #[test]
        fn r5238002_played_on_the_opponents_side_it_becomes_a_3_3() {
            let mut s = board(true, json!([]), json!([]));
            let billy = played(&mut s, P2, 4);
            assert_eq!(s.unit_at(P2, 4).id, billy);
            s.expect_stats(billy.as_str(), json!({ "attack": 3, "health": 3, "maxHealth": 3 }));
        }

        #[test]
        fn r5238003_stacked_on_an_enemy_unit_it_destroys_it_and_becomes_a_3_3() {
            let mut s = board(true, json!([]), json!([{ "def": BODY, "lane": 1 }]));
            let foe = s.unit_at(P2, 1);
            let billy = played(&mut s, P2, 1);
            s.expect_in_zone(&foe, "graveyard");
            assert_eq!(s.unit_at(P2, 1).id, billy);
            s.expect_stats(billy.as_str(), json!({ "attack": 3, "health": 3 }));
        }

        #[test]
        fn r5238003_stacked_on_your_own_unit_it_destroys_it_and_stays_12_12() {
            let mut s = board(true, json!([{ "def": BODY, "lane": 5 }]), json!([]));
            let own = s.unit_at(P1, 5);
            let billy = played(&mut s, P1, 5);
            s.expect_in_zone(&own, "graveyard");
            s.expect_stats(billy.as_str(), json!({ "attack": 12, "health": 12 }));
        }
    }
}
