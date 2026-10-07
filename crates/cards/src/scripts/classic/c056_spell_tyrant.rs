//! C #56 Spell Tyrant (SPEC §8.6 row 56; §6.3 Cast, §10.6; R70, R386). Unit 5/5 → 10/10, cost 4, Legendary.
//!   Base:    "Cry: Choose up to {spells|Spell|Spells} in your graveyard. Cast them, then exile them." (3)
//!   Radiant: "Cry: Cast every Spell in your graveyard, oldest first, then exile them."
//!   Engine:  casts from the graveyard one at a time, each free, counted as played and with your choices
//!            (R70), each exiled after it resolves instead of going to the graveyard. The base face's
//!            three are your pick as the Cry resolves (a `pick` prompt, its budget `spells` cards); the
//!            Radiant face casts the Spells there as the Cry begins, oldest first, so a Spell a cast puts
//!            in the graveyard is not cast. "Spell" is the Spell type.

use jackioh_engine::effects::{CastEachArgs, cast_each, choose_pick};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-056";

/// R453: `castEach` with "then exile them" (`afterward: "exile"`), each cast's choices its caster's (R70).
/// `cards` answers the instance ids to cast, read once as the list reaches it.
fn cast_each_then_exile(cards: impl Fn(&EffectContext<'_>) -> Vec<String> + Send + Sync + 'static) -> Effect {
    cast_each(CastEachArgs {
        cards: Arc::new(cards),
        how: json_as(json!({ "afterward": "exile" })),
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![choose_pick(json_as(json!({
                "step": "cast",
                "from": [{ "zone": "graveyard" }],
                "filter": { "type": "Spell" },
                "max": param(ctx, "spells"),
                "prompt": "Choose Spells in your graveyard to cast",
            })))]
        })),
        resume: IndexMap::from([(
            "cast",
            hook(|ctx| {
                let picked: Vec<String> = ctx
                    .targets
                    .iter()
                    .filter_map(|selection| match selection {
                        Selection::Instance { instance_id } => Some(instance_id.clone()),
                        _ => None,
                    })
                    .collect();
                vec![cast_each_then_exile(move |_ctx| picked.clone())]
            }),
        )]),
        ..Script::default()
    };

    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![cast_each_then_exile(|ctx| {
                zone_cards(&ctx.state, ctx.controller, OffFieldZone::Graveyard)
                    .into_iter()
                    .filter(|card| card_type_of(&ctx.state, card) == CardType::Spell)
                    .map(|card| card.id.clone())
                    .collect()
            })]
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #56 Spell Tyrant — SPEC §8.6 row 56, BUILD M9 Classic row C 56: "Cry: choose up to 3 Spells (the
// Spell type) in your graveyard and cast each in turn (free, counted as played, your choices, R70),
// each exiled after it resolves instead of returning to the graveyard; fewer than 3 → those there;
// none → nothing; radiant 10/10: every Spell there as the Cry begins, oldest first, with no choice; a
// Spell a cast puts in the graveyard is not cast; its tuned number (spells) reads through `param()`
// (R386)".
//
// The Spells are Core cards with their own tests: Stockpile (draw 2, heal 2), Friend of Felinors
// (fill your board with Felinor Tokens), 5pek Controller (switch every Unit's position), Lunar Eclipse
// (3 damage to a target, chosen as the cast begins) and Zao Gao (discard 2 at random). Mana Well is a
// Field Spell and Mr. Vanilla a Unit, neither of them a Spell.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TYRANT: &str = "classic-056";
    const STOCKPILE: &str = "core-005"; // (1) Spell: draw 2, heal your hero 2.
    const FRIENDS: &str = "core-062"; // (1) Spell: fill your board with Felinor Tokens.
    const SPEK: &str = "core-048"; // (0) Spell: switch the position of every Unit.
    const LUNAR: &str = "core-035"; // (1) Spell: deal 3 damage to a target.
    const ZAO_GAO: &str = "core-080"; // (2) Spell: discard 2 random cards; summon 2 Rush Tokens.
    const PILE_ON: &str = "classic-060"; // C #60 (5) Spell: Recruit every permanent in your deck; if this would go to your graveyard, bottom of your deck instead.
    const MANA_WELL: &str = "core-006"; // Field Spell
    const VANILLA: &str = "core-008"; // Unit
    const MENACE: &str = "core-019"; // 9/9
    const FILLER: &str = "core-016"; // Hit Job: a Spell to hold.
    const DECK: [&str; 4] = [FILLER, FILLER, FILLER, FILLER];

    /// An engine value as the JSON the TS test reads (SURFACE §5.1: the same keys and values).
    fn js<T: serde::Serialize + ?Sized>(value: &T) -> Value {
        serde_json::to_value(value).expect("an engine value serialises")
    }

    /// TS `JSON.parse(JSON.stringify(state))`: written and read back field by field, in field order.
    fn round_trip(state: &GameState) -> GameState {
        let text = serde_json::to_string(state).expect("the state serialises");
        serde_json::from_str(&text).expect("the state parses back")
    }

    fn played(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .map(js)
            .filter(|event| event["type"] == "cardPlayed")
            .filter_map(|event| event["defId"].as_str().map(str::to_string))
            .collect()
    }

    fn ids(s: &Scenario, def_ids: &[&str]) -> Vec<String> {
        def_ids.iter().map(|def_id| s.card(*def_id).id.clone()).collect()
    }

    fn def_ids(cards: Vec<CardInstance>) -> Vec<String> {
        cards.into_iter().map(|card| card.def_id).collect()
    }

    mod c_56_spell_tyrant {
        use super::*;

        #[test]
        fn declares_its_number_spells_the_base_face_picks_the_radiant_face_casts_them_all() {
            crate::register_all();
            let def = js(&crate::card_def(ID));
            assert_eq!(def["id"], TYRANT);
            assert_eq!(
                def["params"],
                json!([{ "key": "spells", "base": 3, "radiant": 3, "better": "up", "step": 1, "min": 1 }]),
            );
            let scripts = script();
            assert!(scripts.base.resume.contains_key("cast"));
            assert!(scripts.radiant.resume.is_empty());
        }

        mod base {
            use super::*;

            #[test]
            fn cry_offers_you_the_spells_of_your_graveyard_no_field_spell_no_unit_up_to_3() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TYRANT, FILLER], "graveyard": [STOCKPILE, VANILLA, MANA_WELL, SPEK, FRIENDS, LUNAR], "library": DECK },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(TYRANT, json!({}));
                let pending = js(&s.state().pending);
                assert_eq!(pending["playerId"], "p1");
                assert_eq!(pending["kind"], "pick");
                assert_eq!(pending["min"], 0);
                assert_eq!(pending["max"], 3);
                let selections: Vec<Value> = pending["options"]
                    .as_array()
                    .map(|options| options.iter().map(|option| option["selection"].clone()).collect())
                    .unwrap_or_default();
                let expected: Vec<Value> = ids(&s, &[STOCKPILE, SPEK, FRIENDS, LUNAR])
                    .into_iter()
                    .map(|instance_id| json!({ "pick": "instance", "instanceId": instance_id }))
                    .collect();
                assert_eq!(selections, expected);
                assert_eq!(js(&s.view(P2).pending), json!({ "forYou": false, "pendingFor": "p1" }));
            }

            #[test]
            fn legalactions_offers_only_spells_at_most_3_and_an_answer_naming_anything_else_is_refused() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TYRANT, FILLER], "graveyard": [STOCKPILE, VANILLA, MANA_WELL, SPEK, FRIENDS, LUNAR], "library": DECK },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(TYRANT, json!({}));
                let spells = ids(&s, &[STOCKPILE, SPEK, FRIENDS, LUNAR]);
                let answers: Vec<Value> = legal_actions(s.state(), P1)
                    .iter()
                    .map(js)
                    .filter(|action| action["type"] == "answer")
                    .map(|action| action["selection"].clone())
                    .collect();
                assert!(!answers.is_empty());
                for selection in &answers {
                    let each = selection.as_array().cloned().unwrap_or_default();
                    assert!(each.len() <= 3);
                    for one in each {
                        assert!(
                            one["pick"] == "instance"
                                && one["instanceId"].as_str().is_some_and(|id| spells.iter().any(|spell| spell == id))
                        );
                    }
                }
                let vanilla = ids(&s, &[VANILLA]);
                let well = ids(&s, &[MANA_WELL]);
                let four = ids(&s, &[STOCKPILE, SPEK, FRIENDS, LUNAR]);
                s.expect_refused(|s| s.answer(json!(vanilla)));
                s.expect_refused(|s| s.answer(json!(well)));
                s.expect_refused(|s| s.answer(json!(four)));
                assert_eq!(js(&s.state().pending)["kind"], "pick");
            }

            #[test]
            fn r70_casts_each_pick_in_turn_free_and_counted_as_played_and_exiles_each_after_it_resolves() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TYRANT, FILLER], "graveyard": [STOCKPILE, SPEK, FRIENDS], "library": DECK, "health": 20 },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(TYRANT, json!({}));
                let picks = ids(&s, &[STOCKPILE, FRIENDS]);
                s.answer(json!(picks));
                assert_eq!(played(&s), vec![TYRANT, STOCKPILE, FRIENDS]);
                assert_eq!(cards_played_this_turn(s.state(), P1), 3);
                s.expect_mana(P1, 0);
                // Stockpile drew 2 and healed 2; Friend of Felinors filled the other four zones.
                s.expect_health(P1, 22);
                assert_eq!(s.hand(P1).len(), 3);
                assert_eq!(
                    (2..=5).map(|lane| s.unit(P1, lane).map(|unit| unit.def_id)).collect::<Vec<Option<String>>>(),
                    vec![Some("core-t-felinor".to_string()); 4],
                );
                assert_eq!(def_ids(s.pile(P1, "exile")), vec![STOCKPILE, FRIENDS]);
                assert_eq!(def_ids(s.pile(P1, "graveyard")), vec![SPEK]);
            }

            #[test]
            fn r221_the_picks_are_cast_in_the_order_the_graveyard_holds_them_oldest_first_however_the_answer_lists_them() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TYRANT, FILLER], "graveyard": [SPEK, STOCKPILE, FRIENDS], "library": DECK },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(TYRANT, json!({}));
                let picks = ids(&s, &[FRIENDS, SPEK]);
                s.answer(json!(picks));
                assert_eq!(played(&s), vec![TYRANT, SPEK, FRIENDS]);
            }

            #[test]
            fn up_to_you_may_cast_fewer_or_none_and_the_rest_stay_in_your_graveyard() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [TYRANT, FILLER], "graveyard": [STOCKPILE, SPEK], "library": DECK }, "p2": { "hand": [FILLER] } }));
                s.play(TYRANT, json!({}));
                s.answer(json!([]));
                assert_eq!(played(&s), vec![TYRANT]);
                assert_eq!(def_ids(s.pile(P1, "graveyard")), vec![STOCKPILE, SPEK]);
            }

            #[test]
            fn fewer_than_3_spells_there_the_pick_offers_those_there() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [TYRANT, FILLER], "graveyard": [STOCKPILE, VANILLA], "library": DECK }, "p2": { "hand": [FILLER] } }));
                s.play(TYRANT, json!({}));
                assert_eq!(js(&s.state().pending)["max"], 1);
                let picks = ids(&s, &[STOCKPILE]);
                s.answer(json!(picks));
                s.expect_in_zone(STOCKPILE, "exile");
            }

            #[test]
            fn none_there_nothing_is_asked_and_nothing_is_cast() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [TYRANT, FILLER], "graveyard": [VANILLA, MANA_WELL] }, "p2": { "hand": [FILLER] } }));
                s.play(TYRANT, json!({}));
                assert!(s.state().pending.is_none());
                assert_eq!(played(&s), vec![TYRANT]);
                s.expect_in_zone(TYRANT, "field");
            }

            #[test]
            fn r70_a_cast_spells_choices_are_yours_asked_as_its_cast_begins_then_the_next_pick_is_cast() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TYRANT, FILLER], "graveyard": [LUNAR, STOCKPILE], "library": DECK },
                    "p2": { "hand": [FILLER], "field": [MENACE] },
                }));
                s.play(TYRANT, json!({}));
                let picks = ids(&s, &[LUNAR, STOCKPILE]);
                s.answer(json!(picks));
                assert_eq!(js(&s.state().pending)["playerId"], "p1");
                let menace = s.card(MENACE).id.clone();
                s.answer(json!(menace));
                s.expect_stats(MENACE, json!({ "health": 6 }));
                assert_eq!(played(&s), vec![TYRANT, LUNAR, STOCKPILE]);
                assert_eq!(def_ids(s.pile(P1, "exile")), vec![LUNAR, STOCKPILE]);
            }

            #[test]
            fn a_cast_paused_on_its_choice_survives_a_round_trip_the_frozen_state_answers_to_the_same_game() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TYRANT, FILLER], "graveyard": [LUNAR, STOCKPILE], "library": DECK },
                    "p2": { "hand": [FILLER], "field": [MENACE] },
                }));
                s.play(TYRANT, json!({}));
                let picks = ids(&s, &[LUNAR, STOCKPILE]);
                s.answer(json!(picks));
                let pending = s.state().pending.clone();
                assert!(pending.is_some());
                let thawed = round_trip(s.state());
                let answer: Action = json_as(json!({
                    "type": "answer",
                    "choiceId": pending.as_ref().map(|choice| choice.id.clone()).unwrap_or_default(),
                    "selection": [{ "pick": "hero", "player": "p2" }],
                    "playerId": "p1",
                    "nonce": "tyrant-roundtrip",
                }));
                let live = reduce(s.state(), &answer);
                let frozen = reduce(&thawed, &answer);
                assert!(live.error.is_none());
                assert_eq!(frozen.state, live.state);
                assert_eq!(frozen.events, live.events);
                assert_eq!(live.state.players.p2.hero.health, 27);
                assert_eq!(def_ids(live.state.players.p1.exile.clone()), vec![LUNAR, STOCKPILE]);
            }

            #[test]
            fn a_cast_it_exiles_never_goes_to_the_graveyard_c_60_pile_ons_return_to_the_deck_does_not_apply() {
                crate::register_all();
                // A full deck turns Pile On's return away (R80), which is how it reaches the graveyard to start
                // with; the cast's Recruit then makes room, so only the exile keeps it out of the deck.
                let mut full = vec![VANILLA];
                full.extend((0..LIBRARY_CAP - 1).map(|_| FILLER));
                let mut s = scenario(json!({ "p1": { "hand": [TYRANT, FILLER], "graveyard": [PILE_ON], "library": full }, "p2": { "hand": [FILLER] } }));
                s.expect_in_zone(PILE_ON, "graveyard");
                s.play(TYRANT, json!({}));
                let picks = ids(&s, &[PILE_ON]);
                s.answer(json!(picks));
                assert_eq!(played(&s), vec![TYRANT, PILE_ON]);
                assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id), Some(VANILLA.to_string()));
                s.expect_in_zone(PILE_ON, "exile");
                assert_eq!(s.pile(P1, "library").len(), (LIBRARY_CAP - 1) as usize);
            }

            #[test]
            fn r386_an_upgrade_offers_4_picks_a_degrade_2() {
                crate::register_all();
                let grave = [STOCKPILE, SPEK, FRIENDS, LUNAR, STOCKPILE];
                let mut up = scenario(json!({ "p1": { "hand": [TYRANT, FILLER], "graveyard": grave, "library": DECK }, "p2": { "hand": [FILLER] } }));
                step_param(up.card_mut(TYRANT), "spells", 1);
                up.play(TYRANT, json!({}));
                assert_eq!(js(&up.state().pending)["max"], 4);

                let mut down = scenario(json!({ "p1": { "hand": [TYRANT, FILLER], "graveyard": grave, "library": DECK }, "p2": { "hand": [FILLER] } }));
                step_param(down.card_mut(TYRANT), "spells", -1);
                down.play(TYRANT, json!({}));
                assert_eq!(js(&down.state().pending)["max"], 2);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn casts_every_spell_in_your_graveyard_as_the_cry_begins_oldest_first_with_no_choice_and_exiles_each() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [{ "def": TYRANT, "radiant": true }, FILLER],
                        "graveyard": [STOCKPILE, VANILLA, SPEK, MANA_WELL, FRIENDS],
                        "library": DECK,
                    },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(TYRANT, json!({}));
                assert_eq!(played(&s), vec![TYRANT, STOCKPILE, SPEK, FRIENDS]);
                assert_eq!(def_ids(s.pile(P1, "exile")), vec![STOCKPILE, SPEK, FRIENDS]);
                assert_eq!(def_ids(s.pile(P1, "graveyard")), vec![VANILLA, MANA_WELL]);
                s.expect_stats(TYRANT, json!({ "attack": 10, "health": 10 }));
                assert_eq!(cards_played_this_turn(s.state(), P1), 4);
            }

            #[test]
            fn a_spell_a_cast_puts_in_the_graveyard_is_not_cast_zao_gaos_discarded_lunar_eclipse_stays_there() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": TYRANT, "radiant": true }, LUNAR, VANILLA], "graveyard": [ZAO_GAO] },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(TYRANT, json!({}));
                assert_eq!(played(&s), vec![TYRANT, ZAO_GAO]);
                s.expect_in_zone(LUNAR, "graveyard").expect_in_zone(ZAO_GAO, "exile");
            }

            #[test]
            fn r70_a_cast_spells_choices_are_still_yours() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": TYRANT, "radiant": true }, FILLER], "graveyard": [LUNAR] },
                    "p2": { "hand": [FILLER], "field": [MENACE] },
                }));
                s.play(TYRANT, json!({}));
                assert_eq!(js(&s.state().pending)["playerId"], "p1");
                let menace = s.card(MENACE).id.clone();
                s.answer(json!(menace));
                s.expect_stats(MENACE, json!({ "health": 6 }));
                s.expect_in_zone(LUNAR, "exile");
            }

            #[test]
            fn with_no_spell_in_your_graveyard_it_casts_nothing() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": TYRANT, "radiant": true }, FILLER], "graveyard": [VANILLA] }, "p2": { "hand": [FILLER] } }));
                s.play(TYRANT, json!({}));
                assert_eq!(played(&s), vec![TYRANT]);
                assert!(s.state().pending.is_none());
            }
        }
    }
}
