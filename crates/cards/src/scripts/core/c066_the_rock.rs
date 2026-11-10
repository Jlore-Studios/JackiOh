//! #66 The Rock (SPEC §8.3, §6.3 Tribute, §3.2, R23, R46, R69, R81, R90).
//!
//! Base "Tribute 1, Indestructible"; radiant "Plus Immutable" (§8 Conventions: the base list plus X).
//! Both lists are on the catalog faces and §10.4 layer 1 reads them off the def, so this file grants
//! no keyword. It carries only the Tribute cost (§6.3), which `play_choices.rs` validates (a Sheep
//! Token pays 2, §3.2; only #55 Lava Golem sets `tribute_enemies`). Chosen units travel in the `play`
//! action's `tributes` (R81, R90), so there is no hook.
//!
//! Engine-side: Indestructible takes no damage (§4.4 step 4); R46 and §4.5 step 1 ignore a destroy
//! mark; R69: it dies anyway once max health is 0 or less; §6.1: Sacrifice and Exile still remove it.
//! Immutable (R23) blocks Vanilla, Transform and Fuse-onto; #41 Sheepish still fires, for nothing (R17).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-066";

/// §6.3 "Tribute 1": one of your units, or one Sheep Token, which is worth 2 of it (§3.2).
const TRIBUTE_COST: i32 = 1;

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    // §8 Conventions keep every base clause the radiant cell does not restate: the same script.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// #66 The Rock — SPEC §8.3, BUILD M4-T4.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const ROCK: &str = "core-066"; // The Rock: Human. Tribute 1, Indestructible.
    const TIMMY: &str = "core-011"; // Tempo Timmy: a plain unit: a body to tribute or to attack with

    /// R82: a turn left with only end, concede and offer-draw auto-ends after every action, so each
    /// scenario keeps this free Spell in p1's hand, never played, as one legal action on the turn.
    const ANCHOR: &str = "core-010"; // Rapid Replenish: a Spell, always an affordable play

    /// The refusal names the Tribute, capitalised or not, so the word's common tail is matched.
    const TRIBUTE_TEXT: &str = "ribute";

    /// `scenario(opts)` with ANCHOR appended to p1's hand.
    fn board(opts: Value) -> Scenario {
        crate::register_all();
        let mut opts = opts;
        let mut p1 = opts.get("p1").cloned().unwrap_or_else(|| json!({}));
        let mut hand = p1.get("hand").and_then(Value::as_array).cloned().unwrap_or_default();
        hand.push(json!(ANCHOR));
        p1["hand"] = Value::Array(hand);
        opts["p1"] = p1;
        scenario(opts)
    }

    /// The keyword set §10.4 computes for a unit, as the engine's own view reports it.
    fn keywords_of(s: &Scenario, player: PlayerId, lane: usize) -> Vec<String> {
        let view = s.view(PlayerId::P1);
        let side = if player == PlayerId::P1 { &view.you } else { &view.opponent };
        side.units
            .get(lane - 1)
            .cloned()
            .flatten()
            .map(|unit| unit.keywords.iter().map(|keyword| keyword.kind().as_str().to_string()).collect())
            .unwrap_or_default()
    }

    mod the_rock {
        use super::*;

        // The script itself

        #[test]
        fn s8_3_the_script_carries_only_the_tribute_cost_both_keywords_are_catalog_data_s10_4_layer_1() {
            crate::register_all();
            // Granting Indestructible or Immutable here would be a second source of truth (see #25).
            let scripts = super::super::script();
            let def = crate::card_def(super::super::ID);
            assert_eq!(serde_json::to_value(&scripts.base.static_flags).unwrap(), json!({ "tribute": 1 }));
            assert_eq!(
                def.base.keywords.iter().map(|keyword| keyword.kind().as_str()).collect::<Vec<_>>(),
                ["Indestructible"]
            );
            assert_eq!(
                def.radiant.keywords.iter().map(|keyword| keyword.kind().as_str()).collect::<Vec<_>>(),
                ["Indestructible", "Immutable"]
            );
            assert!(scripts.base.cry.is_none());
            assert!(scripts.base.aura.is_none());
        }

        #[test]
        fn r81_r90_the_tribute_travels_in_the_play_actions_own_list_so_the_card_declares_no_targets() {
            crate::register_all();
            let scripts = super::super::script();
            assert!(scripts.base.targets.is_empty());
            assert!(scripts.base.modes.is_empty());
        }

        // Base: the Tribute cost (§6.3, §3.2)

        #[test]
        fn build_row_66_refuses_the_play_with_no_unit_on_the_board_to_tribute_s6_3() {
            let mut s = board(json!({ "p1": { "hand": [ROCK] } }));
            s.expect_refused_with(|s| s.play(ROCK, json!({})), TRIBUTE_TEXT);
            s.expect_in_zone(ROCK, "hand").expect_mana(PlayerId::P1, 4);
        }

        #[test]
        fn build_row_66_refuses_the_play_when_a_unit_is_available_but_the_play_names_none() {
            let mut s = board(json!({ "p1": { "hand": [ROCK], "field": [TIMMY] } }));
            s.expect_refused_with(|s| s.play(ROCK, json!({})), TRIBUTE_TEXT);
            s.expect_in_zone(TIMMY, "field");
        }

        #[test]
        fn s6_3_plays_for_the_printed_4_once_a_unit_pays_the_tribute_and_that_unit_dies() {
            let mut s = board(json!({ "p1": { "hand": [ROCK], "field": [TIMMY] } }));
            s.play(ROCK, json!({ "tributes": [TIMMY] }));
            s.expect_in_zone(ROCK, "field")
                .expect_stats(ROCK, json!({ "attack": 10, "health": 10, "maxHealth": 10 }))
                .expect_in_zone(TIMMY, "graveyard")
                .expect_mana(PlayerId::P1, 0)
                .expect_events(json!(["destroyed", "cardPlayed"]));
        }

        // Base: Indestructible (§4.4 step 4, R46, R69)

        #[test]
        fn s4_4_step_4_indestructible_takes_no_damage_at_all_and_the_attacker_takes_the_full_10_back() {
            let mut s = board(json!({ "p1": { "field": [TIMMY] }, "p2": { "field": [ROCK] } }));
            s.attack(TIMMY, ROCK);
            s.expect_stats(ROCK, json!({ "health": 10, "maxHealth": 10 }))
                .expect_in_zone(TIMMY, "graveyard");
        }

        // Radiant: "Plus Immutable" (§8 Conventions, R23)

        #[test]
        fn r23_the_radiant_face_computes_as_indestructible_plus_immutable_s8_conventions_plus() {
            let s = board(json!({ "p1": { "field": [{ "def": ROCK, "radiant": true }] } }));
            let kinds = keywords_of(&s, PlayerId::P1, 1);
            assert!(
                ["Indestructible", "Immutable"].iter().all(|wanted| kinds.iter().any(|kind| kind == wanted)),
                "keywords {kinds:?}"
            );
            // R23's blocking is in `effects/transform.rs` and `traps.rs`; no reachable verb applies it to
            // a chosen unit yet, so the cross-card cases live in #41, #61, #83 and #85.
        }

        #[test]
        fn s8_conventions_the_radiant_face_still_costs_tribute_1_a_clause_the_cell_does_not_restate() {
            let mut s = board(json!({ "p1": { "hand": [{ "def": ROCK, "radiant": true }] } }));
            s.expect_refused_with(|s| s.play(ROCK, json!({})), TRIBUTE_TEXT);
        }

        #[test]
        fn s4_4_step_4_the_radiant_face_is_still_indestructible_under_damage() {
            let mut s = board(json!({
                "p1": { "field": [TIMMY] },
                "p2": { "field": [{ "def": ROCK, "radiant": true }] }
            }));
            s.attack(TIMMY, ROCK);
            s.expect_stats(ROCK, json!({ "health": 20, "maxHealth": 20 }))
                .expect_in_zone(TIMMY, "graveyard");
        }
    }
}
