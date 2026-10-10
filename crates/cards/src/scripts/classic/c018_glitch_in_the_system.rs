//! C #18 Glitch in the System (SPEC §8.6 row 18, §6.3 Exile, §10.6; R13, R65, R66, R81, R113, R135,
//! R396). Spell, cost 3, Epic.
//!   Base:    "Choose a number. Exile every card on the field that costs that much." (balance patch 1:
//!            the Field alone, the whole board)
//!   Radiant: "Choose a number. Exile every card on your opponent's field, in their hand and in their
//!            deck that costs that much."
//!   Engine:  "A `number` choice (§10.6), declared with the play (R81) from a fixed list, 0 to 10
//!            (`GLITCH_NUMBERS`), so the options reveal nothing. Costs read per R65 at resolution:
//!            a hand card at its hand cost, a deck or field card at its own (as R66 reads #94 Genn's
//!            Greed's); an X-cost card counts its X on the field and 0 anywhere else (R396)."
//!
//! The sweep is C #25 Lag in the System's: read once as the Spell resolves (`forEachCard`, R66, R113),
//! each card its own exile (R135), in R68's walk. A card dormant under a Stack pile is not on the
//! field (§3.2, R13). The Spell itself is resolving, in no pile, and is spared; graveyards and exile
//! are untouched.

use jackioh_engine::effects::{ForEachCardArgs, cards_in_scope, exile, for_each_card, sides_of};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-018";

/// The fixed range the number is chosen from (BUILD §2's `GLITCH_NUMBERS`), as the choice's options.
fn glitch_options() -> Vec<String> {
    GLITCH_NUMBERS.iter().map(|number| number.to_string()).collect()
}

fn number_choice() -> Vec<ModeDecl> {
    vec![ModeDecl {
        kind: PromptKind::Number,
        options: glitch_options(),
    }]
}

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

/// The number the play carried, or null (§10.6's choices are strings).
fn chosen_number(ctx: &EffectContext<'_>) -> Option<i32> {
    let picked = ctx.modes.first()?;
    if !glitch_options().contains(picked) {
        return None;
    }
    picked.parse::<i32>().ok()
}

/// Every card on the field and in the hands and decks of `whose` sides, the Spell's controller first,
/// or the field alone (the whole board) when `field_only`, the base face's scope (balance patch 1).
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

fn glitch(whose: Whose, field_only: bool) -> Script {
    Script {
        modes: number_choice(),
        cry: Some(hook(move |ctx| {
            let Some(number) = chosen_number(ctx) else {
                return vec![];
            };
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(move |read: &mut EffectContext<'_>| -> Vec<String> {
                    reachable(read, whose, field_only)
                        .into_iter()
                        .filter(|card| cost_now(&*read.state, card) == number)
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
        base: glitch(Whose::Any, true),
        radiant: glitch(Whose::Enemy, false),
    }
}

// C #18 Glitch in the System — SPEC §8.6 row 18, BUILD M9 Classic row C 18: the number is chosen with
// the play (R81) from `GLITCH_NUMBERS`; the base face exiles that cost on the Field alone (balance
// patch 1), the Radiant the opponent's field, hand and deck; costs read per R65 (R396). No tuned
// numbers. An X card on the field was "played for X": a Buff Billy with `CardInstance.x` set (§2.3).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GLITCH: &str = "classic-018";
    const NUMBERS: &[&str] = &["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10"];
    const REPLENISH: &str = "core-010"; // (0) Spell
    const VANILLA: &str = "core-008"; // (1) Unit
    const STOCKPILE: &str = "core-005"; // (1) Spell
    const FELINORS: &str = "core-012"; // (2) Unit
    const EXPERIMENT: &str = "core-085"; // (2) Trap
    const TWINSPELL: &str = "core-079"; // (2) Field Spell
    const MENACE: &str = "core-019"; // (3) Unit
    const MANA_WELL: &str = "core-006"; // (3) Field Spell
    const SEVEN: &str = "core-025"; // (4) Unit
    const ROCK: &str = "core-066"; // (4) Unit, Indestructible, Tribute 1
    const DIVIDEND: &str = "core-024"; // (X) Spell
    const BILLY: &str = "classicplus-069"; // (X) Unit
    const TOE_CRACKER: &str = "classic-006"; // (2) Unit: "Aura: Your Traps cost (0)."

    fn glitch_board(radiant_face: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": GLITCH, "radiant": radiant_face }, FELINORS, VANILLA],
                "field": [FELINORS, MENACE],
                "backrow": [{ "def": EXPERIMENT, "faceUp": false }, MANA_WELL],
                "library": [FELINORS, SEVEN],
                "graveyard": [FELINORS],
                "exile": [FELINORS],
            },
            "p2": {
                "hand": [FELINORS, STOCKPILE],
                "field": [VANILLA, FELINORS],
                "backrow": [TWINSPELL],
                "library": [REPLENISH, FELINORS],
                "graveyard": [FELINORS],
                "exile": [FELINORS],
            },
        }))
    }

    /// One side's Felinors by zone, compared whole.
    #[derive(Debug, PartialEq, Eq)]
    struct Felinors {
        field: usize,
        hand: usize,
        library: usize,
        graveyard: usize,
        exile: usize,
    }

    fn felinors_in(s: &Scenario, player: PlayerId) -> Felinors {
        let count = |zone: &str| -> usize { s.pile(player, zone).iter().filter(|card| card.def_id == FELINORS).count() };
        let field = [1, 2, 3, 4, 5]
            .iter()
            .filter(|&&lane| s.unit(player, lane).map(|card| card.def_id) == Some(FELINORS.to_string()))
            .count();
        Felinors {
            field,
            hand: count("hand"),
            library: count("library"),
            graveyard: count("graveyard"),
            exile: count("exile"),
        }
    }

    /// The `modes` of every `play` action `legalActions` offers for this instance.
    fn offered_modes(s: &Scenario, instance_id: &str) -> Vec<Vec<String>> {
        legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play { instance_id: id, modes, .. } if id == instance_id => Some(modes.unwrap_or_default()),
                _ => None,
            })
            .collect()
    }

    fn numbers_as_modes() -> Vec<Vec<String>> {
        NUMBERS.iter().map(|number| vec![number.to_string()]).collect()
    }

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    fn exiled_events(events: &[GameEvent]) -> Vec<GameEvent> {
        events.iter().filter(|event| event.event_type() == GameEventType::Exiled).cloned().collect()
    }

    mod c_n18_glitch_in_the_system {
        use super::*;

        #[test]
        fn declares_no_numbers_and_the_number_choice_as_a_mode_of_kind_number_0_to_10() {
            crate::register_all();
            let def = crate::card_def(ID);
            let scripts = script();
            assert!(def.params.is_none());
            assert_eq!(
                serde_json::to_value(&scripts.base.modes).unwrap(),
                json!([{ "kind": "number", "options": NUMBERS }])
            );
            assert_eq!(scripts.radiant.modes, scripts.base.modes);
            assert!(scripts.base.targets.is_empty());
        }

        mod base {
            use super::*;

            #[test]
            fn r81_the_number_is_chosen_with_the_play_legalactions_offers_the_same_eleven_every_time() {
                crate::register_all();
                let full = glitch_board(false);
                let empty = scenario(json!({ "p1": { "hand": [GLITCH, STOCKPILE] }, "p2": { "hand": [STOCKPILE] } }));
                for s in [&full, &empty] {
                    let glitch = s.card(GLITCH).clone();
                    let offered = offered_modes(s, &glitch.id);
                    assert_eq!(offered, numbers_as_modes());
                }
            }

            #[test]
            fn a_number_outside_the_list_is_refused() {
                crate::register_all();
                let mut s = glitch_board(false);
                s.expect_refused(|s| s.play(GLITCH, json!({ "modes": ["11"] })));
                s.expect_refused(|s| s.play(GLITCH, json!({ "modes": [] })));
            }

            #[test]
            fn exiles_every_card_of_that_cost_on_both_fields_and_nothing_anywhere_else() {
                crate::register_all();
                let mut s = glitch_board(false);
                s.play(GLITCH, json!({ "modes": ["2"] }));
                assert_eq!(felinors_in(&s, P1), Felinors { field: 0, hand: 1, library: 1, graveyard: 1, exile: 2 });
                assert_eq!(felinors_in(&s, P2), Felinors { field: 0, hand: 1, library: 1, graveyard: 1, exile: 2 });
                // The face-down (2) Trap and the (2) Field Spell go too.
                assert!(s.backrow(P1, 1).is_none());
                assert!(s.backrow(P2, 1).is_none());
                // Other costs stay.
                assert_eq!(s.unit(P1, 2).map(|card| card.def_id), Some(MENACE.to_string()));
                assert_eq!(s.unit(P2, 1).map(|card| card.def_id), Some(VANILLA.to_string()));
                assert_eq!(def_ids(&s.hand(P1)), vec![FELINORS, VANILLA]);
                assert_eq!(def_ids(&s.hand(P2)), vec![FELINORS, STOCKPILE]);
            }

            #[test]
            fn graveyards_and_exile_are_untouched_their_2_cost_cards_stay_where_they_are() {
                crate::register_all();
                let mut s = glitch_board(false);
                let mut graves = s.pile(P1, "graveyard");
                graves.extend(s.pile(P2, "graveyard"));
                s.play(GLITCH, json!({ "modes": ["2"] }));
                for card in &graves {
                    s.expect_in_zone(card, "graveyard");
                }
            }

            #[test]
            fn the_spell_itself_is_resolving_and_spared_though_it_costs_3() {
                crate::register_all();
                let mut s = glitch_board(false);
                let glitch = s.card(GLITCH).clone();
                s.play(GLITCH, json!({ "modes": ["3"] }));
                s.expect_in_zone(&glitch, "graveyard");
                assert!(s.unit(P1, 2).is_none());
                assert!(s.backrow(P1, 2).is_none());
            }

            #[test]
            fn face_down_and_indestructible_cards_are_included() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [GLITCH, STOCKPILE], "field": [ROCK] },
                    "p2": { "hand": [STOCKPILE], "field": [SEVEN], "backrow": [{ "def": EXPERIMENT, "faceUp": false }] },
                }));
                let rock = s.card(ROCK).clone();
                s.play(GLITCH, json!({ "modes": ["4"] }));
                s.expect_in_zone(&rock, "exile");
                s.expect_in_zone(SEVEN, "exile");
            }

            #[test]
            fn r65_costs_at_resolution_a_set_card_at_its_own_cost_a_2_trap_under_toe_cracker_costs_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [GLITCH, STOCKPILE], "field": [TOE_CRACKER], "backrow": [{ "def": EXPERIMENT, "faceUp": false }] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                let trap = s.card(EXPERIMENT).clone();
                s.play(GLITCH, json!({ "modes": ["0"] }));
                s.expect_in_zone(&trap, "field");
                let mut two = scenario(json!({
                    "p1": { "hand": [GLITCH, STOCKPILE], "field": [TOE_CRACKER], "backrow": [{ "def": EXPERIMENT, "faceUp": false }] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                two.play(GLITCH, json!({ "modes": ["2"] }));
                two.expect_in_zone(EXPERIMENT, "exile");
            }

            #[test]
            fn r65_a_costmod_counts_on_the_field_the_menace_at_2_goes_with_a_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [GLITCH, STOCKPILE], "field": [{ "def": MENACE, "costMod": -1 }] },
                    "p2": { "hand": [STOCKPILE], "field": [{ "def": SEVEN, "costMod": -2 }] },
                }));
                s.play(GLITCH, json!({ "modes": ["2"] }));
                s.expect_in_zone(MENACE, "exile");
                s.expect_in_zone(SEVEN, "exile");
            }

            #[test]
            fn r396_an_x_card_on_the_field_costs_its_x_played_for_3_a_3_exiles_it_and_a_0_does_not() {
                crate::register_all();
                let mut three = scenario(json!({
                    "p1": { "hand": [GLITCH, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE], "field": [{ "def": BILLY, "statsOverride": { "attack": 9, "health": 9 } }] },
                }));
                let billy = three.card(BILLY).id.clone();
                find_instance_mut(three.state_mut(), &billy).unwrap().x = Some(3);
                three.play(GLITCH, json!({ "modes": ["3"] }));
                three.expect_in_zone(BILLY, "exile");

                let mut zero = scenario(json!({
                    "p1": { "hand": [GLITCH, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE], "field": [{ "def": BILLY, "statsOverride": { "attack": 9, "health": 9 } }] },
                }));
                let billy = zero.card(BILLY).id.clone();
                find_instance_mut(zero.state_mut(), &billy).unwrap().x = Some(3);
                zero.play(GLITCH, json!({ "modes": ["0"] }));
                zero.expect_in_zone(BILLY, "field");
            }

            #[test]
            fn r396_an_x_card_on_the_field_with_no_x_chosen_costs_0() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [GLITCH, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE], "field": [{ "def": BILLY, "statsOverride": { "attack": 3, "health": 3 } }] },
                }));
                let billy = s.card(BILLY).clone();
                s.play(GLITCH, json!({ "modes": ["0"] }));
                s.expect_in_zone(&billy, "exile");
            }

            #[test]
            fn a_number_nothing_costs_exiles_nothing() {
                crate::register_all();
                let mut s = glitch_board(false);
                s.play(GLITCH, json!({ "modes": ["10"] }));
                assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Exiled));
                s.expect_in_zone(GLITCH, "graveyard");
            }

            #[test]
            fn the_exiled_cards_are_public_to_both_players() {
                crate::register_all();
                let mut s = glitch_board(false);
                s.play(GLITCH, json!({ "modes": ["2"] }));
                let exiled = exiled_events(s.events());
                assert_eq!(exiled.len(), 4);
                for event in &exiled {
                    let value = serde_json::to_value(event).unwrap();
                    let mut keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();
                    keys.sort();
                    assert_eq!(keys, vec!["defId", "instanceId", "owner", "type"]);
                }
                for viewer in [P1, P2] {
                    assert_eq!(exiled_events(&s.view(viewer).events), exiled);
                }
            }

            #[test]
            fn r13_a_card_dormant_under_a_stack_pile_is_not_on_the_field() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [GLITCH, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE], "field": [FELINORS, { "def": MENACE, "stack": true }] },
                }));
                let felinors = s.card(FELINORS).clone();
                s.play(GLITCH, json!({ "modes": ["2"] }));
                s.expect_in_zone(&felinors, "field");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn exiles_every_card_of_that_cost_on_the_opponent_s_field_in_their_hand_and_in_their_deck_only() {
                crate::register_all();
                let mut s = glitch_board(true);
                s.play(GLITCH, json!({ "modes": ["2"] }));
                assert_eq!(felinors_in(&s, P2), Felinors { field: 0, hand: 0, library: 0, graveyard: 1, exile: 4 });
                assert!(s.backrow(P2, 1).is_none());
                // Yours are untouched.
                assert_eq!(felinors_in(&s, P1), Felinors { field: 1, hand: 1, library: 1, graveyard: 1, exile: 1 });
                assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some(EXPERIMENT.to_string()));
            }

            #[test]
            fn offers_the_same_eleven_numbers() {
                crate::register_all();
                let s = glitch_board(true);
                let glitch = s.card(GLITCH).clone();
                let offered = offered_modes(&s, &glitch.id);
                assert_eq!(offered, numbers_as_modes());
            }

            #[test]
            fn their_graveyard_and_exile_are_untouched_and_a_number_nothing_of_theirs_costs_exiles_nothing() {
                crate::register_all();
                let mut s = glitch_board(true);
                s.play(GLITCH, json!({ "modes": ["4"] }));
                assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Exiled));
                assert_eq!(felinors_in(&s, P2), Felinors { field: 1, hand: 1, library: 1, graveyard: 1, exile: 1 });
            }

            #[test]
            fn r65_a_hand_card_at_its_hand_cost_a_2_trap_under_toe_cracker_costs_0() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": GLITCH, "radiant": true }, STOCKPILE] },
                    "p2": { "hand": [EXPERIMENT, STOCKPILE], "field": [TOE_CRACKER], "library": [REPLENISH] },
                }));
                s.play(GLITCH, json!({ "modes": ["0"] }));
                // The opponent's hand Trap costs (0) under their Toe Cracker, so the 0 takes it.
                s.expect_in_zone(EXPERIMENT, "exile");
            }

            #[test]
            fn r65_a_deck_card_at_its_own_cost_a_2_trap_in_the_deck_costs_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": GLITCH, "radiant": true }, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE], "field": [TOE_CRACKER], "library": [EXPERIMENT] },
                }));
                s.play(GLITCH, json!({ "modes": ["0"] }));
                s.expect_in_zone(EXPERIMENT, "library");
                let mut two = scenario(json!({
                    "p1": { "hand": [{ "def": GLITCH, "radiant": true }, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE], "field": [TOE_CRACKER], "library": [EXPERIMENT] },
                }));
                two.play(GLITCH, json!({ "modes": ["2"] }));
                two.expect_in_zone(EXPERIMENT, "exile");
            }

            #[test]
            fn r396_an_x_card_in_a_hand_or_a_deck_costs_0() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": GLITCH, "radiant": true }, STOCKPILE] },
                    "p2": { "hand": [DIVIDEND, STOCKPILE], "library": [BILLY] },
                }));
                s.play(GLITCH, json!({ "modes": ["0"] }));
                s.expect_in_zone(DIVIDEND, "exile");
                s.expect_in_zone(BILLY, "exile");
            }
        }
    }
}
