//! C #64 Malzahar's Recycler (SPEC §8.6 row 64). (1) Field Spell, Rare.
//!   Base:    "Activate: Discard 2 random cards. / Whenever you discard cards, draw that many."
//!   Radiant: "Activate: Discard 2 random cards. / Whenever you discard cards, draw your deck."
//!   Engine:  The Activate (R384, once a turn) pays its discard as its cost: 2 random cards of the hand
//!            (R682), so a hand of fewer than 2 cannot activate it, and the ability does nothing more.
//!            "Whenever you discard cards" answers your own `discarded` events, one draw each, once the
//!            discarding effect has finished (queued triggers, §10.3), so an effect that discards 2
//!            draws 2; whoever's effect made you discard counts, the opponent's own discards never. A
//!            Brittle crumble (§6.1, R385) is not a discard.
//!            Radiant: each answer draws the deck as its size stands then (R58), so the first answer
//!            takes it all and the rest find it empty, no fatigue (R87); most of it burns at the hand
//!            cap (§2.4), which is the card.
//!   It discarded at its controller's end of turn until patch v0.3.13 (#1194); `script_v0_3_7b` keeps
//!   that for the replays of v0.3.7b and before (R767000).

use jackioh_engine::effects::{discard_random, draw};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-064";

/// "Discard 2 random cards." — random (R682).
const DISCARDS: i32 = 2;

/// "Activate: Discard 2 random cards.": the discard is the ability's cost, paid as it is activated
/// (R384), so the ability itself does nothing more; the `recycle` answer draws.
fn discard_two() -> ActivationDecl {
    ActivationDecl {
        id: "discard".to_string(),
        label: "Discard 2 random cards".to_string(),
        uses: ActivationUses::Count(1),
        cost: Some(ActivationCost {
            discard_random: Some(DISCARDS),
            ..ActivationCost::default()
        }),
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|_ctx| vec![]),
    }
}

/// R767000: "End of turn: Discard 2 random cards.", as v0.3.7b and every version before it printed it.
fn end_of_turn(_ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    vec![discard_random(json_as(json!({ "count": DISCARDS })))]
}

/// Whether this event is a card its controller discarded.
fn your_discard(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    matches!(event, GameEvent::Discarded { owner, .. } if *owner == ctx.controller)
}

fn answer(draws: Hook) -> TriggerDef {
    TriggerDef::new("recycle", &[GameEventType::Discarded], move |ctx, event| {
        if your_discard(ctx, event) { draws(ctx) } else { vec![] }
    })
    .with_when(|ctx, event| your_discard(ctx, event))
}

/// "Draw that many": one draw for each card discarded.
fn draw_one(_ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    vec![draw(json_as(json!({ "count": 1 })))]
}

/// "Draw your deck": the deck's size as the draw starts (R58).
fn draw_deck(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let count = zone_count(&*ctx.state, ctx.controller, OffFieldZone::Library);
    vec![draw(json_as(json!({ "count": count })))]
}

pub fn script() -> CardScripts {
    let base = Script {
        activations: vec![discard_two()],
        triggers: vec![answer(hook(draw_one))],
        ..Script::default()
    };

    let radiant = Script {
        activations: vec![discard_two()],
        triggers: vec![answer(hook(draw_deck))],
        ..Script::default()
    };

    CardScripts { base, radiant }
}

/// R767000: the scripts v0.3.7b and every version before it ran, which discarded at the end of their
/// controller's turn; `KEPT_SCRIPTS` folds those versions' replays under them.
pub fn script_v0_3_7b() -> CardScripts {
    let base = Script {
        end_of_turn: Some(hook(end_of_turn)),
        triggers: vec![answer(hook(draw_one))],
        ..Script::default()
    };

    let radiant = Script {
        end_of_turn: Some(hook(end_of_turn)),
        triggers: vec![answer(hook(draw_deck))],
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// Every discard of yours is shown: C #15 Nose Hunter, C #26 Rapid Draw, C #37 Last Hurrah, C #8 Pickle
// played against you, Core #80 Zao Gao, Core #21 Hinder and C #89 Paul Allen's Ghost's targeting cost.
#[cfg(test)]
mod tests {
    use jackioh_engine::subsystems::activate::why_cannot_activate_ability;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const RECYCLER: &str = "classic-064";
    const ZAO_GAO: &str = "core-080"; // (2) Spell: Discard 2 random cards. Summon 2 Rush Tokens …
    const HINDER: &str = "core-021"; // (0) Spell, Cast on draw: … Discard 1 random card.
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const MENACE: &str = "core-019"; // (3) Unit 9/9
    const FILLER: &str = "core-010"; // (0) Spell
    const NOSE_HUNTER: &str = "classic-015"; // Activate: Discard a random card. Exile the bottom card of their deck.
    const RAPID_DRAW: &str = "classic-026"; // (0) Spell: Draw 4. Then discard 4 random cards.
    const LAST_HURRAH: &str = "classic-037"; // (0) Spell: Draw your deck. At the end of this turn, discard your hand.
    const PICKLE: &str = "classic-008"; // (1) Spell: at the start of your opponent's next turn, they choose 3 times: discard 1, exile 1, or you draw 1.
    const INCOME_TAX: &str = "classic-009"; // Trap: when the cards your opponent has drawn in a turn reach 2 …

    use crate::js;

    fn drawn_by(events: &[GameEvent], player: PlayerId) -> Vec<Value> {
        events
            .iter()
            .map(js)
            .filter(|event| event["type"] == "drawn" && event["player"] == js(&player))
            .collect()
    }

    fn discarded_by(events: &[GameEvent], player: PlayerId) -> Vec<Value> {
        events
            .iter()
            .map(js)
            .filter(|event| event["type"] == "discarded" && event["owner"] == js(&player))
            .collect()
    }

    /// The index of the first event of this type, which every caller expects to find.
    fn find_index(events: &[GameEvent], type_: &str) -> usize {
        events
            .iter()
            .position(|event| js(event)["type"] == type_)
            .unwrap_or_else(|| panic!("no {type_} event"))
    }

    fn pick(s: &Scenario, def_ids: &[&str]) -> Value {
        let mut used: IndexSet<String> = IndexSet::new();
        let mut picks: Vec<Value> = Vec::new();
        for def_id in def_ids {
            let card = s
                .hand(P1)
                .iter()
                .find(|held| held.def_id == *def_id && !used.contains(&held.id))
                .map(|held| held.id.clone());
            let Some(card) = card else {
                panic!("no {def_id} in hand");
            };
            used.insert(card.clone());
            picks.push(json!({ "pick": "instance", "instanceId": card }));
        }
        json!(picks)
    }

    /// p1's Recycler on the field, in p1's main phase.
    /// `opts`: `{ radiant?, hand, library? }`.
    fn recycling(opts: Value) -> Scenario {
        let radiant = opts["radiant"] == true;
        let library = if opts["library"].is_null() {
            json!([VANILLA, VANILLA, VANILLA, VANILLA])
        } else {
            opts["library"].clone()
        };
        scenario(json!({
            "p1": {
                "hand": opts["hand"].clone(),
                "backrow": [{ "def": RECYCLER, "radiant": radiant }],
                "library": library,
            },
            "p2": { "hand": [FILLER], "library": [VANILLA, VANILLA, VANILLA] },
        }))
    }

    /// Whether every found def id is one of `pool`.
    fn all_among(found: &[Value], pool: &[&str]) -> bool {
        found.iter().all(|def_id| pool.iter().any(|id| *def_id == *id))
    }

    mod c_64_malzahars_recycler {
        use super::*;

        #[test]
        fn declares_no_tuned_numbers() {
            crate::register_all();
            assert!(crate::card_def(RECYCLER).params.is_none());
        }

        mod base {
            use super::*;

            #[test]
            fn r384_r682_its_activate_discards_2_random_cards_its_own_discard_draws_2_and_not_again_this_turn() {
                crate::register_all();
                let mut s = recycling(json!({ "hand": [MENACE, VANILLA, FILLER] }));
                s.activate(RECYCLER, json!({}));
                // R682: no prompt opens — the two discards land at once, at random.
                assert!(
                    !s.events().iter().map(js).any(|event| event["type"] == "promptOpened" && event["player"] == "p1")
                );
                let discarded: Vec<Value> =
                    discarded_by(s.last_events(), P1).iter().map(|event| event["defId"].clone()).collect();
                assert_eq!(discarded.len(), 2);
                assert!(all_among(&discarded, &[MENACE, VANILLA, FILLER]));
                // The two draws answer the discards in the same action, still p1's main phase.
                assert_eq!(s.state().active, P1);
                assert_eq!(drawn_by(s.last_events(), P1).len(), 2);
                assert_eq!(s.hand(P1).len(), 3);
                // R384: "Activate" is once a turn.
                let recycler = s.card(RECYCLER).id.clone();
                let refused =
                    why_cannot_activate_ability(s.state(), P1, &recycler, None).err().map(|error| error.message);
                assert_eq!(refused.as_deref(), Some("that ability has already been used this turn"));
            }

            #[test]
            fn r682_the_random_discards_come_from_the_match_rng_the_same_game_discards_the_same_cards() {
                crate::register_all();
                let mut first = recycling(json!({ "hand": [MENACE, VANILLA, FILLER] }));
                first.activate(RECYCLER, json!({}));
                let mut second = recycling(json!({ "hand": [MENACE, VANILLA, FILLER] }));
                second.activate(RECYCLER, json!({}));
                let ids = |s: &Scenario| -> Vec<Value> {
                    discarded_by(s.last_events(), P1).iter().map(|event| event["instanceId"].clone()).collect()
                };
                assert_eq!(ids(&first), ids(&second));
            }

            #[test]
            fn r384_its_discard_is_its_cost_with_fewer_than_2_cards_in_hand_it_cannot_be_activated() {
                crate::register_all();
                let s = recycling(json!({ "hand": [MENACE] }));
                let recycler = s.card(RECYCLER).id.clone();
                let refused =
                    why_cannot_activate_ability(s.state(), P1, &recycler, None).err().map(|error| error.message);
                assert_eq!(refused.as_deref(), Some("that ability needs 2 cards in your hand to discard"));
                assert!(!legal_actions(s.state(), P1).into_iter().any(|body| {
                    matches!(body, ActionBody::Activate { instance_id, .. } if instance_id == recycler)
                }));
            }

            #[test]
            fn it_no_longer_discards_at_the_end_of_your_turn() {
                crate::register_all();
                let mut s = recycling(json!({ "hand": [MENACE, VANILLA, FILLER] }));
                s.end_turn();
                let events = s.last_events();
                let p2_starts = find_index(events, "turnStarted");
                assert_eq!(discarded_by(&events[..p2_starts], P1).len(), 0);
                assert_eq!(drawn_by(&events[..p2_starts], P1).len(), 0);
                assert_eq!(s.hand(P1).len(), 3);
            }

            #[test]
            fn a_random_discard_of_yours_counts_zao_gaos_2_random_discards_draw_2() {
                crate::register_all();
                let mut s = recycling(json!({ "hand": [ZAO_GAO, MENACE, VANILLA, FILLER] }));
                s.play(ZAO_GAO, json!({}));
                assert_eq!(discarded_by(s.last_events(), P1).len(), 2);
                assert_eq!(drawn_by(s.last_events(), P1).len(), 2);
            }

            #[test]
            fn r682_a_random_discard_of_yours_counts_hinder_cast_on_draw_discards_1_at_random_and_you_draw_1() {
                crate::register_all();
                let mut s = recycling(json!({
                    "hand": [STOCKPILE, MENACE],
                    "library": [HINDER, VANILLA, VANILLA, VANILLA, VANILLA],
                }));
                s.play(STOCKPILE, json!({}));
                // Hinder is cast on the first draw and discards at random (R682): no prompt opens.
                assert!(s.state().pending.is_none());
                assert_eq!(discarded_by(s.last_events(), P1).len(), 1);
                // Stockpile's two draws (the first repeating past Hinder) and the Recycler's one.
                assert_eq!(drawn_by(s.events(), P1).len(), 4);
            }

            #[test]
            fn a_targeting_costs_discards_count_c_89s_two_random_ones_draw_two() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": {
                        "hand": ["classic-055", FILLER, FILLER],
                        "backrow": [RECYCLER],
                        "library": [VANILLA, VANILLA, VANILLA],
                    },
                    "p2": { "field": ["classic-089"] },
                }));
                let action: Action = json_as(json!({
                    "type": "play",
                    "playerId": "p1",
                    "nonce": "c64-ghost",
                    "instanceId": s.card("classic-055").id.clone(),
                    "targets": [{ "pick": "instance", "instanceId": s.card("classic-089").id.clone() }],
                }));
                let result = reduce(s.state(), &action);
                assert!(result.error.is_none());
                assert_eq!(drawn_by(&result.events, P1).len(), 2);
            }

            #[test]
            fn c_15_nose_hunters_random_discard_its_activates_cost_draws_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [MENACE, FILLER],
                        "field": [NOSE_HUNTER],
                        "backrow": [RECYCLER],
                        "library": [VANILLA, VANILLA],
                    },
                    "p2": { "hand": [FILLER], "library": [VANILLA, VANILLA] },
                }));
                s.activate(NOSE_HUNTER, json!({}));
                assert_eq!(discarded_by(s.last_events(), P1).len(), 1);
                assert_eq!(drawn_by(s.last_events(), P1).len(), 1);
                assert_eq!(s.hand(P1).len(), 2);
            }

            #[test]
            fn c_26_rapid_draws_random_four_are_one_effect_that_discards_4_and_draw_4() {
                crate::register_all();
                let library: Vec<&str> = (0..8).map(|_| VANILLA).collect();
                let mut s = recycling(json!({ "hand": [RAPID_DRAW, FILLER], "library": library }));
                s.play(RAPID_DRAW, json!({}));
                // R682: no prompt opens — four random cards go at once.
                assert!(s.state().pending.is_none());
                assert_eq!(discarded_by(s.last_events(), P1).len(), 4);
                assert_eq!(drawn_by(s.last_events(), P1).len(), 8);
                assert_eq!(s.hand(P1).len(), 5);
                assert_eq!(s.pile(P1, "library").len(), 0);
            }

            #[test]
            fn c_37_last_hurrahs_end_of_turn_discard_of_your_hand_draws_that_many_from_the_deck_it_emptied_fatigue() {
                crate::register_all();
                let mut s = recycling(json!({
                    "hand": [LAST_HURRAH],
                    "library": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA],
                }));
                s.play(LAST_HURRAH, json!({}));
                assert_eq!(s.hand(P1).len(), 5);
                s.end_turn();
                // After `turnEnded`, Last Hurrah discards the whole hand of five, and each discard is
                // answered from the deck it emptied: five fatigue draws (R62).
                let events = s.last_events();
                let after = &events[find_index(events, "turnEnded")..];
                let before_next_turn = &after[..find_index(after, "turnStarted")];
                assert_eq!(discarded_by(before_next_turn, P1).len(), 5);
                assert_eq!(
                    before_next_turn
                        .iter()
                        .map(js)
                        .filter(|event| event["type"] == "fatigue" && event["player"] == "p1")
                        .count(),
                    5,
                );
                assert_eq!(s.state().players.p1.fatigue_count, 5);
            }

            #[test]
            fn c_8_pickle_played_against_you_the_random_discard_is_yours_and_draws_2_behind_the_questions() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [MENACE, FILLER], "backrow": [RECYCLER], "library": [VANILLA, VANILLA, VANILLA, VANILLA] },
                    "p2": { "hand": [PICKLE, FILLER], "library": [VANILLA, VANILLA] },
                    "active": "p2",
                }));
                s.play(PICKLE, json!({}));
                // R1279000: p1 is not active, so no prompt opens until p1's next turn.
                assert!(s.state().pending.is_none());
                s.end_turn();
                assert_eq!(s.state().pending.as_ref().map(|pending| pending.player_id), Some(P1));
                s.answer(json!("discard"));
                // R682: the discard lands at once, at random — no second answer.
                assert_eq!(discarded_by(s.last_events(), P1).len(), 1);
                // The answer is a queued trigger (§10.3): it draws once Pickle has finished asking.
                s.answer(json!("exile")).answer(json!("exile"));
                assert!(s.state().pending.is_none());
                // The owed start-of-turn draw landed behind the questions, then the Recycler's.
                assert_eq!(drawn_by(s.events(), P1).len(), 2);
                assert_eq!(s.hand(P1).len(), 3);
                assert!(s.hand(P1).iter().any(|card| card.def_id == VANILLA));
            }

            #[test]
            fn its_draws_are_your_draws_the_second_sets_off_the_opponents_c_9_income_tax() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [FILLER, FILLER, FILLER],
                        "backrow": [RECYCLER],
                        "library": [STOCKPILE, STOCKPILE, STOCKPILE],
                    },
                    "p2": {
                        "hand": [FILLER],
                        "backrow": [{ "def": INCOME_TAX, "faceUp": false }],
                        "library": [VANILLA, VANILLA],
                    },
                }));
                s.activate(RECYCLER, json!({}));
                // R682: the Recycler's two discards are random, so no prompt opens — and the two draws set off the tax.
                assert!(s.events().iter().map(js).any(|event| event["type"] == "trapFired"));
                let pending = js(&s.state().pending);
                assert_eq!(pending["playerId"], "p1");
                assert_eq!(pending["kind"], "hand");
                let selection = pick(&s, &[FILLER]);
                s.answer(selection);
                assert_eq!(
                    s.hand(P1).iter().map(|card| card.def_id.clone()).collect::<Vec<String>>(),
                    vec![FILLER.to_string()],
                );
                assert_eq!(
                    s.hand(P2).iter().filter(|card| card.def_id == STOCKPILE && card.owner == P2).count(),
                    2,
                );
            }

            #[test]
            fn an_opponents_discard_does_not_their_zao_gao_draws_you_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "backrow": [RECYCLER], "library": [VANILLA, VANILLA] },
                    "p2": { "hand": [ZAO_GAO, MENACE, VANILLA, FILLER], "library": [VANILLA] },
                    "active": "p2",
                }));
                s.play(ZAO_GAO, json!({}));
                assert_eq!(discarded_by(s.last_events(), P2).len(), 2);
                assert_eq!(drawn_by(s.last_events(), P1).len(), 0);
                assert_eq!(drawn_by(s.last_events(), P2).len(), 0);
            }

            #[test]
            fn s6_1_r638_a_brittle_crumble_is_no_discard_it_draws_nothing_and_a_card_in_your_hand_never_crumbles() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [FILLER],
                        "field": [{ "def": MENACE, "lane": 1 }],
                        "backrow": [RECYCLER],
                        "library": [VANILLA, VANILLA, VANILLA],
                    },
                    "p2": { "hand": [FILLER], "library": [VANILLA, VANILLA] },
                    "active": "p2",
                }));
                let menace = s.card(MENACE).id.clone();
                find_instance_mut(s.state_mut(), &menace).expect("the Menace is on the field").brittle =
                    Some(BrittleCounter { count: 1, since: 1, printed: None });
                s.end_turn();
                assert_eq!(s.state().active, P1);
                assert!(s.last_events().iter().map(js).any(|event| event["type"] == "crumbled"));
                assert_eq!(discarded_by(s.last_events(), P1).len(), 0);
                // Only the turn's own draw.
                assert_eq!(drawn_by(s.last_events(), P1).len(), 1);
            }

            #[test]
            fn r97_r682_no_prompt_opens_and_the_cards_drawn_are_never_named_in_the_opponents_view() {
                crate::register_all();
                let mut s = recycling(json!({ "hand": [MENACE, VANILLA, FILLER] }));
                s.activate(RECYCLER, json!({}));
                // R682: no prompt opens at all — nothing to redact options from.
                assert!(s.state().pending.is_none());
                assert!(s.view(P2).pending.is_none());
                let drawn_ids: Vec<Value> =
                    drawn_by(s.last_events(), P1).iter().map(|event| event["instanceId"].clone()).collect();
                assert_eq!(drawn_ids.len(), 2);
                let theirs = js(&s.view(P2));
                for event in theirs["events"]
                    .as_array()
                    .expect("the view's events")
                    .iter()
                    .filter(|e| e["type"] == "drawn" && e["player"] == "p1")
                {
                    assert!(!drawn_ids.contains(&event["instanceId"]));
                }
                // The discards are public: the graveyard is.
                let grave: Vec<Value> = theirs["opponent"]["graveyard"]
                    .as_array()
                    .expect("the opponent's graveyard")
                    .iter()
                    .map(|card| card["defId"].clone())
                    .collect();
                assert_eq!(grave.len(), 2);
                assert!(all_among(&grave, &[MENACE, VANILLA, FILLER]));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r58_its_activate_s_discard_draws_your_whole_deck_as_its_size_stands_then() {
                crate::register_all();
                let mut s = recycling(json!({
                    "radiant": true,
                    "hand": [MENACE, VANILLA],
                    "library": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA],
                }));
                s.activate(RECYCLER, json!({}));
                // R682: no prompt — and the whole two-card hand goes, so both discards are certain.
                assert!(s.state().pending.is_none());
                assert_eq!(discarded_by(s.last_events(), P1).len(), 2);
                assert_eq!(drawn_by(s.last_events(), P1).len(), 5);
                assert_eq!(s.pile(P1, "library").len(), 0);
                // Each discard is answered: the second finds the deck empty and draws nothing, and no fatigue.
                assert_eq!(s.state().players.p1.fatigue_count, 0);
            }

            #[test]
            fn r58_s2_4_a_deck_bigger_than_the_hands_room_burns_past_the_hand_cap() {
                crate::register_all();
                let library: Vec<&str> = (0..12).map(|_| VANILLA).collect();
                let mut s = recycling(json!({ "radiant": true, "hand": [MENACE, VANILLA], "library": library }));
                s.activate(RECYCLER, json!({}));
                // R682: the two-card hand goes with no prompt.
                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(P1).len(), 10);
                assert_eq!(s.events().iter().map(js).filter(|event| event["type"] == "burned").count(), 2);
            }

            #[test]
            fn a_random_discard_of_yours_draws_your_whole_deck_too() {
                crate::register_all();
                let mut s = recycling(json!({
                    "radiant": true,
                    "hand": [ZAO_GAO, MENACE, VANILLA],
                    "library": [VANILLA, VANILLA, VANILLA],
                }));
                s.play(ZAO_GAO, json!({}));
                assert_eq!(drawn_by(s.last_events(), P1).len(), 3);
                assert_eq!(s.pile(P1, "library").len(), 0);
            }

            #[test]
            fn an_opponents_discard_still_draws_you_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "backrow": [{ "def": RECYCLER, "radiant": true }], "library": [VANILLA, VANILLA] },
                    "p2": { "hand": [ZAO_GAO, MENACE, VANILLA, FILLER] },
                    "active": "p2",
                }));
                s.play(ZAO_GAO, json!({}));
                assert_eq!(drawn_by(s.last_events(), P1).len(), 0);
            }
        }
    }
}
