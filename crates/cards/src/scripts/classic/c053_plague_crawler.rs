//! C #53 Plague Crawler (SPEC §8.6 row 53). (1) Unit, Common, 2/2 → 4/4.
//!   Base:    "Cry: Place {tokens|Plague Counter|Plague Counters} on another permanent.
//!             Whenever Plague Counters are placed on this, draw {draw}." — 1 token, draw 1
//!   Radiant: the same text — 2 tokens, draw 2
//!
//! The Cry places on a permanent on either side, itself included, picked in a prompt as it resolves
//! (patch v0.3.10, #831: "another permanent", a declared target, before; R831016), so placing on itself
//! wakes its own draw; face-down cards are offered by id alone (R177). "Whenever Plague Counters are placed on this" triggers once per placement naming
//! it, however many tokens and whoever placed them (removal draws nothing). The condition is the trigger's
//! `when` (R99), checked again in `run`.
//! Tuned numbers `tokens` and `draw` (R386) read through `param` on the running face.

use jackioh_engine::effects::{choose_target, draw, place_plague};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-053";

/// The Cry's resume step: the permanent picked as it resolves.
const PLACED_STEP: &str = "placed";

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
        // "a permanent": the top of any unit pile or any backrow card, either side, this one included
        // (patch v0.3.10, #831; "another permanent", a declared target, before), so the pick is a prompt
        // made once this stands on the field (R831016).
        cry: Some(hook(|_ctx| {
            vec![choose_target(json_as(json!({
                "step": PLACED_STEP,
                "scope": { "side": "any", "of": ["unit", "backrow"] },
                "prompt": "Plague Crawler: place Plague Counters on a permanent",
            })))]
        })),
        resume: IndexMap::from([(
            PLACED_STEP,
            hook(|ctx| {
                let amount = param(&*ctx, "tokens");
                vec![place_plague(json_as(json!({ "target": { "of": "chosen" }, "amount": amount })))]
            }),
        )]),
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

// C #53 — SPEC §8.6 row 53, BUILD M9 Classic row C 53: Cry places plague tokens on a permanent picked as
// it resolves, itself included (R831016; face-down carries only id, R177); draws on placement on it; tuned numbers read via `param()` (R386).
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

    /// The permanents the open prompt offers, each once, in its order.
    fn offered(s: &Scenario) -> Vec<String> {
        let mut ids: IndexSet<String> = IndexSet::new();
        for option in &s.state().pending.as_ref().expect("the Cry's prompt").options {
            if let Selection::Instance { instance_id } = &option.selection {
                ids.insert(instance_id.clone());
            }
        }
        ids.into_iter().collect()
    }

    /// Plays `crawler` and answers its Cry's prompt with `target`: the pick is made as the Cry
    /// resolves (patch v0.3.10, #831; a declared target before), so the Crawler itself may be it (R831016).
    fn crawl(s: &mut Scenario, crawler: impl Into<CardRef>, target: &CardInstance) {
        s.play(crawler, json!({}));
        s.answer(json!(target.id));
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
        fn declares_no_target_a_resume_step_its_two_numbers_and_one_script_on_both_faces() {
            crate::register_all();
            let def = js(&registered_catalog()[ID]);
            assert_eq!(def["id"], CRAWLER);
            let scripts = script();
            // The pick is a prompt as the Cry resolves (R831016), not a declared target.
            assert!(scripts.base.targets.is_empty());
            assert!(scripts.base.resume.contains_key("placed"));
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

                crawl(&mut s, CRAWLER, &vanilla);

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
                crawl(&mut s, &first, &vanilla);
                let mana_well = s.card(MANA_WELL).clone();
                crawl(&mut s, &second, &mana_well);

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
                crawl(&mut s, CRAWLER, &menace);

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
            fn r831016_its_prompt_offers_every_permanent_itself_included_and_on_itself_it_draws_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [CRAWLER, ANCHOR], "field": [VANILLA], "library": lib(3) },
                    "p2": { "hand": [ANCHOR], "field": [MENACE], "backrow": [MANA_WELL] },
                }));
                let crawler = s.card(CRAWLER).clone();

                s.play(&crawler, json!({}));

                let want: IndexSet<String> =
                    [CRAWLER, VANILLA, MENACE, MANA_WELL].iter().map(|def| s.card(*def).id.clone()).collect();
                assert_eq!(offered(&s).into_iter().collect::<IndexSet<String>>(), want);
                s.answer(json!(crawler.id));
                assert_eq!(s.card(&crawler).counters.plague, Some(1));
                assert_eq!(draws_by(s.last_events(), P1), 1);
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
                s.play(&crawler, json!({}));
                assert_eq!(offered(&s), vec![crawler.id.clone(), top.id.clone()]);
                let dormant_id = dormant.id.clone();
                s.expect_refused(move |s| s.answer(json!(dormant_id)));

                s.answer(json!(top.id));
                assert_eq!(s.card(&top).counters.plague, Some(1));
                assert!(s.card(&dormant).counters.plague.is_none());
            }

            #[test]
            fn r831016_with_no_other_permanent_on_the_field_it_offers_itself_alone() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [CRAWLER, ANCHOR], "library": lib(2) }, "p2": { "hand": [ANCHOR] } }));
                let crawler = s.card(CRAWLER).clone();

                s.play(&crawler, json!({}));

                s.expect_in_zone(&crawler, "field");
                assert_eq!(offered(&s), vec![crawler.id.clone()]);
                s.answer(json!(crawler.id));
                assert_eq!(placements(&s), json!([{ "id": crawler.id, "value": 1, "placed": 1 }]));
                assert_eq!(draws_by(s.events(), P1), 1);
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
                s.play(&crawler, json!({}));
                assert_eq!(offered(&s), vec![crawler.id.clone(), trap.id.clone()]);
                assert!(!js(&s.view(P1)).to_string().contains(UNLICENSED));

                s.answer(json!(trap.id));

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
                // The first one alone on the field may only place on itself (R831016), and draws for it.
                s.play(&first, json!({}));
                s.answer(json!(first.id));
                assert_eq!(draws_by(s.events(), P1), 1);

                crawl(&mut s, &second, &first);

                assert_eq!(s.card(&first).counters.plague, Some(2));
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
                crawl(&mut s, &theirs, &mine);

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

                crawl(&mut s, &radiant_one, &on_field);

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
                crawl(&mut s, &theirs, &slime);

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
            fn its_own_cry_on_another_permanent_draws_it_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [CRAWLER, ANCHOR], "field": [VANILLA], "library": lib(3) },
                    "p2": { "hand": [ANCHOR] },
                }));

                let vanilla = s.card(VANILLA).clone();
                crawl(&mut s, CRAWLER, &vanilla);

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
                crawl(&mut s, &theirs, &mine);

                assert_eq!(s.hand(P1).len(), 10);
                assert_eq!(s.last_events().iter().filter(|event| matches!(event, GameEvent::Burned { .. })).count(), 1);
            }

            #[test]
            fn s2_4_with_an_empty_deck_the_draw_is_fatigue() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [CRAWLER, ANCHOR], "field": [CRAWLER], "health": 20 }, "p2": { "hand": [ANCHOR] } }));
                let on_field = s.unit(P1, 1).expect("a Crawler on the field");
                let in_hand = s.hand(P1).into_iter().find(|card| card.def_id == CRAWLER).expect("a Crawler in hand");

                crawl(&mut s, &in_hand, &on_field);

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
                crawl(&mut s, &first, &vanilla);
                assert_eq!(s.card(VANILLA).counters.plague, Some(2));
                assert_eq!(placements(&s), json!([{ "id": vanilla.id, "value": 2, "placed": 2 }]));

                crawl(&mut s, &second, &first);
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

                crawl(&mut s, CRAWLER, &vanilla);

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
                crawl(&mut s, &in_hand, &on_field);

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

                crawl(&mut s, CRAWLER, &trap);

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
                crawl(&mut s, &radiant_one, &vanilla);
                assert_eq!(placements(&s), json!([{ "id": vanilla.id, "value": 1, "placed": 1 }]));

                crawl(&mut s, &base_one, &radiant_one);
                assert_eq!(draws_by(s.last_events(), P1), 1);
            }
        }
    }
}
