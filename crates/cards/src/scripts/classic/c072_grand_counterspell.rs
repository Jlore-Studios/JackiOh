//! C #72 Grand Counterspell (SPEC §8.6 row 72, BUILD M9 Classic row C 72). (2) Trap, Rare.
//!   Base:    "When your opponent plays a Spell, Field Spell, Trap or Field Trap: Counter it."
//!   Radiant: "When your opponent plays a Spell, Field Spell, Trap or Field Trap: Steal it."
//!   Engine:  "Counter (§6.3), in §10.5's announce window, on the opponent's announce of any non-Unit
//!            card, so a Field Spell or a Trap is countered before it reaches the backrow: it never
//!            resolves and goes to its owner's graveyard (a face-down set is announced to you by its zone
//!            only, but the engine knows what it is). Radiant: Steal off the field (§6.3 Steal; the owner
//!            changes, §3.2, R12): the card is countered and moves to your hand as yours, where your hand
//!            cap applies (a burned card goes to your graveyard). The designer named it Counterspell, as
//!            C #17; it is renamed so the two never collide (R381), and no card names either."
//!
//! Readings:
//!   - The trigger is B5 E1's announce (`cardAnnounced`), answered in the window between §10.5
//!     steps 3 and 4 by the trap engine, which fires a face-down Trap the moment an event it watches is
//!     dispatched (§10.3) and spends it afterwards (§5.1: a Trap goes to the graveyard once it fires).
//!   - "Your opponent plays" is the announce's `player`; a cast is a play and is announced too (R70).
//!   - "A Spell, Field Spell, Trap or Field Trap" is every type but Unit, read off the announce's
//!     `cardType` — the type the card is played as (a face with its own type included, B2.7).
//!   - The counter names the announced card (`counterPlay`'s target), so a second counter answering the
//!     same announce finds no live play and stays set (B5 E1). A countered play is treated as never
//!     played: no `cardPlayed`, nothing counted, mana and Tributes spent (§6.3 Counter).
//!   - Radiant: `counterPlay({ to: "thief" })` is E2's steal off the field — the owner becomes this
//!     trap's controller (R12), the hand cap applies (R317), and the card is hidden in the opponent's
//!     view once in the thief's hand (R97).
//! No tuned numbers.

use jackioh_engine::effects::counter_play;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-072";

/// The opponent's announce of anything but a Unit (§6.3 Counter, B5 E1). `to` is TS's
/// `CounterDestination` literal ("graveyard" | "exile" | "thief"), written into the effect's argument.
fn answers(to: &'static str) -> TriggerDef {
    TriggerDef::new("grand-counter", &[GameEventType::CardAnnounced], move |_ctx, event| {
        match event {
            GameEvent::CardAnnounced { instance_id, .. } => vec![counter_play(json_as(json!({
                "to": to,
                "target": { "of": "instance", "instanceId": instance_id },
            })))],
            _ => Vec::new(),
        }
    })
    .with_when(|ctx, event| {
        matches!(
            event,
            GameEvent::CardAnnounced { player, card_type, .. }
                if *player != ctx.controller && *card_type != CardType::Unit
        )
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            triggers: vec![answers("graveyard")],
            ..Script::default()
        },
        // E2, R12: countered and taken into this trap's controller's hand, as theirs.
        radiant: Script {
            triggers: vec![answers("thief")],
            ..Script::default()
        },
    }
}

// C #72 Grand Counterspell — SPEC §8.6 row 72, BUILD M9 Classic row C 72: "Renamed from the designer's
// second Counterspell (R381); face-down (R33); fires on the opponent's announce of any non-Unit (Spell,
// Field Spell, Trap, Field Trap), a cast included (R70), and counters it before it reaches the backrow
// (a face-down set is announced by its zone only while the engine knows what it is), treated as never
// played as C #17's is, mana and Tributes spent; a Unit play leaves it set; the countered card goes to
// its owner's graveyard, where a countered trap is public; radiant: steals it instead: countered and
// moved to your hand as yours (its owner changes, R12), burned at a full hand (R317), and hidden in
// the opponent's view once there (R97); no tuned numbers".

/// `describe("C #72 Grand Counterspell")`.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GRAND: &str = "classic-072";
    const STOCKPILE: &str = "core-005"; // (1) Spell
    const ARMOR: &str = "core-073"; // (2) Field Spell
    const HONEYPOT: &str = "core-060"; // (1) Trap
    const BREAD: &str = "core-018"; // (1) Field Trap
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const CN_VIRUS: &str = "core-090-1"; // (1) Spell, Cast on draw
    const FILLER: &str = "core-010"; // (0) Spell

    use crate::scenario;

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialises")
    }

    /// TS `toMatchObject`: every key the pattern names matches, arrays element for element.
    fn matches_object(actual: &Value, pattern: &Value) -> bool {
        match (actual, pattern) {
            (Value::Object(actual), Value::Object(pattern)) => pattern
                .iter()
                .all(|(key, want)| actual.get(key).is_some_and(|got| matches_object(got, want))),
            (Value::Array(actual), Value::Array(pattern)) => {
                actual.len() == pattern.len() && actual.iter().zip(pattern).all(|(got, want)| matches_object(got, want))
            }
            _ => actual == pattern,
        }
    }

    fn countered(s: &Scenario) -> Vec<GameEvent> {
        s.last_events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Countered)
            .cloned()
            .collect()
    }

    /// p1 holds a Grand Counterspell face-down; p2 is to play.
    fn armed(radiant: bool, p2_hand: &[&str], p1_hand: Option<Vec<&str>>, p2_library: &[&str]) -> Scenario {
        let mut hand: Vec<&str> = p2_hand.to_vec();
        hand.push(FILLER);
        let p1_hand = p1_hand.unwrap_or_else(|| vec![FILLER]);
        scenario(json!({
            "p1": { "hand": p1_hand, "backrow": [{ "def": GRAND, "radiant": radiant }] },
            "p2": { "hand": hand, "library": p2_library },
            "active": "p2",
        }))
    }

    fn any_card_played(s: &Scenario) -> bool {
        s.last_events().iter().any(|event| event.event_type() == GameEventType::CardPlayed)
    }

    /// R381 is Grand Counterspell, a (2) Trap, renamed from the designer's second Counterspell; no card names it
    #[test]
    fn r381_is_grand_counterspell_a_2_trap_renamed_from_the_designers_second_counterspell_no_card_names_it() {
        crate::register_all();
        let def = crate::card_def(GRAND);
        assert_eq!(def.name, "Grand Counterspell");
        assert_eq!(crate::card_def("classic-017").name, "Counterspell");
        assert_eq!(def.type_, CardType::Trap);
        assert_eq!(js(&def.cost), json!(2));
        assert!(def.refs.is_none());
        let scripts = script();
        assert_eq!(
            scripts.base.triggers.first().map(|trigger| trigger.on.clone()),
            Some(vec![GameEventType::CardAnnounced])
        );
        assert_eq!(
            scripts.radiant.triggers.first().map(|trigger| trigger.on.clone()),
            Some(vec![GameEventType::CardAnnounced])
        );
    }

    /// `describe("base")`.
    mod base {
        use super::*;

        /// R33 it is set face-down: its controller reads it, the opponent sees a back and its cost
        #[test]
        fn r33_it_is_set_face_down_its_controller_reads_it_the_opponent_sees_a_back_and_its_cost() {
            let mut s = scenario(json!({ "p1": { "hand": [GRAND, FILLER] } }));
            s.play(GRAND, json!({}));
            let own = js(&s.view(P1))["you"]["backrow"][0].clone();
            assert!(matches_object(&own, &json!({ "defId": GRAND, "unrevealed": true })));
            let theirs = js(&s.view(P2))["opponent"]["backrow"][0].clone();
            assert!(!theirs.to_string().contains(GRAND));
            assert!(matches_object(&theirs, &json!({ "cost": 2 })));
        }

        /// §6.3 counters the opponent's Spell: no resolution, no cardPlayed, to its owner's graveyard, mana spent
        #[test]
        fn s6_3_counters_the_opponents_spell_no_resolution_no_card_played_to_its_owners_graveyard_mana_spent() {
            let mut s = armed(false, &[STOCKPILE], None, &[VANILLA, VANILLA]);
            s.play(STOCKPILE, json!({}));
            assert_eq!(countered(&s).len(), 1);
            assert!(!any_card_played(&s));
            assert!(!s.last_events().iter().any(|event| event.event_type() == GameEventType::Drawn));
            s.expect_in_zone(STOCKPILE, "graveyard").expect_mana(P2, 3);
            assert_eq!(cards_played_this_turn(s.state(), P2), 0);
            assert!(last_spell_played(s.state()).is_none());
            // A Trap fires once and is spent (§5.1).
            s.expect_in_zone(GRAND, "graveyard");
        }

        /// §6.3 counters a Field Spell before it reaches the backrow: it is never placed, its Cry never fires
        #[test]
        fn s6_3_counters_a_field_spell_before_it_reaches_the_backrow_it_is_never_placed_its_cry_never_fires() {
            let mut s = armed(false, &[ARMOR], None, &[VANILLA]);
            s.play(ARMOR, json!({}));
            assert_eq!(countered(&s).len(), 1);
            assert!(s.backrow(P2, 1).is_none());
            s.expect_in_zone(ARMOR, "graveyard");
            assert!(!s.last_events().iter().any(|event| event.event_type() == GameEventType::Drawn));
        }

        /// §6.3 R33 counters a Trap set face-down, announced by its zone only; once countered it is public in the graveyard
        #[test]
        fn s6_3_r33_counters_a_trap_set_face_down_announced_by_its_zone_only_once_countered_it_is_public_in_the_graveyard() {
            let mut s = armed(false, &[HONEYPOT], None, &[]);
            s.play(HONEYPOT, json!({}));
            assert_eq!(countered(&s).len(), 1);
            assert!(s.backrow(P2, 1).is_none());
            s.expect_in_zone(HONEYPOT, "graveyard");
            let p1_graveyard: Vec<String> = s
                .view(P1)
                .opponent
                .graveyard
                .iter()
                .map(|card| card.def_id.clone())
                .collect();
            assert!(p1_graveyard.iter().any(|id| id == HONEYPOT));
            // Its announce named only its zone to p1; the counter made it public.
            let seen = s.view(P1).events;
            let announced: Vec<&GameEvent> = seen
                .iter()
                .filter(|event| event.event_type() == GameEventType::CardAnnounced)
                .collect();
            assert_eq!(announced.len(), 1);
            assert!(!js(&announced).to_string().contains(HONEYPOT));
            let shown = seen.iter().find(|event| event.event_type() == GameEventType::Countered);
            assert!(matches_object(&js(&shown), &json!({ "defId": HONEYPOT })));
        }

        /// §6.3 counters a Field Trap too
        #[test]
        fn s6_3_counters_a_field_trap_too() {
            let mut s = armed(false, &[BREAD], None, &[]);
            s.play(BREAD, json!({}));
            assert_eq!(countered(&s).len(), 1);
            s.expect_in_zone(BREAD, "graveyard");
        }

        /// a Unit play leaves it set
        #[test]
        fn a_unit_play_leaves_it_set() {
            let mut s = armed(false, &[VANILLA], None, &[]);
            s.play(VANILLA, json!({}));
            assert!(countered(&s).is_empty());
            s.expect_in_zone(VANILLA, "field");
            assert_eq!(s.backrow(P1, 1).map(|card| card.def_id).as_deref(), Some(GRAND));
        }

        /// its controller's own plays leave it set
        #[test]
        fn its_controllers_own_plays_leave_it_set() {
            let mut s = scenario(json!({ "p1": { "hand": [STOCKPILE, FILLER], "backrow": [GRAND], "library": [VANILLA, VANILLA] } }));
            s.play(STOCKPILE, json!({}));
            assert!(countered(&s).is_empty());
            assert_eq!(s.backrow(P1, 1).map(|card| card.def_id).as_deref(), Some(GRAND));
        }

        /// R70 a cast is a play: the opponent's Spell cast on draw is countered
        #[test]
        fn r70_a_cast_is_a_play_the_opponents_spell_cast_on_draw_is_countered() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [GRAND] },
                "p2": { "hand": [FILLER], "library": [CN_VIRUS, VANILLA] },
            }));
            s.end_turn();
            assert!(
                s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Countered { def_id, .. } if def_id == CN_VIRUS))
            );
            assert!(
                !s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Damage { target_id, .. } if target_id == "hero-p2"))
            );
            s.expect_in_zone(CN_VIRUS, "graveyard");
        }

        /// B5 E1 beside C #17 Counterspell one counter cancels the Spell and the other stays set
        #[test]
        fn b5_e1_beside_c_n17_counterspell_one_counter_cancels_the_spell_and_the_other_stays_set() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [GRAND, "classic-017"] },
                "p2": { "hand": [STOCKPILE, FILLER] },
                "active": "p2",
            }));
            s.play(STOCKPILE, json!({}));
            assert_eq!(countered(&s).len(), 1);
            s.expect_in_zone(STOCKPILE, "graveyard");
            let set = [s.backrow(P1, 1), s.backrow(P1, 2)].into_iter().flatten().count();
            assert_eq!(set, 1);
        }

        /// B5 E1 the first counter cancels the play; a second Grand Counterspell finds no card and stays set
        #[test]
        fn b5_e1_the_first_counter_cancels_the_play_a_second_grand_counterspell_finds_no_card_and_stays_set() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [GRAND, GRAND] },
                "p2": { "hand": [STOCKPILE, FILLER] },
                "active": "p2",
            }));
            s.play(STOCKPILE, json!({}));
            assert_eq!(countered(&s).len(), 1);
            let set = [s.backrow(P1, 1), s.backrow(P1, 2)].into_iter().flatten().count();
            assert_eq!(set, 1);
        }
    }

    /// `describe("radiant")`.
    mod radiant {
        use super::*;

        /// E2 R12 steals the opponent's Spell: countered, and in your hand as yours
        #[test]
        fn e2_r12_steals_the_opponents_spell_countered_and_in_your_hand_as_yours() {
            let mut s = armed(true, &[STOCKPILE], None, &[]);
            s.play(STOCKPILE, json!({}));
            let cancelled = countered(&s);
            assert_eq!(cancelled.len(), 1);
            assert!(matches_object(&js(&cancelled[0]), &json!({ "to": "hand", "defId": STOCKPILE })));
            let stolen = s.card(STOCKPILE).clone();
            s.expect_in_zone(&stolen, "hand");
            assert_eq!(stolen.owner, P1);
            assert!(s.hand(P1).iter().any(|card| card.id == stolen.id));
            assert!(!any_card_played(&s));
        }

        /// steals a Field Spell before it reaches the backrow
        #[test]
        fn steals_a_field_spell_before_it_reaches_the_backrow() {
            let mut s = armed(true, &[ARMOR], None, &[]);
            s.play(ARMOR, json!({}));
            assert!(s.backrow(P2, 1).is_none());
            assert_eq!(s.card(ARMOR).owner, P1);
            s.expect_in_zone(ARMOR, "hand");
        }

        /// R97 once in your hand the stolen card is hidden in the opponent's view: a count of p1's hand, nothing more
        #[test]
        fn r97_once_in_your_hand_the_stolen_card_is_hidden_in_the_opponents_view_a_count_of_p1s_hand_nothing_more() {
            let mut s = armed(true, &[STOCKPILE], None, &[]);
            s.play(STOCKPILE, json!({}));
            let id = s.card(STOCKPILE).id.clone();
            // The steal itself was public (the opponent watched the card announced, B5 E16), but p2's view of
            // p1's hand is a count; p1 reads the card.
            assert_eq!(js(&s.view(P2).opponent.hand), json!({ "count": 2 }));
            let own = js(&s.view(P1));
            assert!(
                own["you"]["hand"]
                    .as_array()
                    .is_some_and(|cards| cards.iter().any(|card| card["instanceId"] == json!(id)))
            );
        }

        /// steals a Trap set face-down: it never reaches the backrow and is yours in hand
        #[test]
        fn steals_a_trap_set_face_down_it_never_reaches_the_backrow_and_is_yours_in_hand() {
            let mut s = armed(true, &[HONEYPOT], None, &[]);
            s.play(HONEYPOT, json!({}));
            assert!(s.backrow(P2, 1).is_none());
            assert!(matches_object(
                &js(s.card(HONEYPOT)),
                &json!({ "owner": "p1", "zone": { "z": "hand", "player": "p1" } })
            ));
        }

        /// R317 burned at your full hand: it goes to your graveyard, yours
        #[test]
        fn r317_burned_at_your_full_hand_it_goes_to_your_graveyard_yours() {
            let full_hand = vec![FILLER; 10];
            let mut s = armed(true, &[STOCKPILE], Some(full_hand), &[]);
            s.play(STOCKPILE, json!({}));
            let stolen = s.card(STOCKPILE).clone();
            s.expect_in_zone(&stolen, "graveyard");
            assert_eq!(stolen.owner, P1);
            assert!(s.pile(P1, "graveyard").iter().any(|card| card.id == stolen.id));
            assert!(s.last_events().iter().any(|event| event.event_type() == GameEventType::Burned));
        }

        /// a Unit play leaves it set, as on the base face
        #[test]
        fn a_unit_play_leaves_it_set_as_on_the_base_face() {
            let mut s = armed(true, &[VANILLA], None, &[]);
            s.play(VANILLA, json!({}));
            assert!(countered(&s).is_empty());
            assert_eq!(s.backrow(P1, 1).map(|card| card.def_id).as_deref(), Some(GRAND));
        }

        /// the stolen card is playable by its thief as their own
        #[test]
        fn the_stolen_card_is_playable_by_its_thief_as_their_own() {
            let mut s = armed(true, &[STOCKPILE], None, &[]);
            s.play(STOCKPILE, json!({}));
            s.end_turn();
            s.play(STOCKPILE, json!({}));
            assert!(s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::CardPlayed { player, def_id, .. } if *player == P1 && def_id == STOCKPILE
            )));
        }
    }
}
