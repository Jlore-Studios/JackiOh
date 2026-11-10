//! M #47 饕餮 (SPEC §8.8 row 47): (4) Unit, CN, Legendary, 0/1 → 0/2 (2/2 → 4/4 through v0.5.11r).
//!
//! Base:    "End of turn: Fuse a random enemy Unit into this."
//! Radiant: "End of turn: Fuse a random enemy Unit and a random card from your opponent's deck into this."
//! Engine (R62): `end_of_turn` with this as the kept target and one ingredient drawn inside `apply`
//! from the match rng (ME-FUSE-RANDOM, MD-C20): the enemy Units on the field (the top of each pile, both
//! the unit row and a Flying Unit standing in the backrow, R1081000; never a Field Spell or a Trap, face-up
//! or face-down, and never an Immutable one), then on the Radiant face the opponent's library. Through
//! v0.5.11r the field ingredient was any enemy permanent, a Field Spell or a Trap too (`script_v0_5_11o`,
//! R767000). The fusion is R77 and R102's: this keeps its instance, zone, damage and Radiant flag; the
//! eaten card ceases to exist with no Death. An Immutable 饕餮 eats nothing (R23). With no enemy Unit
//! nothing is fused from the field.

use jackioh_engine::effects::fuse_cards;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-047";

/// The field ingredient: an enemy Unit, the unit row's (R1081000), never a backrow card.
fn enemy_unit() -> Value {
    json!({ "side": "enemy", "zones": ["field"], "rows": ["units"], "types": ["Unit"] })
}

/// R767000: through v0.5.11r, any enemy permanent: both rows.
fn enemy_permanent() -> Value {
    json!({ "side": "enemy", "zones": ["field"] })
}

fn taotie(radiant: bool, field_ingredient: fn() -> Value) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            let Some(me) = ctx.live_self() else {
                return vec![];
            };
            let id = me.id.clone();
            let mut fusions = vec![fuse_cards(json_as(json!({
                "targetInstanceId": id,
                "randomIngredient": field_ingredient(),
            })))];
            if radiant {
                fusions.push(fuse_cards(json_as(json!({
                    "targetInstanceId": id,
                    "randomIngredient": { "side": "enemy", "zones": ["library"] },
                }))));
            }
            fusions
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: taotie(false, enemy_unit),
        radiant: taotie(true, enemy_unit),
    }
}

/// R767000: through v0.5.11r the field ingredient was any enemy permanent; `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        base: taotie(false, enemy_permanent),
        radiant: taotie(true, enemy_permanent),
    }
}

// M #47 饕餮 — SPEC §8.8 row 47, BUILD M10 row M 47, R468, R767000.
#[cfg(test)]
mod tests {
    use super::script_v0_5_11o;
    use crate::js;
    use jackioh_engine::testkit::*;

    use crate::{scenario, stand_in_backrow};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TAOTIE: &str = "meditative-047";
    const BODY: &str = "core-012"; // Duplicating Felinors: the meal
    const FILLER: &str = "core-005";
    const MROW: &str = "core-086"; // "Miss" Mrow: Felinor, Can't attack, a Death
    const D_FENDER: &str = "core-001"; // Big D-fender: "Aura: … Defense Position … +2 Armor."
    const SHREDDER: &str = "core-013"; // Jlockeed Shredder-10: "End of turn: Deal 2 damage …".
    const SPECIESIST: &str = "core-002"; // Speciesist: "Cry: Destroy an enemy non-Human Unit."
    const SAINTESS: &str = "core-081"; // Radiant Saintess: "Death: Make your other Units Radiant."
    const SHEEPISH: &str = "core-041"; // Sheepish: a Trap, set face-down
    const MANA_WELL: &str = "core-006"; // Mana Well: a Field Spell, a backrow card
    const BIG_FELINOR: &str = "core-043"; // Big Felinor: the bottom of a Stack pile.
    const FIENDER: &str = "core-092"; // Felinor Fiender: Stack, the pile's top
    const WALL: &str = "meditative-097-7"; // The Great Wall: printed Immutable
    const ECLIPSE: &str = "core-035"; // Lunar Eclipse: deal 3 damage.
    const WORMHOLE: &str = "stinkyscape-139"; // Wormhole: counters an opponent's Spell aimed at their own side.
    const BOOK: &str = "classicplus-057"; // Book of Stats: gives a Unit +attack/+health
    const VANILLA: &str = "core-008"; // Mr. Vanilla: no text

    fn meal(n: i32) -> Value {
        json!([{ "def": BODY, "lane": n }])
    }

    /// p1's 饕餮 in lane 1 (plus `mine`), facing `theirs` on p2's board, p1 to move.
    fn table(seed: &str, mine: Value, theirs: Value) -> Scenario {
        let mut field = vec![json!({ "def": TAOTIE, "lane": 1 })];
        field.extend(mine.as_array().cloned().unwrap_or_default());
        scenario(json!({
            "seed": seed,
            "p1": { "field": field, "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
            "p2": {
                "field": theirs,
                "hand": [ECLIPSE, FILLER],
                "library": [FILLER, FILLER, FILLER],
            },
        }))
    }

    fn fused_def(s: &Scenario, card: &CardInstance) -> CardDef {
        def_of(Some(s.state()), &s.card(card.id.as_str()).def_id).clone()
    }

    fn fused(s: &Scenario) -> bool {
        s.events()
            .iter()
            .any(|event| matches!(event, GameEvent::Fused { .. }))
    }

    mod m47_taotie {
        use super::*;

        #[test]
        fn enters_as_a_0_1_and_radiant_a_0_2() {
            for (radiant, health) in [(false, 1), (true, 2)] {
                let mut s = scenario(json!({
                    "seed": "taotie-enters",
                    "p1": { "hand": [{ "def": TAOTIE, "radiant": radiant }, FILLER], "library": [FILLER] },
                    "p2": { "hand": [FILLER], "library": [FILLER] },
                }));
                s.play(TAOTIE, json!({ "zone": 1 }));
                let taotie = s.unit(P1, 1).expect("taotie");
                s.expect_stats(&taotie, json!({ "attack": 0, "health": health, "maxHealth": health }));
            }
        }

        mod base {
            use super::*;

            #[test]
            fn r1020_at_your_end_of_turn_a_random_enemy_unit_is_fused_into_this() {
                let mut s = scenario(json!({
                    "seed": "taotie-base",
                    "p1": { "field": [{ "def": TAOTIE, "lane": 1 }], "library": [FILLER] },
                    "p2": { "field": [{ "def": BODY, "lane": 1 }], "library": [FILLER] },
                }));
                let before = s.unit(P1, 1).expect("taotie");

                s.end_turn();

                // The meal is gone with no Death, and 饕餮 summed its stats (R77): 0 + 3 attack, 1 + 4 health.
                assert!(s.unit(P2, 1).is_none());
                let after = s.unit(P1, 1).expect("taotie");
                assert_eq!(after.id, before.id);
                s.expect_stats(&after, json!({ "attack": 3, "health": 5 }));
            }

            #[test]
            fn r1020_with_no_enemy_unit_nothing_is_fused() {
                let mut s = scenario(json!({
                    "seed": "taotie-none",
                    "p1": { "field": [{ "def": TAOTIE, "lane": 1 }], "library": [FILLER] },
                    "p2": { "library": [FILLER] },
                }));

                s.end_turn();

                let after = s.unit(P1, 1).expect("taotie");
                s.expect_stats(&after, json!({ "attack": 0, "health": 1 }));
            }

            #[test]
            fn r1020_only_at_your_end_of_turn_never_the_opponents() {
                let mut s = scenario(json!({
                    "seed": "taotie-their-end",
                    "active": "p2",
                    "p1": { "field": [{ "def": TAOTIE, "lane": 1 }], "hand": [FILLER], "library": [FILLER] },
                    "p2": { "field": meal(1), "hand": [FILLER], "library": [FILLER] },
                }));

                s.end_turn();

                assert_eq!(s.state().active, P1);
                assert!(!fused(&s));
                assert_eq!(s.unit(P2, 1).map(|unit| unit.def_id), Some(BODY.to_string()));
                let taotie = s.unit(P1, 1).expect("taotie");
                s.expect_stats(&taotie, json!({ "attack": 0, "health": 1 }));
            }

            #[test]
            fn r1020_never_a_face_down_trap_nor_a_field_spell_and_a_board_without_a_unit_is_left_alone() {
                for (seed, backrow) in [
                    ("taotie-face-down", json!([{ "def": SHEEPISH, "row": "backrow", "faceUp": false }])),
                    ("taotie-field-spell", json!([{ "def": MANA_WELL, "row": "backrow", "faceUp": true }])),
                    (
                        "taotie-both",
                        json!([
                            { "def": SHEEPISH, "row": "backrow", "faceUp": false, "lane": 1 },
                            { "def": MANA_WELL, "row": "backrow", "faceUp": true, "lane": 2 },
                        ]),
                    ),
                ] {
                    let mut s = table(seed, json!([]), backrow);
                    let theirs: Vec<String> = (1..=5).filter_map(|lane| s.backrow(P2, lane)).map(|card| card.id).collect();
                    assert!(!theirs.is_empty());

                    s.end_turn();

                    assert!(!fused(&s), "{seed}");
                    for id in &theirs {
                        s.expect_in_zone(id.as_str(), "field");
                    }
                    let taotie = s.unit(P1, 1).expect("taotie");
                    s.expect_stats(&taotie, json!({ "attack": 0, "health": 1 }));
                }
            }

            #[test]
            fn r1020_it_eats_the_unit_whatever_else_stands_beside_it() {
                for n in 0..8 {
                    let mut s = table(
                        &format!("taotie-among-{n}"),
                        json!([]),
                        json!([
                            { "def": BODY, "lane": 1 },
                            { "def": SHEEPISH, "row": "backrow", "faceUp": false, "lane": 1 },
                            { "def": MANA_WELL, "row": "backrow", "faceUp": true, "lane": 2 },
                        ]),
                    );
                    let unit = s.unit(P2, 1).expect("the Unit").id;

                    s.end_turn();

                    s.expect_in_zone(unit.as_str(), "gone");
                    assert!(s.backrow(P2, 1).is_some() && s.backrow(P2, 2).is_some());
                }
            }

            #[test]
            fn r1020_r1081000_a_flying_unit_standing_in_the_backrow_is_a_unit_it_eats() {
                let mut s = table("taotie-flying", json!([]), meal(1));
                let flier = s.unit(P2, 1).expect("the meal").id;
                stand_in_backrow(&mut s, flier.as_str(), 3);
                assert!(s.unit(P2, 1).is_none());

                s.end_turn();

                assert!(fused(&s));
                s.expect_in_zone(flier.as_str(), "gone");
            }

            #[test]
            fn r1020_only_the_top_of_a_pile_is_eaten() {
                let mut s = table(
                    "taotie-pile",
                    json!([]),
                    json!([{ "def": BIG_FELINOR, "lane": 1 }, { "def": FIENDER, "stack": true }]),
                );
                let top = s.unit(P2, 1).expect("the pile's top");
                assert_eq!(top.def_id, FIENDER);

                s.end_turn();

                assert!(fused(&s));
                s.expect_in_zone(top.id.as_str(), "gone");
                assert_eq!(
                    s.unit(P2, 1).map(|unit| unit.def_id),
                    Some(BIG_FELINOR.to_string())
                );
            }

            #[test]
            fn r23_never_an_immutable_enemy_permanent() {
                let mut s = table(
                    "taotie-immutable-meal",
                    json!([]),
                    json!([{ "def": WALL, "lane": 1 }]),
                );

                s.end_turn();

                assert!(!fused(&s));
                assert_eq!(s.unit(P2, 1).map(|unit| unit.def_id), Some(WALL.to_string()));
                let taotie = s.unit(P1, 1).expect("taotie");
                s.expect_stats(&taotie, json!({ "attack": 0, "health": 1 }));
            }

            #[test]
            fn r23_an_immutable_taotie_eats_nothing() {
                let mut s = table("taotie-immutable", json!([]), meal(1));
                let id = s.unit(P1, 1).expect("taotie").id;
                find_instance_mut(s.state_mut(), &id)
                    .expect("taotie")
                    .granted_keywords
                    .push(json_as(json!({ "kind": "Immutable" })));

                s.end_turn();

                assert!(!fused(&s));
                assert_eq!(s.unit(P2, 1).map(|unit| unit.def_id), Some(BODY.to_string()));
                assert_eq!(s.card(id.as_str()).def_id, TAOTIE);
            }

            #[test]
            fn r1020_keywords_tags_and_text_joined_cost_4_and_no_death() {
                let mut s = table("taotie-joined", json!([]), json!([{ "def": MROW, "lane": 1 }]));
                let id = s.unit(P1, 1).expect("taotie").id;
                let mrow = s.unit(P2, 1).expect("mrow");

                s.end_turn();

                let after = s.card(id.as_str()).clone();
                let def = fused_def(&s, &after);
                s.expect_stats(&after, json!({ "attack": 1, "maxHealth": 2, "health": 2 }));
                let keywords = serde_json::to_string(&s.stats(&after).keywords).expect("JSON");
                assert!(keywords.contains("Can't attack"), "{keywords}");
                let tags = js(&def.tags);
                assert!(tags.as_array().expect("tags").contains(&json!("CN")), "{tags}");
                assert!(
                    tags.as_array().expect("tags").contains(&json!("Felinor")),
                    "{tags}"
                );
                assert!(
                    def.base.text.contains("Fuse a random enemy Unit"),
                    "{}",
                    def.base.text
                );
                assert!(def.base.text.contains("Steal the Unit"), "{}", def.base.text);
                assert_eq!(def.cost, CardCost::Fixed(4));
                s.expect_in_zone(mrow.id.as_str(), "gone");
                assert!(!s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::Destroyed { instance_id, .. } if *instance_id == mrow.id
                )));
                assert!(s.pile(P2, "graveyard").is_empty());
            }

            #[test]
            fn r1020_a_fused_aura_covers_your_side() {
                let mut s = table(
                    "taotie-aura",
                    json!([{ "def": BODY, "lane": 2, "position": "DEF" }]),
                    json!([{ "def": D_FENDER, "lane": 1 }]),
                );
                let guard = s.unit(P1, 2).expect("p1's Defense Unit");
                let before = s.stats(&guard).armor;

                s.end_turn();

                // The D-fender's aura now runs on 饕餮, for 饕餮's side: +2 Armor.
                assert!(s.unit(P2, 1).is_none());
                assert_eq!(s.stats(&guard).armor, before + 2);
            }

            #[test]
            fn r1020_a_fused_end_of_turn_line_runs() {
                let mut s = table("taotie-eot", json!([]), json!([{ "def": SHREDDER, "lane": 1 }]));

                s.end_turn(); // p1's end: the Shredder is eaten.
                assert!(s.unit(P2, 1).is_none());
                s.end_turn(); // p2's turn passes.
                let before = s.state().players.p2.hero.health;
                s.end_turn(); // p1's end again: the fused Shredder's line hits the enemy hero.

                s.expect_health(P2, before - 2);
            }

            #[test]
            fn r1020_a_fused_cry_never_runs() {
                let mut s = table(
                    "taotie-cry",
                    json!([{ "def": BODY, "lane": 2 }]),
                    json!([{ "def": SPECIESIST, "lane": 1 }]),
                );
                let felinor = s.unit(P1, 2).expect("p1's non-Human Unit");

                s.end_turn();

                // Speciesist's Cry would destroy p1's non-Human Felinors; a fusion fires no Cry.
                assert!(fused(&s));
                s.expect_in_zone(&felinor, "field");
            }

            #[test]
            fn r1020_a_fused_death_fires_when_it_dies() {
                let mut s = table(
                    "taotie-death",
                    json!([{ "def": BODY, "lane": 2 }]),
                    json!([{ "def": SAINTESS, "lane": 1 }]),
                );
                let id = s.unit(P1, 1).expect("taotie").id;
                let felinor = s.unit(P1, 2).expect("p1's other Unit");

                s.end_turn(); // 饕餮 eats the Saintess: 2/3.
                assert!(!s.card(&felinor).radiant);
                s.card_mut(id.as_str()).damage = 1;
                s.play(
                    ECLIPSE,
                    json!({ "targets": [{ "pick": "instance", "instanceId": id }] }),
                );

                s.expect_in_zone(id.as_str(), "graveyard");
                assert!(s.card(&felinor).radiant);
            }

            #[test]
            fn r468_the_fused_definition_survives_json() {
                let mut s = scenario(json!({
                    "seed": "taotie-json",
                    "p1": { "field": [{ "def": TAOTIE, "lane": 1 }], "library": [FILLER] },
                    "p2": { "field": meal(1), "library": [FILLER] },
                }));

                s.end_turn();

                let after = s.unit(P1, 1).expect("taotie");
                let round_tripped: GameState =
                    serde_json::from_value(js(s.state())).expect("JSON is a state");
                let same = find_instance(&round_tripped, &after.id).expect("the fused card");
                assert_eq!(same.def_id, after.def_id);
                assert_ne!(same.def_id, TAOTIE);
                let view = unit_view(&round_tripped, same);
                assert_eq!(view.attack, 3);
                assert!(s.unit(P2, 1).is_none());
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r1020_radiant_is_0_2() {
                let mut s = scenario(json!({
                    "seed": "taotie-radiant-stats",
                    "p1": { "field": [{ "def": TAOTIE, "radiant": true, "lane": 1 }], "hand": [FILLER], "library": [FILLER] },
                    "p2": { "hand": [FILLER], "library": [FILLER] },
                }));
                let taotie = s.unit(P1, 1).expect("taotie");
                s.expect_stats(&taotie, json!({ "attack": 0, "health": 2 }));
            }

            #[test]
            fn r23_radiant_never_eats_an_immutable_card_of_the_deck() {
                let mut s = scenario(json!({
                    "seed": "taotie-radiant-immutable",
                    "p1": { "field": [{ "def": TAOTIE, "radiant": true, "lane": 1 }], "hand": [FILLER], "library": [FILLER] },
                    "p2": { "field": meal(1), "hand": [FILLER], "library": [WALL, WALL, WALL] },
                }));
                let taotie = s.unit(P1, 1).expect("taotie");

                s.end_turn();

                // The field's meal went in; every deck card is Immutable, so the deck kept all three
                // (one of them drawn as p2's turn began).
                assert!(s.unit(P2, 1).is_none());
                let walls = s.pile(P2, "library").len()
                    + s.hand(P2).iter().filter(|card| card.def_id == WALL).count();
                assert_eq!(walls, 3);
                s.expect_stats(&taotie, json!({ "attack": 6, "health": 10 }));
            }

            #[test]
            fn r1020_radiant_also_fuses_a_random_card_of_the_opponents_deck() {
                let mut s = scenario(json!({
                    "seed": "taotie-radiant",
                    "p1": { "field": [{ "def": TAOTIE, "radiant": true, "lane": 1 }], "library": [FILLER] },
                    "p2": { "field": meal(1), "library": [BODY, BODY, BODY] },
                }));
                let before = s.unit(P1, 1).expect("taotie");

                s.end_turn();

                // Both meals went in: the field's and one of the deck's (the turn's draw took
                // the third).
                assert!(s.unit(P2, 1).is_none());
                assert_eq!(s.pile(P2, "library").len(), 1);
                let after = s.unit(P1, 1).expect("taotie");
                assert_eq!(after.id, before.id);
                // Both fusions summed their stats (R77), and the kept card is Radiant, so it shows
                // the fusion's radiant form, the sum of the radiant forms: 0 + 6 + 6 attack and
                // 2 + 8 + 8 health (a Radiant Duplicating Felinors is 6/8).
                assert!(after.radiant);
                s.expect_stats(&after, json!({ "attack": 12, "health": 18 }));
            }
            #[test]
            fn r1020_radiant_never_eats_a_trap_or_a_field_spell_either_but_still_eats_the_deck() {
                let mut s = scenario(json!({
                    "seed": "taotie-radiant-no-backrow",
                    "p1": { "field": [{ "def": TAOTIE, "radiant": true, "lane": 1 }], "hand": [FILLER], "library": [FILLER] },
                    "p2": {
                        "backrow": [
                            { "def": SHEEPISH, "faceUp": false, "lane": 1 },
                            { "def": MANA_WELL, "faceUp": true, "lane": 2 },
                        ],
                        "hand": [FILLER],
                        "library": [BODY, BODY, BODY],
                    },
                }));
                let theirs: Vec<String> = (1..=5).filter_map(|lane| s.backrow(P2, lane)).map(|card| card.id).collect();
                assert_eq!(theirs.len(), 2);

                s.end_turn();

                for id in &theirs {
                    s.expect_in_zone(id.as_str(), "field");
                }
                // The deck's card went in alone: 0 + 6 attack, 2 + 8 health.
                let after = s.unit(P1, 1).expect("taotie");
                s.expect_stats(&after, json!({ "attack": 6, "health": 10 }));
            }

            #[test]
            fn r102_the_fusion_keeps_its_damage() {
                let mut s = scenario(json!({
                    "seed": "taotie-damage",
                    "p1": { "field": [{ "def": TAOTIE, "radiant": true, "damage": 1, "lane": 1 }], "hand": [FILLER], "library": [FILLER] },
                    "p2": { "field": [{ "def": MROW, "lane": 1 }], "hand": [FILLER], "library": [] },
                }));
                let id = s.unit(P1, 1).expect("taotie").id;

                s.end_turn();

                let after = s.card(id.as_str()).clone();
                assert_eq!(after.damage, 1);
                assert!(s.stats(&after).health >= 1);
            }
        }

        /// R767000: the script kept for the replays of v0.5.11r and before, which ate any enemy permanent.
        mod v0_5_11o {
            use super::*;

            #[test]
            fn r767000_r1020_a_game_on_v0_5_11o_eats_a_face_down_trap_and_a_field_spell() {
                for backrow in [
                    json!([{ "def": SHEEPISH, "row": "backrow", "faceUp": false }]),
                    json!([{ "def": MANA_WELL, "row": "backrow", "faceUp": true }]),
                ] {
                    crate::under_kept(TAOTIE, script_v0_5_11o(), || {
                        let mut s = table("taotie-kept-backrow", json!([]), backrow.clone());
                        let card = s.backrow(P2, 1).expect("the backrow card");

                        s.end_turn();

                        assert!(fused(&s));
                        assert!(s.backrow(P2, 1).is_none());
                        s.expect_in_zone(card.id.as_str(), "gone");
                    });
                }
            }

            #[test]
            fn r767000_r1020_r2226000_a_game_on_v0_5_11o_eats_a_trap_whose_reveal_never_fires_from_its_unit_zone() {
                crate::under_kept(TAOTIE, script_v0_5_11o(), || {
                    let mut s = scenario(json!({
                        "seed": "taotie-trap",
                        "p1": {
                            "field": [{ "def": TAOTIE, "lane": 1 }],
                            "library": [FILLER, FILLER, FILLER],
                        },
                        "p2": {
                            "backrow": [{ "def": WORMHOLE, "lane": 1 }],
                            "hand": [VANILLA, BOOK],
                            "library": [FILLER, FILLER, FILLER],
                        },
                    }));

                    s.end_turn(); // p1's end: 饕餮 eats the Wormhole, p2's only permanent.
                    assert!(fused(&s));
                    s.play(VANILLA, json!({ "zone": 1 }));
                    let vanilla = s.unit(P2, 1).expect("p2's Vanilla");
                    s.play(
                        BOOK,
                        json!({ "targets": [{ "pick": "instance", "instanceId": vanilla.id }] }),
                    );

                    // The eaten Wormhole's text never answers from 饕餮's unit zone, not even p2's Spell aimed
                    // at its own Unit, which its condition admits (R2271000): no counter, no prompt, and the
                    // Book's +5/+5 lands on p2's 4/4.
                    assert!(s.state().pending.is_none());
                    assert!(!s
                        .events()
                        .iter()
                        .any(|event| matches!(event, GameEvent::Countered { .. })));
                    s.expect_stats(&vanilla, json!({ "attack": 9, "health": 9 }));
                });
            }
        }
    }
}
