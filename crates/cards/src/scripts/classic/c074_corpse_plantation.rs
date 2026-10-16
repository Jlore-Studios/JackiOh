//! C #74 Corpse Plantation (SPEC §8.6 row 74). (2) Field Spell, Epic.
//!   Base:    "Cry: Place {tokens|Plague Counter|Plague Counters} on this.
//!             Activate: Play a Unit from your graveyard, spending at least 1 Plague Counter as mana."
//!             — 2 tokens; Radiant: 4.
//!   Engine:  Play from graveyard (§6.3) for a Unit, armed by the Activate. Token payment spends at least 1
//!            counter, rest in mana; (0) cost cannot use it; choices as from hand; R65 player discounts apply.
//!
//! Patch v0.3.10 (#831): the Cry places {tokens} (R386; the once-a-turn Activate did before), and the
//! Activate (R384) arms the graveyard permission for one play that turn (R831015): it remembers the
//! turn, the permission holds only while that is this turn, and the play from the graveyard it allowed
//! spends it. That play is a play (Cry fires, counts as played; not R70 cast); spent tokens are removed;
//! other cards' tokens pay; R78 clears them on leave.

use jackioh_engine::effects::{place_plague, remember};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-074";

/// Patch v0.3.10 (#831): the Activate arms one play from the graveyard this turn (R831015), the turn it
/// was armed on remembered here; the play that uses it, or the turn's end, disarms it.
const ARMED: &str = "armed";

/// Whether a remembered turn is this very turn.
fn is_this_turn(state: &GameState, value: Option<&Value>) -> bool {
    value.and_then(Value::as_i64) == Some(i64::from(state.turn))
}

/// Activate, once per turn (R384): arm the graveyard permission below for one play.
fn plant() -> ActivationDecl {
    ActivationDecl {
        id: "plant".to_string(),
        label: "Play a Unit from your graveyard".to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(|ctx| vec![remember(json_as(json!({ "key": ARMED, "value": ctx.state.turn })))]),
    }
}

/// The play from your graveyard that the arming allowed spends it. A field trigger's `when` is not read
/// (only a trap's reveal condition is), so `run` asks it.
fn spent() -> TriggerDef {
    TriggerDef::new("plantation-spent", &[GameEventType::CardPlayed], |ctx, event| {
        let GameEvent::CardPlayed { player, from, .. } = event else {
            return vec![];
        };
        if *player != ctx.controller || *from != Some(PlayedFrom::Graveyard) {
            return vec![];
        }
        if !is_this_turn(ctx.state, recalled(ctx, ARMED).as_ref()) {
            return vec![];
        }
        vec![remember(json_as(json!({ "key": ARMED, "value": 0 })))]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        // Cry: place the declared `tokens` on this (R386); a Field Spell's Cry fires as it is played or
        // cast (R1).
        cry: Some(hook(|ctx| {
            vec![place_plague(json_as(json!({
                "target": { "of": "self" },
                "amount": param(&*ctx, "tokens"),
            })))]
        })),
        activations: vec![plant()],
        // A Unit from your graveyard, paid with Plague Counters (§6.3 Play), while armed.
        graveyard_play: Some(read_hook(|args| {
            if !is_this_turn(args.state, args.self_.memory.get(ARMED)) {
                return vec![];
            }
            vec![GraveyardPlayPermission {
                units: Some(true),
                plague: Some(true),
                ..GraveyardPlayPermission::default()
            }]
        })),
        triggers: vec![spent()],
        ..Script::default()
    };
    CardScripts {
        // The same script: the Radiant face's 4 tokens are its declared `tokens`.
        radiant: base.clone(),
        base,
    }
}

// The harness's `play` takes a card from a hand, so a graveyard play is sent to `reduce` as
// `legalActions` offers it and read back off its result (as C #28 Second Wind's test does).

/// `describe("C #74 Corpse Plantation")`.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const PLANTATION: &str = "classic-074";
    const CRAWLER: &str = "classic-053"; // (1) Unit: Cry: place a Plague Counter on a permanent (a prompt).
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

    use crate::js;

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
        // Compares the values as strings ("1" < "2" < "null").
        options.sort_by_key(|option| option.map_or_else(|| "null".to_string(), |tokens| tokens.to_string()));
        options
    }

    /// A `play` from p1 with the body's fields, nonces numbered by applied actions.
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

    /// Its Activate, arming one play from the graveyard this turn (R831015).
    fn arm(s: &mut Scenario) {
        s.activate(PLANTATION, json!({}));
    }

    /// The Plantation already standing with `tokens` on it and armed, and a graveyard of Units and a Spell.
    fn standing(tokens: i32, graveyard: &[&str], mana: Option<i32>) -> Scenario {
        let mut s = standing_unarmed(tokens, graveyard, mana);
        arm(&mut s);
        s
    }

    /// The same, its Activate not used.
    fn standing_unarmed(tokens: i32, graveyard: &[&str], mana: Option<i32>) -> Scenario {
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
        let scripts = script();
        let grant = scripts.base.graveyard_play.as_ref().expect("a graveyard permission");
        let answer = |s: &Scenario| {
            js(&grant(HookArgs {
                state: s.state(),
                self_: s.card(PLANTATION),
                radiant: false,
            }))
        };
        // R831015: nothing until its Activate arms it, then Units paid with its tokens.
        assert_eq!(answer(&standing_unarmed(0, &GRAVEYARD, None)), json!([]));
        assert_eq!(answer(&standing(0, &GRAVEYARD, None)), json!([{ "units": true, "plague": true }]));
        // The Radiant face is the base face, every hook the same one.
        assert!(Arc::ptr_eq(
            grant,
            scripts.radiant.graveyard_play.as_ref().expect("a graveyard permission")
        ));
        assert!(Arc::ptr_eq(&scripts.base.activations[0].run, &scripts.radiant.activations[0].run));
    }

    /// `describe("base")`.
    mod base {
        use super::*;

        /// Cry: one placement of 2 Plague Counters on itself; the Activate places none, once per turn
        #[test]
        fn its_cry_places_2_plague_counters_on_itself_and_its_activate_is_once_per_turn() {
            let mut s = scenario(json!({ "p1": { "hand": [PLANTATION, ANCHOR] }, "p2": { "hand": [ANCHOR] } }));

            s.play(PLANTATION, json!({}));

            let plantation = s.card(PLANTATION).clone();
            assert_eq!(plantation.counters.plague, Some(2));
            assert_eq!(
                counter_changes(s.events()),
                vec![json!({ "type": "counterChanged", "instanceId": plantation.id, "counter": "plague", "value": 2, "placed": 2 })]
            );
            s.activate(&plantation, json!({}));
            assert_eq!(s.card(PLANTATION).counters.plague, Some(2));
            s.expect_refused(|s| s.activate(&plantation, json!({})));
        }

        /// R831015 unarmed, it offers nothing from the graveyard
        #[test]
        fn r831015_unarmed_it_offers_nothing_from_the_graveyard() {
            let s = standing_unarmed(2, &GRAVEYARD, None);
            assert!(graveyard_plays(s.state(), s.card(MENACE)).is_empty());
            let plantation = s.card(PLANTATION).id.clone();
            let menace = s.card(MENACE).id.clone();
            assert!(
                send(s.state(), json!({ "instanceId": menace, "plague": { "from": plantation, "tokens": 1 } }))
                    .error
                    .is_some()
            );
        }

        /// R831015 one Activate arms one play: after it, nothing more is offered that turn
        #[test]
        fn r831015_one_activate_arms_one_play_from_the_graveyard() {
            let s = standing(3, &[MR_TOKEN, VANILLA], None);
            assert!(!graveyard_plays(s.state(), s.card(VANILLA)).is_empty());

            let after = play_from_graveyard(s.state(), s.card(MR_TOKEN), s.card(PLANTATION), 1, None);

            assert_eq!(tokens_on(&after.state, s.card(PLANTATION)), 2);
            assert!(graveyard_plays(&after.state, s.card(VANILLA)).is_empty());
        }

        /// R831015 the arming ends with the turn, and its next Activate arms it again
        #[test]
        fn r831015_the_arming_ends_with_the_turn() {
            let mut s = standing(2, &[MENACE], None);
            s.end_turn();
            s.end_turn();
            assert!(graveyard_plays(s.state(), s.card(MENACE)).is_empty());
            arm(&mut s);
            assert!(!graveyard_plays(s.state(), s.card(MENACE)).is_empty());
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
        fn r81_its_choices_are_as_from_hand_a_plague_crawler_from_the_graveyard_asks_for_its_permanent_and_places_on_it() {
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "backrow": [{ "def": PLANTATION, "counters": { "plague": 1 } }], "graveyard": [CRAWLER] },
                "p2": { "hand": [ANCHOR], "field": [VANILLA] },
            }));
            arm(&mut s);
            let vanilla = s.card(VANILLA).clone();

            let after = play_from_graveyard(s.state(), s.card(CRAWLER), s.card(PLANTATION), 1, None);

            // Its Cry asks for its permanent as it resolves (R831016), as from hand.
            *s.state_mut() = after.state;
            s.answer(json!(vanilla.id));
            assert_eq!(tokens_on(s.state(), &vanilla), 1);
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
            let mut zero = standing_unarmed(0, &[VANILLA], None);
            arm(&mut zero);
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
            arm(&mut s);

            s.play(CRAWLER, json!({}));
            s.answer(json!(plantation.id));

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
            let mut s = scenario(json!({
                "p1": {
                    "hand": [ANCHOR],
                    "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA],
                    "backrow": [{ "def": PLANTATION, "counters": { "plague": 2 } }],
                    "graveyard": [MR_TOKEN],
                },
                "p2": { "hand": [ANCHOR] },
            }));
            arm(&mut s);
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
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "backrow": [{ "def": PLANTATION, "counters": { "plague": 2 } }] },
                "p2": { "hand": [ANCHOR], "graveyard": [MENACE] },
            }));
            arm(&mut s);

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

            assert_eq!(s.card(PLANTATION).counters.plague, Some(3));
        }
    }

    /// `describe("radiant")`.
    mod radiant {
        use super::*;

        /// Cry: one placement of 4 on itself, and once armed a 3 Cost Unit may be paid wholly in tokens
        #[test]
        fn its_cry_places_4_on_itself_and_a_3_cost_unit_may_be_paid_wholly_in_tokens() {
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

            assert_eq!(s.card(PLANTATION).counters.plague, Some(3));
        }
    }
}
