//! Cards buried under a Stack pile, and what Reborn brings back (SPEC §3.2, §4.5, §7, R13, R175).
//! Found by the polish-4 edge-case hunt, round 3 (docs/polish/4-edge-cases.md, lenses L2, L3 and L4);
//! every case here failed before its fix.
//!
//!  - §3.2, R13: a card dormant under a Stack is not on the field, so §4.5's check never collects it
//!    there. The top shields it: an aura or a layer-2 Felinor that stops reaching it cannot kill it,
//!    and it is judged when it resumes on top, with the board's auras reaching it again.
//!  - R175: a token summoned X/X comes back through Reborn with that X/X, its printed face (§7), at
//!    1 health — not as a printed 0/0 that dies again.
//!  - Round 10 (lens "engine invariants"). R212, R119: a card that resumes as its pile's top did not
//!    see what happened while it lay dormant (§3.2, R153), so it answers neither the death that
//!    uncovered it nor the play that was resolving when it resumed.
//!  - The review of round 10. R212: that is kept against the one removal that uncovered it, so a
//!    later move of the card that left — exiled out of its graveyard — does not make the card that
//!    resumed long before miss what its batch did first.
//!
//! Port of `packages/cards/test/stacks-and-reborn.test.ts` (SURFACE §4.1, §8).

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

const VANILLA: &str = "core-008";
const HIT_JOB: &str = "core-016";
/// Felinor Fiender (2) and Hit Job, (3) since patch v0.2.0 (issue #40), in one turn.
const MANA_FOR_FIENDER_AND_HIT_JOB: i32 = 5;
const BIG_FELINOR: &str = "core-043";
const RUSH_TOKEN_FARM: &str = "core-058";
const FIENDER: &str = "core-092";
const RUSH_TOKEN: &str = "core-t-rush";
const BREAD: &str = "core-t-bread";
const LIBRARY: [&str; 6] = [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA];

use super::scenario;

/// `[{ pick: "instance", instanceId: card.id }]`.
fn at(card: &CardInstance) -> Value {
    json!([{ "pick": "instance", "instanceId": card.id }])
}

fn unit_at(g: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
    match g.unit(player, lane) {
        Some(card) => card.clone(),
        None => panic!("setup: {player} lane {lane} is empty"),
    }
}

fn unit_id(g: &Scenario, player: PlayerId, lane: i32) -> Option<String> {
    g.unit(player, lane).map(|card| card.id.clone())
}

/// Every instance a `destroyed` event has named since setup.
fn destroyed_ids(g: &Scenario) -> Vec<String> {
    g.events()
        .iter()
        .filter_map(|event| match event {
            GameEvent::Destroyed { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

/// The `damage` events among `events` whose source is `source`.
fn hits_from(events: &[GameEvent], source: &str) -> Vec<GameEvent> {
    events
        .iter()
        .filter(|event| matches!(event, GameEvent::Damage { source_id: Some(id), .. } if id == source))
        .cloned()
        .collect()
}

fn events_json(events: &[GameEvent]) -> String {
    serde_json::to_string(events).expect("events serialise")
}

/// `registerScripts({ ...registeredScripts(), [id]: { base: script, radiant: script } })`.
fn register_fixture_script(id: &str, script: Script) {
    let mut scripts = registered_scripts().clone();
    scripts.insert(
        id.to_string(),
        CardScripts {
            base: script.clone(),
            radiant: script,
        },
    );
    register_scripts(scripts);
}

/// A Core-set Common definition named after its id, with the same face on both sides.
fn fixture_def(id: &str, type_: CardType, face: Value) -> CardDef {
    json_as(json!({
        "id": id,
        "index": id,
        "name": id,
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face.clone(),
        "radiant": face,
    }))
}

/// A fixture unit placed on p1's side in `lane`, not summoning sick (TS sets `summonedTurn = 0` on the
/// live object, which this writes into the state).
fn place_on_p1_side(g: &mut Scenario, id: &str, lane: i32) -> CardInstance {
    let mut card = new_instance(g.state_mut(), id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
    if !place_on_field(
        g.state_mut(),
        &mut card,
        ZoneSlot {
            player: PlayerId::P1,
            row: Row::Units,
            lane,
        },
        Default::default(),
    ) {
        panic!("could not place the watcher");
    }
    let live = find_instance_mut(g.state_mut(), &card.id).expect("the placed watcher");
    live.summoned_turn = Some(0);
    live.clone()
}

mod r13_the_state_check_never_reaches_under_a_stack_pile {
    use super::*;

    #[test]
    fn r13_a_damaged_rush_token_that_radiant_rush_token_farm_keeps_alive_survives_being_buried_under_a_stack_and_resumes_when_the_top_leaves_3_2()
     {
        let mut g = scenario(json!({
            "p1": {
                "hand": [FIENDER, HIT_JOB],
                "mana": MANA_FOR_FIENDER_AND_HIT_JOB,
                "field": [{ "def": RUSH_TOKEN, "lane": 1, "damage": 4 }],
                "backrow": [{ "def": RUSH_TOKEN_FARM, "lane": 1, "radiant": true }],
                "library": LIBRARY,
            },
            "p2": { "hand": [VANILLA], "library": LIBRARY },
        }));
        let token = unit_at(&g, PlayerId::P1, 1);
        // 3/3 printed, +3/+3 from the radiant Farm's aura, 4 damage taken: alive at 2.
        g.expect_stats(&token.id, json!({ "attack": 6, "maxHealth": 6, "health": 2 }));

        // Felinor Fiender (Stack) goes on top of it: the token is dormant and the aura stops reaching
        // it, but nothing hit it and the check does not look under the pile.
        g.play(FIENDER, json!({ "zone": 1 }));
        let fiender = unit_at(&g, PlayerId::P1, 1);
        assert_eq!(fiender.def_id, FIENDER);
        assert!(!destroyed_ids(&g).contains(&token.id));
        g.expect_in_zone(&token.id, "field");

        // The top leaves; the token resumes on top of the zone, and the Farm's aura reaches it again.
        g.play(HIT_JOB, json!({ "targets": at(&fiender) }));
        assert_eq!(unit_id(&g, PlayerId::P1, 1), Some(token.id.clone()));
        g.expect_stats(&token.id, json!({ "attack": 6, "maxHealth": 6, "health": 2 }));
    }

    #[test]
    fn r13_a_felinor_fiender_buried_under_a_second_fiender_is_not_killed_under_the_pile_when_the_felinor_feeding_it_dies_10_4_layer_2()
     {
        let mut g = scenario(json!({
            "p1": {
                "hand": [FIENDER, HIT_JOB],
                "mana": MANA_FOR_FIENDER_AND_HIT_JOB,
                "field": [{ "def": FIENDER, "lane": 1, "damage": 10 }, { "def": BIG_FELINOR, "lane": 2 }],
                "library": LIBRARY,
            },
            "p2": { "hand": [VANILLA], "library": LIBRARY },
        }));
        let buried = unit_at(&g, PlayerId::P1, 1);
        let felinor = unit_at(&g, PlayerId::P1, 2);
        // 5/7 printed plus Big Felinor's 3/10, with 10 damage: alive at 7.
        g.expect_stats(&buried.id, json!({ "attack": 8, "maxHealth": 17, "health": 7 }));

        g.play(FIENDER, json!({ "zone": 1 }));
        assert_ne!(unit_id(&g, PlayerId::P1, 1), Some(buried.id.clone()));

        g.play(HIT_JOB, json!({ "targets": at(&felinor) }));
        g.expect_in_zone(&felinor.id, "graveyard");
        assert!(!destroyed_ids(&g).contains(&buried.id));
        g.expect_in_zone(&buried.id, "field");
    }

    #[test]
    fn r13_r47_r175_a_dormant_felinor_fiender_with_reborn_that_a_felinor_s_death_leaves_at_0_never_comes_back_on_top_of_the_card_acting_in_its_zone_3_2()
     {
        // Lane 1 is a pile: Fiender A on top of Fiender B. B took 10 damage while Big Felinor fed its
        // stats (8/17), and has Reborn. Hit Job on Big Felinor drops B to 5/7 under 10 damage.
        let mut g = scenario(json!({
            "p1": {
                "hand": [HIT_JOB, VANILLA],
                "field": [
                    { "def": FIENDER, "lane": 1, "damage": 10 },
                    { "def": BIG_FELINOR, "lane": 2 },
                    { "def": FIENDER, "lane": 1, "stack": true },
                ],
                "library": LIBRARY,
            },
            "p2": { "hand": [VANILLA], "library": LIBRARY },
        }));
        let pile = g.state().players.p1.units[0].clone().unwrap_or_default();
        let (Some(top), Some(buried)) = (pile.first().cloned(), pile.get(1).cloned()) else {
            panic!("setup: a two-card pile in lane 1");
        };
        find_instance_mut(g.state_mut(), &buried.id)
            .expect("the buried Fiender")
            .granted_keywords
            .push(Keyword::Reborn);
        let felinor = unit_at(&g, PlayerId::P1, 2);

        g.play(HIT_JOB, json!({ "targets": at(&felinor) }));
        g.expect_in_zone(&felinor.id, "graveyard");

        // B is not on the field, so it neither dies nor returns; A still acts for lane 1. R175 puts a
        // body back on top of a pile only when it died on top of it.
        assert_eq!(unit_id(&g, PlayerId::P1, 1), Some(top.id.clone()));
        assert!(!destroyed_ids(&g).contains(&buried.id));
    }
}

mod r175_a_token_summoned_x_x_comes_back_through_reborn_as_that_x_x {
    use super::*;

    #[test]
    fn r175_a_bread_token_given_reborn_comes_back_at_1_health_with_its_x_x_rather_than_as_a_0_0_that_dies_again_4_5_step_4_7()
     {
        let mut g = scenario(json!({
            "p1": {
                "hand": [HIT_JOB, VANILLA],
                "field": [{ "def": BREAD, "lane": 1, "statsOverride": { "attack": 3, "health": 3 } }],
                "library": LIBRARY,
            },
            "p2": { "hand": [VANILLA], "library": LIBRARY },
        }));
        let bread = unit_at(&g, PlayerId::P1, 1);
        // TS `g.card(bread).grantedKeywords.push(…)`: the card as it stands, written in the state.
        let live = g.card(&bread.id).id.clone();
        find_instance_mut(g.state_mut(), &live)
            .expect("the Bread Token")
            .granted_keywords
            .push(Keyword::Reborn);
        assert_eq!(g.stats(&bread.id).max_health, 3);

        g.play(HIT_JOB, json!({ "targets": at(&bread) }));

        // "Returns ... at 1 health without Reborn", a unit token included: 3/3 with 2 damage.
        assert_eq!(unit_id(&g, PlayerId::P1, 1), Some(bread.id.clone()));
        g.expect_stats(&bread.id, json!({ "attack": 3, "maxHealth": 3, "health": 1 }));
        // It died once, to Hit Job, not a second time to its own printed 0/0.
        assert_eq!(
            g.events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == bread.id))
                .count(),
            1
        );
    }
}

/// A test-only unit on p1's side: a transient def in the match state and its script in the registry.
/// Its one trigger deals 1 damage to the enemy hero whenever an event of type `on` is dispatched, so
/// a firing is a `damage` event whose source is this card. No Core unit watches another card's event
/// from the field (#32 and #91 watch their own), which is why these cases need one.
fn place_watcher(g: &mut Scenario, id: &str, lane: i32, on: GameEventType) -> CardInstance {
    let script = Script {
        triggers: vec![TriggerDef::new(format!("{id}:on-{on}"), &[on], |_ctx, _event| {
            vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 })))]
        })],
        ..Script::default()
    };
    let face = json!({
        "attack": 1,
        "health": 5,
        "keywords": [],
        "text": format!("On {on}: deal 1 damage to the enemy hero"),
    });
    let def = fixture_def(id, CardType::Unit, face);
    g.state_mut().transient_defs.insert(id.to_string(), def);
    register_fixture_script(id, script);
    place_on_p1_side(g, id, lane)
}

mod r212_r119_a_card_that_resumes_on_top_of_its_pile_did_not_see_what_uncovered_it_3_2_r153 {
    use super::*;

    // Found while teaching the probe's stay shadow §3.2's resume: when the top of a Stack pile leaves,
    // the card beneath starts acting in that same step with no event of its own, and the resolution
    // loop then offered it the very event that uncovered it — R212 read the moves the events after it
    // recorded, and a resume records none (`stays.noteUncovered` keeps it now).
    #[test]
    fn r212_r153_a_card_dormant_under_a_stack_does_not_answer_the_death_that_uncovers_it_3_2() {
        let mut g = scenario(json!({
            "p1": { "hand": [FIENDER, HIT_JOB], "mana": MANA_FOR_FIENDER_AND_HIT_JOB },
            "p2": { "field": ["core-011"], "health": 20 },
        }));
        // "Whenever a unit dies, deal 1 damage to the enemy hero", in p1's lane 1.
        let watcher = place_watcher(&mut g, "fixture:r10-death-watcher", 1, GameEventType::Destroyed);
        let fiender = g.card(FIENDER).clone();

        // #92 Felinor Fiender has Stack (§6.2): played onto lane 1, it buries the watcher (§3.2, R13).
        g.play(&fiender.id, json!({ "zone": 1 }));
        assert_eq!(
            g.state().players.p1.units[0]
                .as_ref()
                .map(|pile| pile.iter().map(|card| card.id.clone()).collect::<Vec<_>>()),
            Some(vec![fiender.id.clone(), watcher.id.clone()])
        );

        // #16 Hit Job destroys the Fiender, and the watcher resumes as its pile's top (§3.2). When the
        // Fiender died the watcher was dormant — "not on the field for effects" (§3.2), registering
        // nothing (R153) — and R212 answers an event "as the board stood when it happened": it comes
        // back into play because of that death, as a Reborn body does, and R212 has a Reborn body not
        // answer the hit that killed its unit. Hearthstone agrees: a minion that enters play because a
        // minion died does not see that death.
        g.play(HIT_JOB, json!({ "targets": at(&fiender) }));
        assert_eq!(unit_id(&g, PlayerId::P1, 1), Some(watcher.id.clone()));
        let hits = hits_from(g.last_events(), &watcher.id);
        assert_eq!(hits, Vec::<GameEvent>::new(), "{}", events_json(g.last_events()));
        assert_eq!(g.state().players.p2.hero.health, 20);
    }

    #[test]
    fn r119_r153_a_card_a_play_uncovers_in_its_stack_pile_does_not_answer_that_play_s_card_resolved_3_2() {
        let mut g = scenario(json!({
            "p1": { "hand": [FIENDER, HIT_JOB], "mana": MANA_FOR_FIENDER_AND_HIT_JOB },
            "p2": { "field": ["core-011"], "health": 20 },
        }));
        // "Whenever a card finishes resolving, deal 1 damage to the enemy hero", in p1's lane 1.
        let watcher = place_watcher(&mut g, "fixture:r10-resolve-watcher", 1, GameEventType::CardResolved);
        let fiender = g.card(FIENDER).clone();
        g.play(&fiender.id, json!({ "zone": 1 }));
        // Step 4 buried the watcher before the Fiender's own play reached step 7, so it answered nothing.
        assert_eq!(g.state().players.p2.hero.health, 20);

        // Hit Job kills the Fiender inside its own resolution, so the watcher resumes before step 7's
        // `cardResolved`: it lay dormant as the play began and registered nothing then (R153), and R119
        // counts it with the arrivals the play's `cardResolved` names, as it does a Reborn body.
        g.play(HIT_JOB, json!({ "targets": at(&fiender) }));
        assert_eq!(unit_id(&g, PlayerId::P1, 1), Some(watcher.id.clone()));
        let hits = hits_from(g.last_events(), &watcher.id);
        assert_eq!(hits, Vec::<GameEvent>::new(), "{}", events_json(g.last_events()));
        assert_eq!(g.state().players.p2.hero.health, 20);
    }
}

/// A test-only unit on p1's side that deals 1 damage to the enemy hero whenever another source deals
/// damage, so a firing is a `damage` event whose source is this card (and it never answers its own).
fn place_hit_watcher(g: &mut Scenario, id: &str, lane: i32) -> CardInstance {
    let script = Script {
        triggers: vec![TriggerDef::new(
            format!("{id}:on-damage"),
            &[GameEventType::Damage],
            |ctx, event| {
                let self_id = ctx.self_.as_ref().map(|card| card.id.clone());
                // TS `ctx.event.sourceId !== ctx.self?.id`: only a source that is this card is its own.
                let another = match event {
                    GameEvent::Damage { source_id, .. } => match (source_id, &self_id) {
                        (Some(source), Some(own)) => source != own,
                        _ => true,
                    },
                    _ => false,
                };
                if another {
                    vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 })))]
                } else {
                    vec![]
                }
            },
        )],
        ..Script::default()
    };
    let face = json!({
        "attack": 1,
        "health": 5,
        "keywords": [],
        "text": "Whenever another source deals damage, deal 1 to the enemy hero",
    });
    let def = fixture_def(id, CardType::Unit, face);
    g.state_mut().transient_defs.insert(id.to_string(), def);
    register_fixture_script(id, script);
    place_on_p1_side(g, id, lane)
}

/// A test-only 0-cost Spell in p1's hand whose Cry is `cry`.
fn spell_in_hand(g: &mut Scenario, id: &str, cry: Option<Hook>) -> CardInstance {
    let face = json!({ "keywords": [], "text": id });
    let def = fixture_def(id, CardType::Spell, face);
    g.state_mut().transient_defs.insert(id.to_string(), def);
    let script = match cry {
        None => Script::default(),
        Some(cry) => Script {
            cry: Some(cry),
            ..Script::default()
        },
    };
    register_fixture_script(id, script);
    let card = new_instance(g.state_mut(), id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
    g.state_mut().players.p1.hand.push(card.clone());
    card
}

mod r212_a_resume_is_kept_against_the_removal_that_caused_it_and_no_later_move_3_2_r153 {
    use super::*;

    // Review of round 10: the note a pile top's removal leaves (`stays.noteUncovered`) was kept until
    // that card next left the field, so every later move of the card that left — exiled out of its
    // graveyard, discarded, shuffled back — read as the removal again, and the card that had resumed
    // long before was taken for one that resumed after whatever the same batch did first.
    #[test]
    fn r212_a_card_that_resumed_under_a_stack_answers_a_later_hit_even_when_the_same_list_then_exiles_the_card_that_uncovered_it_from_the_graveyard()
     {
        let mut g = scenario(json!({
            "p1": { "hand": [FIENDER, HIT_JOB], "mana": MANA_FOR_FIENDER_AND_HIT_JOB },
            "p2": { "field": ["core-011"], "health": 20 },
        }));
        let watcher = place_hit_watcher(&mut g, "fixture:r11-hit-watcher", 1);
        let fiender = g.card(FIENDER).clone();
        g.play(&fiender.id, json!({ "zone": 1 }));
        g.play(HIT_JOB, json!({ "targets": at(&fiender) }));
        // The Fiender died on top of the watcher, which resumed then and answered nothing of it.
        assert_eq!(unit_id(&g, PlayerId::P1, 1), Some(watcher.id.clone()));
        assert_eq!(g.state().players.p2.hero.health, 20);

        // A later action: a Spell hits p2's hero, then exiles p1's graveyard, the Fiender with it. The
        // watcher has stood on top since the last action, so it saw the hit; the exile is a move out of a
        // graveyard, not the death that uncovered it.
        let spell = spell_in_hand(
            &mut g,
            "fixture:r11-hit-then-exile",
            Some(hook(|_ctx| {
                vec![
                    effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 }))),
                    effects::exile_matching(json_as(json!({ "zones": ["graveyard"] }))),
                ]
            })),
        );
        g.play(&spell.id, json!({}));
        assert!(
            g.last_events()
                .iter()
                .any(|event| matches!(event, GameEvent::Exiled { instance_id, .. } if *instance_id == fiender.id))
        );
        let hits = hits_from(g.last_events(), &watcher.id);
        assert_eq!(hits.len(), 1, "{}", events_json(g.last_events()));
        assert_eq!(g.state().players.p2.hero.health, 18);
    }

    #[test]
    fn r212_a_card_still_dormant_when_a_hit_landed_does_not_answer_it_though_the_same_list_then_kills_the_card_above_it_and_exiles_that_card_from_the_graveyard()
     {
        let mut g = scenario(json!({
            "p1": { "hand": [FIENDER], "mana": 4 },
            "p2": { "field": ["core-011"], "health": 20 },
        }));
        let watcher = place_hit_watcher(&mut g, "fixture:r11-dormant-watcher", 1);
        let fiender = g.card(FIENDER).clone();
        g.play(&fiender.id, json!({ "zone": 1 }));

        // The hit lands while the watcher lies under the Fiender; the sacrifice uncovers it, and the
        // exile then takes the Fiender on out of the graveyard before the loop reaches the hit. The
        // watcher resumed after the hit, so it did not see it — the later exile changes nothing.
        let fiender_id = fiender.id.clone();
        let spell = spell_in_hand(
            &mut g,
            "fixture:r11-hit-sacrifice-exile",
            Some(hook(move |_ctx| {
                vec![
                    effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 }))),
                    effects::sacrifice(json_as(json!({ "target": { "of": "instance", "instanceId": fiender_id } }))),
                    effects::exile_matching(json_as(json!({ "zones": ["graveyard"] }))),
                ]
            })),
        );
        g.play(&spell.id, json!({}));
        assert_eq!(unit_id(&g, PlayerId::P1, 1), Some(watcher.id.clone()));
        assert!(
            g.last_events()
                .iter()
                .any(|event| matches!(event, GameEvent::Exiled { instance_id, .. } if *instance_id == fiender.id))
        );
        let hits = hits_from(g.last_events(), &watcher.id);
        assert_eq!(hits, Vec::<GameEvent>::new(), "{}", events_json(g.last_events()));
        assert_eq!(g.state().players.p2.hero.health, 19);
    }
}
