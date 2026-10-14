//! C+ #24 Crushing Walls (SPEC §8.7 row 24): destroy the top card of every zone in lanes 1 and 5, both
//! rows (Radiant: the enemy's only); a Unit standing on an Ivory Tower while its play resolves is passed
//! by (R446, R653), and the Tower is destroyed whatever it has fused (R418).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-024";

/// Each player's outermost lanes (§3.1).
const WALL_LANES: [i32; 2] = [1, 5];

/// TS `walls(side: "any" | "enemy")`: `side` is the scope's side, `ScopeSide::Any` or `ScopeSide::Enemy`.
fn walls(side: ScopeSide) -> Script {
    let in_the_walls = move |ctx: &mut EffectContext<'_>| -> Vec<String> {
        cards_in_scope(ctx, &json_as(json!({ "side": side, "rows": ["units", "backrow"] })))
            .into_iter()
            .filter_map(|card| {
                if is_carried(&*ctx.state, &card) {
                    return None;
                }
                let at = slot_of(&*ctx.state, &card)?;
                if WALL_LANES.contains(&at.lane) { Some(card.id.clone()) } else { None }
            })
            .collect()
    };
    Script {
        cry: Some(hook(move |_ctx| {
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(in_the_walls),
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
        base: walls(ScopeSide::Any),
        radiant: walls(ScopeSide::Enemy),
    }
}

// C+ #24 Crushing Walls — SPEC §8.7 row 24, BUILD M9 Classic+ row C+ 24: "Destroys the top card of each of
// the eight zones in lanes 1 and 5 (each side numbers its lanes from its owner's seat), Units and
// backrow cards, face-down ones included, an Indestructible one staying; a card dormant beneath
// resumes; a destroyed backrow card that prints Death fires it (§4.5); lanes 2 to 4 are untouched; an
// Ivory Tower is destroyed like any backrow card, whatever it has fused (R418); radiant only the enemy's
// four zones".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const WALLS: &str = "classicplus-024";
    const BODY: &str = "core-008"; // 4/4
    const ROCK: &str = "core-066"; // Indestructible
    const FIELD_SPELL: &str = "core-064";
    const TRAP: &str = "core-060";
    const FIENDER: &str = "core-092"; // Stack
    const TOP: &str = "classicplus-019-1"; // Radiant: Immune to Spells
    const FROST: &str = "classicplus-012-8"; // Field Spell, Animated on your turn
    const TOWER: &str = "classicplus-033"; // Ivory Tower: the first Unit stacked onto it is fused into it
    const FILLER: &str = "core-005";

    fn walls(radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": {
                "hand": [{ "def": WALLS, "radiant": radiant_face }, FILLER],
                "field": [{ "def": BODY, "lane": 1 }, { "def": BODY, "lane": 3 }, { "def": BODY, "lane": 5 }],
                "backrow": [{ "def": FIELD_SPELL, "lane": 1 }, { "def": TRAP, "lane": 5, "faceUp": false }, { "def": FIELD_SPELL, "lane": 2 }],
            },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": BODY, "lane": 1 }, { "def": BODY, "lane": 2 }, { "def": BODY, "lane": 5 }],
                "backrow": [{ "def": TRAP, "lane": 1, "faceUp": false }, { "def": FIELD_SPELL, "lane": 5 }, { "def": TRAP, "lane": 4, "faceUp": false }],
            },
        }))
    }

    /// `{ units, backrow }`: which zones of each row hold a card.
    #[derive(Debug, PartialEq, Eq)]
    struct Occupied {
        units: Vec<bool>,
        backrow: Vec<bool>,
    }

    fn occupied(s: &Scenario, player: PlayerId) -> Occupied {
        Occupied {
            units: (1..=5).map(|lane| s.unit(player, lane).is_some()).collect(),
            backrow: (1..=5).map(|lane| s.backrow(player, lane).is_some()).collect(),
        }
    }

    fn rows(units: [bool; 5], backrow: [bool; 5]) -> Occupied {
        Occupied {
            units: units.to_vec(),
            backrow: backrow.to_vec(),
        }
    }

    use crate::unit_or_blank;

    mod c_n24_crushing_walls {
        use super::*;

        #[test]
        fn is_a_3_spell_with_no_play_time_choice() {
            let def = crate::card_def(ID);
            assert_eq!(def.cost, CardCost::Fixed(3));
            let scripts = script();
            assert!(scripts.base.targets.is_empty());
            assert!(scripts.radiant.targets.is_empty());
        }

        mod base {
            use super::*;

            #[test]
            fn s3_1_destroys_the_top_card_of_all_eight_zones_in_lanes_1_and_5_face_down_ones_included_lanes_2_to_4_untouched(
            ) {
                let mut s = walls(false);
                s.play(WALLS, json!({}));
                assert_eq!(
                    occupied(&s, P1),
                    rows([false, false, true, false, false], [false, true, false, false, false])
                );
                assert_eq!(
                    occupied(&s, P2),
                    rows([false, true, false, false, false], [false, false, false, true, false])
                );
            }

            #[test]
            fn r59_all_eight_cards_are_destroyed_at_once_in_one_pass() {
                let mut s = walls(false);
                s.play(WALLS, json!({}));
                let types: Vec<GameEventType> = s.last_events().iter().map(|event| event.event_type()).collect();
                assert_eq!(types.iter().filter(|each| **each == GameEventType::Destroyed).count(), 8);
                // One pass: the eight deaths are reported together, before anything else happens.
                let first = types.iter().position(|each| *each == GameEventType::Destroyed).expect("a death");
                assert_eq!(
                    types.get(first..first + 8).map(<[GameEventType]>::to_vec),
                    Some(vec![GameEventType::Destroyed; 8])
                );
            }

            #[test]
            fn s4_5_a_destroyed_backrow_card_that_prints_death_fires_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WALLS, FILLER], "library": [FILLER, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                // A test-only Field Spell whose Death draws its controller a card, so the Death shows (the real
                // backrow Deaths, C+ #12.8's copies and C+ #61's Stockpiles, need a history or another set's card).
                let id = "test-backrow-death";
                let face = json!({ "keywords": [], "text": "Death: Draw 1." });
                let def: CardDef = json_as(json!({
                    "id": id, "index": id, "name": id, "set": "Core", "type": "Field Spell", "tags": [],
                    "rarity": "Common", "token": false, "cost": 0, "base": face, "radiant": face,
                }));
                s.state_mut().transient_defs.insert(id.to_string(), def);
                let dying = || Script {
                    death: Some(hook(|_ctx| vec![jackioh_engine::effects::draw(json_as(json!({ "count": 1 })))])),
                    ..Script::default()
                };
                let mut scripts = registered_scripts();
                scripts.insert(id.to_string(), CardScripts { base: dying(), radiant: dying() });
                register_scripts(scripts);
                let mut card = new_instance(s.state_mut(), id, P1, Zone::Hand { player: P1 });
                if !place_on_field(
                    s.state_mut(),
                    &mut card,
                    ZoneRef { player: P1, row: Row::Backrow, lane: 5 },
                    PlaceOnFieldOptions::default(),
                ) {
                    panic!("setup");
                }
                let hand_before = s.hand(P1).len();
                s.play(WALLS, json!({}));
                s.expect_in_zone(&card, "graveyard");
                // Walls left the hand, the Death drew one.
                assert_eq!(s.hand(P1).len(), hand_before);
                assert!(s.last_events().iter().any(|event| event.event_type() == GameEventType::Drawn));
            }

            #[test]
            fn r418_an_ivory_tower_is_destroyed_like_any_backrow_card_whatever_it_has_fused_in() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [WALLS, FILLER], "library": [FILLER, FILLER] },
                    "p2": { "hand": [BODY, FILLER], "backrow": [{ "def": TOWER, "lane": 1 }], "library": [FILLER, FILLER] },
                }));
                let tower = s.card(TOWER).id.clone();
                let rider = s.card(BODY).id.clone();
                s.play(&rider, json!({ "zone": 1, "row": "backrow" }));
                // R653: once its play resolved, the body was fused into the Tower.
                s.expect_in_zone(&rider, "gone");
                s.end_turn();
                s.play(WALLS, json!({}));
                s.expect_in_zone(&tower, "graveyard");
            }

            #[test]
            fn r46_an_indestructible_one_stays_knocked_to_attack_position() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WALLS, FILLER] },
                    "p2": { "hand": [FILLER], "field": [{ "def": ROCK, "lane": 5, "position": "DEF" }] },
                }));
                s.play(WALLS, json!({}));
                assert_eq!(s.unit(P2, 5).map(|unit| unit.def_id), Some(ROCK.to_string()));
                assert_eq!(s.stats(unit_or_blank(&s, P2, 5)).position, Position::Atk);
            }

            #[test]
            fn s3_2_a_card_dormant_beneath_resumes() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WALLS, FILLER] },
                    "p2": { "hand": [FILLER], "field": [{ "def": BODY, "lane": 1 }, { "def": FIENDER, "lane": 1, "stack": true }] },
                }));
                let beneath = s.state().players[P2]
                    .units
                    .first()
                    .and_then(|pile| pile.as_ref())
                    .and_then(|pile| pile.get(1))
                    .map(|card| card.id.clone());
                s.play(WALLS, json!({}));
                assert_eq!(s.unit(P2, 1).map(|unit| unit.id), beneath);
            }

            #[test]
            fn s6_1_a_unit_immune_to_spells_is_passed_by() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WALLS, FILLER] },
                    "p2": { "hand": [FILLER], "field": [{ "def": TOP, "lane": 1, "radiant": true }] },
                }));
                s.play(WALLS, json!({}));
                assert_eq!(s.unit(P2, 1).map(|unit| unit.def_id), Some(TOP.to_string()));
            }

            #[test]
            fn r383_an_animated_card_is_hit_where_it_stands_in_its_backrow_zone_on_the_opponent_s_turn() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WALLS, FILLER] },
                    "p2": { "hand": [FILLER], "backrow": [{ "def": FROST, "lane": 5 }] },
                }));
                let frost = s.backrow(P2, 5).map(|card| card.id).unwrap_or_default();
                s.play(WALLS, json!({}));
                // A Field Spell in its backrow zone, not a Unit token: R11 sends it to the graveyard.
                s.expect_in_zone(&frost, "graveyard");
            }

            #[test]
            fn r383_an_animated_card_is_hit_where_it_stands_animated_into_unit_zone_1_from_backrow_lane_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": {
                        "hand": [WALLS, FILLER],
                        "field": [{ "def": BODY, "lane": 2 }],
                        "backrow": [{ "def": FROST, "lane": 2 }],
                        "library": [FILLER, FILLER],
                    },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                let frost = s.backrow(P1, 2).expect("setup");
                s.end_turn();
                assert!(matches!(s.card(&frost).zone, Zone::Field { row: Row::Units, lane: 1, .. }));
                s.play(WALLS, json!({}));
                // Destroyed as the Unit it is there (a token, so R11 takes it out of the game).
                assert!(s.last_events().iter().any(|event| matches!(
                    event,
                    GameEvent::Destroyed { instance_id, .. } if *instance_id == frost.id
                )));
                s.expect_in_zone(&frost, "gone");
                assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id), Some(BODY.to_string()));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn destroys_only_the_enemy_s_four_zones_in_lanes_1_and_5() {
                let mut s = walls(true);
                s.play(WALLS, json!({}));
                assert_eq!(
                    occupied(&s, P1),
                    rows([true, false, true, false, true], [true, true, false, false, true])
                );
                assert_eq!(
                    occupied(&s, P2),
                    rows([false, true, false, false, false], [false, false, false, true, false])
                );
            }
        }
    }
}
