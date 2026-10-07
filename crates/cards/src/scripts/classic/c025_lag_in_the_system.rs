//! C #25 Lag in the System (SPEC §8.6 row 25, §6.3 Exile; R13, R65, R66, R113, R135, R396). Spell,
//! cost 0, Epic.
//!   Base:    "Exile every ({threshold}) Cost or less card on the field." (balance patch 1: the
//!            Field alone, the whole board)
//!   Radiant: "Exile every ({threshold}) Cost or less enemy card on the field, in their hand and in
//!            their deck."
//!   Engine:  "C #18 with the numbers fixed at 0 and 1: the same zones, the same cost reading (R65 at
//!            resolution; an X card in a hand or deck costs 0, so it goes, and on the field it costs its
//!            X, R396), the Spell itself spared, graveyards and exile untouched. Tunes: threshold 1 ↑."
//!
//! THE SET is read once, as the Spell resolves (`forEachCard`, R66, R113), and each card is then its
//! own exile (R135), so a card that a sweep uncovers — one dormant under a Stack pile, which is not on
//! the field while it lies there (§3.2, R13) — is not in it. The zones are the field (the tops of the
//! unit piles and every backrow card, face-down ones included, both sides in R68's walk), then each
//! side's hand and deck, the Spell's controller first. Graveyards and exile are never read. The Spell
//! itself is resolving (§10.5), in none of those zones, so it is spared.
//!
//! THE COST is `costNow` (R396), R65's cost as it stands at resolution: a hand card at its hand cost
//! (its player's discounts included), a deck or field card at its own; an X card counts the X it was
//! played for on the field, and 0 anywhere else or when it arrived with none chosen. An exiled card is
//! public, so each `exiled` event names it; none carries a deck position.
//!
//! THE RADIANT FACE reads the opponent's side only: their field, hand and deck.

use jackioh_engine::effects::{ForEachCardArgs, cards_in_scope, exile, for_each_card, sides_of};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-025";

/// TS `type Whose = "any" | "enemy"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Whose {
    Any,
    Enemy,
}

impl Whose {
    /// The `BoardScope.side` literal.
    fn side(self) -> &'static str {
        match self {
            Whose::Any => "any",
            Whose::Enemy => "enemy",
        }
    }
}

/// Every card on the field and in the hands and decks of `whose` sides, in the order the header
/// gives — or the field alone (the whole board) when `field_only`, which is the base face's scope
/// (balance patch 1).
fn reachable(ctx: &EffectContext<'_>, whose: Whose, field_only: bool) -> Vec<CardInstance> {
    let scope: BoardScope = json_as(json!({ "side": whose.side(), "rows": ["units", "backrow"] }));
    let field = cards_in_scope(ctx, &scope);
    if field_only {
        return field;
    }
    let players: Vec<PlayerId> = match whose {
        Whose::Enemy => sides_of(ctx, Some(ScopeSide::Enemy)),
        Whose::Any => {
            let mut players = vec![ctx.controller];
            players.extend(sides_of(ctx, Some(ScopeSide::Enemy)));
            players
        }
    };
    let piles: Vec<CardInstance> = players
        .iter()
        .flat_map(|&player| {
            let mut cards = zone_cards(&*ctx.state, player, OffFieldZone::Hand);
            cards.extend(zone_cards(&*ctx.state, player, OffFieldZone::Library));
            cards
        })
        .collect();
    field.into_iter().chain(piles).collect()
}

fn lag(whose: Whose, field_only: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let threshold = param(&*ctx, "threshold");
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(move |read: &mut EffectContext<'_>| -> Vec<String> {
                    reachable(read, whose, field_only)
                        .into_iter()
                        .filter(|card| cost_now(&*read.state, card) <= threshold)
                        .map(|card| card.id)
                        .collect()
                }),
                each: Arc::new(|instance_id: &str| {
                    exile(json_as(json!({ "target": { "of": "instance", "instanceId": instance_id } })))
                }),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: lag(Whose::Any, true),
        radiant: lag(Whose::Enemy, false),
    }
}

// C #25 Lag in the System — SPEC §8.6 row 25, BUILD M9 Classic row C 25: "Exile every (1) Cost or
// less card on both fields" (balance patch 1: the Field alone — units and backrow, both sides; hands
// and decks stay). Costs read per R65 at resolution; an X card on the field costs the X it was played
// for (0 with none chosen, R396); face-down and Indestructible cards (C #90 In Too Deep) included;
// graveyards and exile untouched; the Spell itself is resolving and spared; radiant: the opponent's
// field, hand and deck only (an X card in a hand or deck costs 0 and goes); its tuned number
// (threshold) reads through `param()` (R386)".
//
// An X card on the field "played for X": the test stands a C+ #69 Buff Billy on the field with its
// stats given (`statsOverride`) and records the X it was played for on the instance, as a play would
// (`CardInstance.x`, §2.3).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const LAG: &str = "classic-025";
    const REPLENISH: &str = "core-010"; // (0) Spell
    const INFINITE: &str = "core-075"; // (0) Field Spell
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const TIMMY: &str = "core-011"; // (1) Unit 3/3
    const STOCKPILE: &str = "core-005"; // (1) Spell
    const SHEEPISH: &str = "core-041"; // (1) Trap
    const BREAD: &str = "core-018"; // (1) Field Trap
    const STATE_OF_GAME: &str = "classic-041"; // (1) Unit, Indestructible
    const IN_TOO_DEEP: &str = "classic-090"; // (1) Field Spell, Indestructible
    const DIVIDEND: &str = "core-024"; // (X) Spell
    const BILLY: &str = "classicplus-069"; // (X) Unit
    const FELINORS: &str = "core-012"; // (2) Unit 3/4
    const EXPERIMENT: &str = "core-085"; // (2) Trap
    const MENACE: &str = "core-019"; // (3) Unit 9/9
    const MANA_WELL: &str = "core-006"; // (3) Field Spell
    const SEVEN: &str = "core-025"; // (4) Unit 7/7
    const TOE_CRACKER: &str = "classic-006"; // (2) Unit: "Aura: Your Traps cost (0)."
    const RUSH_TOKEN: &str = "core-t-rush"; // (1) Unit token

    fn ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.id.clone()).collect()
    }

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    fn lag_board(radiant_face: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": LAG, "radiant": radiant_face }, FELINORS, VANILLA],
                "field": [TIMMY, MENACE],
                "backrow": [{ "def": SHEEPISH, "faceUp": false }, MANA_WELL],
                "library": [REPLENISH, SEVEN],
                "graveyard": [STOCKPILE],
                "exile": [TIMMY],
            },
            "p2": {
                "hand": [FELINORS, STOCKPILE],
                "field": [VANILLA, SEVEN],
                "backrow": [{ "def": BREAD, "faceUp": false }, EXPERIMENT],
                "library": [INFINITE, MENACE, SHEEPISH],
                "graveyard": [VANILLA],
                "exile": [REPLENISH],
            },
        }))
    }

    fn exiled_events(events: &[GameEvent]) -> Vec<GameEvent> {
        events.iter().filter(|event| event.event_type() == GameEventType::Exiled).cloned().collect()
    }

    /// TS `stepParam(s.card(ref), key, steps)`: on the live instance, found again by id.
    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        step_param(find_instance_mut(s.state_mut(), &id).expect("the card is in the state"), key, steps);
    }

    fn def_id_of(card: Option<CardInstance>) -> Option<String> {
        card.map(|card| card.def_id)
    }

    mod c_n25_lag_in_the_system {
        use super::*;

        #[test]
        fn declares_its_one_number_threshold_r386() {
            crate::register_all();
            let scripts = script();
            assert_eq!(
                serde_json::to_value(&crate::card_def(ID).params).unwrap(),
                json!([{ "key": "threshold", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1 }])
            );
            assert!(scripts.base.targets.is_empty());
            assert!(scripts.radiant.targets.is_empty());
        }

        mod base {
            use super::*;

            #[test]
            fn exiles_every_1_cost_or_less_card_on_both_fields_and_nothing_in_either_hand_or_deck() {
                crate::register_all();
                let mut s = lag_board(false);
                let gone: Vec<CardInstance> = [s.unit(P1, 1), s.backrow(P1, 1), s.unit(P2, 1), s.backrow(P2, 1)]
                    .into_iter()
                    .map(|card| card.expect("setup: a card is missing"))
                    .collect();
                s.play(LAG, json!({}));
                for card in &gone {
                    s.expect_in_zone(card, "exile");
                }
                // What costs more stays where it was.
                assert_eq!(def_id_of(s.unit(P1, 2)), Some(MENACE.to_string()));
                assert_eq!(def_id_of(s.backrow(P1, 2)), Some(MANA_WELL.to_string()));
                assert_eq!(def_id_of(s.unit(P2, 2)), Some(SEVEN.to_string()));
                assert_eq!(def_id_of(s.backrow(P2, 2)), Some(EXPERIMENT.to_string()));
                // Hands and decks are out of scope now.
                assert_eq!(def_ids(&s.hand(P1)), vec![FELINORS, VANILLA]);
                assert_eq!(def_ids(&s.pile(P1, "library")), vec![REPLENISH, SEVEN]);
                assert_eq!(def_ids(&s.hand(P2)), vec![FELINORS, STOCKPILE]);
                assert_eq!(def_ids(&s.pile(P2, "library")), vec![INFINITE, MENACE, SHEEPISH]);
            }

            #[test]
            fn graveyards_and_exile_are_untouched() {
                crate::register_all();
                let mut s = lag_board(false);
                let mut graves = ids(&s.pile(P1, "graveyard"));
                graves.extend(ids(&s.pile(P2, "graveyard")));
                s.play(LAG, json!({}));
                for id in &graves {
                    s.expect_in_zone(id.as_str(), "graveyard");
                }
                assert_eq!(def_ids(&s.pile(P1, "graveyard")), vec![STOCKPILE, LAG]);
                assert_eq!(def_ids(&s.pile(P2, "graveyard")), vec![VANILLA]);
            }

            #[test]
            fn the_spell_itself_is_resolving_so_it_is_spared_and_lands_in_the_graveyard() {
                crate::register_all();
                let mut s = lag_board(false);
                let lag = s.card(LAG).clone();
                s.play(LAG, json!({}));
                s.expect_in_zone(&lag, "graveyard");
                assert!(!s
                    .events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Exiled { instance_id, .. } if *instance_id == lag.id)));
            }

            #[test]
            fn face_down_and_indestructible_cards_are_included() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [LAG, FELINORS], "field": [STATE_OF_GAME], "backrow": [IN_TOO_DEEP] },
                    "p2": { "hand": [STOCKPILE, FELINORS], "backrow": [{ "def": SHEEPISH, "faceUp": false }] },
                }));
                let state = s.card(STATE_OF_GAME).clone();
                let deep = s.card(IN_TOO_DEEP).clone();
                let trap = s.card(SHEEPISH).clone();
                s.play(LAG, json!({}));
                s.expect_in_zone(&state, "exile");
                s.expect_in_zone(&deep, "exile");
                s.expect_in_zone(&trap, "exile");
            }

            #[test]
            fn r65_a_set_card_is_read_at_its_own_cost_a_2_trap_under_toe_cracker_costs_2_and_stays() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [LAG, FELINORS], "field": [TOE_CRACKER], "backrow": [{ "def": EXPERIMENT, "faceUp": false }] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                let trap = s.card(EXPERIMENT).clone();
                s.play(LAG, json!({}));
                // The aura prices plays from the hand, not set cards: the (2) Trap stays.
                s.expect_in_zone(&trap, "field");
                s.expect_in_zone(TOE_CRACKER, "field");
            }

            #[test]
            fn r396_an_x_card_on_the_field_costs_the_x_it_was_played_for_played_for_3_it_stays() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [LAG, FELINORS] },
                    "p2": { "hand": [STOCKPILE], "field": [{ "def": BILLY, "statsOverride": { "attack": 9, "health": 9 } }] },
                }));
                let billy = s.card(BILLY).id.clone();
                find_instance_mut(s.state_mut(), &billy).unwrap().x = Some(3);
                s.play(LAG, json!({}));
                s.expect_in_zone(BILLY, "field");
            }

            #[test]
            fn r396_an_x_card_on_the_field_with_no_x_chosen_a_summon_costs_0_and_goes() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [LAG, FELINORS] },
                    "p2": { "hand": [STOCKPILE], "field": [{ "def": BILLY, "statsOverride": { "attack": 3, "health": 3 } }] },
                }));
                let billy = s.card(BILLY).clone();
                s.play(LAG, json!({}));
                s.expect_in_zone(&billy, "exile");
            }

            #[test]
            fn r11_a_unit_token_exiled_from_the_field_ceases_to_exist() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [LAG, FELINORS] }, "p2": { "hand": [STOCKPILE], "field": [RUSH_TOKEN] } }));
                let token = s.card(RUSH_TOKEN).clone();
                s.play(LAG, json!({}));
                s.expect_in_zone(&token, "gone");
                assert!(s.unit(P2, 1).is_none());
            }

            #[test]
            fn r13_a_card_dormant_under_a_stack_pile_is_not_on_the_field_and_is_not_exiled_when_the_top_goes() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [LAG, FELINORS] },
                    "p2": { "hand": [MENACE], "field": [VANILLA, { "def": TIMMY, "stack": true }] },
                }));
                let vanilla = s.card(VANILLA).clone();
                let timmy = s.card(TIMMY).clone();
                s.play(LAG, json!({}));
                s.expect_in_zone(&timmy, "exile");
                // The Vanilla beneath was dormant when the set was read; it now tops the lane.
                s.expect_in_zone(&vanilla, "field");
                assert_eq!(s.unit(P2, 1).map(|card| card.id), Some(vanilla.id.clone()));
            }

            #[test]
            fn no_event_carries_a_deck_position_and_each_exile_is_public_to_both_players() {
                crate::register_all();
                let mut s = lag_board(false);
                s.play(LAG, json!({}));
                let exiled = exiled_events(s.events());
                assert!(!exiled.is_empty());
                for event in &exiled {
                    let value = serde_json::to_value(event).unwrap();
                    let mut keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();
                    keys.sort();
                    assert_eq!(keys, vec!["defId", "instanceId", "owner", "type"]);
                }
                for viewer in [P1, P2] {
                    let seen = exiled_events(&s.view(viewer).events);
                    assert_eq!(seen, exiled);
                }
            }

            #[test]
            fn with_nothing_that_cheap_anywhere_nothing_is_exiled() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [LAG, MENACE], "field": [SEVEN] }, "p2": { "hand": [MENACE], "library": [SEVEN] } }));
                s.play(LAG, json!({}));
                assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Exiled));
            }

            #[test]
            fn r386_a_degrade_of_the_threshold_never_goes_below_1() {
                crate::register_all();
                let mut s = lag_board(false);
                step(&mut s, LAG, "threshold", -1);
                s.play(LAG, json!({}));
                // A (1) Cost card still goes: the Timmy on your field. Hands are out of scope.
                assert_eq!(def_ids(&s.hand(P1)), vec![FELINORS, VANILLA]);
                assert!(s.unit(P1, 1).is_none());
            }

            #[test]
            fn r386_an_upgrade_of_the_threshold_reaches_2_cost_cards_on_the_fields() {
                crate::register_all();
                let mut s = lag_board(false);
                step(&mut s, LAG, "threshold", 1);
                s.play(LAG, json!({}));
                assert!(s.unit(P1, 1).is_none());
                assert!(s.backrow(P2, 2).is_none());
                assert_eq!(def_id_of(s.unit(P1, 2)), Some(MENACE.to_string()));
                // Hands and decks stay whatever the threshold is.
                assert_eq!(def_ids(&s.hand(P1)), vec![FELINORS, VANILLA]);
                assert_eq!(def_ids(&s.hand(P2)), vec![FELINORS, STOCKPILE]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn exiles_the_opponent_s_1_cost_or_less_cards_on_their_field_in_their_hand_and_in_their_deck() {
                crate::register_all();
                let mut s = lag_board(true);
                let theirs = [
                    s.unit(P2, 1),
                    s.backrow(P2, 1),
                    s.pile(P2, "hand").get(1).cloned(),
                    s.pile(P2, "library").first().cloned(),
                    s.pile(P2, "library").get(2).cloned(),
                ];
                s.play(LAG, json!({}));
                for card in theirs {
                    let card = card.expect("setup: a card is missing");
                    s.expect_in_zone(&card, "exile");
                }
                assert_eq!(def_ids(&s.hand(P2)), vec![FELINORS]);
            }

            #[test]
            fn your_own_cheap_cards_are_untouched() {
                crate::register_all();
                let mut s = lag_board(true);
                s.play(LAG, json!({}));
                assert_eq!(def_id_of(s.unit(P1, 1)), Some(TIMMY.to_string()));
                assert_eq!(def_id_of(s.backrow(P1, 1)), Some(SHEEPISH.to_string()));
                assert_eq!(def_ids(&s.hand(P1)), vec![FELINORS, VANILLA]);
                assert_eq!(def_ids(&s.pile(P1, "library")), vec![REPLENISH, SEVEN]);
            }

            #[test]
            fn their_graveyard_and_exile_are_untouched_and_the_spell_is_spared() {
                crate::register_all();
                let mut s = lag_board(true);
                s.play(LAG, json!({}));
                assert_eq!(def_ids(&s.pile(P2, "graveyard")), vec![VANILLA]);
                assert_eq!(
                    def_ids(&s.pile(P2, "exile")),
                    vec![REPLENISH, VANILLA, BREAD, STOCKPILE, INFINITE, SHEEPISH]
                );
                s.expect_in_zone(LAG, "graveyard");
            }

            #[test]
            fn r386_an_upgrade_of_the_threshold_reaches_their_2_cost_cards() {
                crate::register_all();
                let mut s = lag_board(true);
                step(&mut s, LAG, "threshold", 1);
                s.play(LAG, json!({}));
                assert!(s.hand(P2).is_empty());
                assert!(s.backrow(P2, 2).is_none());
                assert_eq!(def_ids(&s.hand(P1)), vec![FELINORS, VANILLA]);
            }

            #[test]
            fn r65_their_hand_card_is_read_at_its_hand_cost_a_2_trap_under_their_toe_cracker_costs_0_and_goes() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": LAG, "radiant": true }, FELINORS] },
                    "p2": { "hand": [EXPERIMENT, STOCKPILE], "field": [TOE_CRACKER], "library": [FELINORS] },
                }));
                s.play(LAG, json!({}));
                s.expect_in_zone(EXPERIMENT, "exile");
                s.expect_in_zone(STOCKPILE, "exile");
                s.expect_in_zone(TOE_CRACKER, "field");
            }

            #[test]
            fn r396_an_x_card_in_their_hand_or_deck_costs_0_and_goes() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": LAG, "radiant": true }, FELINORS] },
                    "p2": { "hand": [DIVIDEND, FELINORS], "library": [BILLY] },
                }));
                s.play(LAG, json!({}));
                s.expect_in_zone(DIVIDEND, "exile");
                s.expect_in_zone(BILLY, "exile");
                s.expect_in_zone(FELINORS, "hand");
            }
        }
    }
}
