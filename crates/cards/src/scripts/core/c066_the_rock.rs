//! #66 The Rock (SPEC §8.3, §6.3 Tribute, §3.2, R23, R46, R69, R81, R90).
//!
//! Base "Tribute 1, Indestructible"; radiant "Plus Immutable" (§8 Conventions: the base list plus X).
//! Both lists are on the catalog faces and §10.4 layer 1 reads them off the def, so this file grants
//! no keyword. It carries only the Tribute cost (§6.3: "an additional cost of playing a card"), which
//! `play_choices.rs` validates: `tribute_cost_of` reads `static_flags.tribute`; `tribute_value_of` gives
//! the Sheep Token 2 and any other unit 1 (§3.2, §6.3); `legal_tribute_units` offers only the chooser's
//! own units (only #55 Lava Golem sets `tribute_enemies`); `refuse_tributes` refuses an unpayable play.
//! Chosen units travel in the `play` action's `tributes` (R81, R90), so there is no hook.
//!
//! Engine-side: Indestructible takes no damage (§4.4 step 4); R46 and §4.5 step 1 ignore a destroy
//! mark, and the unit switches to Attack Position and loses Taunt for the turn; R69: it dies anyway
//! once max health is 0 or less, since no destroy effect is involved; §6.1: Sacrifice and Exile still
//! remove it. Immutable (R23) blocks Vanilla, Transform and Fuse-onto; Radiant is still allowed and
//! #41 Sheepish still fires, consumed for nothing (R17).

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
    // The radiant cell adds a keyword and nothing else, and §8's Conventions keep every base clause a
    // radiant cell does not restate — so the Tribute cost is kept and the script is the same object.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// #66 The Rock — SPEC §8.3, BUILD M4-T4: "Play refused without a tribute; Indestructible; radiant
// Immutable".
//
// The §8.3 row is "Tribute 1, Indestructible" → "Plus Immutable", with the Engine cell "Tribute
// validator; Indestructible per 6.1". So the cases below are: the validator refusing and accepting
// a Tribute (§6.3, §3.2's Sheep worth 2), Indestructible under damage (§4.4 step 4), under a destroy
// mark (R46) and at 0 max health (R69), and the radiant face keeping every base clause the cell does
// not restate (§8 Conventions) while adding Immutable (R23).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const ROCK: &str = "core-066"; // Unit 10/10 → 20/20, cost 4, Human. Tribute 1, Indestructible.
    const TIMMY: &str = "core-011"; // Tempo Timmy, a plain 1-cost 3/3: a body to tribute or to attack with.
    const SHEEP: &str = "core-t-sheep"; // Sheep Token, 1/1, "worth 2 Tributes while on the field" (§3.2).
    const RUSH: &str = "core-t-rush"; // Rush Token, 3/3: a second body, so an action can run a state check.
    const AURA: &str = "core-046"; // Suppressive Aura, Field Spell, embiggen price 4 — R69's −10/−10.

    /// R82: a turn whose only legal actions are ending it, conceding and offering a draw auto-ends, and
    /// `reduce` checks that after EVERY action, so every scenario keeps one free 0-cost Spell in p1's
    /// hand. It is never played; it only keeps one legal action on the turn.
    const ANCHOR: &str = "core-010"; // Rapid Replenish, Spell, cost 0 — always an affordable play.

    /// The refusal names the Tribute, capitalised or not, so the word's common tail is matched.
    const TRIBUTE_TEXT: &str = "ribute";

    /// `scenario(opts)` with ANCHOR appended to p1's hand. The shipped cards are registered first.
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

        #[test]
        fn s8_conventions_the_radiant_cell_adds_a_keyword_only_so_the_radiant_script_keeps_tribute_1() {
            crate::register_all();
            let scripts = super::super::script();
            // The radiant face is the base script, so every part of it matches.
            assert_eq!(scripts.radiant.static_flags, scripts.base.static_flags);
            assert!(scripts.radiant.cry.is_none() && scripts.radiant.aura.is_none());
            assert!(scripts.radiant.targets.is_empty() && scripts.radiant.modes.is_empty());
            assert_eq!(serde_json::to_value(&scripts.radiant.static_flags).unwrap(), json!({ "tribute": 1 }));
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

        #[test]
        fn s3_2_one_sheep_token_pays_tribute_1_on_its_own_because_it_is_worth_2() {
            let mut s = board(json!({ "p1": { "hand": [ROCK], "field": [SHEEP] } }));
            let sheep = s.card(SHEEP).clone();
            s.play(ROCK, json!({ "tributes": [SHEEP] }));
            s.expect_in_zone(ROCK, "field").expect_mana(PlayerId::P1, 0);
            // R11: a unit token that leaves the field ceases to exist rather than entering a graveyard.
            s.expect_in_zone(&sheep, "gone");
            assert!(!s.pile(PlayerId::P1, "graveyard").iter().any(|card| card.def_id == SHEEP));
        }

        #[test]
        fn s6_1_a_tribute_sacrifices_an_indestructible_unit_sacrifice_bypasses_it() {
            // Two copies: the string form resolves the hand one for `play` and the field one for `tributes`,
            // because each search is narrowed to its own place.
            let mut s = board(json!({ "p1": { "hand": [ROCK], "field": [ROCK] } }));
            let on_field = s.unit(PlayerId::P1, 1);
            s.play(ROCK, json!({ "tributes": [ROCK] }));
            assert!(on_field.is_some());
            s.expect_in_zone(on_field.as_ref().expect("setup: p1 holds The Rock in lane 1"), "graveyard");
        }

        // Base: Indestructible (§4.4 step 4, R46, R69)

        #[test]
        fn s4_4_step_4_indestructible_takes_no_damage_at_all_and_the_attacker_takes_the_full_10_back() {
            let mut s = board(json!({ "p1": { "field": [TIMMY] }, "p2": { "field": [ROCK] } }));
            s.attack(TIMMY, ROCK);
            s.expect_stats(ROCK, json!({ "health": 10, "maxHealth": 10 }))
                .expect_in_zone(TIMMY, "graveyard");
        }

        #[test]
        fn r46_an_indestructible_unit_ignores_a_destroy_mark_switching_to_attack_position_for_the_turn() {
            let mut s = board(json!({
                "p1": { "field": [{ "def": ROCK, "position": "DEF", "lane": 1 }, { "def": RUSH, "lane": 2 }] }
            }));
            // §6.3 Destroy "only marks the card"; `effects/destroy.rs` sets this flag and §4.5 step 1 collects
            // it at the next state check. No reachable verb destroys a chosen unit yet, so the mark is set
            // directly through `s.state_mut()`, a test-only liberty.
            let rock = s.card(ROCK).id.clone();
            find_instance_mut(s.state_mut(), &rock)
                .expect("setup: The Rock is on the field")
                .marked_destroyed = Some(true);
            s.switch_position(RUSH); // Any action runs the state check (R59).

            s.expect_in_zone(ROCK, "field").expect_events(json!(["positionSwitched"]));
            assert_ne!(s.card(ROCK).marked_destroyed, Some(true));
            assert_eq!(s.card(ROCK).position, Some(Position::Atk));
            // R46's other half: Taunt is suppressed for this turn. The Rock prints no Taunt, so the stamp is
            // what there is to see; #19 Midrange Menace and #55 Lava Golem are where it bites.
            assert_eq!(s.card(ROCK).taunt_suppressed_turn, Some(s.state().turn));
        }

        #[test]
        fn r69_an_indestructible_unit_whose_max_health_falls_to_0_dies_anyway_n46_radiant_3_2_2_then_paid_4_4_4() {
            // (#46 radiant: 3 × −2/−2, then paid 4 → −4/−4)
            let radiant_aura = json!({ "def": AURA, "radiant": true });
            let mut s = board(json!({
                "p1": { "hand": [radiant_aura], "backrow": [radiant_aura, radiant_aura, radiant_aura] },
                "p2": { "field": [ROCK] },
            }));
            s.expect_stats(ROCK, json!({ "maxHealth": 4 }));
            s.play(AURA, json!({ "embiggen": true }));
            // No destroy effect is involved, so Indestructible has nothing to ignore: it is collected like
            // any other unit and fires Death (R69, Hearthstone).
            s.expect_in_zone(ROCK, "graveyard").expect_events(json!(["destroyed"]));
        }

        // Radiant: "Plus Immutable" (§8 Conventions, R23)

        #[test]
        fn s5_2_the_radiant_face_is_20_20() {
            let mut s = board(json!({ "p2": { "field": [{ "def": ROCK, "radiant": true }] } }));
            s.expect_stats(ROCK, json!({ "attack": 20, "health": 20, "maxHealth": 20 }));
        }

        #[test]
        fn r23_the_radiant_face_computes_as_indestructible_plus_immutable_s8_conventions_plus() {
            let s = board(json!({ "p1": { "field": [{ "def": ROCK, "radiant": true }] } }));
            let kinds = keywords_of(&s, PlayerId::P1, 1);
            // Both are there, whatever else is.
            assert!(
                ["Indestructible", "Immutable"].iter().all(|wanted| kinds.iter().any(|kind| kind == wanted)),
                "keywords {kinds:?}"
            );
            // R23's blocking lives in `effects/transform.rs` (`transform` and `vanilla` return early on an
            // Immutable card) and in `traps.rs` for #41 Sheepish. No reachable verb applies Vanilla or Transform
            // to a chosen unit yet, so the cross-card cases live in #41, #61, #83 and #85.
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
