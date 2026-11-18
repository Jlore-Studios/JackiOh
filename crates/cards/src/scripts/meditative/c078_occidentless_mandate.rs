//! M #78 Occidentless Mandate (SPEC §8.8 row 78): (4) Spell, CN, Epic.
//!
//! Base:    "Destroy every non-CN permanent."
//! Radiant: "Destroy every enemy non-CN permanent."
//! Engine:
//! - **The destroy** (issue #4154, R4154103): `destroy_all` over every non-CN permanent of both sides
//!   (the Radiant face's: its opponent's alone), both rows, face-down ones included, all marked at once
//!   and collected in one state check (R59), so every Death fires once the board has gone (§6.3); an
//!   Indestructible one keeps its place (R46). A unit token ceases to exist (R11); a card dormant under
//!   a Stack is not on the field (R13); a Unit immune to Spells is spared (B5 E35, MD-B1, R940).
//! - **Tribal hate** (R1424): "non-CN" picks cards by lacking a tribal tag, and a card in Chinese has
//!   it (R1176000).
//! - **Kept for replays** (R767000): through v0.6.6e it read "Exile every non-CN permanent." (Radiant:
//!   "every enemy non-CN permanent"), `exile_all`, which ignores Indestructible and runs no Death
//!   (`script_v0_6_6e`, issue #4154); through v0.6.5b both faces exiled both sides and added a random CN
//!   card to their caster's hand for each of the caster's permanents exiled, counted first with the
//!   exile's own scope (R1142, Frozen Wastes' pattern, C+ #12.6), the Radiant face's Radiant and
//!   costing (`discount`) less (issue #3996); `script_v0_6_5b` keeps that.

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-078";

/// Every non-CN permanent of `side` ("any", or "enemy" on the Radiant face), both rows.
fn non_cn_permanents(side: &str) -> BoardScope {
    json_as(json!({ "side": side, "rows": ["units", "backrow"], "notTags": ["CN"] }))
}

/// R4154103: the destroy and nothing more.
fn mandate(side: &'static str) -> Script {
    Script {
        cry: Some(hook(move |_ctx| vec![destroy_all(non_cn_permanents(side))])),
        ..Script::default()
    }
}

/// Issue #3996, through v0.6.6e: the exile and nothing more.
fn mandate_v0_6_6e(side: &'static str) -> Script {
    Script {
        cry: Some(hook(move |_ctx| vec![exile_all(non_cn_permanents(side))])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: mandate("any"),
        radiant: mandate("enemy"),
    }
}

/// R767000: after v0.6.5b through v0.6.6e it exiled where it now destroys (issue #4154). `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_6_6e() -> CardScripts {
    CardScripts {
        base: mandate_v0_6_6e("any"),
        radiant: mandate_v0_6_6e("enemy"),
    }
}

/// R767000: the scripts v0.6.5b and every version before it ran: both sides exiled, and a random CN card
/// for each of the caster's permanents exiled (issue #3996). `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_5b() -> CardScripts {
    CardScripts {
        base: mandate_v0_6_5b(false),
        radiant: mandate_v0_6_5b(true),
    }
}

fn mandate_v0_6_5b(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let scope = non_cn_permanents("any");
            // R1142: counted before the exile, by the controller as it stands now.
            // MD-B1, R940: the exile is harmful — an immune card is in neither the count nor the exile.
            let yours = cards_in_scope_aimed(&*ctx, &scope, TargetAim::Harm)
                .iter()
                .filter(|card| card.controller == ctx.controller)
                .count() as i32;
            let mut effects = vec![exile_all(scope)];
            if yours > 0 {
                let mut add = json!({ "query": { "tags": ["CN"] }, "count": yours });
                if radiant {
                    add["radiant"] = json!(true);
                    add["costMod"] = json!(-param(&*ctx, "discount"));
                }
                effects.push(add_random_from_catalog(json_as(add)));
            }
            effects
        })),
        ..Script::default()
    }
}

// M #78 Occidentless Mandate — SPEC §8.8 row 78, BUILD M10 row M 78 (R4154103).
#[cfg(test)]
mod tests {
    use super::{ID, script_v0_6_5b, script_v0_6_6e};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008"; // Mr. Vanilla: Human
    const WALL: &str = "classic-041"; // State of the Game: Indestructible
    const SHEEPISH: &str = "core-041"; // Sheepish: a Trap, set face-down
    const BAT: &str = "classicplus-004"; // Juhan Biggest Bat: a CN Unit.
    const TATCHES: &str = "meditative-087"; // Tatches the Totem: CN among its five tribes
    const RUSH_TOKEN: &str = "core-t-rush";
    const BIG_FELINOR: &str = "core-043";
    const FIENDER: &str = "core-092"; // Felinor Fiender: Stack: played onto a pile, the cards beneath it are dormant
    const TOP_LOSER: &str = "classicplus-019-1"; // Top Loser: Radiant: Immune to Spells.
    const FILLER: &str = "core-016";

    fn decreed(seed: &str, radiant: bool, mine: Value, theirs: Value) -> Scenario {
        crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": [FILLER, FILLER], "field": mine },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER], "field": theirs },
        }))
    }

    /// The cards the play added to p1's hand, behind the filler it kept.
    fn added(s: &Scenario) -> Vec<CardInstance> {
        crate::hand_and_summoned_tatches(s, P1).into_iter().skip(1).collect()
    }

    fn on_field(s: &Scenario, player: PlayerId) -> Vec<String> {
        let mut out = Vec::new();
        for lane in 1..=5 {
            // Issue #3951: under the kept script, a Tatches the Cry added summons itself from the hand
            // once the play resolves, after the exile; it is counted among the cards added.
            if let Some(unit) = s.unit(player, lane).filter(|unit| unit.def_id != TATCHES) {
                out.push(unit.def_id);
            }
            if let Some(card) = s.backrow(player, lane) {
                out.push(card.def_id);
            }
        }
        out
    }

    /// R767000: `run` under the script kept for v0.6.5b, with the `discount` the card declared then (2, tuned
    /// on its Radiant face).
    fn kept<T>(run: impl FnOnce() -> T) -> T {
        crate::under_kept_as(
            ID,
            script_v0_6_5b(),
            |def| {
                def.params = serde_json::from_value(json!([
                    { "key": "discount", "base": 2, "radiant": 2, "better": "up", "step": 1, "min": 1, "tunedOn": "radiant" }
                ]))
                .expect("the v0.6.5b params");
            },
            run,
        )
    }

    mod m78_occidentless_mandate {
        use super::*;

        mod base {
            use super::*;

            /// R4154103: every non-CN permanent of both sides is destroyed, a face-down one too, and each
            /// goes to its graveyard; an Indestructible one stands.
            #[test]
            fn r4154103_destroys_every_non_cn_permanent_face_down_too_and_an_indestructible_one_stands() {
                let mut s = decreed(
                    "mandate-sweep",
                    false,
                    json!([VANILLA, BAT]),
                    json!([WALL, { "def": SHEEPISH, "row": "backrow" }]),
                );
                let wall = s.unit(P2, 1).expect("the wall");
                let trap = s.backrow(P2, 1).expect("the trap");
                let vanilla = s.unit(P1, 1).expect("p1's Vanilla");
                s.play(ID, json!({}));
                // The CN Unit and the Indestructible wall stand.
                assert_eq!(on_field(&s, P1), vec![BAT.to_string()]);
                assert_eq!(on_field(&s, P2), vec![WALL.to_string()]);
                s.expect_in_zone(&wall, "field");
                s.expect_in_zone(&trap, "graveyard");
                s.expect_in_zone(&vanilla, "graveyard");
                assert!(s.events().iter().any(|event| event.event_type() == GameEventType::Destroyed));
                assert!(s.pile(P1, "exile").is_empty() && s.pile(P2, "exile").is_empty(), "nothing exiled");
            }

            /// Issue #3996: no card comes in return, and nothing is drawn from the match rng for one.
            #[test]
            fn it_adds_no_card_to_your_hand() {
                let mut s = decreed("mandate-no-cards", false, json!([VANILLA, VANILLA, RUSH_TOKEN]), json!([VANILLA]));
                let hand = s.hand(P1).len();
                let cursor = s.state().rng_cursor;
                s.play(ID, json!({}));
                assert!(on_field(&s, P1).is_empty());
                assert!(on_field(&s, P2).is_empty());
                assert_eq!(s.hand(P1).len(), hand - 1, "only the Mandate left the hand");
                assert!(added(&s).is_empty());
                assert_eq!(s.state().rng_cursor, cursor);
            }

            #[test]
            fn r1176000_spares_a_translated_permanent() {
                // A Vanilla in Chinese is CN (R1176000): it is not "non-CN", so it is not destroyed, on
                // either side; p1's English Vanilla is.
                let mut s = decreed("mandate-translated", false, json!([VANILLA, VANILLA]), json!([VANILLA]));
                let translated = s.unit(P1, 1).expect("p1's Vanilla").id;
                let enemy = s.unit(P2, 1).expect("p2's Vanilla").id;
                let english = s.unit(P1, 2).expect("p1's other Vanilla");
                s.card_mut(&translated).chinese = Some(true);
                s.card_mut(&enemy).chinese = Some(true);
                s.play(ID, json!({}));
                assert_eq!(s.unit(P1, 1).expect("the translated Vanilla stands").id, translated);
                assert_eq!(s.unit(P2, 1).expect("the enemy's translated Vanilla stands").id, enemy);
                s.expect_in_zone(&english, "graveyard");
            }

            #[test]
            fn r940_a_unit_immune_to_spells_is_spared() {
                let mut s = decreed(
                    "mandate-immune",
                    false,
                    json!([{ "def": TOP_LOSER, "radiant": true }, VANILLA]),
                    json!([]),
                );
                let immune = s.unit(P1, 1).expect("the immune unit");
                s.play(ID, json!({}));
                assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(immune.id));
                assert!(s.unit(P1, 2).is_none(), "the Vanilla is destroyed");
            }
        }

        mod radiant {
            use super::*;

            /// R4154103: "Destroy every enemy non-CN permanent." Yours stay, and no card comes.
            #[test]
            fn r4154103_radiant_destroys_every_enemy_non_cn_permanent_and_spares_yours() {
                let mut s = decreed(
                    "mandate-radiant",
                    true,
                    json!([VANILLA, { "def": SHEEPISH, "row": "backrow" }]),
                    json!([WALL, BAT, { "def": SHEEPISH, "row": "backrow" }]),
                );
                let wall = s.unit(P2, 1).expect("the wall");
                let trap = s.backrow(P2, 1).expect("their trap");
                let hand = s.hand(P1).len();
                s.play(ID, json!({}));
                assert_eq!(on_field(&s, P1), vec![VANILLA.to_string(), SHEEPISH.to_string()]);
                assert_eq!(on_field(&s, P2), vec![WALL.to_string(), BAT.to_string()]);
                s.expect_in_zone(&wall, "field");
                s.expect_in_zone(&trap, "graveyard");
                assert_eq!(s.hand(P1).len(), hand - 1);
                assert!(added(&s).is_empty());
            }
        }

        /// R767000: after v0.6.5b through v0.6.6e it exiled every (enemy) non-CN permanent (issue #4154).
        mod kept_for_v0_6_6e {
            use super::*;

            #[test]
            fn r767000_v0_6_6e_exiles_every_non_cn_permanent_face_down_and_indestructible_too() {
                crate::under_kept(ID, script_v0_6_6e(), || {
                    let mut s = decreed(
                        "mandate-sweep",
                        false,
                        json!([VANILLA, BAT]),
                        json!([WALL, { "def": SHEEPISH, "row": "backrow" }]),
                    );
                    let wall = s.unit(P2, 1).expect("the wall");
                    let trap = s.backrow(P2, 1).expect("the trap");
                    s.play(ID, json!({}));
                    // Only the CN Unit stands.
                    assert_eq!(on_field(&s, P1), vec![BAT.to_string()]);
                    assert!(on_field(&s, P2).is_empty());
                    s.expect_in_zone(&wall, "exile");
                    s.expect_in_zone(&trap, "exile");
                    // An exile runs no Death and destroys nothing.
                    assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Destroyed));
                });
            }

            /// Issue #3996: no card comes in return, and nothing is drawn from the match rng for one.
            #[test]
            fn r767000_v0_6_6e_it_adds_no_card_to_your_hand() {
                crate::under_kept(ID, script_v0_6_6e(), || {
                    let mut s = decreed("mandate-no-cards", false, json!([VANILLA, VANILLA, RUSH_TOKEN]), json!([VANILLA]));
                    let hand = s.hand(P1).len();
                    let cursor = s.state().rng_cursor;
                    s.play(ID, json!({}));
                    assert!(on_field(&s, P1).is_empty());
                    assert!(on_field(&s, P2).is_empty());
                    assert_eq!(s.hand(P1).len(), hand - 1, "only the Mandate left the hand");
                    assert!(added(&s).is_empty());
                    assert_eq!(s.state().rng_cursor, cursor);
                });
            }

            #[test]
            fn r767000_v0_6_6e_spares_a_translated_permanent() {
                crate::under_kept(ID, script_v0_6_6e(), || {
                    // A Vanilla in Chinese is CN (R1176000): it is not "non-CN", so it is not exiled, on
                    // either side; p1's English Vanilla is.
                    let mut s = decreed("mandate-translated", false, json!([VANILLA, VANILLA]), json!([VANILLA]));
                    let translated = s.unit(P1, 1).expect("p1's Vanilla").id;
                    let enemy = s.unit(P2, 1).expect("p2's Vanilla").id;
                    let english = s.unit(P1, 2).expect("p1's other Vanilla");
                    s.card_mut(&translated).chinese = Some(true);
                    s.card_mut(&enemy).chinese = Some(true);
                    s.play(ID, json!({}));
                    assert_eq!(s.unit(P1, 1).expect("the translated Vanilla stands").id, translated);
                    assert_eq!(s.unit(P2, 1).expect("the enemy's translated Vanilla stands").id, enemy);
                    s.expect_in_zone(&english, "exile");
                });
            }

            #[test]
            fn r767000_v0_6_6e_a_dormant_stack_card_is_spared() {
                crate::under_kept(ID, script_v0_6_6e(), || {
                    let mut s = decreed(
                        "mandate-stack",
                        false,
                        json!([BIG_FELINOR, { "def": FIENDER, "stack": true }]),
                        json!([]),
                    );
                    let fiender = s.unit(P1, 1).expect("the top of the pile");
                    assert_eq!(fiender.def_id, FIENDER);
                    s.play(ID, json!({}));
                    s.expect_in_zone(&fiender, "exile");
                    // The dormant Big Felinor was not on the field (R13): it resumes.
                    assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(BIG_FELINOR.to_string()));
                });
            }

            #[test]
            fn r767000_v0_6_6e_a_unit_immune_to_spells_is_spared() {
                crate::under_kept(ID, script_v0_6_6e(), || {
                    let mut s = decreed(
                        "mandate-immune",
                        false,
                        json!([{ "def": TOP_LOSER, "radiant": true }, VANILLA]),
                        json!([]),
                    );
                    let immune = s.unit(P1, 1).expect("the immune unit");
                    s.play(ID, json!({}));
                    assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(immune.id));
                    assert!(s.unit(P1, 2).is_none(), "the Vanilla is exiled");
                });
            }

            /// Issue #3996: "Exile every enemy non-CN permanent." Yours stay, and no card comes.
            #[test]
            fn r767000_v0_6_6e_radiant_exiles_every_enemy_non_cn_permanent_and_spares_yours() {
                crate::under_kept(ID, script_v0_6_6e(), || {
                    let mut s = decreed(
                        "mandate-radiant",
                        true,
                        json!([VANILLA, { "def": SHEEPISH, "row": "backrow" }]),
                        json!([WALL, BAT, { "def": SHEEPISH, "row": "backrow" }]),
                    );
                    let wall = s.unit(P2, 1).expect("the wall");
                    let trap = s.backrow(P2, 1).expect("their trap");
                    let hand = s.hand(P1).len();
                    s.play(ID, json!({}));
                    assert_eq!(on_field(&s, P1), vec![VANILLA.to_string(), SHEEPISH.to_string()]);
                    assert_eq!(on_field(&s, P2), vec![BAT.to_string()]);
                    s.expect_in_zone(&wall, "exile");
                    s.expect_in_zone(&trap, "exile");
                    assert_eq!(s.hand(P1).len(), hand - 1);
                    assert!(added(&s).is_empty());
                });
            }
        }

        /// R767000: through v0.6.5b both faces exiled both sides and added a random CN card for each of the
        /// caster's permanents exiled, the Radiant face's Radiant and (2) cheaper (issue #3996).
        mod kept_for_v0_6_5b {
            use super::*;

            #[test]
            fn r767000_r1142_one_cn_card_per_permanent_of_yours() {
                kept(|| {
                    let mut s = decreed(
                        "mandate-count",
                        false,
                        json!([VANILLA, VANILLA, BAT]),
                        json!([VANILLA, WALL, { "def": SHEEPISH, "row": "backrow" }]),
                    );
                    s.play(ID, json!({}));
                    let added = added(&s);
                    // Two of p1's were exiled; the CN Bat was not, and p2's three are not p1's.
                    assert_eq!(added.len(), 2);
                    for card in &added {
                        let def = crate::card_def(&card.def_id);
                        assert!(def.tags.contains(&Tag::Cn), "{}", card.def_id);
                        assert!(!def.token);
                        assert_ne!(card.def_id, ID);
                        assert!(!card.radiant);
                        assert_eq!(card.cost_mod, 0);
                    }
                });
            }

            #[test]
            fn r767000_r1142_never_itself_though_its_set_ships() {
                kept(|| {
                    for n in 0..6 {
                        let mut s =
                            decreed(&format!("mandate-pool-{n}"), false, json!([VANILLA, VANILLA, VANILLA]), json!([]));
                        s.play(ID, json!({}));
                        let added = added(&s);
                        assert_eq!(added.len(), 3);
                        assert!(added.iter().all(|card| card.def_id != ID));
                    }
                });
            }

            #[test]
            fn r767000_r1142_a_unit_token_counts() {
                kept(|| {
                    let mut s = decreed("mandate-token", false, json!([RUSH_TOKEN]), json!([]));
                    let token = s.unit(P1, 1).expect("the token");
                    s.play(ID, json!({}));
                    // It ceased to exist instead of reaching exile (R11), and it was p1's permanent.
                    s.expect_in_zone(&token, "gone");
                    assert_eq!(added(&s).len(), 1);
                });
            }

            #[test]
            fn r767000_r1142_a_dormant_stack_card_is_spared_and_uncounted() {
                kept(|| {
                    let mut s = decreed(
                        "mandate-stack",
                        false,
                        json!([BIG_FELINOR, { "def": FIENDER, "stack": true }]),
                        json!([]),
                    );
                    s.play(ID, json!({}));
                    // Only the top counted.
                    assert_eq!(added(&s).len(), 1);
                });
            }

            #[test]
            fn r767000_r1142_counts_by_controller() {
                kept(|| {
                    let mut s = decreed("mandate-control", false, json!([VANILLA]), json!([VANILLA]));
                    // p1's Unit is p2's card that p1 controls (a steal, R12); p2's is p1's card p2 controls.
                    let stolen = s.unit(P1, 1).expect("p1's unit").id;
                    let lent = s.unit(P2, 1).expect("p2's unit").id;
                    find_instance_mut(s.state_mut(), &stolen).expect("the stolen unit").owner = P2;
                    find_instance_mut(s.state_mut(), &lent).expect("the lent unit").owner = P1;
                    s.play(ID, json!({}));
                    assert_eq!(added(&s).len(), 1);
                });
            }

            #[test]
            fn r767000_r1142_a_unit_immune_to_spells_is_neither_exiled_nor_counted() {
                kept(|| {
                    let mut s = decreed(
                        "mandate-immune",
                        false,
                        json!([{ "def": TOP_LOSER, "radiant": true }, VANILLA]),
                        json!([]),
                    );
                    let immune = s.unit(P1, 1).expect("the immune unit");
                    s.play(ID, json!({}));
                    assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(immune.id));
                    assert_eq!(added(&s).len(), 1);
                });
            }

            #[test]
            fn r767000_r1176000_a_translated_permanent_is_not_counted() {
                kept(|| {
                    let mut s = decreed("mandate-translated", false, json!([VANILLA, VANILLA]), json!([VANILLA]));
                    let translated = s.unit(P1, 1).expect("p1's Vanilla").id;
                    s.card_mut(&translated).chinese = Some(true);
                    s.play(ID, json!({}));
                    assert_eq!(added(&s).len(), 1, "one card for the one English Vanilla of p1's");
                });
            }

            #[test]
            fn r767000_s2_4_the_hand_cap_burns_the_overflow() {
                kept(|| {
                    let mut s = decreed("mandate-burn", false, json!([VANILLA, VANILLA, VANILLA]), json!([]));
                    for _ in 0..8 {
                        let filler = new_instance(s.state_mut(), FILLER, P1, Zone::Hand { player: P1 });
                        s.state_mut().players[P1].hand.push(filler);
                    }
                    s.play(ID, json!({}));
                    assert_eq!(s.hand(P1).len() as i32, HAND_CAP);
                    let burned = s.events().iter().filter(|event| event.event_type() == GameEventType::Burned).count();
                    assert_eq!(burned, 2);
                });
            }

            #[test]
            fn r767000_radiant_discounted_radiant_cn_cards_and_both_sides_exiled() {
                kept(|| {
                    let mut s = decreed("mandate-radiant", true, json!([VANILLA, VANILLA]), json!([VANILLA]));
                    s.play(ID, json!({}));
                    assert!(on_field(&s, P1).is_empty());
                    assert!(on_field(&s, P2).is_empty());
                    let added = added(&s);
                    assert_eq!(added.len(), 2);
                    for card in &added {
                        assert!(card.radiant);
                        assert_eq!(card.cost_mod, -2);
                        assert!(crate::card_def(&card.def_id).tags.contains(&Tag::Cn));
                    }
                });
            }
        }
    }
}
