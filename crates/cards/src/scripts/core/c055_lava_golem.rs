//! #55 Lava Golem (SPEC §8.3, §6.3 Tribute/Sacrifice, §3.2; R4, R11, R65, R81, R90, R101, R360).
//! Unit 10/5 → 20/10, cost 3, Rare.
//!   Base:    "Taunt, Tribute 3. Can use enemy Units as Tributes. If any are used, summon this for your
//!            opponent."
//!   Radiant: "Taunt, Tribute 3. Can use enemy Units as Tributes."
//! Tribute targets chosen at play time (§10.5, R81, R90); enemy tributes sacrifice enemy units (R360; control is field-only, R15).
//! If any enemy tribute chosen, summons for opponent (R101; leftmost open zone, R64; §3.2, R4, R11, R65);
//! made GIGA in the hand, only where all four of its zones are open there, else where named (R4990000).
//! A face-down play by a lent Cloaked (M #45) is set on the player's own side whatever it tributed (R4265000).

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
// BUILD M4-T4 row 55: Tribute 3 counts enemy units and Sheep as 2; Taunt; an opposing unit in the
// Tribute summons the base face for the opponent (R360); Sheepish's free copy still needs tributes.
#[cfg(test)]
mod tests {
    use crate::js;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    fn ids_at(s: &Scenario, player: PlayerId, lanes: &[i32]) -> Vec<String> {
        lanes.iter().map(|&lane| s.unit_at(player, lane).id).collect()
    }

    /// Three of an inert 4/6 unit: keywords none, a Cry that never fires because it is placed, not played.
    fn fodder() -> Value {
        json!([
            { "def": "core-053", "lane": 1 },
            { "def": "core-053", "lane": 2 },
            { "def": "core-053", "lane": 3 },
        ])
    }

    use crate::matches_object;

    /// The keyword kinds a unit has now (§10.4), in layer order.
    fn keyword_kinds(s: &Scenario, card: &str) -> Vec<KeywordKind> {
        s.stats(card).keywords.iter().map(|keyword| keyword.kind()).collect()
    }

    /// The hand card #41 Sheepish's radiant creates: a `cost_override` of 0.
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
            let mut s = scenario(json!({ "p1": { "hand": ["core-055"], "field": [{ "def": "core-053", "lane": 1 }] } }));
            let one = ids_at(&s, P1, &[1]);

            s.expect_refused_with(|s| s.play("core-055", json!({ "tributes": one })), "Tribute 3");
            s.expect_refused_with(|s| s.play("core-055", json!({})), "Tribute 3");
            s.expect_in_zone("core-055", "hand");
        }
    }

    mod the_sheep_token_counts_2_s3_2_s6_3 {
        use super::*;

        #[test]
        fn s3_2_one_sheep_plus_one_other_unit_pays_the_3() {
            let mut s = scenario(json!({
                "p1": { "hand": ["core-055"], "field": [{ "def": "core-t-sheep", "lane": 1 }, { "def": "core-053", "lane": 2 }] },
            }));
            let sheep = s.unit_at(P1, 1).id;
            let other = s.unit_at(P1, 2).id;

            s.play("core-055", json!({ "tributes": [sheep, other] }));

            s.expect_in_zone("core-055", "field");
            // R11: a unit token that leaves the field ceases to exist and never reaches a graveyard.
            s.expect_in_zone(&sheep, "gone");
            s.expect_in_zone(&other, "graveyard");
        }

        #[test]
        fn s3_2_the_sheep_is_worth_2_not_3_one_sheep_alone_does_not_pay_this_cost() {
            let mut s = scenario(json!({ "p1": { "hand": ["core-055"], "field": [{ "def": "core-t-sheep", "lane": 1 }] } }));
            let one = ids_at(&s, P1, &[1]);

            s.expect_refused_with(|s| s.play("core-055", json!({ "tributes": one })), "Tribute 3");
        }

        #[test]
        fn s6_3_two_sheep_pay_4_for_a_cost_of_3_and_nothing_smaller_would_do() {
            // Dropping either Sheep leaves 2, under the cost, so the pair is minimal even though it
            // overshoots — the one case §6.3's Sheep clause creates.
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
    }

    mod can_use_opposing_units_as_tributes_r101 {
        use super::*;

        #[test]
        fn r101_an_enemy_unit_is_legal_fodder_and_it_is_sacrificed_not_destroyed() {
            // #66 The Rock is Indestructible, so a destroy would be ignored (§4.5 step 1, R46) — a
            // Sacrifice bypasses it (§6.3), which is what makes this the sharp test of the verb used.
            let mut s = scenario(json!({
                "p1": { "hand": ["core-055"], "field": [{ "def": "core-053", "lane": 1 }, { "def": "core-053", "lane": 2 }] },
                "p2": { "field": [{ "def": "core-066", "lane": 1 }] },
            }));
            let mine = ids_at(&s, P1, &[1, 2]);
            let theirs = s.unit_at(P2, 1).id;
            let mut tributes = mine.clone();
            tributes.push(theirs.clone());

            s.play("core-055", json!({ "tributes": tributes }));

            s.expect_in_zone("core-055", "field");
            // R2618000: The Rock stood on p2's side, so it lands in p2's graveyard.
            s.expect_in_zone(&theirs, "graveyard");
            let p2_graveyard: Vec<String> = s.pile(P2, "graveyard").iter().map(|card| card.id.clone()).collect();
            assert!(p2_graveyard.contains(&theirs));
            let p1_graveyard: Vec<String> = s.pile(P1, "graveyard").iter().map(|card| card.id.clone()).collect();
            assert_eq!(p1_graveyard, mine);
            s.expect_events(json!(["destroyed", "cardPlayed"]));
        }
    }

    mod r360_opposing_units_used_summoned_for_your_opponent {
        use super::*;

        #[test]
        fn r360_base_one_opposing_unit_in_the_tribute_puts_the_golem_on_the_opponent_s_side_in_the_lane_the_play_named() {
            let mut s = scenario(json!({
                "p1": { "hand": ["core-055"], "field": [{ "def": "core-053", "lane": 1 }, { "def": "core-053", "lane": 2 }] },
                "p2": { "field": [{ "def": "core-053", "lane": 1 }] },
            }));

            let turn = s.state().turn;
            let mut tributes = ids_at(&s, P1, &[1, 2]);
            tributes.push(s.unit_at(P2, 1).id);
            s.play("core-055", json!({ "zone": 3, "tributes": tributes }));

            let golem = s.card("core-055").clone();
            assert_eq!(s.unit(P2, 3).map(|unit| unit.id), Some(golem.id.clone()));
            assert!(s.unit(P1, 3).is_none());
            assert_eq!(golem.controller, P2);
            // Control, not ownership (§3.2): it is still p1's card, and it was p1's play.
            assert_eq!(golem.owner, P1);
            let played = s
                .events()
                .iter()
                .find(|event| event.event_type() == GameEventType::CardPlayed)
                .map(js);
            assert!(played.is_some_and(|event| matches_object(&event, &json!({ "player": "p1", "instanceId": golem.id }))));
            // It entered p2's side this turn, sick and with its exertion fresh like any arrival (R171).
            assert_eq!(golem.summoned_turn, Some(turn));
        }

        /// R4265000: set by the Cloaked M #45.1 lends (M #45's did through v0.6.13h), the Golem paid with an opposing
        /// unit stays in p1's backrow (fuzz handicapped seed 6928).
        #[test]
        fn r4265000_a_cloaked_play_paid_with_an_opposing_unit_is_set_in_the_player_s_own_backrow() {
            let mut s = scenario(json!({
                "p1": { "mana": 10, "hand": ["core-055"], "field": [{ "def": "core-053", "lane": 1 }, { "def": "core-053", "lane": 2 }, { "def": "meditative-045-1", "lane": 4 }] },
                "p2": { "field": [{ "def": "core-053", "lane": 1 }] },
            }));
            let golem = s.card("core-055").id.clone();
            let mut tributes = ids_at(&s, P1, &[1, 2]);
            tributes.push(s.unit_at(P2, 1).id);
            s.play("core-055", json!({ "row": "backrow", "zone": 3, "tributes": tributes, "faceDown": "cloaked" }));
            let set = s.backrow(P1, 3).expect("set in p1's backrow");
            assert_eq!(set.def_id, "core-055");
            assert_eq!(set.controller, P1);
            assert!(set.set_as.is_some());
            assert_ne!(set.id, golem, "a set card takes a fresh id (R227)");
            assert!(s.state().players.p2.units.iter().all(Option::is_none));
        }

        /// R1081000: a Flying Golem named into a backrow zone is still summoned for the opponent, into
        /// their unit zone of that lane.
        #[test]
        fn r1081000_played_flying_into_the_backrow_it_is_still_summoned_for_the_opponent() {
            let mut s = scenario(json!({
                "p1": { "hand": ["core-055"], "field": [{ "def": "core-053", "lane": 1 }, { "def": "core-053", "lane": 2 }] },
                "p2": { "field": [{ "def": "core-053", "lane": 1 }] },
            }));
            let golem = s.card("core-055").id.clone();
            crate::give_flying(&mut s, &golem);
            let mut tributes = ids_at(&s, P1, &[1, 2]);
            tributes.push(s.unit_at(P2, 1).id);

            s.play("core-055", json!({ "row": "backrow", "zone": 3, "tributes": tributes }));

            assert_eq!(s.unit(P2, 3).map(|unit| unit.id), Some(golem));
            assert!(s.backrow(P1, 3).is_none());
        }

        #[test]
        fn r360_base_the_opponent_s_leftmost_open_zone_when_the_lane_the_play_named_is_taken_there_r15() {
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
            assert_eq!(s.unit(P2, 1).map(|unit| unit.def_id).as_deref(), Some("core-055"));
            assert_eq!(s.card("core-055").controller, P2);
        }

        #[test]
        fn r360_base_a_tribute_of_the_player_s_own_units_only_keeps_the_golem_on_their_side() {
            let mut s = scenario(json!({
                "p1": { "hand": ["core-055"], "field": fodder() },
                "p2": { "field": [{ "def": "core-053", "lane": 1 }] },
            }));
            let mine = ids_at(&s, P1, &[1, 2, 3]);

            s.play("core-055", json!({ "zone": 4, "tributes": mine }));

            assert_eq!(s.unit(P1, 4).map(|unit| unit.def_id).as_deref(), Some("core-055"));
            assert_eq!(s.card("core-055").controller, P1);
        }

        #[test]
        fn r360_radiant_opposing_units_may_pay_and_the_golem_stays_with_the_player() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": "core-055", "radiant": true }] },
                "p2": { "field": fodder() },
            }));
            let theirs = ids_at(&s, P2, &[1, 2, 3]);

            s.play("core-055", json!({ "zone": 2, "tributes": theirs }));

            assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id).as_deref(), Some("core-055"));
            assert_eq!(s.card("core-055").controller, P1);
            assert_eq!(s.pile(P2, "graveyard").len(), 3);
        }
    }

    /// R4990000: a base Golem made GIGA in the hand (S+ #100 Plesiosize, S+ #23 GIGA-ify) and paid for
    /// with an opposing unit takes the opponent's anchor in the named lane only when all four of its zones
    /// are open, else their leftmost such anchor, else it lands where the play named (R360), never in a
    /// graveyard.
    mod r4990000_a_giga_golem_handed_over {
        use super::*;

        /// p1's GIGA Golem in hand and two of p1's units in lanes 4 and 5; p2's units in `p2_lanes`.
        /// Plays the Golem into p1's zone 1 paid with p1's two units and p2's lane-1 unit; returns the
        /// scenario and the Golem's id.
        fn handed_over(p2_lanes: &[i32]) -> (Scenario, String) {
            let p2_field: Vec<Value> =
                p2_lanes.iter().map(|lane| json!({ "def": "core-053", "lane": lane })).collect();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-055"], "field": [{ "def": "core-053", "lane": 4 }, { "def": "core-053", "lane": 5 }] },
                "p2": { "field": p2_field },
            }));
            let golem = s.card("core-055").id.clone();
            crate::give_giga(&mut s, &golem);
            let mut tributes = ids_at(&s, P1, &[4, 5]);
            tributes.push(s.unit_at(P2, 1).id);
            s.play("core-055", json!({ "zone": 1, "tributes": tributes }));
            (s, golem)
        }

        fn in_no_graveyard(s: &Scenario, id: &str) -> bool {
            [P1, P2].iter().all(|&player| s.pile(player, "graveyard").iter().all(|card| card.id != id))
        }

        #[test]
        fn r4990000_a_giga_base_golem_takes_the_opponent_s_leftmost_fitting_anchor() {
            // p2's lane 1 is open once its unit is paid, but lane 2 beside it is taken: lane 3 fits.
            let (s, golem) = handed_over(&[1, 2]);
            let placed = s.unit(P2, 3).expect("the Golem anchored in p2's lane 3");
            assert_eq!(placed.id, golem);
            assert_eq!(placed.controller, P2);
            assert_eq!(placed.giga, Some(true));
            assert!(in_no_graveyard(&s, &golem));
        }

        #[test]
        fn r4990000_a_giga_base_golem_with_no_fit_on_the_opponent_s_side_lands_where_the_play_named() {
            // p2's units in lanes 2 and 4 leave no two open lanes side by side.
            let (s, golem) = handed_over(&[1, 2, 4]);
            let placed = s.unit(P1, 1).expect("the Golem in p1's lane 1, where the play named");
            assert_eq!(placed.id, golem);
            assert_eq!(placed.controller, P1);
            assert_eq!(placed.giga, Some(true));
            assert!(s.state().players.p2.units.iter().flatten().flatten().all(|unit| unit.id != golem));
            assert!(in_no_graveyard(&s, &golem));
        }
    }

    /// R4834000: an Equip play onto S #16 Rod of Ages that R360 hands to the opponent fuses nothing.
    mod r4834000_an_equip_play_onto_rod_of_ages_s_16 {
        use super::*;

        const ROD: &str = "stinkyscape-016"; // Rod of Ages: lends Equip (R1869000)

        /// Rod of Ages face-up in p1's backrow zone 1, the Golem in hand, three of p2's units to pay with;
        /// the Golem's and Rod's ids, and p2's three units.
        fn onto_rod(golem: Value) -> (Scenario, String, String, Vec<String>) {
            let s = scenario(json!({
                "p1": { "mana": 10, "hand": [golem], "backrow": [{ "def": ROD, "lane": 1, "faceUp": true }] },
                "p2": { "field": fodder() },
            }));
            let golem = s.card("core-055").id.clone();
            let rod = s.backrow(P1, 1).expect("Rod of Ages").id;
            let theirs = ids_at(&s, P2, &[1, 2, 3]);
            (s, golem, rod, theirs)
        }

        #[test]
        fn r4834000_base_paid_with_opposing_units_it_is_handed_over_and_rod_of_ages_fuses_nothing() {
            let (mut s, golem, rod, theirs) = onto_rod(json!("core-055"));
            assert!(
                s.plays_of("core-055").iter().any(|play| matches!(
                    play,
                    ActionBody::Play { zone: Some(zone), equip: Some(true), tributes: Some(paid), .. }
                        if zone.row == Row::Backrow && zone.lane == 1 && paid.iter().all(|id| theirs.contains(id))
                )),
                "the Equip play onto Rod of Ages paid with p2's units is listed"
            );

            s.play("core-055", json!({ "zone": 1, "row": "backrow", "equip": true, "tributes": theirs }));

            // R360: summoned for p2 in the lane the play named, and no card stood on Rod to fuse.
            assert_eq!(s.unit(P2, 1).map(|unit| unit.id), Some(golem.clone()));
            assert_eq!(s.card("core-055").controller, P2);
            let rod_after = s.backrow(P1, 1).expect("Rod of Ages, left as it was");
            assert_eq!(rod_after.id, rod);
            assert_eq!(rod_after.def_id, ROD);
            assert!(!s.last_events().iter().any(|event| matches!(event, GameEvent::Fused { .. })));
            assert_eq!(s.pile(P2, "graveyard").len(), 3);
        }

        #[test]
        fn r4834000_radiant_paid_with_opposing_units_it_stays_and_fuses_into_rod_of_ages() {
            let (mut s, golem, rod, theirs) = onto_rod(json!({ "def": "core-055", "radiant": true }));

            s.play("core-055", json!({ "zone": 1, "row": "backrow", "equip": true, "tributes": theirs }));

            assert!(find_instance(s.state(), &golem).is_none(), "the Golem fused into Rod of Ages");
            assert!(s.last_events().iter().any(|event| matches!(event, GameEvent::Fused { .. })));
            assert_eq!(find_instance(s.state(), &rod).map(|card| card.controller), Some(P1));
            assert!(s.state().players.p2.units.iter().all(Option::is_none));
        }
    }

    mod printed_keywords_s6_1_s10_4_layer_1 {
        use super::*;

        #[test]
        fn patch_v0_1_1_no_armor_on_either_face_so_every_point_of_a_hit_lands() {
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
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": "core-055", "radiant": true, "lane": 1 }] },
                "p2": { "field": [{ "def": "core-054", "lane": 1 }], "hand": ["core-010"] },
            }));

            assert_eq!(keyword_kinds(&s, "core-055"), vec![KeywordKind::Taunt]);
            s.end_turn();
            s.attack("core-054", "core-055");

            // Straaza (#54) is a 6/6 since issue #4154: its 6 lands on the 20/10.
            s.expect_stats("core-055", json!({ "health": 4, "attack": 20, "maxHealth": 10 }));
            // The 20-attack retaliation still kills the 6/6 attacker.
            s.expect_in_zone("core-054", "graveyard");
        }

        #[test]
        fn s8_the_radiant_face_keeps_tribute_3() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": "core-055", "radiant": true }], "field": [{ "def": "core-053", "lane": 1 }] },
            }));
            let one = ids_at(&s, P1, &[1]);

            s.expect_refused_with(|s| s.play("core-055", json!({ "tributes": one })), "Tribute 3");
        }

        #[test]
        fn s6_3_a_radiant_golem_on_the_field_is_legal_fodder_like_any_unit() {
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
            let mut s = scenario(json!({ "p1": { "hand": ["core-055"], "field": [{ "def": "core-053", "lane": 1 }], "mana": 0 } }));
            // The card #41 Sheepish's radiant creates; a Tribute is an additional cost no price touches (§6.3).
            set_cost_override(&mut s, "core-055", 0);
            let one = ids_at(&s, P1, &[1]);

            s.expect_refused_with(|s| s.play("core-055", json!({ "tributes": one })), "Tribute 3");
        }

        #[test]
        fn r65_with_three_units_the_free_copy_plays_for_0_mana_and_still_sacrifices_all_three() {
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
