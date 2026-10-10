//! C #53 Plague Crawler (SPEC §8.6 row 53). (1) Unit, Common, 2/2 → 4/4.
//!   Base:    "Cry: Place {tokens|Plague Counter|Plague Counters} on another permanent.
//!             Whenever Plague Counters are placed on this, draw {draw}." — 1 token, draw 1
//!   Radiant: the same text — 2 tokens, draw 2
//!
//! The Cry targets another permanent on either side (R81, §6.3; face-down offered by id alone, R177;
//! fizzles if none). "Whenever Plague Counters are placed on this" triggers once per placement naming
//! it, whoever placed them (R99; removal draws nothing).
//! Tuned numbers `tokens` and `draw` (R386) read through `param` on the running face.

use jackioh_engine::effects::{draw, place_plague};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-053";

/// "another permanent": the top of any unit pile or any backrow card, either side, never this one.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(
        1,
        1,
        json!({ "side": "any", "of": ["unit", "backrow"], "excludeSelf": true }),
    )]
}

/// A placement of Plague Counters on this card: `counterChanged` for `plague` with `placed`, naming it.
fn placed_on_this(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(me) = &ctx.self_ else {
        return false;
    };
    matches!(
        event,
        GameEvent::CounterChanged { counter: CounterKind::Plague, instance_id, placed, .. }
            if *instance_id == me.id && placed.unwrap_or(0) > 0
    )
}

fn draw_on_placement() -> TriggerDef {
    TriggerDef::new("plague-crawler-placed", &[GameEventType::CounterChanged], |ctx, event| {
        if !placed_on_this(ctx, event) {
            return vec![];
        }
        let count = param(&*ctx, "draw");
        vec![draw(json_as(json!({ "count": count })))]
    })
    .with_when(placed_on_this)
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            let amount = param(&*ctx, "tokens");
            vec![place_plague(json_as(json!({ "target": { "of": "chosen" }, "amount": amount })))]
        })),
        triggers: vec![draw_on_placement()],
        ..Script::default()
    };

    // The same script: the Radiant face's 2 tokens and draw 2 are its declared numbers, which `param`
    // reads off the running face.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// C #53 — SPEC §8.6 row 53, BUILD M9 Classic row C 53: Cry places plague tokens on another permanent
// (face-down carries only id, R177); draws on placement on it; tuned numbers read via `param()` (R386).
// Fusing Crawler onto C #27 Pestilent Slime (R77) tests multiplier and trigger sitting on one card.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CRAWLER: &str = "classic-053";
    const SLIME: &str = "classic-027"; // (0) Unit 1/1: Plague Counters placed on this are doubled.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt; Radiant adds Immutable.
    const MANA_WELL: &str = "core-006"; // (3) Field Spell.
    const UNLICENSED: &str = "core-085"; // (2) Trap, answers only a permanent of a type its controller controls.
    const FIENDER: &str = "core-092"; // (2) Unit 5/7 Stack.
    const FLOOD: &str = "core-017"; // (4) Spell: Bounce all Units.
    const FILLER: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const ANCHOR: &str = "core-010"; // (0) Spell, a card to keep a hand from auto-ending the turn (§2.5).
    const X: &str = "core-020"; // library filler.

    use crate::js;

    fn at(card: &CardInstance) -> Value {
        json!([{ "pick": "instance", "instanceId": card.id }])
    }

    fn lib(n: usize) -> Vec<&'static str> {
        (0..n).map(|_| X).collect()
    }

    /// Every placement so far: one entry per `counterChanged` that carries `placed`.
    fn placements(s: &Scenario) -> Value {
        let entries: Vec<Value> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CounterChanged { instance_id, counter: CounterKind::Plague, value, placed: Some(placed) } => {
                    Some(json!({ "id": instance_id, "value": value, "placed": placed }))
                }
                _ => None,
            })
            .collect();
        json!(entries)
    }

    fn draws_by(events: &[GameEvent], player: PlayerId) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Drawn { player: who, .. } if *who == player))
            .count()
    }

    fn plays_of(s: &Scenario, card: &CardInstance, player: PlayerId) -> Vec<ActionBody> {
        legal_actions(s.state(), player)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == card.id))
            .collect()
    }

    /// The cards `legal_actions` offers as the play's target, each once (a play is listed once per zone).
    fn offered_targets(s: &Scenario, card: &CardInstance, player: PlayerId) -> Vec<String> {
        let mut ids: IndexSet<String> = IndexSet::new();
        for play in plays_of(s, card, player) {
            let ActionBody::Play { targets, .. } = play else {
                continue;
            };
            for target in targets.unwrap_or_default() {
                if let Selection::Instance { instance_id } = target {
                    ids.insert(instance_id);
                }
            }
        }
        ids.into_iter().collect()
    }

    fn crawlers(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.hand(player).into_iter().filter(|card| card.def_id == CRAWLER).collect()
    }

    fn two_crawlers(s: &Scenario) -> (CardInstance, CardInstance) {
        let mut both = crawlers(s, P1).into_iter();
        match (both.next(), both.next()) {
            (Some(first), Some(second)) => (first, second),
            _ => panic!("two Crawlers in hand"),
        }
    }

    mod c53_plague_crawler {
        use super::*;

        #[test]
        fn declares_one_target_another_permanent_on_either_side_its_two_numbers_and_one_script_on_both_faces() {
            crate::register_all();
            let def = js(&registered_catalog()[ID]);
            assert_eq!(def["id"], CRAWLER);
            let scripts = script();
            assert_eq!(
                js(&scripts.base.targets),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "backrow"], "excludeSelf": true } }]),
            );
            assert_eq!(
                def["params"],
                json!([
                    { "key": "tokens", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 },
                    { "key": "draw", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 },
                ]),
            );
            // The Radiant face is the same script.
            assert_eq!(js(&scripts.radiant.targets), js(&scripts.base.targets));
            assert_eq!(scripts.radiant.cry.is_some(), scripts.base.cry.is_some());
            assert_eq!(
                scripts.radiant.triggers.iter().map(|trigger| trigger.id.clone()).collect::<Vec<_>>(),
                scripts.base.triggers.iter().map(|trigger| trigger.id.clone()).collect::<Vec<_>>(),
            );
        }

        mod base {
            use super::*;

            #[test]
            fn is_a_2_2_and_its_cry_places_1_plague_counter_on_an_enemy_unit_you_choose_as_one_placement() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [CRAWLER, ANCHOR] }, "p2": { "hand": [ANCHOR], "field": [VANILLA] } }));
                let vanilla = s.card(VANILLA).clone();

                s.play(CRAWLER, json!({ "targets": at(&vanilla) }));

                s.expect_stats(CRAWLER, json!({ "attack": 2, "health": 2 }));
                assert_eq!(s.card(&vanilla).counters.plague, Some(1));
                assert_eq!(placements(&s), json!([{ "id": vanilla.id, "value": 1, "placed": 1 }]));
            }

            #[test]
            fn r81_either_side_your_own_unit_or_a_field_spell_in_your_backrow_takes_the_token() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [CRAWLER, CRAWLER, ANCHOR], "field": [VANILLA], "backrow": [MANA_WELL], "library": lib(2) },
                    "p2": { "hand": [ANCHOR] },
                }));
                let (first, second) = two_crawlers(&s);

                let vanilla = s.card(VANILLA).clone();
                s.play(&first, json!({ "targets": at(&vanilla) }));
                let mana_well = s.card(MANA_WELL).clone();
                s.play(&second, json!({ "targets": at(&mana_well) }));

                assert_eq!(s.card(VANILLA).counters.plague, Some(1));
                assert_eq!(s.card(MANA_WELL).counters.plague, Some(1));
            }

            #[test]
            fn an_immutable_permanent_takes_tokens_too_a_token_is_a_counter_not_a_change_of_text() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [CRAWLER, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [{ "def": MENACE, "radiant": true }] },
                }));

                let menace = s.card(MENACE).clone();
                s.play(CRAWLER, json!({ "targets": at(&menace) }));

                let kinds: Vec<Value> = js(&s.stats(MENACE).keywords)
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .map(|keyword| keyword["kind"].clone())
                    .collect();
                assert!(kinds.contains(&json!("Immutable")));
                assert_eq!(s.card(MENACE).counters.plague, Some(1));
            }

            #[test]
            fn another_it_never_offers_itself_and_legal_actions_offers_every_other_permanent() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [CRAWLER, ANCHOR], "field": [VANILLA] },
                    "p2": { "hand": [ANCHOR], "field": [MENACE], "backrow": [MANA_WELL] },
                }));
                let crawler = s.card(CRAWLER).clone();

                let offered = offered_targets(&s, &crawler, P1);

                assert!(!offered.contains(&crawler.id));
                let want: IndexSet<String> =
                    [VANILLA, MENACE, MANA_WELL].iter().map(|def| s.card(*def).id.clone()).collect();
                assert_eq!(offered.into_iter().collect::<IndexSet<String>>(), want);
                let own = at(&crawler);
                s.expect_refused(move |s| s.play(&crawler, json!({ "targets": own })));
            }

            #[test]
            fn s3_2_r13_a_card_dormant_under_a_stack_pile_is_not_offered_the_top_of_the_pile_is() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [CRAWLER, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, { "def": FIENDER, "stack": true }] },
                }));
                let dormant = s.card(VANILLA).clone();
                let top = s.card(FIENDER).clone();

                let crawler = s.card(CRAWLER).clone();
                assert_eq!(offered_targets(&s, &crawler, P1), vec![top.id.clone()]);
                let dormant_target = at(&dormant);
                s.expect_refused(move |s| s.play(CRAWLER, json!({ "targets": dormant_target })));

                s.play(CRAWLER, json!({ "targets": at(&top) }));
                assert_eq!(s.card(&top).counters.plague, Some(1));
                assert!(s.card(&dormant).counters.plague.is_none());
            }

            #[test]
            fn with_no_other_permanent_on_the_field_it_enters_anyway_and_places_nothing() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [CRAWLER, ANCHOR] }, "p2": { "hand": [ANCHOR] } }));
                let crawler = s.card(CRAWLER).clone();
                assert!(!plays_of(&s, &crawler, P1).is_empty());
                assert!(offered_targets(&s, &crawler, P1).is_empty());

                s.play(&crawler, json!({}));

                s.expect_in_zone(&crawler, "field");
                assert_eq!(placements(&s), json!([]));
                assert_eq!(draws_by(s.events(), P1), 0);
            }

            #[test]
            fn r177_a_face_down_enemy_trap_is_offered_by_its_id_alone_and_the_placement_on_it_never_names_it_to_you() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [CRAWLER, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "backrow": [{ "def": UNLICENSED, "faceUp": false }] },
                }));
                let trap = s.card(UNLICENSED).clone();

                let crawler = s.card(CRAWLER).clone();
                assert_eq!(offered_targets(&s, &crawler, P1), vec![trap.id.clone()]);
                assert!(!js(&s.view(P1)).to_string().contains(UNLICENSED));

                s.play(CRAWLER, json!({ "targets": at(&trap) }));

                assert_eq!(s.card(&trap).counters.plague, Some(1));
                assert_ne!(s.card(&trap).face_up, Some(true));
                assert!(!js(&s.view(P1)).to_string().contains(UNLICENSED));
                // Its controller still reads it, tokens and all.
                assert!(js(&s.view(P2))["you"]["backrow"].to_string().contains(UNLICENSED));
            }

            #[test]
            fn whenever_plague_counters_are_placed_on_it_it_draws_1_a_second_crawlers_cry_on_it() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [CRAWLER, CRAWLER, ANCHOR], "library": lib(3) }, "p2": { "hand": [ANCHOR] } }));
                let (first, second) = two_crawlers(&s);
                s.play(&first, json!({}));
                assert_eq!(draws_by(s.events(), P1), 0);

                s.play(&second, json!({ "targets": at(&first) }));

                assert_eq!(s.card(&first).counters.plague, Some(1));
                assert_eq!(draws_by(s.last_events(), P1), 1);
            }

            #[test]
            fn by_either_player_the_opponents_crawler_placing_on_yours_draws_you_1_and_them_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [ANCHOR], "field": [CRAWLER], "library": lib(3) },
                    "p2": { "hand": [CRAWLER, ANCHOR], "library": lib(3) },
                }));
                let Some(mine) = s.unit(P1, 1) else {
                    panic!("p1's Crawler is on the field");
                };

                let theirs = crawlers(&s, P2)[0].clone();
                s.play(&theirs, json!({ "targets": at(&mine) }));

                assert_eq!(draws_by(s.last_events(), P1), 1);
                assert_eq!(draws_by(s.last_events(), P2), 0);
            }

            #[test]
            fn once_per_placement_however_many_tokens_a_radiant_crawlers_placement_of_2_on_it_draws_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": CRAWLER, "radiant": true }, ANCHOR], "field": [CRAWLER], "library": lib(3) },
                    "p2": { "hand": [ANCHOR] },
                }));
                let on_field = s.unit(P1, 1).expect("a Crawler on the field");
                let radiant_one = s.hand(P1).into_iter().find(|card| card.def_id == CRAWLER).expect("a Crawler in hand");

                s.play(&radiant_one, json!({ "targets": at(&on_field) }));

                assert_eq!(s.card(&on_field).counters.plague, Some(2));
                assert_eq!(placements(&s), json!([{ "id": on_field.id, "value": 2, "placed": 2 }]));
                assert_eq!(draws_by(s.last_events(), P1), 1);
            }

            #[test]
            fn c_27_a_crawler_fused_onto_a_pestilent_slime_takes_2_from_a_placement_of_1_and_still_draws_once() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [CRAWLER, ANCHOR], "field": [SLIME], "library": lib(3) },
                    "p2": { "hand": [CRAWLER, ANCHOR] },
                }));
                let slime = s.card(SLIME).clone();
                let ingredient = s.hand(P1).into_iter().find(|card| card.def_id == CRAWLER).expect("a Crawler in hand");
                let mut rng = Rng::new(&s.state().seed.clone(), s.state().rng_cursor);
                let mut events: Vec<GameEvent> = Vec::new();
                {
                    let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                    subsystems::fuse::fuse(&mut sink, json_as(json!({ "ingredients": [ingredient], "target": slime })));
                }
                s.end_turn();

                let theirs = crawlers(&s, P2)[0].clone();
                s.play(&theirs, json!({ "targets": at(&slime) }));

                assert_eq!(s.card(&slime).counters.plague, Some(2));
                let on_slime: Vec<Value> = placements(&s)
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|entry| entry["id"] == slime.id.as_str())
                    .collect();
                assert_eq!(json!(on_slime), json!([{ "id": slime.id, "value": 2, "placed": 2 }]));
                assert_eq!(draws_by(s.last_events(), P1), 1);
            }

            #[test]
            fn its_own_cry_draws_it_nothing_the_placement_is_on_another_permanent() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [CRAWLER, ANCHOR], "field": [VANILLA], "library": lib(3) },
                    "p2": { "hand": [ANCHOR] },
                }));

                let vanilla = s.card(VANILLA).clone();
                s.play(CRAWLER, json!({ "targets": at(&vanilla) }));

                assert_eq!(draws_by(s.events(), P1), 0);
            }

            #[test]
            fn s2_4_a_full_hand_burns_the_draw() {
                crate::register_all();
                let ten: Vec<&str> = (0..10).map(|_| FILLER).collect();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": ten, "field": [CRAWLER], "library": lib(3) },
                    "p2": { "hand": [CRAWLER, ANCHOR] },
                }));
                let mine = s.unit(P1, 1).expect("p1's Crawler is on the field");

                let theirs = crawlers(&s, P2)[0].clone();
                s.play(&theirs, json!({ "targets": at(&mine) }));

                assert_eq!(s.hand(P1).len(), 10);
                assert_eq!(s.last_events().iter().filter(|event| matches!(event, GameEvent::Burned { .. })).count(), 1);
            }

            #[test]
            fn s2_4_with_an_empty_deck_the_draw_is_fatigue() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [CRAWLER, ANCHOR], "field": [CRAWLER], "health": 20 }, "p2": { "hand": [ANCHOR] } }));
                let on_field = s.unit(P1, 1).expect("a Crawler on the field");
                let in_hand = s.hand(P1).into_iter().find(|card| card.def_id == CRAWLER).expect("a Crawler in hand");

                s.play(&in_hand, json!({ "targets": at(&on_field) }));

                assert_eq!(
                    s.last_events()
                        .iter()
                        .filter(|event| matches!(event, GameEvent::Fatigue { player: PlayerId::P1, .. }))
                        .count(),
                    1,
                );
                assert!(s.state().players.p1.hero.health < 20);
            }

            #[test]
            fn r78_its_tokens_go_when_it_leaves_the_field_bounced_it_comes_back_with_none() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [CRAWLER, ANCHOR], "field": [{ "def": CRAWLER, "counters": { "plague": 3 } }], "library": lib(3) },
                    "p2": { "hand": [FLOOD, ANCHOR] },
                    "active": "p2",
                }));
                let on_field = s.unit(P1, 1).expect("a Crawler on the field");

                s.play(FLOOD, json!({}));

                s.expect_in_zone(&on_field, "hand");
                assert!(s.card(&on_field).counters.plague.is_none());
            }

            #[test]
            fn r386_an_upgrade_of_its_tokens_places_2_in_one_placement_of_its_draw_draws_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [CRAWLER, CRAWLER, ANCHOR], "library": lib(4) },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA] },
                }));
                let (first, second) = two_crawlers(&s);
                step_param(s.card_mut(&first), "tokens", 1);
                step_param(s.card_mut(&first), "draw", 1);

                let vanilla = s.card(VANILLA).clone();
                s.play(&first, json!({ "targets": at(&vanilla) }));
                assert_eq!(s.card(VANILLA).counters.plague, Some(2));
                assert_eq!(placements(&s), json!([{ "id": vanilla.id, "value": 2, "placed": 2 }]));

                s.play(&second, json!({ "targets": at(&first) }));
                assert_eq!(draws_by(s.last_events(), P1), 2);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn is_a_4_4_and_its_cry_places_2_plague_counters_on_another_permanent_as_one_placement() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": CRAWLER, "radiant": true }, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA] },
                }));
                let vanilla = s.card(VANILLA).clone();

                s.play(CRAWLER, json!({ "targets": at(&vanilla) }));

                s.expect_stats(CRAWLER, json!({ "attack": 4, "health": 4 }));
                assert_eq!(placements(&s), json!([{ "id": vanilla.id, "value": 2, "placed": 2 }]));
            }

            #[test]
            fn whenever_plague_counters_are_placed_on_it_it_draws_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [CRAWLER, ANCHOR], "field": [{ "def": CRAWLER, "radiant": true }], "library": lib(3) },
                    "p2": { "hand": [ANCHOR] },
                }));
                let on_field = s.unit(P1, 1).expect("a Crawler on the field");

                let in_hand = s.hand(P1).into_iter().find(|card| card.def_id == CRAWLER).expect("a Crawler in hand");
                s.play(&in_hand, json!({ "targets": at(&on_field) }));

                assert_eq!(draws_by(s.last_events(), P1), 2);
            }

            #[test]
            fn r177_a_face_down_enemy_trap_takes_its_2_tokens_without_being_named_to_you() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": CRAWLER, "radiant": true }, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "backrow": [{ "def": UNLICENSED, "faceUp": false }] },
                }));
                let trap = s.card(UNLICENSED).clone();

                s.play(CRAWLER, json!({ "targets": at(&trap) }));

                assert_eq!(s.card(&trap).counters.plague, Some(2));
                assert!(!js(&s.view(P1)).to_string().contains(UNLICENSED));
            }

            #[test]
            fn r386_a_degrade_of_its_draw_draws_1_of_its_tokens_places_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [CRAWLER, { "def": CRAWLER, "radiant": true }, ANCHOR], "library": lib(4) },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA] },
                }));
                let radiant_one =
                    s.hand(P1).into_iter().find(|card| card.def_id == CRAWLER && card.radiant).expect("a Radiant Crawler");
                let base_one =
                    s.hand(P1).into_iter().find(|card| card.def_id == CRAWLER && !card.radiant).expect("a base Crawler");
                step_param(s.card_mut(&radiant_one), "tokens", -1);
                step_param(s.card_mut(&radiant_one), "draw", -1);

                let vanilla = s.card(VANILLA).clone();
                s.play(&radiant_one, json!({ "targets": at(&vanilla) }));
                assert_eq!(placements(&s), json!([{ "id": vanilla.id, "value": 1, "placed": 1 }]));

                s.play(&base_one, json!({ "targets": at(&radiant_one) }));
                assert_eq!(draws_by(s.last_events(), P1), 1);
            }
        }
    }
}
