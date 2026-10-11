//! M #78 Occidentless Mandate (SPEC §8.8 row 78): (4) Spell, CN, Epic.
//!
//! Base:    "Exile every non-CN permanent. Add a random CN card to your hand for each of yours exiled
//!          this way."
//! Radiant: "Exile every non-CN permanent. Add a random Radiant CN card to your hand for each of yours
//!          exiled this way. Each costs ({discount}) less."
//! Engine:
//! - **Count, then exile** (Frozen Wastes' pattern, C+ #12.6): the count is the permanents the exile's
//!   own scope names that this player controls (R1142) — the tops of unit piles and every backrow
//!   card, face-down ones included, a stolen enemy card included and this player's card the opponent
//!   controls left out. Then `exile_all` over the same scope: exile ignores Indestructible and runs no
//!   Death (§6.3). A unit token counts, though it ceases to exist instead of reaching exile (R11); a
//!   card dormant under a Stack is not on the field (R13), so it is neither exiled nor counted and
//!   resumes; a Unit immune to Spells is in neither (B5 E35).
//! - **The cards:** `add_random_from_catalog` from `{ tags: ["CN"] }`, one per counted permanent:
//!   non-token CN cards of every set that ships (R380, R1420), never this card (R387), repeats allowed
//!   (R60); the hand cap burns the overflow (§2.4). The Radiant face's are Radiant and cost `discount`
//!   less (`costMod`; an X card ignores it, R65).
//! - **Tribal hate** (R1424): "non-CN" picks cards by lacking a tribal tag. The count and the exile read
//!   one scope, so a card that a tribal-hate immunity keeps out of the scope is kept out of both.

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-078";

/// Every non-CN permanent, both sides, both rows.
fn non_cn_permanents() -> BoardScope {
    json_as(json!({ "side": "any", "rows": ["units", "backrow"], "notTags": ["CN"] }))
}

fn mandate(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let scope = non_cn_permanents();
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

pub fn script() -> CardScripts {
    CardScripts {
        base: mandate(false),
        radiant: mandate(true),
    }
}

// M #78 Occidentless Mandate — SPEC §8.8 row 78, BUILD M10 row M 78: "Exiles every non-CN permanent
// on both sides and rows (face-down and Indestructible ones too, no Death); then adds one random
// non-token CN card for each permanent of yours it exiled (R1142), a unit token counting and a card
// dormant under a Stack neither exiled nor counted; the hand cap burns the overflow; radiant the cards
// are Radiant and cost (2) less each, the discount read through `param()`".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008"; // (1) Unit 4/4, Human.
    const WALL: &str = "classic-041"; // 3/3 Indestructible.
    const SHEEPISH: &str = "core-041"; // A Trap, set face-down.
    const BAT: &str = "classicplus-004"; // Juhan Biggest Bat: a CN Unit.
    const RUSH_TOKEN: &str = "core-t-rush";
    const BIG_FELINOR: &str = "core-043";
    const FIENDER: &str = "core-092"; // Stack: played onto a pile, the cards beneath it are dormant.
    const TOP_LOSER: &str = "classicplus-019-1"; // Radiant: Immune to Spells.
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
        s.hand(P1).into_iter().skip(1).collect()
    }

    fn on_field(s: &Scenario, player: PlayerId) -> Vec<String> {
        let mut out = Vec::new();
        for lane in 1..=5 {
            if let Some(unit) = s.unit(player, lane) {
                out.push(unit.def_id);
            }
            if let Some(card) = s.backrow(player, lane) {
                out.push(card.def_id);
            }
        }
        out
    }

    mod m78_occidentless_mandate {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn exiles_every_non_cn_permanent_face_down_and_indestructible_too() {
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
            }

            #[test]
            fn r1142_one_cn_card_per_permanent_of_yours() {
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
            }

            #[test]
            fn r1142_never_itself_with_the_set_previewed() {
                let _preview = preview_sets(&[SetName::Meditative]);
                for n in 0..6 {
                    let mut s = decreed(&format!("mandate-pool-{n}"), false, json!([VANILLA, VANILLA, VANILLA]), json!([]));
                    s.play(ID, json!({}));
                    let added = added(&s);
                    assert_eq!(added.len(), 3);
                    assert!(added.iter().all(|card| card.def_id != ID));
                }
            }

            #[test]
            fn r1142_a_unit_token_counts() {
                let mut s = decreed("mandate-token", false, json!([RUSH_TOKEN]), json!([]));
                let token = s.unit(P1, 1).expect("the token");
                s.play(ID, json!({}));
                // It ceased to exist instead of reaching exile (R11), and it was p1's permanent.
                s.expect_in_zone(&token, "gone");
                assert_eq!(added(&s).len(), 1);
            }

            #[test]
            fn r1142_a_dormant_stack_card_is_spared_and_uncounted() {
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
                // The dormant Big Felinor was not on the field (R13): it resumes, and only the top counted.
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(BIG_FELINOR.to_string()));
                assert_eq!(added(&s).len(), 1);
            }

            #[test]
            fn r1142_counts_by_controller() {
                let mut s = decreed("mandate-control", false, json!([VANILLA]), json!([VANILLA]));
                // p1's Unit is p2's card that p1 controls (a steal, R12); p2's is p1's card p2 controls.
                let stolen = s.unit(P1, 1).expect("p1's unit").id;
                let lent = s.unit(P2, 1).expect("p2's unit").id;
                find_instance_mut(s.state_mut(), &stolen).expect("the stolen unit").owner = P2;
                find_instance_mut(s.state_mut(), &lent).expect("the lent unit").owner = P1;
                s.play(ID, json!({}));
                assert_eq!(added(&s).len(), 1);
            }

            #[test]
            fn r1142_a_unit_immune_to_spells_is_neither_exiled_nor_counted() {
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
            }

            #[test]
            fn s2_4_the_hand_cap_burns_the_overflow() {
                let mut s = decreed("mandate-burn", false, json!([VANILLA, VANILLA, VANILLA]), json!([]));
                for _ in 0..8 {
                    let filler = new_instance(s.state_mut(), FILLER, P1, Zone::Hand { player: P1 });
                    s.state_mut().players[P1].hand.push(filler);
                }
                s.play(ID, json!({}));
                assert_eq!(s.hand(P1).len() as i32, HAND_CAP);
                let burned = s.events().iter().filter(|event| event.event_type() == GameEventType::Burned).count();
                assert_eq!(burned, 2);
            }

            #[test]
            fn nothing_of_yours_adds_nothing() {
                let mut s = decreed("mandate-none", false, json!([BAT]), json!([VANILLA]));
                let cursor = s.state().rng_cursor;
                s.play(ID, json!({}));
                assert!(added(&s).is_empty());
                assert_eq!(on_field(&s, P2), Vec::<String>::new());
                assert_eq!(s.state().rng_cursor, cursor);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn radiant_discounted_radiant_cn_cards() {
                let mut s = decreed("mandate-radiant", true, json!([VANILLA, VANILLA]), json!([VANILLA]));
                s.play(ID, json!({}));
                let added = added(&s);
                assert_eq!(added.len(), 2);
                for card in &added {
                    assert!(card.radiant);
                    assert_eq!(card.cost_mod, -2);
                    assert!(crate::card_def(&card.def_id).tags.contains(&Tag::Cn));
                }
            }

            #[test]
            fn discount_reads_through_param() {
                let mut s = decreed("mandate-radiant-param", true, json!([VANILLA]), json!([]));
                set_param(s.card_mut(ID), "discount", 3);
                s.play(ID, json!({}));
                assert_eq!(added(&s).iter().map(|card| card.cost_mod).collect::<Vec<i32>>(), vec![-3]);
            }
        }
    }
}
