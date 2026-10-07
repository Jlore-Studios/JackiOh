//! C #38 Jackiestan Auctioneer (SPEC §8.6 row 38). Field Trap, Human, cost 2, Rare, 4/4 → 8/8 (its unit
//! face).
//!   Both faces: "Animated
//!                Reveals when the cards a player has played in a turn reach {plays}: Summon this as a
//!                Unit.
//!                Once this has revealed: Whenever a player plays a card, draw {draw} and deal {damage}
//!                damage to the enemy hero." — plays 3 on the base face and 2 on the Radiant, damage 2 and 4.
//!
//! R395: while it is face-down only the reveal condition is live. It answers the `cardPlayed` that
//! takes any player's plays this turn to {plays} (the per-player per-turn count, which already counts
//! the play under way; a cast counts, R70; a countered card was never played, R448, so it never
//! reaches here), and then animates (Animated, B3.1, R383) in Attack Position into the unit zone in its
//! own lane, else the leftmost open, unlocked, unreserved one (R64), summoning sick; with no open unit
//! zone it stays face-up in its backrow zone. It remembers that it has revealed (`memory.activated`).
//!
//! From the next play on — never the play that set it off, as a permanent never answers its own arrival
//! (R119) — every card either player plays makes its controller draw {draw} and deals one hit of
//! {damage} from it to the enemy hero ("each enemy hero" is the multiplayer phrasing, R45), whether it is
//! animated or stuck face-up in the backrow. A later firing, while it is still in the
//! backrow and a unit zone has opened, animates it then — every firing of an Animated trap ends with
//! its animating (B3.1 rule 4) — and one already a Unit stays put (R383).
//!
//! A Field Trap is never consumed. One trigger carries both texts, so one event can never be answered
//! by both the reveal and the "whenever" (R395). The conditions live in `when` (R99).

use jackioh_engine::prelude::*;
use jackioh_engine::effects::{animate, damage, draw, remember};

pub const ID: &str = "classic-038";

/// What the Auctioneer keeps once it has revealed.
const ACTIVATED: &str = "activated";

/// TS's answer union `"activate" | "sale"` (`null` is `None`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    Activate,
    Sale,
}

fn has_activated(ctx: &EffectContext) -> bool {
    // TS `recalled(ctx, ACTIVATED) === true`.
    recalled(ctx, ACTIVATED).and_then(|value| value.as_bool()) == Some(true)
}

/// The answer this play gets: "activate" (the {plays}th play of a turn), "sale" (once activated), or none.
fn answer(ctx: &EffectContext, event: &GameEvent) -> Option<Answer> {
    // TS `type Played = Extract<GameEvent, { type: "cardPlayed" }>`: the variant itself.
    let GameEvent::CardPlayed { player, .. } = event else {
        return None;
    };
    let reached = cards_played_this_turn(&ctx.state, *player) == param(ctx, "plays");
    if has_activated(ctx) {
        return Some(Answer::Sale);
    }
    if reached { Some(Answer::Activate) } else { None }
}

fn run(ctx: &mut EffectContext, event: &GameEvent) -> Vec<Effect> {
    let kind = answer(ctx, event);
    if kind == Some(Answer::Activate) {
        return vec![
            remember(json_as(json!({ "key": ACTIVATED, "value": true }))),
            animate(Default::default()),
        ];
    }
    if kind == Some(Answer::Sale) {
        // B3.1 rule 4: every firing of an Animated trap ends with its animating, so one stuck in the
        // backrow for want of a zone steps into a unit zone as soon as one is open; a Unit stays put.
        return vec![
            draw(json_as(json!({ "count": param(ctx, "draw") }))),
            damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": param(ctx, "damage") }))),
            animate(Default::default()),
        ];
    }
    vec![]
}

/// TS `const auction: TrapTrigger` (a `TrapTrigger` is a `TriggerDef`).
fn auction() -> TriggerDef {
    TriggerDef::new("auctioneer", &[GameEventType::CardPlayed], run).with_when(|ctx, event| answer(ctx, event).is_some())
}

pub fn script() -> CardScripts {
    let base = Script {
        triggers: vec![auction()],
        ..Script::default()
    };

    // The same script: the Radiant face's 2nd play and 4 damage are its declared numbers, which `param`
    // reads off the face; its 8/8 body is printed on the catalog face.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// C #38 Jackiestan Auctioneer — SPEC §8.6 row 38, BUILD M9 Classic row C 38: "Face-down, only its
// reveal condition is live (R395): it fires when any player plays their 3rd card in a turn
// (counted per player per turn; casts count, R70; a countered card was never played); then it animates
// (R383) in Attack Position into its lane's unit zone, else the leftmost open one, summoning sick; with
// no open zone it stays face-up in the backrow; from the next play on, never the play that set it off
// (R395), whenever either player plays a card you draw 1 and deal one hit of 2 to the enemy hero,
// animated or stuck in the backrow; animated it is a Unit for every rule and keeps that trigger; the
// opponent's view never names it while face-down (R33); radiant 8/8: the 2nd card, 4 damage; its tuned
// numbers (trigger play, never below 2; draw; damage) read through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::{script, ID};
    use jackioh_engine::testkit::*;

    const AUCTION: &str = "classic-038";
    const COUNTER: &str = "classic-017"; // Counterspell: counters the opponent's Spell.
    const FILLER: &str = "core-039"; // (0) Spell Recycling Initiative: exiles itself; its effect waits for the turn's end.
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2, heal 2.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const TIMMY: &str = "core-011"; // (1) 3/3.
    const MENACE: &str = "core-019"; // 9/9.
    const HINDER: &str = "core-021"; // (0) cast on draw.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy target Unit.

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// An engine value as the JSON the TS test reads (SURFACE §5.1: the same keys and values).
    fn js<T: serde::Serialize + ?Sized>(value: &T) -> Value {
        serde_json::to_value(value).expect("an engine value serialises")
    }

    /// TS `expect(actual).toMatchObject(pattern)`: every key the pattern names matches, recursively;
    /// arrays match element for element and in length.
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

    /// `{ ...base, ...over }`: the TS object spread, the override's keys winning.
    fn spread(mut base: Value, over: Value) -> Value {
        if let (Some(into), Some(from)) = (base.as_object_mut(), over.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        base
    }

    /// TS `setAuction(radiantFace = false, lane = 2)`.
    fn set_auction(radiant_face: bool, lane: i32) -> Value {
        json!({ "def": AUCTION, "radiant": radiant_face, "faceUp": false, "lane": lane })
    }

    /// p1 sets the Auctioneer; p2 is active with four (0) Cost Spells to play. `p2`, `p1`: TS `SideSetup`
    /// overrides (`json!({})` for none); `radiant_face` is TS's default `false` unless given.
    fn setup(p2: Value, p1: Value, radiant_face: bool) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": spread(
                json!({ "hand": [VANILLA], "backrow": [set_auction(radiant_face, 2)], "library": [TIMMY, TIMMY, TIMMY, TIMMY] }),
                p1,
            ),
            "p2": spread(
                json!({ "hand": [FILLER, FILLER, FILLER, FILLER, VANILLA], "library": [VANILLA, VANILLA] }),
                p2,
            ),
        }))
    }

    /// TS `fillers(s, player = "p2")`.
    fn fillers(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.hand(player).into_iter().filter(|card| card.def_id == FILLER).collect()
    }

    /// TS `playFillers(s, n, player = "p2")`.
    fn play_fillers(s: &mut Scenario, n: usize, player: PlayerId) {
        for _ in 0..n {
            let next = fillers(s, player).first().map(|card| card.id.clone()).expect("no filler left");
            s.play(&next, json!({}));
        }
    }

    fn count(events: &[GameEvent], type_: &str) -> usize {
        events.iter().map(js).filter(|event| event["type"] == type_).count()
    }

    fn draws_by(events: &[GameEvent], player: &str) -> usize {
        events
            .iter()
            .map(js)
            .filter(|event| event["type"] == "drawn" && event["player"] == player)
            .count()
    }

    fn zone_of(s: &Scenario, card: &str) -> Value {
        js(&s.card(card).zone)
    }

    mod c_38_jackiestan_auctioneer {
        use super::*;

        #[test]
        fn is_an_animated_field_trap_both_faces_run_one_script() {
            crate::register_all();
            let def = crate::card_def(ID);
            assert_eq!(def.type_, CardType::FieldTrap);
            let kinds: Vec<Value> = def.base.keywords.iter().map(|keyword| js(keyword)["kind"].clone()).collect();
            assert_eq!(kinds, vec![json!("Animated")]);
            // TS `expect(radiant).toBe(base)`: both faces carry the one trigger.
            let scripts = script();
            for face in [&scripts.base, &scripts.radiant] {
                assert_eq!(face.triggers.len(), 1);
                assert_eq!(face.triggers[0].id, "auctioneer");
                assert_eq!(face.triggers[0].on, vec![GameEventType::CardPlayed]);
                assert!(face.triggers[0].when.is_some());
            }
        }

        mod base {
            use super::*;

            #[test]
            fn r395_face_down_only_the_reveal_condition_is_live_a_players_1st_and_2nd_plays_do_nothing() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);

                play_fillers(&mut s, 2, P2);

                assert_eq!(count(s.events(), "trapFired"), 0);
                assert_eq!(s.card(AUCTION).face_up, Some(false));
                assert_eq!(draws_by(s.events(), "p1"), 0);
            }

            #[test]
            fn r395_the_3rd_play_of_a_turn_sets_it_off_it_animates_in_attack_position_into_its_own_lane_summoning_sick() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);

                play_fillers(&mut s, 3, P2);

                let auction = s.card(AUCTION).clone();
                assert!(matches_object(&js(&auction.zone), &json!({ "z": "field", "row": "units", "lane": 2 })));
                assert_eq!(js(&s.stats(&auction).position), json!("ATK"));
                assert_eq!(auction.summoned_turn, Some(s.state().turn));
                s.expect_events(json!(["trapFired", "animated"]));
            }

            #[test]
            fn r395_the_play_that_set_it_off_draws_nothing_and_deals_nothing() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);

                play_fillers(&mut s, 3, P2);

                assert_eq!(draws_by(s.events(), "p1"), 0);
                s.expect_health(P2, 30);
            }

            #[test]
            fn r395_from_the_next_play_on_any_players_play_you_draw_1_and_deal_2_to_the_enemy_hero() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);

                play_fillers(&mut s, 4, P2);

                assert_eq!(draws_by(s.last_events(), "p1"), 1);
                s.expect_health(P2, 28);
                let hit = s
                    .last_events()
                    .iter()
                    .map(js)
                    .find(|event| event["type"] == "damage")
                    .expect("a damage event");
                let auction = s.card(AUCTION).id.clone();
                assert!(matches_object(&hit, &json!({ "sourceId": auction, "amount": 2 })));
            }

            #[test]
            fn its_own_controllers_plays_count_and_answer_too() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [FILLER, FILLER, FILLER, FILLER, VANILLA],
                        "backrow": [set_auction(false, 2)],
                        "library": [TIMMY, TIMMY],
                    },
                    "p2": { "hand": [VANILLA] },
                }));

                play_fillers(&mut s, 3, P1);
                assert!(matches_object(&zone_of(&s, AUCTION), &json!({ "row": "units" })));
                play_fillers(&mut s, 1, P1);

                assert_eq!(draws_by(s.last_events(), "p1"), 1);
                s.expect_health(P2, 28);
            }

            #[test]
            fn r70_a_cast_counts_as_a_play_a_cast_on_draw_card_can_be_the_3rd() {
                crate::register_all();
                let mut s = setup(
                    json!({ "hand": [FILLER, STOCKPILE, VANILLA], "library": [{ "def": HINDER, "radiant": true }, VANILLA] }),
                    json!({}),
                    false,
                );

                let first = fillers(&s, P2).first().map(|card| card.id.clone()).unwrap_or_else(|| FILLER.to_string());
                s.play(&first, json!({}));
                // Stockpile is the 2nd play; its first draw casts Hinder, the 3rd.
                s.play(STOCKPILE, json!({}));

                assert!(count(s.events(), "trapFired") >= 1);
                assert!(matches_object(&zone_of(&s, AUCTION), &json!({ "row": "units" })));
            }

            #[test]
            fn r448_a_countered_card_was_never_played_it_does_not_count() {
                crate::register_all();
                let mut s = setup(
                    json!({ "hand": [FILLER, FILLER, STOCKPILE, VANILLA] }),
                    json!({ "backrow": [set_auction(false, 2), { "def": COUNTER, "faceUp": false, "lane": 4 }] }),
                    false,
                );

                play_fillers(&mut s, 2, P2);
                s.play(STOCKPILE, json!({}));

                assert_eq!(count(s.events(), "countered"), 1);
                assert_eq!(s.card(AUCTION).face_up, Some(false));
            }

            #[test]
            fn the_count_is_per_player_per_turn_plays_across_two_turns_never_add_up() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER, FILLER, VANILLA], "backrow": [set_auction(false, 2)], "library": [TIMMY, TIMMY] },
                    "p2": { "hand": [FILLER, VANILLA], "library": [VANILLA] },
                }));

                play_fillers(&mut s, 2, P1);
                s.end_turn();
                play_fillers(&mut s, 1, P2);

                assert_eq!(count(s.events(), "trapFired"), 0);
            }

            #[test]
            fn r383_no_open_unit_zone_it_stays_face_up_in_the_backrow_and_still_sells() {
                crate::register_all();
                let full = json!([VANILLA, VANILLA, VANILLA, VANILLA, VANILLA]);
                let mut s = setup(json!({}), json!({ "field": full }), false);

                play_fillers(&mut s, 4, P2);

                let auction = s.card(AUCTION).clone();
                assert!(matches_object(&js(&auction.zone), &json!({ "z": "field", "row": "backrow", "lane": 2 })));
                assert_eq!(auction.face_up, Some(true));
                assert_eq!(draws_by(s.last_events(), "p1"), 1);
                s.expect_health(P2, 28);
            }

            #[test]
            fn r383_its_own_lanes_unit_zone_taken_it_animates_into_the_leftmost_open_one() {
                crate::register_all();
                let mut s = setup(
                    json!({}),
                    json!({ "field": [{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 1 }] }),
                    false,
                );

                play_fillers(&mut s, 3, P2);

                assert!(matches_object(&zone_of(&s, AUCTION), &json!({ "z": "field", "row": "units", "lane": 3 })));
            }

            #[test]
            fn r383_stuck_in_the_backrow_a_later_firing_animates_it_once_a_unit_zone_has_opened() {
                crate::register_all();
                // p1's row is full, so the 3rd play leaves it face-up in the backrow. Hit Job then empties
                // p1's lane 2 (its own firing on that play finds the row still full); the next play's firing
                // animates it into that zone.
                let full = json!([VANILLA, VANILLA, VANILLA, VANILLA, VANILLA]);
                let mut s = setup(json!({ "hand": [FILLER, FILLER, FILLER, FILLER, HIT_JOB] }), json!({ "field": full }), false);
                play_fillers(&mut s, 3, P2);
                assert!(matches_object(&zone_of(&s, AUCTION), &json!({ "row": "backrow", "lane": 2 })));
                let doomed = s.unit(P1, 2).expect("fixture").id.clone();

                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": doomed }] }));
                s.expect_in_zone(&doomed, "graveyard");
                assert!(matches_object(&zone_of(&s, AUCTION), &json!({ "row": "backrow", "lane": 2 })));
                play_fillers(&mut s, 1, P2);

                assert!(matches_object(&zone_of(&s, AUCTION), &json!({ "z": "field", "row": "units", "lane": 2 })));
                assert_eq!(count(s.events(), "animated"), 1);
            }

            #[test]
            fn r383_animated_it_is_a_unit_for_every_rule_and_keeps_its_trigger() {
                crate::register_all();
                let mut s = setup(json!({ "field": [MENACE] }), json!({}), false);
                play_fillers(&mut s, 3, P2);
                let auction = s.card(AUCTION).id.clone();

                s.attack(MENACE, &auction);

                s.expect_in_zone(&auction, "graveyard");
            }

            #[test]
            fn r33_the_opponents_view_never_names_it_while_face_down() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                play_fillers(&mut s, 2, P2);
                assert!(!serde_json::to_string(&s.view(P2)).expect("a view serialises").contains(AUCTION));
            }

            #[test]
            fn r386_a_degrade_of_its_trigger_play_makes_it_wait_for_the_4th() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                step_param(s.card_mut(AUCTION), "plays", 1);

                play_fillers(&mut s, 3, P2);
                assert_eq!(s.card(AUCTION).face_up, Some(false));
                play_fillers(&mut s, 1, P2);

                assert!(matches_object(&zone_of(&s, AUCTION), &json!({ "row": "units" })));
            }

            #[test]
            fn r386_an_upgrade_of_its_draw_and_damage_2_cards_and_3_damage_per_play() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                step_param(s.card_mut(AUCTION), "draw", 1);
                step_param(s.card_mut(AUCTION), "damage", 1);

                play_fillers(&mut s, 4, P2);

                assert_eq!(draws_by(s.last_events(), "p1"), 2);
                s.expect_health(P2, 27);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r275_its_unit_face_is_an_8_8_and_it_reveals_on_the_2nd_play() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), true);

                play_fillers(&mut s, 2, P2);

                s.expect_stats(AUCTION, json!({ "attack": 8, "health": 8, "maxHealth": 8 }));
                assert!(matches_object(&zone_of(&s, AUCTION), &json!({ "row": "units" })));
                s.expect_health(P2, 30);
            }

            #[test]
            fn then_each_play_draws_1_and_deals_4() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), true);

                play_fillers(&mut s, 3, P2);

                assert_eq!(draws_by(s.last_events(), "p1"), 1);
                s.expect_health(P2, 26);
            }

            #[test]
            fn r386_an_upgrade_never_takes_its_trigger_play_below_2() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), true);
                step_param(s.card_mut(AUCTION), "plays", -1);

                play_fillers(&mut s, 1, P2);
                assert_eq!(s.card(AUCTION).face_up, Some(false));
                play_fillers(&mut s, 1, P2);

                assert!(matches_object(&zone_of(&s, AUCTION), &json!({ "row": "units" })));
            }
        }
    }
}
