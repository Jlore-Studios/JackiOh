//! M #69 The Maestro (SPEC §8.8 row 69): (4) Unit, Human, Plague, Rare, 4/4 → 8/8.
//!
//! Base:    "When you spend mana, place 1 Plague Counter on this.
//!          Activate: Spend ({price}) Plague Counters: Exile {exiles|random card|random cards} each from
//!          their field, hand, deck and graveyard."
//! Radiant: the same text, two exiles per zone.
//! Engine: the `manaSpent` event (MD-D26) feeds a field trigger; the Activate refuses below {price}
//! Plague Counters and exiles {exiles} per zone (MD-D27).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-069";

/// "Whenever you spend mana, place 1 Plague Counter on this": one placement per mana the
/// controller spent — never the opponent's, never the Maestro's own play (R212: it was not on the
/// field yet).
/// "place 1 Plague Counter": what it printed before it declared `counters` (#2657).
const PRINTED_COUNTERS: i32 = 1;

fn mana_trigger() -> TriggerDef {
    TriggerDef::new("maestro-mana-spent", &[GameEventType::ManaSpent], |ctx, event| {
        let GameEvent::ManaSpent { player, amount, .. } = event else {
            return vec![];
        };
        if *player != ctx.controller {
            return vec![];
        }
        // R2657006: the declared `counters`, 1 as printed before #2657 (`param_or`).
        let counters = param_or(&*ctx, "counters", PRINTED_COUNTERS);
        (0..*amount)
            .map(|_| place_plague(json_as(json!({ "amount": counters }))))
            .collect()
    })
}

/// Its label is a template: the client fills `{price}` and `{exiles}` from the card's
/// current numbers (R386), so a tuned number shows as the one the ability uses.
fn exile_ability() -> ActivationDecl {
    ActivationDecl {
        id: "maestro-exile".to_string(),
        label: "Spend ({price}) Plague Counters: exile {exiles} from each enemy zone".to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: Some(condition_hook(move |c| {
            // R1850000, R1884005: `param` reads a copier's copied text, where the Activate came from.
            plague_on(c.self_) >= param(&c, "price")
        })),
        has: None,
        run: hook(move |ctx| {
            let price = param(&*ctx, "price");
            let exiles = param(&*ctx, "exiles");
            let mut effects = vec![consume_plague(json_as(json!({ "amount": price })))];
            for from in ["field", "hand", "library", "graveyard"] {
                effects.push(exile_random_from(json_as(json!({
                    "player": "enemy",
                    "from": from,
                    "count": exiles,
                }))));
            }
            effects
        }),
    }
}

fn maestro() -> Script {
    Script {
        triggers: vec![mana_trigger()],
        activations: vec![exile_ability()],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: maestro(),
        radiant: maestro(),
    }
}

// M #69 The Maestro — SPEC §8.8 row 69, BUILD M10 row M 69.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::filler;
    use crate::js;
    use jackioh_engine::testkit::*;

    const MAESTRO: &str = "meditative-069";
    const FILLER: &str = "core-005";

    /// p1 holds the Maestro (base unless `radiant_face`); both sides keep cards in hand so no turn
    /// auto-ends.
    fn casting(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": MAESTRO, "radiant": radiant_face }, FILLER, FILLER, FILLER, FILLER],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// Play the Maestro into lane 1 and place `counters` Plague Counters on it.
    fn staged(seed: &str, radiant_face: bool, counters: i32) -> Scenario {
        let mut s = casting(seed, radiant_face);
        s.play(MAESTRO, json!({ "zone": 1 }));
        let maestro = s.unit("p1", 1).expect("the Maestro");
        for _ in 0..counters {
            s.card_mut(&maestro.id).counters.plague =
                Some(s.card(&maestro.id).counters.plague.unwrap_or(0) + 1);
        }
        s
    }

    fn plague(s: &Scenario, id: &str) -> i32 {
        s.card(id).counters.plague.unwrap_or(0).max(0)
    }

    mod m69_the_maestro {
        use super::*;

        #[test]
        fn places_a_counter_per_mana_spent() {
            let mut s = staged("maestro-counters", false, 0);
            let maestro = s.unit("p1", 1).expect("the Maestro");
            assert_eq!(plague(&s, &maestro.id), 0);
            s.state_mut().players.p1.mana.current = 2;
            s.play(FILLER, json!({}));
            s.play(FILLER, json!({}));
            let maestro = s.unit("p1", 1).expect("the Maestro");
            assert_eq!(plague(&s, &maestro.id), 2);
        }

        #[test]
        fn its_own_play_and_the_opponent_spending_count_nothing() {
            let mut s = casting("maestro-own", false);
            // Its own play cost 4: unheard while resolving, so no counters (R212).
            s.play(MAESTRO, json!({ "zone": 1 }));
            let maestro = s.unit("p1", 1).expect("the Maestro");
            assert_eq!(plague(&s, &maestro.id), 0);
            s.end_turn();
            s.state_mut().players.p2.mana.current = 2;
            s.play(FILLER, json!({}));
            let maestro = s.unit("p1", 1).expect("the Maestro");
            assert_eq!(plague(&s, &maestro.id), 0);
        }

        #[test]
        fn refused_below_4_counters() {
            let mut s = staged("maestro-refused", false, 3);
            s.expect_refused(|s| {
                let maestro = s.unit("p1", 1).expect("the Maestro");
                s.activate(&maestro, json!({ "ability": "maestro-exile" }))
            });
        }

        #[test]
        fn r1110_one_exile_per_zone_empty_skipped() {
            let mut s = staged("maestro-exile", false, 4);
            let maestro = s.unit("p1", 1).expect("the Maestro");
            s.activate(&maestro, json!({ "ability": "maestro-exile" }));
            assert_eq!(plague(&s, &maestro.id), 0);
            // One card from each enemy zone that holds one; the empty field and graveyard
            // are skipped (p2 holds 1 in hand, 4 in the library).
            assert!(s.hand("p2").is_empty());
            assert_eq!(s.pile("p2", "library").len(), 3);
            let exiled = s
                .last_events()
                .iter()
                .filter(|event| event.event_type() == GameEventType::Exiled)
                .count();
            assert_eq!(exiled, 2);
        }

        /// #98 Heroic Power, whose Life Tap ("draw") is an Activate with a mana price of (1) (R752).
        const HEROIC_POWER: &str = "core-098";
        const VANILLA: &str = "core-008";

        fn exiled_of(s: &Scenario) -> Vec<GameEvent> {
            s.last_events()
                .iter()
                .filter(|event| event.event_type() == GameEventType::Exiled)
                .cloned()
                .collect()
        }

        /// The Maestro on p1's field with `counters` Plague Counters, and every enemy zone full.
        fn full_enemy(seed: &str, radiant_face: bool, counters: i32) -> Scenario {
            crate::register_all();
            scenario(json!({
                "seed": seed,
                "p1": {
                    "field": [{ "def": MAESTRO, "radiant": radiant_face, "counters": { "plague": counters } }],
                    "hand": [FILLER, FILLER],
                    "library": filler(4),
                },
                "p2": {
                    "hand": [FILLER, FILLER, FILLER],
                    "field": [VANILLA, VANILLA, VANILLA],
                    "graveyard": [FILLER, FILLER, FILLER],
                    "library": filler(4),
                },
            }))
        }

        fn activate_exile(s: &mut Scenario) {
            let maestro = s.unit("p1", 1).expect("the Maestro");
            s.activate(&maestro, json!({ "ability": "maestro-exile" }));
        }

        #[test]
        fn an_activations_mana_places_counters_too() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "maestro-activation",
                "p1": {
                    "mana": 10,
                    "field": [MAESTRO],
                    "backrow": [HEROIC_POWER],
                    "hand": [FILLER, FILLER],
                    "library": filler(4),
                },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            let maestro = s.unit("p1", 1).expect("the Maestro");
            let power = s.card(HEROIC_POWER).id.clone();
            find_instance_mut(s.state_mut(), &power)
                .expect("the Heroic Power")
                .memory
                .insert(jackioh_engine::subsystems::POWER_KEY.to_string(), json!("draw"));
            s.activate(HEROIC_POWER, json!({}));
            assert_eq!(plague(&s, &maestro.id), 1);
        }

        #[test]
        fn r1110_one_random_card_from_each_of_the_four_enemy_zones() {
            let mut s = full_enemy("maestro-four", false, 4);
            activate_exile(&mut s);
            let maestro = s.unit("p1", 1).expect("the Maestro");
            assert_eq!(plague(&s, &maestro.id), 0, "the counters are removed");
            let units = (1..=UNIT_ZONES)
                .filter(|lane| s.unit("p2", *lane).is_some())
                .count();
            assert_eq!(units, 2);
            assert_eq!(s.hand("p2").len(), 2);
            assert_eq!(s.pile("p2", "library").len(), 3);
            assert_eq!(s.pile("p2", "graveyard").len(), 2);
            assert_eq!(s.pile("p2", "exile").len(), 4);
            assert_eq!(exiled_of(&s).len(), 4);
        }

        #[test]
        fn the_exiles_are_public() {
            let mut s = full_enemy("maestro-public", false, 4);
            activate_exile(&mut s);
            // p1 reads each exiled card, the hand and deck ones included.
            let events = js(&s.view(PlayerId::P1).events);
            let exiled: Vec<&Value> = events
                .as_array()
                .into_iter()
                .flatten()
                .filter(|event| event["type"] == json!("exiled"))
                .collect();
            assert_eq!(exiled.len(), 4);
            for event in exiled {
                assert_ne!(event["instanceId"], json!("hidden"), "{event}");
                assert!(event["defId"].is_string(), "{event}");
            }
        }

        #[test]
        fn below_4_counters_legal_actions_does_not_list_it() {
            let s = full_enemy("maestro-listed", false, 3);
            let maestro = s.unit("p1", 1).expect("the Maestro");
            let listed = |s: &Scenario| {
                legal_actions(s.state(), PlayerId::P1).iter().any(|body| {
                    matches!(body, ActionBody::Activate { instance_id, .. } if *instance_id == maestro.id)
                })
            };
            assert!(!listed(&s));
            let s = full_enemy("maestro-listed-4", false, 4);
            let maestro = s.unit("p1", 1).expect("the Maestro");
            assert!(legal_actions(s.state(), PlayerId::P1).iter().any(|body| {
                matches!(body, ActionBody::Activate { instance_id, .. } if *instance_id == maestro.id)
            }));
        }

        #[test]
        fn once_a_turn() {
            let mut s = full_enemy("maestro-once", false, 8);
            activate_exile(&mut s);
            let maestro = s.unit("p1", 1).expect("the Maestro");
            assert_eq!(plague(&s, &maestro.id), 4);
            s.expect_refused(|s| s.activate(&maestro, json!({ "ability": "maestro-exile" })));
        }

        #[test]
        fn r386_price_and_exiles_read_through_param() {
            let mut s = full_enemy("maestro-param", false, 3);
            let maestro = s.unit("p1", 1).expect("the Maestro");
            set_param(s.card_mut(&maestro.id), "price", 3);
            set_param(s.card_mut(&maestro.id), "exiles", 2);
            activate_exile(&mut s);
            assert_eq!(plague(&s, &maestro.id), 0);
            assert_eq!(exiled_of(&s).len(), 8);
        }

        /// R386: the Activate's words are the template the view's `params` fill, so a button shows the
        /// price and exile count the ability will use, tuned or printed, on either face.
        #[test]
        fn r386_the_activate_label_is_its_template_once_its_numbers_are_tuned() {
            for radiant_face in [false, true] {
                let mut s = full_enemy("maestro-label", radiant_face, 4);
                let maestro = s.unit("p1", 1).expect("the Maestro");
                // One Upgrade each: the exiles one more, the price one less.
                let exiles_now = crate::upgrade_number(&mut s, &maestro.id, "exiles");
                let price_now = crate::upgrade_number(&mut s, &maestro.id, "price");
                assert_eq!(
                    (price_now, exiles_now),
                    (3, if radiant_face { 3 } else { 2 }),
                    "radiant {radiant_face}"
                );
                let shown = s.view(PlayerId::P1).you.units[0]
                    .clone()
                    .expect("the Maestro's view");
                let params = shown.params.expect("its declared numbers");
                assert_eq!(params.get("price"), Some(&price_now), "radiant {radiant_face}");
                assert_eq!(params.get("exiles"), Some(&exiles_now), "radiant {radiant_face}");
                let labels: Vec<String> = shown
                    .activations
                    .unwrap_or_default()
                    .into_iter()
                    .map(|activation| activation.label)
                    .collect();
                assert_eq!(
                    labels,
                    ["Spend ({price}) Plague Counters: exile {exiles} from each enemy zone"],
                    "radiant {radiant_face}"
                );
            }
        }

        #[test]
        fn two_per_zone_on_the_radiant_face_from_each_of_four_zones() {
            let mut s = full_enemy("maestro-radiant-four", true, 4);
            activate_exile(&mut s);
            assert_eq!(exiled_of(&s).len(), 8);
            assert_eq!(s.hand("p2").len(), 1);
            assert_eq!(s.pile("p2", "graveyard").len(), 1);
        }

        #[test]
        fn two_per_zone_on_the_radiant_face() {
            let mut s = staged("maestro-radiant", true, 4);
            let maestro = s.unit("p1", 1).expect("the Maestro");
            s.activate(&maestro, json!({ "ability": "maestro-exile" }));
            assert_eq!(plague(&s, &maestro.id), 0);
            // Two from the library, one from the hand (all it holds).
            assert!(s.hand("p2").is_empty());
            assert_eq!(s.pile("p2", "library").len(), 2);
            let exiled = s
                .last_events()
                .iter()
                .filter(|event| event.event_type() == GameEventType::Exiled)
                .count();
            assert_eq!(exiled, 3);
        }
    }
    /// R1850000, R1884005: a copier running the Maestro's text (S #20 Tnack Trav copying a Legendary
    /// fusion of the Maestro and Core #92 Felinor Fiender) reads the Activate's price off the copied
    /// text, as `param` does, and its legality check does not panic on the copier's own card.
    #[test]
    fn r1884005_a_copier_of_the_maestro_s_text_reads_its_price_off_the_copy() {
        const TNACK_TRAV: &str = "stinkyscape-020";
        const FIENDER: &str = "core-092"; // Legendary, so the fusion is.
        crate::register_all();
        let mut s = scenario(json!({
            "seed": "maestro-copied",
            "p1": { "hand": [TNACK_TRAV, FILLER], "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        let mut events: Vec<GameEvent> = vec![];
        let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
        let fused = {
            let state = s.state_mut();
            let ingredients: Vec<CardInstance> = [MAESTRO, FIENDER]
                .iter()
                .map(|def_id| new_instance(&mut *state, def_id, PlayerId::P1, Zone::Gone { player: PlayerId::P1 }))
                .collect();
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            subsystems::fuse(
                &mut sink,
                subsystems::FuseArgs {
                    ingredients,
                    to_hand: Some(PlayerId::P1),
                    ..subsystems::FuseArgs::default()
                },
            )
        }
        .expect("the fused card");
        s.state_mut().rng_cursor = rng.cursor();
        s.play(&fused.id, json!({ "zone": 1 }));
        let fusion = s.unit_at(PlayerId::P1, 1);
        s.play(TNACK_TRAV, json!({ "zone": 2, "targets": [{ "pick": "instance", "instanceId": fusion.id }] }));
        let copier = s.unit_at(PlayerId::P1, 2);
        assert!(subsystems::unit_text::unit_text_of(&copier).is_some(), "Tnack Trav copied the fusion");
        // The legality check reads the copied Activate's price: no panic, and no use without counters.
        let actions = legal_actions(s.state(), PlayerId::P1);
        assert!(!actions.iter().any(|action| matches!(
            action,
            ActionBody::Activate { instance_id, .. } if *instance_id == copier.id
        )));
    }
}
