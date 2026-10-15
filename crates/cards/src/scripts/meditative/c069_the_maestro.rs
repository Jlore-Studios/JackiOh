//! M #69 The Maestro (SPEC §8.8 row 69): (4) Unit, Human, Plague, Rare, 4/4 → 8/8.
//!
//! Base:    "Whenever you spend mana, place 1 Plague Counter on this.
//!          Activate: Spend ({price}) Plague Counters: Exile {exiles|random enemy card|random enemy
//!          cards} from each of your opponent's field, hand, deck and graveyard."
//! Radiant: the same text, two exiles per zone.
//! Engine: the `manaSpent` event (MD-D26) — a field trigger answers the controller's own spending
//! with one placement per mana (R212 already keeps the Maestro from answering its own play, and the
//! opponent's spending is never its controller's); the Activate refuses below {price} Plague
//! Counters and exiles {exiles} per zone (MD-D27), two per zone on the Radiant face.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-069";

/// "Whenever you spend mana, place 1 Plague Counter on this": one placement per mana the
/// controller spent — never the opponent's, never the Maestro's own play (R212: it was not on the
/// field yet).
fn mana_trigger() -> TriggerDef {
    TriggerDef::new("maestro-mana-spent", &[GameEventType::ManaSpent], |ctx, event| {
        let GameEvent::ManaSpent { player, amount, .. } = event else {
            return vec![];
        };
        if *player != ctx.controller {
            return vec![];
        }
        (0..*amount)
            .map(|_| place_plague(json_as(json!({ "amount": 1 }))))
            .collect()
    })
}

/// "Activate: Spend ({price}) Plague Counters: Exile {exiles} from each of the opponent's field,
/// hand, deck, and graveyard zones."
fn exile_ability(price: i32, exiles: i32) -> ActivationDecl {
    ActivationDecl {
        id: "maestro-exile".to_string(),
        label: format!("Spend ({price}) Plague Counters: exile {exiles} from each enemy zone"),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: Some(condition_hook(move |c| {
            plague_on(c.self_) >= param_value(c.state, Some(c.self_), "price", ParamValueOptions::default())
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

fn maestro(radiant: bool) -> Script {
    Script {
        triggers: vec![mana_trigger()],
        activations: vec![exile_ability(4, if radiant { 2 } else { 1 })],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: maestro(false),
        radiant: maestro(true),
    }
}

// M #69 The Maestro — SPEC §8.8 row 69, BUILD M10 row M 69: "Counters per mana (MD-D26: its own
// play and the opponent's spending not counted); refused below 4; one exile per zone, empty zones
// skipped (MD-D27); two per zone on the Radiant face".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const MAESTRO: &str = "meditative-069";
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

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
            // Two (1)s spend 2 mana: 2 counters.
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
            // The opponent's spending is never its controller's.
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

        /// Money Machine (Meditative #54): "Activate: Spend (2). Tribute this."
        const MONEY_MACHINE: &str = "meditative-054";
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
                    "backrow": [MONEY_MACHINE],
                    "hand": [FILLER, FILLER],
                    "library": filler(4),
                },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            let maestro = s.unit("p1", 1).expect("the Maestro");
            s.activate(MONEY_MACHINE, json!({ "ability": "tribute" }));
            // Money Machine's price is (2): two counters.
            assert_eq!(plague(&s, &maestro.id), 2);
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
            let events = serde_json::to_value(s.view(PlayerId::P1).events).expect("events");
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
            // The price set to 3 and exiles to 2 per zone: the Activate reads both off the card.
            set_param(s.card_mut(&maestro.id), "price", 3);
            set_param(s.card_mut(&maestro.id), "exiles", 2);
            activate_exile(&mut s);
            assert_eq!(plague(&s, &maestro.id), 0);
            assert_eq!(exiled_of(&s).len(), 8);
        }

        #[test]
        fn the_radiant_face_is_8_8() {
            let s = full_enemy("maestro-stats", true, 0);
            let maestro = s.unit("p1", 1).expect("the Maestro");
            let stats = s.stats(maestro.id.as_str());
            assert_eq!((stats.attack, stats.health), (8, 8));
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
}
