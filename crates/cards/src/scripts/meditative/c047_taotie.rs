//! M #47 饕餮 (SPEC §8.8 row 47): (4) Unit, CN, Legendary, 8/8 → 16/16.
//!
//! Base:    "End of turn: Fuse a random enemy permanent into this."
//! Radiant: "End of turn: Fuse a random enemy permanent and a random card from your opponent's deck
//!           into this."
//! Engine (R62): `end_of_turn` with this as the kept target and one ingredient drawn inside `apply`
//! from the match rng (ME-FUSE-RANDOM, MD-C20): the base face draws from the enemy permanents on
//! the field (tops of piles, both rows, face-down cards included, never an Immutable one); the
//! Radiant face then draws a second ingredient from the opponent's library. The fusion is R77 and
//! R102's: this keeps its instance, zone, damage and Radiant flag; stats are summed; keywords, tags
//! and texts joined; the eaten card ceases to exist with no Death. With no enemy permanent, nothing
//! is fused; an Immutable 饕餮 eats nothing (R23).

use jackioh_engine::effects::fuse_cards;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-047";

fn taotie(radiant: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            let Some(me) = ctx.live_self() else {
                return vec![];
            };
            let id = me.id.clone();
            let mut fusions = vec![fuse_cards(json_as(json!({
                "targetInstanceId": id,
                "randomIngredient": { "side": "enemy", "zones": ["field"] },
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
        base: taotie(false),
        radiant: taotie(true),
    }
}

// M #47 饕餮 — SPEC §8.8 row 47, BUILD M10 row M 47: "At your end of turn only, one random enemy
// permanent (tops of piles, both rows, face-down ones included, never an Immutable one) is fused
// into it (MD-C20) … with no enemy permanent, nothing; an Immutable 饕餮 eats nothing (R23); the
// fused definition survives a JSON round trip (R468); radiant 16/16 and also a random non-Immutable
// card of the opponent's deck".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TAOTIE: &str = "meditative-047";
    const BODY: &str = "core-012"; // Duplicating Felinors 3/4, the meal.
    const FILLER: &str = "core-005";
    const MROW: &str = "core-086"; // "Miss" Mrow 1/1 Felinor, Can't attack, a Death.
    const D_FENDER: &str = "core-001"; // Big D-fender 0/7: "Aura: … Defense Position … +2 Armor."
    const SHREDDER: &str = "core-013"; // Jlockeed Shredder-10: "End of turn: Deal 2 damage …".
    const BIGOT: &str = "core-002"; // Bigot: "Cry: Destroy an enemy non-Human Unit."
    const SAINTESS: &str = "core-081"; // Radiant Saintess 2/2: "Death: Make your other Units Radiant."
    const SHEEPISH: &str = "core-041"; // a Trap, set face-down.
    const BIG_FELINOR: &str = "core-043"; // the bottom of a Stack pile.
    const FIENDER: &str = "core-092"; // Felinor Fiender, Stack, the pile's top.
    const WALL: &str = "meditative-097-7"; // The Great Wall, printed Immutable.
    const ECLIPSE: &str = "core-035"; // Lunar Eclipse: deal 3 damage.

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

        mod base {
            use super::*;

            #[test]
            fn r1020_at_your_end_of_turn_a_random_enemy_permanent_is_fused_into_this() {
                let mut s = scenario(json!({
                    "seed": "taotie-base",
                    "p1": { "field": [{ "def": TAOTIE, "lane": 1 }], "library": [FILLER] },
                    "p2": { "field": [{ "def": BODY, "lane": 1 }], "library": [FILLER] },
                }));
                let before = s.unit(P1, 1).expect("taotie");

                s.end_turn();

                // The meal is gone with no Death, and 饕餮 summed its stats (R77): 8 + 3 attack.
                assert!(s.unit(P2, 1).is_none());
                let after = s.unit(P1, 1).expect("taotie");
                assert_eq!(after.id, before.id);
                s.expect_stats(&after, json!({ "attack": 11, "health": 12 }));
            }

            #[test]
            fn r1020_with_no_enemy_permanent_nothing_is_fused() {
                let mut s = scenario(json!({
                    "seed": "taotie-none",
                    "p1": { "field": [{ "def": TAOTIE, "lane": 1 }], "library": [FILLER] },
                    "p2": { "library": [FILLER] },
                }));

                s.end_turn();

                let after = s.unit(P1, 1).expect("taotie");
                s.expect_stats(&after, json!({ "attack": 8, "health": 8 }));
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

                // p2's turn ended, not p1's: nothing eaten.
                assert_eq!(s.state().active, P1);
                assert!(!fused(&s));
                assert_eq!(s.unit(P2, 1).map(|unit| unit.def_id), Some(BODY.to_string()));
                let taotie = s.unit(P1, 1).expect("taotie");
                s.expect_stats(&taotie, json!({ "attack": 8, "health": 8 }));
            }

            #[test]
            fn r1020_a_face_down_backrow_card_is_a_permanent_it_can_eat() {
                let mut s = table(
                    "taotie-face-down",
                    json!([]),
                    json!([{ "def": SHEEPISH, "row": "backrow", "faceUp": false }]),
                );
                let trap = s.backrow(P2, 1).expect("the face-down trap");

                s.end_turn();

                assert!(fused(&s));
                assert!(s.backrow(P2, 1).is_none());
                s.expect_in_zone(trap.id.as_str(), "gone");
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

                // The Fiender on top went in; the Big Felinor under it stays in its lane.
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

                // The only enemy permanent is Immutable: nothing to eat.
                assert!(!fused(&s));
                assert_eq!(s.unit(P2, 1).map(|unit| unit.def_id), Some(WALL.to_string()));
                let taotie = s.unit(P1, 1).expect("taotie");
                s.expect_stats(&taotie, json!({ "attack": 8, "health": 8 }));
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
            fn r1020_keywords_tags_and_text_joined_damage_kept_cost_4_and_no_death() {
                let mut s = table("taotie-joined", json!([]), json!([{ "def": MROW, "lane": 1 }]));
                let id = s.unit(P1, 1).expect("taotie").id;
                s.card_mut(id.as_str()).damage = 2;
                let mrow = s.unit(P2, 1).expect("mrow");

                s.end_turn();

                let after = s.card(id.as_str()).clone();
                let def = fused_def(&s, &after);
                // Stats summed (8 + 1 / 8 + 1), the 2 damage kept.
                s.expect_stats(&after, json!({ "attack": 9, "maxHealth": 9, "health": 7 }));
                assert_eq!(after.damage, 2);
                // Keywords, tags and text joined; the cost 饕餮's (4).
                let keywords = serde_json::to_string(&s.stats(&after).keywords).expect("JSON");
                assert!(keywords.contains("Can't attack"), "{keywords}");
                let tags = serde_json::to_value(&def.tags).expect("JSON");
                assert!(tags.as_array().expect("tags").contains(&json!("CN")), "{tags}");
                assert!(
                    tags.as_array().expect("tags").contains(&json!("Felinor")),
                    "{tags}"
                );
                assert!(
                    def.base.text.contains("Fuse a random enemy permanent"),
                    "{}",
                    def.base.text
                );
                assert!(def.base.text.contains("Steal the Unit"), "{}", def.base.text);
                assert_eq!(def.cost, CardCost::Fixed(4));
                // The meal ceased to exist with no Death: never destroyed, never in a graveyard.
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
                    json!([{ "def": BIGOT, "lane": 1 }]),
                );
                let felinor = s.unit(P1, 2).expect("p1's non-Human Unit");

                s.end_turn();

                // Bigot's Cry would destroy p1's non-Human Felinors; a fusion fires no Cry.
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

                s.end_turn(); // 饕餮 eats the Saintess: 10/10.
                assert!(!s.card(&felinor).radiant);
                s.card_mut(id.as_str()).damage = 7;
                s.play(
                    ECLIPSE,
                    json!({ "targets": [{ "pick": "instance", "instanceId": id }] }),
                );

                // 饕餮 died, and the Saintess's Death it carried made p1's other Unit Radiant.
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
                    serde_json::from_value(serde_json::to_value(s.state()).expect("state is JSON"))
                        .expect("JSON is a state");
                let same = find_instance(&round_tripped, &after.id).expect("the fused card");
                // The fusion's transient definition travelled with it: still one fused card with
                // the summed stats, not two cards and not the printed one.
                assert_eq!(same.def_id, after.def_id);
                assert_ne!(same.def_id, TAOTIE);
                let view = unit_view(&round_tripped, same);
                assert_eq!(view.attack, 11);
                assert!(s.unit(P2, 1).is_none());
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r1020_radiant_is_16_16() {
                let mut s = scenario(json!({
                    "seed": "taotie-radiant-stats",
                    "p1": { "field": [{ "def": TAOTIE, "radiant": true, "lane": 1 }], "hand": [FILLER], "library": [FILLER] },
                    "p2": { "hand": [FILLER], "library": [FILLER] },
                }));
                let taotie = s.unit(P1, 1).expect("taotie");
                s.expect_stats(&taotie, json!({ "attack": 16, "health": 16 }));
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
                s.expect_stats(&taotie, json!({ "attack": 22, "health": 24 }));
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
                // the fusion's radiant form, the sum of the radiant forms: 16 + 6 + 6 attack and
                // 16 + 8 + 8 health (a Radiant Duplicating Felinors is 6/8 since patch v0.3.10).
                assert!(after.radiant);
                s.expect_stats(&after, json!({ "attack": 28, "health": 32 }));
            }
        }
    }
}
