//! SPEC §8.12 S+ #76 Ayys LMAO, a card of the Stinkyscape+ set (R1420, R3535000). (2) Spell,
//! Common.
//!   Base:    "Summon a mix of {summons} Cloaked Ayy tokens or Cloaked Tiny Asteroids."
//!   Radiant: "Summon a mix of {summons} Radiant Cloaked Ayy tokens or Cloaked Tiny Asteroids."
//!
//! `summons` summons (3), each its own fair pick between the Ayy (S+ #76.1) and Stinkyscape #169 Tiny
//! Asteroid ("Small Asteroids", R3557003), repeats allowed with no limit from the pool's two (R60). The
//! pick is drawn in the Cry, so a replay folds it (§9.3), and each summon is a fresh card of the pick's
//! definition with no Cry (R1). Each summon is set face-down in the leftmost open backrow zone by its
//! Cloaked, the Ayy's printed and the asteroid's granted by the summon, silently (`setCloaked`, R4327000,
//! R4327002), hidden from the opponent until its Activate: Reveal; with no backrow zone open it is
//! summoned face-up into the leftmost open unit zone (R64). Neither the set nor the reveal is a Unit
//! summoned (R4327001). A full board summons fewer (R129). The Radiant face's Ayys are Radiant (4/4 and
//! Poisonous), read as the card they are while set (R783005); the adjective is the Ayy's, so its
//! asteroids stay base.

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-076";

/// S+ #76.1 Ayy, the token.
const AYY: &str = "stinkyscapeplus-076-1";
/// Stinkyscape #169 Tiny Asteroid, the card the text calls "Small Asteroids" (R3557003).
const ASTEROID: &str = "stinkyscape-169";

/// The Cry: `summons` fair picks, each summoned set face-down by its Cloaked when `set` (R4327002), else
/// as through v0.6.8b (R3557003).
fn ayys(radiant: bool, set: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let summons = param(ctx, "summons");
            (0..summons)
                .map(|_| {
                    let pick = ctx.rng.pick(&[AYY, ASTEROID]).copied().unwrap_or(AYY);
                    let mut at = if pick == AYY {
                        json!({ "defId": AYY, "radiant": radiant })
                    } else {
                        json!({ "defId": ASTEROID, "keywords": [Keyword::Cloaked] })
                    };
                    if set {
                        at["setCloaked"] = json!(true);
                    } else if pick == AYY {
                        at["backrowFirst"] = json!(true);
                    }
                    summon(json_as(at))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: ayys(false, true),
        radiant: ayys(true, true),
    }
}

/// R767000, R4327002: the Ayys standing in the backrow and the asteroids in the unit row (R3557003),
/// through v0.6.8b (issue #4327); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_8b() -> CardScripts {
    CardScripts {
        base: ayys(false, false),
        radiant: ayys(true, false),
    }
}

// S+ #76 Ayys LMAO — SPEC §8.12 row 76 (R3557003, R4327000–R4327002).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::{ASTEROID, AYY, ID, script_v0_6_8b};
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    /// C #5 Tesla: "Reveals when your opponent summons a Unit".
    const TESLA: &str = "classic-005";

    fn board(radiant: bool, seed: &str, p1: Value) -> Scenario {
        board_against(radiant, seed, p1, json!({}))
    }

    fn board_against(radiant: bool, seed: &str, p1: Value, p2: Value) -> Scenario {
        let mut p1 = p1;
        p1["hand"] = json!([{ "def": ID, "radiant": radiant }, FILLER]);
        p1["library"] = filler(3);
        p1["mana"] = json!(10);
        let mut p2 = p2;
        p2["hand"] = json!([FILLER]);
        p2["library"] = filler(4);
        scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
    }

    /// Every card the cast summoned, by definition.
    fn summoned(s: &Scenario) -> Vec<CardInstance> {
        let side = &s.state().players[P1];
        let mut cards: Vec<CardInstance> = side.units.iter().flatten().filter_map(|pile| pile.first().cloned()).collect();
        cards.extend(side.backrow.iter().flatten().cloned());
        cards.retain(|card| card.def_id == AYY || card.def_id == ASTEROID);
        cards
    }

    /// Whether this card is set face-down by its Cloaked.
    fn is_set(card: &CardInstance) -> bool {
        card.set_as.map(|stamp| stamp.reveal) == Some(RevealAt::Cloaked)
    }

    /// The Activate: Reveal its controller is offered on this set card (R783006).
    fn reveal_offered(s: &Scenario, card: &CardInstance) -> bool {
        s.activations_of(card.id.as_str()).iter().any(|action| {
            matches!(action, ActionBody::Activate { ability, .. } if ability.as_deref() == Some("reveal"))
        })
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
                // The Ayy prints Cloaked; the asteroid is given it by the summon.
                assert!(
                    card.def_id == AYY || card.granted_keywords.contains(&Keyword::Cloaked),
                    "{} is Cloaked",
                    card.def_id
                );
                assert!(!card.radiant);
            }
            seen_ayy |= cards.iter().any(|card| card.def_id == AYY);
            seen_asteroid |= cards.iter().any(|card| card.def_id == ASTEROID);
        }
        assert!(seen_ayy && seen_asteroid, "both kinds came up across seeds");
    }

    #[test]
    fn r4327002_all_three_are_set_face_down_in_the_leftmost_backrow_zones() {
        crate::register_all();
        for n in 0..12 {
            let mut s = board(false, &format!("r4327002-set-{n}"), json!({}));
            s.play(ID, json!({}));
            assert!(s.state().pending.is_none(), "no prompt");
            assert!(s.state().players[P1].units.iter().all(Option::is_none), "the unit row stays empty");
            for lane in 1..=3 {
                let card = s.backrow(P1, lane).unwrap_or_else(|| panic!("a set card in backrow lane {lane}"));
                assert!(card.def_id == AYY || card.def_id == ASTEROID, "{}", card.def_id);
                assert!(is_set(&card), "{} is set by its Cloaked", card.def_id);
                assert_eq!(card.face_up, None, "{} is face-down", card.def_id);
                assert_eq!(faces::card_type_of(s.state(), &card), CardType::FieldTrap, "{}", card.def_id);
                assert!(!jackioh_engine::zones::stands_in_backrow(s.state(), &card), "{} is no Unit", card.def_id);
            }
            assert_eq!(summoned(&s).len(), 3);
        }
    }

    #[test]
    fn r4327000_p2_sees_three_backs_and_no_event_names_them() {
        crate::register_all();
        for n in 0..6 {
            let mut s = board(n % 2 == 1, &format!("r4327000-hidden-{n}"), json!({}));
            s.play(ID, json!({}));
            let theirs = serde_json::to_value(s.view(P2)).expect("a view serializes");
            let mine = serde_json::to_value(s.view(P1)).expect("a view serializes");
            for lane in 0..3 {
                let back = &theirs["opponent"]["backrow"][lane];
                assert_eq!(back["faceDown"], json!(true), "{back}");
                for hidden in ["defId", "instanceId", "radiant", "revealAt", "activations", "keywords"] {
                    assert!(back.get(hidden).is_none(), "{hidden} is hidden: {back}");
                }
                assert_eq!(mine["you"]["backrow"][lane]["revealAt"], json!("cloaked"));
            }
            let events = theirs["events"].as_array().cloned().unwrap_or_default();
            let summons: Vec<&Value> = events.iter().filter(|event| event["type"] == "summoned").collect();
            assert_eq!(summons.len(), 3);
            for event in summons {
                assert_eq!(event["defId"], json!("hidden"), "{event}");
                assert_eq!(event["instanceId"], json!("hidden"), "{event}");
            }
            assert!(!events.iter().any(|event| event["type"] == "keywordGranted"), "the asteroids' Cloaked is silent");
        }
    }

    #[test]
    fn r4327000_each_is_revealed_into_a_unit_zone_summoning_sick() {
        crate::register_all();
        let mut s = board(false, "r4327000-reveal", json!({}));
        s.play(ID, json!({}));
        let set: Vec<CardInstance> = (1..=3).filter_map(|lane| s.backrow(P1, lane)).collect();
        assert_eq!(set.len(), 3);
        for card in &set {
            assert!(reveal_offered(&s, card), "{} offers its Reveal", card.def_id);
        }
        let first = &set[0];
        s.activate(first.id.as_str(), json!({ "ability": "reveal" }));
        let revealed = s.unit(P1, 1).expect("the revealed card in unit lane 1");
        assert_eq!(revealed.id, first.id);
        assert_eq!(revealed.set_as, None);
        assert_eq!(revealed.summoned_turn, Some(s.state().turn), "summoning sick");
        assert!(s.attacks_of(first.id.as_str()).is_empty(), "no attack the turn it is revealed");
        assert!(s.backrow(P1, 1).is_none());
    }

    #[test]
    fn r4327001_an_opposing_tesla_answers_neither_the_sets_nor_a_reveal() {
        crate::register_all();
        let mut checked = false;
        for n in 0..6 {
            let mut s = board_against(
                false,
                &format!("r4327001-tesla-{n}"),
                json!({}),
                json!({ "backrow": [{ "def": TESLA, "faceUp": false, "lane": 1 }] }),
            );
            s.play(ID, json!({}));
            assert_eq!(s.count("trapFired"), 0, "no set summon is a Unit summoned");
            // Reveal an Ayy, the one of the two that can attack.
            let Some(ayy) = (1..=3).filter_map(|lane| s.backrow(P1, lane)).find(|card| card.def_id == AYY) else {
                continue;
            };
            s.activate(ayy.id.as_str(), json!({ "ability": "reveal" }));
            assert!(matches!(s.card(ayy.id.as_str()).zone, Zone::Field { row: Row::Units, .. }), "the Ayy came out");
            // Its own Reveal fires it as a Trap; Tesla answers nothing.
            assert!(
                s.events_of("trapFired").iter().all(|event| matches!(event,
                    GameEvent::TrapFired { instance_id, .. } if *instance_id == ayy.id)),
                "only the Ayy's own Reveal fired"
            );
            let tesla = s.backrow(P2, 1).expect("Tesla still set");
            assert_eq!(tesla.def_id, TESLA);
            assert_ne!(tesla.face_up, Some(true), "Tesla is still face-down");
            checked = true;
        }
        assert!(checked, "an Ayy came up");
    }

    #[test]
    fn r4327002_with_no_backrow_zone_open_each_summon_takes_the_unit_row_face_up() {
        crate::register_all();
        // Every backrow zone taken: each falls back to the unit row, face-up.
        let traps: Vec<Value> = (0..5).map(|_| json!("stand-in (1) Trap")).collect();
        for n in 0..6 {
            let mut s = board(false, &format!("r4327002-full-backrow-{n}"), json!({ "backrow": traps }));
            s.play(ID, json!({}));
            let cards = summoned(&s);
            assert_eq!(cards.len(), 3);
            for card in &cards {
                assert!(matches!(card.zone, Zone::Field { row: Row::Units, .. }), "{}", card.def_id);
                assert_eq!(card.set_as, None, "{}", card.def_id);
                assert!(unit_has(s.state(), card, KeywordKind::Cloaked), "{} is Cloaked", card.def_id);
            }
        }
    }

    #[test]
    fn r4327002_with_one_backrow_zone_open_the_first_is_set_and_the_rest_take_the_unit_row() {
        crate::register_all();
        let traps: Vec<Value> = (1..=4).map(|lane| json!({ "def": "stand-in (1) Trap", "lane": lane })).collect();
        let mut s = board(false, "r4327002-one-open", json!({ "backrow": traps }));
        s.play(ID, json!({}));
        let set = s.backrow(P1, 5).expect("the first summon set in backrow lane 5");
        assert!(is_set(&set), "{}", set.def_id);
        let face_up: Vec<CardInstance> = summoned(&s).into_iter().filter(|card| card.id != set.id).collect();
        assert_eq!(face_up.len(), 2);
        for card in &face_up {
            assert!(matches!(card.zone, Zone::Field { row: Row::Units, .. }), "{}", card.def_id);
            assert_eq!(card.set_as, None);
        }
    }

    #[test]
    fn r129_a_full_board_summons_fewer() {
        crate::register_all();
        let units: Vec<Value> = (0..4).map(|_| json!("stand-in (1) 1/1")).collect();
        let traps: Vec<Value> = (0..5).map(|_| json!("stand-in (1) Trap")).collect();
        let mut s = board(false, "r3557003-full", json!({ "field": units, "backrow": traps }));
        s.play(ID, json!({}));
        let cards = summoned(&s);
        assert_eq!(cards.len(), 1, "one unit zone was left");
        assert_eq!(cards[0].set_as, None, "it is summoned face-up");
    }

    #[test]
    fn r4327002_the_radiant_face_sets_radiant_ayys_and_base_asteroids() {
        crate::register_all();
        let mut checked = false;
        for n in 0..12 {
            let mut s = board(true, &format!("r3557003-radiant-{n}"), json!({}));
            s.play(ID, json!({}));
            for card in summoned(&s) {
                assert!(is_set(&card), "{}", card.def_id);
                assert_eq!(card.radiant, card.def_id == AYY);
            }
            if checked {
                continue;
            }
            let Some(ayy) = summoned(&s).into_iter().find(|card| card.def_id == AYY) else {
                continue;
            };
            s.activate(ayy.id.as_str(), json!({ "ability": "reveal" }));
            let revealed = s.card(ayy.id.as_str()).clone();
            let view = unit_view(s.state(), &revealed);
            assert_eq!((view.attack, view.health), (4, 4));
            assert!(unit_has(s.state(), &revealed, KeywordKind::Poisonous));
            checked = true;
        }
        assert!(checked);
    }

    /// R767000, R3557003: through v0.6.8b the Ayys stood in the backrow as Units and the asteroids took
    /// the unit row, face-up, and a replay of those games folds under that script.
    #[test]
    fn r3557003_r767000_the_kept_v0_6_8b_script_stands_the_ayys_in_the_backrow_and_the_asteroids_in_the_unit_row() {
        crate::under_kept(ID, script_v0_6_8b(), || {
            for n in 0..6 {
                let mut s = board(false, &format!("r3557003-places-{n}"), json!({}));
                s.play(ID, json!({}));
                let cards = summoned(&s);
                assert_eq!(cards.len(), 3);
                for card in cards {
                    assert_eq!(card.set_as, None, "{} is face-up", card.def_id);
                    let in_backrow = matches!(card.zone, Zone::Field { row: Row::Backrow, .. });
                    assert_eq!(in_backrow, card.def_id == AYY, "{} stands in the backrow only as an Ayy", card.def_id);
                    if in_backrow {
                        assert!(jackioh_engine::zones::stands_in_backrow(s.state(), &card), "it stands as a Unit");
                    }
                }
            }
        });
    }
}
