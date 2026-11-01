//! SPEC §8.11 Training Grounds #53 Legacy: Prejudiced Postdoc,
//! the Legacy card of Core #61 Prejudiced Postdoc (`core-061`), a card of the held Training Grounds set (R1420,
//! R2194000), built by #2195.
//!
//! The code is a copy of the Core card's at the foundation's commit, not a re-export, so a later Core
//! patch does not move it (R2195003); it is named `Legacy: <the Core card's name>`, tagged Legacy
//! (R2511000), and is its own card, which a deck holds beside the Core card (R2511001). The Core
//! file's notes, as they stood:
//!
//! #61 Prejudiced Postdoc (SPEC §8.3, R23, R57, R64, R81, R90).
//! Base: "Cry: choose a Human unit on the field; summon a Vanilla copy". The radiant cell is "Any
//! unit", which per §8's Conventions restates only which units the pick may name.
//!
//! The copy is R57's (form and radiant flag carried; damage, exertion and counters reset) with three
//! deviations only #61 asks for, both arguments to `summon_copy` (CLAUDE.md rule 5): its Vanilla flag is
//! on, so it is at its printed stats with no buffs (R781003), and it has no granted keywords (BUILD
//! M4-T4 #8). R23: Immutable blocks Vanilla on the
//! Immutable card ITSELF, so an Immutable target is a legal pick. Auras (§10.4) are computed on read.
//!
//! R81/R90: the pick travels in the `play` action, so resolution never pauses. The Postdoc is still in
//! hand when the play's choices are validated (§10.5 step 1), so it needs no `excludeSelf`. A pick the
//! board cannot satisfy fizzles the Cry and the unit still enters (§8 Conventions, R90).

use jackioh_engine::effects::summon_copy;
use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-053";

/// The one difference between the faces: base narrows the pick to Humans, radiant does not.
fn postdoc(humans_only: bool) -> Script {
    let mut filter = json!({
        // "on the field" and §8's Conventions: either side unless the cell narrows it.
        "side": "any",
        "of": ["unit"],
    });
    if humans_only {
        filter["tags"] = json!(["Human"]);
    }
    let targets = vec![TargetDecl::target(1, 1, filter)];

    Script {
        targets,
        cry: Some(hook(|_ctx| {
            vec![summon_copy(json_as(json!({
                "of": { "of": "chosen" },
                // §8.3: the copy has no text and no granted keywords of its own.
                "vanilla": true,
                "grantedKeywords": false,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: postdoc(true),
        radiant: postdoc(false),
    }
}

// #61 Prejudiced Postdoc — SPEC §8.3, BUILD M4-T4 row 61.
// Must-pass: "Vanilla copy of a Human keeps the target's form at its printed stats, no buffs (R781003), no keywords or text, no
// damage; auras apply afresh; an Immutable target is legal (R23, R57); radiant any unit."
#[cfg(test)]
mod tests {
    use super::script;
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const POSTDOC: &str = "traininggrounds-053"; // Unit, Human, 2 — 2/4 → 4/8
    const TIMMY: &str = "core-011"; // Unit, Human, 3/3, Rush + First Strike
    const ROCK: &str = "core-066"; // Unit, Human, 10/10 Indestructible → 20/20 plus Immutable
    const SHREDDER: &str = "core-013"; // Unit, no tags, 6/10 — the radiant face's "any unit"
    const PILLOW: &str = "core-065-1"; // #65.1, the aura source for "auras apply afresh"
    const SURGERY: &str = "core-063"; // #63, the only buff this wave can put on an enemy unit

    fn sel(card: &CardInstance) -> Value {
        json!({ "pick": "instance", "instanceId": card.id })
    }

    /// The Postdoc takes p1's leftmost free lane itself (R64) and its copy the next one.
    const COPY_LANE: i32 = 2;

    /// The message a refused step panics with; the caller matches it by hand (no regex crate in a
    /// pure crate).
    fn refusal(s: &mut Scenario, step: impl FnOnce(&mut Scenario)) -> String {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| step(s))) {
            Ok(()) => panic!("expected the step to be refused"),
            Err(payload) => payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|message| (*message).to_string()))
                .unwrap_or_default(),
        }
    }

    mod prejudiced_postdoc {
        use super::*;

        #[test]
        fn r781003_the_copy_is_at_its_printed_stats_with_no_buffs_and_no_damage() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [SURGERY, POSTDOC], "mana": 8 },
                "p2": { "field": [{ "def": TIMMY, "damage": 2 }] },
            }));
            let timmy = s.card(TIMMY).clone();

            s.play(SURGERY, json!({ "targets": [sel(&timmy)] }));
            s.expect_stats(&timmy, json!({ "attack": 6, "maxHealth": 6, "health": 4 }));

            s.play(POSTDOC, json!({ "targets": [sel(&timmy)] }));

            let copy = s.unit_at(P1, COPY_LANE);
            assert_eq!(copy.def_id, TIMMY);
            // A Vanilla copy drops the buffs R57 would carry (R781003); R57 resets its damage.
            s.expect_stats(&copy, json!({ "attack": 3, "maxHealth": 3, "health": 3 }));
            assert_eq!((copy.buffs.attack, copy.buffs.health), (0, 0));
            assert_eq!(copy.damage, 0);
        }

        #[test]
        fn s8_3_the_copy_has_no_text_and_no_granted_keywords() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [SURGERY, POSTDOC], "mana": 8 },
                "p2": { "field": [TIMMY] },
            }));
            let timmy = s.card(TIMMY).clone();

            s.play(SURGERY, json!({ "targets": [sel(&timmy)] }));
            assert_eq!(s.card(&timmy).granted_keywords.len(), 1);

            s.play(POSTDOC, json!({ "targets": [sel(&timmy)] }));

            let copy = s.unit_at(P1, COPY_LANE);
            // Vanilla: §10.4 drops the printed keywords (Rush, First Strike) on the read.
            assert!(copy.vanilla);
            assert!(copy.granted_keywords.is_empty());
            assert_eq!(s.card(&timmy).granted_keywords.len(), 1);
            assert!(!s.card(&timmy).vanilla);
        }

        #[test]
        fn s10_4_auras_apply_to_the_copy_afresh_instead_of_being_copied() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [POSTDOC], "mana": 4 },
                "p2": { "field": [PILLOW, TIMMY] },
            }));
            let timmy = s.card(TIMMY).clone();
            // p2's Spikey Pillow drains p2's units: 3 attack − 2.
            s.expect_stats(&timmy, json!({ "attack": 1 }));

            s.play(POSTDOC, json!({ "targets": [sel(&timmy)] }));

            // The copy lands on p1, where no aura reaches it, so it reads its own layers from scratch.
            let copy = s.unit_at(P1, COPY_LANE);
            s.expect_stats(&copy, json!({ "attack": 3, "maxHealth": 3 }));
        }

        #[test]
        fn r23_an_immutable_target_is_legal_and_the_copy_is_the_one_that_is_made_vanilla() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [POSTDOC], "mana": 4 },
                // A Radiant The Rock is an Immutable Human (§8.3 row 66).
                "p2": { "field": [{ "def": ROCK, "radiant": true }] },
            }));
            let immutable = s.card(ROCK).clone();

            s.play(POSTDOC, json!({ "targets": [sel(&immutable)] }));

            let copy = s.unit_at(P1, COPY_LANE);
            assert_eq!(copy.def_id, ROCK);
            s.expect_stats(&copy, json!({ "attack": 20, "maxHealth": 20 }));
            // The copy is Vanilla, so neither Indestructible nor Immutable reaches it.
            assert!(s.stats(&copy).keywords.is_empty());
            assert!(copy.vanilla);
            assert!(!s.card(&immutable).vanilla);
        }

        #[test]
        fn r74_the_copy_keeps_the_target_s_radiant_flag() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [POSTDOC], "mana": 4 },
                "p2": { "field": [{ "def": TIMMY, "radiant": true }] },
            }));
            let timmy = s.card(TIMMY).clone();
            s.expect_stats(&timmy, json!({ "attack": 6, "maxHealth": 6 }));

            s.play(POSTDOC, json!({ "targets": [sel(&timmy)] }));

            let copy = s.unit_at(P1, COPY_LANE);
            assert!(copy.radiant);
            s.expect_stats(&copy, json!({ "attack": 6, "maxHealth": 6 }));
        }

        #[test]
        fn r64_no_free_zone_means_no_copy_and_the_postdoc_still_enters() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [POSTDOC], "field": ["core-008", "core-t-felinor", "core-t-rush", "core-t-sheep"], "mana": 4 },
                "p2": { "field": [TIMMY] },
            }));
            let timmy = s.card(TIMMY).clone();

            s.play(POSTDOC, json!({ "targets": [sel(&timmy)] }));

            s.expect_in_zone(POSTDOC, "field");
            assert_eq!(s.unit_at(P1, 5).def_id, POSTDOC);
            let p1_defs: Vec<String> = (1..=5).map(|lane| s.unit_at(P1, lane).def_id).collect();
            assert!(!p1_defs.iter().any(|def_id| def_id == TIMMY));
        }

        #[test]
        fn radiant_prejudiced_postdoc_copies_a_unit_with_no_human_tag() {
            // §5.2's in-hand swap: the card is Radiant before it is played, so its radiant Cry runs.
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": POSTDOC, "radiant": true }], "mana": 4 },
                "p2": { "field": [SHREDDER] },
            }));
            let shredder = s.card(SHREDDER).clone();

            s.play(POSTDOC, json!({ "targets": [sel(&shredder)] }));

            // The radiant face's own stats prove the radiant text is the one that ran.
            let postdoc = s.unit_at(P1, 1);
            s.expect_stats(&postdoc, json!({ "attack": 4, "maxHealth": 8 }));
            let copy = s.unit_at(P1, COPY_LANE);
            assert_eq!(copy.def_id, SHREDDER);
            s.expect_stats(&copy, json!({ "attack": 6, "maxHealth": 10 }));
            assert!(copy.vanilla);
        }

        #[test]
        fn base_postdoc_copies_a_newly_human_medina_enforcer() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [POSTDOC], "mana": 4 },
                "p2": { "field": ["meditative-083"] },
            }));
            let enforcer = s.card("meditative-083").clone();

            s.play(POSTDOC, json!({ "targets": [sel(&enforcer)] }));

            let copy = s.unit_at(P1, COPY_LANE);
            assert_eq!(copy.def_id, "meditative-083");
            assert!(copy.vanilla);
        }

        #[test]
        fn r81_the_base_face_declares_a_human_only_pick_and_the_radiant_face_declares_any_unit() {
            // The pick travels in the play action, so the narrowing lives in the declaration `legalActions`
            // and R90 read, not in the hook.
            crate::register_all();
            let faces = script();
            assert_eq!(
                js(&faces.base.targets),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit"], "tags": ["Human"] } }])
            );
            assert_eq!(
                js(&faces.radiant.targets),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit"] } }])
            );
        }

        #[test]
        fn r90_a_human_only_declaration_refuses_a_non_human_pick() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [POSTDOC], "mana": 4 },
                "p2": { "field": [SHREDDER] },
            }));
            let shredder = s.card(SHREDDER).clone();

            let message = refusal(&mut s, |s| {
                s.play(POSTDOC, json!({ "targets": [sel(&shredder)] }));
            })
            .to_lowercase();
            assert!(
                message.contains("human") || message.contains("not a legal") || message.contains("option"),
                "unexpected refusal: {message}"
            );
        }

        #[test]
        fn s8_conventions_a_cry_with_no_legal_target_fizzles_and_the_unit_still_enters() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [POSTDOC], "mana": 4 }, "p2": {} }));

            s.play(POSTDOC, json!({}));

            s.expect_in_zone(POSTDOC, "field");
            assert!(s.unit(P1, COPY_LANE).is_none());
        }
    }

    /// R2195003: the snapshot plays exactly as core-061, on both faces.
    #[test]
    fn r2195003_plays_exactly_as_core_061() {
        for radiant in [false, true] {
            crate::expect_plays_as(
                "core-061",
                super::ID,
                |id| json!({ "p1": { "hand": [{ "def": id, "radiant": radiant }] }, "p2": { "field": ["core-008"] } }),
                |s, id| {
                    let t = s.card("core-008").id.clone();
                    s.play(id, json!({ "targets": [{ "pick": "instance", "instanceId": t }] }));
                },
            );
        }
    }
}
