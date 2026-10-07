//! C+ #61 Bauble Bubble (SPEC §8.7 row 61). (1) Field Spell, Fruit, Rare.
//!   Base:    "Death: Add {cards|Stockpile|Stockpiles} to your hand. Each costs (0)." — cards 2
//!   Radiant: "Death: Add {cards|Radiant Stockpile|Radiant Stockpiles} to your hand. Each costs (0)."
//!   Engine:  "A Death on a backrow card (§4.5): it fires when the card goes from the field to a
//!            graveyard, destroyed or sacrificed, as a Unit's does; not on bounce, exile, transform or
//!            steal (§6.2). Stockpile is #5; `costOverride` 0; the hand cap burns extras. A bait card:
//!            nothing until it pops. Tunes: cards 2 ↑."
//!
//! §4.5 step 3 fires a collected backrow card's Death as it does a Unit's (the engine's, proved in
//! `packages/engine/test/backrow-death.test.ts`), so the card is one Death hook.

use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-061";

/// TS `cardDef("core-005").id`.
const STOCKPILE: &str = "core-005";

fn bauble_bubble(radiant: bool) -> Script {
    Script {
        death: Some(hook(move |ctx| {
            (0..param(&*ctx, "cards"))
                .map(|_| add_to_hand(json_as(json!({ "defId": STOCKPILE, "costOverride": 0, "radiant": radiant }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: bauble_bubble(false),
        radiant: bauble_bubble(true),
    }
}

// C+ #61 Bauble Bubble — SPEC §8.7 row 61, BUILD M9 Classic+ row C+ 61: "Field Spell with no effect
// while it sits; its Death fires as it goes from its backrow zone to your graveyard (§4.5: destroyed by
// Guy Att, Crushing Walls, Twisting Nether) and adds 2 Stockpiles (Core #5) that cost (0) to your hand;
// exile, a bounce or a Transform adds nothing; Carnivorous Cube can't eat it (R428: Units only); a full
// hand burns; hidden from the opponent (R97); the count reads through `param()`; radiant the
// Stockpiles are Radiant".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const BAUBLE: &str = "classicplus-061";
    const STOCKPILE: &str = "core-005";
    const NETHER: &str = "core-088"; // (4) Spell: destroy all permanents
    const JAMMED: &str = "core-036"; // (1) Spell: destroy target backrow card, Lock its zone
    const COLLATERAL: &str = "core-034"; // (4) Spell: exile target permanent and a random card of the enemy deck
    const TRANSMOGULATE: &str = "core-083"; // (2) Spell: replace every card of yours with a random Legendary
    const SILAS: &str = "core-052"; // Radiant: cards crossing to the opponent bounce to their controller's hand
    const CUBE: &str = "core-022"; // Carnivorous Cube: its Cry tributes one of your other Units
    const MIND_CONTROL: &str = "core-049"; // (4) Spell: steal target enemy permanent
    const VANILLA: &str = "core-008";
    const FILLER: &str = "core-011"; // Tempo Timmy, a (1) Unit (never a Stockpile, so the adds read plainly)

    use crate::scenario;

    /// p1's Stockpile `addedToHand` events in the last step, as `(instanceId, defId)`.
    fn stockpiles(s: &Scenario) -> Vec<(String, String)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: PlayerId::P1, instance_id, def_id } if def_id == STOCKPILE => {
                    Some((instance_id.clone(), def_id.clone()))
                }
                _ => None,
            })
            .collect()
    }

    /// Whether any event so far added a Stockpile to a hand.
    fn any_stockpile_added(s: &Scenario) -> bool {
        s.events()
            .iter()
            .any(|event| matches!(event, GameEvent::AddedToHand { def_id, .. } if def_id == STOCKPILE))
    }

    /// TS `bubble({ radiant?, hand, lane?, field? })`.
    fn bubble(radiant: bool, hand: Value, lane: Option<i32>, field: Value) -> Scenario {
        let mut bauble = json!({ "def": BAUBLE, "radiant": radiant });
        if let Some(lane) = lane {
            bauble["lane"] = json!(lane);
        }
        scenario(json!({
            "p1": { "backrow": [bauble], "hand": hand, "field": field, "library": [FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER], "field": [VANILLA], "library": [FILLER, FILLER] },
        }))
    }

    fn fillers(count: usize) -> Vec<Value> {
        (0..count).map(|_| json!(FILLER)).collect()
    }

    #[test]
    fn is_a_1_field_spell_fruit_naming_stockpile() {
        crate::register_all();
        let def = crate::card_def(super::ID);
        assert_eq!(def.type_, CardType::FieldSpell);
        assert_eq!(def.tags, vec![Tag::Fruit]);
        assert_eq!(def.refs, Some(vec![STOCKPILE.to_string()]));
    }

    mod base {
        use super::*;

        #[test]
        fn a_bait_card_nothing_happens_while_it_sits_across_both_players_turns() {
            let mut s = bubble(false, json!([FILLER]), None, json!([]));
            s.end_turn().end_turn();
            assert!(!any_stockpile_added(&s));
            s.expect_in_zone(BAUBLE, "field");
        }

        #[test]
        fn s4_5_destroyed_by_twisting_nether_its_death_adds_2_stockpiles_that_cost_0() {
            let mut s = bubble(false, json!([NETHER, FILLER]), None, json!([]));
            s.play(NETHER, json!({}));
            s.expect_in_zone(BAUBLE, "graveyard");
            let added: Vec<CardInstance> = stockpiles(&s).iter().map(|(id, _)| s.card(id).clone()).collect();
            assert_eq!(added.len(), 2);
            for card in &added {
                assert_eq!(card.zone.z(), ZoneName::Hand);
                assert_eq!(card.cost_override, Some(0));
                assert!(!card.radiant);
            }
            s.expect_events(json!(["destroyed", "addedToHand"]));
        }

        #[test]
        fn s4_5_destroyed_on_the_opponents_turn_by_their_nether_the_stockpiles_still_go_to_your_hand() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [BAUBLE], "hand": [FILLER], "library": [FILLER] },
                "p2": { "hand": [NETHER, FILLER], "library": [FILLER] },
            }));
            s.play(NETHER, json!({}));
            s.expect_in_zone(BAUBLE, "graveyard");
            assert_eq!(stockpiles(&s).len(), 2);
            assert!(!s.hand(PlayerId::P2).iter().any(|card| card.def_id == STOCKPILE));
        }

        #[test]
        fn s4_5_a_targeted_destroy_of_the_backrow_card_magic_jammed_fires_it_too() {
            let mut s = bubble(false, json!([JAMMED, FILLER]), None, json!([]));
            let bauble = s.card(BAUBLE).id.clone();
            s.play(JAMMED, json!({ "targets": [{ "pick": "instance", "instanceId": bauble }] }));
            s.expect_in_zone(BAUBLE, "graveyard");
            assert_eq!(stockpiles(&s).len(), 2);
        }

        #[test]
        fn s6_2_an_exile_adds_nothing() {
            let mut s = bubble(false, json!([COLLATERAL, FILLER]), None, json!([]));
            let bauble = s.card(BAUBLE).id.clone();
            s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": bauble }] }));
            s.expect_in_zone(BAUBLE, "exile");
            assert_eq!(stockpiles(&s), Vec::<(String, String)>::new());
        }

        #[test]
        fn s6_2_a_bounce_adds_nothing_a_radiant_silly_silas_rotates_it_across_so_it_bounces_home() {
            let mut s = bubble(false, json!([{ "def": SILAS, "radiant": true }, FILLER]), Some(5), json!([]));
            s.play(SILAS, json!({ "modes": ["right"] }));
            s.expect_in_zone(BAUBLE, "hand");
            let bauble = s.card(BAUBLE).id.clone();
            assert!(
                s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Bounced { instance_id, .. } if *instance_id == bauble))
            );
            assert_eq!(stockpiles(&s), Vec::<(String, String)>::new());
        }

        #[test]
        fn s6_2_a_transform_adds_nothing_transmogulate_replaces_it() {
            let mut s = bubble(false, json!([TRANSMOGULATE, FILLER]), None, json!([]));
            let id = s.card(BAUBLE).id.clone();
            s.play(TRANSMOGULATE, json!({}));
            assert!(s.events().iter().any(|event| matches!(event, GameEvent::Transformed { .. })));
            assert!(!any_stockpile_added(&s));
            s.expect_in_zone(&id, "gone");
        }

        #[test]
        fn s6_2_a_steal_adds_nothing_it_changes_sides_and_stays_on_the_field() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [BAUBLE], "hand": [FILLER], "library": [FILLER] },
                "p2": { "hand": [MIND_CONTROL, FILLER], "library": [FILLER] },
            }));
            let bauble = s.card(BAUBLE).id.clone();
            s.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": bauble }] }));
            s.expect_in_zone(BAUBLE, "field");
            assert_eq!(s.card(BAUBLE).controller, PlayerId::P2);
            assert!(!any_stockpile_added(&s));
        }

        #[test]
        fn r428_carnivorous_cube_cant_eat_it_units_only() {
            let mut s = bubble(false, json!([CUBE, FILLER]), None, json!([VANILLA]));
            let cube = s.card(CUBE).id.clone();
            let ids: Vec<String> = legal_actions(s.state(), PlayerId::P1)
                .into_iter()
                .flat_map(|action| match action {
                    ActionBody::Play { instance_id, targets, .. } if instance_id == cube => targets.unwrap_or_default(),
                    _ => Vec::new(),
                })
                .filter_map(|pick| match pick {
                    Selection::Instance { instance_id } => Some(instance_id),
                    _ => None,
                })
                .collect();
            let own = s.unit(PlayerId::P1, 1).map(|card| card.id).unwrap_or_default();
            assert!(ids.contains(&own));
            let bauble = s.card(BAUBLE).id.clone();
            assert!(!ids.contains(&bauble));
            s.expect_refused(|s| s.play(CUBE, json!({ "targets": [{ "pick": "instance", "instanceId": bauble }] })));
        }

        #[test]
        fn s2_4_r4_a_full_hand_burns_what_does_not_fit() {
            let mut hand = vec![json!(NETHER)];
            hand.extend(fillers(9));
            let mut s = bubble(false, json!(hand), None, json!([]));
            s.play(NETHER, json!({}));
            assert_eq!(stockpiles(&s).len(), 1);
            let burned = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Burned { def_id, .. } if def_id == STOCKPILE))
                .count();
            assert_eq!(burned, 1);
        }

        #[test]
        fn r97_the_opponent_sees_the_adds_under_the_sentinel() {
            let mut s = bubble(false, json!([NETHER, FILLER]), None, json!([]));
            s.play(NETHER, json!({}));
            let ids: Vec<String> = stockpiles(&s).into_iter().map(|(id, _)| id).collect();
            let theirs = s.view(PlayerId::P2);
            let text = serde_json::to_string(&theirs).unwrap();
            for id in &ids {
                assert!(!text.contains(&format!("\"{id}\"")));
            }
            let events: Vec<&GameEvent> = theirs
                .events
                .iter()
                .filter(|event| matches!(event, GameEvent::AddedToHand { player: PlayerId::P1, .. }))
                .collect();
            assert!(events.len() >= 2);
            for event in events {
                assert!(matches!(
                    event,
                    GameEvent::AddedToHand { instance_id, def_id, .. } if instance_id == "hidden" && def_id == "hidden"
                ));
            }
        }

        #[test]
        fn r386_an_upgrade_adds_3_a_degrade_1() {
            let mut up = bubble(false, json!([NETHER, FILLER]), None, json!([]));
            step_param(up.card_mut(BAUBLE), "cards", 1);
            up.play(NETHER, json!({}));
            assert_eq!(stockpiles(&up).len(), 3);

            let mut down = bubble(false, json!([NETHER, FILLER]), None, json!([]));
            step_param(down.card_mut(BAUBLE), "cards", -4);
            down.play(NETHER, json!({}));
            assert_eq!(stockpiles(&down).len(), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r74_its_death_adds_2_radiant_stockpiles_that_cost_0() {
            let mut s = bubble(true, json!([NETHER, FILLER]), None, json!([]));
            s.play(NETHER, json!({}));
            let added: Vec<CardInstance> = stockpiles(&s).iter().map(|(id, _)| s.card(id).clone()).collect();
            assert_eq!(added.len(), 2);
            assert!(added.iter().all(|card| card.radiant && card.cost_override == Some(0)));
        }

        #[test]
        fn s6_2_the_radiant_face_is_no_death_on_an_exile_either() {
            let mut s = bubble(true, json!([COLLATERAL, FILLER]), None, json!([]));
            let bauble = s.card(BAUBLE).id.clone();
            s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": bauble }] }));
            assert_eq!(stockpiles(&s), Vec::<(String, String)>::new());
        }
    }
}
