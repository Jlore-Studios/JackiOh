//! C #59 Plague Doctor (SPEC §8.6 row 59). (1) Unit, Human, Common, 2/3 → 4/6.
//!   Base:    "Cry: Deal damage equal to the number of Plague Counters on the field."
//!   Radiant: "Cry: Place {tokens|Plague Counter|Plague Counters} on this. Then deal damage equal to the
//!            number of Plague Counters on the field." — 2 tokens
//!   Engine:  "A declared target (R81); one hit of N, N = every Plague Counter on both sides, counted as it
//!            resolves, after the Radiant's own placement (N = 0 is no hit, R63). A `preview` (R280)
//!            shows N. Tunes: Radiant tokens 2 ↑."
//!
//! The target is declared with the play (R81): a Unit — the top of a unit pile, either side — or a hero.
//! N is every Plague Counter on the field, both sides, face-down cards included, as the Cry resolves. On the Radiant face the Cry first makes one placement of {tokens} on the Doctor itself
//! (`placePlague`, multiplied by the Doctor's own multiplier, as a placement on any card is), and N
//! counts those too. One hit of N on the target; N = 0 is no hit at all (R63).
//!
//! The Cry builds its list once, so N is read as it begins and the Radiant's own placement is added to
//! it: `tokensItPlaces` is exactly what `placePlague` puts on the Doctor (the declared number times its
//! multiplier, nothing when it is not on the field), and nothing else in the list moves a token between
//! the placement and the hit — a trigger the placement wakes waits for the whole Cry.
//!
//! R280: the preview is N — `damageNow`, the same function the Cry deals with: the tokens on the field
//! now plus, on the Radiant face, the ones its own placement would add. Plague Counters are public on
//! every permanent, a face-down one's included (§10.8), so the number reveals nothing. The label is
//! the phrase both faces print. Its proofs are in `test/preview.test.ts`.
//!
//! The number is the declared `tokens` (R386), read through `param`; the base face declares it too (the
//! entry's params are per card) but never reads it.

use jackioh_engine::prelude::*;
use jackioh_engine::effects::{damage, place_plague};
use std::sync::Arc;

pub const ID: &str = "classic-059";

/// R280: the formula as both faces print it.
pub const DOCTOR_LABEL: &str = "the number of Plague Counters on the field";

/// "a target": a Unit on either side, or either hero (R81).
fn targets() -> Vec<TargetDecl> {
    json_as(json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "hero"] } }]))
}

/// The TS `Read` (`{ state, self, radiant, controller }`): what the Cry's `EffectContext` and the
/// preview's `ConditionContext` both carry. `tokens` is `param(read, "tokens")` on the context it was
/// taken off, called only where the TS calls it (it refuses a read with no card to read it on).
pub struct Read<'a> {
    pub state: &'a GameState,
    pub self_: Option<&'a CardInstance>,
    pub radiant: bool,
    pub controller: PlayerId,
    pub tokens: &'a dyn Fn() -> i32,
}

/// Every Plague Counter on the field: each permanent on both sides, face-down ones included, the top of
/// each pile only (R13). Walked from the controller's side, so the read never asks whose turn it is.
fn tokens_on_field(read: &Read) -> i32 {
    permanents_on_field(read.state, Some(read.controller)).iter().fold(0, |sum, card| sum + plague_on(card))
}

/// The Plague Counters the Radiant face's own placement puts on the Doctor: the declared number times its
/// own multiplier (a Doctor fused onto a C #27 Pestilent Slime doubles it), in hand as it would once
/// played. A preview is asked only of a Doctor acting on the field or in a hand (R13), and its Cry runs
/// as it arrives on top of its zone, so the Doctor always carries the placement.
fn tokens_it_places(read: &Read) -> i32 {
    match read.self_ {
        Some(self_) if read.radiant => (read.tokens)().max(0) * plague_multiplier_of(read.state, self_),
        _ => 0,
    }
}

/// N: every Plague Counter on the field, plus the Radiant face's own placement, as the Cry resolves now.
pub fn damage_now(read: &Read) -> i32 {
    tokens_on_field(read) + tokens_it_places(read)
}

fn cry(ctx: &EffectContext) -> Vec<Effect> {
    let amount = damage_now(&Read {
        state: &ctx.state,
        self_: ctx.self_.as_ref(),
        radiant: ctx.radiant,
        controller: ctx.controller,
        tokens: &|| param(ctx, "tokens"),
    });
    let hit = if amount > 0 { vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))] } else { vec![] };
    if !ctx.radiant {
        return hit;
    }
    let mut effects = vec![place_plague(json_as(json!({ "target": { "of": "self" }, "amount": param(ctx, "tokens") })))];
    effects.extend(hit);
    effects
}

fn preview(read: &ConditionContext) -> Vec<PreviewValue> {
    let value = damage_now(&Read {
        state: &read.state,
        self_: Some(&read.self_),
        radiant: read.radiant,
        controller: read.controller,
        tokens: &|| param(read, "tokens"),
    });
    vec![json_as(json!({ "label": DOCTOR_LABEL, "value": value }))]
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(cry)),
        preview: Some(Arc::new(preview)),
        ..Script::default()
    };

    // The same script: the Radiant face's placement is `ctx.radiant`'s branch, and its 2 is the declared
    // `tokens`, which `param` reads off the running face.
    CardScripts { radiant: base.clone(), base }
}

// C #59 Plague Doctor — SPEC §8.6 row 59, BUILD M9 Classic row C 59: "Cry: one hit of N on a declared
// target (Unit or hero), N = every Plague Counter on the field, both sides and face-down cards included,
// counted as it resolves; N = 0 → no hit (R63); its preview is N (R280); radiant 4/6: first place 2
// tokens on itself, then count them too (its preview includes them); its tuned number (radiant tokens)
// reads through `param()` (R386)".
//
// The preview's proofs — its value on both faces against what the Cry then deals, its label in each
// face's text, and that it reads only public facts — are in `test/preview.test.ts`.
#[cfg(test)]
mod tests {
    use super::{script, ID};
    use jackioh_engine::testkit::*;
    use std::collections::BTreeSet;

    const DOCTOR: &str = "classic-059";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const FIENDER: &str = "core-092"; // (2) Unit 5/7 Stack.
    const PAWN: &str = "core-096"; // (1) Trap: answers only an attack that would be lethal.
    const MANA_WELL: &str = "core-006"; // (3) Field Spell.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    /// An engine value as the JSON the TS test reads (SURFACE §5.1: the same keys and values).
    fn js<T: serde::Serialize + ?Sized>(value: &T) -> Value {
        serde_json::to_value(value).expect("an engine value serialises")
    }

    /// The selection naming one instance, by its id.
    fn at(card: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": card }])
    }

    /// The TS `ENEMY_HERO` constant.
    fn enemy_hero() -> Value {
        json!([{ "pick": "hero", "player": "p2" }])
    }

    fn hits(s: &Scenario, target_id: &str) -> Vec<Value> {
        s.events()
            .iter()
            .map(js)
            .filter(|event| event["type"] == "damage" && event["targetId"] == target_id)
            .map(|event| event["amount"].clone())
            .collect()
    }

    fn all_hits(s: &Scenario) -> usize {
        s.events().iter().map(js).filter(|event| event["type"] == "damage").count()
    }

    /// The ids of `player`'s Doctors in hand, in hand order.
    fn doctors_in_hand(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player).iter().filter(|card| card.def_id == DOCTOR).map(|card| card.id.clone()).collect()
    }

    mod c_59_plague_doctor {
        use super::*;

        #[test]
        fn declares_one_target_a_unit_or_a_hero_on_either_side_its_one_number_and_one_script_on_both_faces() {
            crate::register_all();
            let def = js(&registered_catalog()[ID]);
            assert_eq!(def["id"], DOCTOR);
            let scripts = script();
            assert_eq!(
                js(&scripts.base.targets),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "hero"] } }]),
            );
            assert_eq!(def["params"], json!([{ "key": "tokens", "base": 2, "radiant": 2, "better": "up", "step": 1, "min": 1 }]));
            // TS `expect(radiant).toBe(base)`: the Radiant face is the same declaration and the same hooks.
            assert_eq!(js(&scripts.radiant.targets), js(&scripts.base.targets));
            assert_eq!(scripts.radiant.cry.is_some(), scripts.base.cry.is_some());
            assert_eq!(scripts.radiant.preview.is_some(), scripts.base.preview.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn is_a_2_3_its_cry_deals_one_hit_of_n_every_plague_counter_on_both_sides_face_down_cards_included() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [DOCTOR, ANCHOR],
                        "field": [{ "def": VANILLA, "counters": { "plague": 2 } }],
                        "backrow": [{ "def": MANA_WELL, "counters": { "plague": 1 } }],
                    },
                    "p2": {
                        "hand": [ANCHOR],
                        "field": [{ "def": MENACE, "counters": { "plague": 1 } }],
                        "backrow": [{ "def": PAWN, "faceUp": false, "counters": { "plague": 3 } }],
                    },
                }));
                let menace = s.card(MENACE).id.clone();

                s.play(DOCTOR, json!({ "targets": at(&menace) }));

                s.expect_stats(DOCTOR, json!({ "attack": 2, "health": 3 }));
                assert_eq!(hits(&s, &menace), vec![json!(7)]);
                s.expect_stats(&menace, json!({ "health": 2 }));
                // Nothing places or removes a token.
                assert_eq!(
                    s.events().iter().map(js).filter(|event| event["type"] == "counterChanged").collect::<Vec<Value>>(),
                    Vec::<Value>::new(),
                );
            }

            #[test]
            fn r81_a_hero_is_a_target_too_the_enemy_hero_or_your_own() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [DOCTOR, DOCTOR, ANCHOR], "field": [{ "def": VANILLA, "counters": { "plague": 3 } }], "health": 20 },
                    "p2": { "hand": [ANCHOR], "health": 20 },
                }));
                let doctors = doctors_in_hand(&s, PlayerId::P1);
                let (Some(first), Some(second)) = (doctors.first().cloned(), doctors.get(1).cloned()) else {
                    panic!("two Doctors in hand");
                };

                s.play(&first, json!({ "targets": enemy_hero() }));
                s.play(&second, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));

                s.expect_health(PlayerId::P2, 17);
                s.expect_health(PlayerId::P1, 17);
            }

            #[test]
            fn r81_legalactions_offers_every_unit_and_both_heroes_and_nothing_in_a_backrow() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [DOCTOR, ANCHOR], "field": [VANILLA] },
                    "p2": { "hand": [ANCHOR], "field": [MENACE], "backrow": [MANA_WELL] },
                }));
                let doctor = s.card(DOCTOR).id.clone();
                let offered: BTreeSet<String> = legal_actions(s.state(), PlayerId::P1)
                    .iter()
                    .map(js)
                    .filter(|action| action["type"] == "play" && action["instanceId"] == doctor)
                    .flat_map(|play| {
                        play["targets"]
                            .as_array()
                            .cloned()
                            .unwrap_or_default()
                            .into_iter()
                            .map(|target| {
                                if target["pick"] == "instance" {
                                    target["instanceId"].as_str().unwrap_or_default().to_string()
                                } else if target["pick"] == "hero" {
                                    format!("hero-{}", target["player"].as_str().unwrap_or_default())
                                } else {
                                    "?".to_string()
                                }
                            })
                            .collect::<Vec<String>>()
                    })
                    .collect();

                // TS compares two `Set`s: a `BTreeSet` keeps the comparison order-free.
                assert_eq!(
                    offered,
                    BTreeSet::from([
                        s.card(VANILLA).id.clone(),
                        s.card(MENACE).id.clone(),
                        "hero-p1".to_string(),
                        "hero-p2".to_string(),
                    ]),
                );
                let well = s.card(MANA_WELL).id.clone();
                s.expect_refused(|s| {
                    s.play(&doctor, json!({ "targets": at(&well) }));
                });
            }

            #[test]
            fn r63_n_0_is_no_hit_with_no_plague_counter_on_the_field_nothing_is_dealt() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [DOCTOR, ANCHOR] }, "p2": { "hand": [ANCHOR], "field": [MENACE], "health": 20 } }));

                s.play(DOCTOR, json!({ "targets": enemy_hero() }));

                s.expect_in_zone(DOCTOR, "field");
                assert_eq!(all_hits(&s), 0);
                s.expect_health(PlayerId::P2, 20);
            }

            #[test]
            fn c3_2_r13_a_card_dormant_under_a_stack_pile_is_not_on_the_field_its_tokens_are_not_counted() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [DOCTOR, ANCHOR] },
                    "p2": {
                        "hand": [ANCHOR],
                        "field": [{ "def": VANILLA, "counters": { "plague": 4 } }, { "def": FIENDER, "stack": true, "counters": { "plague": 1 } }],
                        "health": 20,
                    },
                }));

                s.play(DOCTOR, json!({ "targets": enemy_hero() }));

                s.expect_health(PlayerId::P2, 19);
            }

            #[test]
            fn counted_as_it_resolves_the_tokens_a_c_53_plague_crawler_placed_earlier_this_turn_count() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": ["classic-053", DOCTOR, ANCHOR] }, "p2": { "hand": [ANCHOR], "field": [MENACE], "health": 20 } }));
                assert_eq!(s.state().players.p2.hero.health, 20);
                let menace = s.card(MENACE).id.clone();
                s.play("classic-053", json!({ "targets": at(&menace) }));

                s.play(DOCTOR, json!({ "targets": enemy_hero() }));

                s.expect_health(PlayerId::P2, 19);
            }

            #[test]
            fn r177_the_tokens_on_a_face_down_enemy_trap_count_and_the_hit_never_names_the_trap_to_you() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [DOCTOR, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "backrow": [{ "def": PAWN, "faceUp": false, "counters": { "plague": 2 } }], "health": 20 },
                }));

                s.play(DOCTOR, json!({ "targets": enemy_hero() }));

                s.expect_health(PlayerId::P2, 18);
                assert!(!js(&s.view(PlayerId::P1)).to_string().contains(PAWN));
            }

            #[test]
            fn r386_the_base_face_never_reads_its_tokens_an_upgrade_of_them_changes_nothing() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [DOCTOR, ANCHOR], "field": [{ "def": VANILLA, "counters": { "plague": 1 } }] }, "p2": { "hand": [ANCHOR], "health": 20 } }));
                step_param(s.card_mut(DOCTOR), "tokens", 1);

                s.play(DOCTOR, json!({ "targets": enemy_hero() }));

                s.expect_health(PlayerId::P2, 19);
                assert!(s.card(DOCTOR).counters.plague.is_none());
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn is_a_4_6_its_cry_first_places_2_plague_counters_on_itself_then_deals_n_counting_them() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": DOCTOR, "radiant": true }, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [{ "def": MENACE, "counters": { "plague": 1 } }] },
                }));
                let menace = s.card(MENACE).id.clone();

                s.play(DOCTOR, json!({ "targets": at(&menace) }));

                s.expect_stats(DOCTOR, json!({ "attack": 4, "health": 6 }));
                assert_eq!(s.card(DOCTOR).counters.plague, Some(2));
                assert_eq!(hits(&s, &menace), vec![json!(3)]);
                // The placement comes first, then the hit.
                let order: Vec<Value> = s
                    .events()
                    .iter()
                    .map(js)
                    .filter(|event| event["type"] == "counterChanged" || event["type"] == "damage")
                    .map(|event| event["type"].clone())
                    .collect();
                assert_eq!(order, vec![json!("counterChanged"), json!("damage")]);
            }

            #[test]
            fn with_no_other_token_on_the_field_it_still_deals_2_its_own() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": DOCTOR, "radiant": true }, ANCHOR] }, "p2": { "hand": [ANCHOR], "health": 20 } }));

                s.play(DOCTOR, json!({ "targets": enemy_hero() }));

                s.expect_health(PlayerId::P2, 18);
            }

            #[test]
            fn its_placement_on_itself_is_one_placement_a_placement_trigger_on_it_answers_once() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": DOCTOR, "radiant": true }, ANCHOR] }, "p2": { "hand": [ANCHOR], "health": 20 } }));

                s.play(DOCTOR, json!({ "targets": enemy_hero() }));

                let doctor = s.card(DOCTOR).id.clone();
                assert_eq!(
                    s.events()
                        .iter()
                        .map(js)
                        .filter(|event| event["type"] == "counterChanged" && event["instanceId"] == doctor && event.get("placed").is_some())
                        .collect::<Vec<Value>>(),
                    vec![json!({ "type": "counterChanged", "instanceId": doctor, "counter": "plague", "value": 2, "placed": 2 })],
                );
            }

            #[test]
            fn r386_an_upgrade_places_3_on_itself_and_deals_3_more_than_the_field_held_a_degrade_places_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [{ "def": DOCTOR, "radiant": true }, { "def": DOCTOR, "radiant": true }, ANCHOR],
                        "field": [{ "def": VANILLA, "counters": { "plague": 1 } }],
                    },
                    "p2": { "hand": [ANCHOR], "health": 20 },
                }));
                let doctors = doctors_in_hand(&s, PlayerId::P1);
                let (Some(up), Some(down)) = (doctors.first().cloned(), doctors.get(1).cloned()) else {
                    panic!("two Doctors in hand");
                };
                step_param(s.card_mut(&up), "tokens", 1);
                step_param(s.card_mut(&down), "tokens", -1);

                s.play(&up, json!({ "targets": enemy_hero() }));
                assert_eq!(s.card(&up).counters.plague, Some(3));
                s.expect_health(PlayerId::P2, 16);

                // 1 + 3 on the field, then 1 more of its own.
                s.play(&down, json!({ "targets": enemy_hero() }));
                assert_eq!(s.card(&down).counters.plague, Some(1));
                s.expect_health(PlayerId::P2, 11);
            }
        }
    }
}
