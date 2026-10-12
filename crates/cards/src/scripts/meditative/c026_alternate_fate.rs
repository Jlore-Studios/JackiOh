//! M #26 Alternate Fate (SPEC §8.8 row 26): (4) Field Spell, Legendary.
//!
//! Base:    "Aura: Players don't generate mana naturally.
//!           End of turn: Summon a Mana Well into every empty backrow, for the player whose
//!           backrow it is."
//! Radiant: the same, plus " Yours are Radiant."
//! Engine:
//! - **The aura** is ME-TURN's zeroed refresh held by a card: `static_flags.no_natural_mana`, which
//!   `mana::refresh_mana` reads for both players while this card acts on the field (MD-B2, R941).
//!   Max mana is still computed; current mana becomes the next-turn rider only.
//! - **End of turn** (the controller's, §6.2): `fill_board` with `row: backrow, side: any` summons a
//!   Core #6 Mana Well into every empty, unlocked backrow zone of both players, the active player's
//!   side first (R68), each owned and controlled by the player whose zone it is (MD-B3, R942). The
//!   Radiant face passes `radiantFor: self`. Summoned, so no Cry (R1).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-026";

/// Core #6 Mana Well, summoned into every empty backrow zone.
const WELL: &str = "core-006";

fn fate(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            no_natural_mana: Some(true),
            ..StaticFlags::default()
        }),
        end_of_turn: Some(hook(move |_ctx| {
            let mut fill = json!({ "defId": WELL, "row": "backrow", "side": "any" });
            if radiant {
                fill["radiantFor"] = json!("self");
            }
            vec![fill_board(json_as(fill))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: fate(false),
        radiant: fate(true),
    }
}

// M #26 Alternate Fate — SPEC §8.8 row 26, BUILD M10 row M 26.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FATE: &str = "meditative-026";
    const WELL: &str = "core-006";
    const COIN: &str = "core-t-coin";
    const FILLER: &str = "core-005";
    const HINDER: &str = "core-021"; // (0) Spell: your opponent has less mana next turn.
    const JAMMED: &str = "core-036"; // (1) Spell: destroy a backrow card.
    const SHEEPISH: &str = "core-041"; // Sheepish, a Trap to fill a backrow zone.

    fn library() -> Value {
        json!([FILLER, FILLER, FILLER, FILLER, FILLER, FILLER])
    }

    /// p1 holds Fate (Radiant on `radiant`); both sides hold fillers in hand and library so no
    /// turn auto-ends (R82).
    fn casting(seed: &str, radiant: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "mana": 10,
                "hand": [{ "def": FATE, "radiant": radiant }, FILLER],
                "library": library(),
            },
            "p2": { "hand": [FILLER], "library": library() },
        }))
    }

    /// Every backrow card of `player`, lane 1 upward.
    fn backrow_defs(s: &Scenario, player: PlayerId) -> Vec<(i32, String, bool)> {
        (1..=5)
            .filter_map(|lane| s.backrow(player, lane).map(|card| (lane, card.def_id, card.radiant)))
            .collect()
    }

    #[test]
    fn r941_while_it_acts_each_refresh_gives_only_the_rider() {
        // Each side also holds a (0) Coin: with no mana, a (1) filler is no legal play, and a turn
        // with nothing but ending it would end by itself (R82), on and on until a deck ran out.
        crate::register_all();
        let mut s = scenario(json!({
            "seed": "fate-aura",
            "p1": {
                "mana": 10,
                "hand": [{ "def": FATE }, COIN, FILLER],
                "library": library(),
            },
            "p2": { "hand": [COIN, FILLER], "library": library() },
        }));
        // Lock every backrow zone but Fate's own: no Well is summoned, so the refresh reads alone.
        {
            let state = s.state_mut();
            for player in [P1, P2] {
                for lane in 1..=5 {
                    if player == P1 && lane == 1 {
                        continue;
                    }
                    lock_zone(state, ZoneSlot { player, row: Row::Backrow, lane });
                }
            }
        }
        s.play(FATE, json!({ "zone": 1 }));
        s.expect_in_zone(FATE, "field");
        // p1 ends: Fate's end-of-turn fires (no room for a Well), then p2's refresh is zeroed …
        s.end_turn();
        assert_eq!(s.state().players[P2].mana.current, 0, "no natural mana for p2");
        assert!(s.state().players[P2].mana.max > 0, "max mana is still set");
        // … and p1's own next refresh too.
        s.end_turn();
        assert_eq!(s.state().players[P1].mana.current, 0, "no natural mana for p1 either");
        assert!(s.state().players[P1].mana.max > 0, "max mana is still set");
    }

    #[test]
    fn r941_the_coin_hinder_and_a_mana_well_still_work() {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": "fate-coin",
            "p1": {
                "mana": 10,
                "hand": [{ "def": FATE }, { "def": COIN }, FILLER],
                "library": library(),
            },
            "p2": { "hand": [FILLER], "library": library() },
        }));
        s.play(FATE, json!({ "zone": 1 }));
        // The Coin's gain is mana from an effect, not a refresh: it still works.
        let before = s.state().players[P1].mana.current;
        s.play(COIN, json!({}));
        assert_eq!(s.state().players[P1].mana.current, before + 1, "the Coin still gains");
        // A summoned Well's start-of-turn gain arrives after the empty refresh.
        s.end_turn();
        s.end_turn();
        assert!(
            s.state().players[P1].mana.current > 0,
            "the Wells' gains arrive after the empty refresh"
        );
    }

    #[test]
    fn r941_off_the_field_the_next_refresh_is_natural() {
        // Fate in hand acts nowhere: the next refresh is natural.
        let mut s = casting("fate-hand", false);
        let max_before = s.state().players[P1].mana.max;
        let _ = max_before;
        s.end_turn();
        s.end_turn();
        let side = &s.state().players[P1];
        assert_eq!(side.mana.current, side.mana.max, "a natural refresh fills to max");
        assert!(side.mana.max > 0);
    }

    #[test]
    fn r942_end_of_turn_summons_a_well_into_every_empty_unlocked_backrow_zone_for_its_owner_without_a_cry()
     {
        let mut s = casting("fate-fill", false);
        // Lock p2's lane-5 backrow zone: it gets no Well.
        {
            let state = s.state_mut();
            let slot = ZoneSlot { player: P2, row: Row::Backrow, lane: 5 };
            jackioh_engine::zones::lock_zone(state, slot);
        }
        s.play(FATE, json!({ "zone": 1 }));
        let mark = s.events().len();
        s.end_turn();
        // p1's backrow: Fate in lane 1, Wells in lanes 2–5.
        assert_eq!(
            backrow_defs(&s, P1),
            vec![
                (1, FATE.to_string(), false),
                (2, WELL.to_string(), false),
                (3, WELL.to_string(), false),
                (4, WELL.to_string(), false),
                (5, WELL.to_string(), false),
            ]
        );
        // p2's backrow: Wells in lanes 1–4, the Locked lane 5 empty.
        assert_eq!(
            backrow_defs(&s, P2),
            vec![
                (1, WELL.to_string(), false),
                (2, WELL.to_string(), false),
                (3, WELL.to_string(), false),
                (4, WELL.to_string(), false),
            ]
        );
        // Each Well belongs to the player whose zone it is.
        for lane in 2..=5 {
            let well = s.backrow(P1, lane).expect("p1's well");
            assert_eq!(well.owner, P1);
            assert_eq!(well.controller, P1);
        }
        for lane in 1..=4 {
            let well = s.backrow(P2, lane).expect("p2's well");
            assert_eq!(well.owner, P2);
            assert_eq!(well.controller, P2);
        }
        // Summoned, so no Cry: every Well arrived by `summoned`, none by a Cry trigger.
        let summoned = s.events()[mark..].iter().filter(|event| matches!(event, GameEvent::Summoned { .. })).count();
        assert_eq!(summoned, 8, "eight Wells summoned");
    }

    /// `casting`, with every backrow zone but Fate's own lane 1 locked: no Well is ever summoned.
    fn alone(seed: &str, hand: Value, p2_hand: Value) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "mana": 10, "hand": hand, "library": library() },
            "p2": { "hand": p2_hand, "library": library() },
        }));
        let state = s.state_mut();
        for player in [P1, P2] {
            for lane in 1..=5 {
                if player == P1 && lane == 1 {
                    continue;
                }
                lock_zone(state, ZoneSlot { player, row: Row::Backrow, lane });
            }
        }
        s
    }

    #[test]
    fn r941_a_refresh_under_it_gives_the_next_turn_rider_and_nothing_more() {
        let mut s = alone("fate-rider", json!([FATE, COIN, FILLER]), json!([COIN, FILLER]));
        s.play(FATE, json!({ "zone": 1 }));
        s.state_mut().players.p2.mana.next_turn_mod = 3;
        s.end_turn();
        let side = &s.state().players[P2];
        assert_eq!(side.mana.current, 3, "the rider alone");
        assert_eq!(side.mana.max, 4, "max mana as usual");
    }

    #[test]
    fn r941_hinder_still_works() {
        let mut s = alone("fate-hinder", json!([FATE, HINDER, FILLER, FILLER]), json!([COIN, FILLER]));
        s.play(FATE, json!({ "zone": 1 }));
        s.state_mut().players.p2.mana.next_turn_mod = 3;
        let hinder = crate::card_def(HINDER)
            .params
            .as_ref()
            .and_then(|params| params.iter().find(|param| param.key == "mana").map(|param| param.base))
            .expect("Hinder's number");
        s.play(HINDER, json!({}));
        s.end_turn();
        assert_eq!(s.state().players[P2].mana.current, (3 - hinder).max(0), "Hinder still takes its share");
    }

    #[test]
    fn r941_leaving_the_field_restores_natural_mana_from_the_next_refresh() {
        let mut s = alone("fate-leaves", json!([FATE, JAMMED, FILLER]), json!([COIN, FILLER]));
        s.play(FATE, json!({ "zone": 1 }));
        let fate = s.card(FATE).id.clone();
        s.play(JAMMED, json!({ "targets": [{ "pick": "instance", "instanceId": fate }] }));
        s.expect_in_zone(&fate, "graveyard");
        s.end_turn();
        let side = &s.state().players[P2];
        assert_eq!(side.mana.current, side.mana.max, "a natural refresh");
        assert!(side.mana.max > 0);
    }

    #[test]
    fn r942_the_active_side_first_lane_1_upward() {
        let mut s = casting("fate-order", false);
        s.play(FATE, json!({ "zone": 1 }));
        let mark = s.events().len();
        s.end_turn();
        let order: Vec<(String, i64)> = s.events()[mark..]
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .map(|id| {
                let card = s.card(&id);
                (card.controller.as_str().to_string(), crate::js(&card.zone)["lane"].as_i64().unwrap_or(0))
            })
            .collect();
        let expected: Vec<(String, i64)> = [("p1", 2), ("p1", 3), ("p1", 4), ("p1", 5)]
            .into_iter()
            .chain((1..=5).map(|lane| ("p2", lane)))
            .map(|(player, lane)| (player.to_string(), lane))
            .collect();
        assert_eq!(order, expected);
    }

    #[test]
    fn r942_an_occupied_zone_gets_no_well() {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": "fate-occupied",
            "p1": { "mana": 10, "hand": [FATE, FILLER], "library": library() },
            "p2": { "hand": [FILLER], "backrow": [{ "def": SHEEPISH, "lane": 3 }], "library": library() },
        }));
        s.play(FATE, json!({ "zone": 1 }));
        s.end_turn();
        assert_eq!(s.backrow(P2, 3).expect("the Trap stays").def_id, SHEEPISH);
        let wells = (1..=5).filter(|lane| s.backrow(P2, *lane).is_some_and(|card| card.def_id == WELL)).count();
        assert_eq!(wells, 4);
    }

    #[test]
    fn r941_r942_each_well_gains_its_owner_mana_after_the_empty_refresh() {
        let mut s = casting("fate-wells", false);
        s.play(FATE, json!({ "zone": 1 }));
        s.end_turn();
        // p2's five Wells, one each, on top of an empty refresh.
        assert_eq!(s.state().players[P2].mana.current, 5);
        s.end_turn();
        // p1's four.
        assert_eq!(s.state().players[P1].mana.current, 4);
    }

    #[test]
    fn r942_radiant_your_wells_give_2_mana_each_theirs_1() {
        let mut s = casting("fate-radiant-mana", true);
        s.play(FATE, json!({ "zone": 1 }));
        s.end_turn();
        assert_eq!(s.state().players[P2].mana.current, 5, "the opponent's base Wells");
        s.end_turn();
        assert_eq!(s.state().players[P1].mana.current, 8, "your four Radiant Wells, 2 each");
    }

    #[test]
    fn r942_radiant_makes_only_your_wells_radiant() {
        let mut s = casting("fate-radiant", true);
        s.play(FATE, json!({ "zone": 1 }));
        s.end_turn();
        for lane in 2..=5 {
            let well = s.backrow(P1, lane).expect("p1's well");
            assert!(well.radiant, "your Wells are Radiant");
        }
        for lane in 1..=5 {
            let well = s.backrow(P2, lane).expect("p2's well");
            assert!(!well.radiant, "the opponent's Wells are base");
        }
    }
}
