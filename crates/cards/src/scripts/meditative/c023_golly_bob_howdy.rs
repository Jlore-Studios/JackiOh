//! M #23 Golly Bob Howdy (SPEC §8.8 row 23, §3.1, §3.2, §7, R11, R13, R23, R691): (1) Unit, Human, Common,
//! 3/5 → 6/10.
//!
//! Base:    "Cry: Transform the Units next to this into Sheep Tokens."
//! Radiant: "Cry: Transform the Units next to this into Radiant Sheep Tokens."
//! Engine: `for_each_card` over the Units next to it (§3.1: lanes N − 1 and N + 1 on its own side and
//! row, the tops of piles, R13), listed once before the first transform. Each becomes a Sheep Token in
//! place (R691): its Cry does not fire, an Immutable one is left alone (R23), and the Unit token it
//! replaces ceases to exist (R11). A Sheep is worth 2 Tributes, a Radiant one 3 (§3.2, §7).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-023";

/// §7: the Sheep Token.
const SHEEP_TOKEN: &str = "core-t-sheep";

fn golly_bob_howdy(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(|at: &mut EffectContext<'_>| {
                    adjacent_to(at, &json_as(json!({ "of": "self" })), &BoardScope::default())
                        .into_iter()
                        .map(|card| card.id)
                        .collect()
                }),
                each: Arc::new(move |instance_id: &str| {
                    transform(json_as(json!({ "instanceId": instance_id, "defId": SHEEP_TOKEN, "radiant": radiant })))
                }),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: golly_bob_howdy(false),
        radiant: golly_bob_howdy(true),
    }
}

// M #23 Golly Bob Howdy — SPEC §8.8 row 23, BUILD M10 row M 23: "Cry (played, R1; no shipped effect casts a
// Unit): transforms the Units next to it, on its own side and row (§3.1), into base Sheep Tokens in place
// (R691), the Radiant face into Radiant ones; the old cards cease to exist (R11); an Immutable neighbour
// stays (R23); lane 1 has one neighbour; the enemy's Units are never touched; a Sheep is worth 2 Tributes, a
// Radiant one 3 (§3.2, §7)".
//
// Fixtures: #8 Mr. Vanilla (4/4) and #4 Gary the Gambler (1/1) are the neighbours, #66 The Rock (Radiant,
// Immutable) the one that stays, and #10 Rapid Replenish the spare card that keeps the turn open (R82). The
// harness places Units rather than playing them, so none of their Cries fire (R1).
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008";
    const GARY: &str = "core-004";
    const ROCK: &str = "core-066";
    const FILLER: &str = "core-010";
    const SHEEP: &str = "core-t-sheep";

    /// p1 holds Golly Bob and a spare card, with `field` on its side and a Vanilla and a Gary on p2's.
    fn board(radiant: bool, field: Value) -> Scenario {
        crate::scenario(json!({
            "p1": {
                "field": field,
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": [VANILLA, VANILLA],
            },
            "p2": { "field": [VANILLA, { "def": GARY, "lane": 3 }], "hand": [FILLER], "library": [VANILLA, VANILLA] },
        }))
    }

    /// Vanilla in lane 1 (damaged) and Gary in lane 3, with lane 2 free for Golly Bob.
    fn around_lane_2() -> Value {
        json!([{ "def": VANILLA, "lane": 1, "damage": 2 }, { "def": GARY, "lane": 3 }])
    }

    fn sheep_in(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        match s.unit(player, lane) {
            Some(unit) if unit.def_id == SHEEP => unit,
            other => panic!("lane {lane} of {player:?} holds {:?}, not a Sheep", other.map(|unit| unit.def_id)),
        }
    }

    #[test]
    fn is_a_1_cost_3_5_human() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(1));
        assert_eq!(def.type_, CardType::Unit);
        assert_eq!(def.rarity, Rarity::Common);
        assert_eq!(js(&def.tags), json!(["Human"]));
        assert_eq!(
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
            [Some(3), Some(5), Some(6), Some(10)]
        );
        assert!(def.params.is_none());
        assert_eq!(js(&def.refs), json!([SHEEP]));
        let scripts = super::script();
        assert!(scripts.base.cry.is_some());
        assert!(scripts.radiant.cry.is_some());
    }

    mod base {
        use super::*;

        #[test]
        fn cry_turns_the_units_next_to_it_into_base_sheep_in_place() {
            let mut s = board(false, around_lane_2());
            s.play(ID, json!({ "zone": 2 }));
            s.expect_stats(ID, json!({ "attack": 3, "health": 5, "maxHealth": 5 }));
            for lane in [1, 3] {
                let sheep = sheep_in(&s, P1, lane);
                assert!(!sheep.radiant);
                s.expect_stats(&sheep, json!({ "attack": 1, "health": 1, "maxHealth": 1 }));
            }
            // In place, with Golly Bob between them, and a Transform for each.
            assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id), Some(ID.to_string()));
            let transformed = s
                .last_events()
                .iter()
                .filter(|event| event.event_type() == GameEventType::Transformed)
                .count();
            assert_eq!(transformed, 2);
        }

        #[test]
        fn r11_the_old_cards_cease_to_exist() {
            let mut s = board(false, around_lane_2());
            let old: Vec<CardInstance> = [1, 3].iter().filter_map(|lane| s.unit(P1, *lane)).collect();
            assert_eq!(old.len(), 2);
            s.play(ID, json!({ "zone": 2 }));
            for card in &old {
                s.expect_in_zone(card, "gone");
            }
            assert!(s.pile(P1, "graveyard").is_empty());
            assert!(s.pile(P1, "exile").is_empty());
        }

        #[test]
        fn r23_an_immutable_neighbour_stays() {
            let field = json!([{ "def": ROCK, "radiant": true, "lane": 1 }, { "def": GARY, "lane": 3 }]);
            let mut s = board(false, field);
            let rock = s.card(ROCK).clone();
            s.play(ID, json!({ "zone": 2 }));
            assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(rock.id));
            sheep_in(&s, P1, 3);
        }

        #[test]
        fn lane_1_has_one_neighbour() {
            let field = json!([{ "def": VANILLA, "lane": 2 }, { "def": GARY, "lane": 3 }]);
            let mut s = board(false, field);
            s.play(ID, json!({ "zone": 1 }));
            sheep_in(&s, P1, 2);
            // Lane 3 is not next to lane 1.
            assert_eq!(s.unit(P1, 3).map(|unit| unit.def_id), Some(GARY.to_string()));
        }

        #[test]
        fn s3_1_enemy_units_are_never_touched() {
            let mut s = board(false, around_lane_2());
            // p2's Vanilla and Gary stand in lanes 1 and 3, as p1's neighbours do.
            let theirs: Vec<String> = [1, 3].iter().filter_map(|lane| s.unit(P2, *lane)).map(|unit| unit.id).collect();
            s.play(ID, json!({ "zone": 2 }));
            let after: Vec<String> = [1, 3].iter().filter_map(|lane| s.unit(P2, *lane)).map(|unit| unit.id).collect();
            assert_eq!(after, theirs);
            assert_eq!(s.unit(P2, 1).map(|unit| unit.def_id), Some(VANILLA.to_string()));
        }

        #[test]
        fn a_sheep_is_worth_2_tributes() {
            let mut s = board(false, around_lane_2());
            s.play(ID, json!({ "zone": 2 }));
            let sheep = sheep_in(&s, P1, 1);
            assert_eq!(jackioh_engine::play_choices::tribute_value_of(s.state(), &sheep), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_6_10_and_its_sheep_are_radiant_worth_3() {
            let mut s = board(true, around_lane_2());
            s.play(ID, json!({ "zone": 2 }));
            s.expect_stats(ID, json!({ "attack": 6, "health": 10, "maxHealth": 10 }));
            for lane in [1, 3] {
                let sheep = sheep_in(&s, P1, lane);
                assert!(sheep.radiant);
                s.expect_stats(&sheep, json!({ "attack": 2, "health": 2, "maxHealth": 2 }));
                assert_eq!(jackioh_engine::play_choices::tribute_value_of(s.state(), &sheep), 3);
            }
        }
    }
}
