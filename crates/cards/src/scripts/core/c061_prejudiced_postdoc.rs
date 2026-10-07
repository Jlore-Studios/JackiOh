//! #61 Prejudiced Postdoc (SPEC §8.3, R23, R57, R64, R81, R90).
//!
//! Base: "Cry: choose a Human unit on the field; summon a Vanilla copy". The radiant cell is "Any
//! unit", which per §8's Conventions restates only which units the pick may name — the Cry, the
//! Vanilla copy and the side it lands on are all kept.
//!
//! §8.3's Engine cell: "Copy per R57 with `vanilla` set and no granted keywords: the target's form,
//! radiant flag and buffs, no damage; auras apply to it afresh; an Immutable target is legal (R23)."
//! So the copy is R57's copy — defId, radiant flag, permanent buffs and `statsOverride` carried,
//! damage, exertion and counters reset — with two deviations R57 does not make on its own and that
//! only #61 asks for: the copy's Vanilla flag is on, and the copy carries NO granted keywords
//! (BUILD M4-T4 #8: "copy is stats only"). Both are arguments to `summon_copy`, never work this file
//! does: a card file composes effects and never touches state (CLAUDE.md rule 5).
//!
//! R23: Immutable blocks the Vanilla and Transform verbs on the Immutable card ITSELF. Here the
//! Vanilla applies to a brand-new card, so an Immutable target is a legal pick and the copy comes
//! out textless — the ruling names #61 for exactly this.
//!
//! Auras are layer 5 of §10.4 and are computed on read, never stored, so "auras apply to it afresh"
//! needs no code: the copy is a new instance under whatever auras its own side has.
//!
//! R81/R90: the pick travels in the `play` action as a declared target, so resolution never pauses.
//! The Postdoc is still in hand when the play's choices are validated (§10.5 step 1) and only
//! reaches the field at step 4, so it can never be its own target and needs no `excludeSelf`.
//! A declaration the board cannot satisfy does not refuse the play: the Cry fizzles and the unit
//! still enters (§8 Conventions, R90).

use jackioh_engine::effects::summon_copy;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-061";

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
//
// Must-pass: "Vanilla copy of a Human keeps the target's form and buffs, no keywords or text, no
// damage; auras apply afresh; an Immutable target is legal (R23, R57); radiant any unit."
#[cfg(test)]
mod tests {
    use super::script;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const POSTDOC: &str = "core-061"; // Unit, Human, 2 — 2/4 → 4/8
    const TIMMY: &str = "core-011"; // Unit, Human, 3/3, Rush + First Strike
    const ROCK: &str = "core-066"; // Unit, Human, 10/10 Indestructible → 20/20 plus Immutable
    const SHREDDER: &str = "core-013"; // Unit, no tags, 8/10 — the radiant face's "any unit"
    const PILLOW: &str = "core-065-1"; // #65.1, the aura source for "auras apply afresh"
    const SURGERY: &str = "core-063"; // #63, the only buff this wave can put on an enemy unit

    /// `s.unit()` is nullable; a lane assertion wants a hard failure when the lane is empty.
    fn unit_at(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        match s.unit(player, lane) {
            Some(found) => found,
            None => panic!("expected a unit in {player} lane {lane}, found none"),
        }
    }

    fn sel(card: &CardInstance) -> Value {
        json!({ "pick": "instance", "instanceId": card.id })
    }

    /// The Postdoc is a Unit, so it takes p1's leftmost free lane itself (R64) and its copy takes the
    /// next one. With an empty p1 board that is lane 1 for the Postdoc and lane 2 for the copy.
    const COPY_LANE: i32 = 2;

    /// The message a refused step panics with, for a TS `toThrow(/a|b|c/i)` whose pattern is a real
    /// regex (no regex crate in a pure crate, SURFACE §8): the caller matches it by hand.
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
        fn r57_the_copy_keeps_the_target_s_buffs_and_resets_its_damage() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [SURGERY, POSTDOC], "mana": 8 },
                "p2": { "field": [{ "def": TIMMY, "damage": 2 }] },
            }));
            let timmy = s.card(TIMMY).clone();

            s.play(SURGERY, json!({ "targets": [sel(&timmy)] }));
            s.expect_stats(&timmy, json!({ "attack": 6, "maxHealth": 6, "health": 4 }));

            s.play(POSTDOC, json!({ "targets": [sel(&timmy)] }));

            let copy = unit_at(&s, P1, COPY_LANE);
            assert_eq!(copy.def_id, TIMMY);
            // The target's form and buffs, at full health: R57 copies buffs and resets damage.
            s.expect_stats(&copy, json!({ "attack": 6, "maxHealth": 6, "health": 6 }));
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

            let copy = unit_at(&s, P1, COPY_LANE);
            // Vanilla: §10.4 drops the printed keywords (Rush, First Strike) on the read.
            assert!(copy.vanilla);
            // "no granted keywords": Plastic Surgery's keyword stayed on the original.
            assert!(copy.granted_keywords.is_empty());
            assert_eq!(s.card(&timmy).granted_keywords.len(), 1);
            // The original is untouched: only the copy is Vanilla.
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
            let copy = unit_at(&s, P1, COPY_LANE);
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

            let copy = unit_at(&s, P1, COPY_LANE);
            assert_eq!(copy.def_id, ROCK);
            s.expect_stats(&copy, json!({ "attack": 20, "maxHealth": 20 }));
            // The copy is Vanilla, so neither Indestructible nor Immutable reaches it.
            assert!(s.stats(&copy).keywords.is_empty());
            assert!(copy.vanilla);
            // R23 blocks Vanilla on the Immutable card itself, and nothing here touched it.
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

            let copy = unit_at(&s, P1, COPY_LANE);
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
            assert_eq!(unit_at(&s, P1, 5).def_id, POSTDOC);
            // Every lane is taken, so the copy had nowhere to go and was never created.
            let p1_defs: Vec<String> = (1..=5).map(|lane| unit_at(&s, P1, lane).def_id).collect();
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
            let postdoc = unit_at(&s, P1, 1);
            s.expect_stats(&postdoc, json!({ "attack": 4, "maxHealth": 8 }));
            let copy = unit_at(&s, P1, COPY_LANE);
            assert_eq!(copy.def_id, SHREDDER);
            s.expect_stats(&copy, json!({ "attack": 8, "maxHealth": 10 }));
            assert!(copy.vanilla);
        }

        #[test]
        fn r81_the_base_face_declares_a_human_only_pick_and_the_radiant_face_declares_any_unit() {
            // The pick travels in the play action, so the narrowing lives in the declaration `legalActions`
            // and R90 read — not in the hook. This is the only place the base/radiant difference exists.
            crate::register_all();
            let faces = script();
            assert_eq!(
                serde_json::to_value(&faces.base.targets).expect("declarations serialise"),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit"], "tags": ["Human"] } }])
            );
            assert_eq!(
                serde_json::to_value(&faces.radiant.targets).expect("declarations serialise"),
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

            // TS `toThrow(/Human|not a legal|option/i)`.
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
}
