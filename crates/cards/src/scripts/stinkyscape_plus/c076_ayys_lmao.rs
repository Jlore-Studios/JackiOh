//! SPEC §8.12 S+ #76 Ayys LMAO, a card of the Stinkyscape+ set (R1420, R3535000). (2) Spell,
//! Common.
//!   Base:    "Summon a mix of {summons} Cloaked Ayy tokens or Cloaked Tiny Asteroids."
//!   Radiant: "Summon a mix of {summons} Radiant Cloaked Ayy tokens or Cloaked Tiny Asteroids."
//!
//! `summons` summons (3), each its own fair pick between the Ayy (S+ #76.1) and Stinkyscape #169 Tiny
//! Asteroid ("Small Asteroids", R3557003), repeats allowed with no limit from the pool's two (R60). The
//! pick is drawn in the Cry, so a replay folds it (§9.3), and each summon is a fresh card of the pick's
//! definition with no Cry (R1). An Ayy is Cloaked as its catalog prints it and goes to the leftmost open
//! backrow zone where it stands as a Unit for every rule (`backrowFirst`, R1081000, R3557003), else the
//! leftmost open unit zone (R64); a Tiny Asteroid is granted Cloaked and takes a unit zone. A full board
//! summons fewer (R129). The Radiant face's Ayys are Radiant (4/4 and Poisonous); the adjective is the
//! Ayy's, so its asteroids stay base.

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-076";

/// S+ #76.1 Ayy, the token.
const AYY: &str = "stinkyscapeplus-076-1";
/// Stinkyscape #169 Tiny Asteroid, the card the text calls "Small Asteroids" (R3557003).
const ASTEROID: &str = "stinkyscape-169";

fn ayys(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let summons = param(ctx, "summons");
            (0..summons)
                .map(|_| {
                    let pick = ctx.rng.pick(&[AYY, ASTEROID]).copied().unwrap_or(AYY);
                    if pick == AYY {
                        summon(json_as(json!({ "defId": AYY, "radiant": radiant, "backrowFirst": true })))
                    } else {
                        summon(json_as(json!({ "defId": ASTEROID, "keywords": [Keyword::Cloaked] })))
                    }
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: ayys(false),
        radiant: ayys(true),
    }
}

// S+ #76 Ayys LMAO — SPEC §8.12 row 76 (R3557003).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::{ASTEROID, AYY, ID};
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    fn board(radiant: bool, seed: &str, p1: Value) -> Scenario {
        let mut p1 = p1;
        p1["hand"] = json!([{ "def": ID, "radiant": radiant }, FILLER]);
        p1["library"] = filler(3);
        p1["mana"] = json!(10);
        scenario(json!({
            "seed": seed,
            "p1": p1,
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// Every card the cast summoned, by definition.
    fn summoned(s: &Scenario) -> Vec<CardInstance> {
        let side = &s.state().players[P1];
        let mut cards: Vec<CardInstance> = side.units.iter().flatten().filter_map(|pile| pile.first().cloned()).collect();
        cards.extend(side.backrow.iter().flatten().cloned());
        cards.retain(|card| card.def_id == AYY || card.def_id == ASTEROID);
        cards
    }

    #[test]
    fn r3557003_three_cloaked_summons_each_an_ayy_or_an_asteroid() {
        crate::register_all();
        let mut seen_ayy = false;
        let mut seen_asteroid = false;
        for n in 0..12 {
            let mut s = board(false, &format!("r3557003-mix-{n}"), json!({}));
            s.play(ID, json!({}));
            let cards = summoned(&s);
            assert_eq!(cards.len(), 3, "three summons");
            for card in &cards {
                assert!(unit_has(s.state(), card, KeywordKind::Cloaked), "{} is Cloaked", card.def_id);
                assert!(!card.radiant);
            }
            seen_ayy |= cards.iter().any(|card| card.def_id == AYY);
            seen_asteroid |= cards.iter().any(|card| card.def_id == ASTEROID);
        }
        assert!(seen_ayy && seen_asteroid, "both kinds came up across seeds");
    }

    #[test]
    fn r3557003_the_ayys_stand_in_the_backrow_as_units_and_the_asteroids_take_the_unit_row() {
        crate::register_all();
        let mut s = board(false, "r3557003-places", json!({}));
        s.play(ID, json!({}));
        for card in summoned(&s) {
            let in_backrow = matches!(card.zone, Zone::Field { row: Row::Backrow, .. });
            assert_eq!(in_backrow, card.def_id == AYY, "{} stands in the backrow only as an Ayy", card.def_id);
            if in_backrow {
                assert!(jackioh_engine::zones::stands_in_backrow(s.state(), &card), "it stands as a Unit");
            }
        }
    }

    #[test]
    fn r3557003_with_no_backrow_zone_free_an_ayy_takes_the_unit_row() {
        crate::register_all();
        // Every backrow zone taken: the Ayys fall back to the unit row.
        let traps: Vec<Value> = (0..5).map(|_| json!("stand-in (1) Trap")).collect();
        let mut s = board(false, "r3557003-full-backrow", json!({ "backrow": traps }));
        s.play(ID, json!({}));
        let cards = summoned(&s);
        assert_eq!(cards.len(), 3);
        assert!(cards.iter().all(|card| matches!(card.zone, Zone::Field { row: Row::Units, .. })));
    }

    #[test]
    fn r129_a_full_board_summons_fewer() {
        crate::register_all();
        let units: Vec<Value> = (0..4).map(|_| json!("stand-in (1) 1/1")).collect();
        let traps: Vec<Value> = (0..5).map(|_| json!("stand-in (1) Trap")).collect();
        let mut s = board(false, "r3557003-full", json!({ "field": units, "backrow": traps }));
        s.play(ID, json!({}));
        assert_eq!(summoned(&s).len(), 1, "one unit zone was left");
    }

    #[test]
    fn the_radiant_face_summons_radiant_ayys_and_base_asteroids() {
        crate::register_all();
        let mut checked = false;
        for n in 0..12 {
            let mut s = board(true, &format!("r3557003-radiant-{n}"), json!({}));
            s.play(ID, json!({}));
            for card in summoned(&s) {
                assert_eq!(card.radiant, card.def_id == AYY);
                if card.def_id == AYY {
                    let view = unit_view(s.state(), &card);
                    assert_eq!((view.attack, view.health), (4, 4));
                    assert!(unit_has(s.state(), &card, KeywordKind::Poisonous));
                    checked = true;
                }
            }
        }
        assert!(checked);
    }
}
