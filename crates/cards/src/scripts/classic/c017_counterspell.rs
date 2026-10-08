//! C #17 Counterspell (SPEC §8.6 row 17). Trap, cost 2, Common.
//!   Base:    "Activates when your opponent plays a Spell: Counter it."
//!   Radiant: "Activates when your opponent plays a Spell: Counter it. Add a copy of it to your hand.
//!             The copy costs ({setCost})."
//!
//! Counter (§6.3, B5 E1, R448) answers the opponent's `cardAnnounced` of a Spell — the Spell type, so a
//! Field Spell, a Trap or a Unit leaves it set — in §10.5's announce window, before the card moves: a
//! play or a cast (a cast is a play and is announced too, R70). The countered Spell never resolves and
//! goes to its owner's graveyard; it is treated as never played (no spell script, no `cardPlayed` or
//! `cardResolved`, not counted by the turn's or the game's plays, Combo, Quickstriker or Ceaseless
//! Void, no Echo repeats), and the mana and Tributes paid for it stay spent. With two Counters set, the
//! first to resolve cancels the play and the other finds no live announce — the window never offers it
//! the cancelled card, so it stays set (R448). A Trap is consumed when it fires (§5.1).
//!
//! The condition lives in `when`, never in an early return from `run` (R99, R61): a Unit play must
//! leave the trap armed and face-down.
//!
//! Radiant: a fresh copy of the countered Spell (its Radiant flag kept, R57) goes to your hand as your
//! card, costing ({setCost}) (`costOverride`, which persists in every zone, R78); a full hand burns it
//! (R317), and in your hand the opponent's view names it no more (R97).

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-017";

/// TS `Extract<GameEvent, { type: "cardAnnounced" }>`: the fields of the announce this card reads.
struct Announced {
    instance_id: String,
    def_id: String,
}

/// "When your opponent plays a Spell": the announce of an opponent's Spell-type play or cast.
fn opponents_spell(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<Announced> {
    let GameEvent::CardAnnounced { player, instance_id, def_id, card_type, .. } = event else {
        return None;
    };
    if *player == ctx.controller {
        return None;
    }
    if *card_type == CardType::Spell {
        Some(Announced { instance_id: instance_id.clone(), def_id: def_id.clone() })
    } else {
        None
    }
}

fn counter_it(ctx: &EffectContext<'_>, announced: &Announced, copy: bool) -> Vec<Effect> {
    let countered = counter_play(json_as(json!({
        "target": { "of": "instance", "instanceId": announced.instance_id },
    })));
    if !copy {
        return vec![countered];
    }
    // Read before the counter moves it: the copy keeps the countered card's face (R57) and, a copy of a
    // Chinese card being Chinese, its language (R1300).
    let countered_card = find_instance(ctx.state, &announced.instance_id);
    let radiant = countered_card.is_some_and(|card| card.radiant);
    let mut args = json!({
        "defId": announced.def_id,
        "radiant": radiant,
        "costOverride": param(ctx, "setCost"),
    });
    if countered_card.is_some_and(|card| card.chinese == Some(true)) {
        args["chinese"] = json!(true);
    }
    vec![countered, add_to_hand(json_as(args))]
}

fn counterspell(copy: bool) -> TriggerDef {
    TriggerDef::new("counterspell", &[GameEventType::CardAnnounced], move |ctx, event| {
        match opponents_spell(ctx, event) {
            None => vec![],
            Some(announced) => counter_it(ctx, &announced, copy),
        }
    })
    .with_when(|ctx, event| opponents_spell(ctx, event).is_some())
}

pub fn script() -> CardScripts {
    let base = Script { triggers: vec![counterspell(false)], ..Script::default() };

    let radiant = Script { triggers: vec![counterspell(true)], ..Script::default() };
    CardScripts { base, radiant }
}

// C #17 Counterspell — SPEC §8.6 row 17, BUILD M9 Classic row C 17: "Face-down (R33); fires on the
// opponent's announce of a Spell (the Spell type; a Field Spell, Trap or Unit leaves it set), a cast by
// an effect included (R70), before the card moves (§10.5); the Spell never resolves and goes to its
// owner's graveyard, treated as never played: no spell script, no `cardPlayed` or `cardResolved`, not
// counted by the turn's or the game's plays, Combo, Quickstriker or Ceaseless Void, and no Echo
// repeats; its mana and Tributes stay spent; with two counters set the first to resolve cancels it and
// the other stays set; your own Spells never fire it; `cardAnnounced` and `countered` name only what
// `cardPlayed` would; radiant: also a fresh copy of the Spell (its Radiant flag kept) in your hand,
// yours, that costs (0) (`costOverride`), burned at a full hand (R317) and never named in the opponent's
// view once there (R97); its tuned number (radiant cost) reads through `param()` (R386)".
//
// Every case sets the trap face-down in p1's backrow and makes p2 the active player: a Trap answers the
// OPPONENT's play. The Spell under test is Stockpile (core-005, "Draw 2. Heal your hero 2."), whose
// resolution would be visible in p2's hand and health.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const COUNTER: &str = "classic-017";
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const REPLENISH: &str = "core-010"; // (0) Spell, Combo 3.
    const HINDER: &str = "core-021"; // (0) Spell, cast on draw.
    const TWINSPELL: &str = "core-079"; // (2) Field Spell: your next Spell gains Echo +1.
    const BEAR: &str = "core-060"; // (1) Trap.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019";

    /// TS `armed(radiantFace = false, lane = 2)`.
    fn armed(radiant_face: bool, lane: i32) -> Value {
        json!({ "def": COUNTER, "radiant": radiant_face, "faceUp": false, "lane": lane })
    }

    use crate::merged;

    /// TS `setup(p1 = {}, p2 = {}, radiantFace = false)`.
    fn setup(p1: Value, p2: Value, radiant_face: bool) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": merged(json!({ "hand": [VANILLA], "backrow": [armed(radiant_face, 2)], "library": [VANILLA, VANILLA] }), p1),
            "p2": merged(json!({ "hand": [STOCKPILE, REPLENISH], "library": [VANILLA, VANILLA, VANILLA, VANILLA], "health": 20 }), p2),
        }))
    }

    fn count(events: &[GameEvent], kind: GameEventType) -> usize {
        events.iter().filter(|event| event.event_type() == kind).count()
    }

    /// TS `toMatchObject`: every key of `expected` is equal in `actual`, objects compared key by key.
    fn assert_subset(actual: &Value, expected: &Value) {
        match (actual, expected) {
            (Value::Object(actual_map), Value::Object(expected_map)) => {
                for (key, want) in expected_map {
                    let Some(got) = actual_map.get(key) else {
                        panic!("{actual} has no key {key}");
                    };
                    assert_subset(got, want);
                }
            }
            _ => assert_eq!(actual, expected),
        }
    }

    fn copy_in_hand(s: &Scenario, player: PlayerId) -> Option<CardInstance> {
        s.hand(player).into_iter().find(|card| card.def_id == STOCKPILE)
    }

    #[test]
    fn is_a_trap_whose_condition_lives_in_when_r99() {
        assert_eq!(crate::card_def(COUNTER).type_, CardType::Trap);
        let CardScripts { base, radiant } = script();
        assert!(base.triggers.first().is_some_and(|trigger| trigger.when.is_some()));
        assert!(radiant.triggers.first().is_some_and(|trigger| trigger.when.is_some()));
    }

    mod base {
        use super::*;

        #[test]
        fn r33_it_is_face_down_and_the_opponent_s_view_never_names_it() {
            let s = setup(json!({}), json!({}), false);
            let trap = s.card(COUNTER).clone();
            assert_eq!(trap.face_up, Some(false));
            assert!(!serde_json::to_string(&s.view(P2)).unwrap().contains(COUNTER));
        }

        #[test]
        fn r448_counters_the_opponent_s_spell_before_it_moves_it_never_resolves_and_goes_to_its_owner_s_graveyard() {
            let mut s = setup(json!({}), json!({}), false);
            let spell = s.card(STOCKPILE).clone();
            let hand_before = s.hand(P2).len();

            s.play(&spell, json!({}));

            s.expect_in_zone(&spell, "graveyard");
            assert!(s.pile(P2, "graveyard").iter().any(|card| card.id == spell.id));
            // Stockpile's script never ran: no draws, no heal.
            assert_eq!(s.hand(P2).len(), hand_before - 1);
            s.expect_health(P2, 20);
            s.expect_events(json!(["cardAnnounced", "trapFired", "countered"]));
            assert_eq!(count(s.last_events(), GameEventType::CardPlayed), 0);
            assert_eq!(count(s.last_events(), GameEventType::CardResolved), 0);
            assert_eq!(count(s.last_events(), GameEventType::Drawn), 0);
        }

        #[test]
        fn sec5_1_the_trap_is_consumed_when_it_fires() {
            let mut s = setup(json!({}), json!({}), false);
            let trap = s.card(COUNTER).clone();

            s.play(STOCKPILE, json!({}));

            s.expect_in_zone(&trap, "graveyard");
            assert!(s.backrow(P1, 2).is_none());
        }

        #[test]
        fn r448_a_countered_spell_counts_as_never_played_and_its_mana_stays_spent() {
            let mut s = setup(json!({}), json!({}), false);
            let played = s.state().players.p2.turn_log.cards_played;
            let game = s.state().counters.played;

            s.play(STOCKPILE, json!({}));

            assert_eq!(s.state().players.p2.turn_log.cards_played, played);
            assert_eq!(s.state().counters.played, game);
            s.expect_mana(P2, 3);
        }

        #[test]
        fn r448_combo_does_not_count_it_a_later_combo_reads_only_the_plays_that_were_made() {
            let mut s = setup(json!({}), json!({ "hand": [STOCKPILE, REPLENISH, VANILLA] }), false);

            s.play(VANILLA, json!({ "zone": 1 }));
            s.play(STOCKPILE, json!({}));
            s.play(REPLENISH, json!({}));

            // Rapid Replenish's Combo 3 needs three earlier plays; the Vanilla alone was played.
            assert_eq!(s.state().players.p2.turn_log.cards_played, 2);
            assert_eq!(count(s.last_events(), GameEventType::Drawn), 0);
        }

        #[test]
        fn r448_a_countered_spell_makes_no_echo_repeats() {
            let mut s = setup(json!({}), json!({ "backrow": [{ "def": TWINSPELL, "faceUp": true }] }), false);

            s.play(STOCKPILE, json!({}));

            assert_eq!(count(s.last_events(), GameEventType::Drawn), 0);
            assert_eq!(count(s.last_events(), GameEventType::CardResolved), 0);
            s.expect_health(P2, 20);
        }

        #[test]
        fn r70_a_spell_cast_by_an_effect_is_announced_and_countered_too_a_cast_on_draw_card() {
            // p2's start-of-turn draw finds a Radiant Hinder, which casts itself on draw (R58): the cast is
            // announced like a play, and the trap counters it before it resolves.
            let mut s = scenario(json!({
                "p1": { "hand": [VANILLA], "backrow": [armed(false, 2)], "library": [VANILLA, VANILLA] },
                "p2": { "hand": [VANILLA], "library": [{ "def": HINDER, "radiant": true }, VANILLA, VANILLA] },
            }));
            let hinder = s.card(HINDER).clone();

            s.end_turn();

            assert_eq!(s.state().active, P2);
            s.expect_in_zone(&hinder, "graveyard");
            assert_eq!(count(s.events(), GameEventType::Countered), 1);
            assert_eq!(count(s.events(), GameEventType::CardResolved), 0);
            // Hinder never resolved, so p1's next refresh is not lowered.
            assert_eq!(s.state().players.p1.mana.next_turn_mod, 0);
        }

        #[test]
        fn a_field_spell_a_trap_and_a_unit_leave_it_set() {
            let mut s = setup(json!({}), json!({ "hand": [TWINSPELL, BEAR, VANILLA] }), false);
            s.play(TWINSPELL, json!({ "zone": 1 }));
            s.play(BEAR, json!({ "zone": 3 }));
            s.play(VANILLA, json!({ "zone": 1 }));

            assert_eq!(count(s.events(), GameEventType::Countered), 0);
            assert_eq!(s.backrow(P1, 2).and_then(|card| card.face_up), Some(false));
            s.expect_in_zone(TWINSPELL, "field");
            s.expect_in_zone(VANILLA, "field");
        }

        #[test]
        fn your_own_spells_never_fire_it() {
            let mut s = scenario(json!({
                "p1": { "hand": [STOCKPILE, VANILLA], "backrow": [armed(false, 2)], "library": [VANILLA, VANILLA, VANILLA] },
                "p2": { "hand": [VANILLA] },
            }));

            s.play(STOCKPILE, json!({}));

            assert_eq!(count(s.events(), GameEventType::Countered), 0);
            assert_eq!(s.backrow(P1, 2).and_then(|card| card.face_up), Some(false));
            assert_eq!(count(s.events(), GameEventType::Drawn), 2);
        }

        #[test]
        fn r448_with_two_set_the_first_to_resolve_cancels_the_play_and_the_other_stays_set() {
            let mut s = setup(json!({ "backrow": [armed(false, 1), armed(false, 2)] }), json!({}), false);

            s.play(STOCKPILE, json!({}));

            assert_eq!(count(s.events(), GameEventType::Countered), 1);
            assert_eq!(count(s.events(), GameEventType::TrapFired), 1);
            let left: Vec<CardInstance> = [s.backrow(P1, 1), s.backrow(P1, 2)].into_iter().flatten().collect();
            assert_eq!(left.len(), 1);
            assert_eq!(left.first().and_then(|card| card.face_up), Some(false));
        }

        #[test]
        fn r97_cardannounced_and_countered_name_the_spell_as_cardplayed_would_for_a_spell_public() {
            let mut s = setup(json!({}), json!({}), false);
            let spell = s.card(STOCKPILE).clone();

            s.play(&spell, json!({}));

            let view = serde_json::to_value(s.view(P1)).unwrap();
            let seen = view["events"].as_array().cloned().unwrap_or_default();
            let Some(announced) = seen.iter().find(|event| event["type"] == "cardAnnounced") else {
                panic!("no cardAnnounced");
            };
            let Some(countered) = seen.iter().find(|event| event["type"] == "countered") else {
                panic!("no countered");
            };
            assert_subset(announced, &json!({ "instanceId": spell.id, "defId": STOCKPILE, "cardType": "Spell" }));
            assert_subset(countered, &json!({ "instanceId": spell.id, "defId": STOCKPILE, "to": "graveyard" }));
        }

        #[test]
        fn the_base_face_adds_no_copy_to_your_hand() {
            let mut s = setup(json!({}), json!({}), false);
            let before = s.hand(P1).len();

            s.play(STOCKPILE, json!({}));

            assert_eq!(s.hand(P1).len(), before);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn counters_the_spell_and_adds_a_fresh_copy_of_it_to_your_hand_yours_costing_0() {
            let mut s = setup(json!({}), json!({}), true);
            let spell = s.card(STOCKPILE).clone();

            s.play(&spell, json!({}));

            s.expect_in_zone(&spell, "graveyard");
            let copy = copy_in_hand(&s, P1);
            assert!(copy.is_some());
            let Some(copy) = copy else {
                panic!("no copy");
            };
            assert_ne!(copy.id, spell.id);
            assert_eq!(copy.owner, P1);
            assert_eq!(copy.cost_override, Some(0));
            s.expect_health(P2, 20);
        }

        #[test]
        fn r57_the_copy_keeps_the_countered_spell_s_radiant_flag() {
            let mut s = setup(json!({}), json!({ "hand": [{ "def": STOCKPILE, "radiant": true }, REPLENISH] }), true);

            s.play(STOCKPILE, json!({}));

            assert_eq!(copy_in_hand(&s, P1).map(|card| card.radiant), Some(true));
        }

        #[test]
        fn r1300_a_copy_of_a_chinese_card_is_chinese() {
            for chinese in [true, false] {
                let mut s = setup(json!({}), json!({}), true);
                let spell = s.card(STOCKPILE).id.clone();
                if chinese && let Some(card) = find_instance_mut(s.state_mut(), &spell) {
                    card.chinese = Some(true);
                }

                s.play(STOCKPILE, json!({}));

                assert_eq!(copy_in_hand(&s, P1).and_then(|card| card.chinese), chinese.then_some(true));
            }
        }

        #[test]
        fn r317_a_full_hand_burns_the_copy_into_your_graveyard() {
            let ten = vec![VANILLA; 10];
            let mut s = setup(json!({ "hand": ten }), json!({}), true);

            s.play(STOCKPILE, json!({}));

            assert_eq!(s.hand(P1).len(), 10);
            assert!(s.pile(P1, "graveyard").iter().any(|card| card.def_id == STOCKPILE));
            assert_eq!(count(s.events(), GameEventType::Burned), 1);
        }

        #[test]
        fn r97_once_in_your_hand_the_opponent_s_view_never_names_the_copy() {
            let mut s = setup(json!({}), json!({}), true);

            s.play(STOCKPILE, json!({}));

            let Some(copy) = copy_in_hand(&s, P1) else {
                panic!("no copy");
            };
            let quoted = format!("\"{}\"", copy.id);
            // The event that put it in p1's hand named it; p2's whole view (its events included) does not.
            assert!(s.events().iter().any(|event| serde_json::to_string(event).unwrap().contains(&quoted)));
            assert!(!serde_json::to_string(&s.view(P2)).unwrap().contains(&quoted));
            assert!(serde_json::to_string(&s.view(P1).you.hand).unwrap().contains(&quoted));
        }

        #[test]
        fn r386_a_degrade_of_its_set_cost_makes_the_copy_cost_1() {
            let mut s = setup(json!({}), json!({}), true);
            step_param(s.card_mut(COUNTER), "setCost", 1);

            s.play(STOCKPILE, json!({}));

            assert_eq!(copy_in_hand(&s, P1).and_then(|card| card.cost_override), Some(1));
        }

        #[test]
        fn a_unit_leaves_the_radiant_face_set_too() {
            let mut s = setup(json!({}), json!({ "hand": [VANILLA, MENACE] }), true);

            s.play(VANILLA, json!({ "zone": 1 }));

            assert_eq!(count(s.events(), GameEventType::Countered), 0);
            assert_eq!(s.backrow(P1, 2).and_then(|card| card.face_up), Some(false));
        }
    }
}
