//! C+ #14 Forever& (SPEC §8.7 row 14, R410): (1) Spell, Epic.
//!   Base:    "The next Spell you play gains "After this resolves, return it to hand. This can't
//!            cost less than ({floor})."" — floor 2.
//!   Radiant: floor 1, no Draw (balance patch 1).
//! A player modifier that waits until used (not turn-scoped) stamps E39's enchantment on the next Spell
//! played, a cast included (`enchantNextSpell`); the enchantment rides the card in every zone and its
//! floor applies after every discount (R65). Installed as this resolves, so Forever& never stamps itself.

use jackioh_engine::effects::enchant_next_spell;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-014";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![enchant_next_spell(json_as(json!({
                "enchantment": { "kind": "returnAfterResolve", "floor": param(&*ctx, "floor") },
            })))]
        })),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(|ctx| {
            vec![enchant_next_spell(json_as(json!({
                "enchantment": { "kind": "returnAfterResolve", "floor": param(&*ctx, "floor") },
            })))]
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// C+ #14 Forever& — SPEC §8.7 row 14, R410, BUILD M9 Classic+ row C+ 14: "Leaves a waiting player
// modifier, not turn-scoped (it survives cleanup), that stamps the next Spell you play, never Forever&
// itself, with "After this resolves, return it to hand. This can't cost less than (2)": the Spell
// resolves and comes back to your hand, and does so after every later play too, the enchantment riding
// the card in every zone; its floor applies after every discount; a discarded or countered stamped
// Spell does not come back (R410); a Unit, Field Spell or Trap play leaves the modifier waiting; a full
// hand burns the returning card (R4); the floor reads through `param()` and never drops below 1;
// radiant the floor is (1) with no draw (balance patch 1)".

/// `describe("C+ #14 Forever&")`.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FOREVER: &str = "classicplus-014";
    const LUNAR: &str = "core-035"; // (1) Spell: 3 damage; your next Spell this turn costs (1) less
    const STOCKPILE: &str = "core-005"; // (1) Spell: draw 2, heal 2
    const MENACE: &str = "core-019"; // a Unit
    const MANA_WELL: &str = "core-006"; // a Field Spell
    const SHEEPISH: &str = "core-041"; // a Trap
    const RAPID_DRAW: &str = "classic-026"; // (0) Spell: draw 4, then discard 4 at random
    const COUNTERSPELL: &str = "classic-017"; // Trap: counters the opponent's Spell
    const JELLY_BEAN: &str = "core-027"; // (1) Spell, cast on draw: make a random hand card Radiant, lose 5 health
    const FILLER: &str = "core-008"; // Mr. Vanilla: a Unit, inert

    use crate::scenario;

    use crate::merged;

    /// TS `AT_HERO`.
    fn at_hero() -> Value {
        json!([{ "pick": "hero", "player": "p2" }])
    }

    fn forever(radiant: bool, hand: &[&str], p1: Value, p2: Value) -> Scenario {
        let mut cards = vec![json!({ "def": FOREVER, "radiant": radiant })];
        cards.extend(hand.iter().map(|card| json!(card)));
        let mut s = scenario(json!({
            "p1": merged(json!({ "hand": cards, "library": [STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE], "mana": 10 }), p1),
            "p2": merged(json!({ "hand": [STOCKPILE], "library": [STOCKPILE, STOCKPILE, STOCKPILE] }), p2),
        }));
        s.play(FOREVER, json!({}));
        s
    }

    fn lunar_in(s: &Scenario) -> String {
        match s.hand(P1).into_iter().find(|held| held.def_id == LUNAR) {
            Some(card) => card.id,
            None => panic!("no Lunar Eclipse in hand"),
        }
    }

    fn stamped_with(s: &Scenario, id: &str, floor: i32) -> bool {
        s.card(id)
            .enchantments
            .as_ref()
            .is_some_and(|stamps| stamps.contains(&Enchantment::ReturnAfterResolve { floor }))
    }

    fn cost_of(s: &Scenario, id: &str) -> i32 {
        effective_cost(s.state(), s.card(id), Default::default())
    }

    /// `describe("base")`.
    mod base {
        use super::*;

        /// R410 never stamps itself: Forever& goes to the graveyard and a modifier waits
        #[test]
        fn r410_never_stamps_itself_forever_goes_to_the_graveyard_and_a_modifier_waits() {
            let mut s = forever(false, &[LUNAR], json!({}), json!({}));
            s.expect_in_zone(FOREVER, "graveyard");
            assert!(!s.view(P1).you.modifiers.is_empty());
        }

        /// R410 the next Spell resolves and comes back to your hand, and again after every later play
        #[test]
        fn r410_the_next_spell_resolves_and_comes_back_to_your_hand_and_again_after_every_later_play() {
            let mut s = forever(false, &[LUNAR], json!({}), json!({}));
            let id = lunar_in(&s);
            s.play(&id, json!({ "targets": at_hero() }));
            s.expect_health(P2, HERO_HEALTH - 3);
            assert_eq!(s.card(&id).zone.z(), ZoneName::Hand);
            s.play(&id, json!({ "targets": at_hero() }));
            s.expect_health(P2, HERO_HEALTH - 6);
            assert_eq!(s.card(&id).zone.z(), ZoneName::Hand);
            assert!(stamped_with(&s, &id, 2));
        }

        /// only the next Spell: the one after is not stamped
        #[test]
        fn only_the_next_spell_the_one_after_is_not_stamped() {
            let mut s = forever(false, &[LUNAR, STOCKPILE], json!({}), json!({}));
            let stockpile = s
                .hand(P1)
                .into_iter()
                .find(|card| card.def_id == STOCKPILE)
                .map(|card| card.id)
                .unwrap_or_default();
            let lunar = lunar_in(&s);
            s.play(&lunar, json!({ "targets": at_hero() }));
            s.play(&stockpile, json!({}));
            assert_eq!(s.card(&stockpile).zone.z(), ZoneName::Graveyard);
        }

        /// the modifier waits across turns: it survives cleanup
        #[test]
        fn the_modifier_waits_across_turns_it_survives_cleanup() {
            let mut s = forever(false, &[LUNAR, MENACE], json!({}), json!({}));
            s.end_turn().end_turn();
            let id = lunar_in(&s);
            s.play(&id, json!({ "targets": at_hero() }));
            assert_eq!(s.card(&id).zone.z(), ZoneName::Hand);
        }

        /// R70 a cast is a play: a Spell cast as the next one is stamped and comes back to your hand
        #[test]
        fn r70_a_cast_is_a_play_a_spell_cast_as_the_next_one_is_stamped_and_comes_back_to_your_hand() {
            // No Spell is played: the turn's draw takes the Jelly Bean, which casts itself (R70).
            let mut s = forever(false, &[FILLER], json!({ "library": [JELLY_BEAN, STOCKPILE, STOCKPILE] }), json!({}));
            s.end_turn().end_turn();
            let jelly = s.card(JELLY_BEAN).id.clone();
            s.expect_health(P1, HERO_HEALTH - 5);
            assert_eq!(s.card(&jelly).zone.z(), ZoneName::Hand);
            assert!(stamped_with(&s, &jelly, 2));
        }

        /// a Unit, Field Spell or Trap play leaves the modifier waiting
        #[test]
        fn a_unit_field_spell_or_trap_play_leaves_the_modifier_waiting() {
            let mut s = forever(false, &[MENACE, MANA_WELL, SHEEPISH, LUNAR], json!({}), json!({}));
            s.play(MENACE, json!({}));
            s.play(MANA_WELL, json!({}));
            s.play(SHEEPISH, json!({}));
            let id = lunar_in(&s);
            s.play(&id, json!({ "targets": at_hero() }));
            assert_eq!(s.card(&id).zone.z(), ZoneName::Hand);
        }

        /// R65 its floor applies after every discount: a (1) Spell stamped costs (2), and a discount can't lower it
        #[test]
        fn r65_its_floor_applies_after_every_discount_a_1_spell_stamped_costs_2_and_a_discount_cant_lower_it() {
            let mut s = forever(false, &[LUNAR, LUNAR], json!({}), json!({}));
            let lunars: Vec<String> = s
                .hand(P1)
                .into_iter()
                .filter(|card| card.def_id == LUNAR)
                .map(|card| card.id)
                .collect();
            let (first, second) = match lunars.as_slice() {
                [first, second, ..] => (first.clone(), second.clone()),
                _ => panic!("two Lunar Eclipses"),
            };
            s.play(&first, json!({ "targets": at_hero() }));
            assert_eq!(cost_of(&s, &first), 2);
            // The second Lunar's discount (the next Spell costs (1) less) cannot take it below the floor.
            s.play(&second, json!({ "targets": at_hero() }));
            assert_eq!(cost_of(&s, &first), 2);
        }

        /// R410 a discarded stamped Spell does not come back
        #[test]
        fn r410_a_discarded_stamped_spell_does_not_come_back() {
            // Only two cards come back from the library: all three in hand go, the stamped Lunar among them.
            let mut s = forever(false, &[LUNAR, RAPID_DRAW], json!({ "library": [FILLER, FILLER] }), json!({}));
            let id = lunar_in(&s);
            s.play(&id, json!({ "targets": at_hero() })); // stamped: back in hand
            assert_eq!(s.card(&id).zone.z(), ZoneName::Hand);
            s.play(RAPID_DRAW, json!({})); // draws 2, then discards all 3 at random (R682)
            assert!(s.state().pending.is_none());
            assert_eq!(s.card(&id).zone.z(), ZoneName::Graveyard);
            s.end_turn().end_turn();
            assert_eq!(s.card(&id).zone.z(), ZoneName::Graveyard);
        }

        /// R410 a countered stamped Spell does not come back
        #[test]
        fn r410_a_countered_stamped_spell_does_not_come_back() {
            let mut s = forever(false, &[LUNAR, MENACE], json!({}), json!({ "hand": [COUNTERSPELL, STOCKPILE] }));
            s.end_turn();
            s.play(COUNTERSPELL, json!({})); // p2 sets it on their turn
            s.end_turn();
            let id = lunar_in(&s);
            s.play(&id, json!({ "targets": at_hero() }));
            assert!(
                s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Countered { instance_id, .. } if *instance_id == id))
            );
            assert_eq!(s.card(&id).zone.z(), ZoneName::Graveyard);
            s.expect_health(P2, HERO_HEALTH);
        }

        /// R4 a full hand burns the returning card
        #[test]
        fn r4_a_full_hand_burns_the_returning_card() {
            // Stockpile is the stamped Spell: with ten cards in hand, its own draws fill the hand again.
            let mut hand = vec![STOCKPILE];
            hand.extend(std::iter::repeat_n(FILLER, HAND_CAP as usize - 1));
            let mut s = forever(false, &hand, json!({}), json!({}));
            let id = s
                .hand(P1)
                .into_iter()
                .find(|card| card.def_id == STOCKPILE)
                .map(|card| card.id)
                .unwrap_or_default();
            s.play(&id, json!({}));
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            assert_eq!(s.card(&id).zone.z(), ZoneName::Graveyard);
            assert!(
                s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Burned { instance_id, .. } if *instance_id == id))
            );
        }

        /// R386 the floor reads through param() and never drops below 1
        #[test]
        fn r386_the_floor_reads_through_param_and_never_drops_below_1() {
            let mut s = scenario(json!({ "p1": { "hand": [FOREVER, LUNAR, FILLER], "mana": 10 }, "p2": { "hand": [STOCKPILE] } }));
            step_param(s.card_mut(FOREVER), "floor", -5);
            s.play(FOREVER, json!({}));
            let id = lunar_in(&s);
            s.play(&id, json!({ "targets": at_hero() }));
            assert!(stamped_with(&s, &id, 1));
            assert_eq!(cost_of(&s, &id), 1);
        }
    }

    /// `describe("radiant")`.
    mod radiant {
        use super::*;

        /// the floor is (1), with no draw
        #[test]
        fn the_floor_is_1_with_no_draw() {
            let mut s = forever(true, &[LUNAR], json!({}), json!({}));
            // Balance patch 1: the Radiant face draws nothing.
            let held: Vec<String> = s.hand(P1).into_iter().map(|card| card.def_id).collect();
            assert_eq!(held, vec![LUNAR]);
            let id = lunar_in(&s);
            s.play(&id, json!({ "targets": at_hero() }));
            assert_eq!(s.card(&id).zone.z(), ZoneName::Hand);
            assert!(stamped_with(&s, &id, 1));
            assert_eq!(cost_of(&s, &id), 1);
        }

        /// R386 the floor reads through param(): a Degrade raises the Radiant floor to (2)
        #[test]
        fn r386_the_floor_reads_through_param_a_degrade_raises_the_radiant_floor_to_2() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": FOREVER, "radiant": true }, LUNAR, FILLER], "mana": 10 },
                "p2": { "hand": [STOCKPILE] },
            }));
            step_param(s.card_mut(FOREVER), "floor", 1);
            s.play(FOREVER, json!({}));
            let id = lunar_in(&s);
            s.play(&id, json!({ "targets": at_hero() }));
            assert!(stamped_with(&s, &id, 2));
        }
    }
}
