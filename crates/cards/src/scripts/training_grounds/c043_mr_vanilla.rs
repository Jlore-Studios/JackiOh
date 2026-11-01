//! SPEC §8.11 Training Grounds #43 Legacy: Mr. Vanilla,
//! the Legacy card of Core #8 Mr. Vanilla (`core-008`), a card of the held Training Grounds set (R1420,
//! R2194000), built by #2195.
//!
//! The code is a copy of the Core card's at the foundation's commit, not a re-export, so a later Core
//! patch does not move it (R2195003); it is named `Legacy: <the Core card's name>`, tagged Legacy
//! (R2511000), and is its own card, which a deck holds beside the Core card (R2511001). The Core
//! file's notes, as they stood:
//!
//! #8 Mr. Vanilla (SPEC §8.1): a 1-cost 4/4 → 12/12 Human Unit with no text on either face (patch
//! v0.1.1 took away its Immutable and the Radiant face's Divine Shield, and made it a true vanilla:
//! its Radiant face is its stats alone, tripled). A card with no text has no hooks, so both Scripts
//! are empty — an empty Script is the answer, not a placeholder.
//!
//! Nothing about it is special any more: Transform, Vanilla and Fuse-onto all reach it like any unit,
//! and making it Radiant on the field swaps its stat layer in place (§5.2), keeping its damage.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-043";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = Script::default();
    CardScripts { base, radiant }
}

// #8 Mr. Vanilla — SPEC §8.1 row 8, BUILD M4-T4.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    /// The §10.4-computed keyword kinds of a card on the field.
    fn keyword_kinds(s: &Scenario, card: &CardInstance) -> Vec<String> {
        s.stats(card)
            .keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    const FILLER: &str = "core-016"; // #16 Hit Job: an inert hand and library card, so no turn auto-ends.
    const MENACE: &str = "core-019"; // #19 Midrange Menace, 9/9 Taunt.
    const GIGA: &str = "core-029"; // #29 GIGA Glowy Jelly Bean.

    mod n8_mr_vanilla_base {
        use super::*;

        #[test]
        fn enters_as_a_4_4_with_no_keywords_and_no_text_of_its_own() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-008-base",
                "p1": { "hand": ["traininggrounds-043"], "mana": 4 },
                "p2": { "field": ["core-020"] }
            }));

            s.play("traininggrounds-043", json!({}));

            s.expect_stats("traininggrounds-043", json!({ "attack": 4, "health": 4, "maxHealth": 4 }));
            let vanilla = s.card("traininggrounds-043").clone();
            assert_eq!(keyword_kinds(&s, &vanilla), Vec::<String>::new());
            // No text: the play emits the play itself and nothing else — no Cry, no trigger. Its
            // announce (§10.5 step 3a, R448) is part of the play.
            let types: Vec<&str> = s.last_events().iter().map(|event| event.event_type().as_str()).collect();
            assert_eq!(
                types,
                vec!["manaChanged", "cardAnnounced", "cardPlayed", "summoned", "cardResolved"]
            );
        }

        #[test]
        fn patch_v0_1_1_no_longer_immutable_so_sheepish_transforms_it_into_a_sheep_token() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-008-sheepish",
                "p1": { "hand": ["traininggrounds-043"], "mana": 4 },
                // #41 Sheepish: "When your opponent plays a Unit: Transform it into a Sheep Token" (§8.2).
                "p2": { "backrow": ["core-041"], "field": ["core-020"] }
            }));
            let vanilla = s.card("traininggrounds-043").clone();

            s.play(&vanilla, json!({}));

            s.expect_in_zone(&vanilla, "gone");
            assert_eq!(
                s.unit(PlayerId::P1, 1).map(|unit| unit.def_id),
                Some("core-t-sheep".to_string())
            );
            s.expect_in_zone("core-041", "graveyard");
        }
    }

    mod n8_mr_vanilla_radiant {
        use super::*;

        #[test]
        fn r275_the_radiant_face_is_a_12_12_with_no_keywords_its_stats_are_its_whole_upgrade() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-008-radiant",
                // #26 Glowy Jelly Bean makes a chosen hand card Radiant (§8.2 row 26).
                "p1": { "hand": ["core-026", "traininggrounds-043"], "mana": 8 },
                "p2": { "field": ["core-020"] }
            }));
            let vanilla = s.card("traininggrounds-043").clone();

            s.play("core-026", json!({ "targets": [{ "pick": "instance", "instanceId": vanilla.id }] }));
            s.play(&vanilla, json!({}));

            s.expect_stats(&vanilla, json!({ "attack": 12, "health": 12, "maxHealth": 12 }));
            assert_eq!(keyword_kinds(&s, &vanilla), Vec::<String>::new());
        }

        #[test]
        fn s5_2_made_radiant_on_the_field_it_takes_the_12_12_face_at_once_and_keeps_its_damage() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-008-radiant-flip",
                // Radiant #29 GIGA Glowy Jelly Bean: "every card in your hand and every permanent you
                // control becomes Radiant" (§8.2 row 29); cost 7, so the mana is seeded above the
                // refresh (§2.3).
                "p1": {
                    "hand": [{ "def": GIGA, "radiant": true }, FILLER],
                    "field": [{ "def": "traininggrounds-043", "damage": 3 }],
                    "mana": 7,
                    "library": [FILLER]
                },
                "p2": { "hand": [FILLER], "field": [MENACE], "library": [FILLER] }
            }));
            let vanilla = s.card("traininggrounds-043").clone();
            s.expect_stats(&vanilla, json!({ "attack": 4, "health": 1, "maxHealth": 4 }));

            s.play(GIGA, json!({}));

            assert!(s.card(&vanilla).radiant);
            s.expect_stats(&vanilla, json!({ "attack": 12, "health": 9, "maxHealth": 12 }));

            // The damage came along (R22): at 9 health it trades with #19's 9 on p2's turn.
            s.end_turn();
            s.attack(MENACE, &vanilla);
            s.expect_in_zone(&vanilla, "graveyard");
            s.expect_in_zone(MENACE, "graveyard");
        }
    }

    /// R2195003: the snapshot plays exactly as core-008, on both faces: a vanilla body enters and stays.
    #[test]
    fn r2195003_plays_exactly_as_core_008() {
        for radiant in [false, true] {
            crate::expect_plays_as(
                "core-008",
                super::ID,
                |id| json!({ "p1": { "hand": [{ "def": id, "radiant": radiant }] }, "p2": { "field": ["core-025"] } }),
                |s, id| {
                    s.play(id, json!({}));
                    s.end_turn();
                    s.end_turn();
                },
            );
        }
    }
}
