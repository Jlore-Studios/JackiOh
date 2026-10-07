//! #55 Lava Golem (SPEC §8.3, §6.3 Tribute/Sacrifice, §3.2; R4, R11, R65, R81, R90, R101, R360).
//! Unit 10/5 → 20/10, cost 3, Rare.
//!   Base:    "Taunt, Tribute 3. Can use opposing Units as Tributes. If opposing Units are used,
//!            summon for your opponent."
//!   Radiant: "Taunt, Tribute 3. Can use opposing Units as Tributes."
//! Patch v0.1.1 took Armor 3 off both faces and Indestructible off the Radiant one, and gave the
//! base face its price: a Tribute that takes any opposing unit summons the Golem for the opponent.
//!
//! KEYWORDS ARE DATA, NOT SCRIPT. Both faces print [Taunt], read straight off the def by §10.4
//! layer 1 (`face_of` in engine/src/layers.rs), so this file grants nothing.
//!
//! THE COST IS THE SCRIPT. §6.3 calls Tribute "an additional cost of playing a card", so it lives in
//! the play validator (`play_choices.rs`), and the units chosen travel in the `play` action's own
//! `tributes` list (R81). What this file declares, and who reads it:
//!   * `tribute` — `tribute_cost_of(card)`: Tribute 3, with the Sheep Token worth 2 (`tribute_value_of`,
//!     §3.2), and `refuse_tributes` refusing a board that cannot pay; the units are sacrificed at §10.5
//!     step 2, which bypasses Indestructible and counts as a death (§6.3).
//!   * `tribute_enemies` — "Can use opposing Units as Tributes" (R101): `legal_tribute_units` offers both
//!     sides' units only to a card that says so.
//!   * `enemy_tribute_hands_over` — the base face's "If opposing Units are used, summon for your
//!     opponent" (R360): step 2 records whether the Tribute it paid took an opposing unit, and step 4
//!     then puts the Golem in the opponent's zone in the lane the player named, else their leftmost
//!     open one (R15), under their control; it stays the player's card and the player's play.
//!
//! R65/§6.3: a Tribute is an *additional* cost, so a mana price of 0 does not touch it — #41
//! Sheepish's radiant "Add a Lava Golem to your hand. It costs (0)." is a `cost_override` of 0 on the
//! mana term alone and that free copy still needs three units on the field.
//!
//! Nothing here is a hook: a play cost, and where the play lands, have to be readable before the card
//! resolves.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-055";

/// §8: "Tribute 3", counted with the Sheep Token worth 2 (§3.2).
const TRIBUTE_COST: i32 = 3;

/// The Radiant face: its Tribute, and the permission to pay it with opposing units (R101).
fn radiant_flags() -> StaticFlags {
    StaticFlags {
        tribute: Some(TRIBUTE_COST),
        tribute_enemies: Some(true),
        ..StaticFlags::default()
    }
}

/// The base face adds its price: paid with an opposing unit, it is summoned for the opponent (R360).
fn base_flags() -> StaticFlags {
    StaticFlags {
        enemy_tribute_hands_over: Some(true),
        ..radiant_flags()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            static_flags: Some(base_flags()),
            ..Script::default()
        },
        radiant: Script {
            static_flags: Some(radiant_flags()),
            ..Script::default()
        },
    }
}

// #55 Lava Golem (SPEC §8.3, §6.3 Tribute/Sacrifice, §3.2, §6.1, §4.2; R11, R12, R65, R81, R90,
// R101, R360).
// BUILD M4-T4 row 55: "Tribute 3 counts enemy units and Sheep as 2, enemies sacrificed; Taunt; an
// opposing unit in the Tribute summons the base face for the opponent (R360); radiant keeps it;
// Sheepish's free copy still needs tributes".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A unit on the board, so a test can name it in `tributes` without a non-null assertion.
    fn unit_at(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        match s.unit(player, lane) {
            Some(unit) => unit.clone(),
            None => panic!("no unit in {player}'s lane {lane}"),
        }
    }

    fn ids_at(s: &Scenario, player: PlayerId, lanes: &[i32]) -> Vec<String> {
        lanes.iter().map(|&lane| unit_at(s, player, lane).id).collect()
    }

    /// Three of an inert 4/6 unit: keywords none, a Cry that never fires because it is placed, not played.
    fn fodder() -> Value {
        json!([
            { "def": "core-053", "lane": 1 },
            { "def": "core-053", "lane": 2 },
            { "def": "core-053", "lane": 3 },
        ])
    }

    /// TS `[...list, ...more]` over two JSON arrays.
    fn concat(list: Value, more: Value) -> Value {
        let mut all = list.as_array().cloned().unwrap_or_default();
        all.extend(more.as_array().cloned().unwrap_or_default());
        Value::Array(all)
    }

    /// TS `toMatchObject`: every key the pattern names is in `actual`, with a matching value.
    fn matches_object(actual: &Value, pattern: &Value) -> bool {
        match (actual, pattern) {
            (Value::Object(have), Value::Object(want)) => want
                .iter()
                .all(|(key, value)| have.get(key).is_some_and(|got| matches_object(got, value))),
            (Value::Array(have), Value::Array(want)) => {
                have.len() == want.len() && have.iter().zip(want).all(|(got, value)| matches_object(got, value))
            }
            _ => actual == pattern,
        }
    }

    /// The keyword kinds a unit has now (§10.4), in layer order.
    fn keyword_kinds(s: &Scenario, card: &str) -> Vec<KeywordKind> {
        s.stats(card).keywords.iter().map(|keyword| keyword.kind()).collect()
    }

    /// `s.card(ref).costOverride = 0`: the hand card #41 Sheepish's radiant `addToHand` creates.
    fn set_cost_override(s: &mut Scenario, card: &str, cost: i32) {
        let id = s.card(card).id.clone();
        match find_instance_mut(s.state_mut(), &id) {
            Some(instance) => instance.cost_override = Some(cost),
            None => panic!("no instance {id}"),
        }
    }

    mod tribute_3_s6_3_r81_r90 {
        use super::*;

        #[test]
        fn s6_3_is_paid_with_three_of_your_own_units_which_are_sacrificed_to_your_graveyard() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": ["core-055"], "field": fodder() } }));
            let paid = ids_at(&s, P1, &[1, 2, 3]);

            s.play("core-055", json!({ "tributes": paid }));

            s.expect_in_zone("core-055", "field");
            for id in &paid {
                s.expect_in_zone(id, "graveyard");
            }
            assert_eq!(s.pile(P1, "graveyard").len(), 3);
            // §6.3 Sacrifice "counts as a death", so each one emits `destroyed` before it lands.
            s.expect_events(json!(["destroyed", "destroyed", "destroyed", "cardPlayed", "summoned"]));
            // §6.3: the Tribute is an ADDITIONAL cost, so the 3 mana is still paid too.
            s.expect_mana(P1, 1);
        }

        #[test]
        fn r90_a_play_with_too_few_units_is_refused_outright_s6_3_tribute_is_a_cost() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": ["core-055"], "field": [{ "def": "core-053", "lane": 1 }] } }));
            let one = ids_at(&s, P1, &[1]);

            s.expect_refused_with(|s| s.play("core-055", json!({ "tributes": one })), "Tribute 3");
            s.expect_refused_with(|s| s.play("core-055", json!({})), "Tribute 3");
            s.expect_in_zone("core-055", "hand");
        }

        #[test]
        fn r90_the_same_unit_cannot_pay_twice() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": ["core-055"], "field": fodder() } }));
            let one = unit_at(&s, P1, 1).id;

            s.expect_refused_with(
                |s| s.play("core-055", json!({ "tributes": [one, one, one] })),
                "same unit twice",
            );
        }

        #[test]
        fn s6_3_tributes_exactly_3_a_fourth_unit_on_top_of_a_paying_set_is_refused() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-055"], "field": concat(fodder(), json!([{ "def": "core-053", "lane": 4 }])) },
            }));
            let four = ids_at(&s, P1, &[1, 2, 3, 4]);

            s.expect_refused_with(|s| s.play("core-055", json!({ "tributes": four })), "no more");
        }
    }

    mod the_sheep_token_counts_2_s3_2_s6_3 {
        use super::*;

        #[test]
        fn s3_2_one_sheep_plus_one_other_unit_pays_the_3() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-055"], "field": [{ "def": "core-t-sheep", "lane": 1 }, { "def": "core-053", "lane": 2 }] },
            }));
            let sheep = unit_at(&s, P1, 1).id;
            let other = unit_at(&s, P1, 2).id;

            s.play("core-055", json!({ "tributes": [sheep, other] }));

            s.expect_in_zone("core-055", "field");
            // R11: a unit token that leaves the field ceases to exist and never reaches a graveyard.
            s.expect_in_zone(&sheep, "gone");
            s.expect_in_zone(&other, "graveyard");
        }

        #[test]
        fn s3_2_the_sheep_is_worth_2_not_3_one_sheep_alone_does_not_pay_this_cost() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": ["core-055"], "field": [{ "def": "core-t-sheep", "lane": 1 }] } }));
            let one = ids_at(&s, P1, &[1]);

            s.expect_refused_with(|s| s.play("core-055", json!({ "tributes": one })), "Tribute 3");
        }

        #[test]
        fn s6_3_two_sheep_pay_4_for_a_cost_of_3_and_nothing_smaller_would_do() {
            // `isMinimalTribute`: dropping either Sheep leaves 2, under the cost, so the pair is minimal
            // even though it overshoots — the one case §6.3's Sheep clause creates.
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-055"], "field": [{ "def": "core-t-sheep", "lane": 1 }, { "def": "core-t-sheep", "lane": 2 }] },
            }));
            let sheep = ids_at(&s, P1, &[1, 2]);

            s.play("core-055", json!({ "tributes": sheep }));

            s.expect_in_zone("core-055", "field");
            for id in &sheep {
                s.expect_in_zone(id, "gone");
            }
        }

        #[test]
        fn s6_3_a_tribute_written_into_a_script_ignores_the_2_which_is_why_this_card_has_no_hook() {
            // "A tribute written into a card's script is an ordinary Sacrifice of the permanent that script
            // names, where the Sheep Token's 2 never applies" (§6.3, #22 Carnivorous Cube). Lava Golem's
            // Tribute is the play COST instead, so the 2 does apply here and nowhere in this file.
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": ["core-055"], "field": [{ "def": "core-t-sheep", "lane": 1 }] } }));
            let one = ids_at(&s, P1, &[1]);

            s.expect_refused_with(|s| s.play("core-055", json!({ "tributes": one })), "Tribute 3");
        }
    }

    mod can_use_opposing_units_as_tributes_r101 {
        use super::*;

        #[test]
        fn r101_an_enemy_unit_is_legal_fodder_and_it_is_sacrificed_not_destroyed() {
            // #66 The Rock is Indestructible, so a destroy would be ignored (§4.5 step 1, R46) — a
            // Sacrifice bypasses it (§6.3), which is what makes this the sharp test of the verb used.
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-055"], "field": [{ "def": "core-053", "lane": 1 }, { "def": "core-053", "lane": 2 }] },
                "p2": { "field": [{ "def": "core-066", "lane": 1 }] },
            }));
            let mine = ids_at(&s, P1, &[1, 2]);
            let theirs = unit_at(&s, P2, 1).id;
            let mut tributes = mine.clone();
            tributes.push(theirs.clone());

            s.play("core-055", json!({ "tributes": tributes }));

            s.expect_in_zone("core-055", "field");
            // R12: off the field a card is always its OWNER's, so The Rock lands in p2's graveyard.
            s.expect_in_zone(&theirs, "graveyard");
            let p2_graveyard: Vec<String> = s.pile(P2, "graveyard").iter().map(|card| card.id.clone()).collect();
            assert!(p2_graveyard.contains(&theirs));
            let p1_graveyard: Vec<String> = s.pile(P1, "graveyard").iter().map(|card| card.id.clone()).collect();
            assert_eq!(p1_graveyard, mine);
            s.expect_events(json!(["destroyed", "cardPlayed"]));
        }

        #[test]
        fn r101_three_enemy_units_alone_can_pay_the_whole_cost() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-055"] },
                "p2": { "field": fodder() },
            }));
            let theirs = ids_at(&s, P2, &[1, 2, 3]);

            s.play("core-055", json!({ "tributes": theirs }));

            s.expect_in_zone("core-055", "field");
            assert_eq!(s.pile(P2, "graveyard").len(), 3);
        }

        #[test]
        fn r101_an_enemy_sheep_token_also_counts_2_s3_2_does_not_name_a_side() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-055"], "field": [{ "def": "core-053", "lane": 1 }] },
                "p2": { "field": [{ "def": "core-t-sheep", "lane": 1 }] },
            }));
            let tributes = [unit_at(&s, P1, 1).id, unit_at(&s, P2, 1).id];

            s.play("core-055", json!({ "tributes": tributes }));

            s.expect_in_zone("core-055", "field");
        }
    }

    mod r360_opposing_units_used_summoned_for_your_opponent {
        use super::*;

        #[test]
        fn r360_base_one_opposing_unit_in_the_tribute_puts_the_golem_on_the_opponent_s_side_in_the_lane_the_play_named() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-055"], "field": [{ "def": "core-053", "lane": 1 }, { "def": "core-053", "lane": 2 }] },
                "p2": { "field": [{ "def": "core-053", "lane": 1 }] },
            }));

            let turn = s.state().turn;
            let mut tributes = ids_at(&s, P1, &[1, 2]);
            tributes.push(unit_at(&s, P2, 1).id);
            s.play("core-055", json!({ "zone": 3, "tributes": tributes }));

            let golem = s.card("core-055").clone();
            assert_eq!(s.unit(P2, 3).map(|unit| unit.id.clone()), Some(golem.id.clone()));
            assert!(s.unit(P1, 3).is_none());
            assert_eq!(golem.controller, P2);
            // Control, not ownership (§3.2): it is still p1's card, and it was p1's play.
            assert_eq!(golem.owner, P1);
            let played = s
                .events()
                .iter()
                .find(|event| event.event_type() == GameEventType::CardPlayed)
                .map(|event| serde_json::to_value(event).expect("an event serialises"));
            assert!(played.is_some_and(|event| matches_object(&event, &json!({ "player": "p1", "instanceId": golem.id }))));
            // It entered p2's side this turn, sick and with its exertion fresh like any arrival (R171).
            assert_eq!(golem.summoned_turn, Some(turn));
        }

        #[test]
        fn r360_base_the_opponent_s_leftmost_open_zone_when_the_lane_the_play_named_is_taken_there_r15() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-055"] },
                "p2": { "field": [
                    { "def": "core-053", "lane": 2 },
                    { "def": "core-053", "lane": 3 },
                    { "def": "core-053", "lane": 4 },
                    { "def": "core-053", "lane": 5 },
                ] },
            }));
            let theirs = ids_at(&s, P2, &[2, 3, 4]);

            s.play("core-055", json!({ "zone": 5, "tributes": theirs }));

            // p2's lane 5 is taken, so the Golem takes p2's leftmost open zone, lane 1.
            assert_eq!(s.unit(P2, 1).map(|unit| unit.def_id.clone()).as_deref(), Some("core-055"));
            assert_eq!(s.card("core-055").controller, P2);
        }

        #[test]
        fn r360_base_a_tribute_of_the_player_s_own_units_only_keeps_the_golem_on_their_side() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-055"], "field": fodder() },
                "p2": { "field": [{ "def": "core-053", "lane": 1 }] },
            }));
            let mine = ids_at(&s, P1, &[1, 2, 3]);

            s.play("core-055", json!({ "zone": 4, "tributes": mine }));

            assert_eq!(s.unit(P1, 4).map(|unit| unit.def_id.clone()).as_deref(), Some("core-055"));
            assert_eq!(s.card("core-055").controller, P1);
        }

        #[test]
        fn r360_radiant_opposing_units_may_pay_and_the_golem_stays_with_the_player() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": "core-055", "radiant": true }] },
                "p2": { "field": fodder() },
            }));
            let theirs = ids_at(&s, P2, &[1, 2, 3]);

            s.play("core-055", json!({ "zone": 2, "tributes": theirs }));

            assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id.clone()).as_deref(), Some("core-055"));
            assert_eq!(s.card("core-055").controller, P1);
            assert_eq!(s.pile(P2, "graveyard").len(), 3);
        }
    }

    mod printed_keywords_s6_1_s10_4_layer_1 {
        use super::*;

        #[test]
        fn patch_v0_1_1_no_armor_on_either_face_so_every_point_of_a_hit_lands() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": "core-055", "lane": 1 }] },
                "p2": { "field": [{ "def": "core-t-rush", "lane": 1 }], "hand": ["core-010"] },
            }));

            s.end_turn();
            s.attack("core-t-rush", "core-055");
            s.expect_stats("core-055", json!({ "health": 2, "attack": 10, "maxHealth": 5 }));
            assert_eq!(keyword_kinds(&s, "core-055"), vec![KeywordKind::Taunt]);
        }

        #[test]
        fn s4_2_step_3_taunt_the_enemy_cannot_go_past_it_to_the_hero() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": "core-055", "lane": 2 }] },
                "p2": { "field": [{ "def": "core-t-rush", "lane": 1 }], "hand": ["core-010"] },
            }));

            s.end_turn();
            s.expect_refused_with(|s| s.attack("core-t-rush", "hero"), "Taunt");
            // The same attack aimed at the Taunt unit is legal.
            s.attack("core-t-rush", "core-055");
            s.expect_health(P1, 30);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn patch_v0_1_1_a_20_10_taunt_with_no_indestructible_so_it_takes_damage_and_can_die() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": "core-055", "radiant": true, "lane": 1 }] },
                "p2": { "field": [{ "def": "core-054", "lane": 1 }], "hand": ["core-010"] },
            }));

            assert_eq!(keyword_kinds(&s, "core-055"), vec![KeywordKind::Taunt]);
            s.end_turn();
            s.attack("core-054", "core-055");

            s.expect_stats("core-055", json!({ "health": 2, "attack": 20, "maxHealth": 10 }));
            // The 20-attack retaliation still kills the 8/8 attacker.
            s.expect_in_zone("core-054", "graveyard");
        }

        #[test]
        fn s8_the_radiant_face_keeps_tribute_3() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": "core-055", "radiant": true }], "field": [{ "def": "core-053", "lane": 1 }] },
            }));
            let one = ids_at(&s, P1, &[1]);

            s.expect_refused_with(|s| s.play("core-055", json!({ "tributes": one })), "Tribute 3");
        }

        #[test]
        fn s6_3_a_radiant_golem_on_the_field_is_legal_fodder_like_any_unit() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": ["core-055"],
                    "field": [
                        { "def": "core-055", "radiant": true, "lane": 1 },
                        { "def": "core-053", "lane": 2 },
                        { "def": "core-053", "lane": 3 },
                    ],
                },
            }));
            let paid = ids_at(&s, P1, &[1, 2, 3]);

            s.play("core-055", json!({ "tributes": paid }));

            assert_eq!(s.pile(P1, "graveyard").len(), 3);
        }
    }

    mod sheepish_s_free_copy_s8_engine_cell_r65 {
        use super::*;

        #[test]
        fn r65_a_costoverride_of_0_pays_no_mana_but_still_owes_the_tribute() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": ["core-055"], "field": [{ "def": "core-053", "lane": 1 }], "mana": 0 } }));
            // #41 Sheepish's radiant text is `addToHand({ defId: "core-055", costOverride: 0 })`; this is
            // the card that verb creates, and a Tribute is an additional cost that no price touches (§6.3).
            set_cost_override(&mut s, "core-055", 0);
            let one = ids_at(&s, P1, &[1]);

            s.expect_refused_with(|s| s.play("core-055", json!({ "tributes": one })), "Tribute 3");
        }

        #[test]
        fn r65_with_three_units_the_free_copy_plays_for_0_mana_and_still_sacrifices_all_three() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": ["core-055"], "field": fodder(), "mana": 0 } }));
            set_cost_override(&mut s, "core-055", 0);
            let paid = ids_at(&s, P1, &[1, 2, 3]);

            s.play("core-055", json!({ "tributes": paid }));

            s.expect_in_zone("core-055", "field").expect_mana(P1, 0);
            for id in &paid {
                s.expect_in_zone(id, "graveyard");
            }
        }
    }
}
