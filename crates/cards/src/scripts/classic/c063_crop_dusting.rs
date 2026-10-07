//! C #63 Crop Dusting (SPEC §8.6 row 63). (2) Trap, Common.
//!   Base:    "Start of turn: Reveal. Place {tokens|Plague Counter|Plague Counters} on each
//!            permanent. Draw {draw}." — 1 counter, draw 1
//!   Radiant: the same text — 3 counters, draw 3 (the designer's Radiant face, issue #44)
//!   Engine:  "A Trap whose condition is its controller's start of turn, fired with the start-of-turn
//!            triggers (§2.2, R62); it fires once and goes to the graveyard. Each permanent on the field,
//!            both sides, face-down ones included, gets one placement (Plague Counters, §6.3) of 1
//!            (Radiant 3). Tunes: tokens 1 ↑; draw 1 ↑."
//!
//! A Trap is set face-down (R33) and fires by answering an event (§5.1, R99): this one answers its own
//! controller's `turnStarted`, so it stays set through the opponent's turn and fires at the start of its
//! controller's next one. The engine dispatches `turnStarted` to the traps at the first settle of the
//! turn (after the refresh and the Brittle tick), so it fires before the turn's draw, as R62 has every
//! start-of-turn trigger do. Firing turns it face-up and spends it to its owner's graveyard.
//!
//! "Each permanent" is one placement of {tokens} on every permanent on the field (`placePlagueEach`):
//! the top of each unit pile and every backrow card, both sides, face-down ones included, in R68's
//! order — each multiplied by the card that receives it (C #27) and each its own placement for "whenever
//! Plague Counters are placed on this" (C #53). The firing trap is one of them too (R550); it is spent to
//! the graveyard as its firing ends, and its tokens go with it (R78). A placement on a card a player may not
//! read never names it to them (R97). Then the draw.
//!
//! Both numbers are the declared `tokens` and `draw` (R386), read through `param`.

use jackioh_engine::effects::{draw, place_plague_each};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-063";

/// "At the start of your turn": its controller's `turnStarted`.
fn your_turn_starts(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    matches!(event, GameEvent::TurnStarted { player, .. } if *player == ctx.controller)
}

/// The Trap's one trigger (TS `TrapTrigger`, a `TriggerDef`).
fn dusting() -> TriggerDef {
    TriggerDef::new("crop-dusting", &[GameEventType::TurnStarted], |ctx, _event| {
        vec![
            place_plague_each(json_as(json!({
                "scope": { "side": "any", "rows": ["units", "backrow"] },
                "amount": param(ctx, "tokens"),
            }))),
            draw(json_as(json!({ "count": param(ctx, "draw") }))),
        ]
    })
    .with_when(your_turn_starts)
}

pub fn script() -> CardScripts {
    let base = Script {
        triggers: vec![dusting()],
        ..Script::default()
    };

    // The same script: the Radiant face's 3 counters and draw 3 are its declared numbers, which `param` reads
    // off the running face.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C #63 Crop Dusting — SPEC §8.6 row 63, BUILD M9 Classic row C 63: "Face-down (R33); fires at the start
// of your next turn with the start-of-turn triggers (R62) and goes to the graveyard; places 1 Plague
// Counter on each permanent on the field, both sides, face-down ones included (C #27 doubles its own),
// then draws 1; the placement on a face-down card never names it to the player who can't read it (R97);
// radiant: 3 counters each and draw 3 (the designer's Radiant face, issue #44); its tuned numbers
// (tokens, draw) read through `param()` (R386)".
//
// The C #27 Pestilent Slime case needs C #27's script (cards-classic-a) registered.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const DUSTING: &str = "classic-063";
    const CRAWLER: &str = "classic-053"; // (1) Unit: whenever Plague Counters are placed on this, draw 1.
    const SLIME: &str = "classic-027"; // (0) Unit: Plague Counters placed on this are doubled.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const FIENDER: &str = "core-092"; // (2) Unit 5/7 Stack.
    const PAWN: &str = "core-096"; // (1) Trap: answers only an attack that would be lethal.
    const MANA_WELL: &str = "core-006"; // (3) Field Spell.
    const FILLER: &str = "core-005"; // (1) Spell, a card to keep a turn from auto-ending (§2.5).
    const X: &str = "core-020"; // library filler.
    const Y: &str = "core-011"; // library filler of another kind, to tell draws apart.

    /// An engine value as the JSON the TS test reads (SURFACE §5.1: the same keys and values).
    fn js<T: serde::Serialize + ?Sized>(value: &T) -> Value {
        serde_json::to_value(value).expect("an engine value serialises")
    }

    fn lib(n: usize, card: &'static str) -> Vec<&'static str> {
        vec![card; n]
    }

    /// Each card's placements, in the order the first one reached it (TS's insertion-ordered `Map`).
    fn placed_on(s: &Scenario) -> IndexMap<String, Vec<Value>> {
        let mut out: IndexMap<String, Vec<Value>> = IndexMap::new();
        for event in s.events().iter().map(js) {
            if event["type"] != "counterChanged" || event.get("placed").is_none() {
                continue;
            }
            let instance_id = event["instanceId"].as_str().unwrap_or_default().to_string();
            out.entry(instance_id).or_default().push(event["placed"].clone());
        }
        out
    }

    fn draws_by(events: &[GameEvent], player: PlayerId) -> Vec<Value> {
        events
            .iter()
            .map(js)
            .filter(|event| event["type"] == "drawn" && event["player"] == js(&player))
            .collect()
    }

    /// p1 sets Crop Dusting on a board with permanents on both sides.
    fn set_up(radiant_face: bool) -> Scenario {
        let mut library = lib(2, Y);
        library.extend(lib(4, X));
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": DUSTING, "radiant": radiant_face }, FILLER],
                "field": [VANILLA],
                "backrow": [{ "def": MANA_WELL, "lane": 2 }],
                "library": library,
            },
            "p2": {
                "hand": [FILLER],
                "field": [MENACE],
                "backrow": [{ "def": PAWN, "faceUp": false }],
                "library": lib(4, X),
            },
        }));
        s.play(DUSTING, json!({}));
        s
    }

    mod c_63_crop_dusting {
        use super::*;

        #[test]
        fn is_a_trap_answering_its_controllers_turn_start_with_two_numbers_and_one_script_on_both_faces() {
            crate::register_all();
            let def = js(&crate::card_def(ID));
            assert_eq!(def["id"], DUSTING);
            assert_eq!(def["type"], "Trap");
            let scripts = script();
            let declared: Vec<Value> = scripts
                .base
                .triggers
                .iter()
                .map(|trigger| json!([trigger.id, trigger.on.iter().map(|on| on.as_str()).collect::<Vec<&str>>()]))
                .collect();
            assert_eq!(declared, vec![json!(["crop-dusting", ["turnStarted"]])]);
            assert_eq!(
                def["params"],
                json!([
                    { "key": "tokens", "base": 1, "radiant": 3, "better": "up", "step": 1, "min": 1 },
                    { "key": "draw", "base": 1, "radiant": 3, "better": "up", "step": 1, "min": 1 },
                ]),
            );
            // TS `expect(radiant).toBe(base)`: the Radiant face is the same declaration and the same hooks.
            assert_eq!(
                scripts.radiant.triggers.iter().map(|trigger| trigger.id.clone()).collect::<Vec<String>>(),
                scripts.base.triggers.iter().map(|trigger| trigger.id.clone()).collect::<Vec<String>>(),
            );
            assert_eq!(scripts.radiant.triggers[0].on, scripts.base.triggers[0].on);
            assert_eq!(scripts.radiant.triggers[0].when.is_some(), scripts.base.triggers[0].when.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn r33_it_is_set_face_down_the_opponent_sees_a_card_back_and_never_its_name() {
                crate::register_all();
                let s = set_up(false);
                let dusting = s.card(DUSTING).clone();

                assert_eq!(js(&dusting.zone)["z"], "field");
                assert_ne!(s.card(&dusting).face_up, Some(true));
                assert!(!js(&s.view(P2)).to_string().contains(DUSTING));
            }

            #[test]
            fn it_stays_set_through_the_opponents_turn_start_their_turnstarted_is_not_its_condition() {
                crate::register_all();
                let mut s = set_up(false);
                let dusting = s.card(DUSTING).clone();

                s.end_turn();

                assert_eq!(s.state().active, P2);
                s.expect_in_zone(&dusting, "field");
                assert!(!s.events().iter().map(js).any(|event| event["type"] == "trapFired"));
                assert_eq!(placed_on(&s).len(), 0);
            }

            #[test]
            fn r62_r550_at_the_start_of_your_next_turn_it_fires_1_token_on_each_permanent_on_both_sides_itself_included_then_draw_1_then_the_graveyard() {
                crate::register_all();
                let mut s = set_up(false);
                let dusting = s.card(DUSTING).clone();
                let vanilla = s.card(VANILLA).clone();
                let well = s.card(MANA_WELL).clone();
                let menace = s.card(MENACE).clone();
                let pawn = s.card(PAWN).clone();
                s.end_turn();

                s.end_turn();

                assert_eq!(s.state().active, P1);
                s.expect_in_zone(&dusting, "graveyard");
                assert!(
                    s.last_events()
                        .iter()
                        .map(js)
                        .any(|event| event["type"] == "trapFired" && event["instanceId"] == dusting.id.as_str())
                );
                for card in [&vanilla, &well, &menace, &pawn] {
                    assert_eq!(s.card(card).counters.plague, Some(1), "{}", card.def_id);
                }
                // The firing trap is a permanent too; its token goes with it to the graveyard (R78).
                let placed = placed_on(&s);
                assert_eq!(
                    placed.keys().cloned().collect::<Vec<String>>(),
                    vec![vanilla.id.clone(), dusting.id.clone(), well.id.clone(), menace.id.clone(), pawn.id.clone()],
                );
                assert_eq!(placed.values().cloned().collect::<Vec<Vec<Value>>>(), vec![vec![json!(1)]; 5]);
                assert!(s.card(&dusting).counters.plague.is_none());
                // Its draw (the first of the library, Y) then the turn's draw (Y): the trap fired first.
                let draws = draws_by(s.last_events(), P1);
                assert_eq!(draws.len(), 2);
                let order: Vec<Value> = s
                    .last_events()
                    .iter()
                    .map(js)
                    .filter(|event| {
                        event["type"] == "trapFired"
                            || event["type"] == "counterChanged"
                            || (event["type"] == "drawn" && event["player"] == "p1")
                    })
                    .map(|event| event["type"].clone())
                    .collect();
                let mut expected = vec![json!("trapFired")];
                expected.extend(vec![json!("counterChanged"); 5]);
                expected.extend([json!("drawn"), json!("drawn")]);
                assert_eq!(order, expected);
            }

            #[test]
            fn r97_the_placement_on_the_opponents_face_down_trap_never_names_it_to_you() {
                crate::register_all();
                let mut s = set_up(false);
                s.end_turn();
                s.end_turn();

                assert_eq!(s.card(PAWN).counters.plague, Some(1));
                assert!(!js(&s.view(P1)).to_string().contains(PAWN));
                assert!(!js(&s.view(P1)).to_string().contains("My Pawn"));
            }

            #[test]
            fn r97_nor_the_placement_on_your_own_face_down_trap_to_the_opponent() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [DUSTING, FILLER], "backrow": [{ "def": PAWN, "faceUp": false, "lane": 2 }], "library": lib(4, X) },
                    "p2": { "hand": [FILLER], "library": lib(4, X) },
                }));
                s.play(DUSTING, json!({}));
                s.end_turn();
                s.end_turn();

                assert_eq!(s.card(PAWN).counters.plague, Some(1));
                assert!(!js(&s.view(P2)).to_string().contains(PAWN));
                assert!(!js(&s.view(P2)).to_string().contains("My Pawn"));
            }

            #[test]
            fn s3_2_r13_a_card_dormant_under_a_stack_pile_takes_none_the_top_of_the_pile_does() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [DUSTING, FILLER], "library": lib(4, X) },
                    "p2": { "hand": [FILLER], "field": [VANILLA, { "def": FIENDER, "stack": true }], "library": lib(4, X) },
                }));
                s.play(DUSTING, json!({}));
                s.end_turn();
                s.end_turn();

                assert_eq!(s.card(FIENDER).counters.plague, Some(1));
                assert!(s.card(VANILLA).counters.plague.is_none());
            }

            #[test]
            fn with_no_other_permanent_on_the_field_it_still_fires_and_draws_placing_only_on_itself() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [DUSTING, FILLER], "library": lib(4, X) }, "p2": { "hand": [FILLER], "library": lib(4, X) } }));
                s.play(DUSTING, json!({}));
                s.end_turn();
                s.end_turn();

                s.expect_in_zone(DUSTING, "graveyard");
                assert_eq!(placed_on(&s).keys().cloned().collect::<Vec<String>>(), vec![s.card(DUSTING).id.clone()]);
                assert_eq!(draws_by(s.last_events(), P1).len(), 2);
            }

            #[test]
            fn each_placement_is_its_own_a_c_53_plague_crawler_on_either_side_draws_its_controller_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [DUSTING, FILLER], "field": [CRAWLER], "library": lib(5, X) },
                    "p2": { "hand": [FILLER], "field": [CRAWLER], "library": lib(5, X) },
                }));
                s.play(DUSTING, json!({}));
                s.end_turn();
                let p2_before = draws_by(s.events(), P2).len();

                s.end_turn();

                // p1: the trap's draw, the Crawler's, and the turn's draw; p2: its Crawler's.
                assert_eq!(draws_by(s.last_events(), P1).len(), 3);
                assert_eq!(draws_by(s.events(), P2).len() - p2_before, 1);
            }

            #[test]
            fn c_27_a_pestilent_slime_doubles_its_own_placement_it_takes_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [DUSTING, FILLER], "library": lib(4, X) },
                    "p2": { "hand": [FILLER], "field": [SLIME, { "def": VANILLA, "lane": 2 }], "library": lib(4, X) },
                }));
                s.play(DUSTING, json!({}));
                s.end_turn();
                s.end_turn();

                assert_eq!(s.card(SLIME).counters.plague, Some(2));
                assert_eq!(s.card(VANILLA).counters.plague, Some(1));
            }

            #[test]
            fn s2_4_a_full_hand_burns_the_draw() {
                crate::register_all();
                let mut hand = vec![DUSTING];
                hand.extend([FILLER; 9]);
                let mut s = scenario(json!({
                    "p1": { "hand": hand, "library": lib(4, X) },
                    "p2": { "hand": [FILLER], "library": lib(4, X) },
                }));
                s.play(DUSTING, json!({}));
                s.play(FILLER, json!({})); // Stockpile: draw 2 refills the hand to 10.
                assert_eq!(s.hand(P1).len(), 10);
                s.end_turn();

                s.end_turn();

                assert_eq!(s.last_events().iter().map(js).filter(|event| event["type"] == "burned").count(), 2);
                assert_eq!(s.hand(P1).len(), 10);
            }

            #[test]
            fn r386_an_upgrade_places_2_on_each_permanent_and_draws_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [DUSTING, FILLER], "field": [VANILLA], "library": lib(5, X) },
                    "p2": { "hand": [FILLER], "field": [MENACE], "library": lib(4, X) },
                }));
                step_param(s.card_mut(DUSTING), "tokens", 1);
                step_param(s.card_mut(DUSTING), "draw", 1);
                s.play(DUSTING, json!({}));
                s.end_turn();

                s.end_turn();

                assert_eq!(placed_on(&s).values().cloned().collect::<Vec<Vec<Value>>>(), vec![vec![json!(2)]; 3]);
                assert_eq!(draws_by(s.last_events(), P1).len(), 3);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_designers_radiant_face_places_3_plague_counters_on_each_permanent_one_placement_each_and_draws_3() {
                crate::register_all();
                let mut s = set_up(true);
                let cards: Vec<CardInstance> =
                    vec![s.card(VANILLA).clone(), s.card(MANA_WELL).clone(), s.card(MENACE).clone(), s.card(PAWN).clone()];
                s.end_turn();

                s.end_turn();

                for card in &cards {
                    assert_eq!(s.card(card).counters.plague, Some(3), "{}", card.def_id);
                }
                assert_eq!(placed_on(&s).values().cloned().collect::<Vec<Vec<Value>>>(), vec![vec![json!(3)]; 5]);
                // Its three draws, then the turn's.
                assert_eq!(draws_by(s.last_events(), P1).len(), 4);
                s.expect_in_zone(DUSTING, "graveyard");
            }

            #[test]
            fn it_too_waits_through_the_opponents_turn_start() {
                crate::register_all();
                let mut s = set_up(true);
                s.end_turn();
                assert_eq!(placed_on(&s).len(), 0);
                s.expect_in_zone(DUSTING, "field");
            }

            #[test]
            fn r386_a_degrade_places_2_on_each_and_draws_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": DUSTING, "radiant": true }, FILLER], "field": [VANILLA], "library": lib(5, X) },
                    "p2": { "hand": [FILLER], "library": lib(4, X) },
                }));
                step_param(s.card_mut(DUSTING), "tokens", -1);
                step_param(s.card_mut(DUSTING), "draw", -1);
                s.play(DUSTING, json!({}));
                s.end_turn();

                s.end_turn();

                assert_eq!(s.card(VANILLA).counters.plague, Some(2));
                assert_eq!(draws_by(s.last_events(), P1).len(), 3);
            }
        }
    }
}
