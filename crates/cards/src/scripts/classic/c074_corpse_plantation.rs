//! C #74 Corpse Plantation (SPEC §8.6 row 74). (2) Field Spell, Epic.
//!   Base:    "Activate: Place {tokens|Plague Counter|Plague Counters} on this.
//!             You may play Units from your graveyard, paying with Plague Counters from this: each counter
//!             pays (1), and each such play spends at least 1 counter." — 2 tokens; Radiant: 4.
//!   Engine:  "Play from the graveyard (§6.3 Play) for Units, live while this is on the field, with a
//!            second way to pay: the `play` action carries how many tokens pay (at least 1, at most the
//!            tokens on this and the price), the rest in mana, and each paying token is removed from this
//!            card; so a Unit that costs (0) can't be played this way. … its choices and counting are as
//!            from hand, and R65's player discounts apply. Tunes: tokens 2 ↑."
//!
//! The token placement is an Activate ability, once per turn (R384): one placement of {tokens} on
//! itself (R386's declared number). The permission is `graveyardPlay` with `units` and `plague`: the
//! engine offers and checks the token payment, takes the card from the graveyard as a play (its Cry
//! fires, it counts as played; not R70's free cast) and removes the spent tokens from this card.
//! Tokens other cards place here pay too; R78 clears them when it leaves.

use jackioh_engine::effects::place_plague;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-074";

/// TS `const plant: ActivationDecl`.
fn plant() -> ActivationDecl {
    ActivationDecl {
        id: "plant".to_string(),
        label: "Place Plague Counters on this".to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(|ctx| {
            vec![place_plague(json_as(json!({
                "target": { "of": "self" },
                "amount": param(&*ctx, "tokens"),
            })))]
        }),
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        activations: vec![plant()],
        graveyard_play: Some(read_hook(|_args| {
            vec![GraveyardPlayPermission {
                units: Some(true),
                plague: Some(true),
                ..GraveyardPlayPermission::default()
            }]
        })),
        ..Script::default()
    };
    CardScripts {
        // The same script: the Radiant face's 4 tokens are its declared `tokens`.
        radiant: base.clone(),
        base,
    }
}

// C #74 Corpse Plantation — SPEC §8.6 row 74, BUILD M9 Classic row C 74: "Activate: one placement of 2
// Plague Counters on itself, once per turn; while it has tokens, `legalActions` offers `play` for Units in
// your graveyard, the action carrying how many tokens pay, at least 1 and at most the tokens on it and
// the price, each paying (1) and the rest paid in mana; it is a play (the Unit's Cry fires and it counts
// as played), not a free cast; a (0) Cost Unit can't use it; with no tokens left it offers nothing;
// tokens others place add to it, and leaving the field resets them (R78); radiant: 4 tokens; its tuned
// number (tokens) reads through `param()` (R386)".
//
// The harness's `play` takes a card from a hand, so a graveyard play is sent to `reduce` as
// `legalActions` offers it and read back off its result (as C #28 Second Wind's test does).

/// `describe("C #74 Corpse Plantation")`.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const PLANTATION: &str = "classic-074";
    const CRAWLER: &str = "classic-053"; // (1) Unit: Cry: place a Plague Counter on another permanent.
    const SLIME: &str = "classic-027"; // (0) Unit 1/1.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const MR_TOKEN: &str = "core-015"; // (1) Unit 1/1: Cry: Summon a Rush Token.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const COLLATERAL: &str = "core-034"; // (4) Spell: Exile target permanent and a random card from your opponent's deck.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).
    const RUSH_TOKEN: &str = "core-t-rush";

    /// `standing`'s default graveyard: Units and a Spell.
    const GRAVEYARD: [&str; 4] = [MENACE, MR_TOKEN, STOCKPILE, SLIME];

    use crate::scenario;

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialises")
    }

    /// TS `type Step = { state; events } & { error? }`.
    struct Step {
        state: GameState,
        events: Vec<GameEvent>,
        error: Option<String>,
    }

    fn graveyard_plays(state: &GameState, card: &CardInstance) -> Vec<ActionBody> {
        legal_actions(state, P1)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == card.id))
            .collect()
    }

    /// The token counts legalActions offers for a play of `card`, each once.
    fn token_options(state: &GameState, card: &CardInstance) -> Vec<Option<i32>> {
        let offered: IndexSet<Option<i32>> = graveyard_plays(state, card)
            .into_iter()
            .map(|play| match play {
                ActionBody::Play { plague, .. } => plague.map(|spend| spend.tokens),
                _ => None,
            })
            .collect();
        let mut options: Vec<Option<i32>> = offered.into_iter().collect();
        // TS's comparator-less `.sort()` compares the values as strings ("1" < "2" < "null").
        options.sort_by_key(|option| option.map_or_else(|| "null".to_string(), |tokens| tokens.to_string()));
        options
    }

    /// TS `send(state, body)`: a `play` from p1 with the body's fields. TS numbered its nonces with a module
    /// counter; here each one is numbered by the state's applied actions, which is as unique along a game.
    fn send(state: &GameState, body: Value) -> Step {
        let mut action = json!({ "type": "play", "playerId": "p1" });
        if let (Some(into), Some(from)) = (action.as_object_mut(), body.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        action["nonce"] = json!(format!("plantation-{}", state.applied.len() + 1));
        let result = reduce(state, &json_as::<Action>(action));
        Step {
            state: result.state,
            events: result.events,
            error: result.error,
        }
    }

    fn play_from_graveyard(state: &GameState, card: &CardInstance, from: &CardInstance, tokens: i32, targets: Option<Value>) -> Step {
        let mut body = json!({ "instanceId": card.id, "plague": { "from": from.id, "tokens": tokens } });
        if let Some(targets) = targets {
            body["targets"] = targets;
        }
        let result = send(state, body);
        if let Some(error) = &result.error {
            panic!("{error}");
        }
        result
    }

    fn tokens_on(state: &GameState, card: &CardInstance) -> i32 {
        find_instance(state, &card.id)
            .and_then(|found| found.counters.plague)
            .unwrap_or(0)
    }

    fn counter_changes(events: &[GameEvent]) -> Vec<Value> {
        events
            .iter()
            .filter(|event| event.event_type() == GameEventType::CounterChanged)
            .map(js)
            .collect()
    }

    /// The Plantation already standing with `tokens` on it, and a graveyard of Units and a Spell.
    fn standing(tokens: i32, graveyard: &[&str], mana: Option<i32>) -> Scenario {
        let counters = if tokens > 0 { json!({ "plague": tokens }) } else { json!({}) };
        let mut p1 = json!({
            "hand": [ANCHOR],
            "backrow": [{ "def": PLANTATION, "counters": counters }],
            "graveyard": graveyard,
        });
        if let Some(mana) = mana {
            p1["mana"] = json!(mana);
        }
        scenario(json!({ "p1": p1, "p2": { "hand": [ANCHOR] } }))
    }

    /// declares its one number and its permission (Units, paid with its tokens), one script on both faces
    #[test]
    fn declares_its_one_number_and_its_permission_units_paid_with_its_tokens_one_script_on_both_faces() {
        let def = crate::card_def(PLANTATION);
        assert_eq!(def.id, PLANTATION);
        assert_eq!(
            js(&def.params),
            json!([{ "key": "tokens", "base": 2, "radiant": 4, "better": "up", "step": 1, "min": 1 }])
        );
        let s = standing(0, &GRAVEYARD, None);
        let scripts = script();
        let grant = scripts.base.graveyard_play.as_ref().expect("a graveyard permission");
        let answer = grant(HookArgs {
            state: s.state(),
            self_: s.card(PLANTATION),
            radiant: false,
        });
        assert_eq!(js(&answer), json!([{ "units": true, "plague": true }]));
        // TS `expect(radiant).toBe(base)`: the Radiant face is the base face, every hook the same one.
        assert!(Arc::ptr_eq(
            grant,
            scripts.radiant.graveyard_play.as_ref().expect("a graveyard permission")
        ));
        assert!(Arc::ptr_eq(&scripts.base.activations[0].run, &scripts.radiant.activations[0].run));
    }

    /// `describe("base")`.
    mod base {
        use super::*;

        /// Activate: one placement of 2 Plague Counters on itself, once per turn
        #[test]
        fn activate_one_placement_of_2_plague_counters_on_itself_once_per_turn() {
            let mut s = scenario(json!({ "p1": { "hand": [PLANTATION, ANCHOR] }, "p2": { "hand": [ANCHOR] } }));

            s.play(PLANTATION, json!({}));

            let plantation = s.card(PLANTATION).clone();
            assert!(plantation.counters.plague.is_none());
            s.activate(&plantation, json!({}));

            assert_eq!(s.card(PLANTATION).counters.plague, Some(2));
            assert_eq!(
                counter_changes(s.events()),
                vec![json!({ "type": "counterChanged", "instanceId": plantation.id, "counter": "plague", "value": 2, "placed": 2 })]
            );
            s.expect_refused(|s| s.activate(&plantation, json!({})));
        }

        /// while it has tokens, legalActions offers each graveyard Unit with 1 up to min(tokens, price) tokens paying, and no Spell
        #[test]
        fn while_it_has_tokens_legal_actions_offers_each_graveyard_unit_with_1_up_to_min_tokens_price_tokens_paying_and_no_spell() {
            let s = standing(2, &GRAVEYARD, None);
            let (menace, mr_token, stockpile) = (s.card(MENACE).clone(), s.card(MR_TOKEN).clone(), s.card(STOCKPILE).clone());

            assert_eq!(token_options(s.state(), &menace), vec![Some(1), Some(2)]);
            assert_eq!(token_options(s.state(), &mr_token), vec![Some(1)]);
            assert!(graveyard_plays(s.state(), &stockpile).is_empty());
            // Every offer spends tokens: the permission pays in tokens and mana, never in mana alone.
            let plantation = s.card(PLANTATION).id.clone();
            assert!(graveyard_plays(s.state(), &menace).iter().all(|play| matches!(
                play,
                ActionBody::Play { plague: Some(spend), .. } if spend.from == plantation
            )));
        }

        /// a play: each counter pays (1), the rest in mana, and the spent tokens come off it
        #[test]
        fn a_play_each_counter_pays_1_the_rest_in_mana_and_the_spent_tokens_come_off_it() {
            let s = standing(2, &GRAVEYARD, None);
            let plantation = s.card(PLANTATION).clone();
            let menace = s.card(MENACE).clone();

            let after = play_from_graveyard(s.state(), &menace, &plantation, 2, None);

            assert_eq!(find_instance(&after.state, &menace.id).map(|card| card.zone.z()), Some(ZoneName::Field));
            assert_eq!(after.state.players.p1.mana.current, 4 - 1);
            assert_eq!(tokens_on(&after.state, &plantation), 0);
            assert_eq!(
                counter_changes(&after.events),
                vec![json!({ "type": "counterChanged", "instanceId": plantation.id, "counter": "plague", "value": 0 })]
            );
        }

        /// it is a play, not a free cast: the Unit's Cry fires and it counts as played
        #[test]
        fn it_is_a_play_not_a_free_cast_the_units_cry_fires_and_it_counts_as_played() {
            let s = standing(2, &GRAVEYARD, None);
            let mr_token = s.card(MR_TOKEN).clone();

            let after = play_from_graveyard(s.state(), &mr_token, s.card(PLANTATION), 1, None);

            assert!(
                after
                    .events
                    .iter()
                    .any(|event| matches!(event, GameEvent::CardPlayed { instance_id, .. } if *instance_id == mr_token.id))
            );
            assert!(
                after
                    .events
                    .iter()
                    .any(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == RUSH_TOKEN))
            );
            assert!(after.state.players.p1.turn_log.played_ids.contains(&mr_token.id));
            assert_eq!(after.state.players.p1.mana.current, 4);
        }

        /// R81 its choices are as from hand: a Plague Crawler from the graveyard declares its target and places on it
        #[test]
        fn r81_its_choices_are_as_from_hand_a_plague_crawler_from_the_graveyard_declares_its_target_and_places_on_it() {
            let s = scenario(json!({
                "p1": { "hand": [ANCHOR], "backrow": [{ "def": PLANTATION, "counters": { "plague": 1 } }], "graveyard": [CRAWLER] },
                "p2": { "hand": [ANCHOR], "field": [VANILLA] },
            }));
            let vanilla = s.card(VANILLA).clone();

            let after = play_from_graveyard(
                s.state(),
                s.card(CRAWLER),
                s.card(PLANTATION),
                1,
                Some(json!([{ "pick": "instance", "instanceId": vanilla.id }])),
            );

            assert_eq!(tokens_on(&after.state, &vanilla), 1);
        }

        /// a (0) Cost Unit can't use it
        #[test]
        fn a_0_cost_unit_cant_use_it() {
            let s = standing(2, &GRAVEYARD, None);
            let slime = s.card(SLIME).clone();

            assert!(graveyard_plays(s.state(), &slime).is_empty());
            let plantation = s.card(PLANTATION).id.clone();
            assert!(
                send(s.state(), json!({ "instanceId": slime.id, "plague": { "from": plantation, "tokens": 1 } }))
                    .error
                    .is_some()
            );
        }

        /// the refusals agree with legalActions: no tokens, too many, more than the price, or mana alone
        #[test]
        fn the_refusals_agree_with_legal_actions_no_tokens_too_many_more_than_the_price_or_mana_alone() {
            let s = standing(2, &GRAVEYARD, None);
            let plantation = s.card(PLANTATION).id.clone();
            let mr_token = s.card(MR_TOKEN).id.clone();
            let menace = s.card(MENACE).id.clone();

            let alone = send(s.state(), json!({ "instanceId": menace })).error;
            assert!(alone.as_deref().is_some_and(|error| error.contains("Plague Counters")), "{alone:?}");
            assert!(
                send(s.state(), json!({ "instanceId": menace, "plague": { "from": plantation, "tokens": 0 } }))
                    .error
                    .is_some()
            );
            assert!(
                send(s.state(), json!({ "instanceId": menace, "plague": { "from": plantation, "tokens": 3 } }))
                    .error
                    .is_some()
            );
            assert!(
                send(s.state(), json!({ "instanceId": mr_token, "plague": { "from": plantation, "tokens": 2 } }))
                    .error
                    .is_some()
            );
        }

        /// the rest of the price must be in mana: with none, only a full token payment is offered
        #[test]
        fn the_rest_of_the_price_must_be_in_mana_with_none_only_a_full_token_payment_is_offered() {
            let s = standing(2, &[MENACE, MR_TOKEN], Some(0));

            assert!(token_options(s.state(), s.card(MENACE)).is_empty());
            assert_eq!(token_options(s.state(), s.card(MR_TOKEN)), vec![Some(1)]);
        }

        /// with no tokens left it offers nothing
        #[test]
        fn with_no_tokens_left_it_offers_nothing() {
            let s = standing(1, &[MR_TOKEN, VANILLA], None);
            let after = play_from_graveyard(s.state(), s.card(MR_TOKEN), s.card(PLANTATION), 1, None);

            assert_eq!(tokens_on(&after.state, s.card(PLANTATION)), 0);
            assert!(graveyard_plays(&after.state, s.card(VANILLA)).is_empty());
            let zero = standing(0, &[VANILLA], None);
            assert!(graveyard_plays(zero.state(), zero.card(VANILLA)).is_empty());
        }

        /// tokens others place add to it: a Plague Crawler's placement makes 3 to pay with
        #[test]
        fn tokens_others_place_add_to_it_a_plague_crawlers_placement_makes_3_to_pay_with() {
            let mut s = scenario(json!({
                "p1": { "hand": [CRAWLER, ANCHOR], "backrow": [{ "def": PLANTATION, "counters": { "plague": 2 } }], "graveyard": [MENACE] },
                "p2": { "hand": [ANCHOR] },
            }));
            let plantation = s.card(PLANTATION).clone();

            s.play(CRAWLER, json!({ "targets": [{ "pick": "instance", "instanceId": plantation.id }] }));

            assert_eq!(s.card(&plantation).counters.plague, Some(3));
            assert_eq!(token_options(s.state(), s.card(MENACE)), vec![Some(1), Some(2), Some(3)]);
        }

        /// R78 leaving the field ends it and resets its tokens
        #[test]
        fn r78_leaving_the_field_ends_it_and_resets_its_tokens() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [ANCHOR], "backrow": [{ "def": PLANTATION, "counters": { "plague": 2 } }], "graveyard": [MENACE], "library": [VANILLA] },
                "p2": { "hand": [COLLATERAL, ANCHOR], "library": [VANILLA] },
            }));
            let plantation = s.card(PLANTATION).clone();

            s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": plantation.id }] }));
            s.end_turn();

            s.expect_in_zone(&plantation, "exile");
            assert!(s.card(&plantation).counters.plague.is_none());
            assert!(graveyard_plays(s.state(), s.card(MENACE)).is_empty());
        }

        /// §3 with your unit row full nothing is offered, and the play is refused
        #[test]
        fn s3_with_your_unit_row_full_nothing_is_offered_and_the_play_is_refused() {
            let s = scenario(json!({
                "p1": {
                    "hand": [ANCHOR],
                    "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA],
                    "backrow": [{ "def": PLANTATION, "counters": { "plague": 2 } }],
                    "graveyard": [MR_TOKEN],
                },
                "p2": { "hand": [ANCHOR] },
            }));
            let mr_token = s.card(MR_TOKEN).clone();

            assert!(graveyard_plays(s.state(), &mr_token).is_empty());
            let plantation = s.card(PLANTATION).id.clone();
            assert!(
                send(s.state(), json!({ "instanceId": mr_token.id, "plague": { "from": plantation, "tokens": 1 } }))
                    .error
                    .is_some()
            );
        }

        /// only your own graveyard: the opponent's Units are not offered
        #[test]
        fn only_your_own_graveyard_the_opponents_units_are_not_offered() {
            let s = scenario(json!({
                "p1": { "hand": [ANCHOR], "backrow": [{ "def": PLANTATION, "counters": { "plague": 2 } }] },
                "p2": { "hand": [ANCHOR], "graveyard": [MENACE] },
            }));

            let menace = s.card(MENACE).id.clone();
            assert!(
                !legal_actions(s.state(), P1)
                    .iter()
                    .any(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == menace))
            );
        }

        /// R386 an Upgrade places 3 on itself
        #[test]
        fn r386_an_upgrade_places_3_on_itself() {
            let mut s = scenario(json!({ "p1": { "hand": [PLANTATION, ANCHOR] }, "p2": { "hand": [ANCHOR] } }));
            step_param(s.card_mut(PLANTATION), "tokens", 1);

            s.play(PLANTATION, json!({}));
            let plantation = s.card(PLANTATION).clone();
            s.activate(&plantation, json!({}));

            assert_eq!(s.card(PLANTATION).counters.plague, Some(3));
        }
    }

    /// `describe("radiant")`.
    mod radiant {
        use super::*;

        /// Activate: one placement of 4 on itself, and a 3 Cost Unit may be paid wholly in tokens
        #[test]
        fn activate_one_placement_of_4_on_itself_and_a_3_cost_unit_may_be_paid_wholly_in_tokens() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": PLANTATION, "radiant": true }, ANCHOR], "graveyard": [MENACE], "mana": 2 },
                "p2": { "hand": [ANCHOR] },
            }));

            s.play(PLANTATION, json!({}));
            let activated = s.card(PLANTATION).clone();
            s.activate(&activated, json!({}));

            let plantation = s.card(PLANTATION).clone();
            assert_eq!(plantation.counters.plague, Some(4));
            // 0 mana left: only the full payment in tokens is offered.
            assert_eq!(token_options(s.state(), s.card(MENACE)), vec![Some(3)]);
            let after = play_from_graveyard(s.state(), s.card(MENACE), &plantation, 3, None);
            assert_eq!(tokens_on(&after.state, &plantation), 1);
            assert_eq!(after.state.players.p1.mana.current, 0);
        }

        /// R386 a Degrade places 3
        #[test]
        fn r386_a_degrade_places_3() {
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": PLANTATION, "radiant": true }, ANCHOR] }, "p2": { "hand": [ANCHOR] } }));
            step_param(s.card_mut(PLANTATION), "tokens", -1);

            s.play(PLANTATION, json!({}));
            let plantation = s.card(PLANTATION).clone();
            s.activate(&plantation, json!({}));

            assert_eq!(s.card(PLANTATION).counters.plague, Some(3));
        }
    }
}
