//! C+ #18 Gullible Treatler (SPEC §8.7 row 18): at its controller's start of turn it Tributes itself
//! when its controller (Radiant: any player) controls no Field Spell, Trap or Field Trap in the backrow.

use jackioh_engine::effects::sacrifice;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-018";

/// "Field Spell or Trap": a Field Trap is a Trap too (§8 Conventions).
const COUNTED: &[CardType] = &[CardType::FieldSpell, CardType::Trap, CardType::FieldTrap];

fn controls_field_spell_or_trap(state: &GameState, player: PlayerId) -> bool {
    slots_of(player, Row::Backrow).iter().any(|slot| match card_at(state, slot) {
        Some(card) => COUNTED.contains(&card_type_of(state, card)),
        None => false,
    })
}

/// Base: its controller controls none. Radiant: no player controls one.
fn tributes_itself(state: &GameState, controller: PlayerId, radiant: bool) -> bool {
    let players: Vec<PlayerId> = if radiant { PLAYER_IDS.to_vec() } else { vec![controller] };
    !players
        .iter()
        .any(|&player| controls_field_spell_or_trap(state, player))
}

fn treatler(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |ctx| {
            if ctx.self_.is_some() && tributes_itself(&*ctx.state, ctx.controller, radiant) {
                vec![sacrifice(json_as(json!({ "target": { "of": "self" } })))]
            } else {
                Vec::new()
            }
        })),
        condition_met: Some(condition_hook(move |ctx| {
            ctx.zone == ConditionZone::Field && tributes_itself(ctx.state, ctx.controller, radiant)
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: treatler(false),
        radiant: treatler(true),
    }
}

// C+ #18 Gullible Treatler — SPEC §8.7 row 18, BUILD M9 Classic+ row C+ 18: "At its controller's start
// of turn, if they control no Field Spell, Trap or Field Trap (face-down cards count; an animated one
// standing in a unit zone is a Unit and doesn't, §6.1), it Tributes itself: a Sacrifice that bypasses
// Indestructible and counts as a death; with one it stays; nothing at the opponent's start of turn;
// `conditionMet` on the field answers whether it would be Tributed now (R195); radiant only when no
// player controls one".
//
// R195's proofs for this card are in its own file (the `describe("R195 conditionMet …")` block).

/// `describe("C+ #18 Gullible Treatler")`.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TREATLER: &str = "classicplus-018";
    const FIELD_SPELL: &str = "core-064"; // Gifted Program
    const TRAP: &str = "core-060"; // Bear Honeypot
    const FIELD_TRAP: &str = "core-018"; // Bread and Butter
    const FROSTSPATULA: &str = "classicplus-012-8"; // a Field Spell, "Animated on your turn"
    const TOWER: &str = "classicplus-033"; // Ivory Tower, a Field Spell that fuses the first Unit stacked onto it
    const VANILLA: &str = "core-008";
    const FILLER: &str = "core-005";
    const DECK: [&str; 4] = ["core-005", "core-005", "core-005", "core-005"];

    use crate::scenario;

    use crate::js;

    /// TS `{ ...base, ...over }` on two object literals.
    fn merged(mut base: Value, over: Value) -> Value {
        if let (Some(into), Some(from)) = (base.as_object_mut(), over.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        base
    }

    use crate::matches_object;

    fn board(p1: Value, p2: Value, active: PlayerId) -> Scenario {
        scenario(json!({
            "active": active,
            "p1": merged(json!({ "hand": [FILLER], "library": DECK }), p1),
            "p2": merged(json!({ "hand": [FILLER], "library": DECK }), p2),
        }))
    }

    fn treatler_at(s: &Scenario) -> Option<CardInstance> {
        s.unit(P1, 1).filter(|unit| unit.def_id == TREATLER)
    }

    /// p1's Treatler in lane 1, face as given, plus p1's other field and p2's.
    fn with_treatler(radiant_face: bool, p1_field: Vec<Value>, p2: Value, active: PlayerId) -> Scenario {
        let mut field = vec![json!({ "def": TREATLER, "lane": 1, "radiant": radiant_face })];
        field.extend(p1_field);
        board(json!({ "field": field }), p2, active)
    }

    fn glows(s: &Scenario) -> bool {
        js(&s.view(P1))["you"]["units"][0]["conditionActive"] == json!(true)
    }

    /// is a (2) 8/9 Human Unit with a start-of-turn hook and no play-time choice
    #[test]
    fn is_a_2_8_9_human_unit_with_a_start_of_turn_hook_and_no_play_time_choice() {
        crate::register_all();
        let def = crate::card_def(TREATLER);
        assert_eq!(def.id, TREATLER);
        assert_eq!(js(&def.cost), json!(2));
        assert!(def.tags.contains(&Tag::Human));
        let scripts = script();
        assert!(scripts.base.start_of_turn.is_some());
        assert!(scripts.radiant.start_of_turn.is_some());
        assert!(scripts.base.targets.is_empty());
    }

    /// `describe("base")`.
    mod base {
        use super::*;

        /// with no Field Spell or Trap of yours, it Tributes itself at your start of turn: a death with no killer
        #[test]
        fn with_no_field_spell_or_trap_of_yours_it_tributes_itself_at_your_start_of_turn_a_death_with_no_killer() {
            let mut s = with_treatler(false, vec![], json!({ "backrow": [{ "def": FIELD_SPELL }] }), P1);
            let treatler = treatler_at(&s).expect("no Treatler");
            let destroyed_before = s.state().counters.destroyed;
            s.start_turn();
            s.expect_in_zone(&treatler, "graveyard");
            let death = s
                .last_events()
                .iter()
                .find(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == treatler.id))
                .cloned();
            assert!(matches_object(&js(&death), &json!({ "type": "destroyed", "killerId": null })));
            assert_eq!(s.state().counters.destroyed, destroyed_before + 1);
        }

        /// a Field Spell of yours keeps it
        #[test]
        fn a_field_spell_of_yours_keeps_it() {
            let mut s = with_treatler(false, vec![json!({ "def": FIELD_SPELL, "row": "backrow" })], json!({}), P1);
            s.start_turn();
            assert!(treatler_at(&s).is_some());
        }

        /// a face-down Trap of yours keeps it
        #[test]
        fn a_face_down_trap_of_yours_keeps_it() {
            let mut s = with_treatler(false, vec![json!({ "def": TRAP, "row": "backrow", "faceUp": false })], json!({}), P1);
            s.start_turn();
            assert!(treatler_at(&s).is_some());
        }

        /// a Field Trap counts as a Trap
        #[test]
        fn a_field_trap_counts_as_a_trap() {
            let mut s = with_treatler(false, vec![json!({ "def": FIELD_TRAP, "row": "backrow", "faceUp": false })], json!({}), P1);
            s.start_turn();
            assert!(treatler_at(&s).is_some());
        }

        /// R383 an animated Field Spell standing in a unit zone is a Unit and does not count
        #[test]
        fn r383_an_animated_field_spell_standing_in_a_unit_zone_is_a_unit_and_does_not_count() {
            // Frostspatula animates at its controller's start of turn, before the start-of-turn triggers.
            let mut s = with_treatler(false, vec![json!({ "def": FROSTSPATULA, "row": "backrow", "lane": 3 })], json!({}), P1);
            let treatler = treatler_at(&s).expect("no Treatler");
            s.start_turn();
            let animated = (1..=5).any(|lane| s.unit(P1, lane).is_some_and(|unit| unit.def_id == FROSTSPATULA));
            assert!(animated);
            s.expect_in_zone(&treatler, "graveyard");
        }

        /// R418 an Ivory Tower that has fused a Unit in is still a Field Spell you control: it keeps it
        #[test]
        fn r418_an_ivory_tower_that_has_fused_a_unit_in_is_still_a_field_spell_you_control_it_keeps_it() {
            let mut s = board(
                json!({ "hand": [VANILLA, FILLER], "field": [{ "def": TREATLER, "lane": 1 }], "backrow": [{ "def": TOWER, "lane": 2 }] }),
                json!({}),
                P1,
            );
            let tower = s.card(TOWER).id.clone();
            let rider = s.card(VANILLA).id.clone();
            s.play(&rider, json!({ "zone": 2, "row": "backrow" }));
            // R653: the Vanilla is fused into the Tower, which stays a Field Spell in its backrow zone.
            s.expect_in_zone(&rider, "gone");
            assert_eq!(s.backrow(P1, 2).map(|card| card.id), Some(tower));
            assert!(!glows(&s));
            s.end_turn().end_turn();
            assert!(treatler_at(&s).is_some());
        }

        /// §6.3 the Tribute is a Sacrifice: Indestructible does not save it
        #[test]
        fn s6_3_the_tribute_is_a_sacrifice_indestructible_does_not_save_it() {
            let mut s = with_treatler(false, vec![], json!({}), P1);
            let treatler = treatler_at(&s).expect("no Treatler");
            find_instance_mut(s.state_mut(), &treatler.id)
                .expect("the Treatler is on the field")
                .granted_keywords
                .push(Keyword::Indestructible);
            assert!(
                s.stats(&treatler.id)
                    .keywords
                    .iter()
                    .any(|keyword| keyword.kind() == KeywordKind::Indestructible)
            );
            s.start_turn();
            s.expect_in_zone(&treatler, "graveyard");
        }

        /// an enemy Field Spell does not keep the base face
        #[test]
        fn an_enemy_field_spell_does_not_keep_the_base_face() {
            let mut s = with_treatler(
                false,
                vec![],
                json!({ "backrow": [{ "def": FIELD_SPELL }, { "def": TRAP, "faceUp": false }] }),
                P1,
            );
            let treatler = treatler_at(&s).expect("no Treatler");
            s.start_turn();
            s.expect_in_zone(&treatler, "graveyard");
        }

        /// nothing happens at the opponent's start of turn
        #[test]
        fn nothing_happens_at_the_opponents_start_of_turn() {
            let mut s = with_treatler(false, vec![], json!({}), P2);
            s.start_turn();
            assert!(treatler_at(&s).is_some());
        }
    }

    /// `describe("radiant")`.
    mod radiant {
        use super::*;

        /// an enemy Field Spell or face-down Trap keeps it
        #[test]
        fn an_enemy_field_spell_or_face_down_trap_keeps_it() {
            let mut spell = with_treatler(true, vec![], json!({ "backrow": [{ "def": FIELD_SPELL }] }), P1);
            spell.start_turn();
            assert!(treatler_at(&spell).is_some());

            let mut trap = with_treatler(true, vec![], json!({ "backrow": [{ "def": TRAP, "faceUp": false }] }), P1);
            trap.start_turn();
            assert!(treatler_at(&trap).is_some());
        }

        /// with no player controlling one, it Tributes itself
        #[test]
        fn with_no_player_controlling_one_it_tributes_itself() {
            let mut s = with_treatler(true, vec![], json!({}), P1);
            let treatler = treatler_at(&s).expect("no Treatler");
            s.expect_stats(&treatler, json!({ "attack": 16, "health": 18 }));
            s.start_turn();
            s.expect_in_zone(&treatler, "graveyard");
        }

        /// a Field Spell of its own keeps it too
        #[test]
        fn a_field_spell_of_its_own_keeps_it_too() {
            let mut s = with_treatler(true, vec![json!({ "def": FIELD_SPELL, "row": "backrow" })], json!({}), P1);
            s.start_turn();
            assert!(treatler_at(&s).is_some());
        }
    }

    /// `describe("R195 conditionMet on the field: whether it would be Tributed now")`.
    mod r195_condition_met_on_the_field_whether_it_would_be_tributed_now {
        use super::*;

        /// base: lit with no Field Spell or Trap of yours, and the start of turn Tributes it
        #[test]
        fn base_lit_with_no_field_spell_or_trap_of_yours_and_the_start_of_turn_tributes_it() {
            let mut s = with_treatler(false, vec![], json!({ "backrow": [{ "def": FIELD_SPELL }] }), P1);
            assert!(script().base.condition_met.is_some());
            assert!(glows(&s));
            s.end_turn().end_turn();
            assert!(treatler_at(&s).is_none());
        }

        /// base: unlit with a Trap of yours, and the start of turn keeps it
        #[test]
        fn base_unlit_with_a_trap_of_yours_and_the_start_of_turn_keeps_it() {
            let mut s = with_treatler(false, vec![json!({ "def": TRAP, "row": "backrow", "faceUp": false })], json!({}), P1);
            assert!(!glows(&s));
            s.end_turn().end_turn();
            assert!(treatler_at(&s).is_some());
        }

        /// radiant: unlit while the opponent controls one, lit when nobody does
        #[test]
        fn radiant_unlit_while_the_opponent_controls_one_lit_when_nobody_does() {
            let mut kept = with_treatler(true, vec![], json!({ "backrow": [{ "def": TRAP, "faceUp": false }] }), P1);
            assert!(!glows(&kept));
            kept.end_turn().end_turn();
            assert!(treatler_at(&kept).is_some());

            let mut gone = with_treatler(true, vec![], json!({}), P1);
            assert!(glows(&gone));
            gone.end_turn().end_turn();
            assert!(treatler_at(&gone).is_none());
        }

        /// never lit on the opponent's view, nor in the hand
        #[test]
        fn never_lit_on_the_opponents_view_nor_in_the_hand() {
            let s = board(
                json!({ "hand": [TREATLER, FILLER] }),
                json!({ "field": [{ "def": TREATLER, "lane": 1 }] }),
                P1,
            );
            let view = js(&s.view(P1));
            assert!(view["opponent"]["units"][0].get("conditionActive").is_none());
            let in_hand = view["you"]["hand"].as_array().expect("own hand is a list");
            assert!(
                in_hand
                    .iter()
                    .find(|card| card["defId"] == json!(TREATLER))
                    .and_then(|card| card.get("conditionActive"))
                    .is_none()
            );
        }
    }
}
