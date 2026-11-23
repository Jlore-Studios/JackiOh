//! M #47 饕餮 (SPEC §8.8 row 47): (4) Unit, CN, Legendary, 2/2 → 4/4 (0/1 → 0/2 from v0.5.12 through
//! v0.6.5b).
//!
//! Base:    "Activate: Fuse a random enemy Unit into this, then Freeze this. Has an extra turn cooldown."
//! Radiant: "Activate: Fuse a random enemy Unit into this."
//! Engine (R3996002): an Activate, once a turn with no cost (R384), offered while 饕餮 is not Immutable
//! and an enemy Unit stands that is not Immutable, with this as the kept target and one ingredient drawn
//! inside `apply` from the match rng (ME-FUSE-RANDOM, MD-C20): the enemy Units on the field (the top of
//! each pile, both the unit row and a Flying Unit standing in the backrow, R1081000; never a Field Spell
//! or a Trap, face-up or face-down, and never an Immutable one, R1020). The base face then Freezes 饕餮
//! (§6.3 Freeze), which costs it its attack this turn, or its next turn and that turn's Activate once it
//! has attacked (R2246000). Since issue #4154 the base face's Activate also has an extra turn cooldown, as
//! the Beams' (R1841000): a use sits out its user's next turn, on top of the Freeze; the Radiant face's
//! has none. The fusion is R77 and R102's: this keeps its instance, zone, damage, Radiant
//! flag and this turn's use; the eaten card ceases to exist with no Death. Through v0.6.5b it was "End of
//! turn: Fuse a random enemy Unit into this." (R62), the Radiant face then also fusing a random card of
//! the opponent's library (`script_v0_6_5b`), through v0.6.6h the base face's Activate had no cooldown
//! (`script_v0_6_6h`), and through v0.5.11r the field ingredient was any enemy
//! permanent, a Field Spell or a Trap too (`script_v0_5_11o`), both kept for their replays (R767000).

use jackioh_engine::effects::fuse_cards;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-047";

/// The Activate's id.
const DEVOUR: &str = "devour";
/// "Activate": once a turn (R384).
const USES: i32 = 1;

/// The field ingredient: an enemy Unit, the unit row's (R1081000), never a backrow card.
fn enemy_unit() -> Value {
    json!({ "side": "enemy", "zones": ["field"], "rows": ["units"], "types": ["Unit"] })
}

/// R767000: through v0.5.11r, any enemy permanent: both rows.
fn enemy_permanent() -> Value {
    json!({ "side": "enemy", "zones": ["field"] })
}

/// R3996002, R23: whether there is a meal: 饕餮 is not Immutable and an enemy Unit stands that is not.
fn has_meal(state: &GameState, me: &CardInstance, controller: PlayerId) -> bool {
    !is_immutable(state, me)
        && active_units_of(state, opponent_of(controller))
            .into_iter()
            .any(|unit| !is_immutable(state, unit))
}

/// R3996002: "Activate: Fuse a random enemy Unit into this[, then Freeze this][. Has an extra turn
/// cooldown]." (`cooldown`, R1841000, the base face's since issue #4154).
fn devour(freeze_self: bool, cooldown: bool) -> ActivationDecl {
    let label = if freeze_self {
        "Fuse a random enemy Unit into this, then Freeze this"
    } else {
        "Fuse a random enemy Unit into this"
    };
    ActivationDecl {
        id: DEVOUR.to_string(),
        label: label.to_string(),
        uses: ActivationUses::Count(USES),
        cost: cooldown.then(|| ActivationCost {
            extra_turn_cooldown: Some(true),
            ..ActivationCost::default()
        }),
        targets: vec![],
        modes: vec![],
        can_activate: Some(condition_hook(|c| has_meal(c.state, c.self_, c.controller))),
        has: None,
        run: hook(move |ctx| {
            let Some(me) = ctx.live_self() else {
                return vec![];
            };
            let id = me.id.clone();
            let mut effects = vec![fuse_cards(json_as(json!({
                "targetInstanceId": id,
                "randomIngredient": enemy_unit(),
            })))];
            if freeze_self {
                effects.push(freeze(FreezeArgs {
                    target: TargetSpec::Instance { instance_id: id },
                    forever: None,
                }));
            }
            effects
        }),
    }
}

/// R767000: through v0.6.5b, the end of turn's fusion (R62), the Radiant face's second meal from the
/// opponent's library.
fn taotie_end_of_turn(radiant: bool, field_ingredient: fn() -> Value) -> Script {
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

/// Both faces' Activates; `cooldown`: whether the base face's has an extra turn cooldown (issue #4154).
fn devouring(cooldown: bool) -> CardScripts {
    CardScripts {
        base: Script {
            activations: vec![devour(true, cooldown)],
            ..Script::default()
        },
        radiant: Script {
            activations: vec![devour(false, false)],
            ..Script::default()
        },
    }
}

pub fn script() -> CardScripts {
    devouring(true)
}

/// R767000: after v0.6.5b through v0.6.6h the base face's Activate had no extra turn cooldown (issue
/// #4154); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_6h() -> CardScripts {
    devouring(false)
}

/// R767000: through v0.6.5b an end of turn ate a random enemy Unit, the Radiant face a random card of
/// the opponent's library too; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_5b() -> CardScripts {
    CardScripts {
        base: taotie_end_of_turn(false, enemy_unit),
        radiant: taotie_end_of_turn(true, enemy_unit),
    }
}

/// R767000: through v0.5.11r the field ingredient was any enemy permanent; `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        base: taotie_end_of_turn(false, enemy_permanent),
        radiant: taotie_end_of_turn(true, enemy_permanent),
    }
}

// M #47 饕餮 — SPEC §8.8 row 47, BUILD M10 row M 47, R468, R767000, R3996002.
#[cfg(test)]
mod tests {
    use super::{DEVOUR, script_v0_5_11o, script_v0_6_5b, script_v0_6_6h};
    use crate::js;
    use jackioh_engine::subsystems::activate::activate_actions_for;
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
    /// An enemy that strikes back for nothing.
    const HARMLESS: &str = "stand-in (1) 0/30";

    /// 饕餮's printed stats: base 2/2, Radiant 4/4.
    const BASE: (i32, i32) = (2, 2);
    const RADIANT: (i32, i32) = (4, 4);

    fn meal(n: i32) -> Value {
        json!([{ "def": BODY, "lane": n }])
    }

    /// p1's 饕餮 in lane 1 (plus `mine`), facing `theirs` on p2's board, p1 to move.
    fn table(seed: &str, mine: Value, theirs: Value) -> Scenario {
        table_as(seed, false, mine, theirs)
    }

    /// `table` with 饕餮 on the face `radiant`.
    fn table_as(seed: &str, radiant: bool, mine: Value, theirs: Value) -> Scenario {
        let mut field = vec![json!({ "def": TAOTIE, "lane": 1, "radiant": radiant })];
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

    /// p1's 饕餮 in lane 1, by its instance id (a fusion keeps it, R77).
    fn taotie_id(s: &Scenario) -> String {
        s.unit(P1, 1).expect("饕餮").id
    }

    /// 饕餮's Activate.
    fn devour<'a>(s: &'a mut Scenario, id: &str) -> &'a mut Scenario {
        s.activate(id, json!({ "ability": DEVOUR }))
    }

    /// How many Activates 饕餮 is offered.
    fn offers(s: &Scenario, id: &str) -> usize {
        activate_actions_for(s.state(), P1, s.card(id))
            .into_iter()
            .filter(|action| action.ability.as_deref() == Some(DEVOUR))
            .count()
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
        fn enters_as_a_2_2_and_radiant_a_4_4() {
            for (radiant, (attack, health)) in [(false, BASE), (true, RADIANT)] {
                let mut s = scenario(json!({
                    "seed": "taotie-enters",
                    "p1": { "hand": [{ "def": TAOTIE, "radiant": radiant }, FILLER], "library": [FILLER] },
                    "p2": { "hand": [FILLER], "library": [FILLER] },
                }));
                s.play(TAOTIE, json!({ "zone": 1 }));
                let taotie = s.unit(P1, 1).expect("taotie");
                s.expect_stats(&taotie, json!({ "attack": attack, "health": health, "maxHealth": health }));
            }
        }

        mod base {
            use super::*;

            #[test]
            fn r3996002_r1020_its_activate_fuses_a_random_enemy_unit_into_this() {
                let mut s = table("taotie-base", json!([]), meal(1));
                let before = taotie_id(&s);
                assert_eq!(offers(&s, &before), 1);

                devour(&mut s, &before);

                // The meal is gone with no Death, and 饕餮 summed its stats (R77): 2 + 3 attack, 2 + 4 health.
                assert!(s.unit(P2, 1).is_none());
                let after = s.unit(P1, 1).expect("taotie");
                assert_eq!(after.id, before);
                s.expect_stats(&after, json!({ "attack": BASE.0 + 3, "health": BASE.1 + 4 }));
            }

            #[test]
            fn r3996002_then_it_freezes_itself_and_loses_this_turns_attack() {
                let mut s = table("taotie-freeze", json!([]), json!([{ "def": BODY, "lane": 1 }, HARMLESS]));
                let id = taotie_id(&s);

                devour(&mut s, &id);

                assert!(s.card(&id).frozen.is_some(), "frozen");
                let target = (1..=UNIT_ZONES).find_map(|lane| s.unit(P2, lane)).expect("an enemy Unit left").id;
                s.expect_refused_with(|s| s.attack(id.as_str(), target.as_str()), "Frozen");
                assert_eq!(offers(&s, &id), 0);
                s.end_turn().end_turn();
                assert!(s.card(&id).frozen.is_none(), "thawed at the cleanup of the turn it lost the attack in");
            }

            /// R1841000: since issue #4154 the base face's Activate has an extra turn cooldown: thawed on its
            /// next turn, it still sits that turn out, and it is offered again the turn after.
            #[test]
            fn r1841000_its_activate_sits_out_its_next_turn_and_comes_back_the_turn_after() {
                let mut s = table("taotie-cooldown", json!([]), json!([{ "def": BODY, "lane": 1 }, HARMLESS, HARMLESS]));
                let id = taotie_id(&s);

                devour(&mut s, &id);

                s.end_turn().end_turn();
                assert!(s.card(&id).frozen.is_none(), "thawed");
                assert_eq!(offers(&s, &id), 0, "cooling down");
                s.expect_refused_with(|s| devour(s, &id), "cooling down");
                s.end_turn().end_turn();
                assert_eq!(offers(&s, &id), 1, "usable again");
                devour(&mut s, &id);
                let meals = s.events().iter().filter(|event| matches!(event, GameEvent::Fused { .. })).count();
                assert_eq!(meals, 2, "a second meal");
            }

            #[test]
            fn r3996002_r2246000_having_attacked_its_freeze_holds_through_its_next_turn_activate_and_all() {
                let mut s = table("taotie-attacked", json!([]), json!([HARMLESS, HARMLESS]));
                let id = taotie_id(&s);
                let struck = s.unit_at(P2, 1).id;
                s.attack(id.as_str(), struck.as_str());

                devour(&mut s, &id);

                s.end_turn().end_turn();
                assert!(s.card(&id).frozen.is_some(), "still frozen on its next turn");
                assert_eq!(offers(&s, &id), 0);
                s.expect_refused_with(|s| devour(s, &id), "Frozen");
            }

            #[test]
            fn r3996002_its_end_of_turn_eats_nothing_now() {
                let mut s = table("taotie-no-end-of-turn", json!([]), meal(1));
                s.end_turn();
                assert!(!fused(&s));
                assert_eq!(s.unit(P2, 1).map(|unit| unit.def_id), Some(BODY.to_string()));
            }

            #[test]
            fn r3996002_with_no_enemy_unit_it_is_not_offered() {
                let mut s = table("taotie-none", json!([]), json!([]));
                let id = taotie_id(&s);
                assert_eq!(offers(&s, &id), 0);
                s.expect_refused(|s| devour(s, &id));
                assert!(!fused(&s));
                assert!(s.card(&id).frozen.is_none(), "no meal, no Freeze");
            }

            #[test]
            fn r1020_never_a_face_down_trap_nor_a_field_spell_and_a_board_without_a_unit_offers_no_activate() {
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
                    let id = taotie_id(&s);
                    let theirs: Vec<String> = (1..=5).filter_map(|lane| s.backrow(P2, lane)).map(|card| card.id).collect();
                    assert!(!theirs.is_empty());

                    assert_eq!(offers(&s, &id), 0, "{seed}");
                    s.expect_refused(|s| devour(s, &id));

                    assert!(!fused(&s), "{seed}");
                    for card in &theirs {
                        s.expect_in_zone(card.as_str(), "field");
                    }
                    s.expect_stats(id.as_str(), json!({ "attack": BASE.0, "health": BASE.1 }));
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
                    let id = taotie_id(&s);
                    let unit = s.unit(P2, 1).expect("the Unit").id;

                    devour(&mut s, &id);

                    s.expect_in_zone(unit.as_str(), "gone");
                    assert!(s.backrow(P2, 1).is_some() && s.backrow(P2, 2).is_some());
                }
            }

            #[test]
            fn r1020_r1081000_a_flying_unit_standing_in_the_backrow_is_a_unit_it_eats() {
                let mut s = table("taotie-flying", json!([]), meal(1));
                let id = taotie_id(&s);
                let flier = s.unit(P2, 1).expect("the meal").id;
                stand_in_backrow(&mut s, flier.as_str(), 3);
                assert!(s.unit(P2, 1).is_none());

                devour(&mut s, &id);

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
                let id = taotie_id(&s);
                let top = s.unit(P2, 1).expect("the pile's top");
                assert_eq!(top.def_id, FIENDER);

                devour(&mut s, &id);

                assert!(fused(&s));
                s.expect_in_zone(top.id.as_str(), "gone");
                assert_eq!(
                    s.unit(P2, 1).map(|unit| unit.def_id),
                    Some(BIG_FELINOR.to_string())
                );
            }

            #[test]
            fn r23_r3996002_an_immutable_enemy_is_no_meal_and_offers_no_activate() {
                let mut s = table(
                    "taotie-immutable-meal",
                    json!([]),
                    json!([{ "def": WALL, "lane": 1 }]),
                );
                let id = taotie_id(&s);

                assert_eq!(offers(&s, &id), 0);
                s.expect_refused(|s| devour(s, &id));

                assert!(!fused(&s));
                assert_eq!(s.unit(P2, 1).map(|unit| unit.def_id), Some(WALL.to_string()));
                s.expect_stats(id.as_str(), json!({ "attack": BASE.0, "health": BASE.1 }));
            }

            #[test]
            fn r23_r3996002_an_immutable_taotie_eats_nothing_and_offers_no_activate() {
                let mut s = table("taotie-immutable", json!([]), meal(1));
                let id = taotie_id(&s);
                find_instance_mut(s.state_mut(), &id)
                    .expect("taotie")
                    .granted_keywords
                    .push(json_as(json!({ "kind": "Immutable" })));

                assert_eq!(offers(&s, &id), 0);
                s.expect_refused(|s| devour(s, &id));

                assert!(!fused(&s));
                assert_eq!(s.unit(P2, 1).map(|unit| unit.def_id), Some(BODY.to_string()));
                assert_eq!(s.card(id.as_str()).def_id, TAOTIE);
            }

            #[test]
            fn r1020_keywords_tags_and_text_joined_cost_4_and_no_death() {
                let mut s = table("taotie-joined", json!([]), json!([{ "def": MROW, "lane": 1 }]));
                let id = taotie_id(&s);
                let mrow = s.unit(P2, 1).expect("mrow");

                devour(&mut s, &id);

                let after = s.card(id.as_str()).clone();
                let def = fused_def(&s, &after);
                // 2/2 and Mrow's 1/1.
                s.expect_stats(&after, json!({ "attack": BASE.0 + 1, "maxHealth": BASE.1 + 1, "health": BASE.1 + 1 }));
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
                let id = taotie_id(&s);
                let guard = s.unit(P1, 2).expect("p1's Defense Unit");
                let before = s.stats(&guard).armor;

                devour(&mut s, &id);

                // The D-fender's aura now runs on 饕餮, for 饕餮's side: +2 Armor.
                assert!(s.unit(P2, 1).is_none());
                assert_eq!(s.stats(&guard).armor, before + 2);
            }

            #[test]
            fn r1020_a_fused_end_of_turn_line_runs() {
                let mut s = table("taotie-eot", json!([]), json!([{ "def": SHREDDER, "lane": 1 }]));
                let id = taotie_id(&s);

                devour(&mut s, &id); // the Shredder is eaten.
                assert!(s.unit(P2, 1).is_none());
                let before = s.state().players.p2.hero.health;
                s.end_turn(); // p1's end: the fused Shredder's line hits the enemy hero.

                s.expect_health(P2, before - 2);
            }

            #[test]
            fn r1020_a_fused_cry_never_runs() {
                let mut s = table(
                    "taotie-cry",
                    json!([{ "def": BODY, "lane": 2 }]),
                    json!([{ "def": SPECIESIST, "lane": 1 }]),
                );
                let id = taotie_id(&s);
                let felinor = s.unit(P1, 2).expect("p1's non-Human Unit");

                devour(&mut s, &id);

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
                let id = taotie_id(&s);
                let felinor = s.unit(P1, 2).expect("p1's other Unit");

                devour(&mut s, &id); // 饕餮 eats the Saintess: 4/4.
                assert!(!s.card(&felinor).radiant);
                s.card_mut(id.as_str()).damage = 1;
                s.end_turn();
                s.play(
                    ECLIPSE,
                    json!({ "targets": [{ "pick": "instance", "instanceId": id }] }),
                );

                s.expect_in_zone(id.as_str(), "graveyard");
                assert!(s.card(&felinor).radiant);
            }

            #[test]
            fn r468_the_fused_definition_survives_json() {
                let mut s = table("taotie-json", json!([]), meal(1));
                let id = taotie_id(&s);

                devour(&mut s, &id);

                let after = s.unit(P1, 1).expect("taotie");
                let round_tripped: GameState =
                    serde_json::from_value(js(s.state())).expect("JSON is a state");
                let same = find_instance(&round_tripped, &after.id).expect("the fused card");
                assert_eq!(same.def_id, after.def_id);
                assert_ne!(same.def_id, TAOTIE);
                let view = unit_view(&round_tripped, same);
                assert_eq!(view.attack, BASE.0 + 3);
                assert!(s.unit(P2, 1).is_none());
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r3996002_radiant_is_4_4() {
                let mut s = table_as("taotie-radiant-stats", true, json!([]), json!([]));
                let taotie = s.unit(P1, 1).expect("taotie");
                s.expect_stats(&taotie, json!({ "attack": RADIANT.0, "health": RADIANT.1 }));
            }

            #[test]
            fn r3996002_radiant_fuses_one_enemy_unit_and_stays_thawed() {
                let mut s = table_as("taotie-radiant", true, json!([]), meal(1));
                let id = taotie_id(&s);

                devour(&mut s, &id);

                // One meal, from the field, and no card of the opponent's deck: the library keeps its three.
                assert_eq!(
                    s.events().iter().filter(|event| matches!(event, GameEvent::Fused { .. })).count(),
                    1
                );
                assert_eq!(s.pile(P2, "library").len(), 3);
                assert!(s.card(&id).frozen.is_none(), "no Freeze on the Radiant face");
                // The kept card is Radiant, so it shows the fusion's radiant form, the sum of the radiant
                // forms: 4 + 6 attack and 4 + 8 health (a Radiant Duplicating Felinors is 6/8).
                assert!(s.card(&id).radiant);
                s.expect_stats(id.as_str(), json!({ "attack": RADIANT.0 + 6, "health": RADIANT.1 + 8 }));
                // It may still attack this turn.
                s.attack(id.as_str(), "hero");
                s.expect_health(P2, HERO_HEALTH - RADIANT.0 - 6);
            }

            /// R3996002, R1841000: once a turn, and with no extra turn cooldown on the Radiant face: it is
            /// offered again on its next turn.
            #[test]
            fn r3996002_once_a_turn_the_fusion_keeps_the_use() {
                let mut s = table_as("taotie-radiant-once", true, json!([]), json!([HARMLESS, HARMLESS]));
                let id = taotie_id(&s);
                devour(&mut s, &id);
                // The fused card is still 饕餮's instance, its use spent: no second meal this turn.
                assert_eq!(offers(&s, &id), 0);
                s.expect_refused_with(|s| devour(s, &id), "already been used");
                s.end_turn().end_turn();
                assert_eq!(offers(&s, &id), 1);
            }

            #[test]
            fn r102_the_fusion_keeps_its_damage() {
                let mut s = scenario(json!({
                    "seed": "taotie-damage",
                    "p1": { "field": [{ "def": TAOTIE, "radiant": true, "damage": 1, "lane": 1 }], "hand": [FILLER], "library": [FILLER] },
                    "p2": { "field": [{ "def": MROW, "lane": 1 }], "hand": [FILLER], "library": [] },
                }));
                let id = taotie_id(&s);

                devour(&mut s, &id);

                let after = s.card(id.as_str()).clone();
                assert_eq!(after.damage, 1);
                assert!(s.stats(&after).health >= 1);
            }
        }

        /// R767000: the script kept for the replays after v0.6.5b through v0.6.6h: the base face's Activate
        /// with no extra turn cooldown.
        mod v0_6_6h {
            use super::*;

            #[test]
            fn r767000_a_game_on_v0_6_6h_offers_the_base_activate_again_on_its_next_turn() {
                crate::under_kept(TAOTIE, script_v0_6_6h(), || {
                    let mut s = table("taotie-kept-v0-6-6e", json!([]), json!([{ "def": BODY, "lane": 1 }, HARMLESS, HARMLESS]));
                    let id = taotie_id(&s);

                    devour(&mut s, &id);

                    assert!(s.card(&id).frozen.is_some(), "frozen");
                    s.end_turn().end_turn();
                    assert!(s.card(&id).frozen.is_none(), "thawed");
                    assert_eq!(offers(&s, &id), 1, "no cooldown");
                });
            }
        }

        /// R767000: the script kept for the replays of v0.6.5b and before, back to v0.5.12: an end of
        /// turn's fusion, the Radiant face's second meal from the opponent's deck.
        mod v0_6_5b {
            use super::*;

            #[test]
            fn r767000_r1020_a_game_on_v0_6_5b_eats_at_its_end_of_turn_only_its_own() {
                crate::under_kept(TAOTIE, script_v0_6_5b(), || {
                    // Its opponent's end of turn eats nothing...
                    let mut s = scenario(json!({
                        "seed": "taotie-kept-their-end",
                        "active": "p2",
                        "p1": { "field": [{ "def": TAOTIE, "lane": 1 }], "hand": [FILLER], "library": [FILLER, FILLER] },
                        "p2": { "field": meal(1), "hand": [FILLER], "library": [FILLER] },
                    }));
                    let id = taotie_id(&s);
                    s.end_turn();
                    assert_eq!(s.state().active, P1);
                    assert!(!fused(&s));
                    assert_eq!(offers(&s, &id), 0, "no Activate");
                    // ...and its own eats the Unit: 2 + 3 attack, 2 + 4 health.
                    s.end_turn();
                    assert!(fused(&s));
                    assert!(s.unit(P2, 1).is_none());
                    s.expect_stats(id.as_str(), json!({ "attack": BASE.0 + 3, "health": BASE.1 + 4 }));
                });
            }

            #[test]
            fn r767000_r1020_radiant_also_fuses_a_random_card_of_the_opponents_deck() {
                crate::under_kept(TAOTIE, script_v0_6_5b(), || {
                    let mut s = scenario(json!({
                        "seed": "taotie-radiant-deck",
                        "p1": { "field": [{ "def": TAOTIE, "radiant": true, "lane": 1 }], "library": [FILLER] },
                        "p2": { "field": meal(1), "library": [BODY, BODY, BODY] },
                    }));
                    let id = taotie_id(&s);

                    s.end_turn();

                    // Both meals went in: the field's and one of the deck's (the turn's draw took
                    // the third).
                    assert!(s.unit(P2, 1).is_none());
                    assert_eq!(s.pile(P2, "library").len(), 1);
                    // Both fusions summed their stats (R77), the kept card's radiant form the sum of the
                    // radiant forms: 4 + 6 + 6 attack and 4 + 8 + 8 health.
                    assert!(s.card(&id).radiant);
                    s.expect_stats(id.as_str(), json!({ "attack": RADIANT.0 + 12, "health": RADIANT.1 + 16 }));
                });
            }

            #[test]
            fn r767000_r23_radiant_never_eats_an_immutable_card_of_the_deck() {
                crate::under_kept(TAOTIE, script_v0_6_5b(), || {
                    let mut s = scenario(json!({
                        "seed": "taotie-radiant-immutable",
                        "p1": { "field": [{ "def": TAOTIE, "radiant": true, "lane": 1 }], "hand": [FILLER], "library": [FILLER] },
                        "p2": { "field": meal(1), "hand": [FILLER], "library": [WALL, WALL, WALL] },
                    }));
                    let id = taotie_id(&s);

                    s.end_turn();

                    // The field's meal went in; every deck card is Immutable, so the deck kept all three
                    // (one of them drawn as p2's turn began).
                    assert!(s.unit(P2, 1).is_none());
                    let walls = s.pile(P2, "library").len()
                        + s.hand(P2).iter().filter(|card| card.def_id == WALL).count();
                    assert_eq!(walls, 3);
                    s.expect_stats(id.as_str(), json!({ "attack": RADIANT.0 + 6, "health": RADIANT.1 + 8 }));
                });
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
