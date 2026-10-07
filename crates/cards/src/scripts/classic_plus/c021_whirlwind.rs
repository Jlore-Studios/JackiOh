//! C+ #21 Whirlwind (SPEC §8.7 row 21): Pierce, {damage} damage to every Unit (R346); the Radiant face
//! returns from the graveyard to hand at the end of the turn it was played (R68).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-021";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![damage_all(json_as(json!({
                "amount": param(&*ctx, "damage"),
                "side": "any",
                "ignoreArmor": true,
            })))]
        })),
        ..Script::default()
    };

    let radiant = Script {
        end_of_turn: Some(hook(|ctx| {
            if ctx.live_self().and_then(|me| me.return_to_hand_at_end_of_turn) == Some(true) {
                vec![bounce(json_as(json!({ "target": { "of": "self" } })))]
            } else {
                Vec::new()
            }
        })),
        ..base.clone()
    };

    CardScripts { base, radiant }
}

// C+ #21 Whirlwind — SPEC §8.7 row 21, BUILD M9 Classic+ row C+ 21: "Pierce: 1 damage to every Unit on
// both sides through Armor (an Armor 7 unit takes 1), Divine Shield still popping, all hits before the
// state check (R59); Spell Damage raises each hit; an Immune to Spells Unit takes none; the damage
// reads through `param()`; radiant also returns from your graveyard to your hand at the end of the
// turn (`returnToHandAtEndOfTurn`, R68), burned when the hand is full".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const WHIRLWIND: &str = "classicplus-021";
    const ARMORED: &str = "core-025"; // 7/7, Armor 7
    const SHIELDED: &str = "core-056"; // Jilliax 3/2, Divine Shield
    const BODY: &str = "core-019"; // 9/9
    const ONE: &str = "core-003"; // Right-house defender 1/1, Taunt, Divine Shield, Reborn
    const SOLARIUS: &str = "classicplus-038"; // Spell Damage +2
    const TOP_LOSER: &str = "classicplus-019-1"; // Radiant: Immune to Spells
    const FILLER: &str = "core-005";
    const DECK: [&str; 4] = [FILLER, FILLER, FILLER, FILLER];

    fn hits(s: &Scenario) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { amount, .. } => Some(*amount),
                _ => None,
            })
            .collect()
    }

    /// TS `s.unit(p, lane) ?? ""`: the unit's id, or a reference that names nothing.
    fn unit_or_blank(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.unit(player, lane).map(|unit| unit.id).unwrap_or_default()
    }

    fn has_keyword(s: &Scenario, card: &str, kind: KeywordKind) -> bool {
        s.stats(card).keywords.iter().any(|keyword| keyword.kind() == kind)
    }

    /// JS `indexOf` / `lastIndexOf` over event types: the position, or −1.
    fn position_of(types: &[GameEventType], wanted: GameEventType, last: bool) -> i64 {
        let found = if last {
            types.iter().rposition(|each| *each == wanted)
        } else {
            types.iter().position(|each| *each == wanted)
        };
        found.map_or(-1, |at| i64::try_from(at).unwrap_or(i64::MAX))
    }

    mod c_n21_whirlwind {
        use super::*;

        #[test]
        fn is_a_0_spell_that_prints_pierce_on_both_faces_only_the_radiant_face_returns() {
            let def = crate::card_def(ID);
            assert_eq!(def.cost, CardCost::Fixed(0));
            assert_eq!(def.base.keywords, vec![Keyword::Pierce]);
            assert_eq!(def.radiant.keywords, vec![Keyword::Pierce]);
            let scripts = script();
            assert!(scripts.base.end_of_turn.is_none());
            assert!(scripts.radiant.end_of_turn.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn r346_deals_1_to_every_unit_on_both_sides_through_armor() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WHIRLWIND, FILLER], "field": [{ "def": BODY, "lane": 2 }] },
                    "p2": { "hand": [FILLER], "field": [{ "def": ARMORED, "lane": 1 }, { "def": BODY, "lane": 3 }] },
                }));
                s.play(WHIRLWIND, json!({}));
                let armored = unit_or_blank(&s, P2, 1);
                s.expect_stats(&armored, json!({ "health": 6 }));
                let body = unit_or_blank(&s, P2, 3);
                s.expect_stats(&body, json!({ "health": 8 }));
                let own = unit_or_blank(&s, P1, 2);
                s.expect_stats(&own, json!({ "health": 8 }));
                assert_eq!(hits(&s), vec![1, 1, 1]);
                s.expect_health(P1, 30).expect_health(P2, 30);
            }

            #[test]
            fn r346_divine_shield_still_pops_the_shielded_unit_takes_nothing_and_loses_its_shield() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WHIRLWIND, FILLER] },
                    "p2": { "hand": [FILLER], "field": [SHIELDED] },
                }));
                s.play(WHIRLWIND, json!({}));
                let jilliax = s.unit(P2, 1).expect("gone");
                s.expect_stats(&jilliax, json!({ "health": 2 }));
                assert!(!has_keyword(&s, &jilliax.id, KeywordKind::DivineShield));
            }

            #[test]
            fn r59_every_hit_lands_before_the_state_check_both_1_health_units_die_in_one_check() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WHIRLWIND, FILLER], "field": [{ "def": BODY, "lane": 1, "damage": 8 }] },
                    "p2": { "hand": [FILLER], "field": [{ "def": BODY, "lane": 1, "damage": 8 }] },
                }));
                s.play(WHIRLWIND, json!({}));
                let types: Vec<GameEventType> = s.last_events().iter().map(|event| event.event_type()).collect();
                let last_hit = position_of(&types, GameEventType::Damage, true);
                let first_death = position_of(&types, GameEventType::Destroyed, false);
                assert!(first_death > last_hit);
                assert_eq!(types.iter().filter(|each| **each == GameEventType::Destroyed).count(), 2);
            }

            #[test]
            fn s3_2_only_the_top_of_a_stack_pile_is_hit_the_dormant_card_beneath_is_not() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WHIRLWIND, FILLER] },
                    "p2": { "hand": [FILLER], "field": [{ "def": BODY, "lane": 1 }, { "def": "core-092", "lane": 1, "stack": true }] },
                }));
                let dormant = s.state().players[P2]
                    .units
                    .first()
                    .and_then(|pile| pile.as_ref())
                    .and_then(|pile| pile.get(1))
                    .expect("no pile")
                    .id
                    .clone();
                s.play(WHIRLWIND, json!({}));
                assert_eq!(s.card(&dormant).damage, 0);
                assert_eq!(hits(&s).len(), 1);
            }

            #[test]
            fn b5_e6_spell_damage_raises_each_hit() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WHIRLWIND, FILLER], "field": [{ "def": SOLARIUS, "lane": 1 }] },
                    "p2": { "hand": [FILLER], "field": [{ "def": BODY, "lane": 1 }, { "def": ARMORED, "lane": 2 }] },
                }));
                s.play(WHIRLWIND, json!({}));
                // Solarius itself is a Unit too: three hits of 1 + 2.
                assert_eq!(hits(&s), vec![3, 3, 3]);
                let body = unit_or_blank(&s, P2, 1);
                s.expect_stats(&body, json!({ "health": 6 }));
                let armored = unit_or_blank(&s, P2, 2);
                s.expect_stats(&armored, json!({ "health": 4 }));
            }

            #[test]
            fn s6_1_an_immune_to_spells_unit_takes_none() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WHIRLWIND, FILLER] },
                    "p2": { "hand": [FILLER], "field": [{ "def": TOP_LOSER, "lane": 1, "radiant": true }, { "def": BODY, "lane": 2 }] },
                }));
                s.play(WHIRLWIND, json!({}));
                let immune = unit_or_blank(&s, P2, 1);
                s.expect_stats(&immune, json!({ "health": 10 }));
                let body = unit_or_blank(&s, P2, 2);
                s.expect_stats(&body, json!({ "health": 8 }));
            }

            #[test]
            fn r386_the_damage_reads_through_param_an_upgrade_makes_it_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WHIRLWIND, FILLER] },
                    "p2": { "hand": [FILLER], "field": [BODY] },
                }));
                step_param(s.card_mut(WHIRLWIND), "damage", 1);
                s.play(WHIRLWIND, json!({}));
                let body = unit_or_blank(&s, P2, 1);
                s.expect_stats(&body, json!({ "health": 7 }));
            }

            #[test]
            fn the_base_face_stays_in_the_graveyard_at_the_end_of_the_turn() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WHIRLWIND, FILLER], "library": DECK },
                    "p2": { "hand": [FILLER], "library": DECK },
                }));
                let card = s.card(WHIRLWIND).clone();
                s.play(WHIRLWIND, json!({})).end_turn();
                s.expect_in_zone(&card, "graveyard");
            }

            #[test]
            fn a_reborn_unit_it_kills_comes_back_at_1_health() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WHIRLWIND, WHIRLWIND, FILLER] },
                    "p2": { "hand": [FILLER], "field": [{ "def": ONE, "lane": 2 }] },
                }));
                let first = s.unit(P2, 2).expect("setup");
                // The defender's Divine Shield takes the first Whirlwind; the second kills it and Reborn returns it.
                let opening = s.hand(P1).first().map_or_else(|| WHIRLWIND.to_string(), |card| card.id.clone());
                s.play(opening, json!({}));
                assert_eq!(s.unit(P2, 2).map(|unit| unit.id), Some(first.id.clone()));
                let second = s
                    .hand(P1)
                    .into_iter()
                    .find(|card| card.def_id == WHIRLWIND)
                    .map_or_else(|| WHIRLWIND.to_string(), |card| card.id);
                s.play(second, json!({}));
                assert!(s.last_events().iter().any(|event| matches!(
                    event,
                    GameEvent::Destroyed { instance_id, .. } if *instance_id == first.id
                )));
                let back = s.unit(P2, 2);
                assert_eq!(back.as_ref().map(|unit| unit.def_id.as_str()), Some(ONE));
                let back = back.map(|unit| unit.id).unwrap_or_default();
                s.expect_stats(&back, json!({ "health": 1 }));
                assert!(!has_keyword(&s, &back, KeywordKind::Reborn));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r68_returns_from_the_graveyard_to_its_owner_s_hand_at_the_end_of_the_turn_it_was_played() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": WHIRLWIND, "radiant": true }, FILLER], "library": DECK },
                    "p2": { "hand": [FILLER], "field": [BODY], "library": DECK },
                }));
                let card = s.card(WHIRLWIND).clone();
                s.play(WHIRLWIND, json!({}));
                s.expect_in_zone(&card, "graveyard");
                let body = unit_or_blank(&s, P2, 1);
                s.expect_stats(&body, json!({ "health": 8 }));
                s.end_turn();
                s.expect_in_zone(&card, "hand");
                assert!(s.card(&card).radiant);
            }

            #[test]
            fn r153_a_radiant_whirlwind_that_reached_the_graveyard_without_being_played_stays_there() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "graveyard": [{ "def": WHIRLWIND, "radiant": true }], "library": DECK },
                    "p2": { "hand": [FILLER], "library": DECK },
                }));
                let card = s.card(WHIRLWIND).clone();
                s.end_turn();
                s.expect_in_zone(&card, "graveyard");
            }

            #[test]
            fn s2_4_r317_a_full_hand_burns_it_on_the_way_back() {
                crate::register_all();
                let cap = usize::try_from(HAND_CAP).expect("a hand cap");
                let mut hand = vec![json!({ "def": WHIRLWIND, "radiant": true })];
                hand.extend(std::iter::repeat_n(json!(FILLER), cap - 1));
                let mut s = scenario(json!({
                    "p1": { "hand": hand, "library": DECK },
                    "p2": { "hand": [FILLER], "library": DECK },
                }));
                let card = s.card(WHIRLWIND).clone();
                s.play(WHIRLWIND, json!({}));
                // Stockpile's "Draw 2" refills the hand to the cap before the end of the turn.
                s.play(FILLER, json!({}));
                assert_eq!(s.hand(P1).len(), cap);
                s.end_turn();
                assert_eq!(s.card(&card).zone.z(), ZoneName::Graveyard);
                assert!(s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::Burned { instance_id, .. } if *instance_id == card.id
                )));
            }
        }
    }
}
